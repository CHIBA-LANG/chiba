use std::fmt;

use crate::ast::Literal;
use crate::control::ContinuationKind;
use crate::typed::{TypedExpr, TypedExprKind};

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
        }
    }
}

impl fmt::Display for CpsProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.term)
    }
}
