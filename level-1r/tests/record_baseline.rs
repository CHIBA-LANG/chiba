use chiba_level1r::core::{CoreOp, LayoutKind};
use chiba_level1r::template::{canonical_closed_row, ShapeType};
use chiba_level1r::typed::{type_expr, RecordTypeField, Type, TypedExprKind};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn record_literal_type_canonicalizes_field_order() {
    let typed = type_expr(&Expr::record(vec![
        ("y", Expr::bool(true)),
        ("x", Expr::i64(1)),
    ]));

    assert_eq!(
        typed.ty,
        Type::Record(vec![
            RecordTypeField {
                name: "x".to_string(),
                ty: Type::I64,
            },
            RecordTypeField {
                name: "y".to_string(),
                ty: Type::Bool,
            },
        ])
    );
    match typed.kind {
        TypedExprKind::Record { fields } => {
            assert_eq!(fields[0].name, "y");
            assert_eq!(fields[1].name, "x");
        }
        other => panic!("expected record expression, got {other:?}"),
    }
}

#[test]
fn record_literal_generates_closed_row_shape() {
    let output = compile_expr(&Expr::record(vec![
        ("y", Expr::bool(true)),
        ("x", Expr::i64(1)),
    ]));

    assert!(output.template.row_shapes.contains(&canonical_closed_row(vec![
        ("x", ShapeType::Unknown),
        ("y", ShapeType::Unknown),
    ])));
}

#[test]
fn record_literal_cps_and_core_use_canonical_layout() {
    let output = compile_expr(&Expr::record(vec![
        ("y", Expr::bool(true)),
        ("x", Expr::i64(1)),
    ]));

    assert_eq!(output.cps.to_string(), "halt record::x+y{x=1, y=true}");
    assert!(output.core.ops.contains(&CoreOp::RecordConstruct {
        layout: "record::x+y".to_string(),
        fields: vec!["x".to_string(), "y".to_string()],
    }));
    assert!(output.core.layouts.iter().any(|layout| {
        matches!(
            &layout.kind,
            LayoutKind::RecordStruct(record)
                if record.fields == vec!["x".to_string(), "y".to_string()]
        )
    }));
    assert_eq!(output.core_validation.diagnostics, vec![]);
}

#[test]
fn record_field_access_infers_type_and_validates_layout() {
    let expr = Expr::field(
        Expr::record(vec![("x", Expr::i64(1)), ("y", Expr::bool(true))]),
        "y",
    );
    let output = compile_expr(&expr);

    assert_eq!(output.typed.ty, Type::Bool);
    assert_eq!(
        output.cps.to_string(),
        "halt record::x+y{x=1, y=true}.y"
    );
    assert!(output.core.ops.contains(&CoreOp::RecordFieldGet {
        layout: "record::x+y".to_string(),
        field: "y".to_string(),
    }));
    assert!(output.core_validation.diagnostics.is_empty());
}
