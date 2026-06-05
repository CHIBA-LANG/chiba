use chiba_level1r::core::{lower_core_with_facts, CoreOp, LayoutKind};
use chiba_level1r::cps::{CpsAtom, CpsProgram, CpsTerm};
use chiba_level1r::typed::{type_expr, FieldAccessKind, Type, TypedExprKind};
use chiba_level1r::{
    compile_expr, compile_program_bundle, Expr, ParamDecl, SourceItem, SourceProgram, Visibility,
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
fn tuple_expression_has_stable_nominal_type_from_ordered_field_types() {
    let typed = type_expr(&Expr::tuple(vec![Expr::i64(1), Expr::bool(true)]));

    assert_eq!(typed.ty, Type::Tuple(vec![Type::I64, Type::Bool]));
    match typed.kind {
        TypedExprKind::Tuple { nominal, fields } => {
            assert_eq!(nominal, "Tuple2_I64_Bool");
            assert_eq!(fields.len(), 2);
        }
        other => panic!(
            "expected tuple expression, got {}",
            typed_expr_kind_name(&other)
        ),
    }
}

#[test]
fn tuple_expression_cps_preserves_left_to_right_field_order_without_admin_chain() {
    let output = compile_expr(&Expr::tuple(vec![Expr::var("a"), Expr::var("b")]));
    let rendered = output.cps.to_string();

    assert_eq!(rendered, "halt Tuple2_Unknown_Unknown(_1=a, _2=b)");
    assert!(!rendered.contains("LetCont"));
    assert!(!rendered.contains("AppCont"));
    assert_eq!(output.usage.vars["a"], chiba_level1r::usage::UseCount::One);
    assert_eq!(output.usage.vars["b"], chiba_level1r::usage::UseCount::One);
}

#[test]
fn tuple_expression_reaches_target_neutral_core_with_tuple_layout() {
    let output = compile_expr(&Expr::tuple(vec![Expr::i64(1), Expr::i64(2)]));

    assert!(output.core.ops.contains(&CoreOp::TupleConstruct {
        nominal: "Tuple2_I64_I64".to_string(),
        layout: "tuple::Tuple2_I64_I64".to_string(),
        fields: vec!["1".to_string(), "2".to_string()],
    }));
    assert!(output.core.layouts.iter().any(|layout| {
        matches!(
            &layout.kind,
            LayoutKind::TupleStruct(tuple)
                if tuple.nominal == "Tuple2_I64_I64"
                    && tuple.fields == vec!["_1".to_string(), "_2".to_string()]
        )
    }));
    assert_eq!(output.core_validation.diagnostics, vec![]);
    assert!(output.render_visual().contains("Tuple2_I64_I64"));
}

#[test]
fn tuple_layout_nominal_comes_from_core_fact_not_layout_key_shape() {
    let cps = CpsProgram {
        term: CpsTerm::Halt(CpsAtom::Tuple {
            nominal: "SourceTupleName".to_string(),
            fields: vec![CpsAtom::Lit(chiba_level1r::Literal::I64(1))],
        }),
    };

    let core = lower_core_with_facts(
        &cps,
        &[],
        &chiba_level1r::closure::ClosureFacts::default(),
        &[],
        &chiba_level1r::lambda_lift::LambdaLiftFacts::default(),
        &chiba_level1r::specialize::SpecializationFacts::default(),
        &chiba_level1r::usage::UsageFacts::default(),
    );

    assert!(core.layouts.iter().any(|layout| {
        matches!(
            &layout.kind,
            LayoutKind::TupleStruct(tuple)
                if layout.key == "tuple::SourceTupleName" && tuple.nominal == "SourceTupleName"
        )
    }));
}

#[test]
fn tuple_positional_field_access_has_stable_underscore_names() {
    let expr = Expr::field(Expr::tuple(vec![Expr::i64(1), Expr::bool(true)]), "_2");
    let output = compile_expr(&expr);

    assert_eq!(output.typed.ty, Type::Bool);
    match &output.typed.kind {
        TypedExprKind::Field { name, access, .. } => {
            assert_eq!(name, "_2");
            assert_eq!(*access, FieldAccessKind::TuplePositionalRow { index: 1 });
        }
        other => panic!(
            "expected ordinary typed field access, got {}",
            typed_expr_kind_name(other)
        ),
    }
    assert_eq!(
        output.cps.to_string(),
        "halt Tuple2_I64_Bool(_1=1, _2=true)._2"
    );
    assert!(output.core.ops.contains(&CoreOp::TupleFieldGet {
        nominal: "Tuple2_I64_Bool".to_string(),
        layout: "tuple::Tuple2_I64_Bool".to_string(),
        field: "_2".to_string(),
        field_index: 1,
    }));
    assert!(output.core_validation.diagnostics.is_empty());
}

#[test]
fn tuple_header_type_field_access_uses_tuple_type_arguments() {
    let program = SourceProgram::new(vec![SourceItem::Def {
        receiver: None,
        generics: Vec::new(),
        name: "second".to_string(),
        visibility: Visibility::Public,
        params: vec![ParamDecl::new("pair", Some("Tuple[i64,bool]".to_string()))],
        return_type: Some("bool".to_string()),
        body: Expr::field(Expr::var("pair"), "_2"),
    }]);

    let output = compile_program_bundle(&program);

    assert_eq!(output.defs[0].output.typed.ty, Type::Bool);
}

#[test]
fn nested_nominal_tuple_header_substitution_preserves_positional_row_type() {
    let source = "type Box[T] = { value: T }
def main(box: Box[Tuple[i64,bool]]) = box.value._2";

    let output = chiba_level1r::compile_source_program_bundle(source).expect("compile source");

    assert_eq!(output.program.defs[0].output.typed.ty, Type::Bool);
}
