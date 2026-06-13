use chiba_level1r::ast::BinaryOp;
use chiba_level1r::template::{
    canonical_open_row, dyn_row_contract, ShapeType, TemplateDiagnostic, TemplateInstantiation,
    TemplateObligation, TemplateParam, TemplateParamSource,
};
use chiba_level1r::typed::{SendColor, Type, TypedExprKind, UsageColor};
use chiba_level1r::{compile_expr, Expr, ProgramDiagnostic};

#[test]
fn row_shape_canonicalization_ignores_source_field_order() {
    let xy = canonical_open_row(vec![
        ("x", ShapeType::Named("i64".to_string())),
        ("y", ShapeType::Named("Bool".to_string())),
    ]);
    let yx = canonical_open_row(vec![
        ("y", ShapeType::Named("Bool".to_string())),
        ("x", ShapeType::Named("i64".to_string())),
    ]);

    assert_eq!(xy, yx);
    assert_eq!(xy.fields[0].name, "x");
    assert_eq!(xy.fields[1].name, "y");
}

#[test]
fn field_access_generates_open_row_template_obligation() {
    let output = compile_expr(&Expr::field(Expr::var("v"), "name"));

    assert_eq!(output.template.row_shapes.len(), 1);
    assert_eq!(
        output.template.obligations,
        vec![TemplateObligation::Field {
            shape: canonical_open_row(vec![("name", ShapeType::Unknown)]),
            field: "name".to_string(),
        }]
    );
}

#[test]
fn operator_resolve_fact_enters_template_obligations() {
    let output = compile_expr(&Expr::binary(
        BinaryOp::Add,
        Expr::nominal("Vec2", Expr::var("a")),
        Expr::nominal("Vec2", Expr::var("b")),
    ));

    assert!(output
        .template
        .obligations
        .contains(&TemplateObligation::Operator {
            op: chiba_level1r::resolve::OperatorSurface::Binary(BinaryOp::Add),
            protocol: "op_add".to_string(),
            receiver: Some("Vec2".to_string()),
        }));
}

#[test]
fn row_field_obligation_allows_deferred_callable_field_without_selecting_nominal_method() {
    let output = compile_expr(&Expr::method_call(
        Expr::field(Expr::var("row"), "field_fn"),
        "len",
        Expr::i64(0),
    ));

    assert!(output
        .template
        .obligations
        .contains(&TemplateObligation::Field {
            shape: canonical_open_row(vec![("field_fn", ShapeType::Unknown)]),
            field: "field_fn".to_string(),
        }));
    assert!(output
        .template
        .obligations
        .contains(&TemplateObligation::Field {
            shape: canonical_open_row(vec![("len", ShapeType::Unknown)]),
            field: "len".to_string(),
        }));
    assert_eq!(output.resolve.diagnostics, vec![]);
}

#[test]
fn dyn_row_contract_carries_adapter_and_colors() {
    let contract = dyn_row_contract(vec![("name", ShapeType::Named("String".to_string()))]);

    assert_eq!(
        contract.shape,
        canonical_open_row(vec![("name", ShapeType::Named("String".to_string()))])
    );
    assert_eq!(contract.payload_usage, UsageColor::Many);
    assert_eq!(contract.send, SendColor::Obligation);
}

#[test]
fn visual_report_contains_template_layer() {
    let output = compile_expr(&Expr::field(Expr::var("v"), "name"));
    let visual = output.render_visual();

    assert!(visual.contains("template:"));
    assert!(visual.contains("obligation field name in {r | name: _}"));
    assert!(!visual.contains("TemplateFacts {"));
    assert!(visual.contains("L3Template: AlphaExpr+ResolveFacts -> TemplateFacts"));
}

#[test]
fn explicit_call_site_instantiation_enters_template_facts() {
    let output = compile_expr(&Expr::call_args(
        Expr::instantiate(Expr::var("id"), vec!["T".to_string()]),
        vec![Expr::var("value")],
    ));

    assert_eq!(
        output.template.explicit_instantiations,
        vec![TemplateInstantiation {
            callee: "id".to_string(),
            type_args: vec!["T".to_string()],
        }]
    );
    let template_visual = &output.visual.template;
    assert!(template_visual.contains("instantiate id[T]"));
    assert!(!template_visual.contains("explicit_instantiations"));
}

#[test]
fn explicit_instantiation_callee_name_is_source_facing_for_call_expr() {
    let output = compile_expr(&Expr::instantiate(
        Expr::call_args(Expr::var("factory"), Vec::new()),
        vec!["T".to_string()],
    ));

    assert_eq!(
        output.template.explicit_instantiations,
        vec![TemplateInstantiation {
            callee: "factory()".to_string(),
            type_args: vec!["T".to_string()],
        }]
    );
    let template_visual = &output.visual.template;
    assert!(template_visual.contains("instantiate factory()[T]"));
    assert!(!template_visual.contains("Call {"));
    assert!(!template_visual.contains("Var("));
}

#[test]
fn conflicting_explicit_call_site_instantiations_are_diagnostic_facts() {
    let output = compile_expr(&Expr::tuple(vec![
        Expr::instantiate(Expr::var("id"), vec!["I64".to_string()]),
        Expr::instantiate(Expr::var("id"), vec!["Bool".to_string()]),
    ]));

    assert_eq!(
        output.template.diagnostics,
        vec![TemplateDiagnostic::ConflictingExplicitInstantiation {
            callee: "id".to_string(),
            previous_type_args: vec!["I64".to_string()],
            type_args: vec!["Bool".to_string()],
        }]
    );
}

#[test]
fn explicit_template_header_enters_program_template_facts() {
    let output = chiba_level1r::compile_source_program_bundle("def id[T](x: T): T = x")
        .expect("compile source");
    let def = &output.program.defs[0].output;

    assert_eq!(
        def.template.explicit_params,
        vec![TemplateParam {
            name: "T".to_string(),
            source: TemplateParamSource::ExplicitHeader,
        }]
    );
    let visual = def.render_visual();
    assert!(visual.contains("param T source=explicit-header"));
    assert!(!visual.contains("explicit_params"));
}

#[test]
fn explicit_header_conflicting_with_auto_generic_is_diagnostic_fact() {
    let output =
        chiba_level1r::compile_source_program_bundle("def id[T_x](x) = x").expect("compile source");
    let def = &output.program.defs[0].output;

    assert!(def.template.explicit_params.contains(&TemplateParam {
        name: "T_x".to_string(),
        source: TemplateParamSource::ExplicitHeader,
    }));
    assert!(def.template.explicit_params.contains(&TemplateParam {
        name: "T_x".to_string(),
        source: TemplateParamSource::SyntheticAutoGeneric,
    }));
    assert_eq!(
        def.template.diagnostics,
        vec![TemplateDiagnostic::ExplicitAutoGenericConflict {
            param: "T_x".to_string(),
        }]
    );
}

#[test]
fn explicit_header_conflicting_with_auto_generic_reaches_program_diagnostic() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def id[T_x](x) = x
def main() = 0",
    )
    .expect("compile source");

    assert!(output.program.diagnostics.contains(
        &chiba_level1r::ProgramDiagnostic::ExplicitAutoGenericConflict {
            def: "id".to_string(),
            param: "T_x".to_string(),
        }
    ));
}

#[test]
fn conflicting_explicit_call_site_instantiations_reach_program_diagnostic() {
    let output =
        chiba_level1r::compile_source_program_bundle("def main() = (id[i64](1), id[Bool](true))")
            .expect("compile source");

    assert!(output.program.diagnostics.contains(
        &chiba_level1r::ProgramDiagnostic::ConflictingExplicitInstantiation {
            def: "main".to_string(),
            callee: "id".to_string(),
            previous_type_args: vec!["i64".to_string()],
            type_args: vec!["Bool".to_string()],
        }
    ));
}

#[test]
fn unannotated_identity_function_promotes_boundary_to_auto_generic() {
    let output =
        chiba_level1r::compile_source_program_bundle("def id(x) = x").expect("compile source");
    let def = &output.program.defs[0].output;

    assert_eq!(
        def.template.explicit_params,
        vec![TemplateParam {
            name: "T_x".to_string(),
            source: TemplateParamSource::SyntheticAutoGeneric,
        }]
    );
    assert_eq!(
        def.specialize.work_items[0].key.template_params,
        def.template.explicit_params
    );
    let template_visual = &def.visual.template;
    assert!(template_visual.contains("param T_x source=synthetic-auto-generic"));
    assert!(!template_visual.contains("SyntheticAutoGeneric"));
}

#[test]
fn unannotated_row_field_function_keeps_auto_generic_and_row_obligation() {
    let output = chiba_level1r::compile_source_program_bundle("def get_name(x) = x.name")
        .expect("compile source");
    let def = &output.program.defs[0].output;

    assert!(def.template.explicit_params.contains(&TemplateParam {
        name: "T_x".to_string(),
        source: TemplateParamSource::SyntheticAutoGeneric,
    }));
    assert!(def.template.explicit_params.contains(&TemplateParam {
        name: "T_return".to_string(),
        source: TemplateParamSource::SyntheticAutoGeneric,
    }));
    assert!(def
        .template
        .obligations
        .contains(&TemplateObligation::Field {
            shape: canonical_open_row(vec![("name", ShapeType::Unknown)]),
            field: "name".to_string(),
        }));
}

#[test]
fn unannotated_row_callable_function_keeps_dependent_field_obligation() {
    let output = chiba_level1r::compile_source_program_bundle("def call_f(v) = v.f(1)")
        .expect("compile source");
    let def = &output.program.defs[0].output;

    assert!(def.template.explicit_params.contains(&TemplateParam {
        name: "T_v".to_string(),
        source: TemplateParamSource::SyntheticAutoGeneric,
    }));
    assert!(def.template.explicit_params.contains(&TemplateParam {
        name: "T_return".to_string(),
        source: TemplateParamSource::SyntheticAutoGeneric,
    }));
    assert!(def
        .template
        .obligations
        .contains(&TemplateObligation::Field {
            shape: canonical_open_row(vec![("f", ShapeType::Unknown)]),
            field: "f".to_string(),
        }));
    assert_eq!(def.resolve.diagnostics, vec![]);
    assert_eq!(def.typed.ty, Type::Unknown);
    assert!(matches!(def.typed.kind, TypedExprKind::Call { .. }));
}

#[test]
fn annotated_function_boundary_does_not_create_auto_generic_params() {
    let output = chiba_level1r::compile_source_program_bundle("def id(x: I64): I64 = x")
        .expect("compile source");
    let def = &output.program.defs[0].output;

    assert!(def.template.explicit_params.is_empty());
}

#[test]
fn open_row_parameter_annotation_enters_template_field_obligations() {
    let output =
        chiba_level1r::compile_source_program_bundle("def get_x(v: {r | x: i64}): i64 = v.x")
            .expect("compile source");
    let def = &output.program.defs[0].output;

    assert!(def.template.explicit_params.is_empty());
    assert!(def.template.row_shapes.contains(&canonical_open_row(vec![(
        "x",
        ShapeType::Named("i64".to_string()),
    )])));
    assert!(def
        .template
        .obligations
        .contains(&TemplateObligation::Field {
            shape: canonical_open_row(vec![("x", ShapeType::Named("i64".to_string()))]),
            field: "x".to_string(),
        }));
    assert_eq!(def.typed.ty, Type::I64);
}

#[test]
fn open_row_generic_bound_enters_template_member_obligations() {
    let output =
        chiba_level1r::compile_source_program_bundle("def get_f[F, T: {r | f: F}](v: T): F = v.f")
            .expect("compile source");
    let def = &output.program.defs[0].output;

    assert!(def.template.explicit_params.contains(&TemplateParam {
        name: "F".to_string(),
        source: TemplateParamSource::ExplicitHeader,
    }));
    assert!(def.template.explicit_params.contains(&TemplateParam {
        name: "T".to_string(),
        source: TemplateParamSource::ExplicitHeader,
    }));
    assert!(!def.template.explicit_params.contains(&TemplateParam {
        name: "T_v".to_string(),
        source: TemplateParamSource::SyntheticAutoGeneric,
    }));
    assert!(def.template.row_shapes.contains(&canonical_open_row(vec![(
        "f",
        ShapeType::Named("F".to_string()),
    )])));
    assert!(def
        .template
        .obligations
        .contains(&TemplateObligation::Field {
            shape: canonical_open_row(vec![("f", ShapeType::Named("F".to_string()))]),
            field: "f".to_string(),
        }));
}

#[test]
fn generic_body_return_mismatch_is_definition_time_diagnostic() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def bad[T](x: T): Bool = 1
def main(): i64 = 0",
    )
    .expect("compile source");

    assert!(output.program.diagnostics.contains(
        &ProgramDiagnostic::DefinitionReturnTypeMismatch {
            def: "bad".to_string(),
            expected: "bool".to_string(),
            actual: "i64".to_string(),
        }
    ));
    assert!(output
        .program
        .render_summary()
        .contains("definition return type mismatch bad: expected bool got i64"));
}

#[test]
fn explicit_instantiation_arity_mismatch_is_program_diagnostic() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def pair[T, F](x: T, y: F): T = x
def main(): i64 = pair[i64](1, true)",
    )
    .expect("compile source");

    assert!(output.program.diagnostics.contains(
        &ProgramDiagnostic::ExplicitInstantiationArityMismatch {
            def: "main".to_string(),
            callee: "pair".to_string(),
            expected: 2,
            actual: 1,
        }
    ));
    assert!(output
        .program
        .render_summary()
        .contains("explicit instantiation arity mismatch main: pair expected 2 got 1"));
}

#[test]
fn explicit_instantiation_of_non_template_is_program_diagnostic() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def one(): i64 = 1
def main(): i64 = one[i64]()",
    )
    .expect("compile source");

    assert!(output.program.diagnostics.contains(
        &ProgramDiagnostic::ExplicitInstantiationOfNonTemplate {
            def: "main".to_string(),
            callee: "one".to_string(),
        }
    ));
    assert!(output
        .program
        .render_summary()
        .contains("explicit instantiation of non-template main: one"));
}

#[test]
fn explicit_instantiation_with_matching_template_arity_has_no_arity_diagnostic() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def id[T](x: T): T = x
def main(): i64 = id[i64](1)",
    )
    .expect("compile source");

    assert!(
        !output.program.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic,
            ProgramDiagnostic::ExplicitInstantiationArityMismatch { .. }
                | ProgramDiagnostic::ExplicitInstantiationOfNonTemplate { .. }
        ))
    );
}
