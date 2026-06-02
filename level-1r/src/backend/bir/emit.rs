use std::collections::BTreeSet;

use crate::core::{
    CoreMatchArm, CoreOp, CorePattern, CoreProgram, CoreValidation, CoreValue, OwnershipDecision,
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
    CoreValidationFailed { diagnostics: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendLinkDiagnostic {
    ArtifactEmitFailed { artifact_index: usize },
    DuplicateFinalSymbol { symbol: String },
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
    pub abi: String,
    pub module: String,
    pub name: String,
    pub signature_hash: String,
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
    BackendArtifact {
        target: BackendTarget::WasmGc,
        wat: render_wat(core, &manifest),
        manifest,
        diagnostics: vec![],
        return_value: first_return_value(core),
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
            CoreOp::OperatorTarget { protocol, target } => entries.push(BackendManifestEntry {
                final_symbol: final_symbol(target),
                source_debug_name: protocol.clone(),
                pass_origin: "L16Core".to_string(),
                ownership: ownership_for_subject(core, target),
            }),
            CoreOp::ReturnValue(_)
            | CoreOp::ReturnBranch { .. }
            | CoreOp::ReturnMatch { .. }
            | CoreOp::DynamicCallableTarget { .. }
            | CoreOp::TupleConstruct { .. }
            | CoreOp::TupleFieldGet { .. }
            | CoreOp::RecordConstruct { .. }
            | CoreOp::RecordUpdate { .. }
            | CoreOp::RecordFieldGet { .. }
            | CoreOp::AdtConstruct { .. }
            | CoreOp::TailCall { .. }
            | CoreOp::Prompt { .. }
            | CoreOp::CaptureContinuation { .. }
            | CoreOp::Branch { .. }
            | CoreOp::Match { .. }
            | CoreOp::StaticRowAccess { .. }
            | CoreOp::DynRowAdapterAccess { .. } => {}
        }
    }
    BackendManifest { entries }
}

fn ownership_for_subject(core: &CoreProgram, subject: &str) -> Option<OwnershipDecision> {
    core.ownership
        .iter()
        .find(|fact| fact.subject == subject)
        .map(|fact| fact.decision)
}

fn render_wat(core: &CoreProgram, manifest: &BackendManifest) -> String {
    let mut wat = String::from("(module\n");
    let mut return_index = 0usize;
    let mut tailcall_index = 0usize;
    for entry in &manifest.entries {
        wat.push_str(&format!(
            "  ;; source={} origin={}\n",
            entry.source_debug_name, entry.pass_origin
        ));
        wat.push_str(&format!("  (func ${} (result i32)\n", entry.final_symbol));
        wat.push_str("    i32.const 0)\n");
    }
    for op in &core.ops {
        match op {
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
                wat.push_str(&format!("  (func ${symbol} (export \"{symbol}\") (result i32)\n"));
                render_core_value_i32(&mut wat, value);
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
                wat.push_str(&format!("  (func ${symbol} (export \"{symbol}\") (result i32)\n"));
                render_core_value_i32(&mut wat, cond);
                wat.push_str("    if (result i32)\n");
                render_core_value_i32_indented(&mut wat, then_value, 6);
                wat.push_str("    else\n");
                render_core_value_i32_indented(&mut wat, else_value, 6);
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
                wat.push_str(&format!("  (func ${symbol} (export \"{symbol}\") (result i32)\n"));
                render_match_arms_i32(&mut wat, scrutinee, arms, 4);
                wat.push_str("  )\n");
                return_index += 1;
            }
            CoreOp::TailCall { func, args } => {
                wat.push_str(&format!(
                    "  ;; tailcall {} args=[{}]\n",
                    final_symbol(func),
                    args.iter()
                        .map(|arg| escape_wat_comment(arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                if args.is_empty() {
                    let symbol = format!("chiba_tailcall_{tailcall_index}");
                    wat.push_str(&format!("  (func ${symbol} (result i32)\n"));
                    wat.push_str(&format!("    call ${}\n", final_symbol(func)));
                    wat.push_str("  )\n");
                    tailcall_index += 1;
                }
            }
            CoreOp::Branch { cond } => {
                wat.push_str(&format!("  ;; branch cond={}\n", escape_wat_comment(cond)));
            }
            CoreOp::Match { scrutinee, patterns } => {
                wat.push_str(&format!(
                    "  ;; match scrutinee={} arms={}\n",
                    escape_wat_comment(scrutinee),
                    patterns.len()
                ));
            }
            CoreOp::TupleConstruct { layout, fields } => {
                wat.push_str(&format!(
                    "  ;; tuple layout={} fields={}\n",
                    escape_wat_comment(layout),
                    fields.len()
                ));
            }
            CoreOp::TupleFieldGet { layout, field } => {
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
            CoreOp::Prompt { kind } => {
                wat.push_str(&format!("  ;; prompt kind={kind:?}\n"));
            }
            CoreOp::CaptureContinuation { binder, kind } => {
                wat.push_str(&format!(
                    "  ;; capture-cont binder={} kind={kind:?}\n",
                    escape_wat_comment(binder)
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
            | CoreOp::OperatorTarget { .. }
            | CoreOp::LiftedFunction { .. } => {}
        }
    }
    wat.push_str(")\n");
    wat
}

fn render_core_value_i32(wat: &mut String, value: &CoreValue) {
    match value {
        CoreValue::Unit => wat.push_str("    i32.const 0\n"),
        CoreValue::I64(value) => wat.push_str(&format!("    i32.const {}\n", *value as i32)),
        CoreValue::Bool(value) => wat.push_str(&format!("    i32.const {}\n", i32::from(*value))),
        CoreValue::TupleField { tuple, field } => {
            if let Some(value) = tuple_field_value(tuple, field) {
                render_core_value_i32(wat, value);
            } else {
                wat.push_str("    i32.const 0\n");
            }
        }
        CoreValue::RecordField { record, field } => {
            if let Some(value) = record_field_value(record, field) {
                render_core_value_i32(wat, value);
            } else {
                wat.push_str("    i32.const 0\n");
            }
        }
        CoreValue::Adt { ctor, variants, .. } => {
            if let Some(tag) = variants.iter().position(|variant| variant == ctor) {
                wat.push_str(&format!("    i32.const {tag}\n"));
            } else {
                wat.push_str("    i32.const 0\n");
            }
        }
        CoreValue::Var(_)
        | CoreValue::Tuple { .. }
        | CoreValue::Record { .. }
        | CoreValue::Rendered { .. } => {
            wat.push_str("    i32.const 0\n")
        }
    }
}

fn tuple_field_value<'a>(tuple: &'a CoreValue, field: &str) -> Option<&'a CoreValue> {
    let CoreValue::Tuple { fields } = tuple else {
        return None;
    };
    let index = field.strip_prefix('_')?.parse::<usize>().ok()?.checked_sub(1)?;
    fields.get(index)
}

fn record_field_value<'a>(record: &'a CoreValue, field: &str) -> Option<&'a CoreValue> {
    let CoreValue::Record { fields } = record else {
        return None;
    };
    fields
        .iter()
        .find(|candidate| candidate.name == field)
        .map(|candidate| &candidate.value)
}

fn render_core_value_i32_indented(wat: &mut String, value: &CoreValue, indent: usize) {
    let mut nested = String::new();
    render_core_value_i32(&mut nested, value);
    for line in nested.lines() {
        wat.push_str(&" ".repeat(indent));
        wat.push_str(line.trim_start());
        wat.push('\n');
    }
}

fn render_match_arms_i32(
    wat: &mut String,
    scrutinee: &CoreValue,
    arms: &[CoreMatchArm],
    indent: usize,
) {
    let Some((first, rest)) = arms.split_first() else {
        wat.push_str(&" ".repeat(indent));
        wat.push_str("unreachable\n");
        return;
    };
    render_match_arm_i32(wat, scrutinee, first, rest, indent);
}

fn render_match_arm_i32(
    wat: &mut String,
    scrutinee: &CoreValue,
    arm: &CoreMatchArm,
    rest: &[CoreMatchArm],
    indent: usize,
) {
    match &arm.pattern {
        CorePattern::Wildcard => render_core_value_i32_indented(wat, &arm.value, indent),
        CorePattern::I64(value) => {
            render_core_value_i32_indented(wat, scrutinee, indent);
            push_indent(wat, indent);
            wat.push_str(&format!("i32.const {}\n", *value as i32));
            push_indent(wat, indent);
            wat.push_str("i32.eq\n");
            push_indent(wat, indent);
            wat.push_str("if (result i32)\n");
            render_core_value_i32_indented(wat, &arm.value, indent + 2);
            push_indent(wat, indent);
            wat.push_str("else\n");
            render_match_arms_i32(wat, scrutinee, rest, indent + 2);
            push_indent(wat, indent);
            wat.push_str("end\n");
        }
        CorePattern::Bool(value) => {
            render_core_value_i32_indented(wat, scrutinee, indent);
            push_indent(wat, indent);
            wat.push_str(&format!("i32.const {}\n", i32::from(*value)));
            push_indent(wat, indent);
            wat.push_str("i32.eq\n");
            push_indent(wat, indent);
            wat.push_str("if (result i32)\n");
            render_core_value_i32_indented(wat, &arm.value, indent + 2);
            push_indent(wat, indent);
            wat.push_str("else\n");
            render_match_arms_i32(wat, scrutinee, rest, indent + 2);
            push_indent(wat, indent);
            wat.push_str("end\n");
        }
    }
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
    }

    entries.sort_by(|left, right| left.final_symbol.cmp(&right.final_symbol));

    if !diagnostics.is_empty() {
        return BackendLinkedBundle {
            target,
            linked_wat: String::new(),
            manifest: BackendManifest { entries },
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
            linked_wat.push_str(line);
            linked_wat.push('\n');
        }
    }
    linked_wat.push_str(")\n");

    BackendLinkedBundle {
        target,
        linked_wat,
        manifest: BackendManifest { entries },
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
    encoded.push_str(if config.features.wasi { "wasi" } else { "no-wasi" });
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
    imports.sort_by(|left, right| canonical_import(left).cmp(&canonical_import(right)));
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
        "{}::{}::{}::{}",
        canonical_abi(&import.abi),
        import.module,
        import.name,
        import.signature_hash
    )
}

fn canonical_abi(abi: &str) -> String {
    if abi.eq_ignore_ascii_case("c") {
        "c".to_string()
    } else {
        abi.to_ascii_lowercase()
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
