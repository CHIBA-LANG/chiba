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
pub enum Literal {
    I64(i64),
    Bool(bool),
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
