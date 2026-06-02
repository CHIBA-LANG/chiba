use crate::ast::{BinaryOp, Expr, SourceItem, SourceProgram};
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
}

impl FrontendParser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<SourceProgram, FrontendError> {
        let mut items = Vec::new();
        while !self.is_eof() {
            items.push(self.parse_def()?);
        }
        Ok(SourceProgram { items })
    }

    fn parse_def(&mut self) -> Result<SourceItem, FrontendError> {
        self.expect("KwDef")?;
        let name = self.expect_lexeme("Ident")?;
        self.expect("LParen")?;
        let params = self.parse_params()?;
        self.expect("RParen")?;
        self.expect("Eq")?;
        let body = self.parse_expr_bp(0)?;
        Ok(SourceItem::Def { name, params, body })
    }

    fn parse_params(&mut self) -> Result<Vec<String>, FrontendError> {
        let mut params = Vec::new();
        if self.peek_name() == Some("RParen") {
            return Ok(params);
        }
        loop {
            params.push(self.expect_lexeme("Ident")?);
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
        while self.peek_name() == Some("LParen") {
            self.pos += 1;
            let arg = self.parse_expr_bp(0)?;
            self.expect("RParen")?;
            expr = Expr::call(expr, arg);
        }
        Ok(expr)
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
            Some("Ident") => self.expect_lexeme("Ident").map(Expr::var),
            Some("LParen") => {
                self.pos += 1;
                let expr = self.parse_expr_bp(0)?;
                self.expect("RParen")?;
                Ok(expr)
            }
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
                        "Ident".to_string(),
                        "LParen".to_string(),
                    ],
                })
            }
            None => Err(FrontendError::UnexpectedEof {
                expected: vec!["expr".to_string()],
            }),
        }
    }

    fn parse_if(&mut self) -> Result<Expr, FrontendError> {
        self.expect("KwIf")?;
        let cond = self.parse_expr_bp(0)?;
        self.expect("KwThen")?;
        let then_branch = self.parse_expr_bp(0)?;
        self.expect("KwElse")?;
        let else_branch = self.parse_expr_bp(0)?;
        Ok(Expr::if_else(cond, then_branch, else_branch))
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

    fn peek_name(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(|token| token.name.as_str())
    }

    fn is_eof(&self) -> bool {
        self.pos == self.tokens.len()
    }
}
