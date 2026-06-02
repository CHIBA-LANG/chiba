use std::collections::BTreeSet;

use crate::core::{CoreOp, CoreProgram, CoreValidation, OwnershipDecision};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackendArtifact {
    pub target: BackendTarget,
    pub wat: String,
    pub manifest: BackendManifest,
    pub diagnostics: Vec<BackendDiagnostic>,
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
        };
    }

    let manifest = manifest_for_core(core);
    BackendArtifact {
        target: BackendTarget::WasmGc,
        wat: render_wat(core, &manifest),
        manifest,
        diagnostics: vec![],
    }
}

fn manifest_for_core(core: &CoreProgram) -> BackendManifest {
    let mut entries = Vec::new();
    for op in &core.ops {
        match op {
            CoreOp::LiftedFunction { source, symbol, .. } => entries.push(BackendManifestEntry {
                final_symbol: final_symbol(symbol),
                source_debug_name: source.clone(),
                pass_origin: "L13LambdaLift".to_string(),
                ownership: ownership_for_subject(core, source),
            }),
            CoreOp::DirectMethodTarget { name, target } => entries.push(BackendManifestEntry {
                final_symbol: final_symbol(target),
                source_debug_name: name.clone(),
                pass_origin: "L14Core".to_string(),
                ownership: ownership_for_subject(core, target),
            }),
            CoreOp::OperatorTarget { protocol, target } => entries.push(BackendManifestEntry {
                final_symbol: final_symbol(target),
                source_debug_name: protocol.clone(),
                pass_origin: "L14Core".to_string(),
                ownership: ownership_for_subject(core, target),
            }),
            CoreOp::ReturnAtom(_)
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
    for entry in &manifest.entries {
        wat.push_str(&format!(
            "  ;; source={} origin={}\n",
            entry.source_debug_name, entry.pass_origin
        ));
        wat.push_str(&format!("  (func ${} (result i32)\n", entry.final_symbol));
        wat.push_str("    i32.const 0)\n");
    }
    for op in &core.ops {
        if let CoreOp::TailCall { func, .. } = op {
            wat.push_str(&format!("  ;; tailcall {}\n", final_symbol(func)));
        }
    }
    wat.push_str(")\n");
    wat
}

fn final_symbol(symbol: &str) -> String {
    symbol
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch
            } else {
                '_'
            }
        })
        .collect()
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
        for line in artifact.wat.lines() {
            if line == "(module" || line == ")" {
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
