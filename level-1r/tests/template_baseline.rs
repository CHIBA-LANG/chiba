use chiba_level1r::ast::BinaryOp;
use chiba_level1r::resolve::ResolveDiagnostic;
use chiba_level1r::template::{
    canonical_open_row, dyn_row_contract, ShapeType, TemplateObligation,
};
use chiba_level1r::typed::{SendColor, UsageColor};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn row_shape_canonicalization_ignores_source_field_order() {
    let xy = canonical_open_row(vec![
        ("x", ShapeType::Named("i64".to_string())),
        ("y", ShapeType::Named("Bool".to_string())),
    ]);
    let yx = canonical_open_row(vec![
        ("y", ShapeType::Named("Bool".to_string())),
        ("x", ShapeType::Named("i64".to_string())),
    ]);

    assert_eq!(xy, yx);
    assert_eq!(xy.fields[0].name, "x");
    assert_eq!(xy.fields[1].name, "y");
}

#[test]
fn field_access_generates_open_row_template_obligation() {
    let output = compile_expr(&Expr::field(Expr::var("v"), "name"));

    assert_eq!(output.template.row_shapes.len(), 1);
    assert_eq!(
        output.template.obligations,
        vec![TemplateObligation::Field {
            shape: canonical_open_row(vec![("name", ShapeType::Unknown)]),
            field: "name".to_string(),
        }]
    );
}

#[test]
fn operator_resolve_fact_enters_template_obligations() {
    let output = compile_expr(&Expr::binary(
        BinaryOp::Add,
        Expr::nominal("Vec2", Expr::var("a")),
        Expr::nominal("Vec2", Expr::var("b")),
    ));

    assert!(output
        .template
        .obligations
        .contains(&TemplateObligation::Operator {
            op: chiba_level1r::resolve::OperatorSurface::Binary(BinaryOp::Add),
            protocol: "op_add".to_string(),
            receiver: Some("Vec2".to_string()),
        }));
}

#[test]
fn row_field_obligation_does_not_prove_nominal_method() {
    let output = compile_expr(&Expr::method_call(
        Expr::field(Expr::var("row"), "field_fn"),
        "len",
        Expr::i64(0),
    ));

    assert!(output
        .template
        .obligations
        .contains(&TemplateObligation::Field {
            shape: canonical_open_row(vec![("field_fn", ShapeType::Unknown)]),
            field: "field_fn".to_string(),
        }));
    assert_eq!(
        output.resolve.diagnostics,
        vec![ResolveDiagnostic::MissingMethod {
            receiver: None,
            name: "len".to_string(),
        }]
    );
}

#[test]
fn dyn_row_contract_carries_adapter_and_colors() {
    let contract = dyn_row_contract(vec![("name", ShapeType::Named("String".to_string()))]);

    assert_eq!(
        contract.shape,
        canonical_open_row(vec![("name", ShapeType::Named("String".to_string()))])
    );
    assert_eq!(contract.payload_usage, UsageColor::Many);
    assert_eq!(contract.send, SendColor::Obligation);
}

#[test]
fn visual_report_contains_template_layer() {
    let output = compile_expr(&Expr::field(Expr::var("v"), "name"));
    let visual = output.render_visual();

    assert!(visual.contains("template:"));
    assert!(visual.contains("L3Template: AlphaExpr+ResolveFacts -> TemplateFacts"));
}
