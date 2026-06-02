use chiba_level1r::backend::{
    emit_wasm_gc, link_backend_artifacts, BackendDiagnostic, BackendLinkDiagnostic, BackendTarget,
};
use chiba_level1r::core::{CoreDiagnostic, CoreOp, CoreProgram, CoreValidation};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn backend_refuses_to_emit_when_core_validation_failed() {
    let core = CoreProgram {
        ops: vec![CoreOp::TailCall {
            func: "module::missing".to_string(),
            arg: "x".to_string(),
        }],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    let validation = CoreValidation {
        diagnostics: vec![CoreDiagnostic::DanglingTailCallTarget {
            target: "module::missing".to_string(),
        }],
    };

    let artifact = emit_wasm_gc(&core, &validation);

    assert_eq!(artifact.target, BackendTarget::WasmGc);
    assert_eq!(artifact.wat, "");
    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::CoreValidationFailed { diagnostics: 1 }]
    );
}

#[test]
fn backend_emits_wasm_gc_wat_and_manifest_for_lifted_symbols() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert_eq!(output.backend.diagnostics, vec![]);
    assert!(output.backend.wat.starts_with("(module\n"));
    assert!(output.backend.wat.contains("(func $lift__0000__closure__x"));
    assert!(output.backend.wat.contains("source=closure::x"));
    assert!(output.backend.wat.contains("origin=L13LambdaLift"));

    let entry = output
        .backend
        .manifest
        .entries
        .iter()
        .find(|entry| entry.source_debug_name == "closure::x")
        .unwrap();
    assert_eq!(entry.final_symbol, "lift__0000__closure__x");
    assert_eq!(entry.pass_origin, "L13LambdaLift");
}

#[test]
fn backend_records_tailcall_targets_in_serialized_output() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::DirectMethodTarget {
                name: "norm".to_string(),
                target: "math.Vec2.norm".to_string(),
            },
            CoreOp::TailCall {
                func: "math.Vec2.norm".to_string(),
                arg: "v".to_string(),
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    let validation = CoreValidation::default();

    let artifact = emit_wasm_gc(&core, &validation);

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains("(func $math_Vec2_norm"));
    assert!(artifact.wat.contains(";; tailcall math_Vec2_norm"));
    assert_eq!(artifact.manifest.entries[0].source_debug_name, "norm");
}

#[test]
fn visual_report_contains_backend_artifact() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert!(output.render_visual().contains("backend:"));
    assert!(output.render_visual().contains("BackendArtifact"));
    assert!(output.render_visual().contains("WasmGc"));
}

#[test]
fn backend_link_merges_artifacts_manifest_and_wat_deterministically() {
    let first = emit_wasm_gc(
        &CoreProgram {
            ops: vec![CoreOp::DirectMethodTarget {
                name: "math.add".to_string(),
                target: "math::add".to_string(),
            }],
            layouts: vec![],
            ownership: vec![],
            callable_storage: vec![],
        },
        &CoreValidation::default(),
    );
    let second = emit_wasm_gc(
        &CoreProgram {
            ops: vec![CoreOp::OperatorTarget {
                protocol: "operator.add".to_string(),
                target: "intrinsic::i64::add".to_string(),
            }],
            layouts: vec![],
            ownership: vec![],
            callable_storage: vec![],
        },
        &CoreValidation::default(),
    );

    let bundle = link_backend_artifacts(vec![second, first]);

    assert_eq!(bundle.target, BackendTarget::WasmGc);
    assert_eq!(bundle.diagnostics, vec![]);
    let symbols: Vec<_> = bundle
        .manifest
        .entries
        .iter()
        .map(|entry| entry.final_symbol.as_str())
        .collect();
    assert_eq!(symbols, vec!["intrinsic__i64__add", "math__add"]);
    assert!(bundle.linked_wat.contains(";; linked artifact 0"));
    assert!(bundle.linked_wat.contains("(func $intrinsic__i64__add"));
    assert!(bundle.linked_wat.contains("(func $math__add"));
}

#[test]
fn backend_link_rejects_duplicate_final_symbols() {
    let core = CoreProgram {
        ops: vec![CoreOp::DirectMethodTarget {
            name: "same".to_string(),
            target: "a::b".to_string(),
        }],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    let first = emit_wasm_gc(&core, &CoreValidation::default());
    let second = emit_wasm_gc(&core, &CoreValidation::default());

    let bundle = link_backend_artifacts(vec![first, second]);

    assert_eq!(
        bundle.diagnostics,
        vec![BackendLinkDiagnostic::DuplicateFinalSymbol {
            symbol: "a__b".to_string()
        }]
    );
    assert_eq!(bundle.linked_wat, "");
}

#[test]
fn pipeline_records_backend_link_artifact() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert_eq!(output.backend_link.diagnostics, vec![]);
    assert!(output.backend_link.linked_wat.starts_with("(module\n"));
    assert_eq!(
        output
            .backend_link
            .manifest
            .entries
            .iter()
            .map(|entry| entry.final_symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["lift__0000__closure__x"]
    );

    let visual = output.render_visual();
    assert!(visual.contains("backend-link:"));
    assert!(visual.contains("BackendLinkedBundle"));
    assert!(visual.contains("L19BackendLink: BackendArtifact -> BackendLinkedBundle"));
}
