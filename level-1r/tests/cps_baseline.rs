use chiba_level1r::cps::{cps_program, CpsAtom, CpsTerm};
use chiba_level1r::core::CoreOp;
use chiba_level1r::typed::type_expr;
use chiba_level1r::usage::UseCount;
use chiba_level1r::{compile_expr, Expr};

#[test]
fn atom_variable_cps_is_halt_without_administrative_continuation() {
    let output = compile_expr(&Expr::var("x"));
    assert_eq!(output.cps.to_string(), "halt x");
    assert!(!output.cps.contains_administrative_let_cont());
}

#[test]
fn simple_call_cps_does_not_create_letcont_appcont_chain() {
    let typed = type_expr(&Expr::call(Expr::var("f"), Expr::var("x")));
    let cps = cps_program(&typed);
    let rendered = cps.to_string();

    assert!(rendered.starts_with("f(x, (cont w"));
    assert!(!rendered.contains("LetCont"));
    assert!(!rendered.contains("AppCont"));
    assert!(!rendered.contains("(lambda a."));
    assert!(!rendered.contains("(lambda b."));
}

#[test]
fn nested_call_preserves_cbv_left_to_right_shape() {
    let expr = Expr::call(Expr::call(Expr::var("f"), Expr::var("g")), Expr::var("x"));
    let output = compile_expr(&expr);
    let rendered = output.cps.to_string();

    assert!(rendered.contains("f(g,"));
    assert!(rendered.contains("(x,"));
}

#[test]
fn lambda_adds_object_level_continuation_parameter() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    match output.cps.term {
        CpsTerm::Halt(CpsAtom::FunLambda { param, k_param, body }) => {
            assert_eq!(param, "x");
            assert!(k_param.starts_with('k'));
            assert_eq!(body.to_string(), format!("{k_param}(x)"));
        }
        other => panic!("expected lambda atom, got {other:?}"),
    }
}

#[test]
fn if_cps_materializes_real_branch_join_without_administrative_chain() {
    let output = compile_expr(&Expr::if_else(
        Expr::var("cond"),
        Expr::call(Expr::var("ok_fn"), Expr::var("x")),
        Expr::var("fallback"),
    ));
    let rendered = output.cps.to_string();

    assert!(rendered.contains("if cond"));
    assert!(rendered.contains("{ ok_fn(x,"));
    assert!(rendered.contains("else"));
    assert!(rendered.contains("join"));
    assert!(!rendered.contains("LetCont"));
    assert!(!rendered.contains("AppCont"));
    assert!(!rendered.contains("lambda a"));
    assert!(!rendered.contains("lambda b"));
    assert!(output.core.ops.contains(&CoreOp::Branch {
        cond: "cond".to_string()
    }));
}

#[test]
fn literal_match_cps_lowers_to_ordered_branch_chain() {
    let output = compile_expr(&Expr::match_expr(
        Expr::var("tag"),
        vec![
            (chiba_level1r::ast::Pattern::lit_i64(0), Expr::i64(10)),
            (chiba_level1r::ast::Pattern::wildcard(), Expr::i64(20)),
        ],
    ));
    let rendered = output.cps.to_string();

    assert!(rendered.contains("match tag"));
    assert!(rendered.contains("0 => join"));
    assert!(rendered.contains("(10)"));
    assert!(rendered.contains("_ => join"));
    assert!(rendered.contains("(20)"));
    assert!(!rendered.contains("LetCont"));
    assert!(!rendered.contains("AppCont"));
    assert!(output.core.ops.contains(&CoreOp::Match {
        scrutinee: "tag".to_string(),
        patterns: vec!["Lit(I64(0))".to_string(), "Wildcard".to_string()],
    }));
}

#[test]
fn if_let_cps_has_success_and_failure_paths() {
    let output = compile_expr(&Expr::if_let(
        chiba_level1r::ast::Pattern::bind("value"),
        Expr::var("candidate"),
        Expr::var("value"),
        Expr::i64(0),
    ));
    let rendered = output.cps.to_string();

    assert!(rendered.contains("match candidate"));
    assert!(rendered.contains("value => join"));
    assert!(rendered.contains("_ => join"));
    assert!(rendered.contains("(0)"));
    assert!(!rendered.contains("LetCont"));
    assert!(!rendered.contains("AppCont"));
}

#[test]
fn branch_usage_traverses_condition_and_all_arms() {
    let output = compile_expr(&Expr::if_else(
        Expr::var("shared"),
        Expr::var("success_only"),
        Expr::call(Expr::var("shared"), Expr::var("else_arg")),
    ));

    assert_eq!(output.usage.vars.get("shared").copied(), Some(UseCount::Many));
    assert_eq!(
        output.usage.vars.get("success_only").copied(),
        Some(UseCount::One)
    );
    assert_eq!(
        output.usage.vars.get("else_arg").copied(),
        Some(UseCount::One)
    );
}

#[test]
fn visual_report_contains_all_initial_layers() {
    let output = compile_expr(&Expr::call(Expr::var("f"), Expr::var("x")));
    let report = output.render_visual();

    assert!(report.contains("source:"));
    assert!(report.contains("typed:"));
    assert!(report.contains("usage:"));
    assert!(report.contains("cps:"));
    assert!(report.contains("f(x,"));
}
