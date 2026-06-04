use chiba_level1r::closure_core_usage::analyze_closure_core_usage;
use chiba_level1r::control::ContinuationKind;
use chiba_level1r::typed::{SendColor, UsageColor};
use chiba_level1r::usage::UseCount;
use chiba_level1r::{compile_expr, Expr};

#[test]
fn closure_core_usage_tracks_env_fields_and_closure_packages() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));

    let package = output
        .closure_core_usage
        .closure_packages
        .get("closure::y")
        .unwrap();
    assert_eq!(package.count, UseCount::Many);
    assert_eq!(package.send, SendColor::Obligation);

    let field = output
        .closure_core_usage
        .env_fields
        .get("closure::y.x")
        .unwrap();
    assert_eq!(field.closure, "closure::y");
    assert_eq!(field.field, "x");
    assert_eq!(field.color, UsageColor::Many);
}

#[test]
fn closure_core_usage_tracks_lifted_code_pointer_directness() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));

    let outer = output
        .closure_core_usage
        .code_pointers
        .get("lift::0000::closure__x")
        .unwrap();
    assert_eq!(outer.source, "closure::x");
    assert_eq!(outer.count, UseCount::One);
    assert!(outer.known_direct);

    let inner = output
        .closure_core_usage
        .code_pointers
        .get("lift::0001::closure__y")
        .unwrap();
    assert_eq!(inner.source, "closure::y");
    assert_eq!(inner.count, UseCount::Many);
    assert!(!inner.known_direct);
}

#[test]
fn closure_core_usage_tracks_continuation_packages() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    let package = output
        .closure_core_usage
        .continuation_packages
        .get("continuation::contn::retry")
        .unwrap();
    assert_eq!(package.kind, ContinuationKind::ContN);
    assert_eq!(package.count, UseCount::Many);
}

#[test]
fn closure_core_usage_visual_dump_is_present() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));
    let direct = analyze_closure_core_usage(&output.core);

    assert_eq!(direct, output.closure_core_usage);
    let visual = output.render_visual();
    assert!(visual.contains("closure-core-usage:"));
    assert!(visual.contains("code-pointers=1"));
    assert!(visual
        .contains("code-pointer lift::0000::closure__x source=closure::x count=1 direct=true"));
    assert!(!output
        .visual
        .closure_core_usage
        .contains("ClosureCoreUsageFacts"));
    assert!(!output.visual.closure_core_usage.contains("code_pointers"));
}
