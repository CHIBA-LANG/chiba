use chiba_level1r::control::{ContinuationKind, ReplaySafety};
use chiba_level1r::core::{
    validate_core, CallableStorageFact, CallableStorageKind, ClosureEnvField, ClosureEnvLayout,
    CompilerIntrinsic, ContinuationEnvLayout, CoreCapturedContinuation, CoreDiagnostic, CoreOp,
    CoreProgram, CoreValue, LayoutFact, LayoutKind, OwnershipDecision, OwnershipFact,
    OwnershipSubjectKind, TargetSpecificCoreTerm, TupleLayout,
};
use chiba_level1r::template::{canonical_open_row, ShapeType};
use chiba_level1r::typed::{SendColor, UsageColor};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn lowered_core_program_validates_when_facts_are_consistent() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));
    let validation = validate_core(&output.core);

    assert_eq!(validation.diagnostics, vec![]);
    assert!(validation.is_ok());
    assert_eq!(output.core_validation.diagnostics, vec![]);
}

#[test]
fn validator_rejects_target_specific_core_terms() {
    let mut output = compile_expr(&Expr::i64(0));
    output.core.ops.push(CoreOp::TargetSpecificTerm {
        term: TargetSpecificCoreTerm::Funcref,
    });

    let validation = validate_core(&output.core);

    assert!(validation
        .diagnostics
        .contains(&CoreDiagnostic::TargetSpecificTerm {
            term: "funcref".to_string()
        }));
}

#[test]
fn validator_rejects_invalid_layout_hash_and_duplicate_keys() {
    let layout = LayoutFact {
        key: "continuation::contn::retry".to_string(),
        hash: 1,
        kind: LayoutKind::ContinuationPackage(continuation_env("retry", ContinuationKind::ContN)),
    };
    let program = CoreProgram {
        ops: vec![],
        layouts: vec![layout.clone(), layout],
        ownership: vec![],
        callable_storage: vec![],
    };

    let validation = validate_core(&program);

    assert!(validation.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            CoreDiagnostic::LayoutHashMismatch {
                key,
                actual: 1,
                ..
            } if key == "continuation::contn::retry"
        )
    }));
    assert!(validation
        .diagnostics
        .contains(&CoreDiagnostic::DuplicateLayoutKey {
            key: "continuation::contn::retry".to_string()
        }));
}

#[test]
fn validator_rejects_missing_contn_package_layout() {
    let program = CoreProgram {
        ops: vec![CoreOp::CaptureContinuation {
            binder: "retry".to_string(),
            kind: ContinuationKind::ContN,
            captured: captured_identity_core("resume"),
        }],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let validation = validate_core(&program);

    assert_eq!(
        validation.diagnostics,
        vec![CoreDiagnostic::MissingContNPackage {
            binder: "retry".to_string()
        }]
    );
}

#[test]
fn validator_rejects_unsafe_contn_replay_capture() {
    let program = CoreProgram {
        ops: vec![CoreOp::CaptureContinuation {
            binder: "retry".to_string(),
            kind: ContinuationKind::ContN,
            captured: captured_identity_core("resume"),
        }],
        layouts: vec![LayoutFact {
            key: "continuation::contn::retry".to_string(),
            hash: 0,
            kind: LayoutKind::ContinuationPackage(continuation_env_with_replay_safety(
                "retry",
                ContinuationKind::ContN,
                ReplaySafety::Unsafe,
            )),
        }],
        ownership: vec![],
        callable_storage: vec![],
    };

    assert!(validate_core(&program).diagnostics.contains(
        &CoreDiagnostic::UnsafeContNReplayCapture {
            binder: "retry".to_string()
        }
    ));
}

fn captured_identity_core(param: &str) -> CoreCapturedContinuation {
    CoreCapturedContinuation {
        param: param.to_string(),
        ops: vec![CoreOp::ReturnValue(CoreValue::Var(param.to_string()))],
    }
}

#[test]
fn validator_rejects_compiler_intrinsic_with_wrong_owner_namespace() {
    let program = CoreProgram {
        ops: vec![CoreOp::CompilerIntrinsicUse {
            intrinsic: CompilerIntrinsic::TupleToAdt,
            owner_namespace: "std".to_string(),
            subject: "Option.Some".to_string(),
        }],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    assert_eq!(
        validate_core(&program).diagnostics,
        vec![CoreDiagnostic::CompilerIntrinsicOwnerMismatch {
            intrinsic: CompilerIntrinsic::TupleToAdt,
            owner_namespace: "std".to_string(),
            expected_owner_namespace: "compiler.intrinsic".to_string(),
        }]
    );
}

#[test]
fn validator_rejects_cont1_package_and_send_rc_contradiction() {
    let program = CoreProgram {
        ops: vec![],
        layouts: vec![LayoutFact {
            key: "continuation::cont1::k".to_string(),
            hash: 0,
            kind: LayoutKind::ContinuationPackage(continuation_env("k", ContinuationKind::Cont1)),
        }],
        ownership: vec![
            OwnershipFact {
                subject: "send::shared-value".to_string(),
                kind: OwnershipSubjectKind::SharedSendValue,
                decision: OwnershipDecision::Rc,
            },
            OwnershipFact {
                subject: "dyn::user".to_string(),
                kind: OwnershipSubjectKind::DynRowPackage,
                decision: OwnershipDecision::Rc,
            },
        ],
        callable_storage: vec![],
    };

    let validation = validate_core(&program);

    assert!(validation
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic, CoreDiagnostic::Cont1HasPackageLayout { .. })));
    assert!(validation
        .diagnostics
        .contains(&CoreDiagnostic::SharedSendSubjectUsesRc {
            subject: "send::shared-value".to_string()
        }));
    assert!(validation
        .diagnostics
        .contains(&CoreDiagnostic::DynPayloadMustUseDynPackage {
            subject: "dyn::user".to_string()
        }));
}

fn continuation_env(binder: &str, kind: ContinuationKind) -> ContinuationEnvLayout {
    continuation_env_with_replay_safety(binder, kind, ReplaySafety::Safe)
}

fn continuation_env_with_replay_safety(
    binder: &str,
    kind: ContinuationKind,
    replay_safety: ReplaySafety,
) -> ContinuationEnvLayout {
    ContinuationEnvLayout {
        binder: binder.to_string(),
        kind,
        fields: Vec::<ClosureEnvField>::new(),
        replay_safety,
        clone_on_resume: kind == ContinuationKind::ContN,
        consumed_state_machine: kind == ContinuationKind::Cont1,
    }
}

#[test]
fn validator_rejects_dangling_direct_tailcall_target() {
    let program = CoreProgram {
        ops: vec![CoreOp::TailCall {
            func: "module::missing".to_string(),
            args: vec![CoreValue::Var("x".to_string())],
        }],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    assert_eq!(
        validate_core(&program).diagnostics,
        vec![CoreDiagnostic::DanglingTailCallTarget {
            target: "module::missing".to_string()
        }]
    );
}

#[test]
fn validator_accepts_known_direct_and_dynamic_callable_tailcalls() {
    let known = CoreProgram {
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
    assert_eq!(validate_core(&known).diagnostics, vec![]);

    let dynamic = CoreProgram {
        ops: vec![
            CoreOp::DynamicCallableTarget {
                target: "f".to_string(),
            },
            CoreOp::TailCall {
                func: "f".to_string(),
                args: vec![CoreValue::Var("x".to_string())],
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    assert_eq!(validate_core(&dynamic).diagnostics, vec![]);

    let dynamic_method = CoreProgram {
        ops: vec![
            CoreOp::DynamicCallableTarget {
                target: "receiver.put".to_string(),
            },
            CoreOp::TailCall {
                func: "receiver.put".to_string(),
                args: vec![CoreValue::I64(1), CoreValue::I64(2)],
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    assert_eq!(validate_core(&dynamic_method).diagnostics, vec![]);

    let dynamic_operator = CoreProgram {
        ops: vec![
            CoreOp::DynamicCallableTarget {
                target: "operator::Add(w0)".to_string(),
            },
            CoreOp::TailCall {
                func: "operator::Add(w0)".to_string(),
                args: vec![CoreValue::I64(1)],
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    assert_eq!(validate_core(&dynamic_operator).diagnostics, vec![]);

    let utf8_dynamic = CoreProgram {
        ops: vec![
            CoreOp::DynamicCallableTarget {
                target: "模块.函数α".to_string(),
            },
            CoreOp::TailCall {
                func: "模块.函数α".to_string(),
                args: vec![CoreValue::Var("标量名".to_string())],
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    assert_eq!(validate_core(&utf8_dynamic).diagnostics, vec![]);

    let emoji_dynamic = CoreProgram {
        ops: vec![
            CoreOp::DynamicCallableTarget {
                target: "结果.🚀Ok".to_string(),
            },
            CoreOp::TailCall {
                func: "结果.🚀Ok".to_string(),
                args: vec![CoreValue::I64(1)],
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };
    assert_eq!(validate_core(&emoji_dynamic).diagnostics, vec![]);
}

#[test]
fn validator_rejects_dyn_adapter_missing_or_wrong_layout_ref() {
    let missing = CoreProgram {
        ops: vec![CoreOp::DynRowAdapterAccess {
            subject: "user".to_string(),
            layout: "dyn-row::missing".to_string(),
        }],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    assert_eq!(
        validate_core(&missing).diagnostics,
        vec![CoreDiagnostic::MissingDynRowLayout {
            layout: "dyn-row::missing".to_string()
        }]
    );

    let wrong_kind = CoreProgram {
        ops: vec![CoreOp::DynRowAdapterAccess {
            subject: "user".to_string(),
            layout: "row::user".to_string(),
        }],
        layouts: vec![LayoutFact {
            key: "row::user".to_string(),
            hash: 0,
            kind: LayoutKind::RowShape(canonical_open_row(vec![("name", ShapeType::Unknown)])),
        }],
        ownership: vec![],
        callable_storage: vec![],
    };

    assert!(validate_core(&wrong_kind).diagnostics.contains(
        &CoreDiagnostic::DynRowLayoutKindMismatch {
            layout: "row::user".to_string()
        }
    ));
}

#[test]
fn validator_rejects_static_row_access_missing_or_wrong_layout_ref() {
    let missing = CoreProgram {
        ops: vec![CoreOp::StaticRowAccess {
            field: "name".to_string(),
            layout: "row::missing".to_string(),
        }],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    assert_eq!(
        validate_core(&missing).diagnostics,
        vec![CoreDiagnostic::MissingStaticRowLayout {
            layout: "row::missing".to_string()
        }]
    );

    let wrong_kind = CoreProgram {
        ops: vec![CoreOp::StaticRowAccess {
            field: "name".to_string(),
            layout: "dyn-row::user".to_string(),
        }],
        layouts: vec![LayoutFact {
            key: "dyn-row::user".to_string(),
            hash: 0,
            kind: LayoutKind::DynRowPackage(chiba_level1r::template::dyn_row_contract(vec![(
                "name",
                ShapeType::Unknown,
            )])),
        }],
        ownership: vec![],
        callable_storage: vec![],
    };

    assert!(validate_core(&wrong_kind).diagnostics.contains(
        &CoreDiagnostic::StaticRowLayoutKindMismatch {
            layout: "dyn-row::user".to_string()
        }
    ));
}

#[test]
fn validator_rejects_tuple_field_access_missing_layout_or_field() {
    let missing = CoreProgram {
        ops: vec![CoreOp::TupleFieldGet {
            nominal: "Tuple2_I64_Bool".to_string(),
            layout: "tuple::Tuple2_I64_Bool".to_string(),
            field: "_2".to_string(),
            field_index: 1,
        }],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    assert_eq!(
        validate_core(&missing).diagnostics,
        vec![CoreDiagnostic::MissingTupleLayout {
            layout: "tuple::Tuple2_I64_Bool".to_string()
        }]
    );

    let missing_field = CoreProgram {
        ops: vec![CoreOp::TupleFieldGet {
            nominal: "Tuple1_I64".to_string(),
            layout: "tuple::Tuple1_I64".to_string(),
            field: "_2".to_string(),
            field_index: 1,
        }],
        layouts: vec![LayoutFact {
            key: "tuple::Tuple1_I64".to_string(),
            hash: 0,
            kind: LayoutKind::TupleStruct(TupleLayout {
                nominal: "Tuple1_I64".to_string(),
                fields: vec!["_1".to_string()],
            }),
        }],
        ownership: vec![],
        callable_storage: vec![],
    };

    assert!(validate_core(&missing_field).diagnostics.contains(
        &CoreDiagnostic::TupleFieldMissing {
            layout: "tuple::Tuple1_I64".to_string(),
            field: "_2".to_string(),
        }
    ));
}

#[test]
fn validator_rejects_env_closure_missing_or_empty_env_layout() {
    let missing = CoreProgram {
        ops: vec![],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![CallableStorageFact {
            subject: "closure::f".to_string(),
            kind: CallableStorageKind::EnvClosure,
            usage: UsageColor::Many,
            send: SendColor::Obligation,
        }],
    };

    assert_eq!(
        validate_core(&missing).diagnostics,
        vec![CoreDiagnostic::EnvClosureMissingLayout {
            subject: "closure::f".to_string()
        }]
    );

    let empty_layout = CoreProgram {
        ops: vec![],
        layouts: vec![LayoutFact {
            key: "closure-env::closure::f".to_string(),
            hash: 0,
            kind: LayoutKind::ClosureEnv(ClosureEnvLayout {
                closure: "closure::f".to_string(),
                fields: vec![],
            }),
        }],
        ownership: vec![],
        callable_storage: vec![CallableStorageFact {
            subject: "closure::f".to_string(),
            kind: CallableStorageKind::EnvClosure,
            usage: UsageColor::Many,
            send: SendColor::Obligation,
        }],
    };

    assert!(validate_core(&empty_layout).diagnostics.contains(
        &CoreDiagnostic::ClosureEnvLayoutHasNoFields {
            layout: "closure-env::closure::f".to_string()
        }
    ));
}

#[test]
fn lifted_functions_enter_core_with_target_neutral_symbols() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));

    assert!(output.core.ops.iter().any(|op| {
        matches!(
            op,
            CoreOp::LiftedFunction {
                source,
                symbol,
                env_params,
                direct: true,
                param: Some(param),
                body,
            } if source == "closure::x"
                && symbol == "lift::0000::closure__x"
                && env_params.is_empty()
                && param == "x"
                && !body.is_empty()
        )
    }));
    assert!(output.core.ops.iter().any(|op| {
        matches!(
            op,
            CoreOp::LiftedFunction {
                source,
                symbol,
                env_params,
                direct: false,
                param: Some(param),
                body,
            } if source == "closure::y"
                && symbol == "lift::0001::closure__y"
                && env_params == &vec!["x".to_string()]
                && param == "y"
                && body.iter().any(|op| matches!(
                    op,
                    CoreOp::TailCall {
                        func,
                        args,
                    } if func == "x" && args == &vec![CoreValue::Var("y".to_string())]
                ))
                && !body.is_empty()
        )
    }));
    assert_eq!(output.core_validation.diagnostics, vec![]);
}

#[test]
fn validator_rejects_duplicate_lifted_symbol_and_missing_env_layout() {
    let program = CoreProgram {
        ops: vec![
            CoreOp::LiftedFunction {
                source: "closure::y".to_string(),
                symbol: "lift::0001::closure__y".to_string(),
                env_params: vec!["x".to_string()],
                direct: false,
                param: None,
                body: vec![],
            },
            CoreOp::LiftedFunction {
                source: "closure::z".to_string(),
                symbol: "lift::0001::closure__y".to_string(),
                env_params: vec![],
                direct: true,
                param: None,
                body: vec![],
            },
        ],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![],
    };

    let diagnostics = validate_core(&program).diagnostics;

    assert!(
        diagnostics.contains(&CoreDiagnostic::LiftedFunctionMissingClosureEnv {
            source: "closure::y".to_string(),
            expected_layout: "closure-env::closure::y".to_string(),
        })
    );
    assert!(
        diagnostics.contains(&CoreDiagnostic::DuplicateLiftedFunctionSymbol {
            symbol: "lift::0001::closure__y".to_string(),
        })
    );
}
