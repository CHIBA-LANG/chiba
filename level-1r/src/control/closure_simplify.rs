use std::collections::BTreeMap;

use crate::closure_core_usage::ClosureCoreUsageFacts;
use crate::core::CallableStorageKind;
use crate::usage::UseCount;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClosureSimplificationFacts {
    pub closure_packages: BTreeMap<String, ClosurePackageDecision>,
    pub env_fields: BTreeMap<String, EnvFieldDecision>,
    pub code_pointers: BTreeMap<String, CodePointerDecision>,
    pub continuation_packages: BTreeMap<String, ContinuationPackageDecision>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClosurePackageDecision {
    EraseNoCapture,
    DirectifySingleUse,
    KeepEnvPackage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnvFieldDecision {
    RemoveDeadField,
    KeepField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodePointerDecision {
    KnownDirectCall,
    KeepIndirectCodePointer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContinuationPackageDecision {
    RemoveUnusedPackage,
    KeepRepeatablePackage,
}

pub fn simplify_closure_core(
    usage: &ClosureCoreUsageFacts,
) -> ClosureSimplificationFacts {
    ClosureSimplificationFacts {
        closure_packages: simplify_closure_packages(usage),
        env_fields: simplify_env_fields(usage),
        code_pointers: simplify_code_pointers(usage),
        continuation_packages: simplify_continuation_packages(usage),
    }
}

fn simplify_closure_packages(
    usage: &ClosureCoreUsageFacts,
) -> BTreeMap<String, ClosurePackageDecision> {
    usage
        .closure_packages
        .iter()
        .map(|(subject, package)| {
            let decision = match (package.storage, package.count) {
                (CallableStorageKind::NoCaptureClosure, _) => {
                    ClosurePackageDecision::EraseNoCapture
                }
                (CallableStorageKind::EnvClosure, UseCount::One) => {
                    ClosurePackageDecision::DirectifySingleUse
                }
                (CallableStorageKind::EnvClosure, UseCount::Zero | UseCount::Many) => {
                    ClosurePackageDecision::KeepEnvPackage
                }
                (
                    CallableStorageKind::DirectFn
                    | CallableStorageKind::BoxedCont1
                    | CallableStorageKind::ContNPackage
                    | CallableStorageKind::ErasedCallableAdt,
                    _,
                ) => ClosurePackageDecision::KeepEnvPackage,
            };
            (subject.clone(), decision)
        })
        .collect()
}

fn simplify_env_fields(usage: &ClosureCoreUsageFacts) -> BTreeMap<String, EnvFieldDecision> {
    usage
        .env_fields
        .iter()
        .map(|(field, facts)| {
            let decision = match facts.color {
                crate::typed::UsageColor::One | crate::typed::UsageColor::Many => {
                    EnvFieldDecision::KeepField
                }
                crate::typed::UsageColor::Obligation => EnvFieldDecision::RemoveDeadField,
            };
            (field.clone(), decision)
        })
        .collect()
}

fn simplify_code_pointers(
    usage: &ClosureCoreUsageFacts,
) -> BTreeMap<String, CodePointerDecision> {
    usage
        .code_pointers
        .iter()
        .map(|(symbol, pointer)| {
            let decision = if pointer.known_direct && pointer.count == UseCount::One {
                CodePointerDecision::KnownDirectCall
            } else {
                CodePointerDecision::KeepIndirectCodePointer
            };
            (symbol.clone(), decision)
        })
        .collect()
}

fn simplify_continuation_packages(
    usage: &ClosureCoreUsageFacts,
) -> BTreeMap<String, ContinuationPackageDecision> {
    usage
        .continuation_packages
        .iter()
        .map(|(package, facts)| {
            let decision = match facts.count {
                UseCount::Zero => ContinuationPackageDecision::RemoveUnusedPackage,
                UseCount::One | UseCount::Many => {
                    ContinuationPackageDecision::KeepRepeatablePackage
                }
            };
            (package.clone(), decision)
        })
        .collect()
}
