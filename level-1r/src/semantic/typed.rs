use std::collections::BTreeMap;

use crate::ast::{BinaryOp, Expr, Literal, Pattern};
use crate::symbol::encode_debug_symbol;

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
        args: Vec<TypedExpr>,
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
    AdtCtor {
        data: String,
        ctor: String,
        variants: Vec<String>,
        args: Vec<TypedExpr>,
    },
    Field {
        receiver: Box<TypedExpr>,
        name: String,
    },
    MethodCall {
        receiver: Box<TypedExpr>,
        name: String,
        args: Vec<TypedExpr>,
    },
    Index {
        receiver: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    Range {
        start: Box<TypedExpr>,
        end: Box<TypedExpr>,
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
    Adt {
        name: String,
        variants: Vec<String>,
    },
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

pub type TypeEnv = BTreeMap<String, Type>;

pub fn type_expr(expr: &Expr) -> TypedExpr {
    type_expr_with_env(expr, &TypeEnv::new())
}

pub fn type_expr_with_env(expr: &Expr, env: &TypeEnv) -> TypedExpr {
    match expr {
        Expr::Var(name) => typed(
            TypedExprKind::Var(name.clone()),
            env.get(name).cloned().unwrap_or(Type::Unknown),
        ),
        Expr::Lit(Literal::I64(value)) => {
            typed(TypedExprKind::Lit(Literal::I64(*value)), Type::I64)
        }
        Expr::Lit(Literal::Bool(value)) => {
            typed(TypedExprKind::Lit(Literal::Bool(*value)), Type::Bool)
        }
        Expr::Lambda { param, body } => {
            let param_ty = Type::Unknown;
            let mut env = env.clone();
            env.insert(param.clone(), param_ty.clone());
            let body = type_expr_with_env(body, &env);
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
        Expr::Call { callee, args } => {
            let callee = type_expr_with_env(callee, env);
            let args = args
                .iter()
                .map(|arg| type_expr_with_env(arg, env))
                .collect();
            typed(
                TypedExprKind::Call {
                    callee: Box::new(callee),
                    args,
                },
                Type::Unknown,
            )
        }
        Expr::Instantiate { callee, .. } => type_expr_with_env(callee, env),
        Expr::Tuple(fields) => {
            let fields: Vec<_> = fields
                .iter()
                .map(|field| type_expr_with_env(field, env))
                .collect();
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
                    value: type_expr_with_env(&field.value, env),
                })
                .collect::<Vec<_>>();
            let ty = Type::Record(record_type_fields(&fields));
            typed(TypedExprKind::Record { fields }, ty)
        }
        Expr::RecordUpdate { base, fields } => {
            let base = type_expr_with_env(base, env);
            let fields = fields
                .iter()
                .map(|field| TypedRecordField {
                    name: field.name.clone(),
                    value: type_expr_with_env(&field.value, env),
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
        Expr::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            let args = args
                .iter()
                .map(|arg| type_expr_with_env(arg, env))
                .collect::<Vec<_>>();
            typed(
                TypedExprKind::AdtCtor {
                    data: data.clone(),
                    ctor: ctor.clone(),
                    variants: canonical_variants(variants),
                    args,
                },
                Type::Adt {
                    name: data.clone(),
                    variants: canonical_variants(variants),
                },
            )
        }
        Expr::Field { receiver, name } => {
            let receiver = type_expr_with_env(receiver, env);
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
            args,
        } => {
            let receiver = type_expr_with_env(receiver, env);
            let args = args
                .iter()
                .map(|arg| type_expr_with_env(arg, env))
                .collect();
            typed(
                TypedExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    name: name.clone(),
                    args,
                },
                Type::Unknown,
            )
        }
        Expr::Index { receiver, index } => {
            let receiver = type_expr_with_env(receiver, env);
            let index = type_expr_with_env(index, env);
            typed(
                TypedExprKind::Index {
                    receiver: Box::new(receiver),
                    index: Box::new(index),
                },
                Type::Unknown,
            )
        }
        Expr::Range { start, end } => {
            let start = type_expr_with_env(start, env);
            let end = type_expr_with_env(end, env);
            typed(
                TypedExprKind::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                },
                Type::Nominal("Range".to_string()),
            )
        }
        Expr::Binary { op, lhs, rhs } => {
            let lhs = type_expr_with_env(lhs, env);
            let rhs = type_expr_with_env(rhs, env);
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
            let cond = type_expr_with_env(cond, env);
            let then_branch = type_expr_with_env(then_branch, env);
            let else_branch = type_expr_with_env(else_branch, env);
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
            let scrutinee = type_expr_with_env(scrutinee, env);
            let then_branch = type_expr_with_env(then_branch, env);
            let else_branch = type_expr_with_env(else_branch, env);
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
            let scrutinee = type_expr_with_env(scrutinee, env);
            let arms: Vec<_> = arms
                .iter()
                .map(|arm| TypedMatchArm {
                    pattern: arm.pattern.clone(),
                    body: type_expr_with_env(&arm.body, env),
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
            let expr = type_expr_with_env(expr, env);
            typed(
                TypedExprKind::Nominal {
                    name: name.clone(),
                    expr: Box::new(expr),
                },
                Type::Nominal(name.clone()),
            )
        }
        Expr::Reset { multi, body } => {
            let body = type_expr_with_env(body, env);
            typed(
                TypedExprKind::Reset {
                    multi: *multi,
                    body: Box::new(body.clone()),
                },
                body.ty,
            )
        }
        Expr::Shift { binder, body } => {
            let body = type_expr_with_env(body, env);
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
    if matches!(base, Type::Nominal(_)) {
        return base.clone();
    }
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
        Type::Adt { name, variants } => {
            let mut stable = format!("Adt{}", sanitize_type_name(name));
            for variant in variants {
                stable.push('_');
                stable.push_str(&sanitize_type_name(variant));
            }
            stable
        }
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
    encode_debug_symbol(name)
}

fn canonical_variants(variants: &[String]) -> Vec<String> {
    let mut variants = variants.to_vec();
    variants.sort();
    variants.dedup();
    variants
}
