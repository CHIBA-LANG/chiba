use crate::typed::{TypedExpr, TypedExprKind, Type, UsageColor};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlFacts {
    pub continuations: Vec<ContinuationFact>,
    pub errors: Vec<ControlError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationFact {
    pub binder: String,
    pub kind: ContinuationKind,
    pub input: Type,
    pub answer: Type,
    pub usage: UsageColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContinuationKind {
    Cont1,
    ContN,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlError {
    ShiftOutsideReset { binder: String },
}

#[derive(Clone, Debug)]
struct Boundary {
    kind: ContinuationKind,
    answer: Type,
}

pub fn analyze_control(expr: &TypedExpr) -> ControlFacts {
    let mut facts = ControlFacts::default();
    visit(expr, &mut Vec::new(), &mut facts);
    facts
}

fn visit(expr: &TypedExpr, stack: &mut Vec<Boundary>, facts: &mut ControlFacts) {
    match &expr.kind {
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
        TypedExprKind::Lambda { body, .. } => visit(body, stack, facts),
        TypedExprKind::Call { callee, args } => {
            visit(callee, stack, facts);
            for arg in args {
                visit(arg, stack, facts);
            }
        }
        TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                visit(field, stack, facts);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                visit(&field.value, stack, facts);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            visit(base, stack, facts);
            for field in fields {
                visit(&field.value, stack, facts);
            }
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                visit(arg, stack, facts);
            }
        }
        TypedExprKind::Field { receiver, .. } => visit(receiver, stack, facts),
        TypedExprKind::MethodCall { receiver, args, .. } => {
            visit(receiver, stack, facts);
            for arg in args {
                visit(arg, stack, facts);
            }
        }
        TypedExprKind::Index { receiver, index } => {
            visit(receiver, stack, facts);
            visit(index, stack, facts);
        }
        TypedExprKind::Range { start, end } => {
            visit(start, stack, facts);
            visit(end, stack, facts);
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            visit(lhs, stack, facts);
            visit(rhs, stack, facts);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit(cond, stack, facts);
            visit(then_branch, stack, facts);
            visit(else_branch, stack, facts);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            visit(scrutinee, stack, facts);
            visit(then_branch, stack, facts);
            visit(else_branch, stack, facts);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            visit(scrutinee, stack, facts);
            for arm in arms {
                visit(&arm.body, stack, facts);
            }
        }
        TypedExprKind::Nominal { expr, .. } => visit(expr, stack, facts),
        TypedExprKind::Reset { multi, body } => {
            stack.push(Boundary {
                kind: if *multi {
                    ContinuationKind::ContN
                } else {
                    ContinuationKind::Cont1
                },
                answer: expr.ty.clone(),
            });
            visit(body, stack, facts);
            stack.pop();
        }
        TypedExprKind::Shift { binder, body } => {
            match stack.last() {
                Some(boundary) => facts.continuations.push(ContinuationFact {
                    binder: binder.clone(),
                    kind: boundary.kind,
                    input: Type::Unknown,
                    answer: boundary.answer.clone(),
                    usage: match boundary.kind {
                        ContinuationKind::Cont1 => UsageColor::One,
                        ContinuationKind::ContN => UsageColor::Many,
                    },
                }),
                None => facts.errors.push(ControlError::ShiftOutsideReset {
                    binder: binder.clone(),
                }),
            }
            visit(body, stack, facts);
        }
    }
}
