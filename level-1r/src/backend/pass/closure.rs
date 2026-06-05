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
        TypedExprKind::Call { callee, args } => {
            collect(callee, scope, facts);
            for arg in args {
                collect(arg, scope, facts);
            }
        }
        TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                collect(field, scope, facts);
            }
        }
        TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                collect(item, scope, facts);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                collect(&field.value, scope, facts);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            collect(base, scope, facts);
            for field in fields {
                collect(&field.value, scope, facts);
            }
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect(arg, scope, facts);
            }
        }
        TypedExprKind::Field { receiver, .. } => collect(receiver, scope, facts),
        TypedExprKind::MethodCall { receiver, args, .. } => {
            collect(receiver, scope, facts);
            for arg in args {
                collect(arg, scope, facts);
            }
        }
        TypedExprKind::Index { receiver, index } => {
            collect(receiver, scope, facts);
            collect(index, scope, facts);
        }
        TypedExprKind::Range { start, end } => {
            collect(start, scope, facts);
            collect(end, scope, facts);
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            collect(lhs, scope, facts);
            collect(rhs, scope, facts);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect(cond, scope, facts);
            collect(then_branch, scope, facts);
            collect(else_branch, scope, facts);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect(scrutinee, scope, facts);
            collect(then_branch, scope, facts);
            collect(else_branch, scope, facts);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect(scrutinee, scope, facts);
            for arm in arms {
                collect(&arm.body, scope, facts);
            }
        }
        TypedExprKind::Nominal { expr, .. } => collect(expr, scope, facts),
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
        AlphaExprKind::Call { callee, args } => {
            collect_alpha(callee, scope, facts);
            for arg in args {
                collect_alpha(arg, scope, facts);
            }
        }
        AlphaExprKind::Tuple(fields) => {
            for field in fields {
                collect_alpha(field, scope, facts);
            }
        }
        AlphaExprKind::SliceLiteral(items) => {
            for item in items {
                collect_alpha(item, scope, facts);
            }
        }
        AlphaExprKind::Record(fields) => {
            for field in fields {
                collect_alpha(&field.value, scope, facts);
            }
        }
        AlphaExprKind::RecordUpdate { base, fields } => {
            collect_alpha(base, scope, facts);
            for field in fields {
                collect_alpha(&field.value, scope, facts);
            }
        }
        AlphaExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_alpha(arg, scope, facts);
            }
        }
        AlphaExprKind::Field { receiver, .. } => collect_alpha(receiver, scope, facts),
        AlphaExprKind::MethodCall { receiver, args, .. } => {
            collect_alpha(receiver, scope, facts);
            for arg in args {
                collect_alpha(arg, scope, facts);
            }
        }
        AlphaExprKind::Index { receiver, index } => {
            collect_alpha(receiver, scope, facts);
            collect_alpha(index, scope, facts);
        }
        AlphaExprKind::Range { start, end } => {
            collect_alpha(start, scope, facts);
            collect_alpha(end, scope, facts);
        }
        AlphaExprKind::Binary { lhs, rhs, .. } => {
            collect_alpha(lhs, scope, facts);
            collect_alpha(rhs, scope, facts);
        }
        AlphaExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_alpha(cond, scope, facts);
            collect_alpha(then_branch, scope, facts);
            collect_alpha(else_branch, scope, facts);
        }
        AlphaExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_alpha(scrutinee, scope, facts);
            collect_alpha(then_branch, scope, facts);
            collect_alpha(else_branch, scope, facts);
        }
        AlphaExprKind::Match { scrutinee, arms } => {
            collect_alpha(scrutinee, scope, facts);
            for arm in arms {
                collect_alpha(&arm.body, scope, facts);
            }
        }
        AlphaExprKind::Nominal { expr, .. } => collect_alpha(expr, scope, facts),
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
        AlphaExprKind::Call { callee, args } => {
            collect_alpha_free_vars(callee, locals, free);
            for arg in args {
                collect_alpha_free_vars(arg, locals, free);
            }
        }
        AlphaExprKind::Tuple(fields) => {
            for field in fields {
                collect_alpha_free_vars(field, locals, free);
            }
        }
        AlphaExprKind::SliceLiteral(items) => {
            for item in items {
                collect_alpha_free_vars(item, locals, free);
            }
        }
        AlphaExprKind::Record(fields) => {
            for field in fields {
                collect_alpha_free_vars(&field.value, locals, free);
            }
        }
        AlphaExprKind::RecordUpdate { base, fields } => {
            collect_alpha_free_vars(base, locals, free);
            for field in fields {
                collect_alpha_free_vars(&field.value, locals, free);
            }
        }
        AlphaExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_alpha_free_vars(arg, locals, free);
            }
        }
        AlphaExprKind::Field { receiver, .. } => collect_alpha_free_vars(receiver, locals, free),
        AlphaExprKind::MethodCall { receiver, args, .. } => {
            collect_alpha_free_vars(receiver, locals, free);
            for arg in args {
                collect_alpha_free_vars(arg, locals, free);
            }
        }
        AlphaExprKind::Index { receiver, index } => {
            collect_alpha_free_vars(receiver, locals, free);
            collect_alpha_free_vars(index, locals, free);
        }
        AlphaExprKind::Range { start, end } => {
            collect_alpha_free_vars(start, locals, free);
            collect_alpha_free_vars(end, locals, free);
        }
        AlphaExprKind::Binary { lhs, rhs, .. } => {
            collect_alpha_free_vars(lhs, locals, free);
            collect_alpha_free_vars(rhs, locals, free);
        }
        AlphaExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_alpha_free_vars(cond, locals, free);
            collect_alpha_free_vars(then_branch, locals, free);
            collect_alpha_free_vars(else_branch, locals, free);
        }
        AlphaExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_alpha_free_vars(scrutinee, locals, free);
            collect_alpha_free_vars(then_branch, locals, free);
            collect_alpha_free_vars(else_branch, locals, free);
        }
        AlphaExprKind::Match { scrutinee, arms } => {
            collect_alpha_free_vars(scrutinee, locals, free);
            for arm in arms {
                collect_alpha_free_vars(&arm.body, locals, free);
            }
        }
        AlphaExprKind::Nominal { expr, .. } => collect_alpha_free_vars(expr, locals, free),
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

fn collect_free_vars(expr: &TypedExpr, locals: &mut BTreeSet<String>, free: &mut BTreeSet<String>) {
    match &expr.kind {
        TypedExprKind::Var(name) => {
            if !locals.contains(name) {
                free.insert(name.clone());
            }
        }
        TypedExprKind::Lit(_) => {}
        TypedExprKind::Call { callee, args } => {
            collect_free_vars(callee, locals, free);
            for arg in args {
                collect_free_vars(arg, locals, free);
            }
        }
        TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                collect_free_vars(field, locals, free);
            }
        }
        TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                collect_free_vars(item, locals, free);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                collect_free_vars(&field.value, locals, free);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            collect_free_vars(base, locals, free);
            for field in fields {
                collect_free_vars(&field.value, locals, free);
            }
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_free_vars(arg, locals, free);
            }
        }
        TypedExprKind::Field { receiver, .. } => collect_free_vars(receiver, locals, free),
        TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_free_vars(receiver, locals, free);
            for arg in args {
                collect_free_vars(arg, locals, free);
            }
        }
        TypedExprKind::Index { receiver, index } => {
            collect_free_vars(receiver, locals, free);
            collect_free_vars(index, locals, free);
        }
        TypedExprKind::Range { start, end } => {
            collect_free_vars(start, locals, free);
            collect_free_vars(end, locals, free);
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            collect_free_vars(lhs, locals, free);
            collect_free_vars(rhs, locals, free);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_free_vars(cond, locals, free);
            collect_free_vars(then_branch, locals, free);
            collect_free_vars(else_branch, locals, free);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_free_vars(scrutinee, locals, free);
            collect_free_vars(then_branch, locals, free);
            collect_free_vars(else_branch, locals, free);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_free_vars(scrutinee, locals, free);
            for arm in arms {
                collect_free_vars(&arm.body, locals, free);
            }
        }
        TypedExprKind::Nominal { expr, .. } => collect_free_vars(expr, locals, free),
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
