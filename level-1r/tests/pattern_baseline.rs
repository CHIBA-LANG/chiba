use chiba_level1r::ast::Pattern;
use chiba_level1r::pattern::PatternDiagnostic;
use chiba_level1r::typed::Type;
use chiba_level1r::{compile_expr, Expr, Literal};

#[test]
fn bool_match_is_exhaustive_when_both_literals_are_covered() {
    let output = compile_expr(&Expr::match_expr(
        Expr::bool(true),
        vec![
            (Pattern::lit_bool(true), Expr::i64(1)),
            (Pattern::lit_bool(false), Expr::i64(0)),
        ],
    ));

    assert_eq!(output.pattern.diagnostics, vec![]);
    assert_eq!(output.pattern.matches.len(), 1);
    let fact = &output.pattern.matches[0];
    assert_eq!(fact.scrutinee_type, Type::Bool);
    assert_eq!(
        fact.covered_literals,
        vec![Literal::Bool(true), Literal::Bool(false)]
    );
    assert_eq!(fact.has_wildcard, false);
    assert_eq!(fact.exhaustive, true);
}

#[test]
fn wildcard_makes_match_exhaustive_for_open_domains() {
    let output = compile_expr(&Expr::match_expr(
        Expr::var("n"),
        vec![
            (Pattern::lit_i64(0), Expr::i64(10)),
            (Pattern::wildcard(), Expr::i64(20)),
        ],
    ));

    assert_eq!(output.pattern.diagnostics, vec![]);
    assert_eq!(output.pattern.matches[0].has_wildcard, true);
    assert_eq!(output.pattern.matches[0].exhaustive, true);
}

#[test]
fn bool_match_reports_missing_literal() {
    let output = compile_expr(&Expr::match_expr(
        Expr::bool(true),
        vec![(Pattern::lit_bool(true), Expr::i64(1))],
    ));

    assert_eq!(
        output.pattern.diagnostics,
        vec![PatternDiagnostic::NonExhaustiveMatch {
            scrutinee_type: Type::Bool,
            missing: vec![Pattern::lit_bool(false)],
        }]
    );
}

#[test]
fn non_bool_literal_match_without_wildcard_requires_fallback() {
    let output = compile_expr(&Expr::match_expr(
        Expr::var("tag"),
        vec![(Pattern::lit_i64(0), Expr::i64(1))],
    ));

    assert_eq!(
        output.pattern.diagnostics,
        vec![PatternDiagnostic::NonExhaustiveMatch {
            scrutinee_type: Type::Unknown,
            missing: vec![Pattern::wildcard()],
        }]
    );
}

#[test]
fn if_let_records_success_only_pattern_environment() {
    let output = compile_expr(&Expr::if_let(
        Pattern::bind("value"),
        Expr::var("candidate"),
        Expr::var("value"),
        Expr::i64(0),
    ));

    assert_eq!(output.pattern.envs.len(), 1);
    let env = &output.pattern.envs[0];
    assert_eq!(env.bindings, vec!["value".to_string()]);
    assert_eq!(env.success_branch_binds, true);
    assert_eq!(env.failure_branch_binds, false);
}

#[test]
fn tuple_pattern_collects_nested_bindings_in_source_order() {
    let output = compile_expr(&Expr::if_let(
        Pattern::tuple(vec![
            Pattern::bind("head"),
            Pattern::tuple(vec![Pattern::wildcard(), Pattern::bind("tail")]),
        ]),
        Expr::var("pair"),
        Expr::var("head"),
        Expr::i64(0),
    ));

    assert_eq!(output.pattern.envs.len(), 1);
    assert_eq!(
        output.pattern.envs[0].bindings,
        vec!["head".to_string(), "tail".to_string()]
    );
    assert_eq!(output.pattern.envs[0].failure_branch_binds, false);
    assert!(output.cps.to_string().contains("(head, (_, tail)) => join"));
}

#[test]
fn tuple_pattern_reports_duplicate_binding_once() {
    let output = compile_expr(&Expr::if_let(
        Pattern::tuple(vec![
            Pattern::bind("x"),
            Pattern::tuple(vec![Pattern::bind("x"), Pattern::bind("y")]),
            Pattern::bind("x"),
        ]),
        Expr::var("pair"),
        Expr::var("y"),
        Expr::i64(0),
    ));

    assert_eq!(
        output.pattern.diagnostics,
        vec![PatternDiagnostic::DuplicateBinding {
            name: "x".to_string(),
        }]
    );
    assert_eq!(
        output.pattern.envs[0].bindings,
        vec!["x".to_string(), "x".to_string(), "y".to_string(), "x".to_string()]
    );
}
