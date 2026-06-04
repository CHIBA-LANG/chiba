use chiba_level1r::typed::UsageColor;
use chiba_level1r::usage::UseCount;
use chiba_level1r::{compile_expr, Expr};

#[test]
fn repeated_variable_is_many() {
    let expr = Expr::call(Expr::var("x"), Expr::var("x"));
    let output = compile_expr(&expr);

    assert_eq!(output.usage.vars.get("x").copied(), Some(UseCount::Many));
    assert_eq!(UseCount::Many.color(), UsageColor::Many);
}

#[test]
fn single_variable_is_one() {
    let output = compile_expr(&Expr::var("x"));

    assert_eq!(output.usage.vars.get("x").copied(), Some(UseCount::One));
    assert_eq!(UseCount::One.color(), UsageColor::One);
}

#[test]
fn method_call_usage_visits_receiver_and_all_arguments() {
    let output = compile_expr(&Expr::method_call_args(
        Expr::var("receiver"),
        "put",
        vec![Expr::var("key"), Expr::var("value")],
    ));

    assert_eq!(
        output.usage.vars.get("receiver").copied(),
        Some(UseCount::One)
    );
    assert_eq!(output.usage.vars.get("key").copied(), Some(UseCount::One));
    assert_eq!(output.usage.vars.get("value").copied(), Some(UseCount::One));
}

#[test]
fn usage_visual_dump_renders_counts_structurally() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::call(
            Expr::var("free"),
            Expr::call(Expr::var("x"), Expr::var("x")),
        ),
    ));

    let visual = output.render_visual();
    assert!(visual.contains("usage:"));
    assert!(visual.contains("var free count=1 color=1"));
    assert!(visual.contains("binder %0 count=many color=N"));
    assert!(!output.visual.usage.contains("UsageFacts"));
    assert!(!output.visual.usage.contains("vars:"));
}
