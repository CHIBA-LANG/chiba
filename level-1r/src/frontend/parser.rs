use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    generic_param_decls_from_names, render_source_pattern, BinaryOp, DataDecl, DataVariant, Expr,
    ExternAbi, ExternDecl, GenericBoundDecl, GenericParamDecl, ItemAttr, MethodReceiver,
    NamespaceDecl, ParamDecl, Pattern, SourceItem, SourceProgram, TypeDecl, TypeField, UseDecl,
    Visibility,
};
use crate::chibalex::{compile_lexer, LexError, LexerRule, LexerSpec, Token};
use crate::regex::RegexError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontendOutput {
    pub tokens: Vec<Token>,
    pub program: SourceProgram,
    pub item_spans: Vec<SourceItemSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceItemSpan {
    pub kind: String,
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrontendError {
    Lex(LexError),
    UnexpectedEof {
        expected: Vec<String>,
    },
    UnexpectedToken {
        found: String,
        lexeme: String,
        expected: Vec<String>,
        offset: usize,
    },
    InvalidInteger {
        lexeme: String,
        offset: usize,
    },
    InvalidRune {
        lexeme: String,
        offset: usize,
    },
    TrailingTokens {
        offset: usize,
    },
}

pub fn render_frontend_error(source: &str, error: &FrontendError) -> String {
    match error {
        FrontendError::Lex(error) => render_lex_error(source, error),
        FrontendError::UnexpectedEof { expected } => {
            format!(
                "frontend error at end of file: expected {}",
                expected.join(", ")
            )
        }
        FrontendError::UnexpectedToken {
            found,
            lexeme,
            expected,
            offset,
        } => render_source_error(
            source,
            *offset,
            format!(
                "unexpected token {found} `{lexeme}`, expected {}",
                expected.join(", ")
            ),
        ),
        FrontendError::InvalidInteger { lexeme, offset } => {
            render_source_error(source, *offset, format!("invalid integer `{lexeme}`"))
        }
        FrontendError::InvalidRune { lexeme, offset } => {
            render_source_error(source, *offset, format!("invalid rune literal `{lexeme}`"))
        }
        FrontendError::TrailingTokens { offset } => {
            render_source_error(source, *offset, "trailing tokens".to_string())
        }
    }
}

fn render_lex_error(source: &str, error: &LexError) -> String {
    match error {
        LexError::Regex(error) => {
            format!("frontend lexer regex error: {}", render_regex_error(error))
        }
        LexError::NoRule { offset, found } => render_source_error(
            source,
            *offset,
            format!(
                "no lexer rule for {}",
                found.map_or("end of file".to_string(), |ch| format!("`{ch}`"))
            ),
        ),
        LexError::EmptyMatch { rule, offset } => render_source_error(
            source,
            *offset,
            format!("lexer rule `{rule}` matched empty text"),
        ),
    }
}

fn render_regex_error(error: &RegexError) -> String {
    match error {
        RegexError::UnexpectedEnd => "unexpected end of regex".to_string(),
        RegexError::UnclosedClass => "unclosed character class".to_string(),
        RegexError::UnsupportedPcreFeature(feature) => {
            format!("unsupported PCRE feature `{feature}`")
        }
    }
}

fn render_source_error(source: &str, offset: usize, message: String) -> String {
    let (line, column) = line_column(source, offset);
    let line_text = source.lines().nth(line.saturating_sub(1)).unwrap_or("");
    format!(
        "frontend error at {line}:{column}: {message}\n  {line_text}\n  {}^",
        " ".repeat(column.saturating_sub(1))
    )
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for (index, ch) in source.chars().enumerate() {
        if index == offset {
            return (line, column);
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

pub fn parse_source_program(source: &str) -> Result<FrontendOutput, FrontendError> {
    let lexer = compile_lexer(chiba_lexer_spec()).map_err(FrontendError::Lex)?;
    let tokens = lexer.lex(source).map_err(FrontendError::Lex)?;
    let mut parser = FrontendParser::new(tokens.clone(), source);
    let program = parser.parse_program()?;
    Ok(FrontendOutput {
        tokens,
        program,
        item_spans: parser.item_spans,
    })
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
                name: "KwType".to_string(),
                pattern: "type".to_string(),
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
                name: "KwPrivate".to_string(),
                pattern: "private".to_string(),
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
                name: "KwExtern".to_string(),
                pattern: "extern".to_string(),
                skip: false,
            },
            LexerRule {
                name: "CStrLit".to_string(),
                pattern: "c\"[^\"\\\\]*(\\\\.[^\"\\\\]*)*\"".to_string(),
                skip: false,
            },
            LexerRule {
                name: "RawHashStringLit".to_string(),
                pattern: "r#\"[^#]*\"#".to_string(),
                skip: false,
            },
            LexerRule {
                name: "RawStringLit".to_string(),
                pattern: "r\"[^\"]*\"".to_string(),
                skip: false,
            },
            LexerRule {
                name: "StringLit".to_string(),
                pattern: "\"[^\"\\\\]*(\\\\.[^\"\\\\]*)*\"".to_string(),
                skip: false,
            },
            LexerRule {
                name: "RuneLit".to_string(),
                pattern: "'[^'\\\\]*(\\\\.[^'\\\\]*)*'".to_string(),
                skip: false,
            },
            LexerRule {
                name: "Ident".to_string(),
                pattern: "\\p{XID_START}\\p{XID_CONTINUE}*".to_string(),
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
            punct("Hash", "#"),
            punct("LBracket", "\\["),
            punct("RBracket", "\\]"),
            punct("FatArrow", "=>"),
            punct("Arrow", "->"),
            punct("PipeForward", "\\|>"),
            punct("ColonEq", ":="),
            punct("DotDot", "\\.\\."),
            punct("Dot", "\\."),
            punct("Colon", ":"),
            punct("Pipe", "\\|"),
            punct("At", "@"),
            punct("Comma", ","),
            punct("Semicolon", ";"),
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
    source_chars: Vec<char>,
    pos: usize,
    data_variants: BTreeMap<String, Vec<String>>,
    item_spans: Vec<SourceItemSpan>,
}

impl FrontendParser {
    fn new(tokens: Vec<Token>, source: &str) -> Self {
        Self {
            tokens,
            source_chars: source.chars().collect(),
            pos: 0,
            data_variants: BTreeMap::new(),
            item_spans: Vec::new(),
        }
    }

    fn parse_program(&mut self) -> Result<SourceProgram, FrontendError> {
        let mut previous_top_level_end = None;
        let namespace = if self.peek_name() == Some("KwNamespace") {
            let namespace = Some(self.parse_namespace()?);
            previous_top_level_end = self.previous_token_end();
            namespace
        } else {
            None
        };
        let mut types = Vec::new();
        let mut data = Vec::new();
        let mut items = Vec::new();
        let mut imports = Vec::new();
        let mut imports_closed = false;
        while !self.is_eof() {
            self.expect_top_level_separator(previous_top_level_end)?;
            match self.peek_name() {
                Some("KwUse") if !imports_closed => imports.push(self.parse_use()?),
                Some("KwType") => {
                    imports_closed = true;
                    types.push(self.parse_type_decl()?);
                }
                Some("KwData") => {
                    imports_closed = true;
                    data.push(self.parse_data()?);
                }
                Some("KwDef") | Some("KwPrivate") | Some("Hash") => {
                    imports_closed = true;
                    let start = self.tokens[self.pos].start;
                    let attrs = self.parse_item_attrs()?;
                    let item = self.parse_def()?;
                    let item = item.with_attrs(attrs);
                    let end = self.previous_token_end().unwrap_or(start);
                    self.item_spans
                        .push(self.source_item_span(&item, start, end));
                    items.push(item);
                }
                Some(found) => {
                    let token = self.tokens[self.pos].clone();
                    return Err(FrontendError::UnexpectedToken {
                        found: found.to_string(),
                        lexeme: token.lexeme,
                        expected: vec![
                            "KwUse".to_string(),
                            "KwType".to_string(),
                            "KwData".to_string(),
                            "KwDef".to_string(),
                            "KwPrivate".to_string(),
                            "Hash".to_string(),
                        ],
                        offset: token.start,
                    });
                }
                None => break,
            }
            previous_top_level_end = self.previous_token_end();
        }
        let variants = data_variant_map(&data);
        let nominal_row_types = nominal_row_type_set(&types);
        let items = items
            .into_iter()
            .map(|item| enrich_item_with_surface_facts(item, &variants, &nominal_row_types))
            .collect();
        Ok(SourceProgram::with_surface(
            namespace, imports, types, data, items,
        ))
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

    fn parse_item_attrs(&mut self) -> Result<Vec<ItemAttr>, FrontendError> {
        let mut attrs = Vec::new();
        while self.peek_name() == Some("Hash") {
            self.pos += 1;
            self.expect("LBracket")?;
            let name = self.expect_lexeme("Ident")?;
            self.expect("RBracket")?;
            match name.as_str() {
                "entry" => attrs.push(ItemAttr::Entry),
                _ => {
                    return Err(self.unexpected_current(vec!["entry"]));
                }
            }
        }
        Ok(attrs)
    }

    fn parse_def(&mut self) -> Result<SourceItem, FrontendError> {
        let visibility = if self.peek_name() == Some("KwPrivate") {
            self.pos += 1;
            Visibility::Private
        } else {
            Visibility::Public
        };
        self.expect("KwDef")?;
        let first_name = self.expect_lexeme("Ident")?;
        let generic_params = if self.peek_name() == Some("LBracket")
            && self.peek_type_list_then(&["LParen", "Dot"])
        {
            self.parse_generic_param_decls()?
        } else {
            Vec::new()
        };
        let generics = generic_param_names(&generic_params);
        let (receiver, name) = if self.peek_name() == Some("Dot") {
            self.pos += 1;
            let method_name = self.expect_lexeme("Ident")?;
            (
                Some(MethodReceiver::new(first_name, generics.clone())),
                method_name,
            )
        } else {
            (None, first_name)
        };
        if self.peek_name() == Some("Colon") {
            if receiver.is_some() {
                return Err(self.unexpected_current(vec!["LParen"]));
            }
            self.pos += 1;
            let ty = Some(self.expect_type_name()?);
            self.expect("Eq")?;
            let body = self.parse_expr_bp(0)?;
            return Ok(SourceItem::static_value(name, ty, body).with_visibility(visibility));
        }
        if self.peek_name() == Some("Eq") {
            if receiver.is_some() {
                return Err(self.unexpected_current(vec!["LParen"]));
            }
            self.pos += 1;
            let body = self.parse_expr_bp(0)?;
            return Ok(SourceItem::static_value(name, None, body).with_visibility(visibility));
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
        if self.peek_name() == Some("KwExtern") {
            let extern_decl = self.parse_extern_decl()?;
            return Ok(SourceItem::ExternDef {
                name,
                attrs: Vec::new(),
                visibility,
                generics: receiver
                    .as_ref()
                    .map(|receiver| receiver.generics.clone())
                    .unwrap_or(generics),
                generic_params: receiver
                    .as_ref()
                    .map(|receiver| generic_param_decls_from_names(&receiver.generics))
                    .unwrap_or(generic_params),
                receiver,
                params,
                return_type,
                extern_decl,
            });
        }
        let body = self.parse_expr_bp(0)?;
        Ok(SourceItem::Def {
            name,
            attrs: Vec::new(),
            visibility,
            generics: receiver
                .as_ref()
                .map(|receiver| receiver.generics.clone())
                .unwrap_or(generics),
            generic_params: receiver
                .as_ref()
                .map(|receiver| generic_param_decls_from_names(&receiver.generics))
                .unwrap_or(generic_params),
            receiver,
            params,
            return_type,
            body,
        })
    }

    fn parse_extern_decl(&mut self) -> Result<ExternDecl, FrontendError> {
        self.expect("KwExtern")?;
        let abi_token = self.expect("StringLit")?;
        let symbol = self.expect_string_literal()?;
        let abi = match string_literal_value(&abi_token.lexeme).as_deref() {
            Some("C") | Some("c") => ExternAbi::C,
            Some("wasi") => ExternAbi::Wasi,
            _ => {
                return Err(FrontendError::UnexpectedToken {
                    found: abi_token.name,
                    lexeme: abi_token.lexeme,
                    expected: vec!["extern ABI \"C\", \"c\", or \"wasi\"".to_string()],
                    offset: abi_token.start,
                })
            }
        };
        Ok(ExternDecl::new(abi, symbol))
    }

    fn expect_string_literal(&mut self) -> Result<String, FrontendError> {
        let token = self.expect("StringLit")?;
        string_literal_value(&token.lexeme).ok_or_else(|| FrontendError::UnexpectedToken {
            found: token.name,
            lexeme: token.lexeme,
            expected: vec!["string literal".to_string()],
            offset: token.start,
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
        self.data_variants
            .insert(data.name.clone(), data.variant_names());
        Ok(data)
    }

    fn parse_type_decl(&mut self) -> Result<TypeDecl, FrontendError> {
        self.expect("KwType")?;
        let name = self.expect_lexeme("Ident")?;
        let generics = if self.peek_name() == Some("LBracket") {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };
        self.expect("Eq")?;
        if self.peek_name() != Some("LBrace") {
            let alias_target = self.expect_type_name()?;
            return Ok(TypeDecl::alias(name, generics, alias_target));
        }
        self.expect("LBrace")?;
        let mut fields = Vec::new();
        while self.peek_name() != Some("RBrace") {
            let field_name = self.expect_lexeme("Ident")?;
            self.expect("Colon")?;
            let ty = self.expect_type_name()?;
            fields.push(TypeField::new(field_name, ty));
            if self.peek_name() == Some("Comma") {
                self.pos += 1;
            }
        }
        self.expect("RBrace")?;
        Ok(TypeDecl::new(name, generics, fields))
    }

    fn parse_generic_params(&mut self) -> Result<Vec<String>, FrontendError> {
        Ok(generic_param_names(&self.parse_generic_param_decls()?))
    }

    fn parse_generic_param_decls(&mut self) -> Result<Vec<GenericParamDecl>, FrontendError> {
        self.expect("LBracket")?;
        let mut params = Vec::new();
        if self.peek_name() == Some("RBracket") {
            self.pos += 1;
            return Ok(params);
        }
        loop {
            let name = self.expect_lexeme("Ident")?;
            let bound = if self.peek_name() == Some("Colon") {
                self.pos += 1;
                Some(self.parse_generic_bound_decl()?)
            } else {
                None
            };
            params.push(GenericParamDecl { name, bound });
            if self.peek_name() != Some("Comma") {
                break;
            }
            self.pos += 1;
        }
        self.expect("RBracket")?;
        Ok(params)
    }

    fn parse_generic_bound_decl(&mut self) -> Result<GenericBoundDecl, FrontendError> {
        self.expect("LBrace")?;
        let row_marker = self.expect_lexeme("Ident")?;
        if row_marker != "r" {
            return Err(self.unexpected_current(vec!["r"]));
        }
        self.expect("Pipe")?;
        let mut fields = Vec::new();
        if self.peek_name() != Some("RBrace") {
            loop {
                let field_name = self.expect_lexeme("Ident")?;
                self.expect("Colon")?;
                let ty = self.expect_type_name()?;
                fields.push(TypeField::new(field_name, ty));
                if self.peek_name() != Some("Comma") {
                    break;
                }
                self.pos += 1;
            }
        }
        self.expect("RBrace")?;
        Ok(GenericBoundDecl::OpenRow(fields))
    }

    fn parse_type_args(&mut self) -> Result<Vec<String>, FrontendError> {
        self.expect("LBracket")?;
        let mut args = Vec::new();
        if self.peek_name() == Some("RBracket") {
            self.pos += 1;
            return Ok(args);
        }
        loop {
            args.push(self.parse_type_name()?);
            if self.peek_name() != Some("Comma") {
                break;
            }
            self.pos += 1;
        }
        self.expect("RBracket")?;
        Ok(args)
    }

    fn peek_type_list_then(&self, next: &[&str]) -> bool {
        let mut pos = self.pos;
        if self.tokens.get(pos).map(|token| token.name.as_str()) != Some("LBracket") {
            return false;
        }
        pos += 1;
        let mut square_depth = 1usize;
        let mut paren_depth = 0usize;
        let mut brace_depth = 0usize;
        while let Some(token) = self.tokens.get(pos) {
            match token.name.as_str() {
                "LBracket" => square_depth += 1,
                "RBracket" => {
                    square_depth -= 1;
                    if square_depth == 0 {
                        pos += 1;
                        return paren_depth == 0
                            && brace_depth == 0
                            && self
                                .tokens
                                .get(pos)
                                .is_some_and(|token| next.contains(&token.name.as_str()));
                    }
                }
                "LParen" => paren_depth += 1,
                "RParen" => {
                    paren_depth = paren_depth.saturating_sub(1);
                }
                "LBrace" => brace_depth += 1,
                "RBrace" => {
                    brace_depth = brace_depth.saturating_sub(1);
                }
                _ => {}
            }
            pos += 1;
        }
        false
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
            let pattern = self.parse_pattern()?;
            let ty = if self.peek_name() == Some("Colon") {
                self.pos += 1;
                Some(self.expect_type_name()?)
            } else {
                None
            };
            params.push(ParamDecl::pattern(pattern, ty));
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
            if self.peek_name() == Some("ColonEq") {
                let (lbp, rbp) = (2, 2);
                if lbp < min_bp {
                    break;
                }
                self.pos += 1;
                let rhs = self.parse_expr_bp(rbp)?;
                lhs = Expr::assign(lhs, rhs);
                continue;
            }
            if self.peek_name() == Some("PipeForward") {
                let (lbp, rbp) = (3, 4);
                if lbp < min_bp {
                    break;
                }
                self.pos += 1;
                let rhs = self.parse_expr_bp(rbp)?;
                lhs = desugar_pipe(lhs, rhs);
                continue;
            }
            if self.peek_name() == Some("DotDot") {
                let (lbp, rbp) = (5, 6);
                if lbp < min_bp {
                    break;
                }
                self.pos += 1;
                let rhs = self.parse_expr_bp(rbp)?;
                lhs = Expr::range(lhs, rhs);
                continue;
            }
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
                Some("LBracket") => {
                    if self.peek_type_list_then(&["LParen"]) && is_instantiable_callee(&expr) {
                        let type_args = self.parse_type_args()?;
                        expr = Expr::instantiate(expr, type_args);
                    } else {
                        self.pos += 1;
                        let index = self.parse_expr_bp(0)?;
                        self.expect("RBracket")?;
                        expr = Expr::index(expr, index);
                    }
                }
                Some("Dot") => {
                    self.pos += 1;
                    if self.peek_name() == Some("Star") {
                        self.pos += 1;
                        expr = Expr::method_call_args(expr, "get", Vec::new());
                        continue;
                    }
                    let name = self.expect_lexeme("Ident")?;
                    if self.peek_name() == Some("LParen") {
                        self.pos += 1;
                        let args = self.parse_expr_args()?;
                        self.expect("RParen")?;
                        expr = match expr {
                            Expr::Var(data) if self.is_known_ctor(&data, &name) => {
                                let variants = self.known_variants(&data, &name);
                                Expr::adt_ctor(data, name.clone(), variants, args)
                            }
                            receiver => Expr::method_call_args(receiver, name, args),
                        };
                    } else {
                        expr = match expr {
                            Expr::Var(data) if self.is_known_ctor(&data, &name) => {
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
        if self.peek_name() == Some("RParen") {
            return Ok(args);
        }
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
                let token = self.expect("Number")?;
                let lexeme = token.lexeme;
                lexeme
                    .parse::<i64>()
                    .map(Expr::i64)
                    .map_err(|_| FrontendError::InvalidInteger {
                        lexeme,
                        offset: token.start,
                    })
            }
            Some("True") => {
                self.pos += 1;
                Ok(Expr::bool(true))
            }
            Some("False") => {
                self.pos += 1;
                Ok(Expr::bool(false))
            }
            Some("StringLit") => {
                let token = self.expect("StringLit")?;
                parse_string_literal_expr(&token.lexeme)
            }
            Some("RawStringLit") => {
                let token = self.expect("RawStringLit")?;
                Ok(Expr::string(raw_string_literal_value(&token.lexeme)))
            }
            Some("RawHashStringLit") => {
                let token = self.expect("RawHashStringLit")?;
                Ok(Expr::string(raw_string_literal_value(&token.lexeme)))
            }
            Some("CStrLit") => {
                let token = self.expect("CStrLit")?;
                Ok(Expr::cstr(unquote_string_literal(&token.lexeme[1..])))
            }
            Some("RuneLit") => {
                let token = self.expect("RuneLit")?;
                rune_literal_value(&token.lexeme)
                    .map(Expr::rune)
                    .ok_or_else(|| FrontendError::InvalidRune {
                        lexeme: token.lexeme,
                        offset: token.start,
                    })
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
            Some("LBracket") => self.parse_slice_literal(),
            Some(found) => {
                let token = self.tokens[self.pos].clone();
                Err(FrontendError::UnexpectedToken {
                    found: found.to_string(),
                    lexeme: token.lexeme,
                    expected: vec![
                        "Number".to_string(),
                        "True".to_string(),
                        "False".to_string(),
                        "RuneLit".to_string(),
                        "CStrLit".to_string(),
                        "KwIf".to_string(),
                        "KwMatch".to_string(),
                        "KwReset".to_string(),
                        "KwResetn".to_string(),
                        "KwShift".to_string(),
                        "Ident".to_string(),
                        "LParen".to_string(),
                        "LBrace".to_string(),
                        "LBracket".to_string(),
                    ],
                    offset: token.start,
                })
            }
            None => Err(FrontendError::UnexpectedEof {
                expected: vec!["expr".to_string()],
            }),
        }
    }

    fn parse_slice_literal(&mut self) -> Result<Expr, FrontendError> {
        self.expect("LBracket")?;
        let mut fields = Vec::new();
        if self.peek_name() == Some("RBracket") {
            self.pos += 1;
            return Ok(Expr::slice_literal(fields));
        }
        loop {
            fields.push(self.parse_expr_bp(0)?);
            if self.peek_name() != Some("Comma") {
                break;
            }
            self.pos += 1;
            if self.peek_name() == Some("RBracket") {
                break;
            }
        }
        self.expect("RBracket")?;
        Ok(Expr::slice_literal(fields))
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
        let param_ty = self.expect_type_name()?;
        self.expect("RParen")?;
        self.expect("Colon")?;
        let return_ty = self.expect_type_name()?;
        self.expect("FatArrow")?;
        let body = self.parse_expr_bp(0)?;
        Ok(Expr::typed_lambda(
            param,
            Some(param_ty),
            Some(return_ty),
            body,
        ))
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

    fn parse_record_fields_until(
        &mut self,
        end: &str,
    ) -> Result<Vec<(String, Expr)>, FrontendError> {
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
        let then_branch = self.parse_block_expr()?;
        self.expect("KwElse")?;
        let else_branch = if self.peek_name() == Some("KwIf") {
            self.parse_if()?
        } else {
            self.parse_block_expr()?
        };
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
            let arm_end = self.previous_token_end();
            if self.peek_name() == Some("Comma") {
                self.pos += 1;
            } else if self.peek_name() == Some("RBrace") {
                break;
            } else if self.has_newline_before_current(arm_end) {
                continue;
            } else {
                self.expect("Comma")?;
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
            let at_offset = self.tokens[self.pos].start;
            self.pos += 1;
            let name = match pattern {
                Pattern::Bind(name) => name,
                other => {
                    return Err(FrontendError::UnexpectedToken {
                        found: render_source_pattern(&other),
                        lexeme: "@".to_string(),
                        expected: vec!["binding name before @".to_string()],
                        offset: at_offset,
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
                let token = self.expect("Number")?;
                let lexeme = token.lexeme;
                lexeme.parse::<i64>().map(Pattern::lit_i64).map_err(|_| {
                    FrontendError::InvalidInteger {
                        lexeme,
                        offset: token.start,
                    }
                })
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
                } else if self.is_known_bare_ctor(&name) {
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
                    offset: token.start,
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
                offset: token.start,
            }),
            None => Err(FrontendError::UnexpectedEof {
                expected: vec![name.to_string()],
            }),
        }
    }

    fn expect_top_level_separator(
        &mut self,
        previous_top_level_end: Option<usize>,
    ) -> Result<(), FrontendError> {
        let Some(previous_top_level_end) = previous_top_level_end else {
            return Ok(());
        };
        let Some(current) = self.tokens.get(self.pos) else {
            return Ok(());
        };
        if current.name == "Semicolon" {
            self.pos += 1;
            Ok(())
        } else if self.has_newline_between(previous_top_level_end, current.start) {
            Ok(())
        } else {
            Err(FrontendError::UnexpectedToken {
                found: current.name.clone(),
                lexeme: current.lexeme.clone(),
                expected: vec!["Newline".to_string(), "Semicolon".to_string()],
                offset: current.start,
            })
        }
    }

    fn has_newline_before_current(&self, previous_end: Option<usize>) -> bool {
        let Some(previous_end) = previous_end else {
            return false;
        };
        self.tokens
            .get(self.pos)
            .map(|current| self.has_newline_between(previous_end, current.start))
            .unwrap_or(false)
    }

    fn has_newline_between(&self, start: usize, end: usize) -> bool {
        self.source_chars[start..end].iter().any(|ch| *ch == '\n')
    }

    fn previous_token_end(&self) -> Option<usize> {
        self.pos
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map(|token| token.end)
    }

    fn source_item_span(&self, item: &SourceItem, start: usize, end: usize) -> SourceItemSpan {
        let (line, column) = self.line_column_at(start);
        let (end_line, end_column) = self.line_column_at(end);
        let (kind, name) = source_item_kind_name(item);
        SourceItemSpan {
            kind,
            name,
            span: SourceSpan {
                start,
                end,
                line,
                column,
                end_line,
                end_column,
            },
        }
    }

    fn line_column_at(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut column = 1;
        for (index, ch) in self.source_chars.iter().enumerate() {
            if index == offset {
                return (line, column);
            }
            if *ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        (line, column)
    }

    fn expect_lexeme(&mut self, name: &str) -> Result<String, FrontendError> {
        self.expect(name).map(|token| token.lexeme)
    }

    fn parse_type_name(&mut self) -> Result<String, FrontendError> {
        if self.peek_name() == Some("Ident")
            && self
                .tokens
                .get(self.pos)
                .is_some_and(|token| token.lexeme == "dyn")
        {
            self.expect("Ident")?;
            self.expect("LBrace")?;
            let mut fields = Vec::new();
            while self.peek_name() != Some("RBrace") {
                let field = self.expect_lexeme("Ident")?;
                self.expect("Colon")?;
                let ty = self.parse_type_name()?;
                fields.push(format!("{field}: {ty}"));
                if self.peek_name() == Some("Comma") {
                    self.pos += 1;
                }
            }
            self.expect("RBrace")?;
            return Ok(format!("dyn {{{}}}", fields.join(", ")));
        }
        if self.peek_name() == Some("LBrace") {
            self.expect("LBrace")?;
            let row = self.expect_lexeme("Ident")?;
            if row != "r" {
                return Err(self.unexpected_current(vec!["row marker `r`"]));
            }
            self.expect("Pipe")?;
            let mut fields = Vec::new();
            while self.peek_name() != Some("RBrace") {
                let field = self.expect_lexeme("Ident")?;
                self.expect("Colon")?;
                let ty = self.parse_type_name()?;
                fields.push(format!("{field}: {ty}"));
                if self.peek_name() == Some("Comma") {
                    self.pos += 1;
                }
            }
            self.expect("RBrace")?;
            return Ok(format!("{{r | {}}}", fields.join(", ")));
        }
        if self.peek_name() == Some("Ident")
            && self
                .tokens
                .get(self.pos)
                .is_some_and(|token| token.lexeme == "cont1" || token.lexeme == "contN")
        {
            let kind = self.expect_lexeme("Ident")?;
            let storage = match kind.as_str() {
                "cont1" => "Cont1",
                "contN" => "ContN",
                _ => unreachable!("continuation type sugar is guarded above"),
            };
            self.expect("LParen")?;
            let param = self.parse_type_name()?;
            self.expect("RParen")?;
            self.expect("Arrow")?;
            let result = self.parse_type_name()?;
            return Ok(format!("{storage}[{param},{result}]"));
        }
        if self.peek_name() == Some("LParen") {
            self.pos += 1;
            let param = self.parse_type_name()?;
            self.expect("RParen")?;
            if self
                .tokens
                .get(self.pos)
                .is_some_and(|token| token.name == "Ident" && token.lexeme == "send")
            {
                self.pos += 1;
                return Ok(format!("({param}) send"));
            }
            self.expect("Arrow")?;
            let result = self.parse_type_name()?;
            if self
                .tokens
                .get(self.pos)
                .is_some_and(|token| token.name == "Ident" && token.lexeme == "send")
            {
                self.pos += 1;
                return Ok(format!("(({param}) -> {result}) send"));
            }
            return Ok(format!("({param}) -> {result}"));
        }
        let mut name = self.expect_type_atom()?;
        if self.peek_name() == Some("LBracket") {
            let args = self.parse_type_args()?;
            name.push('[');
            name.push_str(&args.join(","));
            name.push(']');
        }
        Ok(name)
    }

    fn expect_type_name(&mut self) -> Result<String, FrontendError> {
        self.parse_type_name()
    }

    fn expect_type_atom(&mut self) -> Result<String, FrontendError> {
        match self.peek_name() {
            Some("Ident") => self.expect_lexeme("Ident"),
            Some("KwReset") => self.expect_lexeme("KwReset"),
            Some("KwResetn") => self.expect_lexeme("KwResetn"),
            Some("KwShift") => self.expect_lexeme("KwShift"),
            Some("KwMatch") => self.expect_lexeme("KwMatch"),
            Some("KwIf") => self.expect_lexeme("KwIf"),
            Some("KwLet") => self.expect_lexeme("KwLet"),
            Some("KwElse") => self.expect_lexeme("KwElse"),
            Some("True") => self.expect_lexeme("True"),
            Some("False") => self.expect_lexeme("False"),
            Some(found) => {
                let token = self.tokens[self.pos].clone();
                Err(FrontendError::UnexpectedToken {
                    found: found.to_string(),
                    lexeme: token.lexeme,
                    expected: vec!["type name".to_string()],
                    offset: token.start,
                })
            }
            None => Err(FrontendError::UnexpectedEof {
                expected: vec!["type name".to_string()],
            }),
        }
    }

    fn unexpected_current(&self, expected: Vec<&str>) -> FrontendError {
        match self.tokens.get(self.pos) {
            Some(token) => FrontendError::UnexpectedToken {
                found: token.name.clone(),
                lexeme: token.lexeme.clone(),
                expected: expected.into_iter().map(str::to_string).collect(),
                offset: token.start,
            },
            None => FrontendError::UnexpectedEof {
                expected: expected.into_iter().map(str::to_string).collect(),
            },
        }
    }

    fn peek_name(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(|token| token.name.as_str())
    }

    fn peek_next_name(&self) -> Option<&str> {
        self.tokens
            .get(self.pos + 1)
            .map(|token| token.name.as_str())
    }

    fn peek_n_name(&self, offset: usize) -> Option<&str> {
        self.tokens
            .get(self.pos + offset)
            .map(|token| token.name.as_str())
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

    fn is_known_ctor(&self, data: &str, ctor: &str) -> bool {
        self.data_variants
            .get(data)
            .map(|variants| variants.iter().any(|variant| variant == ctor))
            .unwrap_or(false)
    }

    fn is_known_bare_ctor(&self, ctor: &str) -> bool {
        self.data_variants
            .values()
            .any(|variants| variants.iter().any(|variant| variant == ctor))
    }
}

fn desugar_pipe(input: Expr, rhs: Expr) -> Expr {
    let (rhs, used_placeholder) = replace_pipe_placeholders(rhs, &input);
    if used_placeholder {
        rhs
    } else {
        pipe_default_insert(input, rhs)
    }
}

fn replace_pipe_placeholders(expr: Expr, input: &Expr) -> (Expr, bool) {
    match expr {
        Expr::Var(name) if name == "_" => (input.clone(), true),
        Expr::Var(_) | Expr::Lit(_) => (expr, false),
        Expr::Lambda {
            param,
            param_ty,
            return_ty,
            body,
        } => {
            let (body, used) = replace_pipe_placeholders(*body, input);
            (Expr::typed_lambda(param, param_ty, return_ty, body), used)
        }
        Expr::Call { callee, args } => {
            let (callee, callee_used) = replace_pipe_placeholders(*callee, input);
            let (args, args_used) = replace_pipe_placeholders_in_vec(args, input);
            (Expr::call_args(callee, args), callee_used || args_used)
        }
        Expr::Instantiate { callee, type_args } => {
            let (callee, used) = replace_pipe_placeholders(*callee, input);
            (Expr::instantiate(callee, type_args), used)
        }
        Expr::Tuple(fields) => {
            let (fields, used) = replace_pipe_placeholders_in_vec(fields, input);
            (Expr::tuple(fields), used)
        }
        Expr::SliceLiteral(items) => {
            let (items, used) = replace_pipe_placeholders_in_vec(items, input);
            (Expr::slice_literal(items), used)
        }
        Expr::Record(fields) => {
            let mut used = false;
            let fields = fields
                .into_iter()
                .map(|field| {
                    let (value, field_used) = replace_pipe_placeholders(field.value, input);
                    used |= field_used;
                    crate::ast::RecordField {
                        name: field.name,
                        value,
                    }
                })
                .collect();
            (Expr::Record(fields), used)
        }
        Expr::RecordUpdate { base, fields } => {
            let (base, base_used) = replace_pipe_placeholders(*base, input);
            let mut used = base_used;
            let fields = fields
                .into_iter()
                .map(|field| {
                    let (value, field_used) = replace_pipe_placeholders(field.value, input);
                    used |= field_used;
                    crate::ast::RecordField {
                        name: field.name,
                        value,
                    }
                })
                .collect();
            (
                Expr::RecordUpdate {
                    base: Box::new(base),
                    fields,
                },
                used,
            )
        }
        Expr::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            let (args, used) = replace_pipe_placeholders_in_vec(args, input);
            (Expr::adt_ctor(data, ctor, variants, args), used)
        }
        Expr::Field { receiver, name } => {
            let (receiver, used) = replace_pipe_placeholders(*receiver, input);
            (Expr::field(receiver, name), used)
        }
        Expr::MethodCall {
            receiver,
            name,
            args,
        } => {
            let (receiver, receiver_used) = replace_pipe_placeholders(*receiver, input);
            let (args, args_used) = replace_pipe_placeholders_in_vec(args, input);
            (
                Expr::method_call_args(receiver, name, args),
                receiver_used || args_used,
            )
        }
        Expr::Assign { target, value } => {
            let (target, target_used) = replace_pipe_placeholders(*target, input);
            let (value, value_used) = replace_pipe_placeholders(*value, input);
            (Expr::assign(target, value), target_used || value_used)
        }
        Expr::Index { receiver, index } => {
            let (receiver, receiver_used) = replace_pipe_placeholders(*receiver, input);
            let (index, index_used) = replace_pipe_placeholders(*index, input);
            (Expr::index(receiver, index), receiver_used || index_used)
        }
        Expr::Range { start, end } => {
            let (start, start_used) = replace_pipe_placeholders(*start, input);
            let (end, end_used) = replace_pipe_placeholders(*end, input);
            (Expr::range(start, end), start_used || end_used)
        }
        Expr::Binary { op, lhs, rhs } => {
            let (lhs, lhs_used) = replace_pipe_placeholders(*lhs, input);
            let (rhs, rhs_used) = replace_pipe_placeholders(*rhs, input);
            (Expr::binary(op, lhs, rhs), lhs_used || rhs_used)
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let (cond, cond_used) = replace_pipe_placeholders(*cond, input);
            let (then_branch, then_used) = replace_pipe_placeholders(*then_branch, input);
            let (else_branch, else_used) = replace_pipe_placeholders(*else_branch, input);
            (
                Expr::if_else(cond, then_branch, else_branch),
                cond_used || then_used || else_used,
            )
        }
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            let (scrutinee, scrutinee_used) = replace_pipe_placeholders(*scrutinee, input);
            let (then_branch, then_used) = replace_pipe_placeholders(*then_branch, input);
            let (else_branch, else_used) = replace_pipe_placeholders(*else_branch, input);
            (
                Expr::if_let(pattern, scrutinee, then_branch, else_branch),
                scrutinee_used || then_used || else_used,
            )
        }
        Expr::Match { scrutinee, arms } => {
            let (scrutinee, scrutinee_used) = replace_pipe_placeholders(*scrutinee, input);
            let mut used = scrutinee_used;
            let arms = arms
                .into_iter()
                .map(|arm| {
                    let (body, body_used) = replace_pipe_placeholders(arm.body, input);
                    used |= body_used;
                    (arm.pattern, body)
                })
                .collect();
            (Expr::match_expr(scrutinee, arms), used)
        }
        Expr::Nominal { name, expr } => {
            let (expr, used) = replace_pipe_placeholders(*expr, input);
            (Expr::nominal(name, expr), used)
        }
        Expr::Reset { multi, body } => {
            let (body, used) = replace_pipe_placeholders(*body, input);
            if multi {
                (Expr::resetn(body), used)
            } else {
                (Expr::reset(body), used)
            }
        }
        Expr::Shift { binder, body } => {
            let (body, used) = replace_pipe_placeholders(*body, input);
            (Expr::shift(binder, body), used)
        }
    }
}

fn replace_pipe_placeholders_in_vec(values: Vec<Expr>, input: &Expr) -> (Vec<Expr>, bool) {
    let mut used = false;
    let values = values
        .into_iter()
        .map(|value| {
            let (value, value_used) = replace_pipe_placeholders(value, input);
            used |= value_used;
            value
        })
        .collect();
    (values, used)
}

fn pipe_default_insert(input: Expr, rhs: Expr) -> Expr {
    match rhs {
        Expr::Var(name) => Expr::call_args(Expr::var(name), vec![input]),
        Expr::Call { callee, args } => {
            let mut args = args;
            args.insert(0, input);
            Expr::call_args(*callee, args)
        }
        Expr::Field { receiver, name, .. } if is_type_or_namespace_path(&receiver) => {
            Expr::method_call_args(input, name, Vec::new())
        }
        Expr::MethodCall {
            receiver,
            name,
            args,
        } if is_type_or_namespace_path(&receiver) => Expr::method_call_args(input, name, args),
        other => Expr::call_args(other, vec![input]),
    }
}

fn is_type_or_namespace_path(expr: &Expr) -> bool {
    match expr {
        Expr::Var(_) => true,
        Expr::Field { receiver, .. } => is_type_or_namespace_path(receiver),
        _ => false,
    }
}

fn unquote_string_literal(lexeme: &str) -> String {
    let body = lexeme
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(lexeme);
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

fn parse_string_literal_expr(lexeme: &str) -> Result<Expr, FrontendError> {
    let Some(body) = lexeme
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
    else {
        return Ok(Expr::string(unquote_string_literal(lexeme)));
    };
    let Some(parts) = interpolation_parts(body)? else {
        return Ok(Expr::string(unquote_string_literal(lexeme)));
    };
    Ok(desugar_string_interpolation(parts))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum StringInterpolationPart {
    Text(String),
    Expr(Expr),
}

fn interpolation_parts(body: &str) -> Result<Option<Vec<StringInterpolationPart>>, FrontendError> {
    let chars = body.chars().collect::<Vec<_>>();
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut index = 0;
    let mut saw_expr = false;
    while index < chars.len() {
        if chars[index] == '\\' {
            if let Some(next) = chars.get(index + 1).copied() {
                text.push('\\');
                text.push(next);
                index += 2;
            } else {
                text.push('\\');
                index += 1;
            }
            continue;
        }
        if chars[index] == '$' && chars.get(index + 1) == Some(&'{') {
            if !text.is_empty() {
                parts.push(StringInterpolationPart::Text(unescape_string_body(&text)));
                text.clear();
            }
            let Some((expr_text, next_index)) = interpolation_expr_text(&chars, index + 2) else {
                return Err(FrontendError::UnexpectedEof {
                    expected: vec!["}".to_string()],
                });
            };
            let expr = parse_interpolation_expr(&expr_text)?;
            parts.push(StringInterpolationPart::Expr(expr));
            saw_expr = true;
            index = next_index;
            continue;
        }
        text.push(chars[index]);
        index += 1;
    }
    if !text.is_empty() {
        parts.push(StringInterpolationPart::Text(unescape_string_body(&text)));
    }
    Ok(saw_expr.then_some(parts))
}

fn interpolation_expr_text(chars: &[char], mut index: usize) -> Option<(String, usize)> {
    let start = index;
    let mut depth = 1usize;
    while index < chars.len() {
        match chars[index] {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some((chars[start..index].iter().collect(), index + 1));
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn parse_interpolation_expr(source: &str) -> Result<Expr, FrontendError> {
    let lexer = compile_lexer(chiba_lexer_spec()).map_err(|_| FrontendError::UnexpectedEof {
        expected: vec!["interpolation expression".to_string()],
    })?;
    let tokens = lexer
        .lex(source)
        .map_err(|_| FrontendError::UnexpectedToken {
            found: "InvalidInterpolation".to_string(),
            lexeme: source.to_string(),
            expected: vec!["expression".to_string()],
            offset: 0,
        })?;
    let mut parser = FrontendParser::new(tokens, source);
    let expr = parser.parse_expr_bp(0)?;
    if parser.is_eof() {
        Ok(expr)
    } else {
        let token = parser.tokens.get(parser.pos);
        Err(FrontendError::UnexpectedToken {
            found: parser.peek_name().unwrap_or("Eof").to_string(),
            lexeme: token.map(|token| token.lexeme.clone()).unwrap_or_default(),
            expected: vec!["end of interpolation expression".to_string()],
            offset: token.map(|token| token.start).unwrap_or(source.len()),
        })
    }
}

fn unescape_string_body(body: &str) -> String {
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

fn desugar_string_interpolation(parts: Vec<StringInterpolationPart>) -> Expr {
    let mut expr = Expr::string("");
    for part in parts {
        let segment = match part {
            StringInterpolationPart::Text(text) => Expr::string(text),
            StringInterpolationPart::Expr(expr) => {
                Expr::method_call_args(Expr::var("String"), "from", vec![expr])
            }
        };
        expr = Expr::method_call(expr, "concat", segment);
    }
    expr
}

fn is_instantiable_callee(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Var(_) | Expr::Field { .. } | Expr::MethodCall { .. }
    )
}

fn string_literal_value(lexeme: &str) -> Option<String> {
    let inner = lexeme.strip_prefix('"')?.strip_suffix('"')?;
    let mut value = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            value.push(ch);
            continue;
        }
        let escaped = chars.next()?;
        match escaped {
            '"' => value.push('"'),
            '\\' => value.push('\\'),
            'n' => value.push('\n'),
            't' => value.push('\t'),
            'r' => value.push('\r'),
            other => value.push(other),
        }
    }
    Some(value)
}

fn raw_string_literal_value(lexeme: &str) -> String {
    if let Some(inner) = lexeme
        .strip_prefix("r#\"")
        .and_then(|text| text.strip_suffix("\"#"))
    {
        return inner.to_string();
    }
    lexeme
        .strip_prefix("r\"")
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(lexeme)
        .to_string()
}

fn rune_literal_value(lexeme: &str) -> Option<u32> {
    let inner = lexeme.strip_prefix('\'')?.strip_suffix('\'')?;
    let mut value = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            value.push(ch);
            continue;
        }
        let escaped = chars.next()?;
        match escaped {
            '\'' => value.push('\''),
            '"' => value.push('"'),
            '\\' => value.push('\\'),
            'n' => value.push('\n'),
            't' => value.push('\t'),
            'r' => value.push('\r'),
            other => value.push(other),
        }
    }
    let mut scalars = value.chars();
    let rune = scalars.next()?;
    if scalars.next().is_some() {
        return None;
    }
    Some(rune as u32)
}

fn data_variant_map(data: &[DataDecl]) -> BTreeMap<String, Vec<String>> {
    data.iter()
        .map(|decl| (decl.name.clone(), decl.variant_names()))
        .collect()
}

fn nominal_row_type_set(types: &[TypeDecl]) -> BTreeSet<String> {
    types
        .iter()
        .filter(|decl| decl.alias_target.is_none())
        .map(|decl| decl.name.clone())
        .collect()
}

fn enrich_item_with_surface_facts(
    item: SourceItem,
    variants: &BTreeMap<String, Vec<String>>,
    nominal_row_types: &BTreeSet<String>,
) -> SourceItem {
    match item {
        SourceItem::Def {
            name,
            attrs,
            visibility,
            receiver,
            generics,
            generic_params,
            params,
            return_type,
            body,
        } => SourceItem::Def {
            name,
            attrs,
            visibility,
            receiver,
            generics,
            generic_params,
            params,
            return_type,
            body: enrich_expr_with_surface_facts(body, variants, nominal_row_types),
        },
        SourceItem::ExternDef {
            name,
            attrs,
            visibility,
            receiver,
            generics,
            generic_params,
            params,
            return_type,
            extern_decl,
        } => SourceItem::ExternDef {
            name,
            attrs,
            visibility,
            receiver,
            generics,
            generic_params,
            params,
            return_type,
            extern_decl,
        },
        SourceItem::StaticValue {
            name,
            attrs,
            visibility,
            ty,
            body,
        } => SourceItem::StaticValue {
            name,
            attrs,
            visibility,
            ty,
            body: enrich_expr_with_surface_facts(body, variants, nominal_row_types),
        },
    }
}

fn source_item_kind_name(item: &SourceItem) -> (String, String) {
    match item {
        SourceItem::Def { name, .. } => ("def".to_string(), name.clone()),
        SourceItem::ExternDef { name, .. } => ("extern".to_string(), name.clone()),
        SourceItem::StaticValue { name, .. } => ("static".to_string(), name.clone()),
    }
}

fn enrich_expr_with_surface_facts(
    expr: Expr,
    variants: &BTreeMap<String, Vec<String>>,
    nominal_row_types: &BTreeSet<String>,
) -> Expr {
    match expr {
        Expr::AdtCtor {
            data,
            ctor,
            variants: old_variants,
            args,
        } => {
            let args = args
                .into_iter()
                .map(|arg| enrich_expr_with_surface_facts(arg, variants, nominal_row_types))
                .collect();
            let variants = variants.get(&data).cloned().unwrap_or(old_variants);
            Expr::adt_ctor(data, ctor, variants, args)
        }
        Expr::Lambda {
            param,
            param_ty,
            return_ty,
            body,
        } => Expr::typed_lambda(
            param,
            param_ty,
            return_ty,
            enrich_expr_with_surface_facts(*body, variants, nominal_row_types),
        ),
        Expr::Call { callee, args } => {
            let callee = enrich_expr_with_surface_facts(*callee, variants, nominal_row_types);
            let args = args
                .into_iter()
                .map(|arg| enrich_expr_with_surface_facts(arg, variants, nominal_row_types))
                .collect::<Vec<_>>();
            match (callee, args.as_slice()) {
                (Expr::Var(name), [arg]) if nominal_row_types.contains(&name) => {
                    Expr::nominal(name, arg.clone())
                }
                (callee, args) => Expr::call_args(callee, args.to_vec()),
            }
        }
        Expr::Instantiate { callee, type_args } => Expr::instantiate(
            enrich_expr_with_surface_facts(*callee, variants, nominal_row_types),
            type_args,
        ),
        Expr::Tuple(fields) => Expr::tuple(
            fields
                .into_iter()
                .map(|field| enrich_expr_with_surface_facts(field, variants, nominal_row_types))
                .collect(),
        ),
        Expr::SliceLiteral(items) => Expr::slice_literal(
            items
                .into_iter()
                .map(|item| enrich_expr_with_surface_facts(item, variants, nominal_row_types))
                .collect(),
        ),
        Expr::Record(fields) => Expr::Record(
            fields
                .into_iter()
                .map(|field| crate::ast::RecordField {
                    name: field.name,
                    value: enrich_expr_with_surface_facts(field.value, variants, nominal_row_types),
                })
                .collect(),
        ),
        Expr::RecordUpdate { base, fields } => Expr::RecordUpdate {
            base: Box::new(enrich_expr_with_surface_facts(
                *base,
                variants,
                nominal_row_types,
            )),
            fields: fields
                .into_iter()
                .map(|field| crate::ast::RecordField {
                    name: field.name,
                    value: enrich_expr_with_surface_facts(field.value, variants, nominal_row_types),
                })
                .collect(),
        },
        Expr::Field { receiver, name } => Expr::field(
            enrich_expr_with_surface_facts(*receiver, variants, nominal_row_types),
            name,
        ),
        Expr::MethodCall {
            receiver,
            name,
            args,
        } => {
            let receiver = enrich_expr_with_surface_facts(*receiver, variants, nominal_row_types);
            let args = args
                .into_iter()
                .map(|arg| enrich_expr_with_surface_facts(arg, variants, nominal_row_types))
                .collect::<Vec<_>>();
            match receiver {
                Expr::Var(data) if data_ctor_known(variants, &data, &name) => {
                    let ctor_variants = variants[&data].clone();
                    Expr::adt_ctor(data, name.clone(), ctor_variants, args)
                }
                receiver => Expr::method_call_args(receiver, name, args),
            }
        }
        Expr::Assign { target, value } => Expr::assign(
            enrich_expr_with_surface_facts(*target, variants, nominal_row_types),
            enrich_expr_with_surface_facts(*value, variants, nominal_row_types),
        ),
        Expr::Index { receiver, index } => Expr::index(
            enrich_expr_with_surface_facts(*receiver, variants, nominal_row_types),
            enrich_expr_with_surface_facts(*index, variants, nominal_row_types),
        ),
        Expr::Range { start, end } => Expr::range(
            enrich_expr_with_surface_facts(*start, variants, nominal_row_types),
            enrich_expr_with_surface_facts(*end, variants, nominal_row_types),
        ),
        Expr::Binary { op, lhs, rhs } => Expr::binary(
            op,
            enrich_expr_with_surface_facts(*lhs, variants, nominal_row_types),
            enrich_expr_with_surface_facts(*rhs, variants, nominal_row_types),
        ),
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => Expr::if_else(
            enrich_expr_with_surface_facts(*cond, variants, nominal_row_types),
            enrich_expr_with_surface_facts(*then_branch, variants, nominal_row_types),
            enrich_expr_with_surface_facts(*else_branch, variants, nominal_row_types),
        ),
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => Expr::if_let(
            pattern,
            enrich_expr_with_surface_facts(*scrutinee, variants, nominal_row_types),
            enrich_expr_with_surface_facts(*then_branch, variants, nominal_row_types),
            enrich_expr_with_surface_facts(*else_branch, variants, nominal_row_types),
        ),
        Expr::Match { scrutinee, arms } => Expr::match_expr(
            enrich_expr_with_surface_facts(*scrutinee, variants, nominal_row_types),
            arms.into_iter()
                .map(|arm| {
                    (
                        arm.pattern,
                        enrich_expr_with_surface_facts(arm.body, variants, nominal_row_types),
                    )
                })
                .collect(),
        ),
        Expr::Nominal { name, expr } => Expr::nominal(
            name,
            enrich_expr_with_surface_facts(*expr, variants, nominal_row_types),
        ),
        Expr::Reset { multi, body } => {
            let body = enrich_expr_with_surface_facts(*body, variants, nominal_row_types);
            if multi {
                Expr::resetn(body)
            } else {
                Expr::reset(body)
            }
        }
        Expr::Shift { binder, body } => Expr::shift(
            binder,
            enrich_expr_with_surface_facts(*body, variants, nominal_row_types),
        ),
        Expr::Var(_) | Expr::Lit(_) => expr,
    }
}

fn data_ctor_known(variants: &BTreeMap<String, Vec<String>>, data: &str, ctor: &str) -> bool {
    variants
        .get(data)
        .map(|items| items.iter().any(|item| item == ctor))
        .unwrap_or(false)
}

fn generic_param_names(params: &[GenericParamDecl]) -> Vec<String> {
    params.iter().map(|param| param.name.clone()).collect()
}
