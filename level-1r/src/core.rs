use crate::control::{ContinuationFact, ContinuationKind};
use crate::cps::{CpsAtom, CpsProgram, CpsTerm};
use crate::specialize::SpecializationFacts;
use crate::template::{DynRowContract, RowShape};
use crate::typed::{SendColor, UsageColor};
use crate::usage::UsageFacts;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreProgram {
    pub ops: Vec<CoreOp>,
    pub layouts: Vec<LayoutFact>,
    pub ownership: Vec<OwnershipFact>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutFact {
    pub key: String,
    pub hash: u64,
    pub kind: LayoutKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutKind {
    RowShape(RowShape),
    DynRowPackage(DynRowContract),
    ContinuationPackage(ContinuationKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnershipFact {
    pub subject: String,
    pub decision: OwnershipDecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OwnershipDecision {
    StackValue,
    InplaceReuse,
    Rc,
    Arc,
    StaticData,
    BorrowedView,
    DynPackage,
}

pub fn lower_core(cps: &CpsProgram, continuations: &[ContinuationFact]) -> CoreProgram {
    lower_core_with_facts(
        cps,
        continuations,
        &SpecializationFacts::default(),
        &UsageFacts::default(),
    )
}

pub fn lower_core_with_facts(
    cps: &CpsProgram,
    continuations: &[ContinuationFact],
    specialize: &SpecializationFacts,
    usage: &UsageFacts,
) -> CoreProgram {
    let mut ops = Vec::new();
    lower_term(&cps.term, continuations, &mut ops);
    let layouts = lower_layouts(continuations, specialize);
    let ownership = lower_ownership(continuations, specialize, usage);
    CoreProgram {
        ops,
        layouts,
        ownership,
    }
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

fn lower_layouts(
    continuations: &[ContinuationFact],
    specialize: &SpecializationFacts,
) -> Vec<LayoutFact> {
    let mut layouts = Vec::new();
    for item in &specialize.work_items {
        for shape in &item.key.normalized_shapes {
            let key = format!("row::{shape:?}");
            layouts.push(LayoutFact {
                hash: stable_hash(&key),
                key,
                kind: LayoutKind::RowShape(shape.clone()),
            });
        }
        for contract in &item.key.dyn_contracts {
            let key = format!("dyn-row::{contract:?}");
            layouts.push(LayoutFact {
                hash: stable_hash(&key),
                key,
                kind: LayoutKind::DynRowPackage(contract.clone()),
            });
        }
    }
    for fact in continuations {
        if fact.kind == ContinuationKind::ContN {
            let key = format!("continuation::{:?}::{}", fact.kind, fact.binder);
            layouts.push(LayoutFact {
                hash: stable_hash(&key),
                key,
                kind: LayoutKind::ContinuationPackage(fact.kind),
            });
        }
    }
    layouts
}

fn lower_ownership(
    continuations: &[ContinuationFact],
    specialize: &SpecializationFacts,
    usage: &UsageFacts,
) -> Vec<OwnershipFact> {
    let mut facts = Vec::new();
    for (name, count) in &usage.vars {
        facts.push(OwnershipFact {
            subject: format!("var::{name}"),
            decision: ownership_from_usage(count.color(), SendColor::Obligation),
        });
    }
    for (binder, count) in &usage.binders {
        facts.push(OwnershipFact {
            subject: format!("binder::{binder}"),
            decision: ownership_from_usage(count.color(), SendColor::Obligation),
        });
    }
    for item in &specialize.work_items {
        for contract in &item.key.dyn_contracts {
            facts.push(OwnershipFact {
                subject: format!("dyn::{:?}", contract.shape),
                decision: OwnershipDecision::DynPackage,
            });
            facts.push(OwnershipFact {
                subject: format!("dyn-payload::{:?}", contract.shape),
                decision: ownership_from_usage(contract.payload_usage, contract.send),
            });
        }
    }
    for fact in continuations {
        facts.push(OwnershipFact {
            subject: format!("continuation::{}", fact.binder),
            decision: match fact.kind {
                ContinuationKind::Cont1 => OwnershipDecision::StackValue,
                ContinuationKind::ContN => OwnershipDecision::DynPackage,
            },
        });
    }
    facts
}

fn ownership_from_usage(usage: UsageColor, send: SendColor) -> OwnershipDecision {
    match (usage, send) {
        (UsageColor::One, _) => OwnershipDecision::StackValue,
        (UsageColor::Many, SendColor::Send) => OwnershipDecision::Arc,
        (UsageColor::Many, SendColor::NotSend | SendColor::Obligation) => OwnershipDecision::Rc,
        (UsageColor::Obligation, _) => OwnershipDecision::BorrowedView,
    }
}

fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
