use chiba_level1r::closure::ClosureStorageKind;
use chiba_level1r::control::ContinuationKind;
use chiba_level1r::core::CoreOp;
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
}

#[test]
fn core_ir_preserves_continuation_kind_without_backend_terms() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    assert!(output.core.ops.contains(&CoreOp::Prompt {
        kind: ContinuationKind::ContN
    }));
    assert!(output.core.ops.contains(&CoreOp::CaptureContinuation {
        binder: "retry".to_string(),
        kind: ContinuationKind::ContN
    }));

    let rendered = format!("{:#?}", output.core);
    assert!(!rendered.contains("Wasm"));
    assert!(!rendered.contains("WAT"));
    assert!(!rendered.contains("Binaryen"));
    assert!(!rendered.contains("funcref"));
    assert!(!rendered.contains("eqref"));
}
