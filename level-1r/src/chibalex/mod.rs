use crate::regex::{compile, parse, RegexError, RegexProgram};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexerSpec {
    pub rules: Vec<LexerRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexerRule {
    pub name: String,
    pub pattern: String,
    pub skip: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lexer {
    rules: Vec<CompiledRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompiledRule {
    name: String,
    program: RegexProgram,
    skip: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub name: String,
    pub lexeme: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LexError {
    Regex(RegexError),
    NoRule { offset: usize, found: Option<char> },
    EmptyMatch { rule: String, offset: usize },
}

pub fn compile_lexer(spec: LexerSpec) -> Result<Lexer, LexError> {
    let mut rules = Vec::new();
    for rule in spec.rules {
        let ast = parse(&rule.pattern).map_err(LexError::Regex)?;
        rules.push(CompiledRule {
            name: rule.name,
            program: compile(&ast),
            skip: rule.skip,
        });
    }
    Ok(Lexer { rules })
}

impl Lexer {
    pub fn lex(&self, input: &str) -> Result<Vec<Token>, LexError> {
        let chars = input.chars().collect::<Vec<_>>();
        let mut tokens = Vec::new();
        let mut offset = 0;
        while offset < chars.len() {
            let best = self.best_match(&chars, offset)?;
            if best.end == offset {
                return Err(LexError::EmptyMatch {
                    rule: self.rules[best.rule].name.clone(),
                    offset,
                });
            }
            if !self.rules[best.rule].skip {
                tokens.push(Token {
                    name: self.rules[best.rule].name.clone(),
                    lexeme: chars[offset..best.end].iter().collect(),
                    start: offset,
                    end: best.end,
                });
            }
            offset = best.end;
        }
        Ok(tokens)
    }

    fn best_match(&self, chars: &[char], offset: usize) -> Result<BestMatch, LexError> {
        let mut best = None;
        for (rule, compiled) in self.rules.iter().enumerate() {
            if let Some(end) = compiled.program.match_from(chars, offset) {
                if end >= offset
                    && best
                        .as_ref()
                        .map(|current: &BestMatch| end > current.end)
                        .unwrap_or(true)
                {
                    best = Some(BestMatch { rule, end });
                }
            }
        }
        best.ok_or_else(|| LexError::NoRule {
            offset,
            found: chars.get(offset).copied(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BestMatch {
    rule: usize,
    end: usize,
}
