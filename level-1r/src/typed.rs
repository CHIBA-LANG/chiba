use crate::ast::{Expr, Literal};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
    pub usage: UsageColor,
    pub send: SendColor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedExprKind {
    Var(String),
    Lit(Literal),
    Lambda {
        param: String,
        param_ty: Type,
        body: Box<TypedExpr>,
    },
    Call {
        callee: Box<TypedExpr>,
        arg: Box<TypedExpr>,
    },
    Reset {
        multi: bool,
        body: Box<TypedExpr>,
    },
    Shift {
        binder: String,
        body: Box<TypedExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Unknown,
    I64,
    Bool,
    Func(Box<Type>, Box<Type>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UsageColor {
    One,
    Many,
    Obligation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SendColor {
    Send,
    NotSend,
    Obligation,
}

pub fn type_expr(expr: &Expr) -> TypedExpr {
    match expr {
        Expr::Var(name) => typed(TypedExprKind::Var(name.clone()), Type::Unknown),
        Expr::Lit(Literal::I64(value)) => {
            typed(TypedExprKind::Lit(Literal::I64(*value)), Type::I64)
        }
        Expr::Lit(Literal::Bool(value)) => {
            typed(TypedExprKind::Lit(Literal::Bool(*value)), Type::Bool)
        }
        Expr::Lambda { param, body } => {
            let param_ty = Type::Unknown;
            let body = type_expr(body);
            let ty = Type::Func(Box::new(param_ty.clone()), Box::new(body.ty.clone()));
            typed(
                TypedExprKind::Lambda {
                    param: param.clone(),
                    param_ty,
                    body: Box::new(body),
                },
                ty,
            )
        }
        Expr::Call { callee, arg } => {
            let callee = type_expr(callee);
            let arg = type_expr(arg);
            typed(
                TypedExprKind::Call {
                    callee: Box::new(callee),
                    arg: Box::new(arg),
                },
                Type::Unknown,
            )
        }
        Expr::Reset { multi, body } => {
            let body = type_expr(body);
            typed(
                TypedExprKind::Reset {
                    multi: *multi,
                    body: Box::new(body.clone()),
                },
                body.ty,
            )
        }
        Expr::Shift { binder, body } => {
            let body = type_expr(body);
            typed(
                TypedExprKind::Shift {
                    binder: binder.clone(),
                    body: Box::new(body.clone()),
                },
                body.ty,
            )
        }
    }
}

fn typed(kind: TypedExprKind, ty: Type) -> TypedExpr {
    TypedExpr {
        kind,
        ty,
        usage: UsageColor::Obligation,
        send: SendColor::Obligation,
    }
}
