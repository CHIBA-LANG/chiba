use std::collections::BTreeMap;

use crate::chibalex::Token;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grammar {
    pub start: String,
    pub rules: BTreeMap<String, Rule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub name: String,
    pub alts: Vec<Alt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alt {
    pub label: String,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Token(String),
    Rule(String),
    Optional(Box<Item>),
    Repeat(Box<Item>),
    SepList {
        item: Box<Item>,
        sep: String,
        allow_empty: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ast {
    Token {
        name: String,
        lexeme: String,
    },
    Node {
        label: String,
        children: Vec<Ast>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LabeledAst {
    Ok { ast: Ast, consumed: usize },
    Err { partial: Option<Ast>, skipped: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub farthest: usize,
    pub expected: Vec<String>,
}

pub fn parse_tokens(grammar: &Grammar, tokens: &[Token]) -> LabeledAst {
    match parse_rule(grammar, &grammar.start, tokens, 0) {
        ParseAttempt::Ok { ast, pos } if pos == tokens.len() => LabeledAst::Ok { ast, consumed: pos },
        ParseAttempt::Ok { ast, pos } => LabeledAst::Err {
            partial: Some(ast),
            skipped: tokens.len().saturating_sub(pos),
        },
        ParseAttempt::Err(err) => LabeledAst::Err {
            partial: None,
            skipped: tokens.len().saturating_sub(err.farthest),
        },
    }
}

fn parse_rule(grammar: &Grammar, name: &str, tokens: &[Token], pos: usize) -> ParseAttempt {
    let Some(rule) = grammar.rules.get(name) else {
        return ParseAttempt::Err(ParseError {
            farthest: pos,
            expected: vec![format!("rule {name}")],
        });
    };
    let mut best_err = ParseError {
        farthest: pos,
        expected: Vec::new(),
    };
    for alt in &rule.alts {
        match parse_alt(grammar, alt, tokens, pos) {
            ParseAttempt::Ok { ast, pos } => return ParseAttempt::Ok { ast, pos },
            ParseAttempt::Err(err) => merge_error(&mut best_err, err),
        }
    }
    ParseAttempt::Err(best_err)
}

fn parse_alt(grammar: &Grammar, alt: &Alt, tokens: &[Token], pos: usize) -> ParseAttempt {
    let mut children = Vec::new();
    let mut cur = pos;
    for item in &alt.items {
        match parse_item(grammar, item, tokens, cur) {
            ParseAttempt::Ok { ast, pos } => {
                children.push(ast);
                cur = pos;
            }
            ParseAttempt::Err(err) => return ParseAttempt::Err(err),
        }
    }
    ParseAttempt::Ok {
        ast: Ast::Node {
            label: alt.label.clone(),
            children,
        },
        pos: cur,
    }
}

fn parse_item(grammar: &Grammar, item: &Item, tokens: &[Token], pos: usize) -> ParseAttempt {
    match item {
        Item::Token(name) => match tokens.get(pos) {
            Some(token) if token.name == *name => ParseAttempt::Ok {
                ast: Ast::Token {
                    name: token.name.clone(),
                    lexeme: token.lexeme.clone(),
                },
                pos: pos + 1,
            },
            _ => ParseAttempt::Err(ParseError {
                farthest: pos,
                expected: vec![name.clone()],
            }),
        },
        Item::Rule(name) => parse_rule(grammar, name, tokens, pos),
        Item::Optional(inner) => match parse_item(grammar, inner, tokens, pos) {
            ok @ ParseAttempt::Ok { .. } => ok,
            ParseAttempt::Err(_) => ParseAttempt::Ok {
                ast: Ast::Node {
                    label: "optional_empty".to_string(),
                    children: Vec::new(),
                },
                pos,
            },
        },
        Item::Repeat(inner) => parse_repeat(grammar, inner, tokens, pos),
        Item::SepList {
            item,
            sep,
            allow_empty,
        } => parse_sep_list(grammar, item, sep, *allow_empty, tokens, pos),
    }
}

fn parse_repeat(grammar: &Grammar, inner: &Item, tokens: &[Token], pos: usize) -> ParseAttempt {
    let mut children = Vec::new();
    let mut cur = pos;
    while let ParseAttempt::Ok { ast, pos: next } = parse_item(grammar, inner, tokens, cur) {
        if next == cur {
            break;
        }
        children.push(ast);
        cur = next;
    }
    ParseAttempt::Ok {
        ast: Ast::Node {
            label: "repeat".to_string(),
            children,
        },
        pos: cur,
    }
}

fn parse_sep_list(
    grammar: &Grammar,
    item: &Item,
    sep: &str,
    allow_empty: bool,
    tokens: &[Token],
    pos: usize,
) -> ParseAttempt {
    let mut children = Vec::new();
    let mut cur = pos;
    match parse_item(grammar, item, tokens, cur) {
        ParseAttempt::Ok { ast, pos: next } => {
            children.push(ast);
            cur = next;
        }
        ParseAttempt::Err(err) if allow_empty => {
            return ParseAttempt::Ok {
                ast: Ast::Node {
                    label: "list".to_string(),
                    children,
                },
                pos: err.farthest.min(pos),
            };
        }
        ParseAttempt::Err(err) => return ParseAttempt::Err(err),
    }
    loop {
        let sep_attempt = parse_item(grammar, &Item::Token(sep.to_string()), tokens, cur);
        let ParseAttempt::Ok { pos: after_sep, .. } = sep_attempt else {
            break;
        };
        match parse_item(grammar, item, tokens, after_sep) {
            ParseAttempt::Ok { ast, pos: next } => {
                children.push(ast);
                cur = next;
            }
            ParseAttempt::Err(err) => return ParseAttempt::Err(err),
        }
    }
    ParseAttempt::Ok {
        ast: Ast::Node {
            label: "list".to_string(),
            children,
        },
        pos: cur,
    }
}

fn merge_error(best: &mut ParseError, next: ParseError) {
    if next.farthest > best.farthest {
        *best = next;
    } else if next.farthest == best.farthest {
        for expected in next.expected {
            if !best.expected.contains(&expected) {
                best.expected.push(expected);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParseAttempt {
    Ok { ast: Ast, pos: usize },
    Err(ParseError),
}
