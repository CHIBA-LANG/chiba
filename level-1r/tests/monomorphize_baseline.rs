use chiba_level1r::monomorphize::{
    fail_job, mark_in_progress, schedule_monomorphization, MonomorphizationStatus,
};
use chiba_level1r::specialize::{
    plan_specialization, specialization_key, SpecializationFacts, SpecializationWorkItem,
};
use chiba_level1r::template::{
    canonical_open_row, dyn_row_contract, ShapeType, TemplateFacts, TemplateInstantiation,
    TemplateObligation, TemplateParam, TemplateParamSource,
};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn scheduler_deduplicates_identical_specialization_keys() {
    let template = TemplateFacts::default();
    let key = specialization_key("id", &template);
    let facts = SpecializationFacts {
        work_items: vec![
            SpecializationWorkItem {
                key: key.clone(),
                obligations: vec![],
            },
            SpecializationWorkItem {
                key: key.clone(),
                obligations: vec![],
            },
        ],
        registry: Default::default(),
    };

    let plan = schedule_monomorphization(&facts);

    assert_eq!(plan.jobs.len(), 1);
    assert_eq!(plan.jobs[0].call_sites, vec!["call-site::0", "call-site::1"]);
    assert_eq!(plan.duplicates.len(), 1);
    assert_eq!(plan.duplicates[0].call_site, "call-site::1");
    assert_eq!(plan.duplicates[0].joined_artifact, plan.jobs[0].artifact);
}

#[test]
fn scheduler_artifact_name_is_stable_and_keeps_shape_dyn_abi_dimensions() {
    let mut template = TemplateFacts::default();
    template
        .row_shapes
        .push(canonical_open_row(vec![("name", ShapeType::Unknown)]));
    template
        .dyn_contracts
        .push(dyn_row_contract(vec![("name", ShapeType::Named("String".to_string()))]));
    template.obligations.push(TemplateObligation::Method {
        receiver: Some("User".to_string()),
        name: "render".to_string(),
        resolved: Some("User.render".to_string()),
    });

    let plan = schedule_monomorphization(&plan_specialization("render", &template));
    let artifact = &plan.jobs[0].artifact;

    assert!(artifact.starts_with("mono::render::nominal=User"));
    assert!(artifact.contains("::shape="));
    assert!(artifact.contains("::dyn="));
    assert!(artifact.ends_with("::abi=Chiba"));
    assert_eq!(plan.jobs[0].definition_note, "definition-site::render");
}

#[test]
fn scheduler_artifact_name_keeps_explicit_template_dimensions() {
    let mut template = TemplateFacts::default();
    template.explicit_params.push(TemplateParam {
        name: "T".to_string(),
        source: TemplateParamSource::ExplicitHeader,
    });
    template
        .explicit_instantiations
        .push(TemplateInstantiation {
            callee: "id".to_string(),
            type_args: vec!["i64".to_string()],
        });

    let plan = schedule_monomorphization(&plan_specialization("id", &template));
    let artifact = &plan.jobs[0].artifact;

    assert!(artifact.starts_with("mono::id::params="));
    assert!(artifact.contains("::typeargs="));
    assert!(artifact.ends_with("::abi=Chiba"));
}

#[test]
fn scheduler_tracks_in_progress_and_failed_call_site_with_definition_note() {
    let template = TemplateFacts::default();
    let key = specialization_key("id", &template);
    let facts = plan_specialization("id", &template);
    let mut plan = schedule_monomorphization(&facts);

    mark_in_progress(&mut plan, &key);
    assert_eq!(plan.jobs[0].status, MonomorphizationStatus::InProgress);

    fail_job(&mut plan, &key, "call-site::bad", "missing method op_add");
    assert_eq!(
        plan.jobs[0].status,
        MonomorphizationStatus::Failed {
            call_site: "call-site::bad".to_string(),
            definition_note: "definition-site::id".to_string(),
            diagnostic: "missing method op_add".to_string(),
        }
    );
}

#[test]
fn compile_output_contains_monomorphization_plan_and_visual_dump() {
    let output = compile_expr(&Expr::field(Expr::var("user"), "name"));

    assert_eq!(output.monomorphize.jobs.len(), 1);
    assert!(output.monomorphize.jobs[0].artifact.starts_with("mono::<expr>"));
    assert!(output.render_visual().contains("monomorphize:"));
    assert!(output.render_visual().contains("MonomorphizationPlan"));
}
