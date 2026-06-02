use chiba_level1r::alpha::alpha_expr;
use chiba_level1r::ast::BinaryOp;
use chiba_level1r::resolve::{
    resolve_expr, MethodCandidate, MethodIndex, ResolveDiagnostic, ResolvedCall,
};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn nominal_receiver_method_resolves_to_receiver_method_symbol() {
    let expr = Expr::method_call(Expr::nominal("Vec2", Expr::var("v")), "show", Expr::i64(0));
    let alpha = alpha_expr(&expr);
    let mut methods = MethodIndex::default();
    methods.add_method("Vec2", "show");

    let facts = resolve_expr(&alpha.expr, methods);

    assert_eq!(facts.diagnostics, vec![]);
    assert_eq!(
        facts.resolved_calls,
        vec![ResolvedCall::ReceiverMethod {
            receiver: "Vec2".to_string(),
            name: "show".to_string(),
            symbol: "Vec2.show".to_string(),
        }]
    );
}

#[test]
fn qualified_callee_resolves_without_receiver_injection() {
    let expr = Expr::method_call(Expr::var("Vec2"), "origin", Expr::i64(0));
    let alpha = alpha_expr(&expr);
    let mut methods = MethodIndex::default();
    methods.add_method("Vec2", "origin");

    let facts = resolve_expr(&alpha.expr, methods);

    assert_eq!(
        facts.resolved_calls,
        vec![ResolvedCall::QualifiedCallee {
            path: "Vec2.origin".to_string(),
            symbol: "Vec2.origin".to_string(),
        }]
    );
}

#[test]
fn duplicate_nominal_method_candidates_are_ambiguous_not_order_dependent() {
    let expr = Expr::method_call(Expr::nominal("Vec2", Expr::var("v")), "show", Expr::i64(0));
    let alpha = alpha_expr(&expr);
    let mut methods = MethodIndex::default();
    methods.add_candidate(MethodCandidate {
        receiver: "Vec2".to_string(),
        name: "show".to_string(),
        symbol: "math.Vec2.show".to_string(),
    });
    methods.add_candidate(MethodCandidate {
        receiver: "Vec2".to_string(),
        name: "show".to_string(),
        symbol: "debug.Vec2.show".to_string(),
    });

    let facts = resolve_expr(&alpha.expr, methods);

    assert_eq!(
        facts.diagnostics,
        vec![ResolveDiagnostic::AmbiguousMethod {
            receiver: "Vec2".to_string(),
            name: "show".to_string(),
            candidates: vec!["math.Vec2.show".to_string(), "debug.Vec2.show".to_string()],
        }]
    );
}

#[test]
fn binary_operator_records_protocol_obligation_without_numeric_default() {
    let expr = Expr::binary(
        BinaryOp::Add,
        Expr::nominal("Vec2", Expr::var("a")),
        Expr::nominal("Vec2", Expr::var("b")),
    );
    let output = compile_expr(&expr);

    assert_eq!(output.resolve.operator_obligations.len(), 1);
    let obligation = &output.resolve.operator_obligations[0];
    assert_eq!(obligation.op, BinaryOp::Add);
    assert_eq!(obligation.protocol, "op_add");
    assert_eq!(obligation.receiver.as_deref(), Some("Vec2"));
}

#[test]
fn visual_report_contains_resolve_layer() {
    let output = compile_expr(&Expr::binary(BinaryOp::Mul, Expr::var("a"), Expr::var("b")));
    let visual = output.render_visual();

    assert!(visual.contains("resolve:"));
    assert!(visual.contains("L2Resolve: AlphaExpr -> ResolveFacts"));
}
