use std::collections::BTreeMap;

use crate::ast::{
    BinaryOp, DataDecl, DataVariant, Expr, NamespaceDecl, ParamDecl, Pattern, SourceItem,
    SourceProgram, UseDecl,
};
use crate::chibalex::{compile_lexer, LexError, LexerRule, LexerSpec, Token};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontendOutput {
    pub tokens: Vec<Token>,
    pub program: SourceProgram,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrontendError {
    Lex(LexError),
    UnexpectedEof { expected: Vec<String> },
    UnexpectedToken {
        found: String,
        lexeme: String,
        expected: Vec<String>,
    },
    InvalidInteger { lexeme: String },
    TrailingTokens { offset: usize },
}

pub fn parse_source_program(source: &str) -> Result<FrontendOutput, FrontendError> {
    let lexer = compile_lexer(chiba_lexer_spec()).map_err(FrontendError::Lex)?;
    let tokens = lexer.lex(source).map_err(FrontendError::Lex)?;
    let mut parser = FrontendParser::new(tokens.clone());
    let program = parser.parse_program()?;
    Ok(FrontendOutput { tokens, program })
}

fn chiba_lexer_spec() -> LexerSpec {
    LexerSpec {
        rules: vec![
            LexerRule {
                name: "Whitespace".to_string(),
                pattern: "[ \n\t\r]+".to_string(),
                skip: true,
            },
            LexerRule {
                name: "KwDef".to_string(),
                pattern: "def".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwData".to_string(),
                pattern: "data".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwNamespace".to_string(),
                pattern: "namespace".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwUse".to_string(),
                pattern: "use".to_string(),
                skip: false,
            },
            LexerRule {
                name: "True".to_string(),
                pattern: "true".to_string(),
                skip: false,
            },
            LexerRule {
                name: "False".to_string(),
                pattern: "false".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwIf".to_string(),
                pattern: "if".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwLet".to_string(),
                pattern: "let".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwThen".to_string(),
                pattern: "then".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwElse".to_string(),
                pattern: "else".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwMatch".to_string(),
                pattern: "match".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwResetn".to_string(),
                pattern: "resetn".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwReset".to_string(),
                pattern: "reset".to_string(),
                skip: false,
            },
            LexerRule {
                name: "KwShift".to_string(),
                pattern: "shift".to_string(),
                skip: false,
            },
            LexerRule {
                name: "Ident".to_string(),
                pattern: "[a-zA-Z_][a-zA-Z0-9_]*".to_string(),
                skip: false,
            },
            LexerRule {
                name: "Number".to_string(),
                pattern: "[0-9]+".to_string(),
                skip: false,
            },
            punct("LParen", "\\("),
            punct("RParen", "\\)"),
            punct("LBrace", "\\{"),
            punct("RBrace", "\\}"),
            punct("LBracket", "\\["),
            punct("RBracket", "\\]"),
            punct("FatArrow", "=>"),
            punct("Arrow", "->"),
            punct("Dot", "\\."),
            punct("Colon", ":"),
            punct("Pipe", "\\|"),
            punct("At", "@"),
            punct("Comma", ","),
            punct("Eq", "="),
            punct("Plus", "\\+"),
            punct("Minus", "\\-"),
            punct("Star", "\\*"),
            punct("Slash", "\\/"),
        ],
    }
}

fn punct(name: &str, pattern: &str) -> LexerRule {
    LexerRule {
        name: name.to_string(),
        pattern: pattern.to_string(),
        skip: false,
    }
}

struct FrontendParser {
    tokens: Vec<Token>,
    pos: usize,
    data_variants: BTreeMap<String, Vec<String>>,
}

impl FrontendParser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            data_variants: BTreeMap::new(),
        }
    }

    fn parse_program(&mut self) -> Result<SourceProgram, FrontendError> {
        let namespace = if self.peek_name() == Some("KwNamespace") {
            Some(self.parse_namespace()?)
        } else {
            None
        };
        let mut imports = Vec::new();
        while self.peek_name() == Some("KwUse") {
            imports.push(self.parse_use()?);
        }
        let mut items = Vec::new();
        let mut data = Vec::new();
        while !self.is_eof() {
            match self.peek_name() {
                Some("KwData") => data.push(self.parse_data()?),
                Some("KwDef") => items.push(self.parse_def()?),
                Some(found) => {
                    let token = self.tokens[self.pos].clone();
                    return Err(FrontendError::UnexpectedToken {
                        found: found.to_string(),
                        lexeme: token.lexeme,
                        expected: vec!["KwData".to_string(), "KwDef".to_string()],
                    });
                }
                None => break,
            }
        }
        let variants = data_variant_map(&data);
        let items = items
            .into_iter()
            .map(|item| enrich_item_with_data_variants(item, &variants))
            .collect();
        Ok(SourceProgram::with_surface(namespace, imports, data, items))
    }

    fn parse_namespace(&mut self) -> Result<NamespaceDecl, FrontendError> {
        self.expect("KwNamespace")?;
        Ok(NamespaceDecl::new(self.parse_path()?))
    }

    fn parse_use(&mut self) -> Result<UseDecl, FrontendError> {
        self.expect("KwUse")?;
        let mut path = Vec::new();
        path.push(self.expect_lexeme("Ident")?);
        let mut glob = false;
        while self.peek_name() == Some("Dot") {
            self.pos += 1;
            if self.peek_name() == Some("Star") {
                self.pos += 1;
                glob = true;
                break;
            }
            path.push(self.expect_lexeme("Ident")?);
        }
        Ok(UseDecl::new(path, glob))
    }

    fn parse_path(&mut self) -> Result<Vec<String>, FrontendError> {
        let mut path = Vec::new();
        path.push(self.expect_lexeme("Ident")?);
        while self.peek_name() == Some("Dot") {
            self.pos += 1;
            path.push(self.expect_lexeme("Ident")?);
        }
        Ok(path)
    }

    fn parse_def(&mut self) -> Result<SourceItem, FrontendError> {
        self.expect("KwDef")?;
        let name = self.expect_lexeme("Ident")?;
        if self.peek_name() == Some("Colon") {
            self.pos += 1;
            let return_type = Some(self.expect_type_name()?);
            self.expect("Eq")?;
            let body = self.parse_expr_bp(0)?;
            return Ok(SourceItem::Def {
                name,
                params: Vec::new(),
                return_type,
                body,
            });
        }
        self.expect("LParen")?;
        let params = self.parse_params()?;
        self.expect("RParen")?;
        let return_type = if self.peek_name() == Some("Colon") {
            self.pos += 1;
            Some(self.expect_type_name()?)
        } else {
            None
        };
        self.expect("Eq")?;
        let body = self.parse_expr_bp(0)?;
        Ok(SourceItem::Def {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_data(&mut self) -> Result<DataDecl, FrontendError> {
        self.expect("KwData")?;
        let name = self.expect_lexeme("Ident")?;
        let generics = if self.peek_name() == Some("LBracket") {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };
        self.expect("Eq")?;
        self.expect("LBrace")?;
        let mut variants = Vec::new();
        while self.peek_name() != Some("RBrace") {
            variants.push(self.parse_data_variant()?);
            if self.peek_name() == Some("Comma") {
                self.pos += 1;
            }
        }
        self.expect("RBrace")?;
        let data = DataDecl::new(name, generics, variants);
        self.data_variants.insert(data.name.clone(), data.variant_names());
        Ok(data)
    }

    fn parse_generic_params(&mut self) -> Result<Vec<String>, FrontendError> {
        self.expect("LBracket")?;
        let mut params = Vec::new();
        if self.peek_name() == Some("RBracket") {
            self.pos += 1;
            return Ok(params);
        }
        loop {
            params.push(self.expect_lexeme("Ident")?);
            if self.peek_name() != Some("Comma") {
                break;
            }
            self.pos += 1;
        }
        self.expect("RBracket")?;
        Ok(params)
    }

    fn parse_data_variant(&mut self) -> Result<DataVariant, FrontendError> {
        let name = self.expect_lexeme("Ident")?;
        let fields = if self.peek_name() == Some("LParen") {
            self.pos += 1;
            let mut fields = Vec::new();
            if self.peek_name() != Some("RParen") {
                loop {
                    fields.push(self.expect_type_name()?);
                    if self.peek_name() != Some("Comma") {
                        break;
                    }
                    self.pos += 1;
                }
            }
            self.expect("RParen")?;
            fields
        } else {
            Vec::new()
        };
        Ok(DataVariant::new(name, fields))
    }

    fn parse_params(&mut self) -> Result<Vec<ParamDecl>, FrontendError> {
        let mut params = Vec::new();
        if self.peek_name() == Some("RParen") {
            return Ok(params);
        }
        loop {
            let name = self.expect_lexeme("Ident")?;
            let ty = if self.peek_name() == Some("Colon") {
                self.pos += 1;
                Some(self.expect_type_name()?)
            } else {
                None
            };
            params.push(ParamDecl::new(name, ty));
            if self.peek_name() != Some("Comma") {
                break;
            }
            self.pos += 1;
        }
        Ok(params)
    }

    fn parse_expr_bp(&mut self, min_bp: u32) -> Result<Expr, FrontendError> {
        let mut lhs = self.parse_postfix()?;
        loop {
            let Some((op, lbp, rbp)) = self.peek_infix() else {
                break;
            };
            if lbp < min_bp {
                break;
            }
            self.pos += 1;
            let rhs = self.parse_expr_bp(rbp)?;
            lhs = Expr::binary(op, lhs, rhs);
        }
        Ok(lhs)
    }

    fn parse_postfix(&mut self) -> Result<Expr, FrontendError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_name() {
                Some("LParen") => {
                    self.pos += 1;
                    let args = self.parse_expr_args()?;
                    self.expect("RParen")?;
                    expr = Expr::call_args(expr, args);
                }
                Some("Dot") => {
                    self.pos += 1;
                    let name = self.expect_lexeme("Ident")?;
                    if self.peek_name() == Some("LParen") {
                        self.pos += 1;
                        let args = self.parse_expr_args()?;
                        self.expect("RParen")?;
                        expr = match expr {
                            Expr::Var(data) if is_big_camel(&name) => {
                                let variants = self.known_variants(&data, &name);
                                Expr::adt_ctor(data, name.clone(), variants, args)
                            }
                            receiver => {
                                if args.len() != 1 {
                                    return Err(FrontendError::UnexpectedToken {
                                        found: "RParen".to_string(),
                                        lexeme: ")".to_string(),
                                        expected: vec!["single method argument".to_string()],
                                    });
                                }
                                Expr::method_call(receiver, name, args.into_iter().next().unwrap())
                            }
                        };
                    } else {
                        expr = match expr {
                            Expr::Var(data) if is_big_camel(&name) => {
                                let variants = self.known_variants(&data, &name);
                                Expr::adt_ctor(data, name.clone(), variants, Vec::new())
                            }
                            receiver => Expr::field(receiver, name),
                        };
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_expr_args(&mut self) -> Result<Vec<Expr>, FrontendError> {
        let mut args = Vec::new();
        loop {
            args.push(self.parse_expr_bp(0)?);
            if self.peek_name() != Some("Comma") {
                break;
            }
            self.pos += 1;
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, FrontendError> {
        match self.peek_name() {
            Some("Number") => {
                let lexeme = self.expect_lexeme("Number")?;
                lexeme
                    .parse::<i64>()
                    .map(Expr::i64)
                    .map_err(|_| FrontendError::InvalidInteger { lexeme })
            }
            Some("True") => {
                self.pos += 1;
                Ok(Expr::bool(true))
            }
            Some("False") => {
                self.pos += 1;
                Ok(Expr::bool(false))
            }
            Some("KwIf") => self.parse_if(),
            Some("KwMatch") => self.parse_match(),
            Some("KwReset") => self.parse_reset(false),
            Some("KwResetn") => self.parse_reset(true),
            Some("KwShift") => self.parse_shift(),
            Some("Ident") => self.expect_lexeme("Ident").map(Expr::var),
            Some("LParen") => {
                if self.is_lambda_start() {
                    return self.parse_lambda();
                }
                self.pos += 1;
                let first = self.parse_expr_bp(0)?;
                if self.peek_name() == Some("Comma") {
                    let mut fields = vec![first];
                    while self.peek_name() == Some("Comma") {
                        self.pos += 1;
                        if self.peek_name() == Some("RParen") {
                            break;
                        }
                        fields.push(self.parse_expr_bp(0)?);
                    }
                    self.expect("RParen")?;
                    return Ok(Expr::tuple(fields));
                }
                self.expect("RParen")?;
                Ok(first)
            }
            Some("LBrace") => self.parse_record_or_update(),
            Some(found) => {
                let token = self.tokens[self.pos].clone();
                Err(FrontendError::UnexpectedToken {
                    found: found.to_string(),
                    lexeme: token.lexeme,
                    expected: vec![
                        "Number".to_string(),
                        "True".to_string(),
                        "False".to_string(),
                        "KwIf".to_string(),
                        "KwMatch".to_string(),
                        "KwReset".to_string(),
                        "KwResetn".to_string(),
                        "KwShift".to_string(),
                        "Ident".to_string(),
                        "LParen".to_string(),
                        "LBrace".to_string(),
                    ],
                })
            }
            None => Err(FrontendError::UnexpectedEof {
                expected: vec!["expr".to_string()],
            }),
        }
    }

    fn is_lambda_start(&self) -> bool {
        self.peek_name() == Some("LParen")
            && self.peek_next_name() == Some("Ident")
            && self.peek_n_name(2) == Some("Colon")
    }

    fn parse_lambda(&mut self) -> Result<Expr, FrontendError> {
        self.expect("LParen")?;
        let param = self.expect_lexeme("Ident")?;
        self.expect("Colon")?;
        self.expect_type_name()?;
        self.expect("RParen")?;
        self.expect("Colon")?;
        self.expect_type_name()?;
        self.expect("FatArrow")?;
        let body = self.parse_expr_bp(0)?;
        Ok(Expr::lambda(param, body))
    }

    fn parse_record_or_update(&mut self) -> Result<Expr, FrontendError> {
        self.expect("LBrace")?;
        if self.peek_name() == Some("RBrace") {
            self.pos += 1;
            return Ok(Expr::record(Vec::<(&str, Expr)>::new()));
        }
        if self.peek_name() == Some("Ident") && self.peek_next_name() == Some("Colon") {
            let fields = self.parse_record_fields_until("RBrace")?;
            self.expect("RBrace")?;
            return Ok(Expr::record(fields));
        }
        let base = self.parse_expr_bp(0)?;
        self.expect("Pipe")?;
        let fields = self.parse_record_fields_until("RBrace")?;
        self.expect("RBrace")?;
        Ok(Expr::record_update(base, fields))
    }

    fn parse_record_fields_until(&mut self, end: &str) -> Result<Vec<(String, Expr)>, FrontendError> {
        let mut fields = Vec::new();
        while self.peek_name() != Some(end) {
            let name = self.expect_lexeme("Ident")?;
            self.expect("Colon")?;
            let value = self.parse_expr_bp(0)?;
            fields.push((name, value));
            if self.peek_name() == Some("Comma") {
                self.pos += 1;
            } else {
                break;
            }
        }
        Ok(fields)
    }

    fn parse_if(&mut self) -> Result<Expr, FrontendError> {
        self.expect("KwIf")?;
        if self.peek_name() == Some("KwLet") {
            return self.parse_if_let_after_if();
        }
        let cond = self.parse_expr_bp(0)?;
        self.expect("KwThen")?;
        let then_branch = self.parse_expr_bp(0)?;
        self.expect("KwElse")?;
        let else_branch = self.parse_expr_bp(0)?;
        Ok(Expr::if_else(cond, then_branch, else_branch))
    }

    fn parse_if_let_after_if(&mut self) -> Result<Expr, FrontendError> {
        self.expect("KwLet")?;
        let pattern = self.parse_pattern()?;
        self.expect("Eq")?;
        let scrutinee = self.parse_expr_bp(0)?;
        let then_branch = self.parse_block_expr()?;
        self.expect("KwElse")?;
        let else_branch = self.parse_block_expr()?;
        Ok(Expr::if_let(pattern, scrutinee, then_branch, else_branch))
    }

    fn parse_block_expr(&mut self) -> Result<Expr, FrontendError> {
        self.expect("LBrace")?;
        let expr = self.parse_expr_bp(0)?;
        self.expect("RBrace")?;
        Ok(expr)
    }

    fn parse_match(&mut self) -> Result<Expr, FrontendError> {
        self.expect("KwMatch")?;
        let scrutinee = self.parse_expr_bp(0)?;
        self.expect("LBrace")?;
        let mut arms = Vec::new();
        while self.peek_name() != Some("RBrace") {
            let pattern = self.parse_pattern()?;
            self.expect("FatArrow")?;
            let body = self.parse_expr_bp(0)?;
            arms.push((pattern, body));
            if self.peek_name() == Some("Comma") {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.expect("RBrace")?;
        Ok(Expr::match_expr(scrutinee, arms))
    }

    fn parse_reset(&mut self, multi: bool) -> Result<Expr, FrontendError> {
        if multi {
            self.expect("KwResetn")?;
        } else {
            self.expect("KwReset")?;
        }
        let body = self.parse_block_expr()?;
        if multi {
            Ok(Expr::resetn(body))
        } else {
            Ok(Expr::reset(body))
        }
    }

    fn parse_shift(&mut self) -> Result<Expr, FrontendError> {
        self.expect("KwShift")?;
        let binder = self.expect_lexeme("Ident")?;
        let body = self.parse_block_expr()?;
        Ok(Expr::shift(binder, body))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, FrontendError> {
        let pattern = self.parse_pattern_atom()?;
        if self.peek_name() == Some("At") {
            self.pos += 1;
            let name = match pattern {
                Pattern::Bind(name) => name,
                other => {
                    return Err(FrontendError::UnexpectedToken {
                        found: format!("{other:?}"),
                        lexeme: "@".to_string(),
                        expected: vec!["binding name before @".to_string()],
                    })
                }
            };
            return Ok(Pattern::at(name, self.parse_pattern()?));
        }
        Ok(pattern)
    }

    fn parse_pattern_atom(&mut self) -> Result<Pattern, FrontendError> {
        match self.peek_name() {
            Some("Number") => {
                let lexeme = self.expect_lexeme("Number")?;
                lexeme
                    .parse::<i64>()
                    .map(Pattern::lit_i64)
                    .map_err(|_| FrontendError::InvalidInteger { lexeme })
            }
            Some("True") => {
                self.pos += 1;
                Ok(Pattern::lit_bool(true))
            }
            Some("False") => {
                self.pos += 1;
                Ok(Pattern::lit_bool(false))
            }
            Some("Ident") => {
                let name = self.expect_lexeme("Ident")?;
                if name == "_" {
                    Ok(Pattern::wildcard())
                } else if self.peek_name() == Some("Dot") {
                    self.pos += 1;
                    let ctor = self.expect_lexeme("Ident")?;
                    let args = self.parse_constructor_pattern_args()?;
                    Ok(Pattern::qualified_ctor(name, ctor, args))
                } else if is_big_camel(&name) {
                    let args = self.parse_constructor_pattern_args()?;
                    Ok(Pattern::ctor(name, args))
                } else {
                    Ok(Pattern::bind(name))
                }
            }
            Some("LParen") => self.parse_tuple_pattern(),
            Some("LBrace") => self.parse_record_pattern(),
            Some(found) => {
                let token = self.tokens[self.pos].clone();
                Err(FrontendError::UnexpectedToken {
                    found: found.to_string(),
                    lexeme: token.lexeme,
                    expected: vec![
                        "Number".to_string(),
                        "True".to_string(),
                        "False".to_string(),
                        "Ident".to_string(),
                        "LParen".to_string(),
                        "LBrace".to_string(),
                    ],
                })
            }
            None => Err(FrontendError::UnexpectedEof {
                expected: vec!["pattern".to_string()],
            }),
        }
    }

    fn parse_tuple_pattern(&mut self) -> Result<Pattern, FrontendError> {
        self.expect("LParen")?;
        let first = self.parse_pattern()?;
        let mut fields = vec![first];
        while self.peek_name() == Some("Comma") {
            self.pos += 1;
            if self.peek_name() == Some("RParen") {
                break;
            }
            fields.push(self.parse_pattern()?);
        }
        self.expect("RParen")?;
        Ok(Pattern::tuple(fields))
    }

    fn parse_record_pattern(&mut self) -> Result<Pattern, FrontendError> {
        self.expect("LBrace")?;
        let mut fields = Vec::new();
        while self.peek_name() != Some("RBrace") {
            let name = self.expect_lexeme("Ident")?;
            let pattern = if self.peek_name() == Some("Colon") {
                self.pos += 1;
                self.parse_pattern()?
            } else {
                Pattern::bind(name.clone())
            };
            fields.push((name, pattern));
            if self.peek_name() == Some("Comma") {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.expect("RBrace")?;
        Ok(Pattern::record(fields))
    }

    fn parse_constructor_pattern_args(&mut self) -> Result<Vec<Pattern>, FrontendError> {
        if self.peek_name() != Some("LParen") {
            return Ok(Vec::new());
        }
        self.pos += 1;
        let mut args = Vec::new();
        if self.peek_name() == Some("RParen") {
            self.pos += 1;
            return Ok(args);
        }
        loop {
            args.push(self.parse_pattern()?);
            if self.peek_name() != Some("Comma") {
                break;
            }
            self.pos += 1;
        }
        self.expect("RParen")?;
        Ok(args)
    }

    fn peek_infix(&self) -> Option<(BinaryOp, u32, u32)> {
        match self.peek_name()? {
            "Plus" => Some((BinaryOp::Add, 10, 11)),
            "Minus" => Some((BinaryOp::Sub, 10, 11)),
            "Star" => Some((BinaryOp::Mul, 20, 21)),
            "Slash" => Some((BinaryOp::Div, 20, 21)),
            _ => None,
        }
    }

    fn expect(&mut self, name: &str) -> Result<Token, FrontendError> {
        match self.tokens.get(self.pos) {
            Some(token) if token.name == name => {
                self.pos += 1;
                Ok(token.clone())
            }
            Some(token) => Err(FrontendError::UnexpectedToken {
                found: token.name.clone(),
                lexeme: token.lexeme.clone(),
                expected: vec![name.to_string()],
            }),
            None => Err(FrontendError::UnexpectedEof {
                expected: vec![name.to_string()],
            }),
        }
    }

    fn expect_lexeme(&mut self, name: &str) -> Result<String, FrontendError> {
        self.expect(name).map(|token| token.lexeme)
    }

    fn expect_type_name(&mut self) -> Result<String, FrontendError> {
        match self.peek_name() {
            Some("Ident") => self.expect_lexeme("Ident"),
            Some("KwReset") => self.expect_lexeme("KwReset"),
            Some("KwResetn") => self.expect_lexeme("KwResetn"),
            Some("KwShift") => self.expect_lexeme("KwShift"),
            Some("KwMatch") => self.expect_lexeme("KwMatch"),
            Some("KwIf") => self.expect_lexeme("KwIf"),
            Some("KwLet") => self.expect_lexeme("KwLet"),
            Some("KwThen") => self.expect_lexeme("KwThen"),
            Some("KwElse") => self.expect_lexeme("KwElse"),
            Some("True") => self.expect_lexeme("True"),
            Some("False") => self.expect_lexeme("False"),
            Some(found) => {
                let token = self.tokens[self.pos].clone();
                Err(FrontendError::UnexpectedToken {
                    found: found.to_string(),
                    lexeme: token.lexeme,
                    expected: vec!["type name".to_string()],
                })
            }
            None => Err(FrontendError::UnexpectedEof {
                expected: vec!["type name".to_string()],
            }),
        }
    }

    fn peek_name(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(|token| token.name.as_str())
    }

    fn peek_next_name(&self) -> Option<&str> {
        self.tokens.get(self.pos + 1).map(|token| token.name.as_str())
    }

    fn peek_n_name(&self, offset: usize) -> Option<&str> {
        self.tokens.get(self.pos + offset).map(|token| token.name.as_str())
    }

    fn is_eof(&self) -> bool {
        self.pos == self.tokens.len()
    }

    fn known_variants(&self, data: &str, fallback_ctor: &str) -> Vec<String> {
        self.data_variants
            .get(data)
            .cloned()
            .unwrap_or_else(|| vec![fallback_ctor.to_string()])
    }
}

fn is_big_camel(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}

fn data_variant_map(data: &[DataDecl]) -> BTreeMap<String, Vec<String>> {
    data.iter()
        .map(|decl| (decl.name.clone(), decl.variant_names()))
        .collect()
}

fn enrich_item_with_data_variants(
    item: SourceItem,
    variants: &BTreeMap<String, Vec<String>>,
) -> SourceItem {
    match item {
        SourceItem::Def {
            name,
            params,
            return_type,
            body,
        } => SourceItem::Def {
            name,
            params,
            return_type,
            body: enrich_expr_with_data_variants(body, variants),
        },
    }
}

fn enrich_expr_with_data_variants(expr: Expr, variants: &BTreeMap<String, Vec<String>>) -> Expr {
    match expr {
        Expr::AdtCtor {
            data,
            ctor,
            variants: old_variants,
            args,
        } => {
            let args = args
                .into_iter()
                .map(|arg| enrich_expr_with_data_variants(arg, variants))
                .collect();
            let variants = variants.get(&data).cloned().unwrap_or(old_variants);
            Expr::adt_ctor(data, ctor, variants, args)
        }
        Expr::Lambda { param, body } => {
            Expr::lambda(param, enrich_expr_with_data_variants(*body, variants))
        }
        Expr::Call { callee, args } => Expr::call_args(
            enrich_expr_with_data_variants(*callee, variants),
            args.into_iter()
                .map(|arg| enrich_expr_with_data_variants(arg, variants))
                .collect(),
        ),
        Expr::Tuple(fields) => Expr::tuple(
            fields
                .into_iter()
                .map(|field| enrich_expr_with_data_variants(field, variants))
                .collect(),
        ),
        Expr::Record(fields) => Expr::Record(
            fields
                .into_iter()
                .map(|field| crate::ast::RecordField {
                    name: field.name,
                    value: enrich_expr_with_data_variants(field.value, variants),
                })
                .collect(),
        ),
        Expr::RecordUpdate { base, fields } => Expr::RecordUpdate {
            base: Box::new(enrich_expr_with_data_variants(*base, variants)),
            fields: fields
                .into_iter()
                .map(|field| crate::ast::RecordField {
                    name: field.name,
                    value: enrich_expr_with_data_variants(field.value, variants),
                })
                .collect(),
        },
        Expr::Field { receiver, name } => Expr::field(
            enrich_expr_with_data_variants(*receiver, variants),
            name,
        ),
        Expr::MethodCall {
            receiver,
            name,
            arg,
        } => Expr::method_call(
            enrich_expr_with_data_variants(*receiver, variants),
            name,
            enrich_expr_with_data_variants(*arg, variants),
        ),
        Expr::Binary { op, lhs, rhs } => Expr::binary(
            op,
            enrich_expr_with_data_variants(*lhs, variants),
            enrich_expr_with_data_variants(*rhs, variants),
        ),
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => Expr::if_else(
            enrich_expr_with_data_variants(*cond, variants),
            enrich_expr_with_data_variants(*then_branch, variants),
            enrich_expr_with_data_variants(*else_branch, variants),
        ),
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => Expr::if_let(
            pattern,
            enrich_expr_with_data_variants(*scrutinee, variants),
            enrich_expr_with_data_variants(*then_branch, variants),
            enrich_expr_with_data_variants(*else_branch, variants),
        ),
        Expr::Match { scrutinee, arms } => Expr::match_expr(
            enrich_expr_with_data_variants(*scrutinee, variants),
            arms.into_iter()
                .map(|arm| {
                    (
                        arm.pattern,
                        enrich_expr_with_data_variants(arm.body, variants),
                    )
                })
                .collect(),
        ),
        Expr::Nominal { name, expr } => {
            Expr::nominal(name, enrich_expr_with_data_variants(*expr, variants))
        }
        Expr::Reset { multi, body } => {
            let body = enrich_expr_with_data_variants(*body, variants);
            if multi {
                Expr::resetn(body)
            } else {
                Expr::reset(body)
            }
        }
        Expr::Shift { binder, body } => {
            Expr::shift(binder, enrich_expr_with_data_variants(*body, variants))
        }
        Expr::Var(_) | Expr::Lit(_) => expr,
    }
}
