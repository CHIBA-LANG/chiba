use chiba_level1r::alpha::alpha_expr;
use chiba_level1r::ast::BinaryOp;
use chiba_level1r::resolve::{
    resolve_expr, resolve_expr_with_names, MethodCandidate, MethodIndex, NameIndex,
    ResolveDiagnostic, ResolvedCall, ResolvedName,
};
use chiba_level1r::{
    build_interface_summary, compile_expr, project_surface, DataDecl, DataVariant, Expr,
    MethodReceiver, NamespaceDecl, ParamDecl, SourceItem, SourceProgram, Visibility,
};

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
fn interface_summary_builds_method_index_for_method_style_defs() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::method_def(
            MethodReceiver::new("Box", vec!["T".to_string()]),
            "update",
            vec![
                ParamDecl::new("self", Some("Self".to_string())),
                ParamDecl::new("value", Some("T".to_string())),
            ],
            Some("Self".to_string()),
            Expr::var("self"),
        )],
    );
    let interface = build_interface_summary(&project_surface(&program));
    let methods = MethodIndex::from_interface(&interface);

    let qualified_alpha = alpha_expr(&Expr::method_call(
        Expr::var("Box[T]"),
        "update",
        Expr::var("value"),
    ));
    let qualified = resolve_expr_with_names(
        &qualified_alpha.expr,
        methods.clone(),
        NameIndex::from_interface(&interface),
    );
    assert_eq!(
        qualified.resolved_calls,
        vec![ResolvedCall::QualifiedCallee {
            path: "Box[T].update".to_string(),
            symbol: "parser::Box[T].update".to_string(),
        }]
    );

    let receiver_alpha = alpha_expr(&Expr::method_call(
        Expr::nominal("Box[T]", Expr::var("box_value")),
        "update",
        Expr::var("value"),
    ));
    let receiver = resolve_expr_with_names(
        &receiver_alpha.expr,
        methods,
        NameIndex::from_interface(&interface),
    );
    assert_eq!(
        receiver.resolved_calls,
        vec![ResolvedCall::ReceiverMethod {
            receiver: "Box[T]".to_string(),
            name: "update".to_string(),
            symbol: "parser::Box[T].update".to_string(),
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
        owner: "math".to_string(),
        visibility: Visibility::Public,
    });
    methods.add_candidate(MethodCandidate {
        receiver: "Vec2".to_string(),
        name: "show".to_string(),
        symbol: "debug.Vec2.show".to_string(),
        owner: "debug".to_string(),
        visibility: Visibility::Public,
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
fn interface_summary_resolves_global_function_owner_symbol() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "helper".to_string(),
            visibility: Visibility::Public,
            params: Vec::new(),
            return_type: None,
            body: Expr::i64(1),
        }],
    );
    let interface = build_interface_summary(&project_surface(&program));
    let alpha = alpha_expr(&Expr::var("helper"));

    let facts = resolve_expr_with_names(
        &alpha.expr,
        MethodIndex::default(),
        NameIndex::from_interface(&interface),
    );

    assert_eq!(facts.diagnostics, vec![]);
    assert_eq!(
        facts.resolved_names,
        vec![ResolvedName::Function {
            name: "helper".to_string(),
            symbol: "parser::helper".to_string(),
        }]
    );
}

#[test]
fn name_index_filters_private_functions_by_namespace_visibility() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["demo".to_string(), "math".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![
            SourceItem::def("public_value", Vec::new(), Vec::new(), None, Expr::i64(1)),
            SourceItem::def("hidden", Vec::new(), Vec::new(), None, Expr::i64(0))
                .with_visibility(Visibility::Private),
        ],
    );
    let interface = build_interface_summary(&project_surface(&program));
    let public_alpha = alpha_expr(&Expr::var("public_value"));
    let hidden_alpha = alpha_expr(&Expr::var("hidden"));

    let external_names = NameIndex::from_interface_for_namespace(&interface, "demo.app");
    let public_external = resolve_expr_with_names(
        &public_alpha.expr,
        MethodIndex::default(),
        external_names.clone(),
    );
    let hidden_external =
        resolve_expr_with_names(&hidden_alpha.expr, MethodIndex::default(), external_names);
    let hidden_internal = resolve_expr_with_names(
        &hidden_alpha.expr,
        MethodIndex::default(),
        NameIndex::from_interface_for_namespace(&interface, "demo.math"),
    );

    assert_eq!(public_external.diagnostics, vec![]);
    assert_eq!(
        public_external.resolved_names,
        vec![ResolvedName::Function {
            name: "public_value".to_string(),
            symbol: "demo.math::public_value".to_string(),
        }]
    );
    assert_eq!(hidden_external.diagnostics, vec![]);
    assert_eq!(hidden_external.resolved_names, vec![]);
    assert_eq!(
        hidden_internal.resolved_names,
        vec![ResolvedName::Function {
            name: "hidden".to_string(),
            symbol: "demo.math::hidden".to_string(),
        }]
    );
}

#[test]
fn name_index_resolves_static_values_with_namespace_visibility() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["demo".to_string(), "math".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![
            SourceItem::static_value("PUBLIC_CONST", Some("i64".to_string()), Expr::i64(1)),
            SourceItem::static_value("SECRET", Some("i64".to_string()), Expr::i64(0))
                .with_visibility(Visibility::Private),
        ],
    );
    let interface = build_interface_summary(&project_surface(&program));
    let public_alpha = alpha_expr(&Expr::var("PUBLIC_CONST"));
    let secret_alpha = alpha_expr(&Expr::var("SECRET"));

    let external_names = NameIndex::from_interface_for_namespace(&interface, "demo.app");
    let public_external = resolve_expr_with_names(
        &public_alpha.expr,
        MethodIndex::default(),
        external_names.clone(),
    );
    let secret_external =
        resolve_expr_with_names(&secret_alpha.expr, MethodIndex::default(), external_names);
    let secret_internal = resolve_expr_with_names(
        &secret_alpha.expr,
        MethodIndex::default(),
        NameIndex::from_interface_for_namespace(&interface, "demo.math"),
    );

    assert_eq!(public_external.diagnostics, vec![]);
    assert_eq!(
        public_external.resolved_names,
        vec![ResolvedName::Static {
            name: "PUBLIC_CONST".to_string(),
            symbol: "demo.math::PUBLIC_CONST".to_string(),
        }]
    );
    assert_eq!(secret_external.diagnostics, vec![]);
    assert_eq!(secret_external.resolved_names, vec![]);
    assert_eq!(
        secret_internal.resolved_names,
        vec![ResolvedName::Static {
            name: "SECRET".to_string(),
            symbol: "demo.math::SECRET".to_string(),
        }]
    );
}

#[test]
fn method_index_filters_private_methods_by_namespace_visibility() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["demo".to_string(), "math".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![
            SourceItem::method_def(
                MethodReceiver::new("Box", Vec::new()),
                "show",
                vec![ParamDecl::new("self", Some("Self".to_string()))],
                None,
                Expr::var("self"),
            ),
            SourceItem::method_def(
                MethodReceiver::new("Box", Vec::new()),
                "hidden",
                vec![ParamDecl::new("self", Some("Self".to_string()))],
                None,
                Expr::var("self"),
            )
            .with_visibility(Visibility::Private),
        ],
    );
    let interface = build_interface_summary(&project_surface(&program));
    let public_alpha = alpha_expr(&Expr::method_call(
        Expr::nominal("Box", Expr::var("box")),
        "show",
        Expr::i64(0),
    ));
    let hidden_alpha = alpha_expr(&Expr::method_call(
        Expr::nominal("Box", Expr::var("box")),
        "hidden",
        Expr::i64(0),
    ));

    let external_methods = MethodIndex::from_interface_for_namespace(&interface, "demo.app");
    let public_external = resolve_expr_with_names(
        &public_alpha.expr,
        external_methods.clone(),
        NameIndex::default(),
    );
    let hidden_external =
        resolve_expr_with_names(&hidden_alpha.expr, external_methods, NameIndex::default());
    let hidden_internal = resolve_expr_with_names(
        &hidden_alpha.expr,
        MethodIndex::from_interface_for_namespace(&interface, "demo.math"),
        NameIndex::default(),
    );

    assert_eq!(public_external.diagnostics, vec![]);
    assert_eq!(
        public_external.resolved_calls,
        vec![ResolvedCall::ReceiverMethod {
            receiver: "Box".to_string(),
            name: "show".to_string(),
            symbol: "demo.math::Box.show".to_string(),
        }]
    );
    assert_eq!(
        hidden_external.diagnostics,
        vec![ResolveDiagnostic::MissingMethod {
            receiver: Some("Box".to_string()),
            name: "hidden".to_string(),
        }]
    );
    assert_eq!(
        hidden_internal.resolved_calls,
        vec![ResolvedCall::ReceiverMethod {
            receiver: "Box".to_string(),
            name: "hidden".to_string(),
            symbol: "demo.math::Box.hidden".to_string(),
        }]
    );
}

#[test]
fn interface_summary_resolves_one_arg_function_call_by_arity() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "helper".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("x", Some("I64".to_string()))],
            return_type: Some("I64".to_string()),
            body: Expr::var("x"),
        }],
    );
    let interface = build_interface_summary(&project_surface(&program));
    let alpha = alpha_expr(&Expr::call(Expr::var("helper"), Expr::i64(1)));

    let facts = resolve_expr_with_names(
        &alpha.expr,
        MethodIndex::default(),
        NameIndex::from_interface(&interface),
    );

    assert_eq!(facts.diagnostics, vec![]);
    assert_eq!(
        facts.resolved_names,
        vec![ResolvedName::Function {
            name: "helper".to_string(),
            symbol: "parser::helper".to_string(),
        }]
    );
}

#[test]
fn interface_summary_reports_function_call_arity_mismatch() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "helper".to_string(),
            visibility: Visibility::Public,
            params: Vec::new(),
            return_type: Some("I64".to_string()),
            body: Expr::i64(1),
        }],
    );
    let interface = build_interface_summary(&project_surface(&program));
    let alpha = alpha_expr(&Expr::call(Expr::var("helper"), Expr::i64(1)));

    let facts = resolve_expr_with_names(
        &alpha.expr,
        MethodIndex::default(),
        NameIndex::from_interface(&interface),
    );

    assert_eq!(
        facts.diagnostics,
        vec![ResolveDiagnostic::FunctionArityMismatch {
            name: "helper".to_string(),
            symbol: "parser::helper".to_string(),
            expected: 0,
            actual: 1,
        }]
    );
    assert_eq!(facts.resolved_names, vec![]);
}

#[test]
fn interface_summary_resolves_qualified_constructor_owner_symbol_and_arity() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            vec!["T".to_string()],
            vec![
                DataVariant::new("Some", vec!["T".to_string()]),
                DataVariant::new("None", Vec::new()),
            ],
        )],
        Vec::new(),
    );
    let interface = build_interface_summary(&project_surface(&program));
    let alpha = alpha_expr(&Expr::adt_ctor(
        "Option",
        "Some",
        vec!["Some", "None"],
        vec![Expr::i64(1)],
    ));

    let facts = resolve_expr_with_names(
        &alpha.expr,
        MethodIndex::default(),
        NameIndex::from_interface(&interface),
    );

    assert_eq!(facts.diagnostics, vec![]);
    assert_eq!(
        facts.resolved_names,
        vec![ResolvedName::Constructor {
            data: "Option".to_string(),
            ctor: "Some".to_string(),
            symbol: "parser::Option.Some".to_string(),
            arity: 1,
        }]
    );
}

#[test]
fn interface_summary_does_not_resolve_local_binder_as_global_function() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "x".to_string(),
            visibility: Visibility::Public,
            params: Vec::new(),
            return_type: None,
            body: Expr::i64(1),
        }],
    );
    let interface = build_interface_summary(&project_surface(&program));
    let alpha = alpha_expr(&Expr::lambda("x", Expr::var("x")));

    let facts = resolve_expr_with_names(
        &alpha.expr,
        MethodIndex::default(),
        NameIndex::from_interface(&interface),
    );

    assert_eq!(facts.resolved_names, vec![]);
    assert_eq!(facts.diagnostics, vec![]);
}

#[test]
fn interface_summary_reports_constructor_arity_mismatch() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            Vec::new(),
            vec![DataVariant::new("Some", vec!["I64".to_string()])],
        )],
        Vec::new(),
    );
    let interface = build_interface_summary(&project_surface(&program));
    let alpha = alpha_expr(&Expr::adt_ctor(
        "Option",
        "Some",
        vec!["Some"],
        Vec::<Expr>::new(),
    ));

    let facts = resolve_expr_with_names(
        &alpha.expr,
        MethodIndex::default(),
        NameIndex::from_interface(&interface),
    );

    assert_eq!(
        facts.diagnostics,
        vec![ResolveDiagnostic::ConstructorArityMismatch {
            data: "Option".to_string(),
            ctor: "Some".to_string(),
            expected: 1,
            actual: 0,
        }]
    );
    assert_eq!(facts.resolved_names, vec![]);
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
    assert_eq!(obligation.op, chiba_level1r::resolve::OperatorSurface::Binary(BinaryOp::Add));
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
