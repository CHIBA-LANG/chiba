use std::collections::BTreeMap;

use crate::control::ContinuationKind;
use crate::core::{CallableStorageKind, CoreOp, CoreProgram, LayoutKind};
use crate::typed::{SendColor, UsageColor};
use crate::usage::UseCount;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClosureCoreUsageFacts {
    pub closure_packages: BTreeMap<String, ClosurePackageUsage>,
    pub env_fields: BTreeMap<String, EnvFieldUsage>,
    pub code_pointers: BTreeMap<String, CodePointerUsage>,
    pub continuation_packages: BTreeMap<String, ContinuationPackageUsage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosurePackageUsage {
    pub storage: CallableStorageKind,
    pub count: UseCount,
    pub send: SendColor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvFieldUsage {
    pub closure: String,
    pub field: String,
    pub color: UsageColor,
    pub send: SendColor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodePointerUsage {
    pub source: String,
    pub count: UseCount,
    pub known_direct: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationPackageUsage {
    pub kind: ContinuationKind,
    pub count: UseCount,
}

pub fn analyze_closure_core_usage(core: &CoreProgram) -> ClosureCoreUsageFacts {
    let mut facts = ClosureCoreUsageFacts::default();
    collect_callable_storage(core, &mut facts);
    collect_layout_usage(core, &mut facts);
    collect_lifted_code_pointers(core, &mut facts);
    facts
}

fn collect_callable_storage(core: &CoreProgram, facts: &mut ClosureCoreUsageFacts) {
    for storage in &core.callable_storage {
        match storage.kind {
            CallableStorageKind::NoCaptureClosure | CallableStorageKind::EnvClosure => {
                facts.closure_packages.insert(
                    storage.subject.clone(),
                    ClosurePackageUsage {
                        storage: storage.kind,
                        count: use_count_from_color(storage.usage),
                        send: storage.send,
                    },
                );
            }
            CallableStorageKind::DirectFn
            | CallableStorageKind::BoxedCont1
            | CallableStorageKind::ContNPackage
            | CallableStorageKind::ErasedCallableAdt => {}
        }
    }
}

fn collect_layout_usage(core: &CoreProgram, facts: &mut ClosureCoreUsageFacts) {
    for layout in &core.layouts {
        match &layout.kind {
            LayoutKind::ClosureEnv(env) => {
                for field in &env.fields {
                    let key = format!("{}.{}", env.closure, field.name);
                    facts.env_fields.insert(
                        key,
                        EnvFieldUsage {
                            closure: env.closure.clone(),
                            field: field.name.clone(),
                            color: field.usage,
                            send: field.send,
                        },
                    );
                }
            }
            LayoutKind::ContinuationPackage(kind) => {
                facts.continuation_packages.insert(
                    layout.key.clone(),
                    ContinuationPackageUsage {
                        kind: *kind,
                        count: match kind {
                            ContinuationKind::Cont1 => UseCount::One,
                            ContinuationKind::ContN => UseCount::Many,
                        },
                    },
                );
            }
            LayoutKind::RowShape(_) | LayoutKind::DynRowPackage(_) | LayoutKind::TupleStruct(_) => {}
        }
    }
}

fn collect_lifted_code_pointers(core: &CoreProgram, facts: &mut ClosureCoreUsageFacts) {
    for op in &core.ops {
        if let CoreOp::LiftedFunction {
            source,
            symbol,
            direct,
            ..
        } = op
        {
            facts.code_pointers.insert(
                symbol.clone(),
                CodePointerUsage {
                    source: source.clone(),
                    count: if *direct {
                        UseCount::One
                    } else {
                        UseCount::Many
                    },
                    known_direct: *direct,
                },
            );
        }
    }
}

fn use_count_from_color(color: UsageColor) -> UseCount {
    match color {
        UsageColor::One => UseCount::One,
        UsageColor::Many => UseCount::Many,
        UsageColor::Obligation => UseCount::Many,
    }
}
