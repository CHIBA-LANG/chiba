use crate::symbol::{is_chiba_identifier_continue, is_chiba_identifier_start};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegexAst {
    Empty,
    Literal(char),
    Any,
    Class(Vec<ClassAtom>, bool),
    Seq(Vec<RegexAst>),
    Alt(Box<RegexAst>, Box<RegexAst>),
    Repeat {
        child: Box<RegexAst>,
        kind: RepeatKind,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RepeatKind {
    ZeroOrMore,
    OneOrMore,
    Optional,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegexError {
    UnexpectedEnd,
    UnclosedClass,
    UnsupportedPcreFeature(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegexProgram {
    pub code: Vec<RegexInst>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegexInst {
    Char(char),
    Any,
    Class { atoms: Vec<ClassAtom>, negated: bool },
    Split(usize, usize),
    Jump(usize),
    Accept,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassAtom {
    Char(char),
    Range(char, char),
    XidStart,
    XidContinue,
}

pub fn parse(pattern: &str) -> Result<RegexAst, RegexError> {
    Parser::new(pattern).parse_alt()
}

pub fn compile(ast: &RegexAst) -> RegexProgram {
    let mut code = Vec::new();
    compile_into(ast, &mut code);
    code.push(RegexInst::Accept);
    RegexProgram { code }
}

pub fn is_match(pattern: &str, subject: &str) -> Result<bool, RegexError> {
    let ast = parse(pattern)?;
    Ok(compile(&ast).is_match(subject))
}

impl RegexProgram {
    pub fn is_match(&self, subject: &str) -> bool {
        let chars = subject.chars().collect::<Vec<_>>();
        (0..=chars.len()).any(|start| self.match_from(&chars, start).is_some())
    }

    pub fn match_from(&self, chars: &[char], start: usize) -> Option<usize> {
        self.step(0, start, chars, 0)
    }

    fn step(&self, pc: usize, offset: usize, chars: &[char], depth: usize) -> Option<usize> {
        if depth > self.code.len().saturating_mul(chars.len().saturating_add(1)) {
            return None;
        }
        match self.code.get(pc)? {
            RegexInst::Accept => Some(offset),
            RegexInst::Char(expected) => {
                if chars.get(offset) == Some(expected) {
                    self.step(pc + 1, offset + 1, chars, depth + 1)
                } else {
                    None
                }
            }
            RegexInst::Any => {
                if offset < chars.len() {
                    self.step(pc + 1, offset + 1, chars, depth + 1)
                } else {
                    None
                }
            }
            RegexInst::Class { atoms, negated } => {
                let Some(ch) = chars.get(offset) else {
                    return None;
                };
                let hit = atoms.iter().any(|atom| atom.matches(*ch));
                if hit != *negated {
                    self.step(pc + 1, offset + 1, chars, depth + 1)
                } else {
                    None
                }
            }
            RegexInst::Split(left, right) => self
                .step(*left, offset, chars, depth + 1)
                .or_else(|| self.step(*right, offset, chars, depth + 1)),
            RegexInst::Jump(target) => self.step(*target, offset, chars, depth + 1),
        }
    }
}

fn compile_into(ast: &RegexAst, code: &mut Vec<RegexInst>) {
    match ast {
        RegexAst::Empty => {}
        RegexAst::Literal(ch) => code.push(RegexInst::Char(*ch)),
        RegexAst::Any => code.push(RegexInst::Any),
        RegexAst::Class(atoms, negated) => code.push(RegexInst::Class {
            atoms: atoms.clone(),
            negated: *negated,
        }),
        RegexAst::Seq(items) => {
            for item in items {
                compile_into(item, code);
            }
        }
        RegexAst::Alt(left, right) => {
            let split = code.len();
            code.push(RegexInst::Split(0, 0));
            let left_start = code.len();
            compile_into(left, code);
            let jump = code.len();
            code.push(RegexInst::Jump(0));
            let right_start = code.len();
            compile_into(right, code);
            let end = code.len();
            code[split] = RegexInst::Split(left_start, right_start);
            code[jump] = RegexInst::Jump(end);
        }
        RegexAst::Repeat { child, kind } => match kind {
            RepeatKind::ZeroOrMore => {
                let split = code.len();
                code.push(RegexInst::Split(0, 0));
                let child_start = code.len();
                compile_into(child, code);
                code.push(RegexInst::Jump(split));
                let end = code.len();
                code[split] = RegexInst::Split(child_start, end);
            }
            RepeatKind::OneOrMore => {
                let child_start = code.len();
                compile_into(child, code);
                let split = code.len();
                code.push(RegexInst::Split(child_start, split + 1));
            }
            RepeatKind::Optional => {
                let split = code.len();
                code.push(RegexInst::Split(0, 0));
                let child_start = code.len();
                compile_into(child, code);
                let end = code.len();
                code[split] = RegexInst::Split(child_start, end);
            }
        },
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn parse_alt(&mut self) -> Result<RegexAst, RegexError> {
        let mut left = self.parse_seq()?;
        while self.peek() == Some('|') {
            self.pos += 1;
            let right = self.parse_seq()?;
            left = RegexAst::Alt(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_seq(&mut self) -> Result<RegexAst, RegexError> {
        let mut items = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == ')' || ch == '|' {
                break;
            }
            items.push(self.parse_repeat()?);
        }
        Ok(match items.len() {
            0 => RegexAst::Empty,
            1 => items.remove(0),
            _ => RegexAst::Seq(items),
        })
    }

    fn parse_repeat(&mut self) -> Result<RegexAst, RegexError> {
        let atom = self.parse_atom()?;
        Ok(match self.peek() {
            Some('*') => {
                self.pos += 1;
                RegexAst::Repeat {
                    child: Box::new(atom),
                    kind: RepeatKind::ZeroOrMore,
                }
            }
            Some('+') => {
                self.pos += 1;
                RegexAst::Repeat {
                    child: Box::new(atom),
                    kind: RepeatKind::OneOrMore,
                }
            }
            Some('?') => {
                self.pos += 1;
                RegexAst::Repeat {
                    child: Box::new(atom),
                    kind: RepeatKind::Optional,
                }
            }
            _ => atom,
        })
    }

    fn parse_atom(&mut self) -> Result<RegexAst, RegexError> {
        match self.next().ok_or(RegexError::UnexpectedEnd)? {
            '.' => Ok(RegexAst::Any),
            '[' => self.parse_class(),
            '(' => {
                if self.peek() == Some('?') {
                    return Err(RegexError::UnsupportedPcreFeature("lookaround"));
                }
                let ast = self.parse_alt()?;
                if self.peek() == Some(')') {
                    self.pos += 1;
                }
                Ok(ast)
            }
            '\\' => match self.next().ok_or(RegexError::UnexpectedEnd)? {
                'd' => Ok(RegexAst::Class(vec![ClassAtom::Range('0', '9')], false)),
                'w' => Ok(RegexAst::Class(
                    vec![
                        ClassAtom::Range('a', 'z'),
                        ClassAtom::Range('A', 'Z'),
                        ClassAtom::Range('0', '9'),
                        ClassAtom::Char('_'),
                    ],
                    false,
                )),
                'p' if self.peek() == Some('{') => self.parse_property_escape(),
                other @ ('1'..='9') => {
                    let _ = other;
                    Err(RegexError::UnsupportedPcreFeature("backref"))
                }
                other => Ok(RegexAst::Literal(other)),
            },
            ch => Ok(RegexAst::Literal(ch)),
        }
    }

    fn parse_class(&mut self) -> Result<RegexAst, RegexError> {
        let negated = if self.peek() == Some('^') {
            self.pos += 1;
            true
        } else {
            false
        };
        let mut atoms = Vec::new();
        while let Some(ch) = self.next() {
            if ch == ']' {
                return Ok(RegexAst::Class(atoms, negated));
            }
            if self.peek() == Some('-') {
                self.pos += 1;
                let end = self.next().ok_or(RegexError::UnclosedClass)?;
                atoms.push(ClassAtom::Range(ch, end));
            } else {
                atoms.push(ClassAtom::Char(ch));
            }
        }
        Err(RegexError::UnclosedClass)
    }

    fn parse_property_escape(&mut self) -> Result<RegexAst, RegexError> {
        self.pos += 1;
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch == '}' {
                let name = self.chars[start..self.pos].iter().collect::<String>();
                self.pos += 1;
                return match name.as_str() {
                    "XID_START" => Ok(RegexAst::Class(vec![ClassAtom::XidStart], false)),
                    "XID_CONTINUE" => Ok(RegexAst::Class(vec![ClassAtom::XidContinue], false)),
                    _ => Err(RegexError::UnsupportedPcreFeature("property")),
                };
            }
            self.pos += 1;
        }
        Err(RegexError::UnexpectedEnd)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }
}

impl ClassAtom {
    fn matches(&self, ch: char) -> bool {
        match self {
            ClassAtom::Char(expected) => *expected == ch,
            ClassAtom::Range(start, end) => *start <= ch && ch <= *end,
            ClassAtom::XidStart => is_chiba_identifier_start(ch),
            ClassAtom::XidContinue => is_chiba_identifier_continue(ch),
        }
    }
}
