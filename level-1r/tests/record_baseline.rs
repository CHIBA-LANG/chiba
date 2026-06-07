use chiba_level1r::core::{CoreOp, LayoutKind};
use chiba_level1r::template::{canonical_closed_row, ShapeType};
use chiba_level1r::typed::{
    type_expr, type_expr_with_context, RecordTypeField, Type, TypeContext, TypeEnv, TypedExprKind,
};
use chiba_level1r::{
    compile_expr, compile_program_bundle, Expr, ParamDecl, SourceItem, SourceProgram, TypeDecl,
    TypeField, Visibility,
};

fn typed_expr_kind_name(kind: &TypedExprKind) -> &'static str {
    match kind {
        TypedExprKind::Var(_) => "var",
        TypedExprKind::Lit(_) => "literal",
        TypedExprKind::Lambda { .. } => "lambda",
        TypedExprKind::Call { .. } => "call",
        TypedExprKind::Tuple { .. } => "tuple",
        TypedExprKind::SliceLiteral { .. } => "slice-literal",
        TypedExprKind::Record { .. } => "record",
        TypedExprKind::RecordUpdate { .. } => "record-update",
        TypedExprKind::AdtCtor { .. } => "adt-ctor",
        TypedExprKind::Field { .. } => "field",
        TypedExprKind::MethodCall { .. } => "method-call",
        TypedExprKind::Assign { .. } => "assign",
        TypedExprKind::Index { .. } => "index",
        TypedExprKind::Range { .. } => "range",
        TypedExprKind::Binary { .. } => "binary",
        TypedExprKind::If { .. } => "if",
        TypedExprKind::IfLet { .. } => "if-let",
        TypedExprKind::Match { .. } => "match",
        TypedExprKind::Nominal { .. } => "nominal",
        TypedExprKind::Reset { .. } => "reset",
        TypedExprKind::Shift { .. } => "shift",
    }
}

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
        other => panic!(
            "expected record expression, got {}",
            typed_expr_kind_name(&other)
        ),
    }
}

#[test]
fn record_literal_generates_closed_row_shape() {
    let output = compile_expr(&Expr::record(vec![
        ("y", Expr::bool(true)),
        ("x", Expr::i64(1)),
    ]));

    assert!(output
        .template
        .row_shapes
        .contains(&canonical_closed_row(vec![
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
    assert_eq!(output.cps.to_string(), "halt record::x+y{x=1, y=true}.y");
    assert!(output.core.ops.contains(&CoreOp::RecordFieldGet {
        layout: "record::x+y".to_string(),
        field: "y".to_string(),
    }));
    assert!(output.core_validation.diagnostics.is_empty());
}

#[test]
fn core_visual_renders_structured_facts_without_rust_debug_shape() {
    let output = compile_expr(&Expr::field(
        Expr::record(vec![("x", Expr::i64(1)), ("y", Expr::bool(true))]),
        "y",
    ));
    let visual = &output.visual.core;

    assert!(visual.contains("ops="));
    assert!(visual.contains("record-construct layout=record::x+y fields=[x, y]"));
    assert!(visual.contains("record-field-get layout=record::x+y field=y"));
    assert!(visual.contains("layout record::x+y hash="));
    assert!(visual.contains("record-struct fields=[x, y]"));
    assert!(visual.contains("callable-storage="));
    assert!(!visual.contains("CoreProgram"));
    assert!(!visual.contains("CoreOp"));
    assert!(!visual.contains("RecordFieldGet"));
    assert!(!visual.contains("LayoutKind"));
    assert!(!visual.contains("RecordStruct"));
}

#[test]
fn typed_visual_renders_structured_expr_without_rust_debug_shape() {
    let output = compile_expr(&Expr::field(
        Expr::record(vec![("x", Expr::i64(1)), ("y", Expr::bool(true))]),
        "y",
    ));
    let visual = &output.visual.typed;

    assert!(visual.contains("node kind=field type=bool usage=obligation send=obligation"));
    assert!(visual.contains("field y access=record-or-nominal"));
    assert!(
        visual.contains("node kind=record type={x: i64, y: bool} usage=obligation send=obligation")
    );
    assert!(visual.contains("field x:"));
    assert!(visual.contains("literal 1"));
    assert!(visual.contains("field y:"));
    assert!(visual.contains("literal true"));
    assert!(!visual.contains("TypedExpr"));
    assert!(!visual.contains("TypedExprKind"));
    assert!(!visual.contains("RecordOrNominal"));
    assert!(!visual.contains("TuplePositionalRow"));
    assert!(!visual.contains("param_ty"));
}

#[test]
fn record_field_callable_method_call_uses_field_return_type() {
    let expr = Expr::method_call(
        Expr::record(vec![("apply", Expr::lambda("x", Expr::i64(7)))]),
        "apply",
        Expr::i64(1),
    );
    let typed = type_expr(&expr);

    assert_eq!(typed.ty, Type::I64);
}

#[test]
fn record_field_callable_method_call_steps_through_curried_function_type() {
    let mut env = TypeEnv::new();
    env.insert(
        "record".to_string(),
        Type::Record(vec![RecordTypeField {
            name: "apply".to_string(),
            ty: Type::Func(
                Box::new(Type::I64),
                Box::new(Type::Func(Box::new(Type::Bool), Box::new(Type::I64))),
            ),
        }]),
    );

    let typed = type_expr_with_context(
        &Expr::method_call_args(
            Expr::var("record"),
            "apply",
            vec![Expr::i64(1), Expr::bool(true)],
        ),
        &env,
        &TypeContext::new(),
    );

    assert_eq!(typed.ty, Type::I64);
}

#[test]
fn record_field_callable_method_call_partial_application_keeps_remaining_function_type() {
    let remaining = Type::Func(Box::new(Type::Bool), Box::new(Type::I64));
    let mut env = TypeEnv::new();
    env.insert(
        "record".to_string(),
        Type::Record(vec![RecordTypeField {
            name: "apply".to_string(),
            ty: Type::Func(Box::new(Type::I64), Box::new(remaining.clone())),
        }]),
    );

    let typed = type_expr_with_context(
        &Expr::method_call(Expr::var("record"), "apply", Expr::i64(1)),
        &env,
        &TypeContext::new(),
    );

    assert_eq!(typed.ty, remaining);
}

#[test]
fn nominal_field_callable_method_call_uses_field_return_type() {
    let mut context = TypeContext::new();
    context.insert_nominal_row(
        "CallableBox",
        vec![RecordTypeField {
            name: "apply".to_string(),
            ty: Type::Func(Box::new(Type::I64), Box::new(Type::Bool)),
        }],
    );
    let mut env = TypeEnv::new();
    env.insert("box".to_string(), Type::Nominal("CallableBox".to_string()));

    let typed = type_expr_with_context(
        &Expr::method_call(Expr::var("box"), "apply", Expr::i64(1)),
        &env,
        &context,
    );

    assert_eq!(typed.ty, Type::Bool);
}

#[test]
fn nominal_field_callable_method_call_uses_function_header_type() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "CallableBox",
            Vec::new(),
            vec![TypeField::new("apply", "(i64) -> bool")],
        )],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "main".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("box", Some("CallableBox".to_string()))],
            return_type: Some("bool".to_string()),
            body: Expr::method_call(Expr::var("box"), "apply", Expr::i64(1)),
        }],
    );

    let output = compile_program_bundle(&program);

    assert_eq!(output.defs[0].output.typed.ty, Type::Bool);
}

#[test]
fn record_update_overwrites_existing_field_and_preserves_canonical_layout() {
    let expr = Expr::record_update(
        Expr::record(vec![("x", Expr::i64(1)), ("y", Expr::bool(true))]),
        vec![("y", Expr::bool(false))],
    );
    let output = compile_expr(&expr);

    assert_eq!(
        output.typed.ty,
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
    assert_eq!(output.cps.to_string(), "halt record::x+y{x=1, y=false}");
    assert!(output.core.ops.contains(&CoreOp::RecordConstruct {
        layout: "record::x+y".to_string(),
        fields: vec!["x".to_string(), "y".to_string()],
    }));
    assert!(output.core_validation.diagnostics.is_empty());
}

#[test]
fn record_update_extends_record_shape() {
    let expr = Expr::record_update(
        Expr::record(vec![("x", Expr::i64(1))]),
        vec![("z", Expr::bool(true))],
    );
    let output = compile_expr(&expr);

    assert_eq!(output.cps.to_string(), "halt record::x+z{x=1, z=true}");
    assert!(output.core.layouts.iter().any(|layout| {
        matches!(
            &layout.kind,
            LayoutKind::RecordStruct(record)
                if record.fields == vec!["x".to_string(), "z".to_string()]
        )
    }));
    assert!(output.core_validation.diagnostics.is_empty());
}

#[test]
fn record_update_of_unknown_base_reaches_target_neutral_core_update_op() {
    let expr = Expr::record_update(Expr::var("base"), vec![("z", Expr::i64(4))]);
    let output = compile_expr(&expr);

    assert_eq!(output.cps.to_string(), "halt record::z{base=base, z=4}");
    assert!(output.core.ops.contains(&CoreOp::RecordUpdate {
        base: "base".to_string(),
        layout: "record::z".to_string(),
        fields: vec!["z".to_string()],
    }));
    assert_eq!(
        output.usage.vars["base"],
        chiba_level1r::usage::UseCount::One
    );
    assert!(output.core_validation.diagnostics.is_empty());
    assert!(!output.cps.to_string().contains("LetCont"));
}
