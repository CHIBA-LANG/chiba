use crate::control::{ContinuationFact, ContinuationKind};
use crate::cps::{CpsAtom, CpsProgram, CpsTerm};
use crate::closure::{ClosureFacts, ClosureStorageKind};
use crate::specialize::{DischargedObligation, SpecializationFacts};
use crate::template::{DynRowContract, RowShape};
use crate::typed::{SendColor, UsageColor};
use crate::usage::UsageFacts;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreProgram {
    pub ops: Vec<CoreOp>,
    pub layouts: Vec<LayoutFact>,
    pub ownership: Vec<OwnershipFact>,
    pub callable_storage: Vec<CallableStorageFact>,
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
    DirectMethodTarget {
        name: String,
        target: String,
    },
    OperatorTarget {
        protocol: String,
        target: String,
    },
    DynRowAdapterAccess {
        subject: String,
        layout: String,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallableStorageFact {
    pub subject: String,
    pub kind: CallableStorageKind,
    pub usage: UsageColor,
    pub send: SendColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CallableStorageKind {
    DirectFn,
    NoCaptureClosure,
    EnvClosure,
    BoxedCont1,
    ContNPackage,
    ErasedCallableAdt,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoreValidation {
    pub diagnostics: Vec<CoreDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreDiagnostic {
    TargetSpecificTerm { term: String },
    LayoutHashMismatch {
        key: String,
        expected: u64,
        actual: u64,
    },
    DuplicateLayoutKey { key: String },
    MissingContNPackage { binder: String },
    Cont1HasPackageLayout { key: String },
    MissingDynRowLayout { layout: String },
    DynRowLayoutKindMismatch { layout: String },
    SendableCallableContainsContinuation { subject: String },
    SharedSendSubjectUsesRc { subject: String },
    DynPayloadMustUseDynPackage { subject: String },
}

pub fn lower_core(cps: &CpsProgram, continuations: &[ContinuationFact]) -> CoreProgram {
    lower_core_with_facts(
        cps,
        continuations,
        &ClosureFacts::default(),
        &SpecializationFacts::default(),
        &UsageFacts::default(),
    )
}

pub fn lower_core_with_facts(
    cps: &CpsProgram,
    continuations: &[ContinuationFact],
    closures: &ClosureFacts,
    specialize: &SpecializationFacts,
    usage: &UsageFacts,
) -> CoreProgram {
    let mut ops = Vec::new();
    lower_term(&cps.term, continuations, &mut ops);
    let layouts = lower_layouts(continuations, specialize);
    let ownership = lower_ownership(continuations, specialize, usage);
    let callable_storage = lower_callable_storage(continuations, closures);
    lower_specialization_ops(specialize, &layouts, &mut ops);
    CoreProgram {
        ops,
        layouts,
        ownership,
        callable_storage,
    }
}

pub fn validate_core(program: &CoreProgram) -> CoreValidation {
    let mut diagnostics = Vec::new();
    validate_target_neutral(program, &mut diagnostics);
    validate_layouts(program, &mut diagnostics);
    validate_continuation_packages(program, &mut diagnostics);
    validate_dyn_adapter_layouts(program, &mut diagnostics);
    validate_callable_storage(program, &mut diagnostics);
    validate_ownership(program, &mut diagnostics);
    CoreValidation { diagnostics }
}

impl CoreValidation {
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
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

fn lower_specialization_ops(
    specialize: &SpecializationFacts,
    layouts: &[LayoutFact],
    ops: &mut Vec<CoreOp>,
) {
    for item in &specialize.work_items {
        for obligation in &item.obligations {
            match obligation {
                DischargedObligation::Method {
                    name,
                    target: Some(target),
                } => ops.push(CoreOp::DirectMethodTarget {
                    name: name.clone(),
                    target: target.clone(),
                }),
                DischargedObligation::Operator {
                    protocol,
                    receiver: Some(receiver),
                } => ops.push(CoreOp::OperatorTarget {
                    protocol: protocol.clone(),
                    target: format!("{receiver}.{protocol}"),
                }),
                DischargedObligation::DynAdapter { contract } => {
                    let layout = layouts
                        .iter()
                        .find(|layout| {
                            matches!(&layout.kind, LayoutKind::DynRowPackage(candidate) if candidate == contract)
                        })
                        .map(|layout| layout.key.clone())
                        .unwrap_or_else(|| format!("dyn-row::{contract:?}"));
                    ops.push(CoreOp::DynRowAdapterAccess {
                        subject: format!("{:?}", contract.shape),
                        layout,
                    });
                }
                DischargedObligation::Field { .. }
                | DischargedObligation::Method { target: None, .. }
                | DischargedObligation::Operator { receiver: None, .. } => {}
            }
        }
    }
}

fn validate_target_neutral(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    const FORBIDDEN: [&str; 5] = ["Wasm", "WAT", "Binaryen", "funcref", "eqref"];
    let rendered = format!("{program:#?}");
    for term in FORBIDDEN {
        if rendered.contains(term) {
            diagnostics.push(CoreDiagnostic::TargetSpecificTerm {
                term: term.to_string(),
            });
        }
    }
}

fn validate_layouts(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    let mut keys = Vec::new();
    for layout in &program.layouts {
        let expected = stable_hash(&layout.key);
        if layout.hash != expected {
            diagnostics.push(CoreDiagnostic::LayoutHashMismatch {
                key: layout.key.clone(),
                expected,
                actual: layout.hash,
            });
        }
        if keys.contains(&layout.key) {
            diagnostics.push(CoreDiagnostic::DuplicateLayoutKey {
                key: layout.key.clone(),
            });
        } else {
            keys.push(layout.key.clone());
        }
        if matches!(layout.kind, LayoutKind::ContinuationPackage(ContinuationKind::Cont1)) {
            diagnostics.push(CoreDiagnostic::Cont1HasPackageLayout {
                key: layout.key.clone(),
            });
        }
    }
}

fn validate_continuation_packages(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        if let CoreOp::CaptureContinuation {
            binder,
            kind: ContinuationKind::ContN,
        } = op
        {
            let has_package = program.layouts.iter().any(|layout| {
                matches!(
                    layout.kind,
                    LayoutKind::ContinuationPackage(ContinuationKind::ContN)
                ) && layout.key.contains(binder)
            });
            if !has_package {
                diagnostics.push(CoreDiagnostic::MissingContNPackage {
                    binder: binder.clone(),
                });
            }
        }
    }
}

fn validate_dyn_adapter_layouts(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        if let CoreOp::DynRowAdapterAccess { layout, .. } = op {
            match program.layouts.iter().find(|fact| fact.key == *layout) {
                Some(fact) if matches!(fact.kind, LayoutKind::DynRowPackage(_)) => {}
                Some(_) => diagnostics.push(CoreDiagnostic::DynRowLayoutKindMismatch {
                    layout: layout.clone(),
                }),
                None => diagnostics.push(CoreDiagnostic::MissingDynRowLayout {
                    layout: layout.clone(),
                }),
            }
        }
    }
}

fn validate_callable_storage(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for fact in &program.callable_storage {
        let continuation_variant = matches!(
            fact.kind,
            CallableStorageKind::BoxedCont1 | CallableStorageKind::ContNPackage
        );
        if fact.send == SendColor::Send && continuation_variant {
            diagnostics.push(CoreDiagnostic::SendableCallableContainsContinuation {
                subject: fact.subject.clone(),
            });
        }
    }
}

fn validate_ownership(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for fact in &program.ownership {
        if fact.subject.contains("send") && fact.decision == OwnershipDecision::Rc {
            diagnostics.push(CoreDiagnostic::SharedSendSubjectUsesRc {
                subject: fact.subject.clone(),
            });
        }
        if fact.subject.starts_with("dyn::") && fact.decision != OwnershipDecision::DynPackage {
            diagnostics.push(CoreDiagnostic::DynPayloadMustUseDynPackage {
                subject: fact.subject.clone(),
            });
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

fn lower_callable_storage(
    continuations: &[ContinuationFact],
    closures: &ClosureFacts,
) -> Vec<CallableStorageFact> {
    let mut facts = Vec::new();
    facts.push(CallableStorageFact {
        subject: "top-level-def".to_string(),
        kind: CallableStorageKind::DirectFn,
        usage: UsageColor::Many,
        send: SendColor::Send,
    });
    for closure in &closures.closures {
        let has_captures = !closure.captures.is_empty();
        facts.push(CallableStorageFact {
            subject: format!("closure::{}", closure.param),
            kind: match closure.storage {
                ClosureStorageKind::DirectNoCapture => CallableStorageKind::NoCaptureClosure,
                ClosureStorageKind::EnvClosure => CallableStorageKind::EnvClosure,
            },
            usage: if has_captures {
                UsageColor::Many
            } else {
                UsageColor::One
            },
            send: if has_captures {
                SendColor::Obligation
            } else {
                SendColor::Send
            },
        });
    }
    for fact in continuations {
        facts.push(CallableStorageFact {
            subject: format!("continuation::{}", fact.binder),
            kind: match fact.kind {
                ContinuationKind::Cont1 => CallableStorageKind::BoxedCont1,
                ContinuationKind::ContN => CallableStorageKind::ContNPackage,
            },
            usage: fact.usage,
            send: SendColor::NotSend,
        });
    }
    if !continuations.is_empty() || !closures.closures.is_empty() {
        facts.push(CallableStorageFact {
            subject: "callable-storage::erased".to_string(),
            kind: CallableStorageKind::ErasedCallableAdt,
            usage: UsageColor::Many,
            send: SendColor::Obligation,
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
