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
        TypedExprKind::Call { callee, arg } => {
            visit(callee, facts);
            visit(arg, facts);
        }
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
        AlphaExprKind::Call { callee, arg } => {
            visit_alpha(callee, facts);
            visit_alpha(arg, facts);
        }
        AlphaExprKind::Reset { body, .. } => visit_alpha(body, facts),
        AlphaExprKind::Shift { body, .. } => visit_alpha(body, facts),
    }
}
