use chiba_level1r::backend::{emit_wasm_gc, BackendDiagnostic, BackendTarget};
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
