use chiba_level1r::closure_core_usage::{
    ClosureCoreUsageFacts, ClosurePackageUsage, ContinuationPackageUsage, EnvFieldUsage,
};
use chiba_level1r::closure_simplify::{
    simplify_closure_core, ClosurePackageDecision, CodePointerDecision,
    ContinuationPackageDecision, EnvFieldDecision,
};
use chiba_level1r::control::ContinuationKind;
use chiba_level1r::core::CallableStorageKind;
use chiba_level1r::typed::{SendColor, UsageColor};
use chiba_level1r::usage::UseCount;
use chiba_level1r::{compile_expr, Expr};

#[test]
fn no_capture_closure_package_is_erased_and_direct_called() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert_eq!(
        output.closure_simplification.closure_packages["closure::x"],
        ClosurePackageDecision::EraseNoCapture
    );
    assert_eq!(
        output.closure_simplification.code_pointers["lift::0000::closure__x"],
        CodePointerDecision::KnownDirectCall
    );
}

#[test]
fn capturing_env_closure_keeps_env_package_and_indirect_code_pointer() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));

    assert_eq!(
        output.closure_simplification.closure_packages["closure::y"],
        ClosurePackageDecision::KeepEnvPackage
    );
    assert_eq!(
        output.closure_simplification.env_fields["closure::y.x"],
        EnvFieldDecision::KeepField
    );
    assert_eq!(
        output.closure_simplification.code_pointers["lift::0001::closure__y"],
        CodePointerDecision::KeepIndirectCodePointer
    );
}

#[test]
fn single_use_env_closure_is_marked_for_directification() {
    let mut usage = ClosureCoreUsageFacts::default();
    usage.closure_packages.insert(
        "closure::once".to_string(),
        ClosurePackageUsage {
            storage: CallableStorageKind::EnvClosure,
            count: UseCount::One,
            send: SendColor::Obligation,
        },
    );

    let decisions = simplify_closure_core(&usage);

    assert_eq!(
        decisions.closure_packages["closure::once"],
        ClosurePackageDecision::DirectifySingleUse
    );
}

#[test]
fn contn_package_is_kept_as_repeatable_package() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    assert_eq!(
        output.closure_simplification.continuation_packages["continuation::contn::retry"],
        ContinuationPackageDecision::KeepRepeatablePackage
    );
}

#[test]
fn dead_env_field_and_unused_continuation_package_are_marked_removable() {
    let mut usage = ClosureCoreUsageFacts::default();
    usage.env_fields.insert(
        "closure::f.dead".to_string(),
        EnvFieldUsage {
            closure: "closure::f".to_string(),
            field: "dead".to_string(),
            color: UsageColor::Obligation,
            send: SendColor::Obligation,
        },
    );
    usage.continuation_packages.insert(
        "continuation::contn::unused".to_string(),
        ContinuationPackageUsage {
            kind: ContinuationKind::ContN,
            count: UseCount::Zero,
        },
    );

    let decisions = simplify_closure_core(&usage);

    assert_eq!(
        decisions.env_fields["closure::f.dead"],
        EnvFieldDecision::RemoveDeadField
    );
    assert_eq!(
        decisions.continuation_packages["continuation::contn::unused"],
        ContinuationPackageDecision::RemoveUnusedPackage
    );
}

#[test]
fn closure_simplification_visual_dump_is_present() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    let visual = output.render_visual();
    assert!(visual.contains("closure-simplification:"));
    assert!(visual.contains("closure-package closure::x decision=erase-no-capture"));
    assert!(visual.contains("code-pointer lift::0000::closure__x decision=known-direct-call"));
    assert!(!output
        .visual
        .closure_simplification
        .contains("ClosureSimplificationFacts"));
    assert!(!output
        .visual
        .closure_simplification
        .contains("EraseNoCapture"));
}
