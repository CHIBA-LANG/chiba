use chiba_level1r::control::ContinuationKind;
use chiba_level1r::control::ControlError;
use chiba_level1r::typed::UsageColor;
use chiba_level1r::{compile_expr, Expr};

#[test]
fn reset_shift_captures_cont1_from_delimiter() {
    let output = compile_expr(&Expr::reset(Expr::shift("k", Expr::i64(0))));

    assert_eq!(output.control.errors, vec![]);
    assert_eq!(output.control.continuations.len(), 1);
    let fact = &output.control.continuations[0];
    assert_eq!(fact.binder, "k");
    assert_eq!(fact.kind, ContinuationKind::Cont1);
    assert_eq!(fact.usage, UsageColor::One);
    assert!(output.cps.to_string().contains("shift@cont1 k"));
}

#[test]
fn resetn_shift_captures_contn_from_delimiter() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    assert_eq!(output.control.errors, vec![]);
    assert_eq!(output.control.continuations.len(), 1);
    let fact = &output.control.continuations[0];
    assert_eq!(fact.binder, "retry");
    assert_eq!(fact.kind, ContinuationKind::ContN);
    assert_eq!(fact.usage, UsageColor::Many);
    assert!(output.cps.to_string().contains("shift@contN retry"));
}

#[test]
fn nanopass_report_keeps_ordered_debuggable_passes() {
    let output = compile_expr(&Expr::call(Expr::var("f"), Expr::var("x")));

    assert_eq!(
        output.passes.names(),
        vec![
            "L2Typed",
            "L3AnswerControl",
            "L4Usage",
            "L5OnePassCps",
            "L6Closure",
            "L7Core"
        ]
    );

    let visual = output.render_visual();
    assert!(visual.contains("control:"));
    assert!(visual.contains("closure:"));
    assert!(visual.contains("core:"));
    assert!(visual.contains("nanopass:"));
    assert!(visual.contains("L5OnePassCps: TypedExpr -> CpsProgram"));
}

#[test]
fn shift_without_reset_is_reported_before_lowering_can_claim_success() {
    let output = compile_expr(&Expr::shift("k", Expr::i64(0)));

    assert_eq!(
        output.control.errors,
        vec![ControlError::ShiftOutsideReset {
            binder: "k".to_string()
        }]
    );
}
