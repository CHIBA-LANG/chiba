use crate::control::{ContinuationFact, ContinuationKind};
use crate::cps::{CpsAtom, CpsProgram, CpsTerm};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreProgram {
    pub ops: Vec<CoreOp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreOp {
    ReturnAtom(String),
    TailCall {
        func: String,
        arg: String,
    },
    Prompt {
        kind: ContinuationKind,
    },
    CaptureContinuation {
        binder: String,
        kind: ContinuationKind,
    },
}

pub fn lower_core(cps: &CpsProgram, continuations: &[ContinuationFact]) -> CoreProgram {
    let mut ops = Vec::new();
    lower_term(&cps.term, continuations, &mut ops);
    CoreProgram { ops }
}

fn lower_term(term: &CpsTerm, continuations: &[ContinuationFact], ops: &mut Vec<CoreOp>) {
    match term {
        CpsTerm::Halt(atom) => ops.push(CoreOp::ReturnAtom(render_atom(atom))),
        CpsTerm::AppCont { value, .. } => ops.push(CoreOp::ReturnAtom(render_atom(value))),
        CpsTerm::AppFun { func, arg, .. } => ops.push(CoreOp::TailCall {
            func: render_atom(func),
            arg: render_atom(arg),
        }),
        CpsTerm::Prompt { multi, body } => {
            ops.push(CoreOp::Prompt {
                kind: if *multi {
                    ContinuationKind::ContN
                } else {
                    ContinuationKind::Cont1
                },
            });
            lower_term(body, continuations, ops);
        }
        CpsTerm::Capture { binder, body, .. } => {
            let kind = continuations
                .iter()
                .find(|fact| fact.binder == *binder)
                .map(|fact| fact.kind)
                .unwrap_or(ContinuationKind::Cont1);
            ops.push(CoreOp::CaptureContinuation {
                binder: binder.clone(),
                kind,
            });
            lower_term(body, continuations, ops);
        }
    }
}

fn render_atom(atom: &CpsAtom) -> String {
    match atom {
        CpsAtom::Var(name) => name.clone(),
        CpsAtom::Lit(lit) => format!("{lit:?}"),
        CpsAtom::FunLambda { param, .. } => format!("lambda#{param}"),
        CpsAtom::ContLambda { param, .. } => format!("cont#{param}"),
    }
}
