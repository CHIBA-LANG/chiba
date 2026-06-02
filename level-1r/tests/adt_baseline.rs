use chiba_level1r::ast::Pattern;
use chiba_level1r::core::{CoreOp, LayoutKind};
use chiba_level1r::pattern::PatternDiagnostic;
use chiba_level1r::typed::{type_expr, Type, TypedExprKind};
use chiba_level1r::{compile_expr, Expr};

fn option_some(value: Expr) -> Expr {
    Expr::adt_ctor("Option", "Some", vec!["Some", "None"], vec![value])
}

fn option_none() -> Expr {
    Expr::adt_ctor("Option", "None", vec!["Some", "None"], vec![])
}

#[test]
fn adt_constructor_expression_carries_known_data_shape() {
    let typed = type_expr(&option_some(Expr::i64(1)));

    assert_eq!(
        typed.ty,
        Type::Adt {
            name: "Option".to_string(),
            variants: vec!["None".to_string(), "Some".to_string()],
        }
    );
    match typed.kind {
        TypedExprKind::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            assert_eq!(data, "Option");
            assert_eq!(ctor, "Some");
            assert_eq!(variants, vec!["None".to_string(), "Some".to_string()]);
            assert_eq!(args.len(), 1);
        }
        other => panic!("expected ADT ctor expression, got {other:?}"),
    }
}

#[test]
fn adt_constructor_reaches_cps_core_and_target_neutral_layout() {
    let output = compile_expr(&option_some(Expr::i64(1)));

    assert_eq!(output.cps.to_string(), "halt Option.Some(1)");
    assert!(output.core.ops.contains(&CoreOp::AdtConstruct {
        data: "Option".to_string(),
        ctor: "Some".to_string(),
        variants: vec!["None".to_string(), "Some".to_string()],
        args: vec!["I64(1)".to_string()],
    }));
    assert!(output.core.layouts.iter().any(|layout| {
        matches!(
            &layout.kind,
            LayoutKind::AdtShape(shape)
                if shape.data == "Option"
                    && shape.variants == vec!["None".to_string(), "Some".to_string()]
        )
    }));
    assert_eq!(output.core_validation.diagnostics, vec![]);
    assert!(output.render_visual().contains("Option.Some"));
}

#[test]
fn constructor_pattern_collects_nested_bindings() {
    let output = compile_expr(&Expr::if_let(
        Pattern::ctor("Some", vec![Pattern::bind("value")]),
        option_some(Expr::i64(1)),
        Expr::var("value"),
        Expr::i64(0),
    ));

    assert_eq!(output.pattern.envs.len(), 1);
    assert_eq!(output.pattern.envs[0].bindings, vec!["value".to_string()]);
    assert!(output.cps.to_string().contains("Some(value) => join"));
}

#[test]
fn adt_match_is_exhaustive_when_all_known_variants_are_covered() {
    let output = compile_expr(&Expr::match_expr(
        option_some(Expr::i64(1)),
        vec![
            (Pattern::ctor("Some", vec![Pattern::bind("value")]), Expr::var("value")),
            (Pattern::ctor("None", vec![]), Expr::i64(0)),
        ],
    ));

    assert_eq!(output.pattern.diagnostics, vec![]);
    assert_eq!(output.pattern.matches.len(), 1);
    let fact = &output.pattern.matches[0];
    assert_eq!(
        fact.covered_constructors,
        vec!["Some".to_string(), "None".to_string()]
    );
    assert_eq!(fact.exhaustive, true);
}

#[test]
fn adt_match_reports_missing_constructor_with_qualified_pattern() {
    let output = compile_expr(&Expr::match_expr(
        option_some(Expr::i64(1)),
        vec![(Pattern::ctor("Some", vec![Pattern::bind("value")]), Expr::var("value"))],
    ));

    assert_eq!(
        output.pattern.diagnostics,
        vec![PatternDiagnostic::NonExhaustiveMatch {
            scrutinee_type: Type::Adt {
                name: "Option".to_string(),
                variants: vec!["None".to_string(), "Some".to_string()],
            },
            missing: vec![Pattern::qualified_ctor("Option", "None", vec![])],
        }]
    );
}

#[test]
fn qualified_constructor_pattern_displays_stably() {
    let output = compile_expr(&Expr::match_expr(
        option_none(),
        vec![
            (
                Pattern::qualified_ctor("Option", "Some", vec![Pattern::bind("value")]),
                Expr::var("value"),
            ),
            (Pattern::qualified_ctor("Option", "None", vec![]), Expr::i64(0)),
        ],
    ));

    assert!(output.cps.to_string().contains("Option.Some(value) => join"));
    assert!(output.cps.to_string().contains("Option.None => join"));
    assert_eq!(output.pattern.diagnostics, vec![]);
}
