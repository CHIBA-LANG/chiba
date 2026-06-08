use std::collections::{BTreeMap, BTreeSet};

use crate::control::ContinuationKind;
use crate::core::{
    CoreCapturedContinuation, CoreExternAbi, CoreMatchArm, CoreOp, CorePattern, CoreProgram,
    CoreRefCellLane, CoreValidation, CoreValue, OperatorIntrinsic, OwnershipDecision, RangeField,
    SliceField, TextField,
};
use crate::symbol::encode_debug_symbol;
use crate::typed::{AggregateKind, BuiltinMethodCall, TextKind, Type};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackendArtifact {
    pub target: BackendTarget,
    pub wat: String,
    pub manifest: BackendManifest,
    pub diagnostics: Vec<BackendDiagnostic>,
    pub return_value: Option<CoreValue>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackendLinkedBundle {
    pub target: BackendTarget,
    pub linked_wat: String,
    pub manifest: BackendManifest,
    pub diagnostics: Vec<BackendLinkDiagnostic>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BackendTarget {
    #[default]
    WasmGc,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackendManifest {
    pub entries: Vec<BackendManifestEntry>,
    pub imports: Vec<BackendExternImport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendManifestEntry {
    pub final_symbol: String,
    pub source_debug_name: String,
    pub pass_origin: String,
    pub ownership: Option<OwnershipDecision>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendDiagnostic {
    CoreValidationFailed {
        diagnostics: usize,
    },
    UnsupportedI32ReturnValue {
        value: String,
    },
    UnsupportedExternImportSignature {
        symbol: String,
        signature: String,
    },
    Cont1ResumedMoreThanOnce {
        binder: String,
    },
    UnsupportedContinuationRuntime {
        op: String,
        kind: ContinuationKind,
        binder: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendLinkDiagnostic {
    ArtifactEmitFailed {
        artifact_index: usize,
    },
    DuplicateFinalSymbol {
        symbol: String,
    },
    UnsupportedStaticInitializerLowering {
        static_name: String,
        expr: String,
    },
    TargetMismatch {
        artifact_index: usize,
        expected: BackendTarget,
        actual: BackendTarget,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackendParamAbi {
    pub params: BTreeMap<String, BackendValueKind>,
    pub functions: BTreeMap<String, BackendCallableAbi>,
    pub callable_params: BTreeMap<String, BackendCallableAbi>,
    pub statics: BTreeMap<String, BackendValueKind>,
    pub static_ref_cell_lanes: BTreeMap<String, CoreRefCellLane>,
    pub aggregate_element_lanes: BTreeMap<String, BackendValueKind>,
    pub dyn_row_param_fields: BTreeMap<String, Vec<BackendDynRowParamFieldAbi>>,
    pub dyn_row_param_methods: BTreeMap<String, Vec<BackendDynRowParamMethodAbi>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackendCallableAbi {
    pub params: Vec<BackendValueKind>,
    pub arg_expansions: Vec<BackendCallableArgExpansion>,
    pub result: Option<BackendValueKind>,
    pub result_ref_cell_lane: Option<CoreRefCellLane>,
    pub return_callable: Option<BackendReturnedCallableAbi>,
    pub return_dyn_row_methods: Vec<BackendDynRowParamMethodAbi>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendCallableArgExpansion {
    Direct(BackendValueKind),
    Callable {
        params: Vec<BackendValueKind>,
        env: Vec<BackendValueKind>,
        result: Option<BackendValueKind>,
    },
    DynRowFields(Vec<BackendDynRowParamFieldAbi>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendReturnedCallableAbi {
    pub target: String,
    pub env: Vec<BackendValueKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendDynRowParamFieldAbi {
    pub field: String,
    pub kind: BackendValueKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendDynRowParamMethodAbi {
    pub field: String,
    pub target: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackendValueKind {
    I32,
    ExternRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendCacheConfig {
    pub compiler_version: String,
    pub features: BackendTargetFeatures,
    pub ownership_runtime: BackendOwnershipRuntime,
    pub imports: Vec<BackendExternImport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendTargetFeatures {
    pub tailcall: bool,
    pub wasi: bool,
    pub thread: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackendOwnershipRuntime {
    WasmGc,
    RcArcHelpers,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendExternImport {
    pub abi: BackendExternAbi,
    pub final_symbol: String,
    pub module: String,
    pub name: String,
    pub signature_hash: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackendExternAbi {
    Wasi,
    C,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendCacheKey {
    pub target: BackendTarget,
    pub digest: String,
}

impl Default for BackendCacheConfig {
    fn default() -> Self {
        Self {
            compiler_version: "level-1r-baseline".to_string(),
            features: BackendTargetFeatures {
                tailcall: true,
                wasi: true,
                thread: false,
            },
            ownership_runtime: BackendOwnershipRuntime::WasmGc,
            imports: vec![],
        }
    }
}

pub fn emit_wasm_gc(core: &CoreProgram, validation: &CoreValidation) -> BackendArtifact {
    emit_wasm_gc_with_params(core, validation, &[])
}

pub fn emit_wasm_gc_with_params(
    core: &CoreProgram,
    validation: &CoreValidation,
    params: &[String],
) -> BackendArtifact {
    emit_wasm_gc_with_param_abi(core, validation, params, &BackendParamAbi::default())
}

pub fn emit_wasm_gc_with_param_abi(
    core: &CoreProgram,
    validation: &CoreValidation,
    params: &[String],
    param_abi: &BackendParamAbi,
) -> BackendArtifact {
    if !validation.is_ok() {
        return BackendArtifact {
            target: BackendTarget::WasmGc,
            wat: String::new(),
            manifest: BackendManifest::default(),
            diagnostics: vec![BackendDiagnostic::CoreValidationFailed {
                diagnostics: validation.diagnostics.len(),
            }],
            return_value: None,
        };
    }

    let param_kinds = infer_param_kinds(core, params, param_abi);
    let manifest = manifest_for_core(core, params, &param_kinds, param_abi);
    if let Some(diagnostic) = unsupported_extern_import_signature(&manifest) {
        return BackendArtifact {
            target: BackendTarget::WasmGc,
            wat: String::new(),
            manifest,
            diagnostics: vec![diagnostic],
            return_value: first_return_value(core),
        };
    }

    if let Some(diagnostic) =
        unsupported_runtime_return_value(core, params, &param_kinds, param_abi)
    {
        return BackendArtifact {
            target: BackendTarget::WasmGc,
            wat: String::new(),
            manifest,
            diagnostics: vec![diagnostic],
            return_value: first_return_value(core),
        };
    }
    let wat = match render_wat(core, &manifest, params, &param_kinds, param_abi) {
        Ok(wat) => wat,
        Err(diagnostic) => {
            return BackendArtifact {
                target: BackendTarget::WasmGc,
                wat: String::new(),
                manifest,
                diagnostics: vec![diagnostic],
                return_value: first_return_value(core),
            };
        }
    };

    BackendArtifact {
        target: BackendTarget::WasmGc,
        wat,
        manifest,
        diagnostics: vec![],
        return_value: first_return_value(core),
    }
}

fn unsupported_runtime_return_value(
    core: &CoreProgram,
    params: &[String],
    param_kinds: &BTreeMap<String, WasmValueKind>,
    param_abi: &BackendParamAbi,
) -> Option<BackendDiagnostic> {
    let mut env = env_with_tailcall_result_facts(
        core,
        &RenderEnv::from_param_abi(params, param_kinds, param_abi).with_lifted_function_abis(core),
    );
    let mut continuations = BTreeSet::new();
    for (index, op) in core.ops.iter().enumerate() {
        if let Some(diagnostic) = match op {
            CoreOp::ReturnValue(value)
                if return_value_is_tailcall_result(core, index, value, &env) =>
            {
                None
            }
            CoreOp::ReturnValue(value) => unsupported_runtime_value(value, &env),
            CoreOp::RuntimeLet { binder, value } => {
                if let Some(diagnostic) = unsupported_runtime_value(value, &env) {
                    Some(diagnostic)
                } else {
                    env = env.with_local_kind(binder, runtime_local_value_kind(value, &env));
                    None
                }
            }
            CoreOp::ReturnBranch {
                cond,
                then_value,
                else_value,
            } => [cond, then_value, else_value]
                .into_iter()
                .find_map(|value| unsupported_i32_value(value, &env)),
            CoreOp::ReturnMatch { scrutinee, arms } => unsupported_i32_match(scrutinee, arms, &env),
            CoreOp::TailCall { func, args }
                if continuations.contains(effective_tailcall(core, func, args, &env).0) =>
            {
                let (runtime_func, _) = effective_tailcall(core, func, args, &env);
                let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
                    return Some(BackendDiagnostic::UnsupportedContinuationRuntime {
                        op: "resume-without-result".to_string(),
                        kind: continuation_kind_for_binder(core, runtime_func),
                        binder: Some(runtime_func.to_string()),
                    });
                };
                env = env.with_local_kind(
                    binder,
                    tailcall_result_kind(core, runtime_func, &env).unwrap_or(WasmValueKind::I32),
                );
                None
            }
            CoreOp::TailCall { func, args } => {
                let (runtime_func, runtime_args) = effective_tailcall(core, func, args, &env);
                let diagnostic = unsupported_tailcall_args(core, runtime_func, &runtime_args, &env);
                if diagnostic.is_none() {
                    if let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) {
                        env = env.with_local_kind(
                            binder,
                            tailcall_result_kind(core, runtime_func, &env)
                                .unwrap_or(WasmValueKind::I32),
                        );
                    }
                }
                diagnostic
            }
            CoreOp::TailCallResult { binder } => {
                env = env.with_locals(vec![binder.clone()]);
                None
            }
            CoreOp::CaptureContinuation { binder, .. } => {
                continuations.insert(binder.clone());
                None
            }
            CoreOp::LiftedFunction {
                env_params,
                param,
                body,
                ..
            } => {
                unsupported_lifted_function_runtime_value(env_params, param.as_deref(), body, &env)
            }
            _ => None,
        } {
            return Some(diagnostic);
        }
    }
    None
}

fn unsupported_lifted_function_runtime_value(
    env_params: &[String],
    param: Option<&str>,
    body: &[CoreOp],
    env: &RenderEnv,
) -> Option<BackendDiagnostic> {
    let param = param?;
    let body_core = CoreProgram {
        ops: body.to_vec(),
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    let mut names = env_params.to_vec();
    names.push(param.to_string());
    let mut local_env = env.with_params_as_locals(names);
    for env_param in env_params {
        local_env = local_env.with_local_kind(env_param, WasmValueKind::I32);
    }
    local_env = local_env.with_local_kind(param, WasmValueKind::I32);
    for op in body {
        match op {
            CoreOp::ReturnValue(value) => {
                if let Some(diagnostic) = unsupported_runtime_value(value, &local_env) {
                    return Some(diagnostic);
                }
            }
            CoreOp::TailCall { func, args } => {
                let (runtime_func, runtime_args) =
                    effective_tailcall(&body_core, func, args, &local_env);
                if let Some(diagnostic) =
                    unsupported_tailcall_args(&body_core, runtime_func, &runtime_args, &local_env)
                {
                    return Some(diagnostic);
                }
            }
            CoreOp::TailCallResult { binder } => {
                local_env = local_env.with_local_kind(binder, WasmValueKind::I32);
            }
            _ => {}
        }
    }
    None
}

fn unsupported_runtime_value(value: &CoreValue, env: &RenderEnv) -> Option<BackendDiagnostic> {
    if core_value_is_renderable_i32(value, env)
        || core_value_is_renderable_externref(value, env)
        || core_value_is_renderable_callable_return(value, env)
    {
        None
    } else {
        Some(BackendDiagnostic::UnsupportedI32ReturnValue {
            value: value.debug_name(),
        })
    }
}

fn core_value_is_renderable_callable_return(value: &CoreValue, env: &RenderEnv) -> bool {
    if !matches!(value, CoreValue::LiftedFunction { .. }) {
        return false;
    }
    let env_params = callable_return_env_params(value, env);
    callable_selector_for_value(
        value,
        &[BackendValueKind::I32],
        &env_params,
        Some(BackendValueKind::I32),
        env,
    )
    .is_some()
        && callable_env_values(value, &env_params, env).is_some()
}

fn callable_return_env_params(value: &CoreValue, env: &RenderEnv) -> Vec<BackendValueKind> {
    let CoreValue::LiftedFunction { source, .. } = value else {
        return Vec::new();
    };
    env.closure_env_params(source)
        .map(|params| vec![BackendValueKind::I32; params.len()])
        .unwrap_or_default()
}

fn return_value_is_tailcall_result(
    core: &CoreProgram,
    index: usize,
    value: &CoreValue,
    env: &RenderEnv,
) -> bool {
    let CoreValue::Var(name) = value else {
        return false;
    };
    if matches!(
        index.checked_sub(1).and_then(|previous| core.ops.get(previous)),
        Some(CoreOp::TailCall { func, args }) if {
            let (runtime_func, runtime_args) = effective_tailcall(core, func, args, env);
            tailcall_args_are_renderable(core, runtime_func, &runtime_args, env)
        }
    ) {
        return true;
    }
    core.ops.windows(2).any(|window| {
        matches!(
            window,
            [
                CoreOp::TailCall { func, args },
                CoreOp::TailCallResult { binder },
            ] if binder == name && {
                let (runtime_func, runtime_args) = effective_tailcall(core, func, args, env);
                tailcall_args_are_renderable(core, runtime_func, &runtime_args, env)
            }
        )
    })
}

fn unsupported_i32_value(value: &CoreValue, env: &RenderEnv) -> Option<BackendDiagnostic> {
    (!core_value_is_renderable_i32(value, env)).then(|| {
        BackendDiagnostic::UnsupportedI32ReturnValue {
            value: value.debug_name(),
        }
    })
}

fn unsupported_tailcall_args(
    core: &CoreProgram,
    func: &str,
    args: &[CoreValue],
    env: &RenderEnv,
) -> Option<BackendDiagnostic> {
    if let Some(callable) = env.callable_param(func) {
        return unsupported_callable_param_tailcall_args(func, callable, args, env);
    }
    if let Some(expansions) = callable_arg_expansions_for_target(func, env)
        .filter(|expansions| expansions.len() == args.len())
    {
        return expansions
            .iter()
            .zip(args)
            .find_map(|(expansion, arg)| unsupported_expanded_tailcall_arg(expansion, arg, env));
    }
    let Some((params, _)) = callable_signature_for_target(core, func, env) else {
        return args.iter().find_map(|arg| unsupported_i32_value(arg, env));
    };
    if params.len() != args.len() {
        return Some(BackendDiagnostic::UnsupportedI32ReturnValue {
            value: format!("{} /{}", final_symbol(func), args.len()),
        });
    }
    params.iter().zip(args).find_map(|(kind, arg)| {
        let renderable = match kind {
            WasmValueKind::I32 => core_value_is_renderable_i32(arg, env),
            WasmValueKind::ExternRef => core_value_is_renderable_externref(arg, env),
        };
        (!renderable).then(|| BackendDiagnostic::UnsupportedI32ReturnValue {
            value: arg.debug_name(),
        })
    })
}

fn unsupported_callable_param_tailcall_args(
    func: &str,
    callable: &BackendCallableAbi,
    args: &[CoreValue],
    env: &RenderEnv,
) -> Option<BackendDiagnostic> {
    if callable.params.len() < args.len() {
        return Some(BackendDiagnostic::UnsupportedI32ReturnValue {
            value: format!("{} /{}", final_symbol(func), args.len()),
        });
    }
    let call_param_kinds = &callable.params[callable.params.len() - args.len()..];
    call_param_kinds.iter().zip(args).find_map(|(kind, arg)| {
        let renderable = match WasmValueKind::from(*kind) {
            WasmValueKind::I32 => core_value_is_renderable_i32(arg, env),
            WasmValueKind::ExternRef => core_value_is_renderable_externref(arg, env),
        };
        (!renderable).then(|| BackendDiagnostic::UnsupportedI32ReturnValue {
            value: arg.debug_name(),
        })
    })
}

fn unsupported_expanded_tailcall_arg(
    expansion: &BackendCallableArgExpansion,
    arg: &CoreValue,
    env: &RenderEnv,
) -> Option<BackendDiagnostic> {
    match expansion {
        BackendCallableArgExpansion::Direct(kind) => {
            let renderable = match WasmValueKind::from(*kind) {
                WasmValueKind::I32 => core_value_is_renderable_i32(arg, env),
                WasmValueKind::ExternRef => core_value_is_renderable_externref(arg, env),
            };
            (!renderable).then(|| BackendDiagnostic::UnsupportedI32ReturnValue {
                value: arg.debug_name(),
            })
        }
        BackendCallableArgExpansion::Callable {
            params,
            env: callable_env,
            result,
        } => (!callable_selector_for_value(arg, params, callable_env, *result, env).is_some()
            || callable_env_values(arg, callable_env, env).is_none())
        .then(|| BackendDiagnostic::UnsupportedI32ReturnValue {
            value: arg.debug_name(),
        }),
        BackendCallableArgExpansion::DynRowFields(fields) => fields.iter().find_map(|field| {
            let Some(value) = dyn_row_data_field_value(arg, &field.field, env) else {
                return Some(BackendDiagnostic::UnsupportedI32ReturnValue {
                    value: format!("{}.{}", arg.debug_name(), field.field),
                });
            };
            let renderable = match WasmValueKind::from(field.kind) {
                WasmValueKind::I32 => core_value_is_renderable_i32(&value, env),
                WasmValueKind::ExternRef => core_value_is_renderable_externref(&value, env),
            };
            (!renderable).then(|| BackendDiagnostic::UnsupportedI32ReturnValue {
                value: value.debug_name(),
            })
        }),
    }
}

fn unsupported_i32_match(
    scrutinee: &CoreValue,
    arms: &[CoreMatchArm],
    env: &RenderEnv,
) -> Option<BackendDiagnostic> {
    unsupported_i32_value(scrutinee, env)
        .or_else(|| unsupported_i32_match_arms(scrutinee, arms, env))
}

fn unsupported_i32_match_arms(
    scrutinee: &CoreValue,
    arms: &[CoreMatchArm],
    env: &RenderEnv,
) -> Option<BackendDiagnostic> {
    let Some((first, rest)) = arms.split_first() else {
        return None;
    };
    match &first.pattern {
        CorePattern::Wildcard => unsupported_i32_value(&first.value, env),
        CorePattern::Bind(name) => {
            let arm_env = env.with_binding(name, scrutinee.clone());
            unsupported_i32_value(&first.value, &arm_env)
        }
        CorePattern::I64(_) | CorePattern::Bool(_) => unsupported_i32_value(&first.value, env)
            .or_else(|| unsupported_i32_match_arms(scrutinee, rest, env)),
        CorePattern::Constructor { data, ctor, args } => {
            if let Some((_, arm_env)) =
                constructor_match_env(scrutinee, data.as_deref(), ctor, args, env)
            {
                unsupported_i32_value(&first.value, &arm_env)
                    .or_else(|| unsupported_i32_match_arms(scrutinee, rest, env))
            } else {
                unsupported_i32_match_arms(scrutinee, rest, env)
            }
        }
    }
}

fn first_return_value(core: &CoreProgram) -> Option<CoreValue> {
    core.ops.iter().find_map(|op| match op {
        CoreOp::ReturnValue(value) => Some(value.clone()),
        CoreOp::ReturnBranch { .. } => None,
        CoreOp::ReturnMatch { .. } => None,
        _ => None,
    })
}

fn manifest_for_core(
    core: &CoreProgram,
    params: &[String],
    param_kinds: &BTreeMap<String, WasmValueKind>,
    param_abi: &BackendParamAbi,
) -> BackendManifest {
    let env = env_with_tailcall_result_facts(
        core,
        &RenderEnv::from_param_abi(params, param_kinds, param_abi).with_lifted_function_abis(core),
    );
    let mut entries = Vec::new();
    for op in &core.ops {
        collect_manifest_entries(op, core, &mut entries);
    }
    let mut imports = Vec::new();
    for op in &core.ops {
        collect_manifest_imports(op, &env, &mut imports);
    }
    sort_dedup_imports(&mut imports);
    BackendManifest { entries, imports }
}

fn collect_manifest_entries(
    op: &CoreOp,
    core: &CoreProgram,
    entries: &mut Vec<BackendManifestEntry>,
) {
    match op {
        CoreOp::LiftedFunction { source, symbol, .. } => entries.push(BackendManifestEntry {
            final_symbol: final_symbol(symbol),
            source_debug_name: source.clone(),
            pass_origin: "L15LambdaLift".to_string(),
            ownership: ownership_for_subject(core, source),
        }),
        CoreOp::DirectMethodTarget { name, target } => entries.push(BackendManifestEntry {
            final_symbol: final_symbol(target),
            source_debug_name: name.clone(),
            pass_origin: "L16Core".to_string(),
            ownership: ownership_for_subject(core, target),
        }),
        CoreOp::OperatorTarget {
            protocol, target, ..
        } => entries.push(BackendManifestEntry {
            final_symbol: final_symbol(target),
            source_debug_name: protocol.clone(),
            pass_origin: "L16Core".to_string(),
            ownership: ownership_for_subject(core, target),
        }),
        CoreOp::CaptureContinuation { captured, .. } => {
            for op in &captured.ops {
                collect_manifest_entries(op, core, entries);
            }
        }
        CoreOp::ReturnValue(_)
        | CoreOp::RuntimeLet { .. }
        | CoreOp::ReturnBranch { .. }
        | CoreOp::ReturnMatch { .. }
        | CoreOp::DynamicCallableTarget { .. }
        | CoreOp::DynRowParamMethodTarget { .. }
        | CoreOp::CallableAlias { .. }
        | CoreOp::ExternFunctionTarget { .. }
        | CoreOp::TupleConstruct { .. }
        | CoreOp::TupleFieldGet { .. }
        | CoreOp::RecordConstruct { .. }
        | CoreOp::RecordUpdate { .. }
        | CoreOp::RecordFieldGet { .. }
        | CoreOp::RangeFieldGet { .. }
        | CoreOp::SliceFieldGet { .. }
        | CoreOp::AdtConstruct { .. }
        | CoreOp::AdtTupleBridge { .. }
        | CoreOp::CompilerIntrinsicUse { .. }
        | CoreOp::TargetSpecificTerm { .. }
        | CoreOp::TailCallResult { .. }
        | CoreOp::TailCall { .. }
        | CoreOp::Prompt { .. }
        | CoreOp::Branch { .. }
        | CoreOp::Match { .. }
        | CoreOp::StaticRowAccess { .. }
        | CoreOp::DynRowAdapterAccess { .. } => {}
    }
}

fn collect_manifest_imports(op: &CoreOp, env: &RenderEnv, imports: &mut Vec<BackendExternImport>) {
    match op {
        CoreOp::ReturnValue(value) => collect_runtime_value_imports(value, env, imports),
        CoreOp::RuntimeLet { value, .. } => collect_runtime_value_imports(value, env, imports),
        CoreOp::ReturnBranch {
            cond,
            then_value,
            else_value,
        } => {
            collect_runtime_value_imports(cond, env, imports);
            collect_runtime_value_imports(then_value, env, imports);
            collect_runtime_value_imports(else_value, env, imports);
        }
        CoreOp::ReturnMatch { scrutinee, arms } => {
            collect_runtime_value_imports(scrutinee, env, imports);
            for arm in arms {
                collect_runtime_value_imports(&arm.value, env, imports);
            }
        }
        CoreOp::ExternFunctionTarget {
            target,
            abi,
            name,
            signature,
            ..
        } => imports.push(BackendExternImport {
            abi: backend_extern_abi_from_core(*abi),
            final_symbol: final_symbol(target),
            module: backend_extern_module_from_core(*abi).to_string(),
            name: name.clone(),
            signature_hash: signature.clone(),
        }),
        CoreOp::TailCall { args, .. } => {
            for arg in args {
                collect_runtime_value_imports(arg, env, imports);
            }
        }
        CoreOp::CaptureContinuation { captured, .. } => {
            for op in &captured.ops {
                collect_manifest_imports(op, env, imports);
            }
        }
        _ => {}
    }
}

fn collect_runtime_value_imports(
    value: &CoreValue,
    env: &RenderEnv,
    imports: &mut Vec<BackendExternImport>,
) {
    match value {
        CoreValue::SliceLiteral { items } => {
            if let Some(lane) = aggregate_items_lane(items, env) {
                imports.push(slice_literal_import(items.len(), lane));
            }
            for item in items {
                collect_runtime_value_imports(item, env, imports);
            }
        }
        CoreValue::Range { start, end } => {
            imports.push(range_import());
            collect_runtime_value_imports(start, env, imports);
            collect_runtime_value_imports(end, env, imports);
        }
        CoreValue::RangeField { range, field } => {
            imports.push(range_field_import(*field));
            collect_runtime_value_imports(range, env, imports);
        }
        CoreValue::AggregateField { kind, value, field } => {
            imports.push(aggregate_field_import(*kind, *field));
            collect_runtime_value_imports(value, env, imports);
        }
        CoreValue::TextField { kind, value, field } => {
            imports.push(text_field_import(*kind, *field));
            collect_runtime_value_imports(value, env, imports);
        }
        CoreValue::AggregateIndex { kind, value, index } => {
            match aggregate_index_lane(value, index, env) {
                Some(lane) => imports.push(aggregate_index_import(*kind, lane)),
                None => {
                    imports.push(aggregate_index_import(*kind, RuntimeValueLane::I32));
                    imports.push(aggregate_index_import(*kind, RuntimeValueLane::ExternRef));
                }
            }
            collect_runtime_value_imports(value, env, imports);
            collect_runtime_value_imports(index, env, imports);
        }
        CoreValue::AggregateSlice { kind, value, range } => {
            imports.push(aggregate_slice_import(*kind));
            collect_runtime_value_imports(value, env, imports);
            collect_runtime_value_imports(range, env, imports);
        }
        CoreValue::TextIndex { kind, value, index } => {
            imports.push(text_index_import(*kind));
            collect_runtime_value_imports(value, env, imports);
            collect_runtime_value_imports(index, env, imports);
        }
        CoreValue::TextSlice { kind, value, range } => {
            imports.push(text_slice_import(*kind));
            collect_runtime_value_imports(value, env, imports);
            collect_runtime_value_imports(range, env, imports);
        }
        CoreValue::TextLiteral { kind, value } => {
            imports.push(text_literal_import(*kind, value.as_bytes().len()));
        }
        CoreValue::BuiltinRuntimeCall { call, args } => {
            imports.push(builtin_runtime_import_for_call(*call, args, env));
            for arg in args {
                collect_runtime_value_imports(arg, env, imports);
            }
        }
        CoreValue::Tuple { fields } => {
            for field in fields {
                collect_runtime_value_imports(field, env, imports);
            }
        }
        CoreValue::TupleField { tuple, .. } => collect_runtime_value_imports(tuple, env, imports),
        CoreValue::Record { fields } => {
            for field in fields {
                collect_runtime_value_imports(&field.value, env, imports);
            }
        }
        CoreValue::RecordUpdate { base, fields } => {
            collect_runtime_value_imports(base, env, imports);
            for field in fields {
                collect_runtime_value_imports(&field.value, env, imports);
            }
        }
        CoreValue::RecordField { record, .. } => {
            collect_runtime_value_imports(record, env, imports)
        }
        CoreValue::DynRowPackage { payload, .. } => {
            collect_runtime_value_imports(payload, env, imports);
        }
        CoreValue::DynRowField { package, .. } => {
            collect_runtime_value_imports(package, env, imports);
        }
        CoreValue::Adt { args, .. } => {
            for arg in args {
                collect_runtime_value_imports(arg, env, imports);
            }
        }
        CoreValue::Unit
        | CoreValue::I64(_)
        | CoreValue::Bool(_)
        | CoreValue::Var(_)
        | CoreValue::LiftedFunction { .. }
        | CoreValue::Rendered { .. } => {}
    }
}

fn slice_literal_import(arity: usize, lane: RuntimeValueLane) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: slice_literal_import_symbol(arity, lane),
        module: "env".to_string(),
        name: slice_literal_import_name(arity, lane),
        signature_hash: slice_literal_import_signature(arity, lane),
    }
}

fn slice_literal_import_name(arity: usize, lane: RuntimeValueLane) -> String {
    match lane {
        RuntimeValueLane::I32 => format!("std.slice_i64_literal_{arity}"),
        RuntimeValueLane::ExternRef => format!("std.slice_externref_literal_{arity}"),
    }
}

fn slice_literal_import_symbol(arity: usize, lane: RuntimeValueLane) -> String {
    slice_literal_import_name(arity, lane).replace('.', "_")
}

fn slice_literal_import_signature(arity: usize, lane: RuntimeValueLane) -> String {
    let params = std::iter::repeat(lane.signature_atom())
        .take(arity)
        .collect::<Vec<_>>()
        .join("_");
    format!("{params}_to_externref")
}

fn range_import() -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: range_import_symbol(),
        module: "env".to_string(),
        name: "std.range_i64_new".to_string(),
        signature_hash: "i64_i64_to_externref".to_string(),
    }
}

fn range_import_symbol() -> String {
    "std_range_i64_new".to_string()
}

fn range_field_import(field: RangeField) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: range_field_import_symbol(field),
        module: "env".to_string(),
        name: format!("std.range_i64_{}", range_field_import_suffix(field)),
        signature_hash: "externref_to_i64".to_string(),
    }
}

fn range_field_import_symbol(field: RangeField) -> String {
    format!("std_range_i64_{}", range_field_import_suffix(field))
}

fn range_field_import_suffix(field: RangeField) -> &'static str {
    match field {
        RangeField::Start => "start",
        RangeField::End => "end",
    }
}

fn aggregate_field_import(kind: AggregateKind, field: SliceField) -> BackendExternImport {
    match field {
        SliceField::Len => BackendExternImport {
            abi: BackendExternAbi::C,
            final_symbol: aggregate_field_import_symbol(kind, field),
            module: "env".to_string(),
            name: format!("std.{}_i64_len", kind.runtime_prefix()),
            signature_hash: "externref_to_i64".to_string(),
        },
    }
}

fn aggregate_field_import_symbol(kind: AggregateKind, field: SliceField) -> String {
    match field {
        SliceField::Len => format!("std_{}_i64_len", kind.runtime_prefix()),
    }
}

fn aggregate_index_import(kind: AggregateKind, lane: RuntimeValueLane) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: aggregate_index_import_symbol(kind, lane),
        module: "env".to_string(),
        name: aggregate_index_import_name(kind, lane),
        signature_hash: match lane {
            RuntimeValueLane::I32 => "externref_i64_to_i64".to_string(),
            RuntimeValueLane::ExternRef => "externref_i64_to_externref".to_string(),
        },
    }
}

fn aggregate_index_import_name(kind: AggregateKind, lane: RuntimeValueLane) -> String {
    match lane {
        RuntimeValueLane::I32 => format!("std.{}_i64_get", kind.runtime_prefix()),
        RuntimeValueLane::ExternRef => format!("std.{}_get", kind.runtime_prefix()),
    }
}

fn aggregate_index_import_symbol(kind: AggregateKind, lane: RuntimeValueLane) -> String {
    aggregate_index_import_name(kind, lane).replace('.', "_")
}

fn aggregate_slice_import(kind: AggregateKind) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: aggregate_slice_import_symbol(kind),
        module: "env".to_string(),
        name: format!("std.{}_i64_slice", kind.runtime_prefix()),
        signature_hash: "externref_i64_i64_to_externref".to_string(),
    }
}

fn aggregate_slice_import_symbol(kind: AggregateKind) -> String {
    format!("std_{}_i64_slice", kind.runtime_prefix())
}

fn text_field_import(kind: TextKind, field: TextField) -> BackendExternImport {
    match field {
        TextField::Len => BackendExternImport {
            abi: BackendExternAbi::C,
            final_symbol: text_field_import_symbol(kind, field),
            module: "env".to_string(),
            name: format!("std.{}_i64_len", kind.runtime_prefix()),
            signature_hash: "externref_to_i64".to_string(),
        },
    }
}

fn text_field_import_symbol(kind: TextKind, field: TextField) -> String {
    match field {
        TextField::Len => format!("std_{}_i64_len", kind.runtime_prefix()),
    }
}

fn text_index_import(kind: TextKind) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: text_index_import_symbol(kind),
        module: "env".to_string(),
        name: format!("std.{}_i64_byte_at", kind.runtime_prefix()),
        signature_hash: "externref_i64_to_i64".to_string(),
    }
}

fn text_index_import_symbol(kind: TextKind) -> String {
    format!("std_{}_i64_byte_at", kind.runtime_prefix())
}

fn text_slice_import(kind: TextKind) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: text_slice_import_symbol(kind),
        module: "env".to_string(),
        name: format!("std.{}_slice", kind.runtime_prefix()),
        signature_hash: "externref_i64_i64_to_externref".to_string(),
    }
}

fn text_slice_import_symbol(kind: TextKind) -> String {
    format!("std_{}_slice", kind.runtime_prefix())
}

fn text_literal_import(kind: TextKind, arity: usize) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: text_literal_import_symbol(kind, arity),
        module: "env".to_string(),
        name: format!("std.{}_literal_{}", kind.runtime_prefix(), arity),
        signature_hash: text_literal_import_signature(arity),
    }
}

fn text_literal_import_symbol(kind: TextKind, arity: usize) -> String {
    format!("std_{}_literal_{}", kind.runtime_prefix(), arity)
}

fn text_literal_import_signature(arity: usize) -> String {
    let params = std::iter::repeat("i64")
        .take(arity)
        .collect::<Vec<_>>()
        .join("_");
    format!("{params}_to_externref")
}

fn builtin_runtime_import(call: BuiltinMethodCall) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: builtin_runtime_import_symbol(call),
        module: "env".to_string(),
        name: builtin_runtime_import_name(call).to_string(),
        signature_hash: builtin_runtime_import_signature(call).to_string(),
    }
}

fn builtin_runtime_import_for_call(
    call: BuiltinMethodCall,
    args: &[CoreValue],
    env: &RenderEnv,
) -> BackendExternImport {
    if let Some(lane) = ref_runtime_lane(call, args, env) {
        return ref_runtime_import(call, lane);
    }
    if let Some(lane) = vec_runtime_lane(call, args, env) {
        return vec_runtime_import(call, lane);
    }
    builtin_runtime_import(call)
}

fn builtin_runtime_import_symbol(call: BuiltinMethodCall) -> String {
    builtin_runtime_import_name(call).replace('.', "_")
}

fn builtin_runtime_import_name(call: BuiltinMethodCall) -> &'static str {
    match call {
        BuiltinMethodCall::VecNew => "std.vec_new",
        BuiltinMethodCall::VecPush => "std.vec_push",
        BuiltinMethodCall::VecFreeze => "std.vec_freeze",
        BuiltinMethodCall::StringNew => "std.string_new",
        BuiltinMethodCall::StringFrom => "std.str_to_string",
        BuiltinMethodCall::StringConcat => "std.string_concat",
        BuiltinMethodCall::StringAsStr => "std.string_as_str",
        BuiltinMethodCall::StringToCStr => "std.string_to_cstr",
        BuiltinMethodCall::StringPushRune => "std.string_push_rune",
        BuiltinMethodCall::TextLen {
            kind: TextKind::Str,
        } => "std.str_i64_len",
        BuiltinMethodCall::TextLen {
            kind: TextKind::String,
        } => "std.string_i64_len",
        BuiltinMethodCall::TextLen {
            kind: TextKind::CStr,
        } => "std.cstr_i64_len",
        BuiltinMethodCall::TextRuneLen {
            kind: TextKind::Str,
        } => "std.str_rune_len",
        BuiltinMethodCall::TextRuneLen {
            kind: TextKind::String,
        } => "std.string_rune_len",
        BuiltinMethodCall::TextRuneLen {
            kind: TextKind::CStr,
        } => "std.cstr_rune_len",
        BuiltinMethodCall::TextCharAt {
            kind: TextKind::Str,
        } => "std.str_char_at",
        BuiltinMethodCall::TextCharAt {
            kind: TextKind::String,
        } => "std.string_char_at",
        BuiltinMethodCall::TextCharAt {
            kind: TextKind::CStr,
        } => "std.cstr_char_at",
        BuiltinMethodCall::RefNew => "std.ref_new",
        BuiltinMethodCall::RefGet => "std.ref_get",
        BuiltinMethodCall::RefSet => "std.ref_set",
        BuiltinMethodCall::UnsafeRefNew => "std.unsafe_ref_new",
        BuiltinMethodCall::UnsafeRefGet => "std.unsafe_ref_get",
        BuiltinMethodCall::UnsafeRefSet => "std.unsafe_ref_set",
    }
}

fn builtin_runtime_import_signature(call: BuiltinMethodCall) -> &'static str {
    match call {
        BuiltinMethodCall::VecNew => "_to_externref",
        BuiltinMethodCall::VecPush => "externref_i64_to_externref",
        BuiltinMethodCall::VecFreeze => "externref_to_externref",
        BuiltinMethodCall::StringNew => "_to_externref",
        BuiltinMethodCall::StringFrom => "externref_to_externref",
        BuiltinMethodCall::StringConcat => "externref_externref_to_externref",
        BuiltinMethodCall::StringAsStr => "externref_to_externref",
        BuiltinMethodCall::StringToCStr => "externref_to_externref",
        BuiltinMethodCall::StringPushRune => "externref_i64_to_externref",
        BuiltinMethodCall::TextLen { .. } => "externref_to_i64",
        BuiltinMethodCall::TextRuneLen { .. } => "externref_to_i64",
        BuiltinMethodCall::TextCharAt { .. } => "externref_i64_to_i64",
        BuiltinMethodCall::RefNew => "i64_to_externref",
        BuiltinMethodCall::RefGet => "externref_to_i64",
        BuiltinMethodCall::RefSet => "externref_i64_to_externref",
        BuiltinMethodCall::UnsafeRefNew => "i64_to_externref",
        BuiltinMethodCall::UnsafeRefGet => "externref_to_i64",
        BuiltinMethodCall::UnsafeRefSet => "externref_i64_to_externref",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeValueLane {
    I32,
    ExternRef,
}

impl RuntimeValueLane {
    fn signature_atom(self) -> &'static str {
        match self {
            Self::I32 => "i64",
            Self::ExternRef => "externref",
        }
    }
}

fn runtime_value_lane(value: &CoreValue, env: &RenderEnv) -> Option<RuntimeValueLane> {
    if core_value_is_renderable_externref(value, env) {
        Some(RuntimeValueLane::ExternRef)
    } else if core_value_is_renderable_i32(value, env) {
        Some(RuntimeValueLane::I32)
    } else {
        None
    }
}

fn aggregate_items_lane(items: &[CoreValue], env: &RenderEnv) -> Option<RuntimeValueLane> {
    let mut lane = None;
    for item in items {
        let item_lane = runtime_value_lane(item, env)?;
        lane = Some(match (lane, item_lane) {
            (None, item_lane) => item_lane,
            (Some(previous), item_lane) if previous == item_lane => previous,
            _ => return None,
        });
    }
    lane.or(Some(RuntimeValueLane::I32))
}

fn aggregate_index_lane(
    value: &CoreValue,
    index: &CoreValue,
    env: &RenderEnv,
) -> Option<RuntimeValueLane> {
    if let Some(item) = slice_index_value(value, index, env) {
        return runtime_value_lane(item, env);
    }
    if core_value_is_renderable_i32(index, env) {
        return aggregate_value_element_lane(value, env);
    }
    None
}

fn aggregate_value_element_lane(value: &CoreValue, env: &RenderEnv) -> Option<RuntimeValueLane> {
    match resolve_core_value_binding(value, env) {
        CoreValue::Var(name) => env.aggregate_element_lane(name).map(RuntimeValueLane::from),
        CoreValue::BuiltinRuntimeCall {
            call: BuiltinMethodCall::VecFreeze,
            args,
        } => args
            .first()
            .and_then(|vec| aggregate_value_element_lane(vec, env)),
        CoreValue::BuiltinRuntimeCall {
            call: BuiltinMethodCall::VecPush,
            args,
        } => args.get(1).and_then(|item| runtime_value_lane(item, env)),
        CoreValue::AggregateSlice { value, .. } => aggregate_value_element_lane(value, env),
        CoreValue::SliceLiteral { items } => aggregate_items_lane(items, env),
        _ => None,
    }
}

fn render_runtime_lane_value(
    wat: &mut String,
    value: &CoreValue,
    env: &RenderEnv,
) -> Result<RuntimeValueLane, BackendDiagnostic> {
    match runtime_value_lane(value, env) {
        Some(RuntimeValueLane::ExternRef) => {
            render_core_value_externref(wat, value, env)?;
            Ok(RuntimeValueLane::ExternRef)
        }
        Some(RuntimeValueLane::I32) => {
            render_core_value_i32(wat, value, env)?;
            Ok(RuntimeValueLane::I32)
        }
        None => Err(unsupported_i32_render_diagnostic(value)),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RefRuntimeLane {
    I32,
    ExternRef,
}

impl From<CoreRefCellLane> for RefRuntimeLane {
    fn from(value: CoreRefCellLane) -> Self {
        match value {
            CoreRefCellLane::I32 => Self::I32,
            CoreRefCellLane::ExternRef => Self::ExternRef,
        }
    }
}

fn ref_runtime_lane(
    call: BuiltinMethodCall,
    args: &[CoreValue],
    env: &RenderEnv,
) -> Option<RefRuntimeLane> {
    match (call, args) {
        (BuiltinMethodCall::RefNew | BuiltinMethodCall::UnsafeRefNew, [value])
        | (BuiltinMethodCall::RefSet | BuiltinMethodCall::UnsafeRefSet, [_, value]) => {
            ref_value_lane(value, env)
        }
        (BuiltinMethodCall::RefGet | BuiltinMethodCall::UnsafeRefGet, [cell]) => {
            ref_cell_lane(cell, env)
        }
        _ => None,
    }
}

fn ref_value_lane(value: &CoreValue, env: &RenderEnv) -> Option<RefRuntimeLane> {
    match runtime_value_lane(value, env)? {
        RuntimeValueLane::I32 => Some(RefRuntimeLane::I32),
        RuntimeValueLane::ExternRef => Some(RefRuntimeLane::ExternRef),
    }
}

fn ref_cell_lane(cell: &CoreValue, env: &RenderEnv) -> Option<RefRuntimeLane> {
    if let CoreValue::Var(name) = resolve_core_value_binding(cell, env) {
        if let Some(lane) = env.local_ref_cell_lane(name) {
            return Some(RefRuntimeLane::from(lane));
        }
    }
    let CoreValue::BuiltinRuntimeCall { call, args } = resolve_core_value_binding(cell, env) else {
        return Some(RefRuntimeLane::I32);
    };
    match call {
        BuiltinMethodCall::RefNew
        | BuiltinMethodCall::RefSet
        | BuiltinMethodCall::UnsafeRefNew
        | BuiltinMethodCall::UnsafeRefSet => ref_runtime_lane(*call, args, env),
        _ => Some(RefRuntimeLane::I32),
    }
}

fn ref_runtime_import(call: BuiltinMethodCall, lane: RefRuntimeLane) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: ref_runtime_import_symbol(call, lane),
        module: "env".to_string(),
        name: ref_runtime_import_name(call, lane),
        signature_hash: ref_runtime_import_signature(call, lane),
    }
}

fn ref_runtime_import_symbol(call: BuiltinMethodCall, lane: RefRuntimeLane) -> String {
    ref_runtime_import_name(call, lane).replace('.', "_")
}

fn ref_runtime_import_name(call: BuiltinMethodCall, lane: RefRuntimeLane) -> String {
    match lane {
        RefRuntimeLane::I32 => builtin_runtime_import_name(call).to_string(),
        RefRuntimeLane::ExternRef => format!("{}_externref", builtin_runtime_import_name(call)),
    }
}

fn ref_runtime_import_signature(call: BuiltinMethodCall, lane: RefRuntimeLane) -> String {
    match (call, lane) {
        (BuiltinMethodCall::RefNew | BuiltinMethodCall::UnsafeRefNew, RefRuntimeLane::I32) => {
            "i64_to_externref".to_string()
        }
        (BuiltinMethodCall::RefGet | BuiltinMethodCall::UnsafeRefGet, RefRuntimeLane::I32) => {
            "externref_to_i64".to_string()
        }
        (BuiltinMethodCall::RefSet | BuiltinMethodCall::UnsafeRefSet, RefRuntimeLane::I32) => {
            "externref_i64_to_externref".to_string()
        }
        (
            BuiltinMethodCall::RefNew | BuiltinMethodCall::UnsafeRefNew,
            RefRuntimeLane::ExternRef,
        ) => "externref_to_externref".to_string(),
        (
            BuiltinMethodCall::RefGet | BuiltinMethodCall::UnsafeRefGet,
            RefRuntimeLane::ExternRef,
        ) => "externref_to_externref".to_string(),
        (
            BuiltinMethodCall::RefSet | BuiltinMethodCall::UnsafeRefSet,
            RefRuntimeLane::ExternRef,
        ) => "externref_externref_to_externref".to_string(),
        _ => builtin_runtime_import_signature(call).to_string(),
    }
}

fn vec_runtime_lane(
    call: BuiltinMethodCall,
    args: &[CoreValue],
    env: &RenderEnv,
) -> Option<RuntimeValueLane> {
    match (call, args) {
        (BuiltinMethodCall::VecPush, [_, item]) => runtime_value_lane(item, env),
        _ => None,
    }
}

fn vec_runtime_import(call: BuiltinMethodCall, lane: RuntimeValueLane) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: vec_runtime_import_symbol(call, lane),
        module: "env".to_string(),
        name: vec_runtime_import_name(call, lane),
        signature_hash: vec_runtime_import_signature(call, lane),
    }
}

fn vec_runtime_import_symbol(call: BuiltinMethodCall, lane: RuntimeValueLane) -> String {
    vec_runtime_import_name(call, lane).replace('.', "_")
}

fn vec_runtime_import_name(call: BuiltinMethodCall, lane: RuntimeValueLane) -> String {
    match (call, lane) {
        (BuiltinMethodCall::VecPush, RuntimeValueLane::I32) => {
            builtin_runtime_import_name(call).to_string()
        }
        (BuiltinMethodCall::VecPush, RuntimeValueLane::ExternRef) => {
            "std.vec_push_externref".to_string()
        }
        _ => builtin_runtime_import_name(call).to_string(),
    }
}

fn vec_runtime_import_signature(call: BuiltinMethodCall, lane: RuntimeValueLane) -> String {
    match (call, lane) {
        (BuiltinMethodCall::VecPush, RuntimeValueLane::I32) => {
            "externref_i64_to_externref".to_string()
        }
        (BuiltinMethodCall::VecPush, RuntimeValueLane::ExternRef) => {
            "externref_externref_to_externref".to_string()
        }
        _ => builtin_runtime_import_signature(call).to_string(),
    }
}

fn ownership_for_subject(core: &CoreProgram, subject: &str) -> Option<OwnershipDecision> {
    core.ownership
        .iter()
        .find(|fact| fact.subject == subject)
        .map(|fact| fact.decision)
}

fn render_wat(
    core: &CoreProgram,
    manifest: &BackendManifest,
    params: &[String],
    param_kinds: &BTreeMap<String, WasmValueKind>,
    param_abi: &BackendParamAbi,
) -> Result<String, BackendDiagnostic> {
    if core_contains_continuation_runtime(core) {
        return render_continuation_wat(core, manifest, params, param_kinds, param_abi);
    }

    let mut env =
        RenderEnv::from_param_abi(params, param_kinds, param_abi).with_lifted_function_abis(core);
    let mut wat = String::from("(module\n");
    for import in &manifest.imports {
        render_extern_import_wat(&mut wat, import);
    }
    let mut return_index = 0usize;
    let mut tailcall_index = 0usize;
    for entry in &manifest.entries {
        wat.push_str(&format!(
            "  ;; symbol {} source={} origin={}\n",
            entry.final_symbol, entry.source_debug_name, entry.pass_origin
        ));
    }
    if render_tailcall_result_chain(&mut wat, core, &env)? {
        render_operator_intrinsics_for_ops(&mut wat, &core.ops);
        wat.push_str(")\n");
        return Ok(wat);
    }
    render_operator_intrinsics_for_ops(&mut wat, &core.ops);
    for (index, op) in core.ops.iter().enumerate() {
        match op {
            CoreOp::ReturnValue(value)
                if return_value_is_tailcall_result(core, index, value, &env) => {}
            CoreOp::RuntimeLet { binder, value } => {
                wat.push_str(&format!(
                    "  ;; runtime-let {} = {}\n",
                    escape_wat_comment(binder),
                    escape_wat_comment(&value.debug_name())
                ));
            }
            CoreOp::ReturnValue(value) => {
                let symbol = if return_index == 0 {
                    "main".to_string()
                } else {
                    format!("chiba_return_{return_index}")
                };
                let debug_name = value.debug_name();
                wat.push_str(&format!(
                    "  ;; core-return atom={}\n",
                    escape_wat_comment(&debug_name)
                ));
                render_return_value_wat(&mut wat, &symbol, Some(&symbol), value, &env)?;
                return_index += 1;
            }
            CoreOp::ReturnBranch {
                cond,
                then_value,
                else_value,
            } => {
                let symbol = if return_index == 0 {
                    "main".to_string()
                } else {
                    format!("chiba_return_{return_index}")
                };
                wat.push_str(&format!(
                    "  ;; core-return branch cond={} then={} else={}\n",
                    escape_wat_comment(&cond.debug_name()),
                    escape_wat_comment(&then_value.debug_name()),
                    escape_wat_comment(&else_value.debug_name())
                ));
                render_func_header(&mut wat, &symbol, Some(&symbol), &env);
                render_core_value_i32(&mut wat, cond, &env)?;
                wat.push_str("    if (result i32)\n");
                render_core_value_i32_indented(&mut wat, then_value, &env, 6)?;
                wat.push_str("    else\n");
                render_core_value_i32_indented(&mut wat, else_value, &env, 6)?;
                wat.push_str("    end\n");
                wat.push_str("  )\n");
                return_index += 1;
            }
            CoreOp::ReturnMatch { scrutinee, arms } => {
                let symbol = if return_index == 0 {
                    "main".to_string()
                } else {
                    format!("chiba_return_{return_index}")
                };
                wat.push_str(&format!(
                    "  ;; core-return match scrutinee={} arms={}\n",
                    escape_wat_comment(&scrutinee.debug_name()),
                    arms.len()
                ));
                render_func_header(&mut wat, &symbol, Some(&symbol), &env);
                render_match_arms_i32(&mut wat, scrutinee, arms, &env, 4)?;
                wat.push_str("  )\n");
                return_index += 1;
            }
            CoreOp::TailCall { func, args } => {
                let (runtime_func, runtime_args) = effective_tailcall(core, func, args, &env);
                wat.push_str(&format!(
                    "  ;; tailcall {} args=[{}]\n",
                    final_symbol(runtime_func),
                    runtime_args
                        .iter()
                        .map(CoreValue::debug_name)
                        .map(|arg| escape_wat_comment(&arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                if tailcall_args_are_renderable(core, runtime_func, &runtime_args, &env) {
                    let symbol = if tailcall_index == 0 {
                        "main".to_string()
                    } else {
                        format!("chiba_tailcall_{tailcall_index}")
                    };
                    let export = (tailcall_index == 0).then_some(symbol.as_str());
                    render_tailcall_func_header(
                        &mut wat,
                        core,
                        runtime_func,
                        &symbol,
                        export,
                        &env,
                    );
                    if let Some(callable) = env.callable_param(runtime_func) {
                        render_callable_param_call(
                            &mut wat,
                            runtime_func,
                            callable,
                            &runtime_args,
                            &env,
                        )?;
                    } else {
                        render_tailcall_args(&mut wat, core, runtime_func, &runtime_args, &env)?;
                        wat.push_str(&format!("    call ${}\n", final_symbol(runtime_func)));
                    }
                    wat.push_str("  )\n");
                    tailcall_index += 1;
                }
                if let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) {
                    env = env.with_local_kind(
                        binder,
                        tailcall_result_kind(core, runtime_func, &env)
                            .unwrap_or(WasmValueKind::I32),
                    );
                }
            }
            CoreOp::Branch { cond } => {
                wat.push_str(&format!("  ;; branch cond={}\n", escape_wat_comment(cond)));
            }
            CoreOp::Match {
                scrutinee,
                patterns,
            } => {
                wat.push_str(&format!(
                    "  ;; match scrutinee={} arms={}\n",
                    escape_wat_comment(scrutinee),
                    patterns.len()
                ));
            }
            CoreOp::TupleConstruct { layout, fields, .. } => {
                wat.push_str(&format!(
                    "  ;; tuple layout={} fields={}\n",
                    escape_wat_comment(layout),
                    fields.len()
                ));
            }
            CoreOp::TupleFieldGet { layout, field, .. } => {
                wat.push_str(&format!(
                    "  ;; tuple-field layout={} field={}\n",
                    escape_wat_comment(layout),
                    escape_wat_comment(field)
                ));
            }
            CoreOp::RecordConstruct { layout, fields } => {
                wat.push_str(&format!(
                    "  ;; record layout={} fields={}\n",
                    escape_wat_comment(layout),
                    fields.len()
                ));
            }
            CoreOp::RecordUpdate {
                base,
                layout,
                fields,
            } => {
                wat.push_str(&format!(
                    "  ;; record-update base={} layout={} fields={}\n",
                    escape_wat_comment(base),
                    escape_wat_comment(layout),
                    fields.len()
                ));
            }
            CoreOp::RecordFieldGet { layout, field } => {
                wat.push_str(&format!(
                    "  ;; record-field layout={} field={}\n",
                    escape_wat_comment(layout),
                    escape_wat_comment(field)
                ));
            }
            CoreOp::RangeFieldGet { field } => {
                wat.push_str(&format!("  ;; range-field field={}\n", field.source_name()));
            }
            CoreOp::SliceFieldGet { field } => {
                wat.push_str(&format!("  ;; slice-field field={}\n", field.source_name()));
            }
            CoreOp::AdtConstruct {
                data, ctor, args, ..
            } => {
                wat.push_str(&format!(
                    "  ;; adt data={} ctor={} args={}\n",
                    escape_wat_comment(data),
                    escape_wat_comment(ctor),
                    args.len()
                ));
            }
            CoreOp::AdtTupleBridge {
                data,
                ctor,
                tuple_fields,
                tuple_to_adt_intrinsic,
                adt_to_tuple_intrinsic,
            } => {
                wat.push_str(&format!(
                    "  ;; adt-tuple-bridge data={} ctor={} fields={} tuple_to_adt={} adt_to_tuple={}\n",
                    escape_wat_comment(data),
                    escape_wat_comment(ctor),
                    tuple_fields.len(),
                    escape_wat_comment(tuple_to_adt_intrinsic.debug_name()),
                    escape_wat_comment(adt_to_tuple_intrinsic.debug_name())
                ));
            }
            CoreOp::CompilerIntrinsicUse {
                intrinsic,
                owner_namespace,
                subject,
            } => {
                wat.push_str(&format!(
                    "  ;; compiler-intrinsic owner={} intrinsic={} subject={}\n",
                    escape_wat_comment(owner_namespace),
                    escape_wat_comment(intrinsic.debug_name()),
                    escape_wat_comment(subject)
                ));
            }
            CoreOp::Prompt { kind } => {
                wat.push_str(&format!(
                    "  ;; prompt kind={}\n",
                    render_continuation_kind(*kind)
                ));
            }
            CoreOp::CaptureContinuation { binder, kind, .. } => {
                wat.push_str(&format!(
                    "  ;; capture-cont binder={} kind={}\n",
                    escape_wat_comment(binder),
                    render_continuation_kind(*kind)
                ));
            }
            CoreOp::StaticRowAccess { field, layout } => {
                wat.push_str(&format!(
                    "  ;; static-row field={} layout={}\n",
                    escape_wat_comment(field),
                    escape_wat_comment(layout)
                ));
            }
            CoreOp::DynRowAdapterAccess { subject, layout } => {
                wat.push_str(&format!(
                    "  ;; dyn-row subject={} layout={}\n",
                    escape_wat_comment(subject),
                    escape_wat_comment(layout)
                ));
            }
            CoreOp::DirectMethodTarget { .. }
            | CoreOp::DynamicCallableTarget { .. }
            | CoreOp::DynRowParamMethodTarget { .. }
            | CoreOp::CallableAlias { .. }
            | CoreOp::ExternFunctionTarget { .. }
            | CoreOp::TailCallResult { .. }
            | CoreOp::TargetSpecificTerm { .. } => {}
            CoreOp::LiftedFunction {
                symbol,
                env_params,
                param,
                body,
                ..
            } => {
                render_lifted_function_wat(
                    &mut wat,
                    symbol,
                    env_params,
                    param.as_deref(),
                    body,
                    &env,
                )?;
            }
            CoreOp::OperatorTarget { .. } => {}
        }
    }
    wat.push_str(")\n");
    Ok(wat)
}

fn render_operator_intrinsics_for_ops(wat: &mut String, ops: &[CoreOp]) {
    let mut emitted = BTreeSet::new();
    for op in collect_operator_targets(ops) {
        let CoreOp::OperatorTarget {
            protocol,
            target,
            intrinsic,
        } = op
        else {
            continue;
        };
        if emitted.insert(target.clone()) {
            render_operator_intrinsic_wat(wat, protocol, target, *intrinsic);
        }
    }
}

fn core_contains_continuation_runtime(core: &CoreProgram) -> bool {
    core.ops.iter().any(|op| {
        matches!(
            op,
            CoreOp::Prompt { .. } | CoreOp::CaptureContinuation { .. }
        )
    })
}

fn render_continuation_wat(
    core: &CoreProgram,
    manifest: &BackendManifest,
    params: &[String],
    param_kinds: &BTreeMap<String, WasmValueKind>,
    param_abi: &BackendParamAbi,
) -> Result<String, BackendDiagnostic> {
    let base_env =
        RenderEnv::from_param_abi(params, param_kinds, param_abi).with_lifted_function_abis(core);
    let runtime_local_kinds = collect_runtime_local_kinds(&core.ops, &base_env);
    let env = base_env.with_local_kinds(runtime_local_kinds.clone());
    let mut wat = String::from("(module\n");
    for import in &manifest.imports {
        render_extern_import_wat(&mut wat, import);
    }
    for entry in &manifest.entries {
        wat.push_str(&format!(
            "  ;; symbol {} source={} origin={}\n",
            entry.final_symbol, entry.source_debug_name, entry.pass_origin
        ));
    }
    for op in collect_operator_targets(&core.ops) {
        let CoreOp::OperatorTarget {
            protocol,
            target,
            intrinsic,
        } = op
        else {
            continue;
        };
        render_operator_intrinsic_wat(&mut wat, protocol, target, *intrinsic);
    }

    wat.push_str("  ;; continuation-runtime subset=prompt-capture-i32\n");
    render_func_header(&mut wat, "main", Some("main"), &env);
    for (binder, kind) in &runtime_local_kinds {
        wat.push_str(&format!(
            "    (local ${} {})\n",
            encode_debug_symbol(binder),
            kind.wat_type()
        ));
    }

    let mut continuations = BTreeMap::<String, (ContinuationKind, CoreCapturedContinuation)>::new();
    let mut cont1_consumed = BTreeSet::<String>::new();
    let mut returned = false;
    for (index, op) in core.ops.iter().enumerate() {
        match op {
            CoreOp::Prompt { kind } => {
                wat.push_str(&format!(
                    "    ;; prompt kind={}\n",
                    render_continuation_kind(*kind)
                ));
            }
            CoreOp::CaptureContinuation {
                binder,
                kind,
                captured,
            } => {
                continuations.insert(binder.clone(), (*kind, captured.clone()));
                wat.push_str(&format!(
                    "    ;; capture-cont binder={} kind={}\n",
                    escape_wat_comment(binder),
                    render_continuation_kind(*kind)
                ));
            }
            CoreOp::RuntimeLet { binder, value } => {
                wat.push_str(&format!(
                    "    ;; runtime-let {} = {}\n",
                    escape_wat_comment(binder),
                    escape_wat_comment(&value.debug_name())
                ));
                render_runtime_let_value(&mut wat, binder, value, &env)?;
            }
            CoreOp::TailCall { func, args }
                if continuations.contains_key(effective_tailcall(core, func, args, &env).0) =>
            {
                let (runtime_func, runtime_args) = effective_tailcall(core, func, args, &env);
                let (kind, captured) = continuations
                    .get(runtime_func)
                    .expect("checked continuation binder")
                    .clone();
                if kind == ContinuationKind::Cont1
                    && !cont1_consumed.insert(runtime_func.to_string())
                {
                    return Err(BackendDiagnostic::Cont1ResumedMoreThanOnce {
                        binder: runtime_func.to_string(),
                    });
                }
                let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
                    return Err(BackendDiagnostic::UnsupportedContinuationRuntime {
                        op: "resume-without-result".to_string(),
                        kind,
                        binder: Some(runtime_func.to_string()),
                    });
                };
                let [arg] = runtime_args.as_slice() else {
                    return Err(BackendDiagnostic::UnsupportedContinuationRuntime {
                        op: "resume-arity".to_string(),
                        kind,
                        binder: Some(runtime_func.to_string()),
                    });
                };
                wat.push_str(&format!(
                    "    ;; resume-cont binder={} kind={} result={}\n",
                    escape_wat_comment(runtime_func),
                    render_continuation_kind(kind),
                    escape_wat_comment(binder)
                ));
                render_captured_continuation_i32(&mut wat, &captured, arg, &env, core)?;
                wat.push_str(&format!("    local.set ${}\n", encode_debug_symbol(binder)));
            }
            CoreOp::TailCall { func, args } => {
                let (runtime_func, runtime_args) = effective_tailcall(core, func, args, &env);
                let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
                    continue;
                };
                wat.push_str(&format!(
                    "    ;; tailcall {} args=[{}]\n",
                    final_symbol(runtime_func),
                    runtime_args
                        .iter()
                        .map(CoreValue::debug_name)
                        .map(|arg| escape_wat_comment(&arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                if let Some(callable) = env.callable_param(runtime_func) {
                    render_callable_param_call(
                        &mut wat,
                        runtime_func,
                        callable,
                        &runtime_args,
                        &env,
                    )?;
                } else {
                    render_tailcall_args(&mut wat, core, runtime_func, &runtime_args, &env)?;
                    wat.push_str(&format!("    call ${}\n", final_symbol(runtime_func)));
                }
                wat.push_str(&format!("    local.set ${}\n", encode_debug_symbol(binder)));
            }
            CoreOp::ReturnValue(value) => {
                render_core_value_i32(&mut wat, value, &env)?;
                returned = true;
            }
            CoreOp::ReturnBranch {
                cond,
                then_value,
                else_value,
            } => {
                render_core_value_i32(&mut wat, cond, &env)?;
                wat.push_str("    if (result i32)\n");
                render_core_value_i32_indented(&mut wat, then_value, &env, 6)?;
                wat.push_str("    else\n");
                render_core_value_i32_indented(&mut wat, else_value, &env, 6)?;
                wat.push_str("    end\n");
                returned = true;
            }
            CoreOp::ReturnMatch { scrutinee, arms } => {
                render_match_arms_i32(&mut wat, scrutinee, arms, &env, 4)?;
                returned = true;
            }
            CoreOp::DirectMethodTarget { .. }
            | CoreOp::DynamicCallableTarget { .. }
            | CoreOp::DynRowParamMethodTarget { .. }
            | CoreOp::CallableAlias { .. }
            | CoreOp::ExternFunctionTarget { .. }
            | CoreOp::TailCallResult { .. }
            | CoreOp::OperatorTarget { .. }
            | CoreOp::TargetSpecificTerm { .. }
            | CoreOp::LiftedFunction { .. } => {}
            CoreOp::Branch { cond } => {
                wat.push_str(&format!(
                    "    ;; branch cond={}\n",
                    escape_wat_comment(cond)
                ));
            }
            CoreOp::Match {
                scrutinee,
                patterns,
            } => {
                wat.push_str(&format!(
                    "    ;; match scrutinee={} arms={}\n",
                    escape_wat_comment(scrutinee),
                    patterns.len()
                ));
            }
            CoreOp::TupleConstruct { layout, fields, .. } => {
                wat.push_str(&format!(
                    "    ;; tuple layout={} fields={}\n",
                    escape_wat_comment(layout),
                    fields.len()
                ));
            }
            CoreOp::TupleFieldGet { layout, field, .. } => {
                wat.push_str(&format!(
                    "    ;; tuple-field layout={} field={}\n",
                    escape_wat_comment(layout),
                    escape_wat_comment(field)
                ));
            }
            CoreOp::RecordConstruct { layout, fields } => {
                wat.push_str(&format!(
                    "    ;; record layout={} fields={}\n",
                    escape_wat_comment(layout),
                    fields.len()
                ));
            }
            CoreOp::RecordUpdate {
                base,
                layout,
                fields,
            } => {
                wat.push_str(&format!(
                    "    ;; record-update base={} layout={} fields={}\n",
                    escape_wat_comment(base),
                    escape_wat_comment(layout),
                    fields.len()
                ));
            }
            CoreOp::RecordFieldGet { layout, field } => {
                wat.push_str(&format!(
                    "    ;; record-field layout={} field={}\n",
                    escape_wat_comment(layout),
                    escape_wat_comment(field)
                ));
            }
            CoreOp::RangeFieldGet { field } => {
                wat.push_str(&format!(
                    "    ;; range-field field={}\n",
                    field.source_name()
                ));
            }
            CoreOp::SliceFieldGet { field } => {
                wat.push_str(&format!(
                    "    ;; slice-field field={}\n",
                    field.source_name()
                ));
            }
            CoreOp::AdtConstruct {
                data, ctor, args, ..
            } => {
                wat.push_str(&format!(
                    "    ;; adt data={} ctor={} args={}\n",
                    escape_wat_comment(data),
                    escape_wat_comment(ctor),
                    args.len()
                ));
            }
            CoreOp::AdtTupleBridge {
                data,
                ctor,
                tuple_fields,
                tuple_to_adt_intrinsic,
                adt_to_tuple_intrinsic,
            } => {
                wat.push_str(&format!(
                    "    ;; adt-tuple-bridge data={} ctor={} fields={} tuple_to_adt={} adt_to_tuple={}\n",
                    escape_wat_comment(data),
                    escape_wat_comment(ctor),
                    tuple_fields.len(),
                    escape_wat_comment(tuple_to_adt_intrinsic.debug_name()),
                    escape_wat_comment(adt_to_tuple_intrinsic.debug_name())
                ));
            }
            CoreOp::CompilerIntrinsicUse {
                intrinsic,
                owner_namespace,
                subject,
            } => {
                wat.push_str(&format!(
                    "    ;; compiler-intrinsic owner={} intrinsic={} subject={}\n",
                    escape_wat_comment(owner_namespace),
                    escape_wat_comment(intrinsic.debug_name()),
                    escape_wat_comment(subject)
                ));
            }
            CoreOp::StaticRowAccess { field, layout } => {
                wat.push_str(&format!(
                    "    ;; static-row field={} layout={}\n",
                    escape_wat_comment(field),
                    escape_wat_comment(layout)
                ));
            }
            CoreOp::DynRowAdapterAccess { subject, layout } => {
                wat.push_str(&format!(
                    "    ;; dyn-row subject={} layout={}\n",
                    escape_wat_comment(subject),
                    escape_wat_comment(layout)
                ));
            }
        }
    }
    if !returned {
        let (kind, binder) = continuations
            .iter()
            .next()
            .map(|(binder, (kind, _))| (*kind, Some(binder.clone())))
            .unwrap_or((ContinuationKind::Cont1, None));
        return Err(BackendDiagnostic::UnsupportedContinuationRuntime {
            op: "missing-continuation-return".to_string(),
            kind,
            binder,
        });
    }
    wat.push_str("  )\n");
    wat.push_str(")\n");
    Ok(wat)
}

fn collect_tailcall_result_binders(ops: &[CoreOp]) -> Vec<String> {
    let mut binders = Vec::new();
    for op in ops {
        match op {
            CoreOp::TailCallResult { binder } => binders.push(binder.clone()),
            CoreOp::CaptureContinuation { captured, .. } => {
                binders.extend(collect_tailcall_result_binders(&captured.ops));
            }
            _ => {}
        }
    }
    binders
}

fn collect_runtime_local_binders(ops: &[CoreOp]) -> Vec<String> {
    let mut binders = collect_tailcall_result_binders(ops);
    for op in ops {
        match op {
            CoreOp::RuntimeLet { binder, .. } => binders.push(binder.clone()),
            CoreOp::CaptureContinuation { captured, .. } => {
                binders.extend(collect_runtime_local_binders(&captured.ops));
            }
            _ => {}
        }
    }
    binders.sort();
    binders.dedup();
    binders
}

fn collect_runtime_local_kinds(
    ops: &[CoreOp],
    base_env: &RenderEnv,
) -> BTreeMap<String, WasmValueKind> {
    let mut kinds = BTreeMap::new();
    let mut env = base_env.clone();
    collect_runtime_local_kinds_into(ops, &mut env, &mut kinds);
    kinds
}

fn collect_runtime_local_kinds_into(
    ops: &[CoreOp],
    env: &mut RenderEnv,
    kinds: &mut BTreeMap<String, WasmValueKind>,
) {
    for op in ops {
        match op {
            CoreOp::RuntimeLet { binder, value } => {
                let kind = runtime_local_value_kind(value, env);
                kinds.insert(binder.clone(), kind);
                *env = env.with_local_kind(binder, kind);
            }
            CoreOp::TailCallResult { binder } => {
                kinds.insert(binder.clone(), WasmValueKind::I32);
                *env = env.with_local_kind(binder, WasmValueKind::I32);
            }
            CoreOp::CaptureContinuation { captured, .. } => {
                collect_runtime_local_kinds_into(&captured.ops, env, kinds);
            }
            _ => {}
        }
    }
}

fn runtime_local_value_kind(value: &CoreValue, env: &RenderEnv) -> WasmValueKind {
    if let CoreValue::BuiltinRuntimeCall { call, .. } = value {
        match call {
            BuiltinMethodCall::VecNew
            | BuiltinMethodCall::VecPush
            | BuiltinMethodCall::VecFreeze
            | BuiltinMethodCall::StringNew
            | BuiltinMethodCall::StringFrom
            | BuiltinMethodCall::StringConcat
            | BuiltinMethodCall::StringAsStr
            | BuiltinMethodCall::StringToCStr
            | BuiltinMethodCall::StringPushRune
            | BuiltinMethodCall::RefNew
            | BuiltinMethodCall::RefSet
            | BuiltinMethodCall::UnsafeRefNew
            | BuiltinMethodCall::UnsafeRefSet => return WasmValueKind::ExternRef,
            BuiltinMethodCall::TextLen { .. }
            | BuiltinMethodCall::TextRuneLen { .. }
            | BuiltinMethodCall::TextCharAt { .. }
            | BuiltinMethodCall::RefGet
            | BuiltinMethodCall::UnsafeRefGet => return WasmValueKind::I32,
        }
    }
    if core_value_is_renderable_externref(value, env) {
        WasmValueKind::ExternRef
    } else {
        WasmValueKind::I32
    }
}

fn collect_operator_targets(ops: &[CoreOp]) -> Vec<&CoreOp> {
    let mut targets = Vec::new();
    for op in ops {
        match op {
            CoreOp::OperatorTarget { .. } => targets.push(op),
            CoreOp::LiftedFunction { body, .. } => {
                targets.extend(collect_operator_targets(body));
            }
            CoreOp::CaptureContinuation { captured, .. } => {
                targets.extend(collect_operator_targets(&captured.ops));
            }
            _ => {}
        }
    }
    targets
}

fn render_continuation_kind(kind: ContinuationKind) -> &'static str {
    match kind {
        ContinuationKind::Cont1 => "cont1",
        ContinuationKind::ContN => "contn",
    }
}

fn render_runtime_let_value(
    wat: &mut String,
    binder: &str,
    value: &CoreValue,
    env: &RenderEnv,
) -> Result<(), BackendDiagnostic> {
    if core_value_is_renderable_externref(value, env) {
        render_core_value_externref(wat, value, env)?;
    } else {
        render_core_value_i32(wat, value, env)?;
    }
    wat.push_str(&format!("    local.set ${}\n", encode_debug_symbol(binder)));
    Ok(())
}

fn render_captured_continuation_i32(
    wat: &mut String,
    captured: &CoreCapturedContinuation,
    arg: &CoreValue,
    env: &RenderEnv,
    core: &CoreProgram,
) -> Result<(), BackendDiagnostic> {
    let captured_locals = collect_runtime_local_binders(&captured.ops);
    let mut env = env
        .with_params_as_locals(
            std::iter::once(captured.param.clone())
                .chain(captured_locals.iter().cloned())
                .collect(),
        )
        .with_binding(&captured.param, arg.clone());

    for (index, op) in captured.ops.iter().enumerate() {
        match op {
            CoreOp::RuntimeLet { binder, value } => {
                wat.push_str(&format!(
                    "    ;; captured-runtime-let {} = {}\n",
                    escape_wat_comment(binder),
                    escape_wat_comment(&value.debug_name())
                ));
                render_runtime_let_value(wat, binder, value, &env)?;
            }
            CoreOp::TailCall { func, args } => {
                let (runtime_func, runtime_args) = effective_tailcall(core, func, args, &env);
                let Some(CoreOp::TailCallResult { binder }) = captured.ops.get(index + 1) else {
                    return Err(BackendDiagnostic::UnsupportedContinuationRuntime {
                        op: "captured-tailcall-without-result".to_string(),
                        kind: continuation_kind_for_captured(core, captured),
                        binder: None,
                    });
                };
                wat.push_str(&format!(
                    "    ;; captured-tailcall {} args=[{}]\n",
                    final_symbol(runtime_func),
                    runtime_args
                        .iter()
                        .map(CoreValue::debug_name)
                        .map(|arg| escape_wat_comment(&arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                render_tailcall_args(wat, core, runtime_func, &runtime_args, &env)?;
                wat.push_str(&format!("    call ${}\n", final_symbol(runtime_func)));
                wat.push_str(&format!("    local.set ${}\n", encode_debug_symbol(binder)));
                env = env.with_locals(vec![binder.clone()]);
            }
            CoreOp::TailCallResult { .. } => {}
            CoreOp::ReturnValue(value) => {
                render_core_value_i32(wat, value, &env)?;
                return Ok(());
            }
            CoreOp::ReturnBranch {
                cond,
                then_value,
                else_value,
            } => {
                render_core_value_i32(wat, cond, &env)?;
                wat.push_str("    if (result i32)\n");
                render_core_value_i32_indented(wat, then_value, &env, 6)?;
                wat.push_str("    else\n");
                render_core_value_i32_indented(wat, else_value, &env, 6)?;
                wat.push_str("    end\n");
                return Ok(());
            }
            CoreOp::ReturnMatch { scrutinee, arms } => {
                render_match_arms_i32(wat, scrutinee, arms, &env, 4)?;
                return Ok(());
            }
            CoreOp::OperatorTarget { .. }
            | CoreOp::DynamicCallableTarget { .. }
            | CoreOp::DynRowParamMethodTarget { .. }
            | CoreOp::ExternFunctionTarget { .. } => {}
            _ => {
                return Err(BackendDiagnostic::UnsupportedContinuationRuntime {
                    op: "captured-context-op".to_string(),
                    kind: continuation_kind_for_captured(core, captured),
                    binder: None,
                });
            }
        }
    }
    Err(BackendDiagnostic::UnsupportedContinuationRuntime {
        op: "captured-context-missing-return".to_string(),
        kind: continuation_kind_for_captured(core, captured),
        binder: None,
    })
}

fn continuation_kind_for_captured(
    core: &CoreProgram,
    captured: &CoreCapturedContinuation,
) -> ContinuationKind {
    core.ops
        .iter()
        .find_map(|op| match op {
            CoreOp::CaptureContinuation {
                kind,
                captured: candidate,
                ..
            } if candidate == captured => Some(*kind),
            _ => None,
        })
        .unwrap_or(ContinuationKind::Cont1)
}

fn continuation_kind_for_binder(core: &CoreProgram, binder: &str) -> ContinuationKind {
    core.ops
        .iter()
        .find_map(|op| match op {
            CoreOp::CaptureContinuation {
                binder: candidate,
                kind,
                ..
            } if candidate == binder => Some(*kind),
            _ => None,
        })
        .unwrap_or(ContinuationKind::Cont1)
}

fn render_operator_intrinsic_wat(
    wat: &mut String,
    protocol: &str,
    target: &str,
    intrinsic: Option<OperatorIntrinsic>,
) {
    let Some(opcode) = operator_intrinsic_opcode(intrinsic) else {
        return;
    };
    wat.push_str(&format!(
        "  ;; operator-intrinsic protocol={} target={}\n",
        escape_wat_comment(protocol),
        escape_wat_comment(target)
    ));
    wat.push_str(&format!(
        "  (func ${} (param $lhs i32) (param $rhs i32) (result i32)\n",
        final_symbol(target)
    ));
    wat.push_str("    local.get $lhs\n");
    wat.push_str("    local.get $rhs\n");
    wat.push_str(&format!("    {opcode}\n"));
    wat.push_str("  )\n");
}

fn render_extern_import_wat(wat: &mut String, import: &BackendExternImport) {
    let Some((params, result)) = extern_import_wat_signature(&import.signature_hash) else {
        return;
    };
    wat.push_str(&format!(
        "  (import \"{}\" \"{}\" (func ${}",
        escape_wat_string(&import.module),
        escape_wat_string(&import.name),
        import.final_symbol
    ));
    for param in params {
        wat.push_str(&format!(" (param {param})"));
    }
    if let Some(result) = result {
        wat.push_str(&format!(" (result {result})"));
    }
    wat.push_str("))\n");
}

fn unsupported_extern_import_signature(manifest: &BackendManifest) -> Option<BackendDiagnostic> {
    manifest.imports.iter().find_map(|import| {
        extern_import_wat_signature(&import.signature_hash)
            .is_none()
            .then(|| BackendDiagnostic::UnsupportedExternImportSignature {
                symbol: import.final_symbol.clone(),
                signature: import.signature_hash.clone(),
            })
    })
}

fn extern_import_wat_signature(
    signature: &str,
) -> Option<(Vec<&'static str>, Option<&'static str>)> {
    let (params, result) = signature.split_once("_to_")?;
    let params = if params.is_empty() {
        Vec::new()
    } else {
        params
            .split('_')
            .map(extern_scalar_wat_type)
            .collect::<Option<Vec<_>>>()?
    };
    let result = match result {
        "unit" | "Unit" => None,
        "externref" => Some("externref"),
        scalar => Some(extern_scalar_wat_type(scalar)?),
    };
    Some((params, result))
}

fn extern_scalar_wat_type(scalar: &str) -> Option<&'static str> {
    match scalar {
        "i64" | "I64" | "bool" | "Bool" => Some("i32"),
        "externref" => Some("externref"),
        _ => None,
    }
}

fn extern_signature_for_target<'a>(
    core: &'a CoreProgram,
    target: &str,
) -> Option<(Vec<WasmValueKind>, Option<WasmValueKind>)> {
    core.ops.iter().find_map(|op| {
        let CoreOp::ExternFunctionTarget {
            target: candidate,
            signature,
            ..
        } = op
        else {
            return None;
        };
        if candidate != target {
            return None;
        }
        let (params, result) = extern_import_wat_signature(signature)?;
        let params = params
            .into_iter()
            .map(wasm_value_kind_from_wat)
            .collect::<Option<Vec<_>>>()?;
        let result = match result {
            Some(result) => Some(wasm_value_kind_from_wat(result)?),
            None => None,
        };
        Some((params, result))
    })
}

fn callable_signature_for_target(
    core: &CoreProgram,
    target: &str,
    env: &RenderEnv,
) -> Option<(Vec<WasmValueKind>, Option<WasmValueKind>)> {
    extern_signature_for_target(core, target)
        .or_else(|| env.callable_params.get(target).map(callable_wasm_signature))
        .or_else(|| env.function_abis.get(target).map(callable_wasm_signature))
}

fn callable_wasm_signature(
    abi: &BackendCallableAbi,
) -> (Vec<WasmValueKind>, Option<WasmValueKind>) {
    (
        abi.params
            .iter()
            .copied()
            .map(WasmValueKind::from)
            .collect(),
        abi.result.map(WasmValueKind::from),
    )
}

fn callable_arg_expansions_for_target<'a>(
    target: &str,
    env: &'a RenderEnv,
) -> Option<&'a [BackendCallableArgExpansion]> {
    env.function_abis
        .get(target)
        .map(|abi| abi.arg_expansions.as_slice())
}

fn returned_callable_env_lane(callable: &str, index: usize) -> String {
    format!("{callable}__returned_env_{index}")
}

fn callable_signature_matches(
    abi: &BackendCallableAbi,
    params: &[BackendValueKind],
    env: &[BackendValueKind],
    result: Option<BackendValueKind>,
) -> bool {
    abi.result == result
        && (abi.params == params
            || (abi.params.len() >= env.len()
                && abi.params[..env.len()] == *env
                && abi.params[env.len()..] == *params))
}

fn callable_candidates(
    params: &[BackendValueKind],
    env_params: &[BackendValueKind],
    result: Option<BackendValueKind>,
    env: &RenderEnv,
) -> Vec<String> {
    env.function_abis
        .iter()
        .filter(|(_, abi)| callable_signature_matches(abi, params, env_params, result))
        .map(|(name, _)| name.clone())
        .collect()
}

fn callable_selector_for_value(
    value: &CoreValue,
    params: &[BackendValueKind],
    env_params: &[BackendValueKind],
    result: Option<BackendValueKind>,
    env: &RenderEnv,
) -> Option<i32> {
    let name = match value {
        CoreValue::Var(name) => name.as_str(),
        CoreValue::LiftedFunction { symbol, .. } => symbol.as_str(),
        _ => return None,
    };
    callable_candidates(params, env_params, result, env)
        .iter()
        .position(|candidate| candidate == name)
        .map(|index| index as i32 + 1)
}

fn callable_env_values(
    value: &CoreValue,
    env_params: &[BackendValueKind],
    env: &RenderEnv,
) -> Option<Vec<CoreValue>> {
    if env_params.is_empty() {
        return Some(Vec::new());
    }
    let CoreValue::LiftedFunction { source, .. } = value else {
        return Some(zero_callable_env_values(env_params));
    };
    if env.closure_env_params(source).is_none() {
        return Some(zero_callable_env_values(env_params));
    }
    let capture_name = closure_env_capture_name(source, 0, env)?;
    let values = env_params
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            let capture = if index == 0 {
                capture_name.clone()
            } else {
                closure_env_capture_name(source, index, env)?
            };
            let value = CoreValue::Var(capture);
            match WasmValueKind::from(*kind) {
                WasmValueKind::I32 if core_value_is_renderable_i32(&value, env) => Some(value),
                WasmValueKind::ExternRef if core_value_is_renderable_externref(&value, env) => {
                    Some(value)
                }
                _ => None,
            }
        })
        .collect::<Option<Vec<_>>>()?;
    Some(values)
}

fn zero_callable_env_values(env_params: &[BackendValueKind]) -> Vec<CoreValue> {
    env_params.iter().map(|_| CoreValue::I64(0)).collect()
}

fn closure_env_capture_name(closure: &str, index: usize, env: &RenderEnv) -> Option<String> {
    env.closure_env_params(closure)
        .and_then(|params| params.get(index).cloned())
}

fn wasm_value_kind_from_wat(wat: &str) -> Option<WasmValueKind> {
    match wat {
        "i32" => Some(WasmValueKind::I32),
        "externref" => Some(WasmValueKind::ExternRef),
        _ => None,
    }
}

fn escape_wat_string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_tailcall_result_chain(
    wat: &mut String,
    core: &CoreProgram,
    env: &RenderEnv,
) -> Result<bool, BackendDiagnostic> {
    let mut result_binders = Vec::new();
    for op in &core.ops {
        if let CoreOp::TailCallResult { binder } = op {
            result_binders.push(binder.clone());
        }
    }
    if result_binders.is_empty()
        || (result_binders.len() == 1
            && tailcall_chain_returns_result_binder(core, &result_binders[0]))
    {
        return Ok(false);
    }
    let chain_env = env_with_tailcall_result_facts(core, &env.with_locals(result_binders.clone()));
    wat.push_str("  ;; tailcall-result-chain\n");
    render_func_header(wat, "main", Some("main"), &chain_env);
    for binder in &result_binders {
        wat.push_str(&format!(
            "    (local ${} {})\n",
            encode_debug_symbol(binder),
            chain_env.param_kind(binder).wat_type()
        ));
        if let Some(callable) = chain_env.returned_callable_local(binder) {
            for (index, kind) in callable.env.iter().enumerate() {
                wat.push_str(&format!(
                    "    (local ${} {})\n",
                    encode_debug_symbol(&returned_callable_env_lane(binder, index)),
                    WasmValueKind::from(*kind).wat_type()
                ));
            }
        }
    }

    let mut local_chain_env = chain_env.clone();
    for (index, op) in core.ops.iter().enumerate() {
        if let CoreOp::TailCall { func, args } = op {
            let (runtime_func, runtime_args) =
                effective_tailcall(core, func, args, &local_chain_env);
            let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
                continue;
            };
            wat.push_str(&format!(
                "    ;; tailcall {} args=[{}]\n",
                final_symbol(runtime_func),
                runtime_args
                    .iter()
                    .map(CoreValue::debug_name)
                    .map(|arg| escape_wat_comment(&arg))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            render_tailcall_args(wat, core, runtime_func, &runtime_args, &local_chain_env)?;
            wat.push_str(&format!("    call ${}\n", final_symbol(runtime_func)));
            render_tailcall_result_store(wat, binder, runtime_func, &local_chain_env);
            local_chain_env = env_after_tailcall_result(binder, runtime_func, &local_chain_env);
        }
    }

    match core.ops.iter().find_map(|op| {
        if let CoreOp::ReturnValue(value) = op {
            Some(value)
        } else {
            None
        }
    }) {
        Some(value) => render_core_value_i32(wat, value, &chain_env)?,
        None => wat.push_str("    i32.const 0\n"),
    }
    wat.push_str("  )\n");
    Ok(true)
}

fn render_tailcall_result_store(
    wat: &mut String,
    binder: &str,
    runtime_func: &str,
    env: &RenderEnv,
) {
    if let Some(callable) = env
        .function_abi(runtime_func)
        .and_then(|abi| abi.return_callable.as_ref())
    {
        for (index, _) in callable.env.iter().enumerate().rev() {
            wat.push_str(&format!(
                "    local.set ${}\n",
                encode_debug_symbol(&returned_callable_env_lane(binder, index))
            ));
        }
    }
    wat.push_str(&format!("    local.set ${}\n", encode_debug_symbol(binder)));
}

fn env_after_tailcall_result(binder: &str, runtime_func: &str, env: &RenderEnv) -> RenderEnv {
    let Some(abi) = env.function_abi(runtime_func) else {
        return env.with_local_kind(binder, WasmValueKind::I32);
    };
    if let Some(callable) = abi.return_callable.as_ref().cloned() {
        env.with_returned_callable_local(binder, callable)
    } else if !abi.return_dyn_row_methods.is_empty() {
        env.with_returned_dyn_row_local(binder, abi.return_dyn_row_methods.clone())
            .with_local_kind(
                binder,
                abi.result
                    .map(WasmValueKind::from)
                    .unwrap_or(WasmValueKind::I32),
            )
    } else {
        env.with_local_kind(
            binder,
            abi.result
                .map(WasmValueKind::from)
                .unwrap_or(WasmValueKind::I32),
        )
    }
}

fn tailcall_chain_returns_result_binder(core: &CoreProgram, binder: &str) -> bool {
    core.ops
        .iter()
        .any(|op| matches!(op, CoreOp::ReturnValue(CoreValue::Var(name)) if name == binder))
}

fn tailcall_result_kinds(core: &CoreProgram, env: &RenderEnv) -> BTreeMap<String, WasmValueKind> {
    let mut kinds = BTreeMap::new();
    for (index, op) in core.ops.iter().enumerate() {
        let CoreOp::TailCall { func, .. } = op else {
            continue;
        };
        let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
            continue;
        };
        let (runtime_func, _) = effective_tailcall(core, func, &[], env);
        if let Some(kind) = tailcall_result_kind(core, runtime_func, env) {
            kinds.insert(binder.clone(), kind);
        }
    }
    kinds
}

fn tailcall_result_ref_cell_lanes(
    core: &CoreProgram,
    env: &RenderEnv,
) -> BTreeMap<String, CoreRefCellLane> {
    let mut lanes = BTreeMap::new();
    for (index, op) in core.ops.iter().enumerate() {
        let CoreOp::TailCall { func, .. } = op else {
            continue;
        };
        let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
            continue;
        };
        let (runtime_func, _) = effective_tailcall(core, func, &[], env);
        if let Some(lane) = tailcall_result_ref_cell_lane(core, runtime_func, env) {
            lanes.insert(binder.clone(), lane);
        }
    }
    lanes
}

fn env_with_tailcall_result_facts(core: &CoreProgram, env: &RenderEnv) -> RenderEnv {
    let mut next = env.clone();
    for binder in tailcall_result_binders(core) {
        next = next.with_locals(vec![binder]);
    }
    for (index, op) in core.ops.iter().enumerate() {
        let CoreOp::TailCall { func, args } = op else {
            continue;
        };
        let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
            continue;
        };
        let (runtime_func, _) = effective_tailcall(core, func, args, &next);
        next = env_after_tailcall_result(binder, runtime_func, &next);
    }
    for (binder, kind) in tailcall_result_kinds(core, env) {
        next = next.with_local_kind(&binder, kind);
    }
    for (binder, lane) in tailcall_result_ref_cell_lanes(core, env) {
        next = next.with_local_ref_cell_lane(&binder, lane);
    }
    next
}

fn tailcall_result_binders(core: &CoreProgram) -> Vec<String> {
    core.ops
        .iter()
        .filter_map(|op| {
            if let CoreOp::TailCallResult { binder } = op {
                Some(binder.clone())
            } else {
                None
            }
        })
        .collect()
}

fn operator_intrinsic_opcode(intrinsic: Option<OperatorIntrinsic>) -> Option<&'static str> {
    match intrinsic {
        Some(OperatorIntrinsic::I64Add) => Some("i32.add"),
        Some(OperatorIntrinsic::I64Sub) => Some("i32.sub"),
        Some(OperatorIntrinsic::I64Mul) => Some("i32.mul"),
        Some(OperatorIntrinsic::I64Div) => Some("i32.div_s"),
        None => None,
    }
}

#[derive(Clone)]
struct RenderEnv {
    params: BTreeSet<String>,
    locals: BTreeMap<String, WasmValueKind>,
    local_ref_cell_lanes: BTreeMap<String, CoreRefCellLane>,
    param_kinds: BTreeMap<String, WasmValueKind>,
    function_abis: BTreeMap<String, BackendCallableAbi>,
    callable_params: BTreeMap<String, BackendCallableAbi>,
    static_kinds: BTreeMap<String, WasmValueKind>,
    static_ref_cell_lanes: BTreeMap<String, CoreRefCellLane>,
    aggregate_element_lanes: BTreeMap<String, WasmValueKind>,
    dyn_row_param_fields: BTreeMap<String, Vec<BackendDynRowParamFieldAbi>>,
    dyn_row_param_methods: BTreeMap<String, Vec<BackendDynRowParamMethodAbi>>,
    closure_env_params: BTreeMap<String, Vec<String>>,
    returned_callable_locals: BTreeMap<String, BackendReturnedCallableAbi>,
    returned_dyn_row_locals: BTreeMap<String, Vec<BackendDynRowParamMethodAbi>>,
    bindings: BTreeMap<String, CoreValue>,
}

impl RenderEnv {
    fn new(params: &[String], param_kinds: &BTreeMap<String, WasmValueKind>) -> Self {
        Self {
            params: params.iter().cloned().collect(),
            locals: BTreeMap::new(),
            local_ref_cell_lanes: BTreeMap::new(),
            param_kinds: param_kinds.clone(),
            function_abis: BTreeMap::new(),
            callable_params: BTreeMap::new(),
            static_kinds: BTreeMap::new(),
            static_ref_cell_lanes: BTreeMap::new(),
            aggregate_element_lanes: BTreeMap::new(),
            dyn_row_param_fields: BTreeMap::new(),
            dyn_row_param_methods: BTreeMap::new(),
            closure_env_params: BTreeMap::new(),
            returned_callable_locals: BTreeMap::new(),
            returned_dyn_row_locals: BTreeMap::new(),
            bindings: BTreeMap::new(),
        }
    }

    fn from_param_abi(
        params: &[String],
        param_kinds: &BTreeMap<String, WasmValueKind>,
        param_abi: &BackendParamAbi,
    ) -> Self {
        Self::new(params, param_kinds)
            .with_function_abis(param_abi.functions.clone())
            .with_callable_params(param_abi.callable_params.clone())
            .with_static_kinds(param_abi.statics.clone())
            .with_static_ref_cell_lanes(param_abi.static_ref_cell_lanes.clone())
            .with_aggregate_element_lanes(param_abi.aggregate_element_lanes.clone())
            .with_dyn_row_param_fields(param_abi.dyn_row_param_fields.clone())
            .with_dyn_row_param_methods(param_abi.dyn_row_param_methods.clone())
    }

    fn with_function_abis(&self, function_abis: BTreeMap<String, BackendCallableAbi>) -> Self {
        let mut next = self.clone();
        next.function_abis = function_abis;
        next
    }

    fn with_lifted_function_abis(&self, core: &CoreProgram) -> Self {
        let mut next = self.clone();
        next.closure_env_params = core
            .layouts
            .iter()
            .filter_map(|layout| {
                let crate::core::LayoutKind::ClosureEnv(env) = &layout.kind else {
                    return None;
                };
                Some((
                    env.closure.clone(),
                    env.fields
                        .iter()
                        .map(|field| field.name.clone())
                        .collect::<Vec<_>>(),
                ))
            })
            .collect();
        for op in &core.ops {
            let CoreOp::LiftedFunction {
                symbol,
                env_params,
                param,
                body,
                ..
            } = op
            else {
                continue;
            };
            if param.is_none() || body.is_empty() {
                continue;
            }
            let mut params = vec![BackendValueKind::I32; env_params.len()];
            params.push(BackendValueKind::I32);
            next.function_abis.insert(
                symbol.clone(),
                BackendCallableAbi {
                    params,
                    arg_expansions: vec![BackendCallableArgExpansion::Direct(
                        BackendValueKind::I32,
                    )],
                    result: Some(BackendValueKind::I32),
                    result_ref_cell_lane: None,
                    return_callable: None,
                    return_dyn_row_methods: Vec::new(),
                },
            );
        }
        next
    }

    fn with_callable_params(&self, callable_params: BTreeMap<String, BackendCallableAbi>) -> Self {
        let mut next = self.clone();
        for (param, callable) in &callable_params {
            for (index, kind) in callable
                .params
                .iter()
                .take(callable.params.len().saturating_sub(1))
                .enumerate()
            {
                next = next.with_local_kind(
                    &callable_param_env_lane(param, index),
                    WasmValueKind::from(*kind),
                );
            }
        }
        next.callable_params = callable_params;
        next
    }

    fn with_static_kinds(&self, static_kinds: BTreeMap<String, BackendValueKind>) -> Self {
        let mut next = self.clone();
        next.static_kinds = static_kinds
            .into_iter()
            .map(|(name, kind)| (name, WasmValueKind::from(kind)))
            .collect();
        next
    }

    fn with_static_ref_cell_lanes(
        &self,
        static_ref_cell_lanes: BTreeMap<String, CoreRefCellLane>,
    ) -> Self {
        let mut next = self.clone();
        next.static_ref_cell_lanes = static_ref_cell_lanes;
        next
    }

    fn with_aggregate_element_lanes(
        &self,
        aggregate_element_lanes: BTreeMap<String, BackendValueKind>,
    ) -> Self {
        let mut next = self.clone();
        next.aggregate_element_lanes = aggregate_element_lanes
            .into_iter()
            .map(|(name, kind)| (name, WasmValueKind::from(kind)))
            .collect();
        next
    }

    fn with_dyn_row_param_fields(
        &self,
        dyn_row_param_fields: BTreeMap<String, Vec<BackendDynRowParamFieldAbi>>,
    ) -> Self {
        let mut next = self.clone();
        for (param, fields) in &dyn_row_param_fields {
            for field in fields {
                next.locals.insert(
                    dyn_row_param_field_lane(param, &field.field),
                    WasmValueKind::from(field.kind),
                );
            }
        }
        next.dyn_row_param_fields = dyn_row_param_fields;
        next
    }

    fn with_dyn_row_param_methods(
        &self,
        dyn_row_param_methods: BTreeMap<String, Vec<BackendDynRowParamMethodAbi>>,
    ) -> Self {
        let mut next = self.clone();
        next.dyn_row_param_methods = dyn_row_param_methods;
        next
    }

    fn dyn_row_param_method_target(&self, param: &str, field: &str) -> Option<&str> {
        self.dyn_row_param_methods
            .get(param)?
            .iter()
            .find(|method| method.field == field)
            .map(|method| method.target.as_str())
    }

    fn dyn_row_value_method_target(&self, value: &str, field: &str) -> Option<&str> {
        self.dyn_row_param_method_target(value, field).or_else(|| {
            self.returned_dyn_row_locals
                .get(value)?
                .iter()
                .find(|method| method.field == field)
                .map(|method| method.target.as_str())
        })
    }

    fn with_returned_dyn_row_local(
        &self,
        local: &str,
        methods: Vec<BackendDynRowParamMethodAbi>,
    ) -> Self {
        let mut next = self.clone();
        next.returned_dyn_row_locals
            .insert(local.to_string(), methods);
        next
    }

    fn dyn_row_param_field(&self, param: &str, field: &str) -> Option<&BackendDynRowParamFieldAbi> {
        self.dyn_row_param_fields
            .get(param)?
            .iter()
            .find(|candidate| candidate.field == field)
    }

    fn dyn_row_param_field_lane(&self, param: &str, field: &str) -> Option<String> {
        self.dyn_row_param_field(param, field)
            .map(|field| dyn_row_param_field_lane(param, &field.field))
    }

    fn has_dyn_row_param_fields(&self, param: &str) -> bool {
        self.dyn_row_param_fields.contains_key(param)
    }

    fn dyn_row_param_single_data_field_lane(&self, param: &str) -> Option<String> {
        let fields = self.dyn_row_param_fields.get(param)?;
        let [field] = fields.as_slice() else {
            return None;
        };
        Some(dyn_row_param_field_lane(param, &field.field))
    }

    fn is_param(&self, name: &str) -> bool {
        self.params.contains(name)
            || self.locals.contains_key(name)
            || self.static_kinds.contains_key(name)
    }

    fn is_static(&self, name: &str) -> bool {
        self.static_kinds.contains_key(name)
    }

    fn static_symbol(&self, name: &str) -> String {
        format!("global__{}", encode_debug_symbol(name))
    }

    fn binding(&self, name: &str) -> Option<&CoreValue> {
        self.bindings.get(name)
    }

    fn with_binding(&self, name: &str, value: CoreValue) -> Self {
        let mut next = self.clone();
        next.bindings.insert(name.to_string(), value);
        next
    }

    fn with_locals(&self, names: Vec<String>) -> Self {
        let mut next = self.clone();
        for name in names {
            next.locals.entry(name).or_insert(WasmValueKind::I32);
        }
        next
    }

    fn with_params_as_locals(&self, names: Vec<String>) -> Self {
        let mut next = self.with_locals(names.clone());
        next.params.extend(names);
        next
    }

    fn with_local_kind(&self, name: &str, kind: WasmValueKind) -> Self {
        let mut next = self.clone();
        next.locals.insert(name.to_string(), kind);
        next
    }

    fn with_local_kinds(&self, kinds: BTreeMap<String, WasmValueKind>) -> Self {
        let mut next = self.clone();
        next.locals.extend(kinds);
        next
    }

    fn with_local_ref_cell_lane(&self, name: &str, lane: CoreRefCellLane) -> Self {
        let mut next = self.clone();
        next.local_ref_cell_lanes.insert(name.to_string(), lane);
        next
    }

    fn local_ref_cell_lane(&self, name: &str) -> Option<CoreRefCellLane> {
        self.local_ref_cell_lanes
            .get(name)
            .copied()
            .or_else(|| self.static_ref_cell_lanes.get(name).copied())
    }

    fn aggregate_element_lane(&self, name: &str) -> Option<WasmValueKind> {
        self.aggregate_element_lanes.get(name).copied()
    }

    fn callable_param(&self, name: &str) -> Option<&BackendCallableAbi> {
        self.callable_params.get(name)
    }

    fn function_abi(&self, name: &str) -> Option<&BackendCallableAbi> {
        self.function_abis.get(name)
    }

    fn returned_callable_local(&self, name: &str) -> Option<&BackendReturnedCallableAbi> {
        self.returned_callable_locals.get(name)
    }

    fn with_returned_callable_local(
        &self,
        name: &str,
        callable: BackendReturnedCallableAbi,
    ) -> Self {
        let mut next = self.clone();
        next.locals.insert(name.to_string(), WasmValueKind::I32);
        for (index, kind) in callable.env.iter().enumerate() {
            next.locals.insert(
                returned_callable_env_lane(name, index),
                WasmValueKind::from(*kind),
            );
        }
        next.returned_callable_locals
            .insert(name.to_string(), callable);
        next
    }

    fn closure_env_params(&self, closure: &str) -> Option<&Vec<String>> {
        self.closure_env_params.get(closure)
    }

    fn signature(&self) -> String {
        let mut out = String::new();
        for param in &self.params {
            if let Some(fields) = self.dyn_row_param_fields.get(param) {
                for field in fields {
                    out.push_str(&format!(
                        " (param ${} {})",
                        encode_debug_symbol(&dyn_row_param_field_lane(param, &field.field)),
                        WasmValueKind::from(field.kind).wat_type()
                    ));
                }
            } else if let Some(callable) = self.callable_params.get(param) {
                out.push_str(&format!(" (param ${} i32)", encode_debug_symbol(param)));
                for (index, kind) in callable
                    .params
                    .iter()
                    .take(callable.params.len().saturating_sub(1))
                    .enumerate()
                {
                    out.push_str(&format!(
                        " (param ${} {})",
                        encode_debug_symbol(&callable_param_env_lane(param, index)),
                        WasmValueKind::from(*kind).wat_type()
                    ));
                }
            } else {
                out.push_str(&format!(
                    " (param ${} {})",
                    encode_debug_symbol(param),
                    self.param_kind(param).wat_type()
                ));
            }
        }
        out
    }

    fn param_kind(&self, name: &str) -> WasmValueKind {
        self.locals
            .get(name)
            .copied()
            .or_else(|| self.param_kinds.get(name).copied())
            .or_else(|| self.static_kinds.get(name).copied())
            .unwrap_or(WasmValueKind::I32)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WasmValueKind {
    I32,
    ExternRef,
}

impl From<BackendValueKind> for WasmValueKind {
    fn from(value: BackendValueKind) -> Self {
        match value {
            BackendValueKind::I32 => Self::I32,
            BackendValueKind::ExternRef => Self::ExternRef,
        }
    }
}

impl From<WasmValueKind> for RuntimeValueLane {
    fn from(value: WasmValueKind) -> Self {
        match value {
            WasmValueKind::I32 => Self::I32,
            WasmValueKind::ExternRef => Self::ExternRef,
        }
    }
}

impl WasmValueKind {
    fn wat_type(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::ExternRef => "externref",
        }
    }
}

fn infer_param_kinds(
    core: &CoreProgram,
    params: &[String],
    param_abi: &BackendParamAbi,
) -> BTreeMap<String, WasmValueKind> {
    let param_names = params.iter().cloned().collect::<BTreeSet<_>>();
    let mut kinds = param_abi
        .params
        .iter()
        .filter(|(name, _)| param_names.contains(*name))
        .map(|(name, kind)| (name.clone(), WasmValueKind::from(*kind)))
        .collect::<BTreeMap<_, _>>();
    for op in &core.ops {
        infer_param_kinds_from_op(op, &param_names, &mut kinds);
    }
    kinds
}

fn infer_param_kinds_from_op(
    op: &CoreOp,
    params: &BTreeSet<String>,
    kinds: &mut BTreeMap<String, WasmValueKind>,
) {
    match op {
        CoreOp::ReturnValue(value) => infer_param_kinds_from_value(value, params, kinds),
        CoreOp::RuntimeLet { value, .. } => infer_param_kinds_from_value(value, params, kinds),
        CoreOp::ReturnBranch {
            cond,
            then_value,
            else_value,
        } => {
            infer_param_kinds_from_value(cond, params, kinds);
            infer_param_kinds_from_value(then_value, params, kinds);
            infer_param_kinds_from_value(else_value, params, kinds);
        }
        CoreOp::ReturnMatch { scrutinee, arms } => {
            infer_param_kinds_from_value(scrutinee, params, kinds);
            for arm in arms {
                infer_param_kinds_from_value(&arm.value, params, kinds);
            }
        }
        CoreOp::TailCall { args, .. } => {
            for arg in args {
                infer_param_kinds_from_value(arg, params, kinds);
            }
        }
        CoreOp::CaptureContinuation { captured, .. } => {
            for captured_op in &captured.ops {
                infer_param_kinds_from_op(captured_op, params, kinds);
            }
        }
        _ => {}
    }
}

fn infer_param_kinds_from_value(
    value: &CoreValue,
    params: &BTreeSet<String>,
    kinds: &mut BTreeMap<String, WasmValueKind>,
) {
    match value {
        CoreValue::AggregateField { value, .. } => {
            mark_externref_operand(value, params, kinds);
            infer_param_kinds_from_value(value, params, kinds);
        }
        CoreValue::TextField { value, .. } => {
            mark_externref_operand(value, params, kinds);
            infer_param_kinds_from_value(value, params, kinds);
        }
        CoreValue::AggregateIndex { value, index, .. } => {
            mark_externref_operand(value, params, kinds);
            infer_param_kinds_from_value(value, params, kinds);
            infer_param_kinds_from_value(index, params, kinds);
        }
        CoreValue::AggregateSlice { value, range, .. } => {
            mark_externref_operand(value, params, kinds);
            infer_param_kinds_from_value(value, params, kinds);
            infer_param_kinds_from_value(range, params, kinds);
        }
        CoreValue::TextIndex { value, index, .. } => {
            mark_externref_operand(value, params, kinds);
            infer_param_kinds_from_value(value, params, kinds);
            infer_param_kinds_from_value(index, params, kinds);
        }
        CoreValue::TextSlice { value, range, .. } => {
            mark_externref_operand(value, params, kinds);
            infer_param_kinds_from_value(value, params, kinds);
            infer_param_kinds_from_value(range, params, kinds);
        }
        CoreValue::BuiltinRuntimeCall { args, .. } => {
            if let Some(receiver) = args.first() {
                mark_externref_operand(receiver, params, kinds);
            }
            for arg in args {
                infer_param_kinds_from_value(arg, params, kinds);
            }
        }
        CoreValue::Tuple { fields } => {
            for field in fields {
                infer_param_kinds_from_value(field, params, kinds);
            }
        }
        CoreValue::TupleField { tuple, .. } => infer_param_kinds_from_value(tuple, params, kinds),
        CoreValue::Range { start, end } => {
            infer_param_kinds_from_value(start, params, kinds);
            infer_param_kinds_from_value(end, params, kinds);
        }
        CoreValue::RangeField { range, .. } => infer_param_kinds_from_value(range, params, kinds),
        CoreValue::Record { fields } => {
            for field in fields {
                infer_param_kinds_from_value(&field.value, params, kinds);
            }
        }
        CoreValue::RecordUpdate { base, fields } => {
            infer_param_kinds_from_value(base, params, kinds);
            for field in fields {
                infer_param_kinds_from_value(&field.value, params, kinds);
            }
        }
        CoreValue::RecordField { record, .. } => {
            infer_param_kinds_from_value(record, params, kinds)
        }
        CoreValue::DynRowPackage { payload, .. } => {
            infer_param_kinds_from_value(payload, params, kinds);
        }
        CoreValue::DynRowField { package, .. } => {
            infer_param_kinds_from_value(package, params, kinds);
        }
        CoreValue::Adt { args, .. } => {
            for arg in args {
                infer_param_kinds_from_value(arg, params, kinds);
            }
        }
        CoreValue::Unit
        | CoreValue::I64(_)
        | CoreValue::Bool(_)
        | CoreValue::Var(_)
        | CoreValue::TextLiteral { .. }
        | CoreValue::SliceLiteral { .. }
        | CoreValue::LiftedFunction { .. }
        | CoreValue::Rendered { .. } => {}
    }
}

fn mark_externref_operand(
    value: &CoreValue,
    params: &BTreeSet<String>,
    kinds: &mut BTreeMap<String, WasmValueKind>,
) {
    if let CoreValue::Var(name) = value {
        if params.contains(name) {
            kinds.insert(name.clone(), WasmValueKind::ExternRef);
        }
    }
}

fn render_func_header(wat: &mut String, symbol: &str, export: Option<&str>, env: &RenderEnv) {
    match export {
        Some(export) => wat.push_str(&format!(
            "  (func ${symbol} (export \"{export}\"){} (result i32)\n",
            env.signature()
        )),
        None => wat.push_str(&format!(
            "  (func ${symbol}{} (result i32)\n",
            env.signature()
        )),
    }
}

fn tailcall_args_are_renderable(
    core: &CoreProgram,
    func: &str,
    args: &[CoreValue],
    env: &RenderEnv,
) -> bool {
    if let Some(callable) = env.callable_param(func) {
        return unsupported_callable_param_tailcall_args(func, callable, args, env).is_none();
    }
    if let Some(expansions) = callable_arg_expansions_for_target(func, env)
        .filter(|expansions| expansions.len() == args.len())
    {
        return expansions
            .iter()
            .zip(args)
            .all(|(expansion, arg)| match expansion {
                BackendCallableArgExpansion::Direct(kind) => match WasmValueKind::from(*kind) {
                    WasmValueKind::I32 => core_value_is_renderable_i32(arg, env),
                    WasmValueKind::ExternRef => core_value_is_renderable_externref(arg, env),
                },
                BackendCallableArgExpansion::Callable {
                    params,
                    env: callable_env,
                    result,
                } => {
                    callable_selector_for_value(arg, params, callable_env, *result, env).is_some()
                        && callable_env_values(arg, callable_env, env).is_some()
                }
                BackendCallableArgExpansion::DynRowFields(fields) => fields.iter().all(|field| {
                    dyn_row_data_field_value(arg, &field.field, env)
                        .map(|value| match WasmValueKind::from(field.kind) {
                            WasmValueKind::I32 => core_value_is_renderable_i32(&value, env),
                            WasmValueKind::ExternRef => {
                                core_value_is_renderable_externref(&value, env)
                            }
                        })
                        .unwrap_or(false)
                }),
            });
    }
    let Some((params, _)) = callable_signature_for_target(core, func, env) else {
        return args
            .iter()
            .all(|arg| core_value_is_renderable_i32(arg, env));
    };
    params.len() == args.len()
        && params.iter().zip(args).all(|(kind, arg)| match kind {
            WasmValueKind::I32 => core_value_is_renderable_i32(arg, env),
            WasmValueKind::ExternRef => core_value_is_renderable_externref(arg, env),
        })
}

fn tailcall_runtime_target<'a>(core: &'a CoreProgram, func: &'a str) -> &'a str {
    core.ops
        .iter()
        .find_map(|op| {
            let CoreOp::CallableAlias { target, value } = op else {
                return None;
            };
            if target != func {
                return None;
            }
            match value {
                CoreValue::Var(name) => Some(name.as_str()),
                _ => None,
            }
        })
        .unwrap_or(func)
}

fn effective_tailcall<'a>(
    core: &'a CoreProgram,
    func: &'a str,
    args: &'a [CoreValue],
    env: &'a RenderEnv,
) -> (&'a str, Vec<CoreValue>) {
    if let Some(callable) = env.returned_callable_local(func) {
        let mut effective_args = callable
            .env
            .iter()
            .enumerate()
            .map(|(index, _)| CoreValue::Var(returned_callable_env_lane(func, index)))
            .collect::<Vec<_>>();
        effective_args.extend(args.iter().cloned());
        return (callable.target.as_str(), effective_args);
    }
    if let Some((param, field)) = core.ops.iter().find_map(|op| {
        let CoreOp::DynRowParamMethodTarget {
            target,
            param,
            field,
        } = op
        else {
            return None;
        };
        (target == func).then_some((param.as_str(), field.as_str()))
    }) {
        if let Some(target) = env.dyn_row_value_method_target(param, field) {
            let mut effective_args = Vec::with_capacity(args.len() + 1);
            effective_args.push(CoreValue::Var(param.to_string()));
            effective_args.extend(args.iter().cloned());
            return (target, effective_args);
        }
    }
    (tailcall_runtime_target(core, func), args.to_vec())
}

fn render_tailcall_args(
    wat: &mut String,
    core: &CoreProgram,
    func: &str,
    args: &[CoreValue],
    env: &RenderEnv,
) -> Result<(), BackendDiagnostic> {
    if let Some(expansions) = callable_arg_expansions_for_target(func, env)
        .filter(|expansions| expansions.len() == args.len())
    {
        if expansions.len() != args.len() {
            return Err(BackendDiagnostic::UnsupportedI32ReturnValue {
                value: format!("{} /{}", final_symbol(func), args.len()),
            });
        }
        for (expansion, arg) in expansions.iter().zip(args) {
            match expansion {
                BackendCallableArgExpansion::Direct(kind) => match WasmValueKind::from(*kind) {
                    WasmValueKind::I32 => render_core_value_i32(wat, arg, env)?,
                    WasmValueKind::ExternRef => render_core_value_externref(wat, arg, env)?,
                },
                BackendCallableArgExpansion::Callable {
                    params,
                    env: callable_env,
                    result,
                } => {
                    let selector =
                        callable_selector_for_value(arg, params, callable_env, *result, env)
                            .ok_or_else(|| BackendDiagnostic::UnsupportedI32ReturnValue {
                                value: arg.debug_name(),
                            })?;
                    wat.push_str(&format!("    i32.const {selector}\n"));
                    let env_values =
                        callable_env_values(arg, callable_env, env).ok_or_else(|| {
                            BackendDiagnostic::UnsupportedI32ReturnValue {
                                value: arg.debug_name(),
                            }
                        })?;
                    for (kind, value) in callable_env.iter().zip(env_values) {
                        match WasmValueKind::from(*kind) {
                            WasmValueKind::I32 => render_core_value_i32(wat, &value, env)?,
                            WasmValueKind::ExternRef => {
                                render_core_value_externref(wat, &value, env)?
                            }
                        }
                    }
                }
                BackendCallableArgExpansion::DynRowFields(fields) => {
                    for field in fields {
                        let value =
                            dyn_row_data_field_value(arg, &field.field, env).ok_or_else(|| {
                                BackendDiagnostic::UnsupportedI32ReturnValue {
                                    value: format!("{}.{}", arg.debug_name(), field.field),
                                }
                            })?;
                        match WasmValueKind::from(field.kind) {
                            WasmValueKind::I32 => render_core_value_i32(wat, &value, env)?,
                            WasmValueKind::ExternRef => {
                                render_core_value_externref(wat, &value, env)?
                            }
                        }
                    }
                }
            }
        }
        return Ok(());
    }
    let Some((params, _)) = callable_signature_for_target(core, func, env) else {
        for arg in args {
            render_core_value_i32(wat, arg, env)?;
        }
        return Ok(());
    };
    if params.len() != args.len() {
        return Err(BackendDiagnostic::UnsupportedI32ReturnValue {
            value: format!("{} /{}", final_symbol(func), args.len()),
        });
    }
    for (kind, arg) in params.iter().zip(args) {
        match kind {
            WasmValueKind::I32 => render_core_value_i32(wat, arg, env)?,
            WasmValueKind::ExternRef => render_core_value_externref(wat, arg, env)?,
        }
    }
    Ok(())
}

fn render_callable_param_call(
    wat: &mut String,
    func: &str,
    callable: &BackendCallableAbi,
    args: &[CoreValue],
    env: &RenderEnv,
) -> Result<(), BackendDiagnostic> {
    let env_param_count = callable.params.len().saturating_sub(args.len());
    let env_param_kinds = &callable.params[..env_param_count];
    let call_param_kinds = &callable.params[env_param_count..];
    let candidates = callable_candidates(call_param_kinds, env_param_kinds, callable.result, env);
    if candidates.is_empty() {
        return Err(BackendDiagnostic::UnsupportedI32ReturnValue {
            value: func.to_string(),
        });
    }
    render_core_value_i32(wat, &CoreValue::Var(func.to_string()), env)?;
    for (index, candidate) in candidates.iter().enumerate() {
        if index == 0 {
            wat.push_str(&format!("    i32.const {}\n", index + 1));
            wat.push_str("    i32.eq\n");
            wat.push_str("    if (result i32)\n");
        } else {
            wat.push_str("    else\n");
            render_core_value_i32(wat, &CoreValue::Var(func.to_string()), env)?;
            wat.push_str(&format!("    i32.const {}\n", index + 1));
            wat.push_str("    i32.eq\n");
            wat.push_str("    if (result i32)\n");
        }
        let candidate_env_param_count = env
            .function_abis
            .get(candidate)
            .map(|abi| abi.params.len().saturating_sub(args.len()))
            .unwrap_or(env_param_kinds.len());
        for (index, kind) in env_param_kinds
            .iter()
            .take(candidate_env_param_count)
            .enumerate()
        {
            let lane = CoreValue::Var(callable_param_env_lane(func, index));
            match WasmValueKind::from(*kind) {
                WasmValueKind::I32 => render_core_value_i32(wat, &lane, env)?,
                WasmValueKind::ExternRef => render_core_value_externref(wat, &lane, env)?,
            }
        }
        for (kind, arg) in call_param_kinds.iter().zip(args) {
            match WasmValueKind::from(*kind) {
                WasmValueKind::I32 => render_core_value_i32(wat, arg, env)?,
                WasmValueKind::ExternRef => render_core_value_externref(wat, arg, env)?,
            }
        }
        wat.push_str(&format!("      call ${}\n", final_symbol(candidate)));
    }
    wat.push_str("    else\n");
    wat.push_str("      i32.const 0\n");
    for _ in candidates.iter().skip(1) {
        wat.push_str("    end\n");
    }
    wat.push_str("    end\n");
    Ok(())
}

fn render_lifted_function_wat(
    wat: &mut String,
    symbol: &str,
    env_params: &[String],
    param: Option<&str>,
    body: &[CoreOp],
    env: &RenderEnv,
) -> Result<(), BackendDiagnostic> {
    let Some(param) = param else {
        return Ok(());
    };
    let body_core = CoreProgram {
        ops: body.to_vec(),
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    let mut local_names = env_params.to_vec();
    local_names.push(param.to_string());
    let mut local_env = env.with_params_as_locals(local_names);
    for env_param in env_params {
        local_env = local_env.with_local_kind(env_param, WasmValueKind::I32);
    }
    let local_env = local_env.with_local_kind(param, WasmValueKind::I32);
    let local_env = env_with_tailcall_result_facts(&body_core, &local_env);
    wat.push_str(&format!("  (func ${}", final_symbol(symbol)));
    for env_param in env_params {
        wat.push_str(&format!(" (param ${} i32)", encode_debug_symbol(env_param)));
    }
    wat.push_str(&format!(
        " (param ${} i32) (result i32)\n",
        encode_debug_symbol(param)
    ));
    for binder in tailcall_result_binders(&body_core) {
        wat.push_str(&format!(
            "    (local ${} {})\n",
            encode_debug_symbol(&binder),
            local_env.param_kind(&binder).wat_type()
        ));
    }
    for (index, op) in body.iter().enumerate() {
        match op {
            CoreOp::ReturnValue(value) => {
                render_core_value_i32(wat, value, &local_env)?;
                wat.push_str("  )\n");
                return Ok(());
            }
            CoreOp::TailCall { func, args } => {
                let (runtime_func, runtime_args) =
                    effective_tailcall(&body_core, func, args, &local_env);
                render_tailcall_args(wat, &body_core, runtime_func, &runtime_args, &local_env)?;
                wat.push_str(&format!("    call ${}\n", final_symbol(runtime_func)));
                if let Some(CoreOp::TailCallResult { binder }) = body.get(index + 1) {
                    wat.push_str(&format!("    local.set ${}\n", encode_debug_symbol(binder)));
                    continue;
                }
            }
            CoreOp::TailCallResult { .. }
            | CoreOp::DynamicCallableTarget { .. }
            | CoreOp::CallableAlias { .. }
            | CoreOp::OperatorTarget { .. } => {}
            _ => {
                return Err(BackendDiagnostic::UnsupportedI32ReturnValue {
                    value: format!("lifted {}", final_symbol(symbol)),
                });
            }
        }
    }
    wat.push_str("    i32.const 0\n");
    wat.push_str("  )\n");
    Ok(())
}

fn dyn_row_param_field_lane(param: &str, field: &str) -> String {
    format!("{param}__{field}")
}

fn callable_param_env_lane(param: &str, index: usize) -> String {
    format!("{param}__env{index}")
}

fn render_externref_func_header(
    wat: &mut String,
    symbol: &str,
    export: Option<&str>,
    env: &RenderEnv,
) {
    match export {
        Some(export) => wat.push_str(&format!(
            "  (func ${symbol} (export \"{export}\"){} (result externref)\n",
            env.signature()
        )),
        None => wat.push_str(&format!(
            "  (func ${symbol}{} (result externref)\n",
            env.signature()
        )),
    }
}

fn render_return_value_wat(
    wat: &mut String,
    symbol: &str,
    export: Option<&str>,
    value: &CoreValue,
    env: &RenderEnv,
) -> Result<(), BackendDiagnostic> {
    if render_callable_return_value_wat(wat, symbol, export, value, env)? {
        return Ok(());
    }
    if core_value_is_renderable_externref(value, env) {
        render_externref_func_header(wat, symbol, export, env);
        render_core_value_externref(wat, value, env)?;
        wat.push_str("  )\n");
    } else {
        render_func_header(wat, symbol, export, env);
        render_core_value_i32(wat, value, env)?;
        wat.push_str(")\n");
    }
    Ok(())
}

fn render_callable_return_value_wat(
    wat: &mut String,
    symbol: &str,
    export: Option<&str>,
    value: &CoreValue,
    env: &RenderEnv,
) -> Result<bool, BackendDiagnostic> {
    let CoreValue::LiftedFunction { .. } = value else {
        return Ok(false);
    };
    let env_params = callable_return_env_params(value, env);
    let selector = callable_selector_for_value(
        value,
        &[BackendValueKind::I32],
        &env_params,
        Some(BackendValueKind::I32),
        env,
    )
    .ok_or_else(|| BackendDiagnostic::UnsupportedI32ReturnValue {
        value: value.debug_name(),
    })?;
    let env_values = callable_env_values(value, &env_params, env).ok_or_else(|| {
        BackendDiagnostic::UnsupportedI32ReturnValue {
            value: value.debug_name(),
        }
    })?;
    let results = std::iter::repeat_n("i32", 1 + env_params.len())
        .collect::<Vec<_>>()
        .join(" ");
    match export {
        Some(export) => wat.push_str(&format!(
            "  (func ${symbol} (export \"{export}\"){} (result {results})\n",
            env.signature()
        )),
        None => wat.push_str(&format!(
            "  (func ${symbol}{} (result {results})\n",
            env.signature()
        )),
    }
    wat.push_str(&format!("    i32.const {selector}\n"));
    for value in env_values {
        render_core_value_i32(wat, &value, env)?;
    }
    wat.push_str("  )\n");
    Ok(true)
}

fn render_tailcall_func_header(
    wat: &mut String,
    core: &CoreProgram,
    func: &str,
    symbol: &str,
    export: Option<&str>,
    env: &RenderEnv,
) {
    match callable_signature_for_target(core, func, env).and_then(|(_, result)| result) {
        Some(WasmValueKind::ExternRef) => render_externref_func_header(wat, symbol, export, env),
        Some(WasmValueKind::I32) | None => render_func_header(wat, symbol, export, env),
    }
}

fn tailcall_result_kind(core: &CoreProgram, func: &str, env: &RenderEnv) -> Option<WasmValueKind> {
    callable_signature_for_target(core, func, env).and_then(|(_, result)| result)
}

fn tailcall_result_ref_cell_lane(
    core: &CoreProgram,
    func: &str,
    env: &RenderEnv,
) -> Option<CoreRefCellLane> {
    core.ops
        .iter()
        .find_map(|op| {
            let CoreOp::ExternFunctionTarget {
                target,
                result_ref_cell_lane,
                ..
            } = op
            else {
                return None;
            };
            (target == func).then_some(*result_ref_cell_lane).flatten()
        })
        .or_else(|| {
            env.function_abis
                .get(func)
                .and_then(|abi| abi.result_ref_cell_lane)
        })
}

fn render_core_value_i32(
    wat: &mut String,
    value: &CoreValue,
    env: &RenderEnv,
) -> Result<(), BackendDiagnostic> {
    let value = resolve_core_value_binding(value, env);
    match value {
        CoreValue::Unit => wat.push_str("    i32.const 0\n"),
        CoreValue::I64(value) => wat.push_str(&format!("    i32.const {}\n", *value as i32)),
        CoreValue::Bool(value) => wat.push_str(&format!("    i32.const {}\n", i32::from(*value))),
        CoreValue::Var(name) if env.is_static(name) => {
            wat.push_str(&format!("    global.get ${}\n", env.static_symbol(name)));
        }
        CoreValue::Var(name) if env.dyn_row_param_single_data_field_lane(name).is_some() => {
            let lane = env
                .dyn_row_param_single_data_field_lane(name)
                .expect("checked single data lane");
            wat.push_str(&format!("    local.get ${}\n", encode_debug_symbol(&lane)));
        }
        CoreValue::Var(name) if env.is_param(name) && !env.has_dyn_row_param_fields(name) => {
            wat.push_str(&format!("    local.get ${}\n", encode_debug_symbol(name)));
        }
        CoreValue::TupleField {
            tuple, field_index, ..
        } => {
            if let Some(value) = tuple_field_value(tuple, *field_index, env) {
                render_core_value_i32(wat, value, env)?;
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::RecordField { record, field } => {
            if let Some(value) = record_field_value(record, field, env) {
                render_core_value_i32(wat, value, env)?;
            } else if let CoreValue::Var(name) = resolve_core_value_binding(record, env) {
                if env.param_kind(name) == WasmValueKind::I32 {
                    wat.push_str(&format!("    local.get ${}\n", encode_debug_symbol(name)));
                } else {
                    return Err(unsupported_i32_render_diagnostic(value));
                }
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::DynRowPackage { .. } => {
            if let Some(value) = single_data_lane_dyn_row_package_i32_value(value, env) {
                render_core_value_i32(wat, &value, env)?;
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::DynRowField { package, field } => {
            if let Some(value) = dyn_row_field_value(package, field, env) {
                render_core_value_i32(wat, value, env)?;
            } else if let Some(param) = single_field_dyn_row_param(package, field, env) {
                wat.push_str(&format!("    local.get ${}\n", encode_debug_symbol(&param)));
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::RangeField { range, field } => {
            if let Some(value) = range_field_value(range, *field, env) {
                render_core_value_i32(wat, value, env)?;
            } else if core_value_is_renderable_externref(range, env) {
                render_core_value_externref(wat, range, env)?;
                wat.push_str(&format!(
                    "    call ${}\n",
                    range_field_import_symbol(*field)
                ));
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::AggregateField { kind, value, field } => {
            if let Some(value) = slice_field_value(value, *field, env) {
                render_core_value_i32(wat, &value, env)?;
            } else if core_value_is_renderable_externref(value, env) {
                render_core_value_externref(wat, value, env)?;
                wat.push_str(&format!(
                    "    call ${}\n",
                    aggregate_field_import_symbol(*kind, *field)
                ));
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::TextField { kind, value, field } => {
            if core_value_is_renderable_externref(value, env) {
                render_core_value_externref(wat, value, env)?;
                wat.push_str(&format!(
                    "    call ${}\n",
                    text_field_import_symbol(*kind, *field)
                ));
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::AggregateIndex { kind, value, index } => {
            if let Some(value) = slice_index_value(value, index, env) {
                render_core_value_i32(wat, value, env)?;
            } else if core_value_is_renderable_externref(value, env)
                && core_value_is_renderable_i32(index, env)
            {
                render_core_value_externref(wat, value, env)?;
                render_core_value_i32(wat, index, env)?;
                wat.push_str(&format!(
                    "    call ${}\n",
                    aggregate_index_import_symbol(*kind, RuntimeValueLane::I32)
                ));
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::AggregateSlice { .. } | CoreValue::TextSlice { .. } => {
            return Err(unsupported_i32_render_diagnostic(value));
        }
        CoreValue::TextIndex { kind, value, index } => {
            if core_value_is_renderable_externref(value, env)
                && core_value_is_renderable_i32(index, env)
            {
                render_core_value_externref(wat, value, env)?;
                render_core_value_i32(wat, index, env)?;
                wat.push_str(&format!("    call ${}\n", text_index_import_symbol(*kind)));
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::BuiltinRuntimeCall { call, args }
            if builtin_runtime_call_is_renderable_i32(*call, args, env) =>
        {
            match call {
                BuiltinMethodCall::TextLen { .. } | BuiltinMethodCall::TextRuneLen { .. } => {
                    render_core_value_externref_operand(wat, &args[0], env)?;
                }
                BuiltinMethodCall::TextCharAt { .. } => {
                    render_core_value_externref_operand(wat, &args[0], env)?;
                    render_core_value_i32(wat, &args[1], env)?;
                }
                BuiltinMethodCall::RefGet => {
                    render_core_value_externref(wat, &args[0], env)?;
                }
                BuiltinMethodCall::UnsafeRefGet => {
                    render_core_value_externref(wat, &args[0], env)?;
                }
                BuiltinMethodCall::VecNew
                | BuiltinMethodCall::VecPush
                | BuiltinMethodCall::VecFreeze
                | BuiltinMethodCall::StringNew
                | BuiltinMethodCall::StringFrom
                | BuiltinMethodCall::StringConcat
                | BuiltinMethodCall::StringAsStr
                | BuiltinMethodCall::StringToCStr
                | BuiltinMethodCall::StringPushRune
                | BuiltinMethodCall::RefNew
                | BuiltinMethodCall::RefSet
                | BuiltinMethodCall::UnsafeRefNew
                | BuiltinMethodCall::UnsafeRefSet => {
                    unreachable!("externref runtime calls are not renderable as i32")
                }
            }
            wat.push_str(&format!(
                "    call ${}\n",
                builtin_runtime_call_symbol(*call, args, env)
            ));
        }
        CoreValue::Adt { ctor, variants, .. } => {
            if let Some(tag) = variants.iter().position(|variant| variant == ctor) {
                wat.push_str(&format!("    i32.const {tag}\n"));
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::Var(_)
        | CoreValue::TextLiteral { .. }
        | CoreValue::Tuple { .. }
        | CoreValue::SliceLiteral { .. }
        | CoreValue::Range { .. }
        | CoreValue::BuiltinRuntimeCall { .. }
        | CoreValue::Record { .. }
        | CoreValue::RecordUpdate { .. }
        | CoreValue::LiftedFunction { .. }
        | CoreValue::Rendered { .. } => {
            return Err(unsupported_i32_render_diagnostic(value));
        }
    }
    Ok(())
}

fn render_core_value_externref(
    wat: &mut String,
    value: &CoreValue,
    env: &RenderEnv,
) -> Result<(), BackendDiagnostic> {
    let value = resolve_core_value_binding(value, env);
    match value {
        CoreValue::Var(name)
            if env.is_static(name) && env.param_kind(name) == WasmValueKind::ExternRef =>
        {
            wat.push_str(&format!("    global.get ${}\n", env.static_symbol(name)));
            Ok(())
        }
        CoreValue::Var(name) if env.param_kind(name) == WasmValueKind::ExternRef => {
            wat.push_str(&format!("    local.get ${}\n", encode_debug_symbol(name)));
            Ok(())
        }
        CoreValue::SliceLiteral { items } if aggregate_items_lane(items, env).is_some() => {
            let Some(lane) = aggregate_items_lane(items, env) else {
                return Err(unsupported_i32_render_diagnostic(value));
            };
            for item in items {
                let item_lane = render_runtime_lane_value(wat, item, env)?;
                if item_lane != lane {
                    return Err(unsupported_i32_render_diagnostic(value));
                }
            }
            wat.push_str(&format!(
                "    call ${}\n",
                slice_literal_import_symbol(items.len(), lane)
            ));
            Ok(())
        }
        CoreValue::AggregateIndex { kind, value, index } => {
            if let Some(item) = slice_index_value(value, index, env) {
                render_core_value_externref(wat, item, env)?;
                return Ok(());
            }
            if core_value_is_renderable_externref(value, env)
                && core_value_is_renderable_i32(index, env)
            {
                render_core_value_externref(wat, value, env)?;
                render_core_value_i32(wat, index, env)?;
                wat.push_str(&format!(
                    "    call ${}\n",
                    aggregate_index_import_symbol(*kind, RuntimeValueLane::ExternRef)
                ));
                Ok(())
            } else {
                Err(unsupported_i32_render_diagnostic(value))
            }
        }
        CoreValue::TextLiteral { kind, value } => {
            for byte in value.as_bytes() {
                wat.push_str(&format!("    i32.const {}\n", *byte as i32));
            }
            wat.push_str(&format!(
                "    call ${}\n",
                text_literal_import_symbol(*kind, value.as_bytes().len())
            ));
            Ok(())
        }
        CoreValue::Range { start, end }
            if core_value_is_renderable_i32(start, env)
                && core_value_is_renderable_i32(end, env) =>
        {
            render_core_value_i32(wat, start, env)?;
            render_core_value_i32(wat, end, env)?;
            wat.push_str(&format!("    call ${}\n", range_import_symbol()));
            Ok(())
        }
        CoreValue::AggregateSlice { kind, value, range } => {
            let Some((start, end)) = range_bounds(range, env) else {
                return Err(unsupported_i32_render_diagnostic(value));
            };
            if core_value_is_renderable_externref(value, env)
                && core_value_is_renderable_i32(start, env)
                && core_value_is_renderable_i32(end, env)
            {
                render_core_value_externref(wat, value, env)?;
                render_core_value_i32(wat, start, env)?;
                render_core_value_i32(wat, end, env)?;
                wat.push_str(&format!(
                    "    call ${}\n",
                    aggregate_slice_import_symbol(*kind)
                ));
                Ok(())
            } else {
                Err(unsupported_i32_render_diagnostic(value))
            }
        }
        CoreValue::TextSlice { kind, value, range } => {
            let Some((start, end)) = range_bounds(range, env) else {
                return Err(unsupported_i32_render_diagnostic(value));
            };
            if core_value_is_renderable_externref(value, env)
                && core_value_is_renderable_i32(start, env)
                && core_value_is_renderable_i32(end, env)
            {
                render_core_value_externref(wat, value, env)?;
                render_core_value_i32(wat, start, env)?;
                render_core_value_i32(wat, end, env)?;
                wat.push_str(&format!("    call ${}\n", text_slice_import_symbol(*kind)));
                Ok(())
            } else {
                Err(unsupported_i32_render_diagnostic(value))
            }
        }
        CoreValue::BuiltinRuntimeCall { call, args }
            if builtin_runtime_call_is_renderable_externref(*call, args, env) =>
        {
            match call {
                BuiltinMethodCall::VecNew => {}
                BuiltinMethodCall::VecPush => {
                    render_core_value_externref(wat, &args[0], env)?;
                    render_runtime_lane_value(wat, &args[1], env)?;
                }
                BuiltinMethodCall::VecFreeze => {
                    render_core_value_externref(wat, &args[0], env)?;
                }
                BuiltinMethodCall::StringNew => {}
                BuiltinMethodCall::StringFrom => {
                    render_core_value_externref(wat, &args[0], env)?;
                }
                BuiltinMethodCall::StringConcat => {
                    render_core_value_externref(wat, &args[0], env)?;
                    render_core_value_externref(wat, &args[1], env)?;
                }
                BuiltinMethodCall::StringAsStr => {
                    render_core_value_externref(wat, &args[0], env)?;
                }
                BuiltinMethodCall::StringToCStr => {
                    render_core_value_externref(wat, &args[0], env)?;
                }
                BuiltinMethodCall::StringPushRune => {
                    render_core_value_externref(wat, &args[0], env)?;
                    render_core_value_i32(wat, &args[1], env)?;
                }
                BuiltinMethodCall::TextLen { .. } | BuiltinMethodCall::TextRuneLen { .. } => {
                    unreachable!("text len returns i32 and is not renderable as externref")
                }
                BuiltinMethodCall::TextCharAt { .. } => {
                    unreachable!("text char_at returns rune/i32 and is not renderable as externref")
                }
                BuiltinMethodCall::RefNew => {
                    render_ref_lane_value(wat, &args[0], env)?;
                }
                BuiltinMethodCall::RefGet => {
                    render_core_value_externref(wat, &args[0], env)?;
                }
                BuiltinMethodCall::RefSet => {
                    render_core_value_externref(wat, &args[0], env)?;
                    render_ref_lane_value(wat, &args[1], env)?;
                }
                BuiltinMethodCall::UnsafeRefNew => {
                    render_ref_lane_value(wat, &args[0], env)?;
                }
                BuiltinMethodCall::UnsafeRefGet => {
                    render_core_value_externref(wat, &args[0], env)?;
                }
                BuiltinMethodCall::UnsafeRefSet => {
                    render_core_value_externref(wat, &args[0], env)?;
                    render_ref_lane_value(wat, &args[1], env)?;
                }
            }
            wat.push_str(&format!(
                "    call ${}\n",
                builtin_runtime_call_symbol(*call, args, env)
            ));
            Ok(())
        }
        _ => Err(unsupported_i32_render_diagnostic(value)),
    }
}

fn render_core_value_externref_operand(
    wat: &mut String,
    value: &CoreValue,
    env: &RenderEnv,
) -> Result<(), BackendDiagnostic> {
    let value = resolve_core_value_binding(value, env);
    match value {
        CoreValue::AggregateIndex { kind, value, index } => {
            if let Some(item) = slice_index_value(value, index, env) {
                return render_core_value_externref_operand(wat, item, env);
            }
            if core_value_is_renderable_externref(value, env)
                && core_value_is_renderable_i32(index, env)
            {
                render_core_value_externref(wat, value, env)?;
                render_core_value_i32(wat, index, env)?;
                wat.push_str(&format!(
                    "    call ${}\n",
                    aggregate_index_import_symbol(*kind, RuntimeValueLane::ExternRef)
                ));
                Ok(())
            } else {
                Err(unsupported_i32_render_diagnostic(value))
            }
        }
        _ => render_core_value_externref(wat, value, env),
    }
}

fn render_ref_lane_value(
    wat: &mut String,
    value: &CoreValue,
    env: &RenderEnv,
) -> Result<(), BackendDiagnostic> {
    match ref_value_lane(value, env) {
        Some(RefRuntimeLane::ExternRef) => render_core_value_externref(wat, value, env),
        Some(RefRuntimeLane::I32) => render_core_value_i32(wat, value, env),
        None => Err(unsupported_i32_render_diagnostic(value)),
    }
}

fn builtin_runtime_call_is_renderable_externref(
    call: BuiltinMethodCall,
    args: &[CoreValue],
    env: &RenderEnv,
) -> bool {
    match (call, args) {
        (BuiltinMethodCall::VecNew, []) => true,
        (BuiltinMethodCall::VecPush, [vec, item]) => {
            core_value_is_renderable_externref(vec, env) && runtime_value_lane(item, env).is_some()
        }
        (BuiltinMethodCall::VecFreeze, [vec]) => core_value_is_renderable_externref(vec, env),
        (BuiltinMethodCall::StringNew, []) => true,
        (BuiltinMethodCall::StringFrom, [text]) => core_value_is_renderable_externref(text, env),
        (BuiltinMethodCall::StringConcat, [left, right]) => {
            core_value_is_renderable_externref(left, env)
                && core_value_is_renderable_externref(right, env)
        }
        (BuiltinMethodCall::StringAsStr, [text]) => core_value_is_renderable_externref(text, env),
        (BuiltinMethodCall::StringToCStr, [text]) => core_value_is_renderable_externref(text, env),
        (BuiltinMethodCall::StringPushRune, [text, rune]) => {
            core_value_is_renderable_externref(text, env) && core_value_is_renderable_i32(rune, env)
        }
        (BuiltinMethodCall::RefNew, [value]) => ref_value_lane(value, env).is_some(),
        (BuiltinMethodCall::RefGet, [cell]) => {
            core_value_is_renderable_externref(cell, env)
                && matches!(ref_cell_lane(cell, env), Some(RefRuntimeLane::ExternRef))
        }
        (BuiltinMethodCall::RefSet, [cell, value]) => {
            core_value_is_renderable_externref(cell, env) && ref_value_lane(value, env).is_some()
        }
        (BuiltinMethodCall::UnsafeRefNew, [value]) => ref_value_lane(value, env).is_some(),
        (BuiltinMethodCall::UnsafeRefGet, [cell]) => {
            core_value_is_renderable_externref(cell, env)
                && matches!(ref_cell_lane(cell, env), Some(RefRuntimeLane::ExternRef))
        }
        (BuiltinMethodCall::UnsafeRefSet, [cell, value]) => {
            core_value_is_renderable_externref(cell, env) && ref_value_lane(value, env).is_some()
        }
        _ => false,
    }
}

fn builtin_runtime_call_is_renderable_i32(
    call: BuiltinMethodCall,
    args: &[CoreValue],
    env: &RenderEnv,
) -> bool {
    match (call, args) {
        (BuiltinMethodCall::TextLen { .. } | BuiltinMethodCall::TextRuneLen { .. }, [text]) => {
            core_value_is_renderable_externref_operand(text, env)
                || core_value_is_renderable_externref(text, env)
        }
        (BuiltinMethodCall::TextCharAt { .. }, [text, index]) => {
            (core_value_is_renderable_externref_operand(text, env)
                || core_value_is_renderable_externref(text, env))
                && core_value_is_renderable_i32(index, env)
        }
        (BuiltinMethodCall::RefGet, [cell]) => {
            core_value_is_renderable_externref(cell, env)
                && matches!(ref_cell_lane(cell, env), Some(RefRuntimeLane::I32))
        }
        (BuiltinMethodCall::UnsafeRefGet, [cell]) => {
            core_value_is_renderable_externref(cell, env)
                && matches!(ref_cell_lane(cell, env), Some(RefRuntimeLane::I32))
        }
        _ => false,
    }
}

fn builtin_runtime_call_symbol(
    call: BuiltinMethodCall,
    args: &[CoreValue],
    env: &RenderEnv,
) -> String {
    if let Some(lane) = ref_runtime_lane(call, args, env) {
        ref_runtime_import_symbol(call, lane)
    } else if let Some(lane) = vec_runtime_lane(call, args, env) {
        vec_runtime_import_symbol(call, lane)
    } else {
        builtin_runtime_import_symbol(call)
    }
}

fn unsupported_i32_render_diagnostic(value: &CoreValue) -> BackendDiagnostic {
    BackendDiagnostic::UnsupportedI32ReturnValue {
        value: value.debug_name(),
    }
}

fn core_value_is_renderable_i32(value: &CoreValue, env: &RenderEnv) -> bool {
    let value = resolve_core_value_binding(value, env);
    match value {
        CoreValue::Unit | CoreValue::I64(_) | CoreValue::Bool(_) => true,
        CoreValue::Adt {
            ctor,
            variants,
            args,
            ..
        } => {
            variants.iter().any(|variant| variant == ctor)
                && args
                    .iter()
                    .all(|arg| core_value_is_renderable_i32(arg, env))
        }
        CoreValue::Var(name) => env.param_kind(name) == WasmValueKind::I32,
        CoreValue::TupleField {
            tuple, field_index, ..
        } => tuple_field_value(tuple, *field_index, env)
            .map(|value| core_value_is_renderable_i32(value, env))
            .unwrap_or(false),
        CoreValue::RecordField { record, field } => record_field_value(record, field, env)
            .map(|value| core_value_is_renderable_i32(value, env))
            .unwrap_or_else(|| {
                matches!(resolve_core_value_binding(record, env), CoreValue::Var(name) if env.param_kind(name) == WasmValueKind::I32)
                    && !field.is_empty()
            }),
        CoreValue::DynRowField { package, field } => dyn_row_field_value(package, field, env)
            .map(|value| core_value_is_renderable_i32(value, env))
            .unwrap_or_else(|| single_field_dyn_row_param(package, field, env).is_some()),
        CoreValue::RangeField { range, field } => range_field_value(range, *field, env)
            .map(|value| core_value_is_renderable_i32(value, env))
            .unwrap_or_else(|| core_value_is_renderable_externref(range, env)),
        CoreValue::AggregateField { value, field, .. } => slice_field_value(value, *field, env)
            .map(|value| core_value_is_renderable_i32(&value, env))
            .unwrap_or_else(|| core_value_is_renderable_externref(value, env)),
        CoreValue::TextField { value, .. } => core_value_is_renderable_externref(value, env),
        CoreValue::AggregateIndex { value, index, .. } => slice_index_value(value, index, env)
            .map(|value| core_value_is_renderable_i32(value, env))
            .unwrap_or_else(|| {
                core_value_is_renderable_externref(value, env)
                    && core_value_is_renderable_i32(index, env)
            }),
        CoreValue::TextIndex { value, index, .. } => {
            core_value_is_renderable_externref(value, env)
                && core_value_is_renderable_i32(index, env)
        }
        CoreValue::AggregateSlice { value, range, .. }
        | CoreValue::TextSlice { value, range, .. } => range_bounds(range, env)
            .map(|(start, end)| {
                core_value_is_renderable_externref(value, env)
                    && core_value_is_renderable_i32(start, env)
                    && core_value_is_renderable_i32(end, env)
            })
            .unwrap_or(false),
        CoreValue::BuiltinRuntimeCall { call, args } => {
            builtin_runtime_call_is_renderable_i32(*call, args, env)
        }
        CoreValue::DynRowPackage { .. } => {
            single_data_lane_dyn_row_package_i32_value(value, env)
                .map(|value| core_value_is_renderable_i32(&value, env))
                .unwrap_or(false)
        }
        CoreValue::Tuple { .. }
        | CoreValue::TextLiteral { .. }
        | CoreValue::SliceLiteral { .. }
        | CoreValue::Range { .. }
        | CoreValue::Record { .. }
        | CoreValue::RecordUpdate { .. }
        | CoreValue::LiftedFunction { .. }
        | CoreValue::Rendered { .. } => false,
    }
}

fn core_value_is_renderable_externref(value: &CoreValue, env: &RenderEnv) -> bool {
    let value = resolve_core_value_binding(value, env);
    match value {
        CoreValue::Var(name) => env.param_kind(name) == WasmValueKind::ExternRef,
        CoreValue::TextLiteral { .. } => true,
        CoreValue::SliceLiteral { items } => aggregate_items_lane(items, env).is_some(),
        CoreValue::AggregateIndex { value, index, .. } => slice_index_value(value, index, env)
            .map(|value| core_value_is_renderable_externref(value, env))
            .unwrap_or_else(|| {
                core_value_is_renderable_externref(value, env)
                    && core_value_is_renderable_i32(index, env)
                    && aggregate_index_lane(value, index, env) == Some(RuntimeValueLane::ExternRef)
            }),
        CoreValue::Range { start, end } => {
            core_value_is_renderable_i32(start, env) && core_value_is_renderable_i32(end, env)
        }
        CoreValue::AggregateSlice { value, range, .. }
        | CoreValue::TextSlice { value, range, .. } => range_bounds(range, env)
            .map(|(start, end)| {
                core_value_is_renderable_externref(value, env)
                    && core_value_is_renderable_i32(start, env)
                    && core_value_is_renderable_i32(end, env)
            })
            .unwrap_or(false),
        CoreValue::BuiltinRuntimeCall { call, args } => {
            builtin_runtime_call_is_renderable_externref(*call, args, env)
        }
        _ => false,
    }
}

fn core_value_is_renderable_externref_operand(value: &CoreValue, env: &RenderEnv) -> bool {
    let value = resolve_core_value_binding(value, env);
    match value {
        CoreValue::AggregateIndex { value, index, .. } => slice_index_value(value, index, env)
            .map(|value| core_value_is_renderable_externref_operand(value, env))
            .unwrap_or_else(|| {
                core_value_is_renderable_externref(value, env)
                    && core_value_is_renderable_i32(index, env)
            }),
        _ => core_value_is_renderable_externref(value, env),
    }
}

fn resolve_core_value_binding<'a>(value: &'a CoreValue, env: &'a RenderEnv) -> &'a CoreValue {
    match value {
        CoreValue::Var(name) => env.binding(name).unwrap_or(value),
        _ => value,
    }
}

fn tuple_field_value<'a>(
    tuple: &'a CoreValue,
    field_index: usize,
    env: &'a RenderEnv,
) -> Option<&'a CoreValue> {
    let tuple = resolve_core_value_binding(tuple, env);
    let CoreValue::Tuple { fields } = tuple else {
        return None;
    };
    fields.get(field_index)
}

fn record_field_value<'a>(
    record: &'a CoreValue,
    field: &str,
    env: &'a RenderEnv,
) -> Option<&'a CoreValue> {
    let record = resolve_core_value_binding(record, env);
    match record {
        CoreValue::Record { fields } => fields
            .iter()
            .find(|candidate| candidate.name == field)
            .map(|candidate| &candidate.value),
        CoreValue::RecordUpdate { base, fields } => fields
            .iter()
            .find(|candidate| candidate.name == field)
            .map(|candidate| &candidate.value)
            .or_else(|| record_field_value(base, field, env)),
        _ => None,
    }
}

fn dyn_row_field_value<'a>(
    package: &'a CoreValue,
    field: &str,
    env: &'a RenderEnv,
) -> Option<&'a CoreValue> {
    let package = resolve_core_value_binding(package, env);
    let CoreValue::DynRowPackage { payload, fields } = package else {
        return None;
    };
    let adapter = fields.iter().find(|candidate| candidate.name == field)?;
    match &adapter.source {
        crate::core::CoreDynRowFieldSource::Field => record_field_value(payload, field, env),
        crate::core::CoreDynRowFieldSource::ContractObligation { .. }
        | crate::core::CoreDynRowFieldSource::ReceiverMethod { .. } => None,
    }
}

fn dyn_row_data_field_value(
    package: &CoreValue,
    field: &str,
    env: &RenderEnv,
) -> Option<CoreValue> {
    let package = resolve_core_value_binding(package, env);
    match package {
        CoreValue::Record { .. } | CoreValue::RecordUpdate { .. } => {
            record_field_value(package, field, env).cloned()
        }
        CoreValue::DynRowPackage { payload, fields } => {
            let adapter = fields.iter().find(|candidate| candidate.name == field)?;
            match adapter.source {
                crate::core::CoreDynRowFieldSource::Field => {
                    record_field_value(payload, field, env).cloned()
                }
                crate::core::CoreDynRowFieldSource::ContractObligation { .. } => {
                    let CoreValue::Var(param) = resolve_core_value_binding(payload, env) else {
                        return None;
                    };
                    env.dyn_row_param_field_lane(param, field)
                        .map(CoreValue::Var)
                }
                crate::core::CoreDynRowFieldSource::ReceiverMethod { .. } => None,
            }
        }
        _ => None,
    }
}

fn backend_dyn_row_contract_storage_kind(ty: &Type) -> Option<BackendValueKind> {
    match ty {
        Type::I64 | Type::Rune | Type::Bool => Some(BackendValueKind::I32),
        _ => None,
    }
}

fn single_field_dyn_row_param(package: &CoreValue, field: &str, env: &RenderEnv) -> Option<String> {
    let package = resolve_core_value_binding(package, env);
    let CoreValue::Var(name) = package else {
        return None;
    };
    if let Some(lane) = env.dyn_row_param_field_lane(name, field) {
        return Some(lane);
    }
    if field.is_empty() || env.param_kind(name) != WasmValueKind::I32 {
        return None;
    }
    Some(name.clone())
}

fn single_data_lane_dyn_row_package_i32_value(
    value: &CoreValue,
    env: &RenderEnv,
) -> Option<CoreValue> {
    let CoreValue::DynRowPackage { payload, fields } = resolve_core_value_binding(value, env)
    else {
        return None;
    };
    let mut data_fields = fields.iter().filter(|field| match &field.source {
        crate::core::CoreDynRowFieldSource::Field => true,
        crate::core::CoreDynRowFieldSource::ContractObligation { ty } => {
            backend_dyn_row_contract_storage_kind(ty).is_some()
        }
        crate::core::CoreDynRowFieldSource::ReceiverMethod { .. } => false,
    });
    let field = data_fields.next()?;
    if data_fields.next().is_some() {
        return None;
    }
    match field.source {
        crate::core::CoreDynRowFieldSource::Field => {
            record_field_value(payload, &field.name, env).cloned()
        }
        crate::core::CoreDynRowFieldSource::ContractObligation { .. } => {
            let CoreValue::Var(param) = resolve_core_value_binding(payload, env) else {
                return None;
            };
            env.dyn_row_param_field_lane(param, &field.name)
                .map(CoreValue::Var)
        }
        crate::core::CoreDynRowFieldSource::ReceiverMethod { .. } => None,
    }
}

fn range_field_value<'a>(
    range: &'a CoreValue,
    field: RangeField,
    env: &'a RenderEnv,
) -> Option<&'a CoreValue> {
    let range = resolve_core_value_binding(range, env);
    let CoreValue::Range { start, end } = range else {
        return None;
    };
    match field {
        RangeField::Start => Some(start),
        RangeField::End => Some(end),
    }
}

fn range_bounds<'a>(
    range: &'a CoreValue,
    env: &'a RenderEnv,
) -> Option<(&'a CoreValue, &'a CoreValue)> {
    let range = resolve_core_value_binding(range, env);
    let CoreValue::Range { start, end } = range else {
        return None;
    };
    Some((start, end))
}

fn slice_field_value(slice: &CoreValue, field: SliceField, env: &RenderEnv) -> Option<CoreValue> {
    let slice = resolve_core_value_binding(slice, env);
    let CoreValue::SliceLiteral { items } = slice else {
        return None;
    };
    match field {
        SliceField::Len => Some(CoreValue::I64(items.len() as i64)),
    }
}

fn slice_index_value<'a>(
    slice: &'a CoreValue,
    index: &'a CoreValue,
    env: &'a RenderEnv,
) -> Option<&'a CoreValue> {
    let slice = resolve_core_value_binding(slice, env);
    let CoreValue::SliceLiteral { items } = slice else {
        return None;
    };
    let index = resolve_core_value_binding(index, env);
    let CoreValue::I64(index) = index else {
        return None;
    };
    usize::try_from(*index)
        .ok()
        .and_then(|index| items.get(index))
}

fn render_core_value_i32_indented(
    wat: &mut String,
    value: &CoreValue,
    env: &RenderEnv,
    indent: usize,
) -> Result<(), BackendDiagnostic> {
    let mut nested = String::new();
    render_core_value_i32(&mut nested, value, env)?;
    for line in nested.lines() {
        wat.push_str(&" ".repeat(indent));
        wat.push_str(line.trim_start());
        wat.push('\n');
    }
    Ok(())
}

fn render_match_arms_i32(
    wat: &mut String,
    scrutinee: &CoreValue,
    arms: &[CoreMatchArm],
    env: &RenderEnv,
    indent: usize,
) -> Result<(), BackendDiagnostic> {
    let Some((first, rest)) = arms.split_first() else {
        wat.push_str(&" ".repeat(indent));
        wat.push_str("unreachable\n");
        return Ok(());
    };
    render_match_arm_i32(wat, scrutinee, first, rest, env, indent)
}

fn render_match_arm_i32(
    wat: &mut String,
    scrutinee: &CoreValue,
    arm: &CoreMatchArm,
    rest: &[CoreMatchArm],
    env: &RenderEnv,
    indent: usize,
) -> Result<(), BackendDiagnostic> {
    match &arm.pattern {
        CorePattern::Wildcard => render_core_value_i32_indented(wat, &arm.value, env, indent)?,
        CorePattern::Bind(name) => {
            let arm_env = env.with_binding(name, scrutinee.clone());
            render_core_value_i32_indented(wat, &arm.value, &arm_env, indent)?;
        }
        CorePattern::I64(value) => {
            render_core_value_i32_indented(wat, scrutinee, env, indent)?;
            push_indent(wat, indent);
            wat.push_str(&format!("i32.const {}\n", *value as i32));
            push_indent(wat, indent);
            wat.push_str("i32.eq\n");
            push_indent(wat, indent);
            wat.push_str("if (result i32)\n");
            render_core_value_i32_indented(wat, &arm.value, env, indent + 2)?;
            push_indent(wat, indent);
            wat.push_str("else\n");
            render_match_arms_i32(wat, scrutinee, rest, env, indent + 2)?;
            push_indent(wat, indent);
            wat.push_str("end\n");
        }
        CorePattern::Bool(value) => {
            render_core_value_i32_indented(wat, scrutinee, env, indent)?;
            push_indent(wat, indent);
            wat.push_str(&format!("i32.const {}\n", i32::from(*value)));
            push_indent(wat, indent);
            wat.push_str("i32.eq\n");
            push_indent(wat, indent);
            wat.push_str("if (result i32)\n");
            render_core_value_i32_indented(wat, &arm.value, env, indent + 2)?;
            push_indent(wat, indent);
            wat.push_str("else\n");
            render_match_arms_i32(wat, scrutinee, rest, env, indent + 2)?;
            push_indent(wat, indent);
            wat.push_str("end\n");
        }
        CorePattern::Constructor { data, ctor, args } => {
            if let Some((tag, arm_env)) =
                constructor_match_env(scrutinee, data.as_deref(), ctor, args, env)
            {
                render_core_value_i32_indented(wat, scrutinee, env, indent)?;
                push_indent(wat, indent);
                wat.push_str(&format!("i32.const {tag}\n"));
                push_indent(wat, indent);
                wat.push_str("i32.eq\n");
                push_indent(wat, indent);
                wat.push_str("if (result i32)\n");
                render_core_value_i32_indented(wat, &arm.value, &arm_env, indent + 2)?;
                push_indent(wat, indent);
                wat.push_str("else\n");
                render_match_arms_i32(wat, scrutinee, rest, env, indent + 2)?;
                push_indent(wat, indent);
                wat.push_str("end\n");
            } else {
                render_match_arms_i32(wat, scrutinee, rest, env, indent)?;
            }
        }
    }
    Ok(())
}

fn constructor_match_env(
    scrutinee: &CoreValue,
    pattern_data: Option<&str>,
    ctor: &str,
    args: &[CorePattern],
    env: &RenderEnv,
) -> Option<(usize, RenderEnv)> {
    let scrutinee = resolve_core_value_binding(scrutinee, env);
    let tag = constructor_tag(scrutinee, pattern_data, ctor)?;
    let CoreValue::Adt { args: payloads, .. } = scrutinee else {
        return None;
    };
    if args.len() != payloads.len() {
        return None;
    }
    let mut next = env.clone();
    for (pattern, payload) in args.iter().zip(payloads) {
        bind_core_pattern(pattern, payload, &mut next)?;
    }
    Some((tag, next))
}

fn bind_core_pattern(pattern: &CorePattern, value: &CoreValue, env: &mut RenderEnv) -> Option<()> {
    match pattern {
        CorePattern::Wildcard => Some(()),
        CorePattern::Bind(name) => {
            env.bindings.insert(name.clone(), value.clone());
            Some(())
        }
        CorePattern::I64(expected) if value == &CoreValue::I64(*expected) => Some(()),
        CorePattern::Bool(expected) if value == &CoreValue::Bool(*expected) => Some(()),
        CorePattern::Constructor { data, ctor, args } => {
            constructor_match_env(value, data.as_deref(), ctor, args, env).map(|(_, next)| {
                *env = next;
            })
        }
        _ => None,
    }
}

fn constructor_tag(scrutinee: &CoreValue, pattern_data: Option<&str>, ctor: &str) -> Option<usize> {
    let CoreValue::Adt { data, variants, .. } = scrutinee else {
        return None;
    };
    if pattern_data.is_some_and(|pattern_data| pattern_data != data) {
        return None;
    }
    variants.iter().position(|variant| variant == ctor)
}

fn push_indent(wat: &mut String, indent: usize) {
    wat.push_str(&" ".repeat(indent));
}

fn escape_wat_comment(text: &str) -> String {
    text.replace('\n', "\\n").replace('\r', "\\r")
}

fn final_symbol(symbol: &str) -> String {
    encode_debug_symbol(symbol)
}

pub fn link_backend_artifacts(mut artifacts: Vec<BackendArtifact>) -> BackendLinkedBundle {
    let target = artifacts
        .first()
        .map(|artifact| artifact.target)
        .unwrap_or_default();
    let mut diagnostics = Vec::new();
    let mut seen_symbols = BTreeSet::new();
    let mut entries = Vec::new();
    let mut imports = Vec::new();

    for (artifact_index, artifact) in artifacts.iter().enumerate() {
        if artifact.target != target {
            diagnostics.push(BackendLinkDiagnostic::TargetMismatch {
                artifact_index,
                expected: target,
                actual: artifact.target,
            });
        }
        if !artifact.diagnostics.is_empty() {
            diagnostics.push(BackendLinkDiagnostic::ArtifactEmitFailed { artifact_index });
        }
        for entry in &artifact.manifest.entries {
            if !seen_symbols.insert(entry.final_symbol.clone()) {
                diagnostics.push(BackendLinkDiagnostic::DuplicateFinalSymbol {
                    symbol: entry.final_symbol.clone(),
                });
            }
            entries.push(entry.clone());
        }
        imports.extend(artifact.manifest.imports.iter().cloned());
    }

    entries.sort_by(|left, right| left.final_symbol.cmp(&right.final_symbol));
    sort_dedup_imports(&mut imports);

    if !diagnostics.is_empty() {
        return BackendLinkedBundle {
            target,
            linked_wat: String::new(),
            manifest: BackendManifest { entries, imports },
            diagnostics,
        };
    }

    artifacts.sort_by(|left, right| {
        let left_key = left
            .manifest
            .entries
            .first()
            .map(|entry| entry.final_symbol.as_str())
            .unwrap_or("");
        let right_key = right
            .manifest
            .entries
            .first()
            .map(|entry| entry.final_symbol.as_str())
            .unwrap_or("");
        left_key.cmp(right_key)
    });

    let mut linked_wat = String::from("(module\n");
    for import in &imports {
        render_extern_import_wat(&mut linked_wat, import);
        linked_wat.push_str(&format!(
            "  ;; extern-import {} symbol={} module={} name={} signature={}\n",
            canonical_abi(import.abi),
            escape_wat_comment(&import.final_symbol),
            escape_wat_comment(&import.module),
            escape_wat_comment(&import.name),
            escape_wat_comment(&import.signature_hash)
        ));
    }
    for (artifact_index, artifact) in artifacts.iter().enumerate() {
        linked_wat.push_str(&format!("  ;; linked artifact {artifact_index}\n"));
        let lines = artifact.wat.lines().collect::<Vec<_>>();
        for (line_index, line) in lines.iter().enumerate() {
            if line_index == 0 && *line == "(module" {
                continue;
            }
            if line_index + 1 == lines.len() && *line == ")" {
                continue;
            }
            if line.trim_start().starts_with("(import ") {
                continue;
            }
            linked_wat.push_str(line);
            linked_wat.push('\n');
        }
    }
    linked_wat.push_str(")\n");

    BackendLinkedBundle {
        target,
        linked_wat,
        manifest: BackendManifest { entries, imports },
        diagnostics,
    }
}

pub fn backend_cache_key(
    bundle: &BackendLinkedBundle,
    config: &BackendCacheConfig,
) -> BackendCacheKey {
    let mut encoded = String::new();
    encoded.push_str("compiler=");
    encoded.push_str(&config.compiler_version);
    encoded.push('\n');
    encoded.push_str("target=");
    encoded.push_str(backend_target_name(bundle.target));
    encoded.push('\n');
    encoded.push_str("features=");
    encoded.push_str(if config.features.tailcall {
        "tailcall"
    } else {
        "no-tailcall"
    });
    encoded.push(',');
    encoded.push_str(if config.features.wasi {
        "wasi"
    } else {
        "no-wasi"
    });
    encoded.push(',');
    encoded.push_str(if config.features.thread {
        "thread"
    } else {
        "no-thread"
    });
    encoded.push('\n');
    encoded.push_str("ownership-runtime=");
    encoded.push_str(ownership_runtime_name(config.ownership_runtime));
    encoded.push('\n');

    let mut imports = config.imports.clone();
    imports.extend(bundle.manifest.imports.iter().cloned());
    sort_dedup_imports(&mut imports);
    for import in &imports {
        encoded.push_str("import=");
        encoded.push_str(&canonical_import(import));
        encoded.push('\n');
    }

    let mut entries = bundle.manifest.entries.clone();
    entries.sort_by(|left, right| left.final_symbol.cmp(&right.final_symbol));
    for entry in &entries {
        encoded.push_str("symbol=");
        encoded.push_str(&entry.final_symbol);
        encoded.push('|');
        encoded.push_str(&entry.source_debug_name);
        encoded.push('|');
        encoded.push_str(&entry.pass_origin);
        encoded.push('|');
        encoded.push_str(
            entry
                .ownership
                .map(ownership_decision_name)
                .unwrap_or("none"),
        );
        encoded.push('\n');
    }
    encoded.push_str("wat=");
    encoded.push_str(&bundle.linked_wat);

    BackendCacheKey {
        target: bundle.target,
        digest: stable_digest(&encoded),
    }
}

pub fn sort_dedup_imports(imports: &mut Vec<BackendExternImport>) {
    imports.sort_by(|left, right| canonical_import(left).cmp(&canonical_import(right)));
    imports.dedup_by(|left, right| canonical_import(left) == canonical_import(right));
}

fn backend_target_name(target: BackendTarget) -> &'static str {
    match target {
        BackendTarget::WasmGc => "wasm-gc",
    }
}

fn ownership_runtime_name(runtime: BackendOwnershipRuntime) -> &'static str {
    match runtime {
        BackendOwnershipRuntime::WasmGc => "wasm-gc",
        BackendOwnershipRuntime::RcArcHelpers => "rc-arc-helpers",
    }
}

fn ownership_decision_name(decision: OwnershipDecision) -> &'static str {
    match decision {
        OwnershipDecision::StackValue => "stack-value",
        OwnershipDecision::InplaceReuse => "inplace-reuse",
        OwnershipDecision::Rc => "rc",
        OwnershipDecision::Arc => "arc",
        OwnershipDecision::StaticData => "static-data",
        OwnershipDecision::BorrowedView => "borrowed-view",
        OwnershipDecision::DynPackage => "dyn-package",
    }
}

fn canonical_import(import: &BackendExternImport) -> String {
    format!(
        "{}::{}::{}::{}::{}",
        canonical_abi(import.abi),
        import.final_symbol,
        import.module,
        import.name,
        import.signature_hash
    )
}

fn backend_extern_abi_from_core(abi: CoreExternAbi) -> BackendExternAbi {
    match abi {
        CoreExternAbi::Wasi => BackendExternAbi::Wasi,
        CoreExternAbi::C => BackendExternAbi::C,
    }
}

fn backend_extern_module_from_core(abi: CoreExternAbi) -> &'static str {
    match abi {
        CoreExternAbi::Wasi => "wasi_snapshot_preview1",
        CoreExternAbi::C => "env",
    }
}

fn canonical_abi(abi: BackendExternAbi) -> &'static str {
    match abi {
        BackendExternAbi::Wasi => "wasi",
        BackendExternAbi::C => "c",
    }
}

fn stable_digest(encoded: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in encoded.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
