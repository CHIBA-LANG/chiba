use std::fmt;

use crate::ast::{Literal, Pattern};
use crate::control::ContinuationKind;
use crate::typed::{TypedExpr, TypedExprKind, Type};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpsProgram {
    pub term: CpsTerm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpsAtom {
    Var(String),
    Lit(Literal),
    FunLambda {
        param: String,
        k_param: String,
        body: Box<CpsTerm>,
    },
    ContLambda {
        param: String,
        body: Box<CpsTerm>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpsTerm {
    Halt(CpsAtom),
    AppFun {
        func: CpsAtom,
        arg: CpsAtom,
        kont: CpsAtom,
    },
    AppCont {
        kont: CpsAtom,
        value: CpsAtom,
    },
    Prompt {
        multi: bool,
        body: Box<CpsTerm>,
    },
    Capture {
        multi: bool,
        binder: String,
        body: Box<CpsTerm>,
    },
    Branch {
        cond: CpsAtom,
        then_term: Box<CpsTerm>,
        else_term: Box<CpsTerm>,
        join: CpsAtom,
    },
    Match {
        scrutinee: CpsAtom,
        arms: Vec<CpsMatchArm>,
        join: CpsAtom,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpsMatchArm {
    pub pattern: Pattern,
    pub body: CpsTerm,
}

#[derive(Default)]
struct CpsCtx {
    next_id: usize,
}

impl CpsCtx {
    fn fresh(&mut self, prefix: &str) -> String {
        let id = self.next_id;
        self.next_id += 1;
        format!("{prefix}{id}")
    }
}

type MetaKont<'a> = Box<dyn FnOnce(CpsAtom, &mut CpsCtx) -> CpsTerm + 'a>;

pub fn cps_program(expr: &TypedExpr) -> CpsProgram {
    let mut ctx = CpsCtx::default();
    let term = transform(
        expr,
        Box::new(|value, _| CpsTerm::Halt(value)),
        Vec::new(),
        &mut ctx,
    );
    CpsProgram { term }
}

fn transform(
    expr: &TypedExpr,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    match &expr.kind {
        TypedExprKind::Var(name) => k(CpsAtom::Var(name.clone()), ctx),
        TypedExprKind::Lit(lit) => k(CpsAtom::Lit(lit.clone()), ctx),
        TypedExprKind::Lambda { param, body, .. } => {
            let k_param = ctx.fresh("k");
            let body_k = k_param.clone();
            let body = transform(
                body,
                Box::new(|value, _| CpsTerm::AppCont {
                    kont: CpsAtom::Var(body_k),
                    value,
                }),
                controls.clone(),
                ctx,
            );
            k(
                CpsAtom::FunLambda {
                    param: param.clone(),
                    k_param,
                    body: Box::new(body),
                },
                ctx,
            )
        }
        TypedExprKind::Call { callee, arg } => {
            transform_call(callee, arg, k, controls, ctx)
        }
        TypedExprKind::Field { receiver, name } => transform(
            receiver,
            Box::new(|value, ctx| k(CpsAtom::Var(format!("{value}.{name}")), ctx)),
            controls,
            ctx,
        ),
        TypedExprKind::MethodCall {
            receiver,
            name,
            arg,
        } => {
            let receiver_controls = controls.clone();
            let arg_controls = controls;
            transform(
                receiver,
                Box::new(|receiver, ctx| {
                    let arg_controls = arg_controls.clone();
                    transform(
                        arg,
                        Box::new(|arg, ctx| {
                            let w = ctx.fresh("w");
                            let kont_body = k(CpsAtom::Var(w.clone()), ctx);
                            CpsTerm::AppFun {
                                func: CpsAtom::Var(format!("{receiver}.{name}")),
                                arg,
                                kont: CpsAtom::ContLambda {
                                    param: w,
                                    body: Box::new(kont_body),
                                },
                            }
                        }),
                        arg_controls,
                        ctx,
                    )
                }),
                receiver_controls,
                ctx,
            )
        }
        TypedExprKind::Binary { op, lhs, rhs } => {
            let op_name = format!("operator::{op:?}");
            let lhs_controls = controls.clone();
            let rhs_controls = controls;
            transform(
                lhs,
                Box::new(|lhs, ctx| {
                    let rhs_controls = rhs_controls.clone();
                    transform(
                        rhs,
                        Box::new(|rhs, ctx| {
                            let w = ctx.fresh("w");
                            let kont_body = k(CpsAtom::Var(w.clone()), ctx);
                            CpsTerm::AppFun {
                                func: CpsAtom::Var(format!("{op_name}({lhs})")),
                                arg: rhs,
                                kont: CpsAtom::ContLambda {
                                    param: w,
                                    body: Box::new(kont_body),
                                },
                            }
                        }),
                        rhs_controls,
                        ctx,
                    )
                }),
                lhs_controls,
                ctx,
            )
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => transform_if(cond, then_branch, else_branch, k, controls, ctx),
        TypedExprKind::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            let arms = vec![
                crate::typed::TypedMatchArm {
                    pattern: pattern.clone(),
                    body: (*then_branch.clone()),
                },
                crate::typed::TypedMatchArm {
                    pattern: Pattern::Wildcard,
                    body: (*else_branch.clone()),
                },
            ];
            transform_match(scrutinee, &arms, k, controls, ctx)
        }
        TypedExprKind::Match { scrutinee, arms } => {
            transform_match(scrutinee, arms, k, controls, ctx)
        }
        TypedExprKind::Nominal { expr, .. } => {
            let _ = nominal_atom_type(expr);
            transform(expr, k, controls, ctx)
        }
        TypedExprKind::Reset { multi, body } => {
            let mut nested_controls = controls;
            nested_controls.push(if *multi {
                ContinuationKind::ContN
            } else {
                ContinuationKind::Cont1
            });
            let body = transform(body, k, nested_controls, ctx);
            CpsTerm::Prompt {
                multi: *multi,
                body: Box::new(body),
            }
        }
        TypedExprKind::Shift { binder, body } => {
            let kind = controls.last().copied().unwrap_or(ContinuationKind::Cont1);
            let body = transform(body, k, controls, ctx);
            CpsTerm::Capture {
                multi: kind == ContinuationKind::ContN,
                binder: binder.clone(),
                body: Box::new(body),
            }
        }
    }
}

fn transform_if(
    cond: &TypedExpr,
    then_branch: &TypedExpr,
    else_branch: &TypedExpr,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    let cond_controls = controls.clone();
    transform(
        cond,
        Box::new(|cond, ctx| {
            let join_param = ctx.fresh("join");
            let join_name = join_param.clone();
            let join = CpsAtom::ContLambda {
                param: join_param,
                body: Box::new(k(CpsAtom::Var(join_name.clone()), ctx)),
            };
            let then_join = join_name.clone();
            let else_join = join_name;
            let then_term = transform(
                then_branch,
                Box::new(move |value, _| CpsTerm::AppCont {
                    kont: CpsAtom::Var(then_join),
                    value,
                }),
                controls.clone(),
                ctx,
            );
            let else_term = transform(
                else_branch,
                Box::new(move |value, _| CpsTerm::AppCont {
                    kont: CpsAtom::Var(else_join),
                    value,
                }),
                controls,
                ctx,
            );
            CpsTerm::Branch {
                cond,
                then_term: Box::new(then_term),
                else_term: Box::new(else_term),
                join,
            }
        }),
        cond_controls,
        ctx,
    )
}

fn transform_match(
    scrutinee: &TypedExpr,
    arms: &[crate::typed::TypedMatchArm],
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    let scrutinee_controls = controls.clone();
    transform(
        scrutinee,
        Box::new(|scrutinee, ctx| {
            let join_param = ctx.fresh("join");
            let join_name = join_param.clone();
            let join = CpsAtom::ContLambda {
                param: join_param,
                body: Box::new(k(CpsAtom::Var(join_name.clone()), ctx)),
            };
            let mut cps_arms = Vec::new();
            for arm in arms {
                let arm_join = join_name.clone();
                let body = transform(
                    &arm.body,
                    Box::new(move |value, _| CpsTerm::AppCont {
                        kont: CpsAtom::Var(arm_join),
                        value,
                    }),
                    controls.clone(),
                    ctx,
                );
                cps_arms.push(CpsMatchArm {
                    pattern: arm.pattern.clone(),
                    body,
                });
            }
            CpsTerm::Match {
                scrutinee,
                arms: cps_arms,
                join,
            }
        }),
        scrutinee_controls,
        ctx,
    )
}

fn transform_call(
    callee: &TypedExpr,
    arg: &TypedExpr,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    let callee_controls = controls.clone();
    let arg_controls = controls;
    transform(
        callee,
        Box::new(|func, ctx| {
            let arg_controls = arg_controls.clone();
            transform(
                arg,
                Box::new(|arg, ctx| {
                    let w = ctx.fresh("w");
                    let kont_body = k(CpsAtom::Var(w.clone()), ctx);
                    CpsTerm::AppFun {
                        func,
                        arg,
                        kont: CpsAtom::ContLambda {
                            param: w,
                            body: Box::new(kont_body),
                        },
                    }
                }),
                arg_controls,
                ctx,
            )
        }),
        callee_controls,
        ctx,
    )
}

fn nominal_atom_type(expr: &TypedExpr) -> Option<&String> {
    match &expr.ty {
        Type::Nominal(name) => Some(name),
        _ => None,
    }
}

impl CpsProgram {
    pub fn contains_administrative_let_cont(&self) -> bool {
        false
    }
}

impl fmt::Display for CpsAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpsAtom::Var(name) => write!(f, "{name}"),
            CpsAtom::Lit(Literal::I64(value)) => write!(f, "{value}"),
            CpsAtom::Lit(Literal::Bool(value)) => write!(f, "{value}"),
            CpsAtom::FunLambda {
                param,
                k_param,
                body,
            } => {
                write!(f, "(lambda {param} {k_param}. {body})")
            }
            CpsAtom::ContLambda { param, body } => {
                write!(f, "(cont {param}. {body})")
            }
        }
    }
}

impl fmt::Display for CpsTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpsTerm::Halt(value) => write!(f, "halt {value}"),
            CpsTerm::AppCont { kont, value } => write!(f, "{kont}({value})"),
            CpsTerm::AppFun { func, arg, kont } => write!(f, "{func}({arg}, {kont})"),
            CpsTerm::Prompt { multi, body } => {
                if *multi {
                    write!(f, "resetn {{ {body} }}")
                } else {
                    write!(f, "reset {{ {body} }}")
                }
            }
            CpsTerm::Capture {
                multi,
                binder,
                body,
            } => {
                if *multi {
                    write!(f, "shift@contN {binder} {{ {body} }}")
                } else {
                    write!(f, "shift@cont1 {binder} {{ {body} }}")
                }
            }
            CpsTerm::Branch {
                cond,
                then_term,
                else_term,
                join,
            } => {
                write!(f, "if {cond} then {then_term} else {else_term} join {join}")
            }
            CpsTerm::Match {
                scrutinee,
                arms,
                join,
            } => {
                write!(f, "match {scrutinee} {{")?;
                for arm in arms {
                    write!(f, " {} => {}", display_pattern(&arm.pattern), arm.body)?;
                }
                write!(f, " }} join {join}")
            }
        }
    }
}

fn display_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Bind(name) => name.clone(),
        Pattern::Tuple(fields) => {
            let fields = fields
                .iter()
                .map(display_pattern)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({fields})")
        }
        Pattern::Lit(Literal::I64(value)) => value.to_string(),
        Pattern::Lit(Literal::Bool(value)) => value.to_string(),
    }
}

impl fmt::Display for CpsProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.term)
    }
}
