use chiba_level1r::ast::BinaryOp;
use chiba_level1r::template::{dyn_row_contract, ShapeType, TemplateFacts, TemplateObligation};
use chiba_level1r::template_audit::{audit_checked_templates, TemplateObligationSource};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn field_obligation_is_checked_discharged_and_monomorphized() {
    let output = compile_expr(&Expr::field(Expr::var("v"), "name"));

    let field = output
        .template_audit
        .obligations
        .iter()
        .find(|entry| entry.source == TemplateObligationSource::Field)
        .expect("field audit entry");

    assert!(field.definition_time_check);
    assert!(field.instantiation_time_discharge);
    assert!(field.monomorphized);
    assert!(!field.rust_trait_solver_used);
    assert!(field.specialization_key.starts_with("field::symbol="));
    assert!(field.specialization_key.contains("::shape=open|name:?"));
    assert!(!field.specialization_key.contains("SpecializationKey"));
    assert!(!field.specialization_key.contains("RowShape"));
    assert!(field.explanation.contains("open-row structural obligation"));
    assert!(output.template_audit.diagnostics.is_empty());
}

#[test]
fn method_and_operator_obligations_do_not_use_rust_trait_solver() {
    let output = compile_expr(&Expr::binary(
        BinaryOp::Add,
        Expr::nominal("Vec2", Expr::var("a")),
        Expr::nominal("Vec2", Expr::var("b")),
    ));

    let operator = output
        .template_audit
        .obligations
        .iter()
        .find(|entry| entry.source == TemplateObligationSource::Operator)
        .expect("operator audit entry");

    assert!(operator.definition_time_check);
    assert!(operator.instantiation_time_discharge);
    assert!(operator.monomorphized);
    assert!(!operator.rust_trait_solver_used);
    assert!(operator
        .explanation
        .contains("structural operator obligation"));
}

#[test]
fn dyn_adapter_audit_is_instantiation_time_packaging() {
    let contract = dyn_row_contract(vec![("name", ShapeType::Named("String".to_string()))]);
    let mut template = TemplateFacts::default();
    template.dyn_contracts.push(contract.clone());
    template
        .obligations
        .push(TemplateObligation::DynAdapter { contract });

    let specialize = chiba_level1r::plan_specialization("dyn_user", &template);
    let monomorphize = chiba_level1r::monomorphize::schedule_monomorphization(&specialize);
    let report = audit_checked_templates(&template, &specialize, &monomorphize);

    assert!(report.diagnostics.is_empty());
    assert!(report.obligations.iter().any(|entry| entry.source
        == TemplateObligationSource::DynAdapter
        && entry.instantiation_time_discharge
        && entry.monomorphized
        && !entry.rust_trait_solver_used
        && entry
            .explanation
            .contains("not resolved by runtime impl search")));
}

#[test]
fn visual_report_contains_template_audit_layer() {
    let output = compile_expr(&Expr::field(Expr::var("v"), "name"));
    let visual = output.render_visual();

    assert!(visual.contains("template-audit:"));
    assert!(visual.contains("source=field"));
    assert!(visual.contains("definition-check=true"));
    assert!(visual.contains("instantiation-discharge=true"));
    assert!(visual.contains("monomorphized=true"));
    assert!(visual.contains("rust-trait-solver=false"));
    assert!(!output.visual.template_audit.contains("TemplateAuditReport"));
    assert!(!output.visual.template_audit.contains("TemplateAuditEntry"));
    assert!(visual.contains(
        "L6TemplateAudit: TemplateFacts+SpecializationFacts+MonomorphizationPlan -> TemplateAuditReport"
    ));
}
