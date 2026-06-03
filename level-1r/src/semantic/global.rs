use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, MatchArm, RecordField, SourceItem, SourceProgram};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlobalInitPlan {
    pub statics: Vec<GlobalStatic>,
    pub init_order: Vec<String>,
    pub diagnostics: Vec<GlobalInitDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalStatic {
    pub name: String,
    pub ty: Option<String>,
    pub body: Expr,
    pub dependencies: Vec<String>,
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
    let mut function_names = BTreeSet::new();
    let mut static_names = BTreeSet::new();
    let mut diagnostics = Vec::new();

    for item in &program.items {
        match item {
            SourceItem::Def { name, .. } => {
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
                    .filter(|dep| static_names.contains(dep) && dep != name)
                    .collect::<Vec<_>>();
                Some(GlobalStatic {
                    name: name.clone(),
                    ty: ty.clone(),
                    body: body.clone(),
                    dependencies,
                })
            }
            SourceItem::Def { .. } => None,
        })
        .collect::<Vec<_>>();

    for static_value in &statics {
        validate_static_initializer(static_value, &statics, &static_names, &mut diagnostics);
    }

    let mut graph = BTreeMap::<String, Vec<String>>::new();
    for static_value in &statics {
        let deps = graph.entry(static_value.name.clone()).or_default();
        for dep in &static_value.dependencies {
            if !deps.contains(dep) {
                deps.push(dep.clone());
            }
        }
    }
    let (init_order, cycles) = topo_sort(&graph);
    diagnostics.extend(
        cycles
            .into_iter()
            .map(|cycle| GlobalInitDiagnostic::StaticInitCycle { cycle }),
    );

    GlobalInitPlan {
        statics,
        init_order,
        diagnostics,
    }
}

fn validate_static_initializer(
    static_value: &GlobalStatic,
    statics: &[GlobalStatic],
    static_names: &BTreeSet<String>,
    diagnostics: &mut Vec<GlobalInitDiagnostic>,
) {
    validate_static_initializer_expr(
        &static_value.body,
        &static_value.name,
        statics,
        static_names,
        diagnostics,
    );
}

fn validate_static_initializer_expr(
    expr: &Expr,
    static_name: &str,
    statics: &[GlobalStatic],
    static_names: &BTreeSet<String>,
    diagnostics: &mut Vec<GlobalInitDiagnostic>,
) -> bool {
    match expr {
        Expr::Var(_) => true,
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
                supported &= validate_static_initializer_expr(
                    arg,
                    static_name,
                    statics,
                    static_names,
                    diagnostics,
                );
            }
            supported
        }
        Expr::Field { receiver, name } => {
            if let Some(value) = static_record_field_expr(receiver, name, statics) {
                validate_static_initializer_expr(
                    value,
                    static_name,
                    statics,
                    static_names,
                    diagnostics,
                )
            } else {
                diagnostics.push(GlobalInitDiagnostic::UnsupportedStaticInitializer {
                    static_name: static_name.to_string(),
                    expr: format!("{expr:?}"),
                });
                false
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            validate_static_initializer_expr(lhs, static_name, statics, static_names, diagnostics)
                & validate_static_initializer_expr(
                    rhs,
                    static_name,
                    statics,
                    static_names,
                    diagnostics,
                )
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            validate_static_initializer_expr(cond, static_name, statics, static_names, diagnostics)
                & validate_static_initializer_expr(
                    then_branch,
                    static_name,
                    statics,
                    static_names,
                    diagnostics,
                )
                & validate_static_initializer_expr(
                    else_branch,
                    static_name,
                    statics,
                    static_names,
                    diagnostics,
                )
        }
        Expr::IfLet {
            pattern: _,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            validate_static_initializer_expr(
                scrutinee,
                static_name,
                statics,
                static_names,
                diagnostics,
            ) & validate_static_initializer_expr(
                then_branch,
                static_name,
                statics,
                static_names,
                diagnostics,
            ) & validate_static_initializer_expr(
                else_branch,
                static_name,
                statics,
                static_names,
                diagnostics,
            )
        }
        Expr::Match { scrutinee, arms } => {
            let mut supported = validate_static_initializer_expr(
                scrutinee,
                static_name,
                statics,
                static_names,
                diagnostics,
            );
            for MatchArm { body, .. } in arms {
                supported &= validate_static_initializer_expr(
                    body,
                    static_name,
                    statics,
                    static_names,
                    diagnostics,
                );
            }
            supported
        }
        Expr::Nominal { expr, .. } => {
            validate_static_initializer_expr(expr, static_name, statics, static_names, diagnostics)
        }
        Expr::Tuple { .. }
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
                expr: format!("{expr:?}"),
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

fn topo_sort(graph: &BTreeMap<String, Vec<String>>) -> (Vec<String>, Vec<Vec<String>>) {
    let mut marks = BTreeMap::<String, Mark>::new();
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
    name: &str,
    graph: &BTreeMap<String, Vec<String>>,
    marks: &mut BTreeMap<String, Mark>,
    stack: &mut Vec<String>,
    order: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    match marks.get(name) {
        Some(Mark::Done) => return,
        Some(Mark::Visiting) => {
            if let Some(start) = stack.iter().position(|entry| entry == name) {
                let mut cycle = stack[start..].to_vec();
                cycle.push(name.to_string());
                if !cycles.contains(&cycle) {
                    cycles.push(cycle);
                }
            }
            return;
        }
        None => {}
    }

    marks.insert(name.to_string(), Mark::Visiting);
    stack.push(name.to_string());
    if let Some(deps) = graph.get(name) {
        for dep in deps {
            visit(dep, graph, marks, stack, order, cycles);
        }
    }
    stack.pop();
    marks.insert(name.to_string(), Mark::Done);
    order.push(name.to_string());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    Visiting,
    Done,
}

fn collect_expr_vars(expr: &Expr, refs: &mut BTreeSet<String>) {
    match expr {
        Expr::Var(name) => {
            refs.insert(name.clone());
        }
        Expr::Lit(_) => {}
        Expr::Lambda { body, .. } => collect_expr_vars(body, refs),
        Expr::Call { callee, args } => {
            collect_expr_vars(callee, refs);
            collect_exprs(args, refs);
        }
        Expr::Instantiate { callee, .. } => collect_expr_vars(callee, refs),
        Expr::Tuple(fields) => collect_exprs(fields, refs),
        Expr::Record(fields) => collect_record_fields(fields, refs),
        Expr::RecordUpdate { base, fields } => {
            collect_expr_vars(base, refs);
            collect_record_fields(fields, refs);
        }
        Expr::AdtCtor { args, .. } => collect_exprs(args, refs),
        Expr::Field { receiver, .. } => collect_expr_vars(receiver, refs),
        Expr::MethodCall { receiver, args, .. } => {
            collect_expr_vars(receiver, refs);
            collect_exprs(args, refs);
        }
        Expr::Index { receiver, index } => {
            collect_expr_vars(receiver, refs);
            collect_expr_vars(index, refs);
        }
        Expr::Range { start, end } => {
            collect_expr_vars(start, refs);
            collect_expr_vars(end, refs);
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_expr_vars(lhs, refs);
            collect_expr_vars(rhs, refs);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_expr_vars(cond, refs);
            collect_expr_vars(then_branch, refs);
            collect_expr_vars(else_branch, refs);
        }
        Expr::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_vars(scrutinee, refs);
            collect_expr_vars(then_branch, refs);
            collect_expr_vars(else_branch, refs);
        }
        Expr::Match { scrutinee, arms } => {
            collect_expr_vars(scrutinee, refs);
            collect_match_arms(arms, refs);
        }
        Expr::Nominal { expr, .. } => collect_expr_vars(expr, refs),
        Expr::Reset { body, .. } | Expr::Shift { body, .. } => collect_expr_vars(body, refs),
    }
}

fn collect_exprs(exprs: &[Expr], refs: &mut BTreeSet<String>) {
    for expr in exprs {
        collect_expr_vars(expr, refs);
    }
}

fn collect_record_fields(fields: &[RecordField], refs: &mut BTreeSet<String>) {
    for field in fields {
        collect_expr_vars(&field.value, refs);
    }
}

fn collect_match_arms(arms: &[MatchArm], refs: &mut BTreeSet<String>) {
    for arm in arms {
        collect_expr_vars(&arm.body, refs);
    }
}
