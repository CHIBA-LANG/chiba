use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    render_source_expr, Expr, MatchArm, Pattern, RecordField, SourceItem, SourceProgram,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlobalInitPlan {
    pub statics: Vec<GlobalStatic>,
    pub init_order: Vec<String>,
    pub init_order_ids: Vec<GlobalStaticId>,
    pub diagnostics: Vec<GlobalInitDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobalStaticId {
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalStatic {
    pub owner: String,
    pub name: String,
    pub ty: Option<String>,
    pub body: Expr,
    pub dependencies: Vec<String>,
    pub dependency_ids: Vec<GlobalStaticId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlobalInitDiagnostic {
    DuplicateStatic {
        name: String,
    },
    StaticFunctionNameConflict {
        name: String,
    },
    StaticInitCycle {
        cycle: Vec<String>,
    },
    InvalidStaticAdtConstructor {
        static_name: String,
        data: String,
        ctor: String,
    },
    UnsupportedStaticInitializer {
        static_name: String,
        expr: String,
    },
}

pub fn analyze_global_init(program: &SourceProgram) -> GlobalInitPlan {
    let owner = program
        .namespace
        .as_ref()
        .map(|namespace| namespace.dotted())
        .unwrap_or_else(|| "root".to_string());
    let mut function_names = BTreeSet::new();
    let mut static_names = BTreeSet::new();
    let mut diagnostics = Vec::new();

    for item in &program.items {
        match item {
            SourceItem::Def { name, .. } | SourceItem::ExternDef { name, .. } => {
                function_names.insert(name.clone());
            }
            SourceItem::StaticValue { name, .. } => {
                if !static_names.insert(name.clone()) {
                    diagnostics.push(GlobalInitDiagnostic::DuplicateStatic { name: name.clone() });
                }
            }
        }
    }

    for name in static_names.intersection(&function_names) {
        diagnostics.push(GlobalInitDiagnostic::StaticFunctionNameConflict { name: name.clone() });
    }

    let statics = program
        .items
        .iter()
        .filter_map(|item| match item {
            SourceItem::StaticValue { name, ty, body, .. } => {
                let mut refs = BTreeSet::new();
                collect_expr_vars(body, &mut refs);
                let dependencies = refs
                    .into_iter()
                    .filter(|dep| static_names.contains(dep))
                    .collect::<Vec<_>>();
                let dependency_ids = dependencies
                    .iter()
                    .map(|dep| GlobalStaticId::new(owner.clone(), dep.clone()))
                    .collect::<Vec<_>>();
                Some(GlobalStatic {
                    owner: owner.clone(),
                    name: name.clone(),
                    ty: ty.clone(),
                    body: body.clone(),
                    dependencies,
                    dependency_ids,
                })
            }
            SourceItem::Def { .. } | SourceItem::ExternDef { .. } => None,
        })
        .collect::<Vec<_>>();

    for static_value in &statics {
        validate_static_initializer(static_value, &statics, &static_names, &mut diagnostics);
    }

    let mut graph = BTreeMap::<GlobalStaticId, Vec<GlobalStaticId>>::new();
    for static_value in &statics {
        let deps = graph
            .entry(GlobalStaticId::new(
                static_value.owner.clone(),
                static_value.name.clone(),
            ))
            .or_default();
        for dep in &static_value.dependency_ids {
            if !deps.contains(dep) {
                deps.push(dep.clone());
            }
        }
    }
    let (init_order_ids, cycles) = topo_sort(&graph);
    diagnostics.extend(
        cycles
            .into_iter()
            .map(|cycle| GlobalInitDiagnostic::StaticInitCycle {
                cycle: cycle.into_iter().map(|id| id.name).collect(),
            }),
    );
    let init_order = init_order_ids
        .iter()
        .map(|id| id.name.clone())
        .collect::<Vec<_>>();

    GlobalInitPlan {
        statics,
        init_order,
        init_order_ids,
        diagnostics,
    }
}

impl GlobalStaticId {
    pub fn new(owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            name: name.into(),
        }
    }
}

fn validate_static_initializer(
    static_value: &GlobalStatic,
    statics: &[GlobalStatic],
    static_names: &BTreeSet<String>,
    diagnostics: &mut Vec<GlobalInitDiagnostic>,
) {
    validate_static_initializer_expr_scoped(
        &static_value.body,
        &static_value.name,
        statics,
        static_names,
        diagnostics,
        &BTreeSet::new(),
    );
}

fn validate_static_initializer_expr_scoped(
    expr: &Expr,
    static_name: &str,
    statics: &[GlobalStatic],
    static_names: &BTreeSet<String>,
    diagnostics: &mut Vec<GlobalInitDiagnostic>,
    bound: &BTreeSet<String>,
) -> bool {
    match expr {
        Expr::Var(name) if bound.contains(name) || static_names.contains(name) => true,
        Expr::Var(_) => {
            diagnostics.push(GlobalInitDiagnostic::UnsupportedStaticInitializer {
                static_name: static_name.to_string(),
                expr: render_source_expr(expr),
            });
            false
        }
        Expr::Lit(_) => true,
        Expr::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            let mut supported = true;
            if !variants.iter().any(|variant| variant == ctor) {
                diagnostics.push(GlobalInitDiagnostic::InvalidStaticAdtConstructor {
                    static_name: static_name.to_string(),
                    data: data.clone(),
                    ctor: ctor.clone(),
                });
                supported = false;
            }
            for arg in args {
                supported &= validate_static_initializer_expr_scoped(
                    arg,
                    static_name,
                    statics,
                    static_names,
                    diagnostics,
                    bound,
                );
            }
            supported
        }
        Expr::Field { receiver, name, .. } => {
            if let Some(value) = static_record_field_expr(receiver, name, statics) {
                validate_static_initializer_expr_scoped(
                    value,
                    static_name,
                    statics,
                    static_names,
                    diagnostics,
                    bound,
                )
            } else {
                diagnostics.push(GlobalInitDiagnostic::UnsupportedStaticInitializer {
                    static_name: static_name.to_string(),
                    expr: render_source_expr(expr),
                });
                false
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            validate_static_initializer_expr_scoped(
                lhs,
                static_name,
                statics,
                static_names,
                diagnostics,
                bound,
            ) & validate_static_initializer_expr_scoped(
                rhs,
                static_name,
                statics,
                static_names,
                diagnostics,
                bound,
            )
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            validate_static_initializer_expr_scoped(
                cond,
                static_name,
                statics,
                static_names,
                diagnostics,
                bound,
            ) & validate_static_initializer_expr_scoped(
                then_branch,
                static_name,
                statics,
                static_names,
                diagnostics,
                bound,
            ) & validate_static_initializer_expr_scoped(
                else_branch,
                static_name,
                statics,
                static_names,
                diagnostics,
                bound,
            )
        }
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            let then_bound = with_pattern_bindings(bound, pattern);
            validate_static_initializer_expr_scoped(
                scrutinee,
                static_name,
                statics,
                static_names,
                diagnostics,
                bound,
            ) & validate_static_initializer_expr_scoped(
                then_branch,
                static_name,
                statics,
                static_names,
                diagnostics,
                &then_bound,
            ) & validate_static_initializer_expr_scoped(
                else_branch,
                static_name,
                statics,
                static_names,
                diagnostics,
                bound,
            )
        }
        Expr::Match { scrutinee, arms } => {
            let mut supported = validate_static_initializer_expr_scoped(
                scrutinee,
                static_name,
                statics,
                static_names,
                diagnostics,
                bound,
            );
            for MatchArm { pattern, body } in arms {
                let arm_bound = with_pattern_bindings(bound, pattern);
                supported &= validate_static_initializer_expr_scoped(
                    body,
                    static_name,
                    statics,
                    static_names,
                    diagnostics,
                    &arm_bound,
                );
            }
            supported
        }
        Expr::Nominal { expr, .. } => validate_static_initializer_expr_scoped(
            expr,
            static_name,
            statics,
            static_names,
            diagnostics,
            bound,
        ),
        Expr::Tuple { .. }
        | Expr::SliceLiteral(_)
        | Expr::Record { .. }
        | Expr::RecordUpdate { .. }
        | Expr::Range { .. }
        | Expr::Lambda { .. }
        | Expr::Call { .. }
        | Expr::Instantiate { .. }
        | Expr::MethodCall { .. }
        | Expr::Index { .. }
        | Expr::Reset { .. }
        | Expr::Shift { .. } => {
            diagnostics.push(GlobalInitDiagnostic::UnsupportedStaticInitializer {
                static_name: static_name.to_string(),
                expr: render_source_expr(expr),
            });
            false
        }
    }
}

fn static_record_field_expr<'a>(
    receiver: &'a Expr,
    name: &str,
    statics: &'a [GlobalStatic],
) -> Option<&'a Expr> {
    static_record_field_expr_seen(receiver, name, statics, &mut BTreeSet::new())
}

fn static_record_field_expr_seen<'a>(
    receiver: &'a Expr,
    name: &str,
    statics: &'a [GlobalStatic],
    seen: &mut BTreeSet<String>,
) -> Option<&'a Expr> {
    match receiver {
        Expr::Var(binding) if seen.insert(binding.clone()) => {
            let value = statics.iter().find(|item| item.name == *binding)?;
            static_record_field_expr_seen(&value.body, name, statics, seen)
        }
        Expr::Record(fields) => fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.value),
        Expr::RecordUpdate { base, fields } => fields
            .iter()
            .rev()
            .find(|field| field.name == name)
            .map(|field| &field.value)
            .or_else(|| static_record_field_expr_seen(base, name, statics, seen)),
        _ => None,
    }
}

fn topo_sort(
    graph: &BTreeMap<GlobalStaticId, Vec<GlobalStaticId>>,
) -> (Vec<GlobalStaticId>, Vec<Vec<GlobalStaticId>>) {
    let mut marks = BTreeMap::<GlobalStaticId, Mark>::new();
    let mut order = Vec::new();
    let mut cycles = Vec::new();
    let mut stack = Vec::new();

    for name in graph.keys() {
        visit(name, graph, &mut marks, &mut stack, &mut order, &mut cycles);
    }

    order.retain(|name| {
        !cycles
            .iter()
            .any(|cycle| cycle.iter().any(|cycle_name| cycle_name == name))
    });
    (order, cycles)
}

fn visit(
    name: &GlobalStaticId,
    graph: &BTreeMap<GlobalStaticId, Vec<GlobalStaticId>>,
    marks: &mut BTreeMap<GlobalStaticId, Mark>,
    stack: &mut Vec<GlobalStaticId>,
    order: &mut Vec<GlobalStaticId>,
    cycles: &mut Vec<Vec<GlobalStaticId>>,
) {
    match marks.get(name) {
        Some(Mark::Done) => return,
        Some(Mark::Visiting) => {
            if let Some(start) = stack.iter().position(|entry| entry == name) {
                let mut cycle = stack[start..].to_vec();
                cycle.push(name.clone());
                if !cycles.contains(&cycle) {
                    cycles.push(cycle);
                }
            }
            return;
        }
        None => {}
    }

    marks.insert(name.clone(), Mark::Visiting);
    stack.push(name.clone());
    if let Some(deps) = graph.get(name) {
        for dep in deps {
            visit(dep, graph, marks, stack, order, cycles);
        }
    }
    stack.pop();
    marks.insert(name.clone(), Mark::Done);
    order.push(name.clone());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    Visiting,
    Done,
}

fn collect_expr_vars(expr: &Expr, refs: &mut BTreeSet<String>) {
    collect_expr_vars_scoped(expr, refs, &BTreeSet::new());
}

fn collect_expr_vars_scoped(expr: &Expr, refs: &mut BTreeSet<String>, bound: &BTreeSet<String>) {
    match expr {
        Expr::Var(name) => {
            if !bound.contains(name) {
                refs.insert(name.clone());
            }
        }
        Expr::Lit(_) => {}
        Expr::Lambda { param, body } => {
            let next_bound = with_bound_names(bound, std::iter::once(param.clone()));
            collect_expr_vars_scoped(body, refs, &next_bound);
        }
        Expr::Call { callee, args } => {
            collect_expr_vars_scoped(callee, refs, bound);
            collect_exprs(args, refs, bound);
        }
        Expr::Instantiate { callee, .. } => collect_expr_vars_scoped(callee, refs, bound),
        Expr::Tuple(fields) => collect_exprs(fields, refs, bound),
        Expr::SliceLiteral(items) => collect_exprs(items, refs, bound),
        Expr::Record(fields) => collect_record_fields(fields, refs, bound),
        Expr::RecordUpdate { base, fields } => {
            collect_expr_vars_scoped(base, refs, bound);
            collect_record_fields(fields, refs, bound);
        }
        Expr::AdtCtor { args, .. } => collect_exprs(args, refs, bound),
        Expr::Field { receiver, .. } => collect_expr_vars_scoped(receiver, refs, bound),
        Expr::MethodCall { receiver, args, .. } => {
            collect_expr_vars_scoped(receiver, refs, bound);
            collect_exprs(args, refs, bound);
        }
        Expr::Index { receiver, index } => {
            collect_expr_vars_scoped(receiver, refs, bound);
            collect_expr_vars_scoped(index, refs, bound);
        }
        Expr::Range { start, end } => {
            collect_expr_vars_scoped(start, refs, bound);
            collect_expr_vars_scoped(end, refs, bound);
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_expr_vars_scoped(lhs, refs, bound);
            collect_expr_vars_scoped(rhs, refs, bound);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_expr_vars_scoped(cond, refs, bound);
            collect_expr_vars_scoped(then_branch, refs, bound);
            collect_expr_vars_scoped(else_branch, refs, bound);
        }
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            collect_expr_vars_scoped(scrutinee, refs, bound);
            let then_bound = with_pattern_bindings(bound, pattern);
            collect_expr_vars_scoped(then_branch, refs, &then_bound);
            collect_expr_vars_scoped(else_branch, refs, bound);
        }
        Expr::Match { scrutinee, arms } => {
            collect_expr_vars_scoped(scrutinee, refs, bound);
            collect_match_arms(arms, refs, bound);
        }
        Expr::Nominal { expr, .. } => collect_expr_vars_scoped(expr, refs, bound),
        Expr::Reset { body, .. } => collect_expr_vars_scoped(body, refs, bound),
        Expr::Shift { binder, body } => {
            let next_bound = with_bound_names(bound, std::iter::once(binder.clone()));
            collect_expr_vars_scoped(body, refs, &next_bound);
        }
    }
}

fn collect_exprs(exprs: &[Expr], refs: &mut BTreeSet<String>, bound: &BTreeSet<String>) {
    for expr in exprs {
        collect_expr_vars_scoped(expr, refs, bound);
    }
}

fn collect_record_fields(
    fields: &[RecordField],
    refs: &mut BTreeSet<String>,
    bound: &BTreeSet<String>,
) {
    for field in fields {
        collect_expr_vars_scoped(&field.value, refs, bound);
    }
}

fn collect_match_arms(arms: &[MatchArm], refs: &mut BTreeSet<String>, bound: &BTreeSet<String>) {
    for arm in arms {
        let arm_bound = with_pattern_bindings(bound, &arm.pattern);
        collect_expr_vars_scoped(&arm.body, refs, &arm_bound);
    }
}

fn with_pattern_bindings(bound: &BTreeSet<String>, pattern: &Pattern) -> BTreeSet<String> {
    with_bound_names(bound, pattern.bindings())
}

fn with_bound_names(
    bound: &BTreeSet<String>,
    names: impl IntoIterator<Item = String>,
) -> BTreeSet<String> {
    let mut next = bound.clone();
    next.extend(names);
    next
}
