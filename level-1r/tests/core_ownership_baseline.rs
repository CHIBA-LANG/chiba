use chiba_level1r::ast::BinaryOp;
use chiba_level1r::closure::ClosureFacts;
use chiba_level1r::control::{ContinuationKind, ReplaySafety};
use chiba_level1r::core::{
    lower_core_with_facts, LayoutKind, OwnershipDecision, OwnershipFact, OwnershipSubjectKind,
};
use chiba_level1r::lambda_lift::LambdaLiftFacts;
use chiba_level1r::template::{canonical_open_row, dyn_row_contract, ShapeType, TemplateFacts};
use chiba_level1r::usage::UsageFacts;
use chiba_level1r::{compile_expr, Expr};

#[test]
fn core_layout_hash_is_stable_for_canonical_row_shape() {
    let output = compile_expr(&Expr::field(Expr::var("user"), "name"));
    let expected = canonical_open_row(vec![("name", ShapeType::Unknown)]);

    let layout = output
        .core
        .layouts
        .iter()
        .find(|layout| matches!(&layout.kind, LayoutKind::RowShape(shape) if shape == &expected))
        .unwrap();

    assert_ne!(layout.hash, 0);
    assert_eq!(layout.key, "row::open::name:?");
    assert!(!layout.key.contains("RowShape"));
}

#[test]
fn repeated_value_lowers_to_rc_when_not_send() {
    let output = compile_expr(&Expr::binary(BinaryOp::Add, Expr::var("x"), Expr::var("x")));

    assert!(output.core.ownership.contains(&OwnershipFact {
        subject: "var::x".to_string(),
        kind: OwnershipSubjectKind::Value,
        decision: OwnershipDecision::Rc,
    }));
}

#[test]
fn dyn_row_contract_lowers_to_dyn_package_and_payload_decision() {
    let mut template = TemplateFacts::default();
    template.dyn_contracts.push(dyn_row_contract(vec![(
        "name",
        ShapeType::Named("String".to_string()),
    )]));
    let output = compile_expr(&Expr::field(Expr::var("user"), "name"));
    let facts = chiba_level1r::specialize::plan_specialization("render", &template);
    let core = lower_core_with_facts(
        &output.cps,
        &output.control.continuations,
        &ClosureFacts::default(),
        &[],
        &LambdaLiftFacts::default(),
        &facts,
        &UsageFacts::default(),
    );

    assert!(core.layouts.iter().any(|layout| {
        matches!(&layout.kind, LayoutKind::DynRowPackage(contract) if contract == &facts.work_items[0].key.dyn_contracts[0])
    }));
    assert!(core
        .ownership
        .iter()
        .any(|fact| fact.kind == OwnershipSubjectKind::DynRowPackage
            && fact.decision == OwnershipDecision::DynPackage));
    assert!(core.ownership.iter().any(|fact| {
        fact.kind
            == OwnershipSubjectKind::DynRowPayload {
                send: chiba_level1r::typed::SendColor::Obligation,
            }
            && fact.decision == OwnershipDecision::Rc
    }));
}

#[test]
fn contn_core_keeps_continuation_package_layout_fact() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    assert!(output.core.layouts.iter().any(|layout| {
        matches!(
            &layout.kind,
            LayoutKind::ContinuationPackage(env)
                if env.kind == ContinuationKind::ContN
                    && env.binder == "retry"
                    && env.replay_safety == ReplaySafety::Safe
                    && env.clone_on_resume
                    && !env.consumed_state_machine
        )
    }));
    assert!(output.core.ownership.iter().any(|fact| {
        fact.subject == "continuation::retry" && fact.decision == OwnershipDecision::DynPackage
    }));
}

#[test]
fn cont1_core_keeps_consumed_state_machine_layout_fact() {
    let output = compile_expr(&Expr::reset(Expr::shift("k", Expr::i64(0))));

    assert!(output.core.layouts.iter().any(|layout| {
        matches!(
            &layout.kind,
            LayoutKind::Cont1StateMachine(env)
                if env.kind == ContinuationKind::Cont1
                    && env.binder == "k"
                    && env.replay_safety == ReplaySafety::Safe
                    && !env.clone_on_resume
                    && env.consumed_state_machine
        )
    }));
    assert!(output.core.ownership.iter().any(|fact| {
        fact.subject == "continuation::k" && fact.decision == OwnershipDecision::StackValue
    }));
}

#[test]
fn core_ir_facts_stay_target_neutral() {
    let output = compile_expr(&Expr::field(Expr::var("user"), "name"));
    let visual = &output.visual.core;

    assert!(visual.contains("static-row-access field=name"));
    assert!(output
        .core
        .layouts
        .iter()
        .any(|layout| matches!(&layout.kind, LayoutKind::RowShape(_))));
    assert!(!visual.contains("target-specific-term"));
    assert!(!visual.contains("Wasm"));
    assert!(!visual.contains("WAT"));
    assert!(!visual.contains("Binaryen"));
    assert!(!visual.contains("funcref"));
    assert!(!visual.contains("eqref"));
}
