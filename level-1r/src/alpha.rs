use std::collections::BTreeMap;
use std::fmt;

use crate::ast::{BinaryOp, Expr, Literal, Pattern};

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
    Field {
        receiver: Box<AlphaExpr>,
        name: String,
    },
    MethodCall {
        receiver: Box<AlphaExpr>,
        name: String,
        arg: Box<AlphaExpr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<AlphaExpr>,
        rhs: Box<AlphaExpr>,
    },
    If {
        cond: Box<AlphaExpr>,
        then_branch: Box<AlphaExpr>,
        else_branch: Box<AlphaExpr>,
    },
    IfLet {
        pattern: AlphaPattern,
        scrutinee: Box<AlphaExpr>,
        then_branch: Box<AlphaExpr>,
        else_branch: Box<AlphaExpr>,
    },
    Match {
        scrutinee: Box<AlphaExpr>,
        arms: Vec<AlphaMatchArm>,
    },
    Nominal {
        name: String,
        expr: Box<AlphaExpr>,
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
pub struct AlphaMatchArm {
    pub pattern: AlphaPattern,
    pub body: AlphaExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlphaPattern {
    Wildcard,
    Bind(AlphaBinder),
    Tuple(Vec<AlphaPattern>),
    Lit(Literal),
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
            Expr::Field { receiver, name } => AlphaExpr {
                kind: AlphaExprKind::Field {
                    receiver: Box::new(self.alpha(receiver)),
                    name: name.clone(),
                },
            },
            Expr::MethodCall {
                receiver,
                name,
                arg,
            } => AlphaExpr {
                kind: AlphaExprKind::MethodCall {
                    receiver: Box::new(self.alpha(receiver)),
                    name: name.clone(),
                    arg: Box::new(self.alpha(arg)),
                },
            },
            Expr::Binary { op, lhs, rhs } => AlphaExpr {
                kind: AlphaExprKind::Binary {
                    op: op.clone(),
                    lhs: Box::new(self.alpha(lhs)),
                    rhs: Box::new(self.alpha(rhs)),
                },
            },
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => AlphaExpr {
                kind: AlphaExprKind::If {
                    cond: Box::new(self.alpha(cond)),
                    then_branch: Box::new(self.alpha(then_branch)),
                    else_branch: Box::new(self.alpha(else_branch)),
                },
            },
            Expr::IfLet {
                pattern,
                scrutinee,
                then_branch,
                else_branch,
            } => {
                let scrutinee = Box::new(self.alpha(scrutinee));
                self.push_scope();
                let pattern = self.alpha_pattern(pattern);
                let then_branch = Box::new(self.alpha(then_branch));
                self.pop_scope();
                AlphaExpr {
                    kind: AlphaExprKind::IfLet {
                        pattern,
                        scrutinee,
                        then_branch,
                        else_branch: Box::new(self.alpha(else_branch)),
                    },
                }
            }
            Expr::Match { scrutinee, arms } => AlphaExpr {
                kind: AlphaExprKind::Match {
                    scrutinee: Box::new(self.alpha(scrutinee)),
                    arms: arms
                        .iter()
                        .map(|arm| {
                            self.push_scope();
                            let pattern = self.alpha_pattern(&arm.pattern);
                            let body = self.alpha(&arm.body);
                            self.pop_scope();
                            AlphaMatchArm { pattern, body }
                        })
                        .collect(),
                },
            },
            Expr::Nominal { name, expr } => AlphaExpr {
                kind: AlphaExprKind::Nominal {
                    name: name.clone(),
                    expr: Box::new(self.alpha(expr)),
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

    fn alpha_pattern(&mut self, pattern: &Pattern) -> AlphaPattern {
        match pattern {
            Pattern::Wildcard => AlphaPattern::Wildcard,
            Pattern::Bind(name) => AlphaPattern::Bind(self.bind(name)),
            Pattern::Tuple(fields) => AlphaPattern::Tuple(
                fields
                    .iter()
                    .map(|field| self.alpha_pattern(field))
                    .collect(),
            ),
            Pattern::Lit(lit) => AlphaPattern::Lit(lit.clone()),
        }
    }
}
