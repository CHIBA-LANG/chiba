use chiba_level1r::backend::{
    backend_cache_key, emit_wasm32_nogc, emit_wasm_gc, emit_wasm_gc_with_params,
    link_backend_artifacts, BackendCacheConfig, BackendDiagnostic, BackendExternAbi,
    BackendExternImport, BackendLayoutPolicy, BackendLinkDiagnostic, BackendOwnershipRuntime,
    BackendTarget,
};
use chiba_level1r::control::ContinuationKind;
use chiba_level1r::core::{
    ClosureEnvField, ClosureEnvLayout, CoreCapturedContinuation, CoreDiagnostic, CoreOp,
    CoreProgram, CoreValidation, CoreValue, LayoutFact, LayoutKind, OperatorIntrinsic, SliceField,
};
use chiba_level1r::typed::{AggregateKind, SendColor, UsageColor};
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

    assert_eq!(output.backend.diagnostics, vec![]);
    assert!(output.backend.wat.contains("(result i32)"));

    let entry = output
        .backend
        .manifest
        .entries
        .iter()
        .find(|entry| entry.source_debug_name == "closure::x")
        .unwrap();
    assert_eq!(entry.final_symbol, "lift__0000__closure__x");
    assert_eq!(
        entry.source.item_path.as_deref(),
        Some("lift::0000::closure__x")
    );
    assert_eq!(entry.source.source_path, None);
    assert_eq!(entry.source.owner_namespace, None);
    assert_eq!(entry.source.layout_key, None);
    assert_eq!(entry.source.specialization_key, None);
    assert_eq!(entry.pass_origin, "L15LambdaLift");
    assert_eq!(entry.lowering_role, "lifted-function");
    assert_eq!(entry.stable_id.len(), 16);
    assert!(output.backend.wat.contains(&format!(
        ";; symbol lift__0000__closure__x source=closure::x origin=L15LambdaLift role=lifted-function stable-id={} owner=none item=lift::0000::closure__x layout=none specialization=none",
        entry.stable_id
    )));
    assert!(output.render_visual().contains(&format!(
        "symbol lift__0000__closure__x source=closure::x source-path=none owner=none item=lift::0000::closure__x specialization=none layout=none origin=L15LambdaLift role=lifted-function stable-id={}",
        entry.stable_id
    )));
}

#[test]
fn backend_manifest_links_final_symbol_to_layout_fact_when_core_proves_it() {
    let core = CoreProgram {
        ops: vec![CoreOp::LiftedFunction {
            source: "closure::y".to_string(),
            symbol: "lift::0000::closure::y".to_string(),
            env_params: vec!["x".to_string()],
            direct: false,
            param: Some("y".to_string()),
            body: vec![CoreOp::ReturnValue(CoreValue::Var("y".to_string()))],
        }],
        layouts: vec![LayoutFact {
            key: "closure-env::closure::y".to_string(),
            hash: 1,
            kind: LayoutKind::ClosureEnv(ClosureEnvLayout {
                closure: "closure::y".to_string(),
                fields: vec![ClosureEnvField {
                    name: "x".to_string(),
                    usage: UsageColor::Many,
                    send: SendColor::Obligation,
                }],
            }),
        }],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());
    let entry = artifact.manifest.entries.first().unwrap();

    assert_eq!(
        entry.source.layout_key.as_deref(),
        Some("closure-env::closure::y")
    );
    assert!(artifact.wat.contains(
        "item=lift::0000::closure::y layout=closure-env::closure::y specialization=none"
    ));
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
    assert!(artifact.wat.contains(
        ";; symbol math_Vec2_norm source=norm origin=L16Core role=direct-method-target stable-id="
    ));
    assert!(artifact
        .wat
        .contains("owner=none item=math.Vec2.norm layout=none specialization=none"));
    assert!(artifact.wat.contains(";; tailcall math_Vec2_norm args=[v]"));
    assert!(artifact.wat.contains("call $math_Vec2_norm"));
    assert!(!artifact.wat.contains("(func $math_Vec2_norm"));
    assert_eq!(artifact.manifest.entries[0].source_debug_name, "norm");
    assert_eq!(
        artifact.manifest.entries[0].lowering_role,
        "direct-method-target"
    );
    assert_eq!(artifact.manifest.entries[0].stable_id.len(), 16);
}

#[test]
fn backend_lowers_cont1_single_resume_to_i32_runtime_subset() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::Prompt {
                kind: ContinuationKind::Cont1,
            },
            CoreOp::CaptureContinuation {
                binder: "k".to_string(),
                kind: ContinuationKind::Cont1,
                captured: captured_identity_core("resume"),
            },
            CoreOp::TailCall {
                func: "k".to_string(),
                args: vec![CoreValue::I64(7)],
            },
            CoreOp::TailCallResult {
                binder: "w0".to_string(),
            },
            CoreOp::ReturnValue(CoreValue::Var("w0".to_string())),
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact
        .wat
        .contains(";; continuation-runtime subset=prompt-capture-i32"));
    assert!(artifact.wat.contains(";; prompt kind=cont1"));
    assert!(artifact.wat.contains(";; capture-cont binder=k kind=cont1"));
    assert!(artifact
        .wat
        .contains(";; resume-cont binder=k kind=cont1 result=w0"));
    assert!(artifact.wat.contains("i32.const 7"));
    assert!(artifact.wat.contains("local.set $w0"));
    assert!(artifact.wat.contains("local.get $w0"));
    assert!(!artifact.wat.contains("kind=Cont1"));
}

#[test]
fn backend_rejects_cont1_repeated_resume() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::Prompt {
                kind: ContinuationKind::Cont1,
            },
            CoreOp::CaptureContinuation {
                binder: "k".to_string(),
                kind: ContinuationKind::Cont1,
                captured: captured_identity_core("resume"),
            },
            CoreOp::TailCall {
                func: "k".to_string(),
                args: vec![CoreValue::I64(1)],
            },
            CoreOp::TailCallResult {
                binder: "w0".to_string(),
            },
            CoreOp::TailCall {
                func: "k".to_string(),
                args: vec![CoreValue::I64(2)],
            },
            CoreOp::TailCallResult {
                binder: "w1".to_string(),
            },
            CoreOp::ReturnValue(CoreValue::Var("w1".to_string())),
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(
        artifact.diagnostics,
        vec![BackendDiagnostic::Cont1ResumedMoreThanOnce {
            binder: "k".to_string(),
        }]
    );
    assert_eq!(artifact.wat, "");
}

#[test]
fn backend_lowers_contn_repeated_resume_to_i32_runtime_subset() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::Prompt {
                kind: ContinuationKind::ContN,
            },
            CoreOp::CaptureContinuation {
                binder: "retry".to_string(),
                kind: ContinuationKind::ContN,
                captured: captured_identity_core("resume"),
            },
            CoreOp::TailCall {
                func: "retry".to_string(),
                args: vec![CoreValue::I64(1)],
            },
            CoreOp::TailCallResult {
                binder: "w0".to_string(),
            },
            CoreOp::TailCall {
                func: "retry".to_string(),
                args: vec![CoreValue::I64(2)],
            },
            CoreOp::TailCallResult {
                binder: "w1".to_string(),
            },
            CoreOp::OperatorTarget {
                protocol: "op_add".to_string(),
                target: "op_add(w0)".to_string(),
                intrinsic: Some(OperatorIntrinsic::I64Add),
            },
            CoreOp::TailCall {
                func: "op_add(w0)".to_string(),
                args: vec![
                    CoreValue::Var("w0".to_string()),
                    CoreValue::Var("w1".to_string()),
                ],
            },
            CoreOp::TailCallResult {
                binder: "w2".to_string(),
            },
            CoreOp::ReturnValue(CoreValue::Var("w2".to_string())),
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains(";; prompt kind=contn"));
    assert!(artifact
        .wat
        .contains(";; capture-cont binder=retry kind=contn"));
    assert!(artifact
        .wat
        .contains(";; resume-cont binder=retry kind=contn result=w0"));
    assert!(artifact
        .wat
        .contains(";; resume-cont binder=retry kind=contn result=w1"));
    assert!(artifact.wat.contains("call $op_add_u28_w0_u29_"));
    assert!(artifact.wat.contains("local.set $w2"));
    assert!(artifact.wat.contains("local.get $w2"));
    assert!(!artifact.wat.contains("kind=ContN"));
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
    assert!(artifact.wat.contains("(func $main (export \"main\")"));
    assert!(!artifact.wat.contains("i32.const 0"));
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
fn backend_lowers_range_return_to_externref_handle() {
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

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains(
        "(import \"env\" \"std.range_i64_new\" (func $std_range_i64_new (param i32) (param i32) (result externref)))"
    ));
    assert!(artifact
        .wat
        .contains("(func $main (export \"main\") (result externref)"));
    assert!(artifact.wat.contains("call $std_range_i64_new"));
    assert_eq!(
        artifact.return_value,
        Some(CoreValue::Range {
            start: Box::new(CoreValue::I64(1)),
            end: Box::new(CoreValue::I64(3))
        })
    );
}

#[test]
fn backend_lowers_range_field_return_to_i32_value() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::RangeFieldGet {
                field: chiba_level1r::core::RangeField::End,
            },
            CoreOp::ReturnValue(CoreValue::RangeField {
                range: Box::new(CoreValue::Range {
                    start: Box::new(CoreValue::I64(1)),
                    end: Box::new(CoreValue::I64(3)),
                }),
                field: chiba_level1r::core::RangeField::End,
            }),
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains(";; range-field field=end"));
    assert!(artifact.wat.contains("i32.const 3"));
}

#[test]
fn backend_lowers_slice_literal_len_return_to_i32_value() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::SliceFieldGet {
                field: SliceField::Len,
            },
            CoreOp::ReturnValue(CoreValue::AggregateField {
                kind: AggregateKind::Slice,
                value: Box::new(CoreValue::SliceLiteral {
                    items: vec![CoreValue::I64(1), CoreValue::I64(2), CoreValue::I64(3)],
                }),
                field: SliceField::Len,
            }),
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains(";; slice-field field=len"));
    assert!(artifact.wat.contains("i32.const 3"));
}

#[test]
fn backend_lowers_slice_literal_return_to_externref_handle() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::SliceLiteral {
            items: vec![CoreValue::I64(1), CoreValue::I64(2), CoreValue::I64(3)],
        })],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc(&core, &CoreValidation::default());

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains(
        "(import \"env\" \"std.slice_i64_literal_3\" (func $std_slice_i64_literal_3 (param i32) (param i32) (param i32) (result externref)))"
    ));
    assert!(artifact
        .wat
        .contains("(func $main (export \"main\") (result externref)"));
    assert!(artifact.wat.contains("call $std_slice_i64_literal_3"));
    assert_eq!(
        artifact.return_value,
        Some(CoreValue::SliceLiteral {
            items: vec![CoreValue::I64(1), CoreValue::I64(2), CoreValue::I64(3)]
        })
    );
}

#[test]
fn backend_lowers_slice_param_len_to_runtime_import() {
    let core = CoreProgram {
        ops: vec![
            CoreOp::SliceFieldGet {
                field: SliceField::Len,
            },
            CoreOp::ReturnValue(CoreValue::AggregateField {
                kind: AggregateKind::Slice,
                value: Box::new(CoreValue::Var("s".to_string())),
                field: SliceField::Len,
            }),
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc_with_params(&core, &CoreValidation::default(), &["s".to_string()]);

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains(
        "(import \"env\" \"std.slice_i64_len\" (func $std_slice_i64_len (param externref) (result i32)))"
    ));
    assert!(artifact
        .wat
        .contains("(func $main (export \"main\") (param $s externref) (result i32)"));
    assert!(artifact.wat.contains("local.get $s"));
    assert!(artifact.wat.contains("call $std_slice_i64_len"));
}

#[test]
fn backend_lowers_slice_param_index_to_runtime_import() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::AggregateIndex {
            kind: AggregateKind::Slice,
            value: Box::new(CoreValue::Var("s".to_string())),
            index: Box::new(CoreValue::I64(1)),
        })],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let artifact = emit_wasm_gc_with_params(&core, &CoreValidation::default(), &["s".to_string()]);

    assert_eq!(artifact.diagnostics, vec![]);
    assert!(artifact.wat.contains(
        "(import \"env\" \"std.slice_i64_get\" (func $std_slice_i64_get (param externref) (param i32) (result i32)))"
    ));
    assert!(artifact
        .wat
        .contains("(func $main (export \"main\") (param $s externref) (result i32)"));
    assert!(artifact.wat.contains("local.get $s"));
    assert!(artifact.wat.contains("i32.const 1"));
    assert!(artifact.wat.contains("call $std_slice_i64_get"));
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

    let visual = output.render_visual();
    assert!(visual.contains("backend:"));
    assert!(visual.contains("target=wasm-gc"));
    assert!(visual.contains("manifest-entries=1"));
    assert!(visual.contains("symbol lift__0000__closure__x"));
    assert!(!output.visual.backend.contains("BackendArtifact"));
    assert!(!output.visual.backend.contains("WasmGc"));
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
fn backend_target_matrix_emits_scalar_wasm_gc_and_wasm32_nogc() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::I64(42))],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let wasm_gc = emit_wasm_gc(&core, &CoreValidation::default());
    let wasm32_nogc = emit_wasm32_nogc(&core, &CoreValidation::default());

    assert_eq!(wasm_gc.target, BackendTarget::WasmGc);
    assert_eq!(wasm32_nogc.target, BackendTarget::Wasm32NoGc);
    assert_eq!(wasm_gc.diagnostics, vec![]);
    assert_eq!(wasm32_nogc.diagnostics, vec![]);
    assert!(wasm_gc
        .wat
        .contains("(func $main (export \"main\") (result i32)"));
    assert!(wasm32_nogc
        .wat
        .contains("(func $main (export \"main\") (result i32)"));
    assert!(!wasm32_nogc.wat.contains("externref"));
    assert!(!wasm32_nogc.wat.contains("struct.new"));
    assert!(!wasm32_nogc.wat.contains("array.new"));
}

#[test]
fn backend_link_rejects_mixed_target_matrix_artifacts() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::I64(7))],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let wasm_gc = emit_wasm_gc(&core, &CoreValidation::default());
    let wasm32_nogc = emit_wasm32_nogc(&core, &CoreValidation::default());
    let bundle = link_backend_artifacts(vec![wasm_gc, wasm32_nogc]);

    assert_eq!(
        bundle.diagnostics,
        vec![BackendLinkDiagnostic::TargetMismatch {
            artifact_index: 1,
            expected: BackendTarget::WasmGc,
            actual: BackendTarget::Wasm32NoGc,
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

    assert_eq!(output.backend_link.diagnostics, vec![]);
    assert!(output
        .backend_link
        .linked_wat
        .contains("lift__0000__closure__x"));
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
    assert!(visual.contains("diagnostics=0"));
    assert!(visual.contains("digest="));
    assert!(!output.visual.backend_link.contains("BackendLinkedBundle"));
    assert!(!output.visual.backend_cache_key.contains("BackendCacheKey"));
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
        final_symbol: "fd_write".to_string(),
        module: "wasi_snapshot_preview1".to_string(),
        name: "fd_write".to_string(),
        signature_hash: "i32_i32_i32_i32_to_i32".to_string(),
    });
    let mut env = BackendCacheConfig::default();
    env.imports.push(BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: "js_log".to_string(),
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

#[test]
fn backend_cache_key_distinguishes_requested_target_and_layout_policy() {
    let core = CoreProgram {
        ops: vec![CoreOp::ReturnValue(CoreValue::I64(1))],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    let wasm_gc_bundle =
        link_backend_artifacts(vec![emit_wasm_gc(&core, &CoreValidation::default())]);
    let wasm32_bundle =
        link_backend_artifacts(vec![emit_wasm32_nogc(&core, &CoreValidation::default())]);
    let wasm_gc_config = BackendCacheConfig::default();
    let mut wasm32_config = BackendCacheConfig::default();
    wasm32_config.target = BackendTarget::Wasm32NoGc;
    wasm32_config.layout_policy = BackendLayoutPolicy::LinearMemoryNoGc;
    wasm32_config.ownership_runtime = BackendOwnershipRuntime::RcArcHelpers;

    assert_ne!(
        backend_cache_key(&wasm_gc_bundle, &wasm_gc_config),
        backend_cache_key(&wasm32_bundle, &wasm32_config)
    );

    let mut requested_native = wasm32_config.clone();
    requested_native.target = BackendTarget::Native;
    requested_native.layout_policy = BackendLayoutPolicy::NativeAbi;
    assert_ne!(
        backend_cache_key(&wasm32_bundle, &wasm32_config),
        backend_cache_key(&wasm32_bundle, &requested_native)
    );
}

fn captured_identity_core(param: &str) -> CoreCapturedContinuation {
    CoreCapturedContinuation {
        param: param.to_string(),
        ops: vec![CoreOp::ReturnValue(CoreValue::Var(param.to_string()))],
    }
}
