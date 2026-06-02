use chiba_level1r::alpha::{alpha_expr, AlphaDiagnostic, AlphaExprKind};
use chiba_level1r::closure::ClosureStorageKind;
use chiba_level1r::usage::UseCount;
use chiba_level1r::{compile_expr, Expr};

#[test]
fn local_shadowing_gets_distinct_stable_binder_ids() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("x", Expr::call(Expr::var("x"), Expr::var("x"))),
    ));

    assert_eq!(output.alpha.binders.len(), 2);
    assert_ne!(output.alpha.binders[0].id, output.alpha.binders[1].id);
    assert_eq!(output.alpha.binders[0].name, "x");
    assert_eq!(output.alpha.binders[1].name, "x");
    assert_eq!(
        output.usage.binders.get(&output.alpha.binders[0].id).copied(),
        None
    );
    assert_eq!(
        output.usage.binders.get(&output.alpha.binders[1].id).copied(),
        Some(UseCount::Many)
    );
}

#[test]
fn nested_lambda_capture_targets_outer_binder_id_not_name_string() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));

    let outer = output.alpha.binders[0].id;
    let inner = output
        .closure
        .closures
        .iter()
        .find(|closure| closure.param == "y")
        .unwrap();

    assert_eq!(inner.storage, ClosureStorageKind::EnvClosure);
    assert_eq!(inner.captures.len(), 1);
    assert_eq!(inner.captures[0].name, "x");
    assert_eq!(inner.captures[0].binder, Some(outer));
}

#[test]
fn undefined_variable_gets_diagnostic_without_fake_binder() {
    let facts = alpha_expr(&Expr::call(Expr::var("missing"), Expr::i64(1)));

    assert_eq!(
        facts.diagnostics,
        vec![AlphaDiagnostic::UndefinedVar {
            name: "missing".to_string()
        }]
    );
    match &facts.expr.kind {
        AlphaExprKind::Call { callee, .. } => match &callee.kind {
            AlphaExprKind::Var(var) => assert_eq!(var.target, None),
            other => panic!("expected unresolved var, got {other:?}"),
        },
        other => panic!("expected call, got {other:?}"),
    }
}

#[test]
fn shift_binder_scope_is_limited_to_shift_body() {
    let facts = alpha_expr(&Expr::call(
        Expr::reset(Expr::shift("k", Expr::var("k"))),
        Expr::var("k"),
    ));

    assert_eq!(
        facts.diagnostics,
        vec![AlphaDiagnostic::UndefinedVar {
            name: "k".to_string()
        }]
    );
    assert_eq!(facts.binders.len(), 1);
}

#[test]
fn visual_report_contains_alpha_layer_and_pass_event() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));
    let visual = output.render_visual();

    assert!(visual.contains("alpha:"));
    assert!(visual.contains("L1Alpha: SourceExpr -> AlphaFacts"));
}
