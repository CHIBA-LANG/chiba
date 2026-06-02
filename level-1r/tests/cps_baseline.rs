use chiba_level1r::cps::{cps_program, CpsAtom, CpsTerm};
use chiba_level1r::typed::type_expr;
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
fn visual_report_contains_all_initial_layers() {
    let output = compile_expr(&Expr::call(Expr::var("f"), Expr::var("x")));
    let report = output.render_visual();

    assert!(report.contains("source:"));
    assert!(report.contains("typed:"));
    assert!(report.contains("usage:"));
    assert!(report.contains("cps:"));
    assert!(report.contains("f(x,"));
}
