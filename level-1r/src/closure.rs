use std::collections::BTreeSet;

use crate::alpha::{AlphaExpr, AlphaExprKind, BinderId};
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
    pub binder: Option<BinderId>,
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

pub fn analyze_alpha_closures(expr: &AlphaExpr) -> ClosureFacts {
    let mut facts = ClosureFacts::default();
    let mut scope = BTreeSet::new();
    collect_alpha(expr, &mut scope, &mut facts);
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
                    binder: None,
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

fn collect_alpha(expr: &AlphaExpr, scope: &mut BTreeSet<BinderId>, facts: &mut ClosureFacts) {
    match &expr.kind {
        AlphaExprKind::Var(_) | AlphaExprKind::Lit(_) => {}
        AlphaExprKind::Call { callee, arg } => {
            collect_alpha(callee, scope, facts);
            collect_alpha(arg, scope, facts);
        }
        AlphaExprKind::Reset { body, .. } | AlphaExprKind::Shift { body, .. } => {
            collect_alpha(body, scope, facts);
        }
        AlphaExprKind::Lambda { param, body } => {
            let mut locals = BTreeSet::new();
            locals.insert(param.id);
            let mut free = Vec::new();
            collect_alpha_free_vars(body, &mut locals, &mut free);
            let captures = free
                .into_iter()
                .filter(|(_, id)| scope.contains(id))
                .map(|(name, id)| CaptureFact {
                    name,
                    binder: Some(id),
                    usage: UsageColor::Many,
                })
                .collect::<Vec<_>>();
            facts.closures.push(ClosureFact {
                param: param.name.clone(),
                storage: if captures.is_empty() {
                    ClosureStorageKind::DirectNoCapture
                } else {
                    ClosureStorageKind::EnvClosure
                },
                captures,
            });
            scope.insert(param.id);
            collect_alpha(body, scope, facts);
            scope.remove(&param.id);
        }
    }
}

fn collect_alpha_free_vars(
    expr: &AlphaExpr,
    locals: &mut BTreeSet<BinderId>,
    free: &mut Vec<(String, BinderId)>,
) {
    match &expr.kind {
        AlphaExprKind::Var(var) => {
            if let Some(target) = var.target {
                if !locals.contains(&target) && !free.iter().any(|(_, id)| *id == target) {
                    free.push((var.name.clone(), target));
                }
            }
        }
        AlphaExprKind::Lit(_) => {}
        AlphaExprKind::Call { callee, arg } => {
            collect_alpha_free_vars(callee, locals, free);
            collect_alpha_free_vars(arg, locals, free);
        }
        AlphaExprKind::Reset { body, .. } | AlphaExprKind::Shift { body, .. } => {
            collect_alpha_free_vars(body, locals, free);
        }
        AlphaExprKind::Lambda { param, body } => {
            locals.insert(param.id);
            collect_alpha_free_vars(body, locals, free);
            locals.remove(&param.id);
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
