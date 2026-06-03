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
        collect_invalid_static_adt_ctors(static_value, &mut diagnostics);
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

fn collect_invalid_static_adt_ctors(
    static_value: &GlobalStatic,
    diagnostics: &mut Vec<GlobalInitDiagnostic>,
) {
    collect_invalid_static_adt_ctors_in_expr(&static_value.body, &static_value.name, diagnostics);
}

fn collect_invalid_static_adt_ctors_in_expr(
    expr: &Expr,
    static_name: &str,
    diagnostics: &mut Vec<GlobalInitDiagnostic>,
) {
    match expr {
        Expr::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            if !variants.iter().any(|variant| variant == ctor) {
                diagnostics.push(GlobalInitDiagnostic::InvalidStaticAdtConstructor {
                    static_name: static_name.to_string(),
                    data: data.clone(),
                    ctor: ctor.clone(),
                });
            }
            for arg in args {
                collect_invalid_static_adt_ctors_in_expr(arg, static_name, diagnostics);
            }
        }
        Expr::Lambda { body, .. } => {
            collect_invalid_static_adt_ctors_in_expr(body, static_name, diagnostics)
        }
        Expr::Call { callee, args } => {
            collect_invalid_static_adt_ctors_in_expr(callee, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_exprs(args, static_name, diagnostics);
        }
        Expr::Instantiate { callee, .. } => {
            collect_invalid_static_adt_ctors_in_expr(callee, static_name, diagnostics)
        }
        Expr::Tuple(fields) => {
            collect_invalid_static_adt_ctors_in_exprs(fields, static_name, diagnostics)
        }
        Expr::Record(fields) => {
            collect_invalid_static_adt_ctors_in_record_fields(fields, static_name, diagnostics)
        }
        Expr::RecordUpdate { base, fields } => {
            collect_invalid_static_adt_ctors_in_expr(base, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_record_fields(fields, static_name, diagnostics);
        }
        Expr::Field { receiver, .. } => {
            collect_invalid_static_adt_ctors_in_expr(receiver, static_name, diagnostics)
        }
        Expr::MethodCall { receiver, args, .. } => {
            collect_invalid_static_adt_ctors_in_expr(receiver, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_exprs(args, static_name, diagnostics);
        }
        Expr::Index { receiver, index } => {
            collect_invalid_static_adt_ctors_in_expr(receiver, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_expr(index, static_name, diagnostics);
        }
        Expr::Range { start, end } => {
            collect_invalid_static_adt_ctors_in_expr(start, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_expr(end, static_name, diagnostics);
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_invalid_static_adt_ctors_in_expr(lhs, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_expr(rhs, static_name, diagnostics);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_invalid_static_adt_ctors_in_expr(cond, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_expr(then_branch, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_expr(else_branch, static_name, diagnostics);
        }
        Expr::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_invalid_static_adt_ctors_in_expr(scrutinee, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_expr(then_branch, static_name, diagnostics);
            collect_invalid_static_adt_ctors_in_expr(else_branch, static_name, diagnostics);
        }
        Expr::Match { scrutinee, arms } => {
            collect_invalid_static_adt_ctors_in_expr(scrutinee, static_name, diagnostics);
            for MatchArm { body, .. } in arms {
                collect_invalid_static_adt_ctors_in_expr(body, static_name, diagnostics);
            }
        }
        Expr::Nominal { expr, .. } => {
            collect_invalid_static_adt_ctors_in_expr(expr, static_name, diagnostics)
        }
        Expr::Reset { body, .. } | Expr::Shift { body, .. } => {
            collect_invalid_static_adt_ctors_in_expr(body, static_name, diagnostics)
        }
        Expr::Var(_) | Expr::Lit(_) => {}
    }
}

fn collect_invalid_static_adt_ctors_in_exprs(
    exprs: &[Expr],
    static_name: &str,
    diagnostics: &mut Vec<GlobalInitDiagnostic>,
) {
    for expr in exprs {
        collect_invalid_static_adt_ctors_in_expr(expr, static_name, diagnostics);
    }
}

fn collect_invalid_static_adt_ctors_in_record_fields(
    fields: &[RecordField],
    static_name: &str,
    diagnostics: &mut Vec<GlobalInitDiagnostic>,
) {
    for field in fields {
        collect_invalid_static_adt_ctors_in_expr(&field.value, static_name, diagnostics);
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
