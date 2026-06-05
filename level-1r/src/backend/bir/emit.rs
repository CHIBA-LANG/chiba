use std::collections::{BTreeMap, BTreeSet};

use crate::control::ContinuationKind;
use crate::core::{
    CoreCapturedContinuation, CoreExternAbi, CoreMatchArm, CoreOp, CorePattern, CoreProgram,
    CoreValidation, CoreValue, OperatorIntrinsic, OwnershipDecision, RangeField,
};
use crate::symbol::encode_debug_symbol;

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

    let manifest = manifest_for_core(core);
    if let Some(diagnostic) = unsupported_extern_import_signature(&manifest) {
        return BackendArtifact {
            target: BackendTarget::WasmGc,
            wat: String::new(),
            manifest,
            diagnostics: vec![diagnostic],
            return_value: first_return_value(core),
        };
    }

    if let Some(diagnostic) = unsupported_i32_return_value(core, params) {
        return BackendArtifact {
            target: BackendTarget::WasmGc,
            wat: String::new(),
            manifest,
            diagnostics: vec![diagnostic],
            return_value: first_return_value(core),
        };
    }
    let wat = match render_wat(core, &manifest, params) {
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

fn unsupported_i32_return_value(
    core: &CoreProgram,
    params: &[String],
) -> Option<BackendDiagnostic> {
    let mut env = RenderEnv::new(params);
    let mut continuations = BTreeSet::new();
    for (index, op) in core.ops.iter().enumerate() {
        if let Some(diagnostic) = match op {
            CoreOp::ReturnValue(value)
                if return_value_is_tailcall_result(core, index, value, &env) =>
            {
                None
            }
            CoreOp::ReturnValue(value) => unsupported_i32_value(value, &env),
            CoreOp::ReturnBranch {
                cond,
                then_value,
                else_value,
            } => [cond, then_value, else_value]
                .into_iter()
                .find_map(|value| unsupported_i32_value(value, &env)),
            CoreOp::ReturnMatch { scrutinee, arms } => unsupported_i32_match(scrutinee, arms, &env),
            CoreOp::TailCall { func, args } if continuations.contains(func) => {
                let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
                    return Some(BackendDiagnostic::UnsupportedContinuationRuntime {
                        op: "resume-without-result".to_string(),
                        kind: continuation_kind_for_binder(core, func),
                        binder: Some(func.clone()),
                    });
                };
                env = env.with_locals(vec![binder.clone()]);
                None
            }
            CoreOp::TailCall { args, .. } => {
                args.iter().find_map(|arg| unsupported_i32_value(arg, &env))
            }
            CoreOp::TailCallResult { binder } => {
                env = env.with_locals(vec![binder.clone()]);
                None
            }
            CoreOp::CaptureContinuation { binder, .. } => {
                continuations.insert(binder.clone());
                None
            }
            _ => None,
        } {
            return Some(diagnostic);
        }
    }
    None
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
        Some(CoreOp::TailCall { args, .. })
            if args.iter().all(|arg| core_value_is_renderable_i32(arg, env))
    ) {
        return true;
    }
    core.ops.windows(2).any(|window| {
        matches!(
            window,
            [
                CoreOp::TailCall { args, .. },
                CoreOp::TailCallResult { binder },
            ] if binder == name && args.iter().all(|arg| core_value_is_renderable_i32(arg, env))
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

fn manifest_for_core(core: &CoreProgram) -> BackendManifest {
    let mut entries = Vec::new();
    for op in &core.ops {
        collect_manifest_entries(op, core, &mut entries);
    }
    let mut imports = Vec::new();
    for op in &core.ops {
        collect_manifest_imports(op, &mut imports);
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
        | CoreOp::ReturnBranch { .. }
        | CoreOp::ReturnMatch { .. }
        | CoreOp::DynamicCallableTarget { .. }
        | CoreOp::ExternFunctionTarget { .. }
        | CoreOp::TupleConstruct { .. }
        | CoreOp::TupleFieldGet { .. }
        | CoreOp::RecordConstruct { .. }
        | CoreOp::RecordUpdate { .. }
        | CoreOp::RecordFieldGet { .. }
        | CoreOp::RangeFieldGet { .. }
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

fn collect_manifest_imports(op: &CoreOp, imports: &mut Vec<BackendExternImport>) {
    match op {
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
        CoreOp::CaptureContinuation { captured, .. } => {
            for op in &captured.ops {
                collect_manifest_imports(op, imports);
            }
        }
        _ => {}
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
) -> Result<String, BackendDiagnostic> {
    if core_contains_continuation_runtime(core) {
        return render_continuation_wat(core, manifest, params);
    }

    let env = RenderEnv::new(params);
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
        for op in &core.ops {
            if let CoreOp::OperatorTarget {
                protocol,
                target,
                intrinsic,
            } = op
            {
                render_operator_intrinsic_wat(&mut wat, protocol, target, *intrinsic);
            }
        }
        wat.push_str(")\n");
        return Ok(wat);
    }
    for (index, op) in core.ops.iter().enumerate() {
        match op {
            CoreOp::ReturnValue(value)
                if return_value_is_tailcall_result(core, index, value, &env) => {}
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
                render_func_header(&mut wat, &symbol, Some(&symbol), &env);
                render_core_value_i32(&mut wat, value, &env)?;
                wat.push_str(")\n");
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
                wat.push_str(&format!(
                    "  ;; tailcall {} args=[{}]\n",
                    final_symbol(func),
                    args.iter()
                        .map(CoreValue::debug_name)
                        .map(|arg| escape_wat_comment(&arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                if args
                    .iter()
                    .all(|arg| core_value_is_renderable_i32(arg, &env))
                {
                    let symbol = format!("chiba_tailcall_{tailcall_index}");
                    render_func_header(&mut wat, &symbol, None, &env);
                    for arg in args {
                        render_core_value_i32(&mut wat, arg, &env)?;
                    }
                    wat.push_str(&format!("    call ${}\n", final_symbol(func)));
                    wat.push_str("  )\n");
                    tailcall_index += 1;
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
            | CoreOp::ExternFunctionTarget { .. }
            | CoreOp::TailCallResult { .. }
            | CoreOp::TargetSpecificTerm { .. }
            | CoreOp::LiftedFunction { .. } => {}
            CoreOp::OperatorTarget {
                protocol,
                target,
                intrinsic,
            } => {
                render_operator_intrinsic_wat(&mut wat, protocol, target, *intrinsic);
            }
        }
    }
    wat.push_str(")\n");
    Ok(wat)
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
) -> Result<String, BackendDiagnostic> {
    let result_binders = collect_tailcall_result_binders(&core.ops);
    let env = RenderEnv::new(params).with_locals(result_binders.clone());
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
    for binder in &result_binders {
        wat.push_str(&format!(
            "    (local ${} i32)\n",
            encode_debug_symbol(binder)
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
            CoreOp::TailCall { func, args } if continuations.contains_key(func) => {
                let (kind, captured) = continuations
                    .get(func)
                    .expect("checked continuation binder")
                    .clone();
                if kind == ContinuationKind::Cont1 && !cont1_consumed.insert(func.clone()) {
                    return Err(BackendDiagnostic::Cont1ResumedMoreThanOnce {
                        binder: func.clone(),
                    });
                }
                let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
                    return Err(BackendDiagnostic::UnsupportedContinuationRuntime {
                        op: "resume-without-result".to_string(),
                        kind,
                        binder: Some(func.clone()),
                    });
                };
                let [arg] = args.as_slice() else {
                    return Err(BackendDiagnostic::UnsupportedContinuationRuntime {
                        op: "resume-arity".to_string(),
                        kind,
                        binder: Some(func.clone()),
                    });
                };
                wat.push_str(&format!(
                    "    ;; resume-cont binder={} kind={} result={}\n",
                    escape_wat_comment(func),
                    render_continuation_kind(kind),
                    escape_wat_comment(binder)
                ));
                render_captured_continuation_i32(&mut wat, &captured, arg, &env, core)?;
                wat.push_str(&format!("    local.set ${}\n", encode_debug_symbol(binder)));
            }
            CoreOp::TailCall { func, args } => {
                let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
                    continue;
                };
                wat.push_str(&format!(
                    "    ;; tailcall {} args=[{}]\n",
                    final_symbol(func),
                    args.iter()
                        .map(CoreValue::debug_name)
                        .map(|arg| escape_wat_comment(&arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                for arg in args {
                    render_core_value_i32(&mut wat, arg, &env)?;
                }
                wat.push_str(&format!("    call ${}\n", final_symbol(func)));
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

fn collect_operator_targets(ops: &[CoreOp]) -> Vec<&CoreOp> {
    let mut targets = Vec::new();
    for op in ops {
        match op {
            CoreOp::OperatorTarget { .. } => targets.push(op),
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

fn render_captured_continuation_i32(
    wat: &mut String,
    captured: &CoreCapturedContinuation,
    arg: &CoreValue,
    env: &RenderEnv,
    core: &CoreProgram,
) -> Result<(), BackendDiagnostic> {
    let captured_locals = captured
        .ops
        .iter()
        .filter_map(|op| match op {
            CoreOp::TailCallResult { binder } => Some(binder.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut env = env
        .with_locals(
            std::iter::once(captured.param.clone())
                .chain(captured_locals.iter().cloned())
                .collect(),
        )
        .with_binding(&captured.param, arg.clone());

    for (index, op) in captured.ops.iter().enumerate() {
        match op {
            CoreOp::TailCall { func, args } => {
                let Some(CoreOp::TailCallResult { binder }) = captured.ops.get(index + 1) else {
                    return Err(BackendDiagnostic::UnsupportedContinuationRuntime {
                        op: "captured-tailcall-without-result".to_string(),
                        kind: continuation_kind_for_captured(core, captured),
                        binder: None,
                    });
                };
                wat.push_str(&format!(
                    "    ;; captured-tailcall {} args=[{}]\n",
                    final_symbol(func),
                    args.iter()
                        .map(CoreValue::debug_name)
                        .map(|arg| escape_wat_comment(&arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                for arg in args {
                    render_core_value_i32(wat, arg, &env)?;
                }
                wat.push_str(&format!("    call ${}\n", final_symbol(func)));
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
        scalar => Some(extern_scalar_wat_type(scalar)?),
    };
    Some((params, result))
}

fn extern_scalar_wat_type(scalar: &str) -> Option<&'static str> {
    match scalar {
        "i64" | "I64" | "bool" | "Bool" => Some("i32"),
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
    if result_binders.len() <= 1 {
        return Ok(false);
    }
    let chain_env = env.with_locals(result_binders.clone());
    wat.push_str("  ;; tailcall-result-chain\n");
    render_func_header(wat, "chiba_tailcall_0", None, &chain_env);
    for binder in &result_binders {
        wat.push_str(&format!(
            "    (local ${} i32)\n",
            encode_debug_symbol(binder)
        ));
    }

    for (index, op) in core.ops.iter().enumerate() {
        if let CoreOp::TailCall { func, args } = op {
            let Some(CoreOp::TailCallResult { binder }) = core.ops.get(index + 1) else {
                continue;
            };
            wat.push_str(&format!(
                "    ;; tailcall {} args=[{}]\n",
                final_symbol(func),
                args.iter()
                    .map(CoreValue::debug_name)
                    .map(|arg| escape_wat_comment(&arg))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            for arg in args {
                render_core_value_i32(wat, arg, &chain_env)?;
            }
            wat.push_str(&format!("    call ${}\n", final_symbol(func)));
            wat.push_str(&format!("    local.set ${}\n", encode_debug_symbol(binder)));
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
    locals: BTreeSet<String>,
    bindings: BTreeMap<String, CoreValue>,
}

impl RenderEnv {
    fn new(params: &[String]) -> Self {
        Self {
            params: params.iter().cloned().collect(),
            locals: BTreeSet::new(),
            bindings: BTreeMap::new(),
        }
    }

    fn is_param(&self, name: &str) -> bool {
        self.params.contains(name) || self.locals.contains(name)
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
        next.locals.extend(names);
        next
    }

    fn signature(&self) -> String {
        self.params
            .iter()
            .map(|param| format!(" (param ${} i32)", encode_debug_symbol(param)))
            .collect()
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
        CoreValue::Var(name) if env.is_param(name) => {
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
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::RangeField { range, field } => {
            if let Some(value) = range_field_value(range, *field, env) {
                render_core_value_i32(wat, value, env)?;
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::Adt { ctor, variants, .. } => {
            if let Some(tag) = variants.iter().position(|variant| variant == ctor) {
                wat.push_str(&format!("    i32.const {tag}\n"));
            } else {
                return Err(unsupported_i32_render_diagnostic(value));
            }
        }
        CoreValue::Var(_)
        | CoreValue::Tuple { .. }
        | CoreValue::Range { .. }
        | CoreValue::Record { .. }
        | CoreValue::RecordUpdate { .. }
        | CoreValue::Rendered { .. } => {
            return Err(unsupported_i32_render_diagnostic(value));
        }
    }
    Ok(())
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
        CoreValue::Var(name) => env.is_param(name),
        CoreValue::TupleField {
            tuple, field_index, ..
        } => tuple_field_value(tuple, *field_index, env)
            .map(|value| core_value_is_renderable_i32(value, env))
            .unwrap_or(false),
        CoreValue::RecordField { record, field } => record_field_value(record, field, env)
            .map(|value| core_value_is_renderable_i32(value, env))
            .unwrap_or(false),
        CoreValue::RangeField { range, field } => range_field_value(range, *field, env)
            .map(|value| core_value_is_renderable_i32(value, env))
            .unwrap_or(false),
        CoreValue::Tuple { .. }
        | CoreValue::Range { .. }
        | CoreValue::Record { .. }
        | CoreValue::RecordUpdate { .. }
        | CoreValue::Rendered { .. } => false,
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
