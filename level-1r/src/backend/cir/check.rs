use crate::typed::{common_type, Type, TypedExpr, TypedExprKind, UsageColor};

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
    pub replay_safety: ReplaySafety,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContinuationKind {
    Cont1,
    ContN,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReplaySafety {
    Safe,
    Unsafe,
    RollbackRegion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlError {
    ShiftOutsideReset { binder: String },
    UnsafeMultiResumeCapture { binder: String },
}

#[derive(Clone, Debug)]
struct Boundary {
    kind: ContinuationKind,
    answer: Type,
}

pub fn analyze_control(expr: &TypedExpr) -> ControlFacts {
    let mut facts = ControlFacts::default();
    visit(expr, &mut Vec::new(), &mut facts, ReplaySafety::Safe);
    facts
}

fn visit(
    expr: &TypedExpr,
    stack: &mut Vec<Boundary>,
    facts: &mut ControlFacts,
    replay_context: ReplaySafety,
) {
    match &expr.kind {
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
        TypedExprKind::Lambda { body, .. } => visit(body, stack, facts, ReplaySafety::Safe),
        TypedExprKind::Call { callee, args } => {
            let child_context = replay_context.join(ReplaySafety::Unsafe);
            visit(callee, stack, facts, child_context);
            for arg in args {
                visit(arg, stack, facts, child_context);
            }
        }
        TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                visit(field, stack, facts, replay_context);
            }
        }
        TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                visit(item, stack, facts, replay_context);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                visit(&field.value, stack, facts, replay_context);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            visit(base, stack, facts, replay_context);
            for field in fields {
                visit(&field.value, stack, facts, replay_context);
            }
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                visit(arg, stack, facts, replay_context);
            }
        }
        TypedExprKind::Field { receiver, .. } => visit(receiver, stack, facts, replay_context),
        TypedExprKind::MethodCall { receiver, args, .. } => {
            let child_context = replay_context.join(ReplaySafety::Unsafe);
            visit(receiver, stack, facts, child_context);
            for arg in args {
                visit(arg, stack, facts, child_context);
            }
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            let child_context = replay_context.join(ReplaySafety::Unsafe);
            visit(receiver, stack, facts, child_context);
            visit(index, stack, facts, child_context);
        }
        TypedExprKind::Range { start, end } => {
            visit(start, stack, facts, replay_context);
            visit(end, stack, facts, replay_context);
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            visit(lhs, stack, facts, replay_context);
            visit(rhs, stack, facts, replay_context);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit(cond, stack, facts, replay_context);
            visit(then_branch, stack, facts, replay_context);
            visit(else_branch, stack, facts, replay_context);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            visit(scrutinee, stack, facts, replay_context);
            visit(then_branch, stack, facts, replay_context);
            visit(else_branch, stack, facts, replay_context);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            visit(scrutinee, stack, facts, replay_context);
            for arm in arms {
                visit(&arm.body, stack, facts, replay_context);
            }
        }
        TypedExprKind::Nominal { expr, .. } => visit(expr, stack, facts, replay_context),
        TypedExprKind::Reset { multi, body } => {
            stack.push(Boundary {
                kind: if *multi {
                    ContinuationKind::ContN
                } else {
                    ContinuationKind::Cont1
                },
                answer: expr.ty.clone(),
            });
            visit(body, stack, facts, ReplaySafety::Safe);
            stack.pop();
        }
        TypedExprKind::Shift { binder, body } => {
            match stack.last() {
                Some(boundary) => {
                    let replay_safety = replay_safety_for(boundary.kind, replay_context);
                    facts.continuations.push(ContinuationFact {
                        binder: binder.clone(),
                        kind: boundary.kind,
                        input: continuation_input_type(binder, body),
                        answer: boundary.answer.clone(),
                        usage: match boundary.kind {
                            ContinuationKind::Cont1 => UsageColor::One,
                            ContinuationKind::ContN => UsageColor::Many,
                        },
                        replay_safety,
                    });
                    if boundary.kind == ContinuationKind::ContN
                        && replay_safety == ReplaySafety::Unsafe
                    {
                        facts.errors.push(ControlError::UnsafeMultiResumeCapture {
                            binder: binder.clone(),
                        });
                    }
                }
                None => facts.errors.push(ControlError::ShiftOutsideReset {
                    binder: binder.clone(),
                }),
            }
            visit(body, stack, facts, ReplaySafety::Safe);
        }
    }
}

impl ReplaySafety {
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (ReplaySafety::Unsafe, _) | (_, ReplaySafety::Unsafe) => ReplaySafety::Unsafe,
            (ReplaySafety::RollbackRegion, _) | (_, ReplaySafety::RollbackRegion) => {
                ReplaySafety::RollbackRegion
            }
            (ReplaySafety::Safe, ReplaySafety::Safe) => ReplaySafety::Safe,
        }
    }
}

fn replay_safety_for(kind: ContinuationKind, context: ReplaySafety) -> ReplaySafety {
    match kind {
        ContinuationKind::Cont1 => ReplaySafety::Safe,
        ContinuationKind::ContN => context,
    }
}

fn continuation_input_type(binder: &str, body: &TypedExpr) -> Type {
    let mut inputs = Vec::new();
    collect_resume_inputs(binder, body, &mut inputs);
    inputs
        .into_iter()
        .reduce(|left, right| common_type(&left, &right))
        .unwrap_or(Type::Unknown)
}

fn collect_resume_inputs(binder: &str, expr: &TypedExpr, inputs: &mut Vec<Type>) {
    match &expr.kind {
        TypedExprKind::Call { callee, args } => {
            if is_resume_callee(binder, callee) {
                match args.as_slice() {
                    [arg] => inputs.push(arg.ty.clone()),
                    _ => inputs.push(Type::Unknown),
                }
            }
            collect_resume_inputs(binder, callee, inputs);
            for arg in args {
                collect_resume_inputs(binder, arg, inputs);
            }
        }
        TypedExprKind::Lambda { body, .. } => collect_resume_inputs(binder, body, inputs),
        TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                collect_resume_inputs(binder, field, inputs);
            }
        }
        TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                collect_resume_inputs(binder, item, inputs);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                collect_resume_inputs(binder, &field.value, inputs);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            collect_resume_inputs(binder, base, inputs);
            for field in fields {
                collect_resume_inputs(binder, &field.value, inputs);
            }
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_resume_inputs(binder, arg, inputs);
            }
        }
        TypedExprKind::Field { receiver, .. } => collect_resume_inputs(binder, receiver, inputs),
        TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_resume_inputs(binder, receiver, inputs);
            for arg in args {
                collect_resume_inputs(binder, arg, inputs);
            }
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            collect_resume_inputs(binder, receiver, inputs);
            collect_resume_inputs(binder, index, inputs);
        }
        TypedExprKind::Range { start, end } => {
            collect_resume_inputs(binder, start, inputs);
            collect_resume_inputs(binder, end, inputs);
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            collect_resume_inputs(binder, lhs, inputs);
            collect_resume_inputs(binder, rhs, inputs);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_resume_inputs(binder, cond, inputs);
            collect_resume_inputs(binder, then_branch, inputs);
            collect_resume_inputs(binder, else_branch, inputs);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_resume_inputs(binder, scrutinee, inputs);
            collect_resume_inputs(binder, then_branch, inputs);
            collect_resume_inputs(binder, else_branch, inputs);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_resume_inputs(binder, scrutinee, inputs);
            for arm in arms {
                collect_resume_inputs(binder, &arm.body, inputs);
            }
        }
        TypedExprKind::Nominal { expr, .. } => collect_resume_inputs(binder, expr, inputs),
        TypedExprKind::Reset { body, .. } | TypedExprKind::Shift { body, .. } => {
            collect_resume_inputs(binder, body, inputs);
        }
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
    }
}

fn is_resume_callee(binder: &str, callee: &TypedExpr) -> bool {
    matches!(
        (&callee.kind, &callee.ty),
        (
            TypedExprKind::Var(name),
            Type::Continuation {
                input: _,
                answer: _,
                ..
            }
        ) if name == binder
    )
}
