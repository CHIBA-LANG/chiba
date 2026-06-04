use chiba_level1r::closure::ClosureStorageKind;
use chiba_level1r::control::ContinuationKind;
use chiba_level1r::core::{CoreOp, LayoutKind};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn no_capture_lambda_is_direct_no_capture() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert_eq!(output.closure.closures.len(), 1);
    assert_eq!(
        output.closure.closures[0].storage,
        ClosureStorageKind::DirectNoCapture
    );
    assert_eq!(output.closure.closures[0].captures, vec![]);
}

#[test]
fn nested_lambda_captures_outer_binder_with_stable_env_field() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));

    assert_eq!(output.closure.closures.len(), 2);
    let inner = output
        .closure
        .closures
        .iter()
        .find(|closure| closure.param == "y")
        .unwrap();
    assert_eq!(inner.storage, ClosureStorageKind::EnvClosure);
    assert_eq!(inner.captures.len(), 1);
    assert_eq!(inner.captures[0].name, "x");

    let layout = output
        .core
        .layouts
        .iter()
        .find(|layout| layout.key == "closure-env::closure::y")
        .unwrap();
    match &layout.kind {
        LayoutKind::ClosureEnv(env) => {
            assert_eq!(env.closure, "closure::y");
            assert_eq!(env.fields.len(), 1);
            assert_eq!(env.fields[0].name, "x");
        }
        other => panic!("expected closure env layout, got {other:?}"),
    }
    assert_eq!(output.core_validation.diagnostics, vec![]);
}

#[test]
fn closure_visual_renders_structured_facts_without_rust_debug_shape() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));
    let visual = &output.visual.closure;

    assert!(visual.contains("closures=2"));
    assert!(visual.contains("closure 0 param=x storage=direct-no-capture captures=0"));
    assert!(visual.contains("closure 1 param=y storage=env-closure captures=1"));
    assert!(visual.contains("closure 1 capture x binder=%0 usage=N"));
    assert!(!visual.contains("ClosureFacts"));
    assert!(!visual.contains("ClosureFact"));
    assert!(!visual.contains("CaptureFact"));
    assert!(!visual.contains("DirectNoCapture"));
    assert!(!visual.contains("EnvClosure"));
}

#[test]
fn no_capture_lambda_does_not_allocate_closure_env_layout() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert!(!output
        .core
        .layouts
        .iter()
        .any(|layout| matches!(&layout.kind, LayoutKind::ClosureEnv(_))));
}

#[test]
fn core_ir_preserves_continuation_kind_without_backend_terms() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    assert!(output.core.ops.contains(&CoreOp::Prompt {
        kind: ContinuationKind::ContN
    }));
    assert!(output.core.ops.iter().any(|op| matches!(
        op,
        CoreOp::CaptureContinuation {
            binder,
            kind: ContinuationKind::ContN,
            ..
        } if binder == "retry"
    )));

    let visual = &output.visual.core;
    assert!(visual.contains("prompt kind=contn"));
    assert!(visual.contains("capture-continuation retry kind=contn"));
    assert!(!visual.contains("target-specific-term"));
    assert!(!visual.contains("Wasm"));
    assert!(!visual.contains("WAT"));
    assert!(!visual.contains("Binaryen"));
    assert!(!visual.contains("funcref"));
    assert!(!visual.contains("eqref"));
}
