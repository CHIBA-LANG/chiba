use chiba_level1r::ast::{
    BinaryOp, Expr, MethodReceiver, ParamDecl, Pattern, SourceItem, TypeDecl, TypeField,
};
use chiba_level1r::control::ContinuationKind;
use chiba_level1r::core::CoreValue;
use chiba_level1r::resolve::ResolvedName;
use chiba_level1r::specialize::DischargedObligation;
use chiba_level1r::template::TemplateObligation;
use chiba_level1r::typed::UsageColor;
use chiba_level1r::{
    compile_program_bundle, compile_source_program_bundle, parse_source_program, FrontendError,
};

#[test]
fn frontend_lexes_and_parses_def_source_to_program() {
    let output = parse_source_program("def main() = 2 + 3 * 4").expect("frontend parse");

    assert_eq!(
        output
            .tokens
            .iter()
            .map(|token| (token.name.as_str(), token.lexeme.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("KwDef", "def"),
            ("Ident", "main"),
            ("LParen", "("),
            ("RParen", ")"),
            ("Eq", "="),
            ("Number", "2"),
            ("Plus", "+"),
            ("Number", "3"),
            ("Star", "*"),
            ("Number", "4"),
        ]
    );
    assert_eq!(output.program.items.len(), 1);
    match &output.program.items[0] {
        SourceItem::Def {
            name,
            receiver,
            generics,
            params,
            return_type,
            body,
            ..
        } => {
            assert_eq!(name, "main");
            assert_eq!(receiver, &None);
            assert_eq!(generics, &Vec::<String>::new());
            assert_eq!(params, &Vec::<ParamDecl>::new());
            assert_eq!(return_type, &None);
            assert_eq!(
                body,
                &Expr::binary(
                    BinaryOp::Add,
                    Expr::i64(2),
                    Expr::binary(BinaryOp::Mul, Expr::i64(3), Expr::i64(4)),
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_method_style_def_with_generic_receiver_and_self_type() {
    let output =
        parse_source_program("def Box[T].update(self: Self, value: T): Self = self")
            .expect("frontend parse");

    match &output.program.items[0] {
        SourceItem::Def {
            name,
            receiver,
            generics,
            params,
            return_type,
            body,
            ..
        } => {
            assert_eq!(name, "update");
            assert_eq!(
                receiver,
                &Some(MethodReceiver::new("Box", vec!["T".to_string()]))
            );
            assert_eq!(generics, &vec!["T".to_string()]);
            assert_eq!(
                params,
                &vec![
                    ParamDecl::new("self", Some("Self".to_string())),
                    ParamDecl::new("value", Some("T".to_string())),
                ]
            );
            assert_eq!(return_type, &Some("Self".to_string()));
            assert_eq!(body, &Expr::var("self"));
        }
        other => panic!("expected method-style function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_row_style_type_decl_with_generics() {
    let output = parse_source_program("type Box[T] = { value: T }
def main() = 0")
        .expect("frontend parse");

    assert_eq!(
        output.program.types,
        vec![TypeDecl::new(
            "Box",
            vec!["T".to_string()],
            vec![TypeField::new("value", "T")]
        )]
    );
    assert_eq!(output.program.items.len(), 1);
}

#[test]
fn frontend_parses_type_alias_decl() {
    let output = parse_source_program("type UserId = i64
def main(id: UserId) = id")
        .expect("frontend parse");

    assert_eq!(
        output.program.types,
        vec![TypeDecl::alias("UserId", Vec::new(), "i64")]
    );
    assert_eq!(output.program.items.len(), 1);
}

#[test]
fn frontend_parses_utf8_row_style_type_decl_names() {
    let output = parse_source_program(
        "type 向量🚀[Τ] = { 值中文: Τ, emoji字段🚀: i64 }
def main() = 0",
    )
    .expect("frontend parse");

    for expected in ["向量🚀", "Τ", "值中文", "emoji字段🚀"] {
        assert!(
            output
                .tokens
                .iter()
                .any(|token| token.name == "Ident" && token.lexeme == expected),
            "missing UTF-8 identifier token {expected}"
        );
    }
    assert_eq!(
        output.program.types,
        vec![TypeDecl::new(
            "向量🚀",
            vec!["Τ".to_string()],
            vec![
                TypeField::new("值中文", "Τ"),
                TypeField::new("emoji字段🚀", "i64"),
            ],
        )]
    );
}

#[test]
fn frontend_source_program_enters_program_bundle_pipeline() {
    let parsed = parse_source_program("def helper() = true
def main() = 7").expect("parse");
    let bundle = compile_program_bundle(&parsed.program);

    assert_eq!(bundle.entry, Some("main".to_string()));
    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main__def1 (export \"main\") (result i32)"));
    assert!(bundle.backend_link.linked_wat.contains("i32.const 1"));
    assert!(bundle.backend_link.linked_wat.contains("i32.const 7"));
}

#[test]
fn frontend_parses_namespace_and_use_headers_as_project_surface() {
    let output = parse_source_program(
        "namespace parser.chiba
use frontend.ast.*
use std.regex
def main() = 7",
    )
    .expect("frontend parse");

    let namespace = output.program.namespace.as_ref().expect("namespace");
    assert_eq!(namespace.path, vec!["parser".to_string(), "chiba".to_string()]);
    assert_eq!(namespace.dotted(), "parser.chiba");
    assert_eq!(output.program.imports.len(), 2);
    assert_eq!(output.program.imports[0].path, vec!["frontend".to_string(), "ast".to_string()]);
    assert!(output.program.imports[0].glob);
    assert_eq!(output.program.imports[0].dotted(), "frontend.ast.*");
    assert_eq!(output.program.imports[1].path, vec!["std".to_string(), "regex".to_string()]);
    assert!(!output.program.imports[1].glob);
    assert_eq!(output.program.items.len(), 1);

    assert_eq!(
        output
            .tokens
            .iter()
            .map(|token| token.name.as_str())
            .take(12)
            .collect::<Vec<_>>(),
        vec![
            "KwNamespace",
            "Ident",
            "Dot",
            "Ident",
            "KwUse",
            "Ident",
            "Dot",
            "Ident",
            "Dot",
            "Star",
            "KwUse",
            "Ident",
        ]
    );
}

#[test]
fn frontend_project_surface_reaches_program_summary_and_keeps_entry() {
    let output = compile_source_program_bundle(
        "namespace parser.chiba
use frontend.ast.*
def helper() = 1
def main() = 7",
    )
    .expect("compile source");

    assert_eq!(output.program.entry, Some("main".to_string()));
    assert_eq!(
        output.program.namespace.as_ref().map(|namespace| namespace.dotted()),
        Some("parser.chiba".to_string())
    );
    assert_eq!(
        output
            .program
            .imports
            .iter()
            .map(|import| import.dotted())
            .collect::<Vec<_>>(),
        vec!["frontend.ast.*".to_string()]
    );

    let summary = output.render_summary();
    assert!(summary.contains("source-program:"));
    assert!(summary.contains("namespace=parser.chiba"));
    assert!(summary.contains("imports=[\"frontend.ast.*\"]"));
    assert!(summary.contains("program:"));
    assert!(summary.contains("defs=2"));
    assert!(summary.contains("entry=Some(\"main\")"));
}

#[test]
fn frontend_source_compile_entry_keeps_tokens_program_and_linked_wat() {
    let output = compile_source_program_bundle("def helper(x) = f(1)
def main() = helper(2)")
        .expect("compile source");

    assert_eq!(output.frontend.tokens.len(), 19);
    assert_eq!(output.frontend.program.items.len(), 2);
    assert_eq!(output.frontend.item_spans.len(), 2);
    assert_eq!(output.frontend.item_spans[0].name, "helper");
    assert_eq!(output.frontend.item_spans[0].span.line, 1);
    assert_eq!(output.frontend.item_spans[0].span.column, 1);
    assert_eq!(output.frontend.item_spans[1].name, "main");
    assert_eq!(output.frontend.item_spans[1].span.line, 2);
    assert_eq!(output.frontend.item_spans[1].span.column, 1);
    assert_eq!(output.program.entry, Some("main".to_string()));
    assert!(output.program.backend_link.diagnostics.is_empty());
    assert!(output
        .program
        .backend_link
        .linked_wat
        .contains("(func $main__def1 (export \"main\") (result i32)"));
    assert!(output.program.defs[0].output.cps.to_string().contains("f(1,"));
    assert!(output.program.defs[1]
        .output
        .cps
        .to_string()
        .contains("helper(2,"));
    assert!(output.program.defs[1]
        .output
        .resolve
        .resolved_names
        .contains(&ResolvedName::Function {
            name: "helper".to_string(),
            symbol: "root::helper".to_string(),
        }));
    assert!(output.program.defs[1]
        .output
        .template
        .obligations
        .contains(&TemplateObligation::Function {
            name: "helper".to_string(),
            resolved: "root::helper".to_string(),
        }));
    assert!(output.program.defs[1]
        .output
        .specialize
        .work_items[0]
        .obligations
        .contains(&DischargedObligation::Function {
            name: "helper".to_string(),
            target: "root::helper".to_string(),
        }));
    let main_visual = output.program.defs[1].output.render_visual();
    assert!(main_visual.contains("symbol-lineage:"));
    assert!(main_visual.contains("source def main @ 2:1..2:23"));
    assert!(main_visual.contains("resolve function helper -> root::helper"));
    assert!(main_visual.contains("template function helper -> root::helper"));
    assert!(main_visual.contains("specialize function helper -> root::helper"));

    let summary = output.render_summary();
    assert!(summary.contains("source-program:"));
    assert!(summary.contains("tokens=19"));
    assert!(summary.contains("items=2"));
    assert!(summary.contains("item-spans:"));
    assert!(summary.contains("def helper @ 1:1..1:21"));
    assert!(summary.contains("def main @ 2:1..2:23"));
    assert!(summary.contains("data=0"));
    assert!(summary.contains("P1ProjectSurface"));
}

#[test]
fn frontend_parses_data_decl_and_uses_variants_for_qualified_ctors() {
    let output = compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def main() = Option.Some(1)",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    assert_eq!(output.frontend.program.data.len(), 1);
    let data = &output.frontend.program.data[0];
    assert_eq!(data.name, "Option");
    assert_eq!(data.generics, vec!["T".to_string()]);
    assert_eq!(data.variant_names(), vec!["Some".to_string(), "None".to_string()]);
    assert_eq!(data.variants[0].fields, vec!["T".to_string()]);
    assert_eq!(data.variants[1].fields, Vec::<String>::new());

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::adt_ctor(
                    "Option",
                    "Some",
                    vec!["Some", "None"],
                    vec![Expr::i64(1)]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::AdtConstruct {
                data,
                ctor,
                variants,
                args
            } if data == "Option"
                && ctor == "Some"
                && variants == &vec!["None".to_string(), "Some".to_string()]
                && args == &vec!["1".to_string()]
        )
    }));
    assert!(main.resolve.resolved_names.contains(&ResolvedName::Constructor {
        data: "Option".to_string(),
        ctor: "Some".to_string(),
        symbol: "root::Option.Some".to_string(),
        arity: 1,
    }));
    assert!(main
        .template
        .obligations
        .contains(&TemplateObligation::Constructor {
            data: "Option".to_string(),
            ctor: "Some".to_string(),
            resolved: "root::Option.Some".to_string(),
            arity: 1,
        }));
    assert!(main.specialize.work_items[0]
        .obligations
        .contains(&DischargedObligation::Constructor {
            data: "Option".to_string(),
            ctor: "Some".to_string(),
            target: "root::Option.Some".to_string(),
            arity: 1,
        }));
    let visual = main.render_visual();
    assert!(visual.contains("resolve constructor Option.Some/1 -> root::Option.Some"));
    assert!(visual.contains("template constructor Option.Some/1 -> root::Option.Some"));
    assert!(visual.contains("specialize constructor Option.Some/1 -> root::Option.Some"));
    assert!(output.render_summary().contains("data=1"));
}

#[test]
fn frontend_data_decl_allows_zero_arg_qualified_ctor_and_exhaustive_match() {
    let output = compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def main() = match Option.None { Option.Some(value) => value, Option.None => 0 }",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::match_expr(
                    Expr::adt_ctor("Option", "None", vec!["Some", "None"], Vec::<Expr>::new()),
                    vec![
                        (
                            Pattern::qualified_ctor(
                                "Option",
                                "Some",
                                vec![Pattern::bind("value")]
                            ),
                            Expr::var("value"),
                        ),
                        (
                            Pattern::qualified_ctor("Option", "None", Vec::<Pattern>::new()),
                            Expr::i64(0),
                        ),
                    ],
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert_eq!(main.pattern.diagnostics, vec![]);
    assert!(main.cps.to_string().contains("match Option.None"));
    assert!(main.cps.to_string().contains("halt join"));
    assert!(main.cps.to_string().contains("Option.Some(value) =>"));
    assert!(main.cps.to_string().contains("Option.None =>"));
}

#[test]
fn frontend_data_variant_summary_is_independent_of_decl_order() {
    let output = compile_source_program_bundle(
        "def main() = Option.Some(1)
data Option[T] = { Some(T), None }",
    )
    .expect("compile source");

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::adt_ctor(
                    "Option",
                    "Some",
                    vec!["Some", "None"],
                    vec![Expr::i64(1)]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }
    assert_eq!(output.frontend.program.data.len(), 1);
    assert_eq!(output.program.defs.len(), 1);
    assert!(output.program.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::AdtConstruct { variants, .. }
                if variants == &vec!["None".to_string(), "Some".to_string()]
        )
    }));
}

#[test]
fn frontend_supports_parameters_and_variables() {
    let parsed = parse_source_program("def id(x) = x").expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def {
            name,
            params,
            return_type,
            body,
            ..
        } => {
            assert_eq!(name, "id");
            assert_eq!(params, &vec![ParamDecl::untyped("x")]);
            assert_eq!(return_type, &None);
            assert_eq!(body, &Expr::var("x"));
        }
        other => panic!("expected function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_wildcard_parameter_as_pattern_not_binding() {
    let parsed = parse_source_program("def ignore(_) = 1").expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { params, .. } => {
            assert_eq!(
                params,
                &vec![ParamDecl::pattern(Pattern::wildcard(), None)]
            );
            assert_eq!(params[0].name, "_");
        }
        other => panic!("expected function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_constructor_parameter_pattern_with_whole_value_type() {
    let parsed = parse_source_program(
        "data Option[T] = { Some(T), None }
def get(Some(x): Option[i64]) = x",
    )
    .expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { params, body, .. } => {
            assert_eq!(
                params,
                &vec![ParamDecl::pattern(
                    Pattern::ctor("Some", vec![Pattern::bind("x")]),
                    Some("Option[i64]".to_string())
                )]
            );
            assert_eq!(body, &Expr::var("x"));
        }
        other => panic!("expected function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_typed_global_value_def_surface() {
    let output = compile_source_program_bundle("def ANSWER: I64 = 42
def main() = ANSWER")
        .expect("compile source");

    assert_eq!(output.frontend.program.items.len(), 2);
    assert_eq!(
        output.frontend.program.items[0],
        SourceItem::StaticValue {
            name: "ANSWER".to_string(),
            visibility: chiba_level1r::Visibility::Public,
            ty: Some("I64".to_string()),
            body: Expr::i64(42),
        }
    );

    assert_eq!(output.program.entry, Some("main".to_string()));
    assert_eq!(output.program.defs.len(), 1);
    assert_eq!(output.program.defs[0].name, "main");
    assert_eq!(
        output.program.global_init.init_order,
        vec!["ANSWER".to_string()]
    );
    assert_eq!(output.program.global_init.statics[0].ty, Some("I64".to_string()));
    let main = &output.program.defs[0].output;
    assert!(main.resolve.resolved_names.contains(&ResolvedName::Static {
        name: "ANSWER".to_string(),
        symbol: "root::ANSWER".to_string(),
    }));
    assert!(main
        .template
        .obligations
        .contains(&TemplateObligation::Static {
            name: "ANSWER".to_string(),
            resolved: "root::ANSWER".to_string(),
        }));
    assert!(main.specialize.work_items[0]
        .obligations
        .contains(&DischargedObligation::Static {
            name: "ANSWER".to_string(),
            target: "root::ANSWER".to_string(),
        }));
    let visual = main.render_visual();
    assert!(visual.contains("resolve static ANSWER -> root::ANSWER"));
    assert!(visual.contains("template static ANSWER -> root::ANSWER"));
    assert!(visual.contains("specialize static ANSWER -> root::ANSWER"));
}

#[test]
fn frontend_consumes_parameter_and_return_type_annotations() {
    let parsed = parse_source_program("def id(x: I64): I64 = x").expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def {
            name,
            params,
            return_type,
            body,
            ..
        } => {
            assert_eq!(name, "id");
            assert_eq!(params, &vec![ParamDecl::new("x", Some("I64".to_string()))]);
            assert_eq!(return_type, &Some("I64".to_string()));
            assert_eq!(body, &Expr::var("x"));
        }
        other => panic!("expected function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_explicit_checked_template_def_header() {
    let parsed = parse_source_program("def id[T](x: T): T = x").expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def {
            name,
            receiver,
            generics,
            params,
            return_type,
            body,
            ..
        } => {
            assert_eq!(name, "id");
            assert_eq!(receiver, &None);
            assert_eq!(generics, &vec!["T".to_string()]);
            assert_eq!(params, &vec![ParamDecl::new("x", Some("T".to_string()))]);
            assert_eq!(return_type, &Some("T".to_string()));
            assert_eq!(body, &Expr::var("x"));
        }
        other => panic!("expected function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_explicit_call_site_instantiation_without_breaking_index() {
    let parsed =
        parse_source_program("def main() = id[T](value)
def at(values, i) = values[i]")
            .expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::call_args(
                    Expr::instantiate(Expr::var("id"), vec!["T".to_string()]),
                    vec![Expr::var("value")]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }
    match &parsed.program.items[1] {
        SourceItem::Def { body, .. } => {
            assert_eq!(body, &Expr::index(Expr::var("values"), Expr::var("i")));
        }
        other => panic!("expected function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_static_value_defs_separately_from_zero_arg_functions() {
    let output = parse_source_program("def ONE: i64 = 1
def TWO = ONE
def main() = TWO")
        .expect("frontend parse");

    assert_eq!(output.program.items.len(), 3);
    assert_eq!(
        output.program.items[0],
        SourceItem::StaticValue {
            name: "ONE".to_string(),
            visibility: chiba_level1r::Visibility::Public,
            ty: Some("i64".to_string()),
            body: Expr::i64(1),
        }
    );
    assert_eq!(
        output.program.items[1],
        SourceItem::StaticValue {
            name: "TWO".to_string(),
            visibility: chiba_level1r::Visibility::Public,
            ty: None,
            body: Expr::var("ONE"),
        }
    );
    match &output.program.items[2] {
        SourceItem::Def { name, params, body, .. } => {
            assert_eq!(name, "main");
            assert_eq!(params, &Vec::<ParamDecl>::new());
            assert_eq!(body, &Expr::var("TWO"));
        }
        other => panic!("expected zero arg function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_call_before_infix_and_reaches_cps_shape() {
    let parsed = parse_source_program("def main() = f(x) + 1").expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::binary(
                    BinaryOp::Add,
                    Expr::call(Expr::var("f"), Expr::var("x")),
                    Expr::i64(1),
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main.cps.to_string().contains("f(x,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "f" && args == &vec![CoreValue::Var("x".to_string())]
        )
    }));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func.starts_with("operator::Add(")
                    && args == &vec![CoreValue::I64(1)]
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains(";; tailcall"));
}

#[test]
fn frontend_preserves_multi_argument_calls_through_cps_and_core() {
    let parsed = parse_source_program("def main() = f(1, 2, 3)").expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::call_args(
                    Expr::var("f"),
                    vec![Expr::i64(1), Expr::i64(2), Expr::i64(3)]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main.cps.to_string().contains("f(1, 2, 3,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "f"
                    && args == &vec![CoreValue::I64(1), CoreValue::I64(2), CoreValue::I64(3)]
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; tailcall f args=[1, 2, 3]"));
}

#[test]
fn frontend_pipe_defaults_to_first_argument_call() {
    let parsed = parse_source_program("def main() = value |> f(1, 2)").expect("parse");

    assert!(parsed
        .tokens
        .iter()
        .any(|token| token.name == "PipeForward" && token.lexeme == "|>"));
    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::call_args(
                    Expr::var("f"),
                    vec![Expr::var("value"), Expr::i64(1), Expr::i64(2)]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main.cps.to_string().contains("f(value, 1, 2,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "f"
                    && args == &vec![
                        CoreValue::Var("value".to_string()),
                        CoreValue::I64(1),
                        CoreValue::I64(2)
                    ]
        )
    }));
}

#[test]
fn frontend_pipe_placeholder_replaces_each_hole_with_input() {
    let parsed = parse_source_program("def main() = value |> f(prefix, _, _)")
        .expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::call_args(
                    Expr::var("f"),
                    vec![
                        Expr::var("prefix"),
                        Expr::var("value"),
                        Expr::var("value")
                    ]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main.cps.to_string().contains("f(prefix, value, value,"));
    assert_eq!(
        main.usage.vars.get("value").copied(),
        Some(chiba_level1r::usage::UseCount::Many)
    );
}

#[test]
fn frontend_pipe_method_path_is_receiver_first_desugar() {
    let parsed = parse_source_program("def main() = value |> Vec.push(1)")
        .expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::method_call_args(Expr::var("value"), "push", vec![Expr::i64(1)])
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main.cps.to_string().contains("value.push(1,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "value.push" && args == &vec![CoreValue::I64(1)]
        )
    }));
}

#[test]
fn frontend_pipe_chains_left_to_right() {
    let parsed = parse_source_program("def main() = value |> f |> g")
        .expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::call_args(
                    Expr::var("g"),
                    vec![Expr::call_args(Expr::var("f"), vec![Expr::var("value")])]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main.cps.to_string().contains("f(value,"));
    assert!(main.cps.to_string().contains("g(w"));
}

#[test]
fn frontend_parses_range_value_as_shared_ast_node() {
    let parsed = parse_source_program("def main() = start..end").expect("parse");

    assert!(parsed
        .tokens
        .iter()
        .any(|token| token.name == "DotDot" && token.lexeme == ".."));
    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(body, &Expr::range(Expr::var("start"), Expr::var("end")));
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main.cps.to_string().contains("start..end"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnValue(
                chiba_level1r::core::CoreValue::Range { start, end }
            ) if matches!(
                (start.as_ref(), end.as_ref()),
                (
                    chiba_level1r::core::CoreValue::Var(start),
                    chiba_level1r::core::CoreValue::Var(end),
                ) if start == "start" && end == "end"
            )
        )
    }));
}

#[test]
fn frontend_indexing_enters_operator_obligation_path() {
    let parsed = parse_source_program("def main() = values[i]").expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(body, &Expr::index(Expr::var("values"), Expr::var("i")));
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main
        .resolve
        .operator_obligations
        .iter()
        .any(|obligation| {
            obligation.op == chiba_level1r::resolve::OperatorSurface::Index
                && obligation.protocol == "op_index"
        }));
    assert!(main
        .template
        .obligations
        .iter()
        .any(|obligation| matches!(
            obligation,
            TemplateObligation::Operator {
                op: chiba_level1r::resolve::OperatorSurface::Index,
                protocol,
                ..
            } if protocol == "op_index"
        )));
    assert!(main.cps.to_string().contains("operator::op_index(values)(i,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "operator::op_index(values)" && args == &vec![CoreValue::Var("i".to_string())]
        )
    }));
}

#[test]
fn frontend_slice_indexing_uses_index_slice_operator_path() {
    let parsed = parse_source_program("def main() = values[start..end]").expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::index(
                    Expr::var("values"),
                    Expr::range(Expr::var("start"), Expr::var("end"))
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main
        .resolve
        .operator_obligations
        .iter()
        .any(|obligation| {
            obligation.op == chiba_level1r::resolve::OperatorSurface::IndexSlice
                && obligation.protocol == "op_index_slice"
        }));
    assert!(main
        .template
        .obligations
        .iter()
        .any(|obligation| matches!(
            obligation,
            TemplateObligation::Operator {
                op: chiba_level1r::resolve::OperatorSurface::IndexSlice,
                protocol,
                ..
            } if protocol == "op_index_slice"
        )));
    assert!(main
        .cps
        .to_string()
        .contains("operator::op_index_slice(values)(start..end,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "operator::op_index_slice(values)"
                    && args == &vec![CoreValue::Range {
                        start: Box::new(CoreValue::Var("start".to_string())),
                        end: Box::new(CoreValue::Var("end".to_string())),
                    }]
        )
    }));
}

#[test]
fn frontend_parses_if_block_else_block_through_branch_cps_and_wat() {
    let output = compile_source_program_bundle("def main() = if flag { f(1) } else { 2 }")
        .expect("compile source");

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::if_else(
                    Expr::var("flag"),
                    Expr::call(Expr::var("f"), Expr::i64(1)),
                    Expr::i64(2),
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let main = &output.program.defs[0].output;
    assert!(main.cps.to_string().contains("if flag"));
    assert!(main.cps.to_string().contains("{ f(1,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::Branch { cond } if cond == "flag"
        )
    }));
    assert!(output.program.backend_link.linked_wat.contains(";; branch cond=flag"));
}

#[test]
fn frontend_parses_else_if_as_nested_if_expr() {
    let output = compile_source_program_bundle(
        "def main() = if a { 1 } else if b { 2 } else { 3 }",
    )
    .expect("compile source");

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::if_else(
                    Expr::var("a"),
                    Expr::i64(1),
                    Expr::if_else(Expr::var("b"), Expr::i64(2), Expr::i64(3)),
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }
}

#[test]
fn frontend_parses_match_with_literal_and_wildcard_through_cps_core() {
    let output = compile_source_program_bundle("def main() = match tag { 0 => f(1), _ => 2 }")
        .expect("compile source");

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::match_expr(
                    Expr::var("tag"),
                    vec![
                        (Pattern::lit_i64(0), Expr::call(Expr::var("f"), Expr::i64(1))),
                        (Pattern::wildcard(), Expr::i64(2)),
                    ],
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let main = &output.program.defs[0].output;
    assert!(main.cps.to_string().contains("match tag"));
    assert!(main.cps.to_string().contains("0 =>"));
    assert!(main.cps.to_string().contains("_ =>"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::Match { scrutinee, patterns }
                if scrutinee == "tag"
                    && patterns == &vec!["Lit(I64(0))".to_string(), "Wildcard".to_string()]
        )
    }));
    assert!(output
        .program
        .backend_link
        .linked_wat
        .contains(";; match scrutinee=tag"));
}

#[test]
fn frontend_parses_nested_tuple_record_and_at_patterns() {
    let output = compile_source_program_bundle(
        "def main() = match value { whole @ (head, {tail: rest}) => whole, _ => 0 }",
    )
    .expect("compile source");

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::match_expr(
                    Expr::var("value"),
                    vec![
                        (
                            Pattern::at(
                                "whole",
                                Pattern::tuple(vec![
                                    Pattern::bind("head"),
                                    Pattern::record(vec![("tail", Pattern::bind("rest"))]),
                                ]),
                            ),
                            Expr::var("whole"),
                        ),
                        (Pattern::wildcard(), Expr::i64(0)),
                    ],
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let main = &output.program.defs[0].output;
    assert_eq!(main.pattern.envs.len(), 1);
    assert_eq!(
        main.pattern.envs[0].bindings,
        vec!["head".to_string(), "rest".to_string(), "whole".to_string()]
    );
    assert!(main.cps.to_string().contains("whole @ (head, {tail: rest}) =>"));
}

#[test]
fn frontend_parses_qualified_adt_constructor_expression() {
    let output = compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def main() = Option.Some(1)",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::adt_ctor(
                    "Option",
                    "Some",
                    vec!["Some", "None"],
                    vec![Expr::i64(1)]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert_eq!(main.cps.to_string(), "halt Option.Some(1)");
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::AdtConstruct {
                data,
                ctor,
                variants,
                args
            } if data == "Option"
                && ctor == "Some"
                && variants == &vec!["None".to_string(), "Some".to_string()]
                && args == &vec!["1".to_string()]
        )
    }));
}

#[test]
fn frontend_unknown_qualified_call_is_not_guessed_as_adt_ctor() {
    let output =
        compile_source_program_bundle("def main() = Option.Some(1)").expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::method_call_args(Expr::var("Option"), "Some", vec![Expr::i64(1)])
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert!(main.cps.to_string().contains("Option.Some(1,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "Option.Some" && args == &vec![CoreValue::I64(1)]
        )
    }));
}

#[test]
fn frontend_utf8_identifiers_and_ctors_are_not_ascii_case_guessed() {
    let output = compile_source_program_bundle(
        "data 选项 = { 成功(i64), 失败 }
def main() = 选项.成功(1)",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    assert!(output
        .frontend
        .tokens
        .iter()
        .any(|token| token.name == "Ident" && token.lexeme == "选项"));
    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::adt_ctor(
                    "选项",
                    "成功",
                    vec!["成功", "失败"],
                    vec![Expr::i64(1)]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert_eq!(main.cps.to_string(), "halt 选项.成功(1)");
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::AdtConstruct {
                data,
                ctor,
                variants,
                args
            } if data == "选项"
                && ctor == "成功"
                && variants == &vec!["失败".to_string(), "成功".to_string()]
                && args == &vec!["1".to_string()]
        )
    }));
}

#[test]
fn frontend_accepts_greek_chinese_and_emoji_symbols() {
    let output = compile_source_program_bundle(
        "data 结果 = { 🚀Ok(i64), 失败 }
def 函数α() = 结果.🚀Ok(1)",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    assert!(output
        .frontend
        .tokens
        .iter()
        .any(|token| token.name == "Ident" && token.lexeme == "函数α"));
    assert!(output
        .frontend
        .tokens
        .iter()
        .any(|token| token.name == "Ident" && token.lexeme == "🚀Ok"));

    match &output.frontend.program.items[0] {
        SourceItem::Def { name, body, .. } => {
            assert_eq!(name, "函数α");
            assert_eq!(
                body,
                &Expr::adt_ctor(
                    "结果",
                    "🚀Ok",
                    vec!["🚀Ok", "失败"],
                    vec![Expr::i64(1)]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::AdtConstruct {
                data,
                ctor,
                variants,
                args
            } if data == "结果"
                && ctor == "🚀Ok"
                && variants == &vec!["失败".to_string(), "🚀Ok".to_string()]
                && args == &vec!["1".to_string()]
        )
    }));
}

#[test]
fn frontend_accepts_utf8_function_scalar_and_adt_constructor_names_together() {
    let output = compile_source_program_bundle(
        "data 结果α = { 🚀成功(i64), 失败中文 }
def 计算🚀(值α: i64) = match 结果α.🚀成功(值α) { 结果α.🚀成功(内值) => 内值, 结果α.失败中文 => 0 }",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def {
            name,
            params,
            body,
            ..
        } => {
            assert_eq!(name, "计算🚀");
            assert_eq!(params, &vec![ParamDecl::new("值α", Some("i64".to_string()))]);
            assert_eq!(
                body,
                &Expr::match_expr(
                    Expr::adt_ctor(
                        "结果α",
                        "🚀成功",
                        vec!["🚀成功", "失败中文"],
                        vec![Expr::var("值α")]
                    ),
                    vec![
                        (
                            Pattern::qualified_ctor(
                                "结果α",
                                "🚀成功",
                                vec![Pattern::bind("内值")]
                            ),
                            Expr::var("内值"),
                        ),
                        (
                            Pattern::qualified_ctor("结果α", "失败中文", Vec::<Pattern>::new()),
                            Expr::i64(0),
                        ),
                    ],
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert!(main.resolve.resolved_names.contains(&ResolvedName::Constructor {
        data: "结果α".to_string(),
        ctor: "🚀成功".to_string(),
        symbol: "root::结果α.🚀成功".to_string(),
        arity: 1,
    }));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnMatch {
                scrutinee: chiba_level1r::core::CoreValue::Adt {
                    data,
                    ctor,
                    args,
                    ..
                },
                arms,
            } if data == "结果α"
                && ctor == "🚀成功"
                && args == &vec![chiba_level1r::core::CoreValue::Var("值α".to_string())]
                && arms.len() == 2
                && arms[0].pattern
                    == chiba_level1r::core::CorePattern::Constructor {
                        data: Some("结果α".to_string()),
                        ctor: "🚀成功".to_string(),
                        args: vec![chiba_level1r::core::CorePattern::Bind("内值".to_string())],
                    }
                && arms[1].pattern
                    == chiba_level1r::core::CorePattern::Constructor {
                        data: Some("结果α".to_string()),
                        ctor: "失败中文".to_string(),
                        args: Vec::new(),
                    }
        )
    }));
}

#[test]
fn frontend_accepts_utf8_names_for_function_scalar_adt_and_ctor_in_one_program() {
    let output = compile_source_program_bundle(
        "data 结果🚀 = { 🚀成功(i64), 失败中文 }
def 标量Ω: i64 = 41
def 计算🚀(输入β: i64) = match 结果🚀.🚀成功(标量Ω) { 结果🚀.🚀成功(绑定中文) => 绑定中文, 结果🚀.失败中文 => 输入β }",
    )
    .expect("compile source");

    for expected in ["结果🚀", "🚀成功", "标量Ω", "计算🚀", "输入β", "绑定中文"] {
        assert!(
            output
                .frontend
                .tokens
                .iter()
                .any(|token| token.name == "Ident" && token.lexeme == expected),
            "missing UTF-8 identifier token {expected}"
        );
    }
    assert_eq!(
        output.program.global_init.statics[0].name,
        "标量Ω".to_string()
    );

    match &output.frontend.program.items[1] {
        SourceItem::Def {
            name,
            params,
            body,
            ..
        } => {
            assert_eq!(name, "计算🚀");
            assert_eq!(params, &vec![ParamDecl::new("输入β", Some("i64".to_string()))]);
            assert_eq!(
                body,
                &Expr::match_expr(
                    Expr::adt_ctor(
                        "结果🚀",
                        "🚀成功",
                        vec!["🚀成功", "失败中文"],
                        vec![Expr::var("标量Ω")]
                    ),
                    vec![
                        (
                            Pattern::qualified_ctor(
                                "结果🚀",
                                "🚀成功",
                                vec![Pattern::bind("绑定中文")]
                            ),
                            Expr::var("绑定中文"),
                        ),
                        (
                            Pattern::qualified_ctor("结果🚀", "失败中文", Vec::<Pattern>::new()),
                            Expr::var("输入β"),
                        ),
                    ],
                )
            );
        }
        other => panic!("expected UTF-8 function def, got {other:?}"),
    }

    let main = &output.program.defs[0].output;
    assert!(main.resolve.resolved_names.contains(&ResolvedName::Constructor {
        data: "结果🚀".to_string(),
        ctor: "🚀成功".to_string(),
        symbol: "root::结果🚀.🚀成功".to_string(),
        arity: 1,
    }));
    assert!(main
        .pattern
        .envs
        .iter()
        .any(|env| env.bindings == vec!["绑定中文".to_string()]));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnMatch {
                scrutinee: chiba_level1r::core::CoreValue::Adt {
                    data,
                    ctor,
                    args,
                    ..
                },
                arms,
            } if data == "结果🚀"
                && ctor == "🚀成功"
                && args == &vec![chiba_level1r::core::CoreValue::Var("标量Ω".to_string())]
                && arms.len() == 2
                && arms[0].pattern
                    == chiba_level1r::core::CorePattern::Constructor {
                        data: Some("结果🚀".to_string()),
                        ctor: "🚀成功".to_string(),
                        args: vec![chiba_level1r::core::CorePattern::Bind("绑定中文".to_string())],
                    }
                && arms[1].pattern
                    == chiba_level1r::core::CorePattern::Constructor {
                        data: Some("结果🚀".to_string()),
                        ctor: "失败中文".to_string(),
                        args: Vec::new(),
                    }
        )
    }));
}

#[test]
fn frontend_preserves_utf8_names_across_type_alias_row_adt_and_patterns() {
    let output = compile_source_program_bundle(
        "type 标识Ω = i64
type 盒子🚀 = { 值中文: 标识Ω }
data 结果🚀 = { 🚀成功(标识Ω), 失败中文 }
def 标量Ω: 标识Ω = 41
def 读取中文(输入β: 盒子🚀): 标识Ω = 输入β.值中文
def 计算🚀(参数盒: 盒子🚀) = match 结果🚀.🚀成功(读取中文(参数盒)) { 结果🚀.🚀成功(绑定中文) => 绑定中文, 结果🚀.失败中文 => 标量Ω }",
    )
    .expect("compile source");

    for expected in [
        "标识Ω",
        "盒子🚀",
        "值中文",
        "结果🚀",
        "🚀成功",
        "失败中文",
        "标量Ω",
        "读取中文",
        "输入β",
        "计算🚀",
        "参数盒",
        "绑定中文",
    ] {
        assert!(
            output
                .frontend
                .tokens
                .iter()
                .any(|token| token.name == "Ident" && token.lexeme == expected),
            "missing UTF-8 identifier token {expected}"
        );
    }

    assert_eq!(
        output.program.interface.types[0].alias_target,
        Some("i64".to_string())
    );
    assert_eq!(output.program.interface.types[1].symbol, "root::盒子🚀");
    assert_eq!(output.program.interface.types[1].fields[0].name, "值中文");
    assert_eq!(
        output.program.defs[0]
            .output
            .typed_signature
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect::<Vec<_>>(),
        vec![("输入β".to_string(), "盒子🚀".to_string())]
    );
    assert_eq!(
        output.program.defs[0].output.typed_signature.return_type,
        Some("i64".to_string())
    );

    let main = &output.program.defs[1].output;
    assert!(main.resolve.resolved_names.contains(&ResolvedName::Constructor {
        data: "结果🚀".to_string(),
        ctor: "🚀成功".to_string(),
        symbol: "root::结果🚀.🚀成功".to_string(),
        arity: 1,
    }));
    assert!(main
        .pattern
        .envs
        .iter()
        .any(|env| env.bindings == vec!["绑定中文".to_string()]));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnMatch {
                scrutinee: chiba_level1r::core::CoreValue::Adt {
                    data,
                    ctor,
                    ..
                },
                arms,
            } if data == "结果🚀"
                && ctor == "🚀成功"
                && arms.len() == 2
                && arms[0].pattern
                    == chiba_level1r::core::CorePattern::Constructor {
                        data: Some("结果🚀".to_string()),
                        ctor: "🚀成功".to_string(),
                        args: vec![chiba_level1r::core::CorePattern::Bind("绑定中文".to_string())],
                    }
                && arms[1].pattern
                    == chiba_level1r::core::CorePattern::Constructor {
                        data: Some("结果🚀".to_string()),
                        ctor: "失败中文".to_string(),
                        args: Vec::new(),
                    }
        )
    }));
}

#[test]
fn frontend_parses_qualified_constructor_patterns() {
    let output = compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def main() = match Option.Some(1) { Option.Some(value) => value, Option.None => 0 }",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::match_expr(
                    Expr::adt_ctor(
                        "Option",
                        "Some",
                        vec!["Some", "None"],
                        vec![Expr::i64(1)]
                    ),
                    vec![
                        (
                            Pattern::qualified_ctor(
                                "Option",
                                "Some",
                                vec![Pattern::bind("value")]
                            ),
                            Expr::var("value"),
                        ),
                        (
                            Pattern::qualified_ctor("Option", "None", Vec::<Pattern>::new()),
                            Expr::i64(0),
                        ),
                    ],
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert!(main.cps.to_string().contains("Option.Some(value) =>"));
    assert!(main.cps.to_string().contains("Option.None =>"));
    assert_eq!(main.pattern.envs[0].bindings, vec!["value".to_string()]);
}

#[test]
fn frontend_parses_if_let_block_form_through_pattern_env_and_cps() {
    let output = compile_source_program_bundle("def main() = if let value = maybe { value } else { 0 }")
        .expect("compile source");

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::if_let(
                    Pattern::bind("value"),
                    Expr::var("maybe"),
                    Expr::var("value"),
                    Expr::i64(0),
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    let main = &output.program.defs[0].output;
    assert_eq!(main.pattern.envs.len(), 1);
    assert_eq!(main.pattern.envs[0].bindings, vec!["value".to_string()]);
    assert_eq!(main.pattern.envs[0].success_branch_binds, true);
    assert_eq!(main.pattern.envs[0].failure_branch_binds, false);
    assert!(main.cps.to_string().contains("match maybe"));
    assert!(main.cps.to_string().contains("value => join"));
    assert!(main.cps.to_string().contains("_ => join"));
}

#[test]
fn frontend_parses_reset_shift_as_cont1_with_usage_audit() {
    let output = compile_source_program_bundle("def main() = reset { shift k { 0 } }")
        .expect("compile source");
    let main = &output.program.defs[0].output;

    assert_eq!(main.control.errors, vec![]);
    assert_eq!(main.control.continuations.len(), 1);
    assert_eq!(main.control.continuations[0].binder, "k");
    assert_eq!(main.control.continuations[0].kind, ContinuationKind::Cont1);
    assert_eq!(main.control.continuations[0].usage, UsageColor::One);
    assert!(main.cps.to_string().contains("reset {"));
    assert!(main.cps.to_string().contains("shift@cont1 k"));
    assert!(main
        .usage_audit
        .entries
        .iter()
        .any(|entry| entry.subject == "continuation::k"
            && entry.usage_signature.contains("k: 1 Cont1")));
}

#[test]
fn frontend_parses_resetn_shift_as_contn_with_rc_usage_audit() {
    let output = compile_source_program_bundle("def main() = resetn { shift retry { 0 } }")
        .expect("compile source");
    let main = &output.program.defs[0].output;

    assert_eq!(main.control.errors, vec![]);
    assert_eq!(main.control.continuations.len(), 1);
    assert_eq!(main.control.continuations[0].binder, "retry");
    assert_eq!(main.control.continuations[0].kind, ContinuationKind::ContN);
    assert_eq!(main.control.continuations[0].usage, UsageColor::Many);
    assert!(main.cps.to_string().contains("resetn {"));
    assert!(main.cps.to_string().contains("shift@contN retry"));
    assert!(main
        .usage_audit
        .entries
        .iter()
        .any(|entry| entry.subject == "continuation::retry"
            && entry.rust_reference_signature == "let retry: Rc<ContNFrame>"));
}

#[test]
fn frontend_parses_tuple_and_stable_underscore_field_access() {
    let output = compile_source_program_bundle("def main() = (1, true)._2").expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::field(Expr::tuple(vec![Expr::i64(1), Expr::bool(true)]), "_2")
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert!(main.cps.to_string().contains("Tuple2_I64_Bool(_1=1, _2=true)._2"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TupleFieldGet { layout, field }
                if layout == "tuple::Tuple2_I64_Bool" && field == "_2"
        )
    }));
}

#[test]
fn frontend_parses_record_literal_field_and_update_through_core() {
    let output =
        compile_source_program_bundle("def main() = { {x: 1, y: true} | y: false }.y")
            .expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::field(
                    Expr::record_update(
                        Expr::record(vec![("x", Expr::i64(1)), ("y", Expr::bool(true))]),
                        vec![("y", Expr::bool(false))],
                    ),
                    "y",
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert!(main.cps.to_string().contains("record::x+y{x=1, y=false}.y"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::RecordFieldGet { layout, field }
                if layout == "record::x+y" && field == "y"
        )
    }));
    assert!(output
        .program
        .backend_link
        .linked_wat
        .contains(";; record layout=record::x+y fields=2"));
}

#[test]
fn frontend_parses_dot_method_call_as_method_call_ast() {
    let output = compile_source_program_bundle("def main() = receiver.show(0)")
        .expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::method_call(Expr::var("receiver"), "show", Expr::i64(0))
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert!(main.cps.to_string().contains("receiver.show(0,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "receiver.show" && args == &vec![CoreValue::I64(0)]
        )
    }));
}

#[test]
fn frontend_preserves_multi_argument_method_calls_through_cps_and_core() {
    let output = compile_source_program_bundle("def main() = receiver.put(1, 2)")
        .expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::method_call_args(
                    Expr::var("receiver"),
                    "put",
                    vec![Expr::i64(1), Expr::i64(2)]
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert!(main.cps.to_string().contains("receiver.put(1, 2,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "receiver.put"
                    && args == &vec![CoreValue::I64(1), CoreValue::I64(2)]
        )
    }));
    assert!(main.backend.wat.contains(";; tailcall receiver_put args=[1, 2]"));
}

#[test]
fn frontend_parses_lambda_closure_surface_to_lifted_function() {
    let output = compile_source_program_bundle("def main() = (x: I64): I64 => x")
        .expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(body, &Expr::lambda("x", Expr::var("x")));
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert_eq!(main.closure.closures.len(), 1);
    assert_eq!(main.closure.closures[0].captures, vec![]);
    assert_eq!(main.lambda_lift.functions.len(), 1);
    assert_eq!(main.lambda_lift.functions[0].source, "closure::x");
    assert!(main.lambda_lift.functions[0].direct);
    assert!(main
        .backend
        .wat
        .contains("(func $lift__0000__closure__x"));
}

#[test]
fn frontend_parses_nested_lambda_and_preserves_capture_env() {
    let output = compile_source_program_bundle(
        "def main() = (x: Unknown): Unknown => (y: Unknown): Unknown => x(y)",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    match &output.frontend.program.items[0] {
        SourceItem::Def { body, .. } => {
            assert_eq!(
                body,
                &Expr::lambda(
                    "x",
                    Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
                )
            );
        }
        other => panic!("expected function def, got {other:?}"),
    }

    assert_eq!(main.closure.closures.len(), 2);
    let inner = main
        .closure
        .closures
        .iter()
        .find(|closure| closure.param == "y")
        .unwrap();
    assert_eq!(inner.captures.len(), 1);
    assert_eq!(inner.captures[0].name, "x");
    assert_eq!(main.lambda_lift.functions[1].env_params, vec!["x".to_string()]);
    assert!(main.render_visual().contains("lift::0001::closure__y"));
}

#[test]
fn frontend_rejects_empty_call_argument() {
    let err = parse_source_program("def main() = f()").unwrap_err();

    assert!(matches!(
        err,
        FrontendError::UnexpectedToken {
            found,
            expected,
            ..
        } if found == "RParen" && expected.contains(&"Number".to_string())
    ));
}

#[test]
fn frontend_reports_unexpected_token_without_scanner_fallback() {
    let err = parse_source_program("def main() = +").unwrap_err();

    assert!(matches!(
        err,
        FrontendError::UnexpectedToken {
            found,
            expected,
            ..
        } if found == "Plus" && expected.contains(&"Number".to_string())
    ));
}

#[test]
fn frontend_reports_lexer_errors_for_unknown_characters() {
    let err = parse_source_program("def main() = $").unwrap_err();

    assert!(matches!(err, FrontendError::Lex(_)));
}

#[test]
fn frontend_rejects_adjacent_top_level_defs_without_newline() {
    let err = parse_source_program("def first() = 1 def second() = 2").unwrap_err();

    assert!(matches!(
        err,
        FrontendError::UnexpectedToken {
            found,
            lexeme,
            expected,
            ..
        } if found == "KwDef"
            && lexeme == "def"
            && expected == vec!["Newline".to_string(), "Semicolon".to_string()]
    ));
}

#[test]
fn frontend_rejects_adjacent_top_level_items_without_newline() {
    for source in [
        "type Id = i64 def main() = 0",
        "data Option[T] = { Some(T), None } def main() = 0",
        "namespace demo use std.regex",
        "use std.regex def main() = 0",
        "def 中文名🚀() = 1 def second() = 2",
    ] {
        let err = parse_source_program(source).unwrap_err();
        assert!(matches!(
            err,
            FrontendError::UnexpectedToken {
                expected,
                ..
            } if expected == vec!["Newline".to_string(), "Semicolon".to_string()]
        ));
    }
}

#[test]
fn frontend_accepts_semicolon_separated_top_level_items() {
    let output =
        parse_source_program("namespace demo; use std.regex; def first() = 1; def second() = 2")
            .expect("semicolon-separated top-level items");

    assert_eq!(
        output.program.namespace.as_ref().map(|namespace| namespace.dotted()),
        Some("demo".to_string())
    );
    assert_eq!(output.program.imports.len(), 1);
    assert_eq!(output.program.items.len(), 2);
}

#[test]
fn frontend_rejects_adjacent_match_arms_without_comma() {
    let err = parse_source_program("def main() = match tag { 0 => 1 _ => 2 }").unwrap_err();

    assert!(matches!(
        err,
        FrontendError::UnexpectedToken {
            found,
            lexeme,
            expected,
            ..
        } if found == "Ident" && lexeme == "_" && expected == vec!["Comma".to_string()]
    ));
}

#[test]
fn frontend_accepts_newline_separated_match_arms() {
    let output = parse_source_program(
        "def main() = match tag {
0 => 1
_ => 2
}",
    )
    .expect("frontend parse");

    let SourceItem::Def { body, .. } = &output.program.items[0] else {
        panic!("expected function def");
    };
    match body {
        Expr::Match { arms, .. } => {
            assert_eq!(arms.len(), 2);
        }
        other => panic!("expected match expression, got {other:?}"),
    }
}
