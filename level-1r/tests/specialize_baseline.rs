use chiba_level1r::specialize::{
    plan_specialization, specialization_key, InstantiationRegistry, InstantiationStatus,
};
use chiba_level1r::template::{
    canonical_open_row, dyn_row_contract, ShapeType, TemplateFacts, TemplateObligation,
};
use chiba_level1r::typed::{SendColor, UsageColor};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn specialization_key_uses_canonical_shape_order() {
    let mut left = TemplateFacts::default();
    left.row_shapes.push(canonical_open_row(vec![
        ("x", ShapeType::Unknown),
        ("y", ShapeType::Unknown),
    ]));

    let mut right = TemplateFacts::default();
    right.row_shapes.push(canonical_open_row(vec![
        ("y", ShapeType::Unknown),
        ("x", ShapeType::Unknown),
    ]));

    assert_eq!(
        specialization_key("get_xy", &left),
        specialization_key("get_xy", &right)
    );
}

#[test]
fn specialization_key_keeps_nominal_identity_even_for_same_shape() {
    let mut vec2 = TemplateFacts::default();
    vec2.row_shapes
        .push(canonical_open_row(vec![("x", ShapeType::Unknown)]));
    vec2.obligations.push(TemplateObligation::Method {
        receiver: Some("Vec2".to_string()),
        name: "norm".to_string(),
        resolved: Some("Vec2.norm".to_string()),
    });

    let mut point = TemplateFacts::default();
    point
        .row_shapes
        .push(canonical_open_row(vec![("x", ShapeType::Unknown)]));
    point.obligations.push(TemplateObligation::Method {
        receiver: Some("Point".to_string()),
        name: "norm".to_string(),
        resolved: Some("Point.norm".to_string()),
    });

    assert_ne!(
        specialization_key("norm_like", &vec2),
        specialization_key("norm_like", &point)
    );
}

#[test]
fn dyn_contract_enters_specialization_key_and_capability_facts() {
    let mut template = TemplateFacts::default();
    template
        .dyn_contracts
        .push(dyn_row_contract(vec![("name", ShapeType::Named("String".to_string()))]));

    let key = specialization_key("render", &template);

    assert_eq!(key.dyn_contracts.len(), 1);
    assert_eq!(key.capabilities.usage, vec![UsageColor::Many]);
    assert_eq!(key.capabilities.send, vec![SendColor::Obligation]);
}

#[test]
fn registry_joins_duplicate_specialization_requests() {
    let template = TemplateFacts::default();
    let key = specialization_key("id", &template);
    let mut registry = InstantiationRegistry::default();

    assert_eq!(registry.request(key.clone()), InstantiationStatus::InProgress);
    assert_eq!(registry.request(key.clone()), InstantiationStatus::InProgress);
    assert_eq!(registry.len(), 1);

    registry.finish(key.clone(), "artifact#id");
    assert_eq!(
        registry.status(&key),
        Some(&InstantiationStatus::Done {
            artifact: "artifact#id".to_string(),
        })
    );
}

#[test]
fn compile_output_contains_specialization_work_item() {
    let output = compile_expr(&Expr::field(Expr::var("user"), "name"));

    assert_eq!(output.specialize.work_items.len(), 1);
    assert!(output
        .specialize
        .work_items[0]
        .key
        .normalized_shapes
        .contains(&canonical_open_row(vec![("name", ShapeType::Unknown)])));
    assert!(output.render_visual().contains("specialize:"));
    assert!(output.render_visual().contains("monomorphize:"));
    assert_eq!(output.monomorphize.jobs.len(), 1);
}

#[test]
fn plan_specialization_discharges_template_obligations() {
    let mut template = TemplateFacts::default();
    template.obligations.push(TemplateObligation::Field {
        shape: canonical_open_row(vec![("name", ShapeType::Unknown)]),
        field: "name".to_string(),
    });

    let facts = plan_specialization("get_name", &template);

    assert_eq!(facts.work_items.len(), 1);
    assert_eq!(facts.work_items[0].obligations.len(), 1);
}
