use chiba_level1r::ast::BinaryOp;
use chiba_level1r::control::ControlError;
use chiba_level1r::control::{ContinuationKind, ReplaySafety};
use chiba_level1r::typed::{Type, UsageColor};
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
    assert_eq!(fact.replay_safety, ReplaySafety::Safe);
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
    assert_eq!(fact.replay_safety, ReplaySafety::Safe);
    assert!(output.cps.to_string().contains("shift@contN retry"));
}

#[test]
fn contn_replay_safety_uses_captured_context_not_shift_body() {
    let output = compile_expr(&Expr::resetn(Expr::binary(
        BinaryOp::Add,
        Expr::i64(10),
        Expr::shift("retry", Expr::call(Expr::var("retry"), Expr::i64(1))),
    )));

    assert_eq!(output.control.errors, vec![]);
    assert_eq!(output.control.continuations.len(), 1);
    assert_eq!(
        output.control.continuations[0].replay_safety,
        ReplaySafety::Safe
    );
}

#[test]
fn contn_capture_under_unknown_call_is_replay_unsafe() {
    let output = compile_expr(&Expr::resetn(Expr::call(
        Expr::var("f"),
        Expr::shift("retry", Expr::call(Expr::var("retry"), Expr::i64(1))),
    )));

    assert_eq!(output.control.continuations.len(), 1);
    assert_eq!(
        output.control.continuations[0].replay_safety,
        ReplaySafety::Unsafe
    );
    assert_eq!(
        output.control.errors,
        vec![ControlError::UnsafeMultiResumeCapture {
            binder: "retry".to_string()
        }]
    );
}

#[test]
fn nested_reset_uses_nearest_delimiter_for_continuation_kind() {
    let inner_contn = compile_expr(&Expr::reset(Expr::resetn(Expr::shift(
        "retry",
        Expr::i64(0),
    ))));
    let inner_cont1 = compile_expr(&Expr::resetn(Expr::reset(Expr::shift("k", Expr::i64(0)))));

    assert_eq!(inner_contn.control.errors, vec![]);
    assert_eq!(inner_contn.control.continuations.len(), 1);
    assert_eq!(inner_contn.control.continuations[0].binder, "retry");
    assert_eq!(
        inner_contn.control.continuations[0].kind,
        ContinuationKind::ContN
    );
    assert_eq!(inner_cont1.control.errors, vec![]);
    assert_eq!(inner_cont1.control.continuations.len(), 1);
    assert_eq!(inner_cont1.control.continuations[0].binder, "k");
    assert_eq!(
        inner_cont1.control.continuations[0].kind,
        ContinuationKind::Cont1
    );
}

#[test]
fn shift_resume_input_type_is_collected_from_typed_resume_calls() {
    let single = compile_expr(&Expr::reset(Expr::shift(
        "k",
        Expr::call(Expr::var("k"), Expr::i64(7)),
    )));
    let repeated_same = compile_expr(&Expr::resetn(Expr::shift(
        "retry",
        Expr::binary(
            BinaryOp::Add,
            Expr::call(Expr::var("retry"), Expr::i64(1)),
            Expr::call(Expr::var("retry"), Expr::i64(2)),
        ),
    )));
    let repeated_conflict = compile_expr(&Expr::resetn(Expr::shift(
        "retry",
        Expr::binary(
            BinaryOp::Add,
            Expr::call(Expr::var("retry"), Expr::i64(1)),
            Expr::call(Expr::var("retry"), Expr::bool(true)),
        ),
    )));

    assert_eq!(single.control.continuations[0].input, Type::I64);
    assert_eq!(repeated_same.control.continuations[0].input, Type::I64);
    assert_eq!(
        repeated_conflict.control.continuations[0].input,
        Type::Unknown
    );
}

#[test]
fn reset_answer_type_uses_captured_context_after_shift_input_refinement() {
    let output = compile_expr(&Expr::reset(Expr::binary(
        BinaryOp::Add,
        Expr::i64(10),
        Expr::shift("k", Expr::call(Expr::var("k"), Expr::i64(7))),
    )));

    assert_eq!(output.typed.ty, Type::I64);
    assert_eq!(output.control.continuations[0].input, Type::I64);
    assert_eq!(output.control.continuations[0].answer, Type::I64);
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
            "L6TemplateAudit",
            "L7TypedSignature",
            "L7Typed",
            "L8PatternElab",
            "L9AnswerControl",
            "L10Usage",
            "L11OnePassCps",
            "L12CpsUsage",
            "L13ContSimplify",
            "L14Closure",
            "L15LambdaLift",
            "L16Core",
            "L17ClosureCoreUsage",
            "L18ClosureSimplify",
            "L19UsageAudit",
            "L20StdAudit",
            "L21CoreValidate",
            "L22BackendEmit",
            "L23BackendLink",
            "L24BackendCacheKey"
        ]
    );

    let visual = output.render_visual();
    assert!(visual.contains("resolve:"));
    assert!(visual.contains("template:"));
    assert!(visual.contains("specialize:"));
    assert!(visual.contains("monomorphize:"));
    assert!(visual.contains("template-audit:"));
    assert!(visual.contains("typed-signature:"));
    assert!(visual.contains("typed:"));
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
    assert!(visual.contains("std-audit:"));
    assert!(visual.contains("core-validation:"));
    assert!(visual.contains("backend:"));
    assert!(visual.contains("backend-link:"));
    assert!(visual.contains("backend-cache-key:"));
    assert!(visual.contains("nanopass:"));
    assert!(visual.contains("L1Alpha: SourceExpr -> AlphaFacts"));
    assert!(visual.contains("L3Template: AlphaExpr+ResolveFacts -> TemplateFacts"));
    assert!(visual.contains("L4Specialize: TemplateFacts -> SpecializationFacts"));
    assert!(visual.contains("L5Monomorphize: SpecializationFacts -> MonomorphizationPlan"));
    assert!(visual.contains(
        "L6TemplateAudit: TemplateFacts+SpecializationFacts+MonomorphizationPlan -> TemplateAuditReport"
    ));
    assert!(visual.contains("L8PatternElab: TypedExpr+ParamPatterns -> PatternFacts"));
    assert!(visual.contains("L11OnePassCps: TypedExpr -> CpsProgram"));
    assert!(visual.contains("L12CpsUsage: CpsProgram -> CpsUsageFacts"));
    assert!(visual.contains("L13ContSimplify: CpsUsageFacts -> ContinuationSimplificationFacts"));
    assert!(visual.contains("L15LambdaLift: ClosureFacts -> LambdaLiftFacts"));
    assert!(visual.contains("L17ClosureCoreUsage: CoreProgram -> ClosureCoreUsageFacts"));
    assert!(
        visual.contains("L18ClosureSimplify: ClosureCoreUsageFacts -> ClosureSimplificationFacts")
    );
    assert!(visual.contains("L19UsageAudit: TypedExpr+UsageFacts+CoreProgram -> UsageAuditReport"));
    assert!(visual.contains("L20StdAudit: CompilerCrate -> StdAuditReport"));
    assert!(visual.contains("L21CoreValidate: CoreProgram -> CoreValidation"));
    assert!(visual.contains("L22BackendEmit: CoreProgram+CoreValidation -> BackendArtifact"));
    assert!(visual.contains("L23BackendLink: BackendArtifact -> BackendLinkedBundle"));
    assert!(visual.contains("L24BackendCacheKey: BackendLinkedBundle -> BackendCacheKey"));
}

#[test]
fn control_visual_dump_shows_replay_safety_color() {
    let output = compile_expr(&Expr::resetn(Expr::shift(
        "retry",
        Expr::call(Expr::var("retry"), Expr::i64(1)),
    )));

    let visual = output.render_visual();
    assert!(visual.contains("control:"));
    assert!(visual.contains("replay_safety: Safe"));
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
