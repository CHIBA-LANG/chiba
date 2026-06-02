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
            "L1Alpha",
            "L2Resolve",
            "L3Template",
            "L4Specialize",
            "L5Monomorphize",
            "L6Typed",
            "L7PatternElab",
            "L8AnswerControl",
            "L9Usage",
            "L10OnePassCps",
            "L11CpsUsage",
            "L12ContSimplify",
            "L13Closure",
            "L14LambdaLift",
            "L15Core",
            "L16ClosureCoreUsage",
            "L17ClosureSimplify",
            "L18UsageAudit",
            "L19CoreValidate",
            "L20BackendEmit",
            "L21BackendLink",
            "L22BackendCacheKey"
        ]
    );

    let visual = output.render_visual();
    assert!(visual.contains("resolve:"));
    assert!(visual.contains("template:"));
    assert!(visual.contains("specialize:"));
    assert!(visual.contains("monomorphize:"));
    assert!(visual.contains("pattern:"));
    assert!(visual.contains("control:"));
    assert!(visual.contains("cps-usage:"));
    assert!(visual.contains("continuation-simplification:"));
    assert!(visual.contains("closure:"));
    assert!(visual.contains("lambda-lift:"));
    assert!(visual.contains("core:"));
    assert!(visual.contains("closure-core-usage:"));
    assert!(visual.contains("closure-simplification:"));
    assert!(visual.contains("usage-audit:"));
    assert!(visual.contains("core-validation:"));
    assert!(visual.contains("backend:"));
    assert!(visual.contains("backend-link:"));
    assert!(visual.contains("backend-cache-key:"));
    assert!(visual.contains("nanopass:"));
    assert!(visual.contains("L1Alpha: SourceExpr -> AlphaFacts"));
    assert!(visual.contains("L3Template: AlphaExpr+ResolveFacts -> TemplateFacts"));
    assert!(visual.contains("L4Specialize: TemplateFacts -> SpecializationFacts"));
    assert!(visual.contains("L5Monomorphize: SpecializationFacts -> MonomorphizationPlan"));
    assert!(visual.contains("L7PatternElab: TypedExpr -> PatternFacts"));
    assert!(visual.contains("L10OnePassCps: TypedExpr -> CpsProgram"));
    assert!(visual.contains("L11CpsUsage: CpsProgram -> CpsUsageFacts"));
    assert!(visual.contains(
        "L12ContSimplify: CpsUsageFacts -> ContinuationSimplificationFacts"
    ));
    assert!(visual.contains("L14LambdaLift: ClosureFacts -> LambdaLiftFacts"));
    assert!(visual.contains("L16ClosureCoreUsage: CoreProgram -> ClosureCoreUsageFacts"));
    assert!(visual.contains(
        "L17ClosureSimplify: ClosureCoreUsageFacts -> ClosureSimplificationFacts"
    ));
    assert!(visual.contains(
        "L18UsageAudit: TypedExpr+UsageFacts+CoreProgram -> UsageAuditReport"
    ));
    assert!(visual.contains("L19CoreValidate: CoreProgram -> CoreValidation"));
    assert!(visual.contains(
        "L20BackendEmit: CoreProgram+CoreValidation -> BackendArtifact"
    ));
    assert!(visual.contains(
        "L21BackendLink: BackendArtifact -> BackendLinkedBundle"
    ));
    assert!(visual.contains(
        "L22BackendCacheKey: BackendLinkedBundle -> BackendCacheKey"
    ));
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
