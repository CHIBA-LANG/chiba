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
pub struct PrattSpec {
    pub prefixes: Vec<PrattPrefix>,
    pub infixes: Vec<PrattInfix>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrattPrefix {
    pub token: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrattInfix {
    pub token: String,
    pub label: String,
    pub lbp: u32,
    pub rbp: u32,
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

pub fn parse_pratt_tokens(spec: &PrattSpec, tokens: &[Token]) -> LabeledAst {
    match parse_pratt_expr(spec, tokens, 0, 0) {
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

fn parse_pratt_expr(spec: &PrattSpec, tokens: &[Token], pos: usize, min_bp: u32) -> ParseAttempt {
    let mut lhs = match parse_pratt_prefix(spec, tokens, pos) {
        ParseAttempt::Ok { ast, pos } => ParsedNode { ast, pos },
        err @ ParseAttempt::Err(_) => return err,
    };

    loop {
        let Some(op_token) = tokens.get(lhs.pos) else {
            break;
        };
        let Some(infix) = spec.infixes.iter().find(|infix| infix.token == op_token.name) else {
            break;
        };
        if infix.lbp < min_bp {
            break;
        }
        let rhs = match parse_pratt_expr(spec, tokens, lhs.pos + 1, infix.rbp) {
            ParseAttempt::Ok { ast, pos } => ParsedNode { ast, pos },
            err @ ParseAttempt::Err(_) => return err,
        };
        let lhs_pos = rhs.pos;
        lhs = ParsedNode {
            ast: Ast::Node {
                label: infix.label.clone(),
                children: vec![
                    lhs.ast,
                    Ast::Token {
                        name: op_token.name.clone(),
                        lexeme: op_token.lexeme.clone(),
                    },
                    rhs.ast,
                ],
            },
            pos: lhs_pos,
        };
    }

    ParseAttempt::Ok {
        ast: lhs.ast,
        pos: lhs.pos,
    }
}

fn parse_pratt_prefix(spec: &PrattSpec, tokens: &[Token], pos: usize) -> ParseAttempt {
    match tokens.get(pos) {
        Some(token) => match spec.prefixes.iter().find(|prefix| prefix.token == token.name) {
            Some(prefix) => ParseAttempt::Ok {
                ast: Ast::Node {
                    label: prefix.label.clone(),
                    children: vec![Ast::Token {
                        name: token.name.clone(),
                        lexeme: token.lexeme.clone(),
                    }],
                },
                pos: pos + 1,
            },
            None => ParseAttempt::Err(ParseError {
                farthest: pos,
                expected: pratt_prefix_expected(spec),
            }),
        },
        None => ParseAttempt::Err(ParseError {
            farthest: pos,
            expected: pratt_prefix_expected(spec),
        }),
    }
}

fn pratt_prefix_expected(spec: &PrattSpec) -> Vec<String> {
    spec.prefixes
        .iter()
        .map(|prefix| prefix.token.clone())
        .collect()
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedNode {
    ast: Ast,
    pos: usize,
}
