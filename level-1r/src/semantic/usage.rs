use std::collections::BTreeMap;

use crate::alpha::{AlphaExpr, AlphaExprKind, BinderId};
use crate::typed::{TypedExpr, TypedExprKind, UsageColor};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsageFacts {
    pub vars: BTreeMap<String, UseCount>,
    pub binders: BTreeMap<BinderId, UseCount>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UseCount {
    Zero,
    One,
    Many,
}

impl UseCount {
    pub fn bump(self) -> Self {
        match self {
            Self::Zero => Self::One,
            Self::One | Self::Many => Self::Many,
        }
    }

    pub fn color(self) -> UsageColor {
        match self {
            Self::Zero | Self::One => UsageColor::One,
            Self::Many => UsageColor::Many,
        }
    }
}

pub fn analyze_usage(expr: &TypedExpr) -> UsageFacts {
    let mut facts = UsageFacts::default();
    visit(expr, &mut facts);
    facts
}

pub fn analyze_alpha_usage(expr: &AlphaExpr) -> UsageFacts {
    let mut facts = UsageFacts::default();
    visit_alpha(expr, &mut facts);
    facts
}

fn visit(expr: &TypedExpr, facts: &mut UsageFacts) {
    match &expr.kind {
        TypedExprKind::Var(name) => {
            let current = facts.vars.get(name).copied().unwrap_or(UseCount::Zero);
            facts.vars.insert(name.clone(), current.bump());
        }
        TypedExprKind::Lit(_) => {}
        TypedExprKind::Lambda { body, .. } => visit(body, facts),
        TypedExprKind::Call { callee, args } => {
            visit(callee, facts);
            for arg in args {
                visit(arg, facts);
            }
        }
        TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                visit(field, facts);
            }
        }
        TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                visit(item, facts);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                visit(&field.value, facts);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            visit(base, facts);
            for field in fields {
                visit(&field.value, facts);
            }
        }
        TypedExprKind::DynRowPackage { payload, .. } => visit(payload, facts),
        TypedExprKind::DynRowField { package, .. } => visit(package, facts),
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                visit(arg, facts);
            }
        }
        TypedExprKind::AdtToTuple { value, .. } => visit(value, facts),
        TypedExprKind::TupleToAdt { value, .. } => visit(value, facts),
        TypedExprKind::Field { receiver, .. } => visit(receiver, facts),
        TypedExprKind::MethodCall { receiver, args, .. } => {
            visit(receiver, facts);
            for arg in args {
                visit(arg, facts);
            }
        }
        TypedExprKind::Assign { target, value, .. } => {
            visit(target, facts);
            visit(value, facts);
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            visit(receiver, facts);
            visit(index, facts);
        }
        TypedExprKind::Range { start, end } => {
            visit(start, facts);
            visit(end, facts);
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            visit(lhs, facts);
            visit(rhs, facts);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit(cond, facts);
            visit(then_branch, facts);
            visit(else_branch, facts);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            visit(scrutinee, facts);
            visit(then_branch, facts);
            visit(else_branch, facts);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            visit(scrutinee, facts);
            for arm in arms {
                visit(&arm.body, facts);
            }
        }
        TypedExprKind::Nominal { expr, .. } => visit(expr, facts),
        TypedExprKind::Reset { body, .. } => visit(body, facts),
        TypedExprKind::Shift { body, .. } => visit(body, facts),
    }
}

fn bump_binder(facts: &mut UsageFacts, binder: BinderId) {
    let current = facts
        .binders
        .get(&binder)
        .copied()
        .unwrap_or(UseCount::Zero);
    facts.binders.insert(binder, current.bump());
}

fn visit_alpha(expr: &AlphaExpr, facts: &mut UsageFacts) {
    match &expr.kind {
        AlphaExprKind::Var(var) => {
            if let Some(target) = var.target {
                bump_binder(facts, target);
            } else {
                let current = facts.vars.get(&var.name).copied().unwrap_or(UseCount::Zero);
                facts.vars.insert(var.name.clone(), current.bump());
            }
        }
        AlphaExprKind::Lit(_) => {}
        AlphaExprKind::Lambda { body, .. } => visit_alpha(body, facts),
        AlphaExprKind::Call { callee, args } => {
            visit_alpha(callee, facts);
            for arg in args {
                visit_alpha(arg, facts);
            }
        }
        AlphaExprKind::Tuple(fields) => {
            for field in fields {
                visit_alpha(field, facts);
            }
        }
        AlphaExprKind::SliceLiteral(items) => {
            for item in items {
                visit_alpha(item, facts);
            }
        }
        AlphaExprKind::Record(fields) => {
            for field in fields {
                visit_alpha(&field.value, facts);
            }
        }
        AlphaExprKind::RecordUpdate { base, fields } => {
            visit_alpha(base, facts);
            for field in fields {
                visit_alpha(&field.value, facts);
            }
        }
        AlphaExprKind::AdtCtor { args, .. } => {
            for arg in args {
                visit_alpha(arg, facts);
            }
        }
        AlphaExprKind::Field { receiver, .. } => visit_alpha(receiver, facts),
        AlphaExprKind::MethodCall { receiver, args, .. } => {
            visit_alpha(receiver, facts);
            for arg in args {
                visit_alpha(arg, facts);
            }
        }
        AlphaExprKind::Assign { target, value } => {
            visit_alpha(target, facts);
            visit_alpha(value, facts);
        }
        AlphaExprKind::Index { receiver, index } => {
            visit_alpha(receiver, facts);
            visit_alpha(index, facts);
        }
        AlphaExprKind::Range { start, end } => {
            visit_alpha(start, facts);
            visit_alpha(end, facts);
        }
        AlphaExprKind::Binary { lhs, rhs, .. } => {
            visit_alpha(lhs, facts);
            visit_alpha(rhs, facts);
        }
        AlphaExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit_alpha(cond, facts);
            visit_alpha(then_branch, facts);
            visit_alpha(else_branch, facts);
        }
        AlphaExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            visit_alpha(scrutinee, facts);
            visit_alpha(then_branch, facts);
            visit_alpha(else_branch, facts);
        }
        AlphaExprKind::Match { scrutinee, arms } => {
            visit_alpha(scrutinee, facts);
            for arm in arms {
                visit_alpha(&arm.body, facts);
            }
        }
        AlphaExprKind::Nominal { expr, .. } => visit_alpha(expr, facts),
        AlphaExprKind::Reset { body, .. } => visit_alpha(body, facts),
        AlphaExprKind::Shift { body, .. } => visit_alpha(body, facts),
    }
}
