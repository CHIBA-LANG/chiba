#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceProgram {
    pub items: Vec<SourceItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceItem {
    Def {
        name: String,
        params: Vec<String>,
        body: Expr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Var(String),
    Lit(Literal),
    Lambda {
        param: String,
        body: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arg: Box<Expr>,
    },
    Tuple(Vec<Expr>),
    Record(Vec<RecordField>),
    RecordUpdate {
        base: Box<Expr>,
        fields: Vec<RecordField>,
    },
    AdtCtor {
        data: String,
        ctor: String,
        variants: Vec<String>,
        args: Vec<Expr>,
    },
    Field {
        receiver: Box<Expr>,
        name: String,
    },
    MethodCall {
        receiver: Box<Expr>,
        name: String,
        arg: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    IfLet {
        pattern: Pattern,
        scrutinee: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Nominal {
        name: String,
        expr: Box<Expr>,
    },
    Reset {
        multi: bool,
        body: Box<Expr>,
    },
    Shift {
        binder: String,
        body: Box<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordField {
    pub name: String,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    Wildcard,
    Bind(String),
    Tuple(Vec<Pattern>),
    Record(Vec<RecordPatternField>),
    Constructor {
        data: Option<String>,
        ctor: String,
        args: Vec<Pattern>,
    },
    At {
        name: String,
        pattern: Box<Pattern>,
    },
    Lit(Literal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordPatternField {
    pub name: String,
    pub pattern: Pattern,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    I64(i64),
    Bool(bool),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl Expr {
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var(name.into())
    }

    pub fn i64(value: i64) -> Self {
        Self::Lit(Literal::I64(value))
    }

    pub fn bool(value: bool) -> Self {
        Self::Lit(Literal::Bool(value))
    }

    pub fn lambda(param: impl Into<String>, body: Expr) -> Self {
        Self::Lambda {
            param: param.into(),
            body: Box::new(body),
        }
    }

    pub fn call(callee: Expr, arg: Expr) -> Self {
        Self::Call {
            callee: Box::new(callee),
            arg: Box::new(arg),
        }
    }

    pub fn tuple(fields: Vec<Expr>) -> Self {
        Self::Tuple(fields)
    }

    pub fn record(fields: Vec<(impl Into<String>, Expr)>) -> Self {
        Self::Record(
            fields
                .into_iter()
                .map(|(name, value)| RecordField {
                    name: name.into(),
                    value,
                })
                .collect(),
        )
    }

    pub fn record_update(base: Expr, fields: Vec<(impl Into<String>, Expr)>) -> Self {
        Self::RecordUpdate {
            base: Box::new(base),
            fields: fields
                .into_iter()
                .map(|(name, value)| RecordField {
                    name: name.into(),
                    value,
                })
                .collect(),
        }
    }

    pub fn adt_ctor(
        data: impl Into<String>,
        ctor: impl Into<String>,
        variants: Vec<impl Into<String>>,
        args: Vec<Expr>,
    ) -> Self {
        Self::AdtCtor {
            data: data.into(),
            ctor: ctor.into(),
            variants: variants.into_iter().map(Into::into).collect(),
            args,
        }
    }

    pub fn field(receiver: Expr, name: impl Into<String>) -> Self {
        Self::Field {
            receiver: Box::new(receiver),
            name: name.into(),
        }
    }

    pub fn method_call(receiver: Expr, name: impl Into<String>, arg: Expr) -> Self {
        Self::MethodCall {
            receiver: Box::new(receiver),
            name: name.into(),
            arg: Box::new(arg),
        }
    }

    pub fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Self {
        Self::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn if_else(cond: Expr, then_branch: Expr, else_branch: Expr) -> Self {
        Self::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    }

    pub fn match_expr(scrutinee: Expr, arms: Vec<(Pattern, Expr)>) -> Self {
        Self::Match {
            scrutinee: Box::new(scrutinee),
            arms: arms
                .into_iter()
                .map(|(pattern, body)| MatchArm { pattern, body })
                .collect(),
        }
    }

    pub fn if_let(pattern: Pattern, scrutinee: Expr, then_branch: Expr, else_branch: Expr) -> Self {
        Self::IfLet {
            pattern,
            scrutinee: Box::new(scrutinee),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    }

    pub fn nominal(name: impl Into<String>, expr: Expr) -> Self {
        Self::Nominal {
            name: name.into(),
            expr: Box::new(expr),
        }
    }

    pub fn reset(body: Expr) -> Self {
        Self::Reset {
            multi: false,
            body: Box::new(body),
        }
    }

    pub fn resetn(body: Expr) -> Self {
        Self::Reset {
            multi: true,
            body: Box::new(body),
        }
    }

    pub fn shift(binder: impl Into<String>, body: Expr) -> Self {
        Self::Shift {
            binder: binder.into(),
            body: Box::new(body),
        }
    }
}

impl Pattern {
    pub fn wildcard() -> Self {
        Self::Wildcard
    }

    pub fn bind(name: impl Into<String>) -> Self {
        Self::Bind(name.into())
    }

    pub fn tuple(fields: Vec<Pattern>) -> Self {
        Self::Tuple(fields)
    }

    pub fn record(fields: Vec<(impl Into<String>, Pattern)>) -> Self {
        Self::Record(
            fields
                .into_iter()
                .map(|(name, pattern)| RecordPatternField {
                    name: name.into(),
                    pattern,
                })
                .collect(),
        )
    }

    pub fn ctor(ctor: impl Into<String>, args: Vec<Pattern>) -> Self {
        Self::Constructor {
            data: None,
            ctor: ctor.into(),
            args,
        }
    }

    pub fn qualified_ctor(data: impl Into<String>, ctor: impl Into<String>, args: Vec<Pattern>) -> Self {
        Self::Constructor {
            data: Some(data.into()),
            ctor: ctor.into(),
            args,
        }
    }

    pub fn at(name: impl Into<String>, pattern: Pattern) -> Self {
        Self::At {
            name: name.into(),
            pattern: Box::new(pattern),
        }
    }

    pub fn lit_i64(value: i64) -> Self {
        Self::Lit(Literal::I64(value))
    }

    pub fn lit_bool(value: bool) -> Self {
        Self::Lit(Literal::Bool(value))
    }
}
