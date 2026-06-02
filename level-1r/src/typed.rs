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
    Tuple {
        fields: Vec<TypedExpr>,
        nominal: String,
    },
    Record {
        fields: Vec<TypedRecordField>,
    },
    RecordUpdate {
        base: Box<TypedExpr>,
        fields: Vec<TypedRecordField>,
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
    IfLet {
        pattern: Pattern,
        scrutinee: Box<TypedExpr>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedRecordField {
    pub name: String,
    pub value: TypedExpr,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Unknown,
    I64,
    Bool,
    Tuple(Vec<Type>),
    Record(Vec<RecordTypeField>),
    Nominal(String),
    Func(Box<Type>, Box<Type>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordTypeField {
    pub name: String,
    pub ty: Type,
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
        Expr::Tuple(fields) => {
            let fields: Vec<_> = fields.iter().map(type_expr).collect();
            let field_types = fields
                .iter()
                .map(|field| field.ty.clone())
                .collect::<Vec<_>>();
            typed(
                TypedExprKind::Tuple {
                    nominal: tuple_nominal_name(&field_types),
                    fields,
                },
                Type::Tuple(field_types),
            )
        }
        Expr::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| TypedRecordField {
                    name: field.name.clone(),
                    value: type_expr(&field.value),
                })
                .collect::<Vec<_>>();
            let ty = Type::Record(record_type_fields(&fields));
            typed(TypedExprKind::Record { fields }, ty)
        }
        Expr::RecordUpdate { base, fields } => {
            let base = type_expr(base);
            let fields = fields
                .iter()
                .map(|field| TypedRecordField {
                    name: field.name.clone(),
                    value: type_expr(&field.value),
                })
                .collect::<Vec<_>>();
            let ty = record_update_type(&base.ty, &fields);
            typed(
                TypedExprKind::RecordUpdate {
                    base: Box::new(base),
                    fields,
                },
                ty,
            )
        }
        Expr::Field { receiver, name } => {
            let receiver = type_expr(receiver);
            let ty = tuple_field_type(&receiver.ty, name)
                .or_else(|| record_field_type(&receiver.ty, name))
                .unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::Field {
                    receiver: Box::new(receiver),
                    name: name.clone(),
                },
                ty,
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
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            let scrutinee = type_expr(scrutinee);
            let then_branch = type_expr(then_branch);
            let else_branch = type_expr(else_branch);
            let ty = common_type(&then_branch.ty, &else_branch.ty);
            typed(
                TypedExprKind::IfLet {
                    pattern: pattern.clone(),
                    scrutinee: Box::new(scrutinee),
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

fn tuple_field_type(receiver: &Type, name: &str) -> Option<Type> {
    let Type::Tuple(fields) = receiver else {
        return None;
    };
    let index = name.strip_prefix('_')?.parse::<usize>().ok()?;
    if index == 0 {
        return None;
    }
    fields.get(index - 1).cloned()
}

fn record_field_type(receiver: &Type, name: &str) -> Option<Type> {
    let Type::Record(fields) = receiver else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.ty.clone())
}

fn record_type_fields(fields: &[TypedRecordField]) -> Vec<RecordTypeField> {
    let mut fields = fields
        .iter()
        .map(|field| RecordTypeField {
            name: field.name.clone(),
            ty: field.value.ty.clone(),
        })
        .collect::<Vec<_>>();
    fields.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| type_stable_name(&left.ty).cmp(&type_stable_name(&right.ty)))
    });
    fields.dedup_by(|left, right| left.name == right.name);
    fields
}

fn record_update_type(base: &Type, fields: &[TypedRecordField]) -> Type {
    let mut merged = match base {
        Type::Record(fields) => fields.clone(),
        _ => Vec::new(),
    };
    for field in fields {
        if let Some(existing) = merged.iter_mut().find(|existing| existing.name == field.name) {
            existing.ty = field.value.ty.clone();
        } else {
            merged.push(RecordTypeField {
                name: field.name.clone(),
                ty: field.value.ty.clone(),
            });
        }
    }
    merged.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| type_stable_name(&left.ty).cmp(&type_stable_name(&right.ty)))
    });
    Type::Record(merged)
}

fn typed(kind: TypedExprKind, ty: Type) -> TypedExpr {
    TypedExpr {
        kind,
        ty,
        usage: UsageColor::Obligation,
        send: SendColor::Obligation,
    }
}

pub fn tuple_nominal_name(fields: &[Type]) -> String {
    let mut name = format!("Tuple{}", fields.len());
    for field in fields {
        name.push('_');
        name.push_str(&type_stable_name(field));
    }
    name
}

fn type_stable_name(ty: &Type) -> String {
    match ty {
        Type::Unknown => "Unknown".to_string(),
        Type::I64 => "I64".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::Tuple(fields) => tuple_nominal_name(fields),
        Type::Record(fields) => {
            let mut name = "Record".to_string();
            for field in fields {
                name.push('_');
                name.push_str(&sanitize_type_name(&field.name));
                name.push('_');
                name.push_str(&type_stable_name(&field.ty));
            }
            name
        }
        Type::Nominal(name) => format!("Nominal{}", sanitize_type_name(name)),
        Type::Func(arg, ret) => format!("Fn_{}_{}", type_stable_name(arg), type_stable_name(ret)),
    }
}

fn sanitize_type_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
