use chiba_level1r::ast::BinaryOp;
use chiba_level1r::core::OwnershipDecision;
use chiba_level1r::typed::UsageColor;
use chiba_level1r::{compile_expr, Expr};

#[test]
fn repeated_value_has_usage_n_and_rust_rc_reference() {
    let output = compile_expr(&Expr::binary(BinaryOp::Add, Expr::var("x"), Expr::var("x")));

    let entry = output
        .usage_audit
        .entries
        .iter()
        .find(|entry| entry.subject == "var::x")
        .unwrap();
    assert_eq!(entry.usage, UsageColor::Many);
    assert_eq!(entry.ownership, Some(OwnershipDecision::Rc));
    assert!(entry.usage_signature.contains("x: N Unknown"));
    assert!(entry.rust_reference_signature.contains("Rc<Unknown>"));
    assert!(entry.aligned);
    assert_eq!(output.usage_audit.diagnostics, vec![]);
}

#[test]
fn single_value_has_usage_one_and_move_only_rust_reference() {
    let output = compile_expr(&Expr::var("x"));

    let entry = output
        .usage_audit
        .entries
        .iter()
        .find(|entry| entry.subject == "var::x")
        .unwrap();
    assert_eq!(entry.usage, UsageColor::One);
    assert_eq!(entry.ownership, Some(OwnershipDecision::StackValue));
    assert!(entry.usage_signature.contains("x: 1 Unknown"));
    assert_eq!(entry.rust_reference_signature, "let x: Unknown");
    assert!(entry.aligned);
    assert_eq!(output.usage_audit.diagnostics, vec![]);
}

#[test]
fn contn_usage_n_requires_rc_continuation_frame_in_audit() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    let entry = output
        .usage_audit
        .entries
        .iter()
        .find(|entry| entry.subject == "continuation::retry")
        .unwrap();
    assert_eq!(entry.usage, UsageColor::Many);
    assert_eq!(entry.ownership, Some(OwnershipDecision::DynPackage));
    assert!(entry.usage_signature.contains("retry: N ContN"));
    assert_eq!(entry.rust_reference_signature, "let retry: Rc<ContNFrame>");
    assert!(entry.aligned);
    assert_eq!(output.usage_audit.diagnostics, vec![]);
}

#[test]
fn cont1_usage_one_stays_non_rc_in_audit() {
    let output = compile_expr(&Expr::reset(Expr::shift("k", Expr::i64(0))));

    let entry = output
        .usage_audit
        .entries
        .iter()
        .find(|entry| entry.subject == "continuation::k")
        .unwrap();
    assert_eq!(entry.usage, UsageColor::One);
    assert_eq!(entry.ownership, Some(OwnershipDecision::StackValue));
    assert!(entry.usage_signature.contains("k: 1 Cont1"));
    assert_eq!(entry.rust_reference_signature, "let k: Cont1Frame");
    assert!(entry.aligned);
    assert_eq!(output.usage_audit.diagnostics, vec![]);
}

#[test]
fn visual_report_contains_four_layer_usage_audit() {
    let output = compile_expr(&Expr::binary(BinaryOp::Add, Expr::var("x"), Expr::var("x")));
    let visual = output.render_visual();

    assert!(visual.contains("usage-audit:"));
    assert!(visual.contains("source_signature"));
    assert!(visual.contains("typed_signature"));
    assert!(visual.contains("usage_signature"));
    assert!(visual.contains("rust_reference_signature"));
}
