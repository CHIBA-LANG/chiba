use chiba_level1r::ast::BinaryOp;
use chiba_level1r::control::ContinuationKind;
use chiba_level1r::core::{lower_core_with_facts, LayoutKind, OwnershipDecision, OwnershipFact};
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
    assert_eq!(layout.key, format!("row::{expected:?}"));
}

#[test]
fn repeated_value_lowers_to_rc_when_not_send() {
    let output = compile_expr(&Expr::binary(
        BinaryOp::Add,
        Expr::var("x"),
        Expr::var("x"),
    ));

    assert!(output.core.ownership.contains(&OwnershipFact {
        subject: "var::x".to_string(),
        decision: OwnershipDecision::Rc,
    }));
}

#[test]
fn dyn_row_contract_lowers_to_dyn_package_and_payload_decision() {
    let mut template = TemplateFacts::default();
    template
        .dyn_contracts
        .push(dyn_row_contract(vec![("name", ShapeType::Named("String".to_string()))]));
    let output = compile_expr(&Expr::field(Expr::var("user"), "name"));
    let facts = chiba_level1r::specialize::plan_specialization("render", &template);
    let core = lower_core_with_facts(
        &output.cps,
        &output.control.continuations,
        &facts,
        &UsageFacts::default(),
    );

    assert!(core.layouts.iter().any(|layout| {
        matches!(&layout.kind, LayoutKind::DynRowPackage(contract) if contract == &facts.work_items[0].key.dyn_contracts[0])
    }));
    assert!(core
        .ownership
        .iter()
        .any(|fact| fact.decision == OwnershipDecision::DynPackage));
    assert!(core.ownership.iter().any(|fact| {
        fact.subject.starts_with("dyn-payload::") && fact.decision == OwnershipDecision::Rc
    }));
}

#[test]
fn contn_core_keeps_continuation_package_layout_fact() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    assert!(output.core.layouts.iter().any(|layout| {
        matches!(
            &layout.kind,
            LayoutKind::ContinuationPackage(ContinuationKind::ContN)
        )
    }));
    assert!(output.core.ownership.iter().any(|fact| {
        fact.subject == "continuation::retry" && fact.decision == OwnershipDecision::DynPackage
    }));
}

#[test]
fn core_ir_facts_stay_target_neutral() {
    let output = compile_expr(&Expr::field(Expr::var("user"), "name"));
    let rendered = format!("{:#?}", output.core);

    assert!(!rendered.contains("Wasm"));
    assert!(!rendered.contains("WAT"));
    assert!(!rendered.contains("Binaryen"));
    assert!(!rendered.contains("funcref"));
    assert!(!rendered.contains("eqref"));
}
