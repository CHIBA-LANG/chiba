use chiba_level1r::std_audit::{
    audit_std_dependencies, StdCapability, StdClassification, StdRequirement,
};
use chiba_level1r::{compile_expr, Expr};

fn find(requirements: &[StdRequirement], capability: StdCapability) -> &StdRequirement {
    requirements
        .iter()
        .find(|requirement| requirement.capability == capability)
        .unwrap()
}

#[test]
fn std_audit_classifies_first_batch_chiba_std_requirements() {
    let report = audit_std_dependencies();

    for capability in [
        StdCapability::GrowableSequence,
        StdCapability::OrderedMap,
        StdCapability::OrderedSet,
        StdCapability::StringBuilder,
        StdCapability::Utf8Chars,
    ] {
        assert_eq!(
            find(&report.requirements, capability).classification,
            StdClassification::ChibaStdFirstBatch
        );
    }
    assert_eq!(report.diagnostics, vec![]);
}

#[test]
fn std_audit_separates_intrinsics_from_host_boundaries_and_rust_convenience() {
    let report = audit_std_dependencies();

    assert_eq!(
        find(&report.requirements, StdCapability::RcSharedReference).classification,
        StdClassification::CompilerIntrinsic
    );
    assert_eq!(
        find(&report.requirements, StdCapability::StableHash).classification,
        StdClassification::CompilerIntrinsic
    );
    assert_eq!(
        find(&report.requirements, StdCapability::FileRead).classification,
        StdClassification::ChibaUnsafeOrMetalBoundary
    );
    assert_eq!(
        find(&report.requirements, StdCapability::PathJoin).classification,
        StdClassification::ChibaUnsafeOrMetalBoundary
    );
    assert_eq!(
        find(&report.requirements, StdCapability::TimeMeasurement).classification,
        StdClassification::RustImplementationConvenience
    );
    assert_eq!(
        find(&report.requirements, StdCapability::Formatting).classification,
        StdClassification::RustImplementationConvenience
    );
}

#[test]
fn std_audit_is_visible_in_pipeline_report() {
    let output = compile_expr(&Expr::var("x"));

    assert!(output
        .std_audit
        .requirements
        .iter()
        .any(|requirement| requirement.capability == StdCapability::GrowableSequence));
    let visual = output.render_visual();
    assert!(visual.contains("std-audit:"));
    assert!(visual.contains("requirement growable-sequence classification=chiba-std-first-batch"));
    assert!(visual.contains("requirement stable-hash classification=compiler-intrinsic"));
    assert!(visual.contains("requirement file-read classification=chiba-unsafe-or-metal-boundary"));
    assert!(!output.visual.std_audit.contains("StdAuditReport"));
    assert!(!output.visual.std_audit.contains("ChibaStdFirstBatch"));
}
