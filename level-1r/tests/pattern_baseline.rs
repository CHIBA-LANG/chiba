use chiba_level1r::ast::Pattern;
use chiba_level1r::pattern::PatternDiagnostic;
use chiba_level1r::typed::{type_expr_with_context, Type, TypeContext, TypeEnv};
use chiba_level1r::{
    compile_expr, compile_program_bundle, compile_source_program_bundle, Expr, Literal, ParamDecl,
    SourceItem, SourceProgram, TemplateParamSource, TypeDecl, TypeField, Visibility,
};

#[test]
fn bool_match_is_exhaustive_when_both_literals_are_covered() {
    let output = compile_expr(&Expr::match_expr(
        Expr::bool(true),
        vec![
            (Pattern::lit_bool(true), Expr::i64(1)),
            (Pattern::lit_bool(false), Expr::i64(0)),
        ],
    ));

    assert_eq!(output.pattern.diagnostics, vec![]);
    assert_eq!(output.pattern.matches.len(), 1);
    let fact = &output.pattern.matches[0];
    assert_eq!(fact.scrutinee_type, Type::Bool);
    assert_eq!(
        fact.covered_literals,
        vec![Literal::Bool(true), Literal::Bool(false)]
    );
    assert_eq!(fact.has_wildcard, false);
    assert_eq!(fact.exhaustive, true);
}

#[test]
fn wildcard_makes_match_exhaustive_for_open_domains() {
    let output = compile_expr(&Expr::match_expr(
        Expr::var("n"),
        vec![
            (Pattern::lit_i64(0), Expr::i64(10)),
            (Pattern::wildcard(), Expr::i64(20)),
        ],
    ));

    assert_eq!(output.pattern.diagnostics, vec![]);
    assert_eq!(output.pattern.matches[0].has_wildcard, true);
    assert_eq!(output.pattern.matches[0].exhaustive, true);
}

#[test]
fn bool_match_reports_missing_literal() {
    let output = compile_expr(&Expr::match_expr(
        Expr::bool(true),
        vec![(Pattern::lit_bool(true), Expr::i64(1))],
    ));

    assert_eq!(
        output.pattern.diagnostics,
        vec![PatternDiagnostic::NonExhaustiveMatch {
            scrutinee_type: Type::Bool,
            missing: vec![Pattern::lit_bool(false)],
        }]
    );
}

#[test]
fn pattern_visual_renders_structured_facts_without_rust_debug_shape() {
    let output = compile_expr(&Expr::match_expr(
        Expr::bool(true),
        vec![(Pattern::lit_bool(true), Expr::i64(1))],
    ));
    let visual = &output.visual.pattern;

    assert!(visual.contains("matches=1"));
    assert!(visual.contains(
        "match 0 scrutinee=bool literals=[true] constructors=[] wildcard=false exhaustive=false"
    ));
    assert!(visual.contains("diagnostics=1"));
    assert!(visual.contains("diagnostic non-exhaustive-match scrutinee=bool missing=[false]"));
    assert!(!visual.contains("PatternFacts"));
    assert!(!visual.contains("MatchExhaustivenessFact"));
    assert!(!visual.contains("PatternEnvFact"));
    assert!(!visual.contains("NonExhaustiveMatch"));
    assert!(!visual.contains("scrutinee_type"));
    assert!(!visual.contains("covered_literals"));
}

#[test]
fn non_bool_literal_match_without_wildcard_requires_fallback() {
    let output = compile_expr(&Expr::match_expr(
        Expr::var("tag"),
        vec![(Pattern::lit_i64(0), Expr::i64(1))],
    ));

    assert_eq!(
        output.pattern.diagnostics,
        vec![PatternDiagnostic::NonExhaustiveMatch {
            scrutinee_type: Type::Unknown,
            missing: vec![Pattern::wildcard()],
        }]
    );
}

#[test]
fn if_let_records_success_only_pattern_environment() {
    let output = compile_expr(&Expr::if_let(
        Pattern::bind("value"),
        Expr::var("candidate"),
        Expr::var("value"),
        Expr::i64(0),
    ));

    assert_eq!(output.pattern.envs.len(), 1);
    let env = &output.pattern.envs[0];
    assert_eq!(env.bindings, vec!["value".to_string()]);
    assert_eq!(env.success_branch_binds, true);
    assert_eq!(env.failure_branch_binds, false);
    assert!(output
        .visual
        .pattern
        .contains("env 0 bindings=[value] success-binds=true failure-binds=false"));
}

#[test]
fn tuple_pattern_collects_nested_bindings_in_source_order() {
    let output = compile_expr(&Expr::if_let(
        Pattern::tuple(vec![
            Pattern::bind("head"),
            Pattern::tuple(vec![Pattern::wildcard(), Pattern::bind("tail")]),
        ]),
        Expr::var("pair"),
        Expr::var("head"),
        Expr::i64(0),
    ));

    assert_eq!(output.pattern.envs.len(), 1);
    assert_eq!(
        output.pattern.envs[0].bindings,
        vec!["head".to_string(), "tail".to_string()]
    );
    assert_eq!(output.pattern.envs[0].failure_branch_binds, false);
    assert!(output.cps.to_string().contains("(head, (_, tail)) => join"));
}

#[test]
fn function_parameter_tuple_pattern_uses_tuple_type_arguments() {
    let program = SourceProgram::new(vec![SourceItem::Def {
        receiver: None,
        generics: Vec::new(),
        name: "first".to_string(),
        visibility: Visibility::Public,
        params: vec![ParamDecl::pattern(
            Pattern::tuple(vec![Pattern::bind("head"), Pattern::bind("tail")]),
            Some("Tuple[i64,bool]".to_string()),
        )],
        return_type: Some("i64".to_string()),
        body: Expr::var("head"),
    }]);

    let output = compile_program_bundle(&program);
    let first = &output.defs[0].output;

    assert_eq!(
        first.pattern.envs[0].bindings,
        vec!["head".to_string(), "tail".to_string()]
    );
    assert_eq!(first.typed.ty, Type::I64);
    assert_eq!(first.pattern.diagnostics, vec![]);
}

#[test]
fn tuple_pattern_does_not_infer_fields_from_nominal_string_shape() {
    let mut env = TypeEnv::new();
    env.insert(
        "pair".to_string(),
        Type::Nominal("Tuple[i64,bool]".to_string()),
    );

    let typed = type_expr_with_context(
        &Expr::if_let(
            Pattern::tuple(vec![Pattern::bind("head"), Pattern::bind("tail")]),
            Expr::var("pair"),
            Expr::var("head"),
            Expr::i64(0),
        ),
        &env,
        &TypeContext::new(),
    );

    match typed.kind {
        chiba_level1r::typed::TypedExprKind::IfLet { then_branch, .. } => {
            assert_eq!(then_branch.ty, Type::Unknown);
        }
        other => panic!("expected if-let typed expr, got {other:?}"),
    }
}

#[test]
fn tuple_pattern_reports_duplicate_binding_once() {
    let output = compile_expr(&Expr::if_let(
        Pattern::tuple(vec![
            Pattern::bind("x"),
            Pattern::tuple(vec![Pattern::bind("x"), Pattern::bind("y")]),
            Pattern::bind("x"),
        ]),
        Expr::var("pair"),
        Expr::var("y"),
        Expr::i64(0),
    ));

    assert_eq!(
        output.pattern.diagnostics,
        vec![PatternDiagnostic::DuplicateBinding {
            name: "x".to_string(),
        }]
    );
    assert_eq!(
        output.pattern.envs[0].bindings,
        vec![
            "x".to_string(),
            "x".to_string(),
            "y".to_string(),
            "x".to_string()
        ]
    );
}

#[test]
fn record_pattern_collects_bindings_by_field_pattern_order() {
    let output = compile_expr(&Expr::if_let(
        Pattern::record(vec![
            ("y", Pattern::bind("py")),
            (
                "x",
                Pattern::tuple(vec![Pattern::bind("px"), Pattern::wildcard()]),
            ),
        ]),
        Expr::var("point"),
        Expr::var("px"),
        Expr::i64(0),
    ));

    assert_eq!(
        output.pattern.envs[0].bindings,
        vec!["py".to_string(), "px".to_string()]
    );
    assert!(output
        .cps
        .to_string()
        .contains("{y: py, x: (px, _)} => join"));
}

#[test]
fn at_pattern_binds_inner_pattern_then_whole_value_alias() {
    let output = compile_expr(&Expr::if_let(
        Pattern::at(
            "whole",
            Pattern::record(vec![("x", Pattern::bind("inner"))]),
        ),
        Expr::var("point"),
        Expr::var("whole"),
        Expr::i64(0),
    ));

    assert_eq!(
        output.pattern.envs[0].bindings,
        vec!["inner".to_string(), "whole".to_string()]
    );
    assert!(output
        .cps
        .to_string()
        .contains("whole @ {x: inner} => join"));
}

#[test]
fn at_pattern_reports_duplicate_binding_against_inner_pattern() {
    let output = compile_expr(&Expr::if_let(
        Pattern::at("x", Pattern::record(vec![("x", Pattern::bind("x"))])),
        Expr::var("point"),
        Expr::var("x"),
        Expr::i64(0),
    ));

    assert_eq!(
        output.pattern.diagnostics,
        vec![PatternDiagnostic::DuplicateBinding {
            name: "x".to_string(),
        }]
    );
}

#[test]
fn chained_at_pattern_is_rejected_but_nested_at_pattern_is_allowed() {
    let chained = compile_expr(&Expr::if_let(
        Pattern::at("a", Pattern::at("b", Pattern::bind("x"))),
        Expr::var("value"),
        Expr::var("a"),
        Expr::i64(0),
    ));

    assert!(chained
        .pattern
        .diagnostics
        .contains(&PatternDiagnostic::ChainedAtPattern {
            name: "b".to_string(),
        }));

    let nested = compile_expr(&Expr::if_let(
        Pattern::record(vec![(
            "node",
            Pattern::at("inner", Pattern::record(vec![("leaf", Pattern::bind("v"))])),
        )]),
        Expr::var("tree"),
        Expr::var("inner"),
        Expr::i64(0),
    ));

    assert!(!nested
        .pattern
        .diagnostics
        .iter()
        .any(|diag| matches!(diag, PatternDiagnostic::ChainedAtPattern { .. })));
    assert_eq!(
        nested.pattern.envs[0].bindings,
        vec!["v".to_string(), "inner".to_string()]
    );
}

#[test]
fn function_parameter_pattern_records_entry_environment() {
    let output = compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def get(Some(x): Option[i64]): i64 = x",
    )
    .expect("compile source");

    let get = &output.program.defs[0].output;
    assert_eq!(get.pattern.envs.len(), 1);
    assert_eq!(get.pattern.envs[0].bindings, vec!["x".to_string()]);
    assert_eq!(get.typed.ty, chiba_level1r::typed::Type::I64);
    assert_eq!(get.pattern.diagnostics, vec![]);
}

#[test]
fn function_parameter_record_pattern_uses_nominal_row_field_types() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            Vec::new(),
            vec![TypeField::new("value", "i64")],
        )],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "get".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::pattern(
                Pattern::record(vec![("value", Pattern::bind("x"))]),
                Some("Box".to_string()),
            )],
            return_type: Some("i64".to_string()),
            body: Expr::var("x"),
        }],
    );

    let output = compile_program_bundle(&program);
    let get = &output.defs[0].output;

    assert_eq!(get.pattern.envs[0].bindings, vec!["x".to_string()]);
    assert_eq!(get.typed.ty, Type::I64);
    assert_eq!(get.pattern.diagnostics, vec![]);
}

#[test]
fn function_parameter_pattern_reports_duplicate_binding() {
    let output = compile_source_program_bundle("def bad((x, x)) = x").expect("compile source");

    let bad = &output.program.defs[0].output;
    assert_eq!(
        bad.pattern.diagnostics,
        vec![PatternDiagnostic::DuplicateBinding {
            name: "x".to_string(),
        }]
    );
    assert_eq!(
        bad.pattern.envs[0].bindings,
        vec!["x".to_string(), "x".to_string()]
    );
}

#[test]
fn wildcard_parameter_pattern_does_not_create_auto_generic_param() {
    let output = compile_source_program_bundle("def ignore(_) = 1").expect("compile source");

    let ignore = &output.program.defs[0].output;
    assert_eq!(ignore.pattern.envs, vec![]);
    assert!(!ignore.template.explicit_params.iter().any(|param| {
        param.source == TemplateParamSource::SyntheticAutoGeneric && param.name == "T__"
    }));
}
