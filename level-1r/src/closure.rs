use std::collections::BTreeSet;

use crate::typed::{TypedExpr, TypedExprKind, UsageColor};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClosureFacts {
    pub closures: Vec<ClosureFact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureFact {
    pub param: String,
    pub captures: Vec<CaptureFact>,
    pub storage: ClosureStorageKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureFact {
    pub name: String,
    pub usage: UsageColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClosureStorageKind {
    DirectNoCapture,
    EnvClosure,
}

pub fn analyze_closures(expr: &TypedExpr) -> ClosureFacts {
    let mut facts = ClosureFacts::default();
    let mut scope = BTreeSet::new();
    collect(expr, &mut scope, &mut facts);
    facts
}

fn collect(expr: &TypedExpr, scope: &mut BTreeSet<String>, facts: &mut ClosureFacts) {
    match &expr.kind {
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
        TypedExprKind::Call { callee, arg } => {
            collect(callee, scope, facts);
            collect(arg, scope, facts);
        }
        TypedExprKind::Reset { body, .. } | TypedExprKind::Shift { body, .. } => {
            collect(body, scope, facts);
        }
        TypedExprKind::Lambda { param, body, .. } => {
            let mut locals = BTreeSet::new();
            locals.insert(param.clone());
            let mut free = BTreeSet::new();
            collect_free_vars(body, &mut locals, &mut free);
            let captures = free
                .into_iter()
                .filter(|name| scope.contains(name))
                .map(|name| CaptureFact {
                    name,
                    usage: UsageColor::Many,
                })
                .collect::<Vec<_>>();
            facts.closures.push(ClosureFact {
                param: param.clone(),
                storage: if captures.is_empty() {
                    ClosureStorageKind::DirectNoCapture
                } else {
                    ClosureStorageKind::EnvClosure
                },
                captures,
            });
            scope.insert(param.clone());
            collect(body, scope, facts);
            scope.remove(param);
        }
    }
}

fn collect_free_vars(
    expr: &TypedExpr,
    locals: &mut BTreeSet<String>,
    free: &mut BTreeSet<String>,
) {
    match &expr.kind {
        TypedExprKind::Var(name) => {
            if !locals.contains(name) {
                free.insert(name.clone());
            }
        }
        TypedExprKind::Lit(_) => {}
        TypedExprKind::Call { callee, arg } => {
            collect_free_vars(callee, locals, free);
            collect_free_vars(arg, locals, free);
        }
        TypedExprKind::Reset { body, .. } | TypedExprKind::Shift { body, .. } => {
            collect_free_vars(body, locals, free);
        }
        TypedExprKind::Lambda { param, body, .. } => {
            locals.insert(param.clone());
            collect_free_vars(body, locals, free);
            locals.remove(param);
        }
    }
}
