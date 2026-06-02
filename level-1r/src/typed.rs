use crate::ast::{BinaryOp, Expr, Literal, Pattern};

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
    Field {
        receiver: Box<TypedExpr>,
        name: String,
    },
    MethodCall {
        receiver: Box<TypedExpr>,
        name: String,
        arg: Box<TypedExpr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<TypedExpr>,
        rhs: Box<TypedExpr>,
    },
    If {
        cond: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Box<TypedExpr>,
    },
    Match {
        scrutinee: Box<TypedExpr>,
        arms: Vec<TypedMatchArm>,
    },
    Nominal {
        name: String,
        expr: Box<TypedExpr>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedMatchArm {
    pub pattern: Pattern,
    pub body: TypedExpr,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Unknown,
    I64,
    Bool,
    Nominal(String),
    Func(Box<Type>, Box<Type>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UsageColor {
    One,
    Many,
    Obligation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        Expr::Field { receiver, name } => {
            let receiver = type_expr(receiver);
            typed(
                TypedExprKind::Field {
                    receiver: Box::new(receiver),
                    name: name.clone(),
                },
                Type::Unknown,
            )
        }
        Expr::MethodCall {
            receiver,
            name,
            arg,
        } => {
            let receiver = type_expr(receiver);
            let arg = type_expr(arg);
            typed(
                TypedExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    name: name.clone(),
                    arg: Box::new(arg),
                },
                Type::Unknown,
            )
        }
        Expr::Binary { op, lhs, rhs } => {
            let lhs = type_expr(lhs);
            let rhs = type_expr(rhs);
            typed(
                TypedExprKind::Binary {
                    op: op.clone(),
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                Type::Unknown,
            )
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond = type_expr(cond);
            let then_branch = type_expr(then_branch);
            let else_branch = type_expr(else_branch);
            let ty = common_type(&then_branch.ty, &else_branch.ty);
            typed(
                TypedExprKind::If {
                    cond: Box::new(cond),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                },
                ty,
            )
        }
        Expr::Match { scrutinee, arms } => {
            let scrutinee = type_expr(scrutinee);
            let arms: Vec<_> = arms
                .iter()
                .map(|arm| TypedMatchArm {
                    pattern: arm.pattern.clone(),
                    body: type_expr(&arm.body),
                })
                .collect();
            let ty = arms
                .iter()
                .map(|arm| arm.body.ty.clone())
                .reduce(|left, right| common_type(&left, &right))
                .unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::Match {
                    scrutinee: Box::new(scrutinee),
                    arms,
                },
                ty,
            )
        }
        Expr::Nominal { name, expr } => {
            let expr = type_expr(expr);
            typed(
                TypedExprKind::Nominal {
                    name: name.clone(),
                    expr: Box::new(expr),
                },
                Type::Nominal(name.clone()),
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

fn common_type(left: &Type, right: &Type) -> Type {
    if left == right {
        left.clone()
    } else {
        Type::Unknown
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
