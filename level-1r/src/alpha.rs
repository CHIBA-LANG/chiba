use std::collections::BTreeMap;
use std::fmt;

use crate::ast::{Expr, Literal};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BinderId(pub usize);

impl fmt::Display for BinderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlphaExpr {
    pub kind: AlphaExprKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlphaExprKind {
    Var(ResolvedVar),
    Lit(Literal),
    Lambda {
        param: AlphaBinder,
        body: Box<AlphaExpr>,
    },
    Call {
        callee: Box<AlphaExpr>,
        arg: Box<AlphaExpr>,
    },
    Reset {
        multi: bool,
        body: Box<AlphaExpr>,
    },
    Shift {
        binder: AlphaBinder,
        body: Box<AlphaExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlphaBinder {
    pub id: BinderId,
    pub name: String,
    pub namespace: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedVar {
    pub name: String,
    pub target: Option<BinderId>,
    pub namespace: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlphaFacts {
    pub expr: AlphaExpr,
    pub binders: Vec<AlphaBinder>,
    pub diagnostics: Vec<AlphaDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlphaDiagnostic {
    UndefinedVar { name: String },
}

#[derive(Default)]
struct AlphaCtx {
    next_id: usize,
    scopes: Vec<BTreeMap<String, BinderId>>,
    binders: Vec<AlphaBinder>,
    diagnostics: Vec<AlphaDiagnostic>,
}

pub fn alpha_expr(expr: &Expr) -> AlphaFacts {
    let mut ctx = AlphaCtx {
        scopes: vec![BTreeMap::new()],
        ..AlphaCtx::default()
    };
    let expr = ctx.alpha(expr);
    AlphaFacts {
        expr,
        binders: ctx.binders,
        diagnostics: ctx.diagnostics,
    }
}

impl AlphaCtx {
    fn alpha(&mut self, expr: &Expr) -> AlphaExpr {
        match expr {
            Expr::Var(name) => {
                let target = self.lookup(name);
                if target.is_none() {
                    self.diagnostics.push(AlphaDiagnostic::UndefinedVar {
                        name: name.clone(),
                    });
                }
                AlphaExpr {
                    kind: AlphaExprKind::Var(ResolvedVar {
                        name: name.clone(),
                        target,
                        namespace: Vec::new(),
                    }),
                }
            }
            Expr::Lit(lit) => AlphaExpr {
                kind: AlphaExprKind::Lit(lit.clone()),
            },
            Expr::Lambda { param, body } => {
                self.push_scope();
                let param = self.bind(param);
                let body = Box::new(self.alpha(body));
                self.pop_scope();
                AlphaExpr {
                    kind: AlphaExprKind::Lambda { param, body },
                }
            }
            Expr::Call { callee, arg } => AlphaExpr {
                kind: AlphaExprKind::Call {
                    callee: Box::new(self.alpha(callee)),
                    arg: Box::new(self.alpha(arg)),
                },
            },
            Expr::Reset { multi, body } => AlphaExpr {
                kind: AlphaExprKind::Reset {
                    multi: *multi,
                    body: Box::new(self.alpha(body)),
                },
            },
            Expr::Shift { binder, body } => {
                self.push_scope();
                let binder = self.bind(binder);
                let body = Box::new(self.alpha(body));
                self.pop_scope();
                AlphaExpr {
                    kind: AlphaExprKind::Shift { binder, body },
                }
            }
        }
    }

    fn bind(&mut self, name: &str) -> AlphaBinder {
        let binder = AlphaBinder {
            id: BinderId(self.next_id),
            name: name.to_string(),
            namespace: Vec::new(),
        };
        self.next_id += 1;
        self.scopes
            .last_mut()
            .expect("alpha has at least one scope")
            .insert(name.to_string(), binder.id);
        self.binders.push(binder.clone());
        binder
    }

    fn lookup(&self, name: &str) -> Option<BinderId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}
