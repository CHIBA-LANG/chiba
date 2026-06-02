use chiba_level1r::control::ContinuationKind;
use chiba_level1r::core::{
    validate_core, CoreDiagnostic, CoreOp, CoreProgram, LayoutFact, LayoutKind, OwnershipDecision,
    OwnershipFact,
};
use chiba_level1r::template::{canonical_open_row, ShapeType};
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
    output
        .core
        .ops
        .push(CoreOp::ReturnAtom("funcref-leak".to_string()));

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
        key: "continuation::ContN::retry".to_string(),
        hash: 1,
        kind: LayoutKind::ContinuationPackage(ContinuationKind::ContN),
    };
    let program = CoreProgram {
        ops: vec![],
        layouts: vec![layout.clone(), layout],
        ownership: vec![],
    };

    let validation = validate_core(&program);

    assert!(validation.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            CoreDiagnostic::LayoutHashMismatch {
                key,
                actual: 1,
                ..
            } if key == "continuation::ContN::retry"
        )
    }));
    assert!(
        validation
            .diagnostics
            .contains(&CoreDiagnostic::DuplicateLayoutKey {
                key: "continuation::ContN::retry".to_string()
            })
    );
}

#[test]
fn validator_rejects_missing_contn_package_layout() {
    let program = CoreProgram {
        ops: vec![CoreOp::CaptureContinuation {
            binder: "retry".to_string(),
            kind: ContinuationKind::ContN,
        }],
        layouts: vec![],
        ownership: vec![],
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
fn validator_rejects_cont1_package_and_send_rc_contradiction() {
    let program = CoreProgram {
        ops: vec![],
        layouts: vec![LayoutFact {
            key: "continuation::Cont1::k".to_string(),
            hash: 0,
            kind: LayoutKind::ContinuationPackage(ContinuationKind::Cont1),
        }],
        ownership: vec![
            OwnershipFact {
                subject: "send::shared-value".to_string(),
                decision: OwnershipDecision::Rc,
            },
            OwnershipFact {
                subject: "dyn::user".to_string(),
                decision: OwnershipDecision::Rc,
            },
        ],
    };

    let validation = validate_core(&program);

    assert!(
        validation
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic, CoreDiagnostic::Cont1HasPackageLayout { .. }))
    );
    assert!(
        validation
            .diagnostics
            .contains(&CoreDiagnostic::SharedSendSubjectUsesRc {
                subject: "send::shared-value".to_string()
            })
    );
    assert!(
        validation
            .diagnostics
            .contains(&CoreDiagnostic::DynPayloadMustUseDynPackage {
                subject: "dyn::user".to_string()
            })
    );
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
            kind: LayoutKind::RowShape(canonical_open_row(vec![(
                "name",
                ShapeType::Unknown,
            )])),
        }],
        ownership: vec![],
    };

    assert!(
        validate_core(&wrong_kind)
            .diagnostics
            .contains(&CoreDiagnostic::DynRowLayoutKindMismatch {
                layout: "row::user".to_string()
            })
    );
}
