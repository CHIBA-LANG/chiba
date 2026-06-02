use std::collections::BTreeMap;

use chiba_level1r::chibacc::{parse_tokens, Alt, Ast, Grammar, Item, LabeledAst, Rule};
use chiba_level1r::chibalex::Token;

fn tok(name: &str, lexeme: &str) -> Token {
    Token {
        name: name.to_string(),
        lexeme: lexeme.to_string(),
        start: 0,
        end: 0,
    }
}

fn grammar(start: &str, rules: Vec<Rule>) -> Grammar {
    Grammar {
        start: start.to_string(),
        rules: rules
            .into_iter()
            .map(|rule| (rule.name.clone(), rule))
            .collect::<BTreeMap<_, _>>(),
    }
}

#[test]
fn parser_consumes_token_sequence_into_labeled_ast() {
    let grammar = grammar(
        "let_stmt",
        vec![Rule {
            name: "let_stmt".to_string(),
            alts: vec![Alt {
                label: "StmtLet".to_string(),
                items: vec![
                    Item::Token("KwLet".to_string()),
                    Item::Token("Ident".to_string()),
                    Item::Token("Eq".to_string()),
                    Item::Token("Number".to_string()),
                ],
            }],
        }],
    );

    let result = parse_tokens(
        &grammar,
        &[
            tok("KwLet", "let"),
            tok("Ident", "x"),
            tok("Eq", "="),
            tok("Number", "1"),
        ],
    );

    match result {
        LabeledAst::Ok { ast, consumed } => {
            assert_eq!(consumed, 4);
            assert!(matches!(ast, Ast::Node { .. }));
        }
        other => panic!("expected OK parse, got {other:?}"),
    }
}

#[test]
fn parser_backtracks_choice_by_token_stream_position() {
    let grammar = grammar(
        "item",
        vec![Rule {
            name: "item".to_string(),
            alts: vec![
                Alt {
                    label: "FnItem".to_string(),
                    items: vec![Item::Token("KwDef".to_string())],
                },
                Alt {
                    label: "DataItem".to_string(),
                    items: vec![Item::Token("KwData".to_string())],
                },
            ],
        }],
    );

    let result = parse_tokens(&grammar, &[tok("KwData", "data")]);

    assert!(matches!(
        result,
        LabeledAst::Ok {
            ast: Ast::Node { label, .. },
            consumed: 1
        } if label == "DataItem"
    ));
}

#[test]
fn parser_handles_comma_separated_token_list() {
    let grammar = grammar(
        "args",
        vec![Rule {
            name: "args".to_string(),
            alts: vec![Alt {
                label: "Args".to_string(),
                items: vec![Item::SepList {
                    item: Box::new(Item::Token("Ident".to_string())),
                    sep: "Comma".to_string(),
                    allow_empty: false,
                }],
            }],
        }],
    );

    let result = parse_tokens(
        &grammar,
        &[
            tok("Ident", "a"),
            tok("Comma", ","),
            tok("Ident", "b"),
            tok("Comma", ","),
            tok("Ident", "c"),
        ],
    );

    assert!(matches!(result, LabeledAst::Ok { consumed: 5, .. }));
}

#[test]
fn parser_reports_err_with_partial_ast_for_trailing_garbage() {
    let grammar = grammar(
        "number",
        vec![Rule {
            name: "number".to_string(),
            alts: vec![Alt {
                label: "NumberExpr".to_string(),
                items: vec![Item::Token("Number".to_string())],
            }],
        }],
    );

    let result = parse_tokens(&grammar, &[tok("Number", "1"), tok("Ident", "x")]);

    assert!(matches!(
        result,
        LabeledAst::Err {
            partial: Some(_),
            skipped: 1
        }
    ));
}
