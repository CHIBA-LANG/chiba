use chiba_level1r::ast::{BinaryOp, Expr, Pattern, SourceItem};
use chiba_level1r::control::ContinuationKind;
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
        SourceItem::Def { name, params, body } => {
            assert_eq!(name, "main");
            assert_eq!(params, &Vec::<String>::new());
            assert_eq!(
                body,
                &Expr::binary(
                    BinaryOp::Add,
                    Expr::i64(2),
                    Expr::binary(BinaryOp::Mul, Expr::i64(3), Expr::i64(4)),
                )
            );
        }
    }
}

#[test]
fn frontend_source_program_enters_program_bundle_pipeline() {
    let parsed = parse_source_program("def helper() = true def main() = 7").expect("parse");
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
fn frontend_source_compile_entry_keeps_tokens_program_and_linked_wat() {
    let output = compile_source_program_bundle("def helper() = f(1) def main() = helper(2)")
        .expect("compile source");

    assert_eq!(output.frontend.tokens.len(), 18);
    assert_eq!(output.frontend.program.items.len(), 2);
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

    let summary = output.render_summary();
    assert!(summary.contains("source-program:"));
    assert!(summary.contains("tokens=18"));
    assert!(summary.contains("items=2"));
    assert!(summary.contains("P1ProgramSurface"));
}

#[test]
fn frontend_supports_parameters_and_variables() {
    let parsed = parse_source_program("def id(x) = x").expect("parse");

    match &parsed.program.items[0] {
        SourceItem::Def { name, params, body } => {
            assert_eq!(name, "id");
            assert_eq!(params, &vec!["x".to_string()]);
            assert_eq!(body, &Expr::var("x"));
        }
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
    }

    let bundle = compile_program_bundle(&parsed.program);
    let main = &bundle.defs[0].output;
    assert!(main.cps.to_string().contains("f(x,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, arg }
                if func.contains("operator::Add")
                    && func.contains("f(x,")
                    && arg == "I64(1)"
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains(";; tailcall"));
}

#[test]
fn frontend_parses_if_then_else_through_branch_cps_and_wat() {
    let output = compile_source_program_bundle("def main() = if flag then f(1) else 2")
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
    }

    let main = &output.program.defs[0].output;
    assert!(main.cps.to_string().contains("if flag"));
    assert!(main.cps.to_string().contains("then f(1,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::Branch { cond } if cond == "flag"
        )
    }));
    assert!(output.program.backend_link.linked_wat.contains(";; branch flag"));
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
    assert!(output.program.backend_link.linked_wat.contains(";; match tag"));
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
    let err = parse_source_program("def main() = @").unwrap_err();

    assert!(matches!(err, FrontendError::Lex(_)));
}
