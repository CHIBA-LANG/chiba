use std::collections::BTreeMap;

use chiba_level1r::chibacc::{
    parse_pratt_tokens, parse_tokens, Alt, Ast, Grammar, Item, LabeledAst, PrattInfix,
    PrattPrefix, PrattSpec, Rule,
};
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

fn calculator_pratt() -> PrattSpec {
    PrattSpec {
        prefixes: vec![PrattPrefix {
            token: "Number".to_string(),
            label: "NumberExpr".to_string(),
        }],
        infixes: vec![
            PrattInfix {
                token: "Star".to_string(),
                label: "MulExpr".to_string(),
                lbp: 90,
                rbp: 91,
            },
            PrattInfix {
                token: "Plus".to_string(),
                label: "AddExpr".to_string(),
                lbp: 80,
                rbp: 81,
            },
        ],
    }
}

#[test]
fn pratt_parser_preserves_precedence_for_calculator_shape() {
    let result = parse_pratt_tokens(
        &calculator_pratt(),
        &[
            tok("Number", "2"),
            tok("Plus", "+"),
            tok("Number", "3"),
            tok("Star", "*"),
            tok("Number", "4"),
        ],
    );

    match result {
        LabeledAst::Ok {
            ast: Ast::Node { label, children },
            consumed: 5,
        } => {
            assert_eq!(label, "AddExpr");
            assert!(matches!(
                &children[2],
                Ast::Node { label, .. } if label == "MulExpr"
            ));
        }
        other => panic!("expected precedence parse, got {other:?}"),
    }
}

#[test]
fn pratt_parser_uses_lbp_rbp_for_left_associative_infix() {
    let result = parse_pratt_tokens(
        &calculator_pratt(),
        &[
            tok("Number", "1"),
            tok("Plus", "+"),
            tok("Number", "2"),
            tok("Plus", "+"),
            tok("Number", "3"),
        ],
    );

    match result {
        LabeledAst::Ok {
            ast: Ast::Node { label, children },
            consumed: 5,
        } => {
            assert_eq!(label, "AddExpr");
            assert!(matches!(
                &children[0],
                Ast::Node { label, .. } if label == "AddExpr"
            ));
        }
        other => panic!("expected left associative parse, got {other:?}"),
    }
}

#[test]
fn pratt_parser_reports_prefix_failure_without_string_terminals() {
    let result = parse_pratt_tokens(&calculator_pratt(), &[tok("Plus", "+")]);

    assert!(matches!(
        result,
        LabeledAst::Err {
            partial: None,
            skipped: 1
        }
    ));
}
