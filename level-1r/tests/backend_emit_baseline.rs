use chiba_level1r::backend::{
    backend_cache_key, emit_wasm_gc, emit_wasm_gc_with_params, link_backend_artifacts,
    BackendCacheConfig, BackendDiagnostic, BackendExternAbi, BackendExternImport,
    BackendLinkDiagnostic, BackendTarget,
};
use chiba_level1r::control::ContinuationKind;
use chiba_level1r::core::{
    CoreDiagnostic, CoreOp, CoreProgram, CoreValidation, CoreValue, OperatorIntrinsic,
};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn backend_refuses_to_emit_when_core_validation_failed() {
    let core = CoreProgram {
        ops: vec![CoreOp::TailCall {
            func: "module::missing".to_string(),
            args: vec![CoreValue::Var("x".to_string())],
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
fn backend_emits_lifted_symbol_manifest_for_lambda_surface() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert_eq!(
        output.backend.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "lambda#x".to_string(),
        }]
    );
    assert_eq!(output.backend.wat, "");

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
                args: vec![CoreValue::Var("v".to_string())],
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    let validation = CoreValidation::default();

    let artifact = emit_wasm_gc_with_params(&core, &validation, &["v".to_string()]);

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact
        .wat
        .contains(";; symbol math_Vec2_norm source=norm origin=L16Core"));
    assert!(artifact.wat.contains(";; tailcall math_Vec2_norm args=[v]"));
    assert!(artifact.wat.contains("call $math_Vec2_norm"));
    assert!(!artifact.wat.contains("(func $math_Vec2_norm"));
    assert_eq!(artifact.manifest.entries[0].source_debug_name, "norm");
}

#[test]
fn backend_rejects_continuation_runtime_ops_until_lowering_is_executable() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::Prompt {
                kind: ContinuationKind::ContN,
            },
            CoreOp::CaptureContinuation {
                binder: "retry".to_string(),
                kind: ContinuationKind::ContN,
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedContinuationRuntime {
            op: "prompt".to_string(),
            kind: ContinuationKind::ContN,
            binder: None,
        }]
    );
    assert_eq!(artifact.wat, "");
    assert!(!artifact.wat.contains("kind=ContN"));
    assert!(!artifact.wat.contains("kind=Cont1"));
}

#[test]
fn backend_skips_tailcall_result_temp_instead_of_faking_return_zero() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::DirectMethodTarget {
                name: "helper".to_string(),
                target: "helper".to_string(),
            },
            CoreOp::TailCall {
                func: "helper".to_string(),
                args: vec![CoreValue::I64(7)],
            },
            CoreOp::ReturnValue(CoreValue::Var("w0".to_string())),
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains(";; tailcall helper args=[7]"));
    assert!(artifact.wat.contains("call $helper"));
    assert!(!artifact.wat.contains("core-return atom=w0"));
    assert!(!artifact.wat.contains("(func $main"));
}

#[test]
fn backend_skips_nested_tailcall_result_temp_instead_of_rejecting_w0() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::DirectMethodTarget {
                name: "inner".to_string(),
                target: "inner".to_string(),
            },
            CoreOp::DirectMethodTarget {
                name: "outer".to_string(),
                target: "outer".to_string(),
            },
            CoreOp::TailCall {
                func: "inner".to_string(),
                args: vec![CoreValue::I64(6)],
            },
            CoreOp::TailCallResult {
                binder: "inner_result".to_string(),
            },
            CoreOp::TailCall {
                func: "outer".to_string(),
                args: vec![CoreValue::Var("inner_result".to_string())],
            },
            CoreOp::TailCallResult {
                binder: "outer_result".to_string(),
            },
            CoreOp::ReturnValue(CoreValue::Var("outer_result".to_string())),
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains(";; tailcall-result-chain"));
    assert!(artifact.wat.contains(";; tailcall inner args=[6]"));
    assert!(artifact.wat.contains("call $inner"));
    assert!(artifact.wat.contains("local.set $inner_result"));
    assert!(artifact.wat.contains("local.get $inner_result"));
    assert!(artifact.wat.contains("call $outer"));
    assert!(artifact.wat.contains("local.set $outer_result"));
    assert!(!artifact.wat.contains("core-return atom=outer_result"));
}

#[test]
fn backend_rejects_tailcall_unbound_arg_instead_of_comment_only_success() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::DirectMethodTarget {
                name: "norm".to_string(),
                target: "math.Vec2.norm".to_string(),
            },
            CoreOp::TailCall {
                func: "math.Vec2.norm".to_string(),
                args: vec![CoreValue::Var("v".to_string())],
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "v".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_rejects_tailcall_structural_arg_instead_of_dropping_call_body() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::DirectMethodTarget {
                name: "take".to_string(),
                target: "demo.take".to_string(),
            },
            CoreOp::TailCall {
                func: "demo.take".to_string(),
                args: vec![CoreValue::Record {
                    fields: vec![chiba_level1r::core::CoreRecordValueField {
                        name: "x".to_string(),
                        value: CoreValue::I64(1),
                    }],
                }],
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "{x=1}".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_emits_exported_main_for_return_atom_core() {
    let output = compile_expr(&Expr::i64(7));

    assert_eq!(output.backend.diagnostics, vec![]);
    assert!(output.backend.wat.contains(";; core-return atom=7"));
    assert!(output
        .backend
        .wat
        .contains("(func $main (export \"main\") (result i32)"));
    assert!(output.backend.wat.contains("i32.const 7"));
}

#[test]
fn backend_emits_params_and_local_get_for_param_return_core() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::Var("x".to_string()))],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    let artifact = emit_wasm_gc_with_params(&core, &CoreValidation::default(), &["x".to_string()]);

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact
        .wat
        .contains("(func $main (export \"main\") (param $x i32) (result i32)"));
    assert!(artifact.wat.contains("local.get $x"));
    assert!(!artifact.wat.contains("i32.const 0"));
}

#[test]
fn backend_rejects_range_return_instead_of_faking_i32_zero() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::Range {
            start: Box::new(CoreValue::I64(1)),
            end: Box::new(CoreValue::I64(3)),
        })],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "1..3".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_rejects_missing_record_field_return_instead_of_faking_i32_zero() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::RecordField {
            record: Box::new(CoreValue::Record { fields: vec![] }),
            field: "missing".to_string(),
        })],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "{}.missing".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_rejects_missing_tuple_field_return_instead_of_faking_i32_zero() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::TupleField {
            tuple: Box::new(CoreValue::Tuple {
                fields: vec![CoreValue::I64(1)],
            }),
            field: "_2".to_string(),
            field_index: 1,
        })],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "(1)._2".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_rejects_tuple_return_instead_of_faking_i32_zero() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::Tuple {
            fields: vec![CoreValue::I64(1), CoreValue::Bool(true)],
        })],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "(1, true)".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_rejects_record_return_instead_of_faking_i32_zero() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::Record {
            fields: vec![chiba_level1r::core::CoreRecordValueField {
                name: "x".to_string(),
                value: CoreValue::I64(1),
            }],
        })],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "{x=1}".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_rejects_rendered_return_instead_of_faking_i32_zero() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::Rendered {
            debug: "opaque-debug-value".to_string(),
        })],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "opaque-debug-value".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_rejects_unknown_adt_constructor_return_instead_of_faking_tag_zero() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::Adt {
            data: "Option".to_string(),
            ctor: "Ghost".to_string(),
            variants: vec!["None".to_string(), "Some".to_string()],
            args: vec![],
        })],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "Option.Ghost()".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_rejects_structural_core_return_instead_of_debug_comment_fake_wat() {
    let output = compile_expr(&Expr::record(vec![
        ("x", Expr::i64(1)),
        ("y", Expr::bool(true)),
    ]));

    assert_eq!(
        output.backend.diagnostics,
        vec![BackendDiagnostic::UnsupportedI32ReturnValue {
            value: "{x=1, y=true}".to_string(),
        }]
    );
    assert_eq!(output.backend.wat, "");
    assert!(output.core.ops.contains(&CoreOp::RecordConstruct {
        layout: "record::x+y".to_string(),
        fields: vec!["x".to_string(), "y".to_string()],
    }));
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
            ops: vec![
                CoreOp::OperatorTarget {
                    protocol: "Add".to_string(),
                    target: "operator::Add(x)".to_string(),
                    intrinsic: Some(OperatorIntrinsic::I64Add),
                },
                CoreOp::TailCall {
                    func: "operator::Add(x)".to_string(),
                    args: vec![CoreValue::I64(1), CoreValue::I64(2)],
                },
            ],
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
    assert_eq!(symbols, vec!["math__add", "operator__Add_u28_x_u29_"]);
    assert!(bundle.linked_wat.contains(";; linked artifact 0"));
    assert!(bundle
        .linked_wat
        .contains(";; symbol operator__Add_u28_x_u29_ source=Add origin=L16Core"));
    assert!(bundle
        .linked_wat
        .contains(";; symbol math__add source=math.add origin=L16Core"));
    assert!(bundle
        .linked_wat
        .contains("(func $operator__Add_u28_x_u29_"));
    assert!(bundle.linked_wat.contains("i32.add"));
    assert!(!bundle.linked_wat.contains("(func $math__add"));
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
fn backend_utf8_final_symbols_are_encoded_without_collision() {
    let first = emit_wasm_gc(
        &CoreProgram {
            ops: vec![CoreOp::DirectMethodTarget {
                name: "函数α".to_string(),
                target: "模块::函数α".to_string(),
            }],
            layouts: vec![],
            ownership: vec![],
            callable_storage: vec![],
        },
        &CoreValidation::default(),
    );
    let second = emit_wasm_gc(
        &CoreProgram {
            ops: vec![CoreOp::DirectMethodTarget {
                name: "函数β".to_string(),
                target: "模块::函数β".to_string(),
            }],
            layouts: vec![],
            ownership: vec![],
            callable_storage: vec![],
        },
        &CoreValidation::default(),
    );

    let bundle = link_backend_artifacts(vec![first, second]);

    assert_eq!(bundle.diagnostics, vec![]);
    let symbols = bundle
        .manifest
        .entries
        .iter()
        .map(|entry| entry.final_symbol.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        symbols,
        vec![
            "_u6A21__u5757____u51FD__u6570__u3B1_",
            "_u6A21__u5757____u51FD__u6570__u3B2_",
        ]
    );
}

#[test]
fn pipeline_records_backend_link_artifact() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert_eq!(
        output.backend_link.diagnostics,
        vec![BackendLinkDiagnostic::ArtifactEmitFailed { artifact_index: 0 }]
    );
    assert_eq!(output.backend_link.linked_wat, "");
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
                intrinsic: None,
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
        abi: BackendExternAbi::Wasi,
        module: "wasi_snapshot_preview1".to_string(),
        name: "fd_write".to_string(),
        signature_hash: "i32_i32_i32_i32_to_i32".to_string(),
    });
    let mut env = BackendCacheConfig::default();
    env.imports.push(BackendExternImport {
        abi: BackendExternAbi::C,
        module: "env".to_string(),
        name: "js_log".to_string(),
        signature_hash: "i32_to_unit".to_string(),
    });
    let mut env_lowercase = env.clone();
    env_lowercase.imports[0].abi = BackendExternAbi::C;

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
