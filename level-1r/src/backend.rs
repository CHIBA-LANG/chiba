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
