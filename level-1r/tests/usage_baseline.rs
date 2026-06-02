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

