use chiba_level1r::{compile_expr, Expr};

#[test]
fn no_capture_lambda_lifts_to_direct_function_symbol() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert_eq!(output.lambda_lift.functions.len(), 1);
    let lifted = &output.lambda_lift.functions[0];
    assert_eq!(lifted.source, "closure::x");
    assert_eq!(lifted.symbol, "lift::0000::closure__x");
    assert_eq!(lifted.env_params, Vec::<String>::new());
    assert!(lifted.direct);
}

#[test]
fn captured_lambda_lifts_with_stable_env_params() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));

    assert_eq!(output.lambda_lift.functions.len(), 2);
    assert_eq!(output.lambda_lift.functions[0].source, "closure::x");
    assert_eq!(
        output.lambda_lift.functions[0].symbol,
        "lift::0000::closure__x"
    );
    assert_eq!(
        output.lambda_lift.functions[0].env_params,
        Vec::<String>::new()
    );
    assert!(output.lambda_lift.functions[0].direct);

    let inner = &output.lambda_lift.functions[1];
    assert_eq!(inner.source, "closure::y");
    assert_eq!(inner.symbol, "lift::0001::closure__y");
    assert_eq!(inner.env_params, vec!["x".to_string()]);
    assert!(!inner.direct);
}

#[test]
fn visual_report_shows_lambda_lift_symbols() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));
    let visual = output.render_visual();

    assert!(visual.contains("lambda-lift:"));
    assert!(visual.contains("lift::0000::closure__x"));
    assert!(visual.contains("lift::0001::closure__y"));
    assert!(visual.contains("env_params"));
}
