use chiba_level1r::backend::{
    backend_cache_key, emit_wasm_gc, link_backend_artifacts, BackendCacheConfig,
    BackendDiagnostic, BackendExternImport, BackendLinkDiagnostic, BackendTarget,
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
    assert!(output.backend.wat.contains("origin=L15LambdaLift"));

    let entry = output
        .backend
        .manifest
        .entries
        .iter()
        .find(|entry| entry.source_debug_name == "closure::x")
        .unwrap();
    assert_eq!(entry.final_symbol, "lift__0000__closure__x");
    assert_eq!(entry.pass_origin, "L15LambdaLift");
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
    assert!(artifact.wat.contains(";; tailcall math_Vec2_norm arg=v"));
    assert_eq!(artifact.manifest.entries[0].source_debug_name, "norm");
}

#[test]
fn backend_emits_exported_main_for_return_atom_core() {
    let output = compile_expr(&Expr::i64(7));

    assert_eq!(output.backend.diagnostics, vec![]);
    assert!(output.backend.wat.contains(";; core-return atom=I64(7)"));
    assert!(output.backend.wat.contains("(func $main (export \"main\") (result i32)"));
    assert!(output.backend.wat.contains("i32.const 7"));
}

#[test]
fn backend_serializes_structural_core_ops_as_debuggable_wat_comments() {
    let output = compile_expr(&Expr::record(vec![
        ("x", Expr::i64(1)),
        ("y", Expr::bool(true)),
    ]));

    assert_eq!(output.backend.diagnostics, vec![]);
    assert!(output.backend.wat.contains(";; record layout=record::x+y fields=2"));
    assert!(output.backend.wat.contains(";; core-return atom=record::x+y{x=1, y=true}"));
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
    assert!(visual.contains("backend-cache-key:"));
    assert!(visual.contains("BackendLinkedBundle"));
    assert!(visual.contains("BackendCacheKey"));
    assert!(visual.contains("L23BackendLink: BackendArtifact -> BackendLinkedBundle"));
    assert!(visual.contains("L24BackendCacheKey: BackendLinkedBundle -> BackendCacheKey"));
}

#[test]
fn backend_cache_key_is_stable_across_manifest_order() {
    let first = emit_wasm_gc(
        &CoreProgram {
            ops: vec![CoreOp::DirectMethodTarget {
                name: "zeta".to_string(),
                target: "z::target".to_string(),
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
                target: "a::target".to_string(),
            }],
            layouts: vec![],
            ownership: vec![],
            callable_storage: vec![],
        },
        &CoreValidation::default(),
    );
    let left = link_backend_artifacts(vec![first.clone(), second.clone()]);
    let right = link_backend_artifacts(vec![second, first]);

    assert_eq!(
        backend_cache_key(&left, &BackendCacheConfig::default()),
        backend_cache_key(&right, &BackendCacheConfig::default())
    );
}

#[test]
fn backend_cache_key_distinguishes_target_features_and_imports() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));
    let mut wasi = BackendCacheConfig::default();
    wasi.imports.push(BackendExternImport {
        abi: "wasi".to_string(),
        module: "wasi_snapshot_preview1".to_string(),
        name: "fd_write".to_string(),
        signature_hash: "i32_i32_i32_i32_to_i32".to_string(),
    });
    let mut env = BackendCacheConfig::default();
    env.imports.push(BackendExternImport {
        abi: "C".to_string(),
        module: "env".to_string(),
        name: "js_log".to_string(),
        signature_hash: "i32_to_unit".to_string(),
    });
    let mut env_lowercase = env.clone();
    env_lowercase.imports[0].abi = "c".to_string();

    let wasi_key = backend_cache_key(&output.backend_link, &wasi);
    let env_key = backend_cache_key(&output.backend_link, &env);
    let env_lowercase_key = backend_cache_key(&output.backend_link, &env_lowercase);

    assert_ne!(wasi_key, env_key);
    assert_eq!(env_key, env_lowercase_key);

    let mut no_tailcall = BackendCacheConfig::default();
    no_tailcall.features.tailcall = false;
    assert_ne!(
        backend_cache_key(&output.backend_link, &BackendCacheConfig::default()),
        backend_cache_key(&output.backend_link, &no_tailcall)
    );
    assert_eq!(
        output.backend_cache_key,
        backend_cache_key(&output.backend_link, &BackendCacheConfig::default())
    );
}
