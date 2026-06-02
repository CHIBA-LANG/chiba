use chiba_level1r::ast::{
    DataDecl, DataVariant, MethodReceiver, NamespaceDecl, ParamDecl, SourceItem, SourceProgram,
    TypeDecl, TypeField, UseDecl, Visibility,
};
use chiba_level1r::typed::{Type, TypedExprKind};
use chiba_level1r::{
    build_interface_summary, compile_program, compile_program_bundle, project_surface_many, Expr,
    ProgramDiagnostic,
};
use std::process::Command;

fn def(name: &str, params: Vec<&str>, body: Expr) -> SourceItem {
    SourceItem::Def {
        receiver: None,
        generics: Vec::new(),
        name: name.to_string(),
        visibility: Visibility::Public,
        params: params.into_iter().map(ParamDecl::untyped).collect(),
        return_type: None,
        body,
    }
}

fn static_value(name: &str, ty: Option<&str>, body: Expr) -> SourceItem {
    SourceItem::StaticValue {
        name: name.to_string(),
        ty: ty.map(str::to_string),
        visibility: Visibility::Public,
        body,
    }
}

#[test]
fn compile_program_keeps_legacy_per_def_outputs() {
    let program = SourceProgram::new(vec![
        def("one", vec![], Expr::i64(1)),
        def("two", vec![], Expr::i64(2)),
    ]);

    let outputs = compile_program(&program);

    assert_eq!(outputs.len(), 2);
    assert!(outputs[0].backend.wat.contains("i32.const 1"));
    assert!(outputs[1].backend.wat.contains("i32.const 2"));
}

#[test]
fn program_bundle_selects_main_and_links_def_artifacts() {
    let program = SourceProgram::new(vec![
        def("helper", vec![], Expr::i64(1)),
        def("main", vec![], Expr::i64(7)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.entry, Some("main".to_string()));
    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(bundle.defs.len(), 2);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert!(bundle.backend_link.linked_wat.contains("(func $helper (result i32)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main__def1 (export \"main\") (result i32)"));
    assert!(bundle.backend_link.linked_wat.contains("i32.const 1"));
    assert!(bundle.backend_link.linked_wat.contains("i32.const 7"));
}

#[test]
fn program_branch_return_lowers_to_executable_wat_if() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::if_else(Expr::bool(true), Expr::i64(1), Expr::i64(2)),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(
        bundle.backend_link.diagnostics.is_empty(),
        "backend link diagnostics: {:?}\nbackend diagnostics: {:?}\ncore diagnostics: {:?}\nwat:\n{}",
        bundle.backend_link.diagnostics,
        bundle.defs[0].output.backend.diagnostics,
        bundle.defs[0].output.core_validation.diagnostics,
        bundle.defs[0].output.backend.wat
    );
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnBranch {
                cond: chiba_level1r::core::CoreValue::Bool(true),
                then_value: chiba_level1r::core::CoreValue::I64(1),
                else_value: chiba_level1r::core::CoreValue::I64(2),
            }
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains("if (result i32)"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_literal_match_return_lowers_to_executable_wat_if_chain() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::match_expr(
            Expr::i64(0),
            vec![
                (chiba_level1r::ast::Pattern::lit_i64(0), Expr::i64(7)),
                (chiba_level1r::ast::Pattern::wildcard(), Expr::i64(9)),
            ],
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnMatch {
                scrutinee: chiba_level1r::core::CoreValue::I64(0),
                arms,
            } if arms.len() == 2
                && arms[0].pattern == chiba_level1r::core::CorePattern::I64(0)
                && arms[0].value == chiba_level1r::core::CoreValue::I64(7)
                && arms[1].pattern == chiba_level1r::core::CorePattern::Wildcard
                && arms[1].value == chiba_level1r::core::CoreValue::I64(9)
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; core-return match scrutinee=0 arms=2"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "7");
}

#[test]
fn program_tuple_field_return_lowers_to_executable_wat_value() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::field(Expr::tuple(vec![Expr::i64(4), Expr::bool(true)]), "_2"),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(
        bundle.backend_link.diagnostics.is_empty(),
        "backend link diagnostics: {:?}\nbackend diagnostics: {:?}\ncore diagnostics: {:?}\nwat:\n{}",
        bundle.backend_link.diagnostics,
        bundle.defs[0].output.backend.diagnostics,
        bundle.defs[0].output.core_validation.diagnostics,
        bundle.defs[0].output.backend.wat
    );
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnValue(
                chiba_level1r::core::CoreValue::TupleField { tuple, field }
            ) if field == "_2"
                && matches!(
                    tuple.as_ref(),
                    chiba_level1r::core::CoreValue::Tuple { fields }
                        if fields == &vec![
                            chiba_level1r::core::CoreValue::I64(4),
                            chiba_level1r::core::CoreValue::Bool(true),
                        ]
                )
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; tuple-field layout=tuple::Tuple2_I64_Bool field=_2"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_record_field_return_lowers_to_executable_wat_value() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::field(
            Expr::record(vec![("x", Expr::i64(4)), ("y", Expr::bool(true))]),
            "y",
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(
        bundle.backend_link.diagnostics.is_empty(),
        "backend link diagnostics: {:?}\nbackend diagnostics: {:?}\ncore diagnostics: {:?}\nwat:\n{}",
        bundle.backend_link.diagnostics,
        bundle.defs[0].output.backend.diagnostics,
        bundle.defs[0].output.core_validation.diagnostics,
        bundle.defs[0].output.backend.wat
    );
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnValue(
                chiba_level1r::core::CoreValue::RecordField { record, field }
            ) if field == "y"
                && matches!(
                    record.as_ref(),
                    chiba_level1r::core::CoreValue::Record { fields }
                        if fields.iter().any(|field|
                            field.name == "x"
                                && field.value == chiba_level1r::core::CoreValue::I64(4)
                        )
                        && fields.iter().any(|field|
                            field.name == "y"
                                && field.value == chiba_level1r::core::CoreValue::Bool(true)
                        )
                )
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; record-field layout=record::x+y field=y"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_zero_arg_tailcall_lowers_to_executable_direct_call_wat() {
    let program = SourceProgram::new(vec![
        def("helper", vec![], Expr::i64(7)),
        def("main", vec![], Expr::call_args(Expr::var("helper"), Vec::new())),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(
        bundle.backend_link.diagnostics.is_empty(),
        "backend link diagnostics: {:?}\nwat:\n{}",
        bundle.backend_link.diagnostics,
        bundle.backend_link.linked_wat
    );
    assert!(bundle.defs[1].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "helper" && args.is_empty()
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains(";; tailcall helper args=[]"));
    assert!(bundle.backend_link.linked_wat.contains("call $helper"));

    let callable_wat = export_func_for_test(
        &bundle.backend_link.linked_wat,
        "$chiba_tailcall_0",
        "tailcall_main",
    );
    assert_eq!(run_wat_export(&callable_wat, "tailcall_main"), "7");
}

#[test]
fn program_adt_constructor_return_lowers_to_executable_tag_wat() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(1)]),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(
        bundle.backend_link.diagnostics.is_empty(),
        "backend link diagnostics: {:?}\nbackend diagnostics: {:?}\ncore diagnostics: {:?}\nwat:\n{}",
        bundle.backend_link.diagnostics,
        bundle.defs[0].output.backend.diagnostics,
        bundle.defs[0].output.core_validation.diagnostics,
        bundle.defs[0].output.backend.wat
    );
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnValue(
                chiba_level1r::core::CoreValue::Adt {
                    data,
                    ctor,
                    variants,
                    args,
                }
            ) if data == "Option"
                && ctor == "Some"
                && variants == &vec!["None".to_string(), "Some".to_string()]
                && args == &vec![chiba_level1r::core::CoreValue::I64(1)]
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; adt data=Option ctor=Some args=1"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_bundle_reports_duplicate_defs_and_entry_params() {
    let program = SourceProgram::new(vec![
        def("main", vec!["x"], Expr::var("x")),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.entry, Some("main".to_string()));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateDef {
            name: "main".to_string(),
        }));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["x".to_string()],
        }));
    assert_eq!(bundle.backend_link.linked_wat.matches("(export \"main\")").count(), 1);
    assert!(bundle.backend_link.linked_wat.contains("(func $main__def1 (result i32)"));
}

#[test]
fn pattern_clause_defs_lower_to_single_dispatcher_without_duplicate_def() {
    let program = SourceProgram::with_surface(
        None,
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
        vec![
            SourceItem::Def {
                receiver: None,
                generics: vec!["T".to_string()],
                name: "unwrap_or_zero".to_string(),
                visibility: Visibility::Public,
                params: vec![ParamDecl::pattern(
                    chiba_level1r::ast::Pattern::ctor("Some", vec![chiba_level1r::ast::Pattern::bind("x")]),
                    Some("Option[i64]".to_string()),
                )],
                return_type: Some("i64".to_string()),
                body: Expr::var("x"),
            },
            SourceItem::Def {
                receiver: None,
                generics: vec!["T".to_string()],
                name: "unwrap_or_zero".to_string(),
                visibility: Visibility::Public,
                params: vec![ParamDecl::pattern(
                    chiba_level1r::ast::Pattern::ctor("None", Vec::new()),
                    Some("Option[i64]".to_string()),
                )],
                return_type: Some("i64".to_string()),
                body: Expr::i64(0),
            },
        ],
    );

    let bundle = compile_program_bundle(&program);

    assert!(!bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateDef {
            name: "unwrap_or_zero".to_string(),
        }));
    assert_eq!(bundle.defs.len(), 1);
    assert_eq!(bundle.defs[0].name, "unwrap_or_zero");
    assert_eq!(bundle.defs[0].params, vec!["value".to_string()]);
    assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
    assert_eq!(bundle.defs[0].output.pattern.matches.len(), 1);
    assert!(bundle.defs[0].output.pattern.matches[0].exhaustive);
}

#[test]
fn source_pattern_clause_defs_compile_through_dispatcher() {
    let output = chiba_level1r::compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def unwrap_or_zero(Some(x): Option[i64]): i64 = x
def unwrap_or_zero(None: Option[i64]): i64 = 0",
    )
    .expect("compile source");

    assert_eq!(output.frontend.program.items.len(), 2);
    assert_eq!(output.program.defs.len(), 1);
    assert_eq!(output.program.defs[0].name, "unwrap_or_zero");
    assert_eq!(output.program.defs[0].params, vec!["value".to_string()]);
    assert_eq!(output.program.defs[0].output.typed.ty, Type::I64);
    assert_eq!(output.program.defs[0].output.pattern.matches.len(), 1);
    assert!(output.program.defs[0].output.pattern.matches[0].exhaustive);
}

#[test]
fn source_pattern_clause_defs_merge_wildcard_fallback() {
    let output = chiba_level1r::compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def unwrap_or_zero(Some(x): Option[i64]): i64 = x
def unwrap_or_zero(_: Option[i64]): i64 = 0",
    )
    .expect("compile source");

    assert_eq!(output.frontend.program.items.len(), 2);
    assert!(!output
        .program
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateDef {
            name: "unwrap_or_zero".to_string(),
        }));
    assert_eq!(output.program.defs.len(), 1);
    assert_eq!(output.program.defs[0].name, "unwrap_or_zero");
    assert_eq!(output.program.defs[0].params, vec!["value".to_string()]);
    assert_eq!(output.program.defs[0].output.typed.ty, Type::I64);
    assert_eq!(output.program.defs[0].output.pattern.matches.len(), 1);
    assert!(output.program.defs[0].output.pattern.matches[0].exhaustive);
}

#[test]
fn source_pattern_clause_defs_merge_binding_fallback() {
    let output = chiba_level1r::compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def unwrap_or_zero(Some(x): Option[i64]): i64 = x
def unwrap_or_zero(other: Option[i64]): i64 = 0",
    )
    .expect("compile source");

    assert_eq!(output.frontend.program.items.len(), 2);
    assert!(!output
        .program
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateDef {
            name: "unwrap_or_zero".to_string(),
        }));
    assert_eq!(output.program.defs.len(), 1);
    assert_eq!(output.program.defs[0].name, "unwrap_or_zero");
    assert_eq!(output.program.defs[0].params, vec!["value".to_string()]);
    assert_eq!(output.program.defs[0].output.typed.ty, Type::I64);
    assert_eq!(output.program.defs[0].output.pattern.matches.len(), 1);
    assert!(output.program.defs[0].output.pattern.matches[0].exhaustive);
}

#[test]
fn program_surface_and_interface_summary_preserve_owner_namespace() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string(), "core".to_string()])),
        vec![UseDecl::new(vec!["std".to_string(), "regex".to_string()], false)],
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            vec!["T".to_string()],
            vec![
                DataVariant::new("Some", vec!["T".to_string()]),
                DataVariant::new("None", Vec::new()),
            ],
        )],
        vec![def("main", vec![], Expr::i64(7))],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.namespace, "parser.core");
    assert_eq!(bundle.surface.imports, vec!["std.regex".to_string()]);
    assert_eq!(bundle.surface.defs[0].owner, "parser.core");
    assert_eq!(bundle.surface.defs[0].param_types, Vec::<Option<String>>::new());
    assert_eq!(bundle.surface.defs[0].return_type, None);
    assert_eq!(bundle.surface.data[0].owner, "parser.core");
    assert_eq!(bundle.surface.constructors.len(), 2);
    assert_eq!(bundle.interface.namespace, "parser.core");
    assert_eq!(bundle.interface.functions[0].symbol, "parser.core::main");
    assert_eq!(bundle.interface.functions[0].param_types, Vec::<Option<String>>::new());
    assert_eq!(bundle.interface.functions[0].return_type, None);
    assert_eq!(bundle.interface.data[0].symbol, "parser.core::Option");
    assert_eq!(
        bundle.interface.constructors[0].symbol,
        "parser.core::Option.Some"
    );
    assert_eq!(bundle.interface.constructors[0].arity, 1);
    assert_eq!(bundle.interface.constructors[1].arity, 0);
    assert_eq!(bundle.interface.imports, vec!["std.regex".to_string()]);
    assert_eq!(bundle.interface.stable_hash.len(), 16);

    let summary = bundle.render_summary();
    assert!(summary.contains("surface=ProjectSurface"));
    assert!(summary.contains("interface=InterfaceSummary"));
    assert!(summary.contains("parser.core::Option.Some"));
}

#[test]
fn private_def_and_static_visibility_reach_surface_and_interface() {
    let output = chiba_level1r::compile_source_program_bundle(
        "namespace demo.math
private def hidden(): i64 = 0
private def SECRET: i64 = 1
def public_value(): i64 = SECRET",
    )
    .expect("compile source");

    assert_eq!(output.program.surface.defs[0].name, "hidden");
    assert_eq!(output.program.surface.defs[0].visibility, Visibility::Private);
    assert_eq!(output.program.surface.defs[1].name, "public_value");
    assert_eq!(output.program.surface.defs[1].visibility, Visibility::Public);
    assert_eq!(output.program.surface.statics[0].name, "SECRET");
    assert_eq!(
        output.program.surface.statics[0].visibility,
        Visibility::Private
    );
    assert_eq!(
        output.program.interface.functions[0].visibility,
        Visibility::Private
    );
    assert_eq!(
        output.program.interface.functions[1].visibility,
        Visibility::Public
    );
    assert_eq!(
        output.program.interface.statics[0].visibility,
        Visibility::Private
    );
    assert!(output.program.render_summary().contains("visibility"));
}

#[test]
fn project_surface_many_merges_namespaces_deterministically() {
    let lexer_program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["lexer".to_string()])),
        vec![UseDecl::new(vec!["std".to_string(), "text".to_string()], false)],
        vec![TypeDecl::alias("TokenId", Vec::new(), "i64")],
        Vec::new(),
        vec![def("scan", vec![], Expr::i64(1))],
    );
    let parser_program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        vec![UseDecl::new(vec!["lexer".to_string()], true)],
        Vec::new(),
        vec![DataDecl::new(
            "Ast",
            Vec::new(),
            vec![DataVariant::new("Node", vec!["TokenId".to_string()])],
        )],
        vec![def("parse", vec![], Expr::i64(2))],
    );

    let forward = project_surface_many(&[lexer_program.clone(), parser_program.clone()]);
    let reverse = project_surface_many(&[parser_program, lexer_program]);

    assert_eq!(forward, reverse);
    assert_eq!(forward.namespace, "<project>");
    assert_eq!(
        forward.imports,
        vec!["lexer.*".to_string(), "std.text".to_string()]
    );
    assert_eq!(
        forward
            .defs
            .iter()
            .map(|def| format!("{}::{}", def.owner, def.name))
            .collect::<Vec<_>>(),
        vec!["lexer::scan".to_string(), "parser::parse".to_string()]
    );
    assert_eq!(forward.types[0].owner, "lexer");
    assert_eq!(forward.data[0].owner, "parser");
    assert_eq!(forward.constructors[0].owner, "parser");

    let forward_summary = build_interface_summary(&forward);
    let reverse_summary = build_interface_summary(&reverse);
    assert_eq!(forward_summary, reverse_summary);
    assert_eq!(forward_summary.functions[0].symbol, "lexer::scan");
    assert_eq!(forward_summary.functions[1].symbol, "parser::parse");
    assert_eq!(forward_summary.types[0].symbol, "lexer::TokenId");
    assert_eq!(forward_summary.data[0].symbol, "parser::Ast");
    assert_eq!(forward_summary.constructors[0].symbol, "parser::Ast.Node");
}

#[test]
fn interface_summary_preserves_function_signature_types() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string(), "core".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "id".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("x", Some("I64".to_string()))],
            return_type: Some("I64".to_string()),
            body: Expr::var("x"),
        }],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.defs[0].arity, 1);
    assert_eq!(
        bundle.surface.defs[0].param_types,
        vec![Some("I64".to_string())]
    );
    assert_eq!(bundle.surface.defs[0].return_type, Some("I64".to_string()));
    assert_eq!(bundle.interface.functions[0].symbol, "parser.core::id");
    assert_eq!(
        bundle.interface.functions[0].param_types,
        vec![Some("I64".to_string())]
    );
    assert_eq!(
        bundle.interface.functions[0].return_type,
        Some("I64".to_string())
    );
    assert!(bundle.render_summary().contains("param_types"));
    assert!(bundle.render_summary().contains("return_type"));
}

#[test]
fn interface_summary_preserves_explicit_checked_template_params() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string(), "core".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: vec!["T".to_string()],
            name: "id".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("x", Some("T".to_string()))],
            return_type: Some("T".to_string()),
            body: Expr::var("x"),
        }],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.defs[0].generics, vec!["T".to_string()]);
    assert_eq!(
        bundle.interface.functions[0].generics,
        vec!["T".to_string()]
    );
    assert_eq!(
        bundle.defs[0].output.template.explicit_params,
        vec![chiba_level1r::TemplateParam {
            name: "T".to_string(),
            source: chiba_level1r::TemplateParamSource::ExplicitHeader,
        }]
    );
    assert_eq!(
        bundle.defs[0]
            .output
            .specialize
            .work_items[0]
            .key
            .template_params,
        bundle.defs[0].output.template.explicit_params
    );
}

#[test]
fn interface_summary_preserves_row_style_type_decl_shape() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string(), "core".to_string()])),
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            vec!["T".to_string()],
            vec![TypeField::new("value", "T")],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.types[0].owner, "parser.core");
    assert_eq!(bundle.surface.types[0].name, "Box");
    assert_eq!(bundle.surface.types[0].generics, vec!["T".to_string()]);
    assert_eq!(bundle.surface.types[0].fields[0].name, "value");
    assert_eq!(bundle.surface.types[0].fields[0].ty, "T");
    assert_eq!(bundle.interface.types[0].symbol, "parser.core::Box");
    assert_eq!(bundle.interface.types[0].fields[0].name, "value");
    assert_eq!(bundle.interface.types[0].fields[0].ty, "T");
    assert!(bundle.render_summary().contains("parser.core::Box"));
}

#[test]
fn interface_summary_preserves_type_alias_target() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string(), "core".to_string()])),
        Vec::new(),
        vec![TypeDecl::alias("UserId", Vec::new(), "i64")],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.interface.types[0].symbol, "parser.core::UserId");
    assert_eq!(
        bundle.interface.types[0].alias_target,
        Some("i64".to_string())
    );
    assert!(bundle.render_summary().contains("alias_target"));
}

#[test]
fn typed_signature_resolves_type_alias_headers() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::alias("UserId", Vec::new(), "i64")],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "id".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("value", Some("UserId".to_string()))],
            return_type: Some("UserId".to_string()),
            body: Expr::var("value"),
        }],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.defs[0]
            .output
            .typed_signature
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect::<Vec<_>>(),
        vec![("value".to_string(), "i64".to_string())]
    );
    assert_eq!(
        bundle.defs[0].output.typed_signature.return_type,
        Some("i64".to_string())
    );
    assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
}

#[test]
fn interface_summary_splits_type_fields_from_phantom_markers() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "User",
            Vec::new(),
            vec![
                TypeField::new("id", "i64"),
                TypeField::new("_", "PhantomUser"),
                TypeField::new("_", "AuditMarker"),
            ],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.interface.types[0].fields.len(), 1);
    assert_eq!(bundle.interface.types[0].fields[0].name, "id");
    assert_eq!(
        bundle.interface.types[0].phantom_markers,
        vec!["PhantomUser".to_string(), "AuditMarker".to_string()]
    );
    assert!(bundle.render_summary().contains("phantom_markers"));
}

#[test]
fn interface_summary_preserves_method_style_receiver_and_self_surface() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string(), "core".to_string()])),
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

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.surface.defs[0].receiver,
        Some(MethodReceiver::new("Box", vec!["T".to_string()]))
    );
    assert_eq!(
        bundle.interface.functions[0].receiver,
        Some(MethodReceiver::new("Box", vec!["T".to_string()]))
    );
    assert_eq!(bundle.interface.functions[0].source_name, "update");
    assert_eq!(
        bundle.interface.functions[0].symbol,
        "parser.core::Box[T].update"
    );
    assert_eq!(
        bundle.interface.functions[0].param_types,
        vec![Some("Self".to_string()), Some("T".to_string())]
    );
    assert_eq!(
        bundle.interface.functions[0].return_type,
        Some("Self".to_string())
    );
    assert_eq!(
        bundle.defs[0]
            .output
            .typed_signature
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("self".to_string(), "Box[T]".to_string()),
            ("value".to_string(), "T".to_string()),
        ]
    );
    assert_eq!(
        bundle.defs[0].output.typed_signature.return_type,
        Some("Box[T]".to_string())
    );
    assert!(bundle.defs[0]
        .output
        .render_visual()
        .contains("typed-signature: def update(self: Box[T], value: T): Box[T]"));
    assert!(matches!(
        &bundle.defs[0].output.typed.kind,
        TypedExprKind::Var(name) if name == "self"
    ));
    assert_eq!(
        bundle.defs[0].output.typed.ty,
        Type::Nominal("Box[T]".to_string())
    );
}

#[test]
fn method_self_record_update_preserves_nominal_receiver_type_in_body() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string(), "core".to_string()])),
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
            Expr::record_update(Expr::var("self"), vec![("value", Expr::var("value"))]),
        )],
    );

    let bundle = compile_program_bundle(&program);
    let typed = &bundle.defs[0].output.typed;

    assert_eq!(typed.ty, Type::Nominal("Box[T]".to_string()));
    match &typed.kind {
        TypedExprKind::RecordUpdate { base, fields } => {
            assert_eq!(base.ty, Type::Nominal("Box[T]".to_string()));
            assert_eq!(fields[0].name, "value");
            assert_eq!(fields[0].value.ty, Type::Nominal("T".to_string()));
        }
        other => panic!("expected typed record update, got {other:?}"),
    }
}

#[test]
fn method_self_field_access_uses_row_style_type_shape() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string(), "core".to_string()])),
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            vec!["T".to_string()],
            vec![TypeField::new("value", "T")],
        )],
        Vec::new(),
        vec![SourceItem::method_def(
            MethodReceiver::new("Box", vec!["T".to_string()]),
            "get",
            vec![ParamDecl::new("self", Some("Self".to_string()))],
            Some("T".to_string()),
            Expr::field(Expr::var("self"), "value"),
        )],
    );

    let bundle = compile_program_bundle(&program);
    let typed = &bundle.defs[0].output.typed;

    assert_eq!(typed.ty, Type::Nominal("T".to_string()));
    match &typed.kind {
        TypedExprKind::Field { receiver, name } => {
            assert_eq!(receiver.ty, Type::Nominal("Box[T]".to_string()));
            assert_eq!(name, "value");
        }
        other => panic!("expected typed field access, got {other:?}"),
    }
}

#[test]
fn nominal_row_field_access_substitutes_concrete_type_arguments() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            vec!["T".to_string()],
            vec![TypeField::new("value", "T")],
        )],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "main".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("box", Some("Box[i64]".to_string()))],
            return_type: Some("i64".to_string()),
            body: Expr::field(Expr::var("box"), "value"),
        }],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
}

#[test]
fn auto_generic_is_lowering_fact_not_source_surface_generic() {
    let program = SourceProgram::new(vec![def("id", vec!["x"], Expr::var("x"))]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.defs[0].generics, Vec::<String>::new());
    assert_eq!(bundle.interface.functions[0].generics, Vec::<String>::new());
    assert_eq!(
        bundle.defs[0].output.template.explicit_params,
        vec![chiba_level1r::TemplateParam {
            name: "T_x".to_string(),
            source: chiba_level1r::TemplateParamSource::SyntheticAutoGeneric,
        }]
    );
    assert_eq!(
        bundle.defs[0]
            .output
            .specialize
            .work_items[0]
            .key
            .template_params,
        bundle.defs[0].output.template.explicit_params
    );
}

#[test]
fn program_surface_and_interface_preserve_static_values_separately_from_functions() {
    let program = SourceProgram::new(vec![
        static_value("ONE", Some("i64"), Expr::i64(1)),
        def("main", vec![], Expr::var("ONE")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.defs.len(), 1);
    assert_eq!(bundle.surface.defs[0].name, "main");
    assert_eq!(bundle.surface.statics.len(), 1);
    assert_eq!(bundle.surface.statics[0].name, "ONE");
    assert_eq!(bundle.surface.statics[0].ty, Some("i64".to_string()));
    assert_eq!(bundle.interface.functions[0].symbol, "root::main");
    assert_eq!(bundle.interface.statics[0].symbol, "root::ONE");
    assert_eq!(bundle.interface.statics[0].ty, Some("i64".to_string()));
    assert_eq!(bundle.defs.len(), 1);
    assert_eq!(bundle.defs[0].name, "main");
}

#[test]
fn global_init_allows_ordered_and_forward_static_dependencies() {
    let program = SourceProgram::new(vec![
        static_value("THREE", Some("i64"), Expr::var("TWO")),
        static_value("ONE", Some("i64"), Expr::i64(1)),
        static_value(
            "TWO",
            Some("i64"),
            Expr::binary(chiba_level1r::ast::BinaryOp::Add, Expr::var("ONE"), Expr::i64(1)),
        ),
        def("main", vec![], Expr::var("THREE")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(
        bundle
            .global_init
            .statics
            .iter()
            .map(|static_value| {
                (
                    static_value.name.as_str(),
                    static_value.dependencies.as_slice(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("THREE", &["TWO".to_string()][..]),
            ("ONE", &[][..]),
            ("TWO", &["ONE".to_string()][..]),
        ]
    );
    assert_eq!(
        bundle.global_init.init_order,
        vec!["ONE".to_string(), "TWO".to_string(), "THREE".to_string()]
    );
    assert_eq!(
        bundle.defs[0].output.backend.return_value,
        Some(chiba_level1r::core::CoreValue::Var("THREE".to_string()))
    );
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__ONE (mut i32) (i32.const 0)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__TWO (mut i32) (i32.const 0)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__THREE (mut i32) (i32.const 0)"));
    assert!(bundle.backend_link.linked_wat.contains("(func $__chiba_init"));
    assert!(bundle.backend_link.linked_wat.contains("(start $__chiba_init)"));
    assert!(bundle.backend_link.linked_wat.contains("global.set $global__ONE"));
    assert!(bundle.backend_link.linked_wat.contains("global.get $global__ONE"));
    assert!(bundle.backend_link.linked_wat.contains("i32.add"));
    assert!(bundle.backend_link.linked_wat.contains("global.set $global__TWO"));
    assert!(bundle.backend_link.linked_wat.contains("global.set $global__THREE"));
    assert!(bundle.backend_link.linked_wat.contains("global.get $global__THREE"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

fn run_wat_text(wat: &str) -> String {
    run_wat_export(wat, "main")
}

fn run_wat_export(wat: &str, export: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "level1r-global-init-{}-{}.wat",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::write(&path, wat).expect("write generated wat fixture");
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("level-1r parent repo root");
    let output = Command::new("node")
        .arg("tools/node/run-wat.mjs")
        .arg(&path)
        .arg("--invoke")
        .arg(export)
        .current_dir(repo_root)
        .output()
        .expect("run generated wat");
    assert!(
        output.status.success(),
        "generated WAT failed\nstdout:\n{}\nstderr:\n{}\nwat:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        wat
    );
    String::from_utf8(output.stdout)
        .expect("wat stdout utf8")
        .trim()
        .to_string()
}

fn export_func_for_test(wat: &str, symbol: &str, export: &str) -> String {
    wat.replace(
        &format!("(func {symbol} (result i32)"),
        &format!("(func {symbol} (export \"{export}\") (result i32)"),
    )
}

#[test]
fn global_init_reports_cycles_and_duplicate_static_names() {
    let program = SourceProgram::new(vec![
        static_value("A", None, Expr::var("B")),
        static_value("B", None, Expr::var("A")),
        static_value("A", None, Expr::i64(0)),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert!(bundle.diagnostics.contains(&ProgramDiagnostic::DuplicateStatic {
        name: "A".to_string(),
    }));
    assert!(bundle.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            ProgramDiagnostic::StaticInitCycle { cycle }
                if cycle == &vec!["A".to_string(), "B".to_string(), "A".to_string()]
        )
    }));
}

#[test]
fn global_init_reports_static_function_name_conflict() {
    let program = SourceProgram::new(vec![
        static_value("main", Some("i64"), Expr::i64(1)),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert!(bundle.diagnostics.contains(
        &ProgramDiagnostic::StaticFunctionNameConflict {
            name: "main".to_string(),
        }
    ));
}

#[test]
fn program_surface_reports_duplicate_data_and_constructor_names() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![
            DataDecl::new(
                "A",
                Vec::new(),
                vec![
                    DataVariant::new("Same", Vec::new()),
                    DataVariant::new("Same", Vec::new()),
                ],
            ),
            DataDecl::new("A", Vec::new(), vec![DataVariant::new("Other", Vec::new())]),
        ],
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateData {
            name: "A".to_string(),
        }));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateConstructor {
            name: "Same".to_string(),
        }));
}

#[test]
fn program_surface_reports_duplicate_type_names() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![
            TypeDecl::new("Box", Vec::new(), vec![TypeField::new("value", "i64")]),
            TypeDecl::new("Box", Vec::new(), vec![TypeField::new("other", "i64")]),
        ],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateType {
            name: "Box".to_string(),
        }));
}

#[test]
fn program_surface_reports_duplicate_ordinary_type_fields() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            Vec::new(),
            vec![
                TypeField::new("value", "i64"),
                TypeField::new("value", "bool"),
                TypeField::new("_", "PhantomA"),
                TypeField::new("_", "PhantomB"),
            ],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateTypeField {
            type_name: "Box".to_string(),
            field: "value".to_string(),
        }));
    assert!(!bundle.diagnostics.contains(
        &ProgramDiagnostic::DuplicateTypeField {
            type_name: "Box".to_string(),
            field: "_".to_string(),
        }
    ));
}

#[test]
fn phantom_type_fields_are_not_available_for_field_access() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            Vec::new(),
            vec![
                TypeField::new("_", "PhantomA"),
                TypeField::new("_", "PhantomB"),
            ],
        )],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "probe".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("box", Some("Box".to_string()))],
            return_type: None,
            body: Expr::field(Expr::var("box"), "_"),
        }, def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(bundle.defs[0].output.typed.ty, Type::Unknown);
}

#[test]
fn program_surface_reports_type_topdef_name_conflicts() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![
            TypeDecl::new("Thing", Vec::new(), vec![TypeField::new("value", "i64")]),
            TypeDecl::new("Config", Vec::new(), vec![TypeField::new("value", "i64")]),
            TypeDecl::new("Global", Vec::new(), vec![TypeField::new("value", "i64")]),
        ],
        vec![DataDecl::new(
            "Thing",
            Vec::new(),
            vec![DataVariant::new("Made", Vec::new())],
        )],
        vec![
            def("Config", vec![], Expr::i64(0)),
            static_value("Global", Some("i64"), Expr::i64(1)),
        ],
    );

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "Thing".to_string(),
        }));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "Config".to_string(),
        }));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "Global".to_string(),
        }));
}

#[test]
fn program_surface_allows_same_constructor_name_across_different_data() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![
            DataDecl::new("Left", Vec::new(), vec![DataVariant::new("Same", Vec::new())]),
            DataDecl::new("Right", Vec::new(), vec![DataVariant::new("Same", Vec::new())]),
        ],
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert!(!bundle.diagnostics.contains(
        &ProgramDiagnostic::DuplicateConstructor {
            name: "Same".to_string(),
        }
    ));
    assert_eq!(
        bundle
            .interface
            .constructors
            .iter()
            .map(|ctor| ctor.symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["root::Left.Same", "root::Right.Same"]
    );
}

#[test]
fn interface_summary_hash_changes_when_constructor_arity_changes() {
    let one_field = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Box",
            Vec::new(),
            vec![DataVariant::new("Wrap", vec!["I64".to_string()])],
        )],
        vec![def("main", vec![], Expr::i64(0))],
    );
    let two_fields = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Box",
            Vec::new(),
            vec![DataVariant::new(
                "Wrap",
                vec!["I64".to_string(), "Bool".to_string()],
            )],
        )],
        vec![def("main", vec![], Expr::i64(0))],
    );

    let one_hash = compile_program_bundle(&one_field).interface.stable_hash;
    let two_hash = compile_program_bundle(&two_fields).interface.stable_hash;

    assert_ne!(one_hash, two_hash);
}

#[test]
fn interface_summary_hash_changes_when_type_phantom_marker_changes() {
    let user_marker = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "User",
            Vec::new(),
            vec![
                TypeField::new("id", "i64"),
                TypeField::new("_", "UserMarker"),
            ],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );
    let admin_marker = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "User",
            Vec::new(),
            vec![
                TypeField::new("id", "i64"),
                TypeField::new("_", "AdminMarker"),
            ],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let user_hash = compile_program_bundle(&user_marker).interface.stable_hash;
    let admin_hash = compile_program_bundle(&admin_marker).interface.stable_hash;

    assert_ne!(user_hash, admin_hash);
}

#[test]
fn program_bundle_reports_missing_entry_for_empty_program() {
    let program = SourceProgram::default();

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.entry, None);
    assert_eq!(bundle.diagnostics, vec![ProgramDiagnostic::MissingEntry]);
    assert!(bundle.backend_link.linked_wat.starts_with("(module\n"));
}

#[test]
fn program_summary_contains_program_level_nanopass_events() {
    let program = SourceProgram::new(vec![def("main", vec![], Expr::i64(7))]);

    let bundle = compile_program_bundle(&program);
    let summary = bundle.render_summary();

    assert!(summary.contains("program:"));
    assert!(summary.contains("namespace=<root>"));
    assert!(summary.contains("imports=[]"));
    assert!(summary.contains("defs=1"));
    assert!(summary.contains("entry=Some(\"main\")"));
    assert!(summary.contains("P1ProjectSurface: SourceProgram -> ProjectSurface"));
    assert!(summary.contains("P2InterfaceSummary: ProjectSurface -> InterfaceSummary"));
    assert!(summary.contains("P3ProgramDiagnostics: ProjectSurface -> ProgramDiagnostics"));
    assert!(summary.contains("P4GlobalInit: SourceProgram+ProjectSurface -> GlobalInitPlan"));
    assert!(summary.contains(
        "P5ProgramDefs: SourceProgram+InterfaceSummary -> ProgramDefOutput"
    ));
    assert!(summary.contains("P6ProgramEntry: ProgramDefOutput -> EntrySelection"));
    assert!(summary.contains(
        "P7ProgramBackendLink: ProgramDefOutput+EntrySelection+GlobalInitPlan -> BackendLinkedBundle"
    ));
    assert!(summary.contains("P8ProgramBackendCacheKey: BackendLinkedBundle -> BackendCacheKey"));
    assert!(summary.contains("global-init=GlobalInitPlan"));
}
