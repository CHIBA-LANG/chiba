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
    Continuation {
        multi: bool,
        input: Box<Type>,
        answer: Box<Type>,
    },
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypeContext {
    nominal_rows: BTreeMap<String, Vec<RecordTypeField>>,
    generic_nominal_rows: BTreeMap<String, NominalRowDecl>,
    data_generics: BTreeMap<String, Vec<String>>,
    constructors: BTreeMap<(String, String), Vec<Type>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NominalRowDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<RecordTypeField>,
}

impl TypeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_nominal_row(&mut self, nominal: impl Into<String>, fields: Vec<RecordTypeField>) {
        self.nominal_rows
            .insert(nominal.into(), canonical_fields(fields));
    }

    pub fn insert_generic_nominal_row(
        &mut self,
        name: impl Into<String>,
        generics: Vec<String>,
        fields: Vec<RecordTypeField>,
    ) {
        let name = name.into();
        self.generic_nominal_rows.insert(
            name.clone(),
            NominalRowDecl {
                name,
                generics,
                fields: canonical_fields(fields),
            },
        );
    }

    pub fn insert_data_constructor(
        &mut self,
        data: impl Into<String>,
        generics: Vec<String>,
        ctor: impl Into<String>,
        payloads: Vec<Type>,
    ) {
        let data = data.into();
        self.data_generics.entry(data.clone()).or_insert(generics);
        self.constructors.insert((data, ctor.into()), payloads);
    }

    pub fn nominal_field_type(&self, receiver: &Type, name: &str) -> Option<Type> {
        let Type::Nominal(nominal) = receiver else {
            return None;
        };
        self.nominal_rows
            .get(nominal)
            .and_then(|fields| field_type(fields, name))
            .or_else(|| self.generic_nominal_field_type(nominal, name))
    }

    fn generic_nominal_field_type(&self, nominal: &str, field: &str) -> Option<Type> {
        let (base, args) = parse_nominal_application(nominal)?;
        let decl = self.generic_nominal_rows.get(base)?;
        if decl.generics.len() != args.len() {
            return None;
        }
        let substitutions = decl
            .generics
            .iter()
            .cloned()
            .zip(args.iter().map(|arg| source_type_name_to_type(arg)))
            .collect::<BTreeMap<_, _>>();
        field_type(&decl.fields, field).map(|ty| substitute_type_params(&ty, &substitutions))
    }

    pub fn pattern_bindings_for(&self, pattern: &Pattern, subject: &Type) -> Vec<(String, Type)> {
        let subject_data =
            nominal_type_name(subject).map(|name| nominal_base_name(name).to_string());
        let substitutions = nominal_type_name(subject)
            .and_then(|nominal| self.substitutions_for_subject(nominal))
            .unwrap_or_default();
        self.pattern_bindings(
            pattern,
            subject.clone(),
            subject_data.as_deref(),
            &substitutions,
        )
    }

    pub fn adt_variants_for_type(&self, ty: &Type) -> Option<(String, Vec<String>)> {
        match ty {
            Type::Adt { name, variants } => Some((name.clone(), variants.clone())),
            Type::Nominal(name) => {
                let base = nominal_base_name(name).to_string();
                let mut variants = self
                    .constructors
                    .keys()
                    .filter(|(data, _)| data == &base)
                    .map(|(_, ctor)| ctor.clone())
                    .collect::<Vec<_>>();
                if variants.is_empty() {
                    None
                } else {
                    variants.sort();
                    variants.dedup();
                    Some((base, variants))
                }
            }
            _ => None,
        }
    }

    fn pattern_bindings(
        &self,
        pattern: &Pattern,
        ty: Type,
        subject_data: Option<&str>,
        substitutions: &BTreeMap<String, Type>,
    ) -> Vec<(String, Type)> {
        match pattern {
            Pattern::Bind(name) => vec![(name.clone(), ty)],
            Pattern::Tuple(fields) => fields
                .iter()
                .enumerate()
                .flat_map(|(index, field)| {
                    let field_ty = self.pattern_tuple_field_type(&ty, index);
                    self.pattern_bindings(field, field_ty, subject_data, substitutions)
                })
                .collect(),
            Pattern::Record(fields) => fields
                .iter()
                .flat_map(|field| {
                    let field_ty = self.pattern_record_field_type(&ty, &field.name);
                    self.pattern_bindings(&field.pattern, field_ty, subject_data, substitutions)
                })
                .collect(),
            Pattern::Constructor { data, ctor, args } => {
                let ctor_data = data.as_deref().or(subject_data);
                let payloads = ctor_data
                    .and_then(|data| self.constructors.get(&(data.to_string(), ctor.clone())))
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                args.iter()
                    .enumerate()
                    .flat_map(|(index, arg)| {
                        let field_ty = payloads
                            .get(index)
                            .map(|field| substitute_type_params(field, substitutions))
                            .unwrap_or(Type::Unknown);
                        self.pattern_bindings(arg, field_ty, subject_data, substitutions)
                    })
                    .collect()
            }
            Pattern::At { name, pattern } => {
                let mut bindings =
                    self.pattern_bindings(pattern, ty.clone(), subject_data, substitutions);
                bindings.push((name.clone(), ty));
                bindings
            }
            Pattern::Wildcard | Pattern::Lit(_) => Vec::new(),
        }
    }

    fn pattern_record_field_type(&self, ty: &Type, field: &str) -> Type {
        match ty {
            Type::Record(fields) => field_type(fields, field).unwrap_or(Type::Unknown),
            _ => self.nominal_field_type(ty, field).unwrap_or(Type::Unknown),
        }
    }

    fn pattern_tuple_field_type(&self, ty: &Type, index: usize) -> Type {
        match ty {
            Type::Tuple(fields) => fields.get(index).cloned().unwrap_or(Type::Unknown),
            Type::Nominal(name) => tuple_intrinsic_field_type(name, index).unwrap_or(Type::Unknown),
            _ => Type::Unknown,
        }
    }

    fn substitutions_for_subject(&self, subject: &str) -> Option<BTreeMap<String, Type>> {
        let (base, args) = parse_nominal_application(subject)?;
        let generics = self.data_generics.get(base)?;
        if generics.len() != args.len() {
            return None;
        }
        Some(
            generics
                .iter()
                .cloned()
                .zip(args.iter().map(|arg| source_type_name_to_type(arg)))
                .collect(),
        )
    }
}

pub fn type_expr(expr: &Expr) -> TypedExpr {
    type_expr_with_env(expr, &TypeEnv::new())
}

pub fn type_expr_with_env(expr: &Expr, env: &TypeEnv) -> TypedExpr {
    type_expr_with_context(expr, env, &TypeContext::new())
}

pub fn type_expr_with_context(expr: &Expr, env: &TypeEnv, context: &TypeContext) -> TypedExpr {
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
            let body = type_expr_with_context(body, &env, context);
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
            let callee = type_expr_with_context(callee, env, context);
            let args = args
                .iter()
                .map(|arg| type_expr_with_context(arg, env, context))
                .collect::<Vec<_>>();
            let ty = call_result_type(&callee.ty, args.len());
            typed(
                TypedExprKind::Call {
                    callee: Box::new(callee),
                    args,
                },
                ty,
            )
        }
        Expr::Instantiate { callee, .. } => type_expr_with_context(callee, env, context),
        Expr::Tuple(fields) => {
            let fields: Vec<_> = fields
                .iter()
                .map(|field| type_expr_with_context(field, env, context))
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
                    value: type_expr_with_context(&field.value, env, context),
                })
                .collect::<Vec<_>>();
            let ty = Type::Record(record_type_fields(&fields));
            typed(TypedExprKind::Record { fields }, ty)
        }
        Expr::RecordUpdate { base, fields } => {
            let base = type_expr_with_context(base, env, context);
            let fields = fields
                .iter()
                .map(|field| TypedRecordField {
                    name: field.name.clone(),
                    value: type_expr_with_context(&field.value, env, context),
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
                .map(|arg| type_expr_with_context(arg, env, context))
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
            let receiver = type_expr_with_context(receiver, env, context);
            let ty = tuple_field_type(&receiver.ty, name)
                .or_else(|| record_field_type(&receiver.ty, name))
                .or_else(|| context.nominal_field_type(&receiver.ty, name))
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
            let receiver = type_expr_with_context(receiver, env, context);
            let args = args
                .iter()
                .map(|arg| type_expr_with_context(arg, env, context))
                .collect::<Vec<_>>();
            let ty = field_callable_result_type(&receiver.ty, name, args.len(), context);
            typed(
                TypedExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    name: name.clone(),
                    args,
                },
                ty,
            )
        }
        Expr::Index { receiver, index } => {
            let receiver = type_expr_with_context(receiver, env, context);
            let index = type_expr_with_context(index, env, context);
            typed(
                TypedExprKind::Index {
                    receiver: Box::new(receiver),
                    index: Box::new(index),
                },
                Type::Unknown,
            )
        }
        Expr::Range { start, end } => {
            let start = type_expr_with_context(start, env, context);
            let end = type_expr_with_context(end, env, context);
            typed(
                TypedExprKind::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                },
                Type::Nominal("Range".to_string()),
            )
        }
        Expr::Binary { op, lhs, rhs } => {
            let lhs = type_expr_with_context(lhs, env, context);
            let rhs = type_expr_with_context(rhs, env, context);
            let ty = binary_result_type(&lhs.ty, &rhs.ty);
            typed(
                TypedExprKind::Binary {
                    op: op.clone(),
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                ty,
            )
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond = type_expr_with_context(cond, env, context);
            let then_branch = type_expr_with_context(then_branch, env, context);
            let else_branch = type_expr_with_context(else_branch, env, context);
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
            let scrutinee = type_expr_with_context(scrutinee, env, context);
            let mut then_env = env.clone();
            then_env.extend(context.pattern_bindings_for(pattern, &scrutinee.ty));
            let then_branch = type_expr_with_context(then_branch, &then_env, context);
            let else_branch = type_expr_with_context(else_branch, env, context);
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
            let scrutinee = type_expr_with_context(scrutinee, env, context);
            let arms: Vec<_> = arms
                .iter()
                .map(|arm| {
                    let mut arm_env = env.clone();
                    arm_env.extend(context.pattern_bindings_for(&arm.pattern, &scrutinee.ty));
                    TypedMatchArm {
                        pattern: arm.pattern.clone(),
                        body: type_expr_with_context(&arm.body, &arm_env, context),
                    }
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
            let expr = type_expr_with_context(expr, env, context);
            typed(
                TypedExprKind::Nominal {
                    name: name.clone(),
                    expr: Box::new(expr),
                },
                Type::Nominal(name.clone()),
            )
        }
        Expr::Reset { multi, body } => {
            let body = type_expr_with_context(body, env, context);
            typed(
                TypedExprKind::Reset {
                    multi: *multi,
                    body: Box::new(body.clone()),
                },
                body.ty,
            )
        }
        Expr::Shift { binder, body } => {
            let body = type_expr_with_context(body, env, context);
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

fn binary_result_type(lhs: &Type, rhs: &Type) -> Type {
    match (lhs, rhs) {
        (Type::I64, Type::I64) => Type::I64,
        _ => Type::Unknown,
    }
}

fn call_result_type(callee: &Type, arity: usize) -> Type {
    let mut current = callee;
    for _ in 0..arity {
        match current {
            Type::Func(_, result) => current = result,
            _ => return Type::Unknown,
        }
    }
    current.clone()
}

fn field_callable_result_type(
    receiver: &Type,
    name: &str,
    arity: usize,
    context: &TypeContext,
) -> Type {
    let Some(field_ty) =
        record_field_type(receiver, name).or_else(|| context.nominal_field_type(receiver, name))
    else {
        return Type::Unknown;
    };
    call_result_type(&field_ty, arity)
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

fn tuple_intrinsic_field_type(nominal: &str, index: usize) -> Option<Type> {
    let (base, args) = parse_nominal_application(nominal)?;
    if base != "Tuple" {
        return None;
    }
    args.get(index).map(|arg| source_type_name_to_type(arg))
}

fn record_field_type(receiver: &Type, name: &str) -> Option<Type> {
    let Type::Record(fields) = receiver else {
        return None;
    };
    field_type(fields, name)
}

fn field_type(fields: &[RecordTypeField], name: &str) -> Option<Type> {
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.ty.clone())
}

fn record_type_fields(fields: &[TypedRecordField]) -> Vec<RecordTypeField> {
    let fields = fields
        .iter()
        .map(|field| RecordTypeField {
            name: field.name.clone(),
            ty: field.value.ty.clone(),
        })
        .collect::<Vec<_>>();
    canonical_fields(fields)
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
        if let Some(existing) = merged
            .iter_mut()
            .find(|existing| existing.name == field.name)
        {
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

fn canonical_fields(mut fields: Vec<RecordTypeField>) -> Vec<RecordTypeField> {
    fields.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| type_stable_name(&left.ty).cmp(&type_stable_name(&right.ty)))
    });
    fields.dedup_by(|left, right| left.name == right.name);
    fields
}

fn parse_nominal_application(nominal: &str) -> Option<(&str, Vec<String>)> {
    let open = nominal.find('[')?;
    let close = nominal.strip_suffix(']')?;
    let base = &nominal[..open];
    let args = &close[open + 1..];
    let args = if args.trim().is_empty() {
        Vec::new()
    } else {
        split_nominal_type_args(args)?
    };
    Some((base, args))
}

fn split_nominal_type_args(args: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in args.char_indices() {
        match ch {
            '[' => depth = depth.checked_add(1)?,
            ']' => depth = depth.checked_sub(1)?,
            ',' if depth == 0 => {
                let arg = args[start..index].trim();
                if arg.is_empty() {
                    return None;
                }
                out.push(arg.to_string());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    let arg = args[start..].trim();
    if arg.is_empty() {
        return None;
    }
    out.push(arg.to_string());
    Some(out)
}

fn nominal_type_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Nominal(name) => Some(name),
        _ => None,
    }
}

fn nominal_base_name(name: &str) -> &str {
    name.find('[').map(|open| &name[..open]).unwrap_or(name)
}

pub(crate) fn source_type_name_to_type(name: &str) -> Type {
    match name {
        "I64" | "i64" => Type::I64,
        "Bool" | "bool" => Type::Bool,
        _ => callable_type(name)
            .or_else(|| continuation_storage_type(name))
            .or_else(|| tuple_intrinsic_type(name))
            .unwrap_or_else(|| Type::Nominal(name.to_string())),
    }
}

fn callable_type(name: &str) -> Option<Type> {
    let arrow = top_level_arrow(name)?;
    let param = name[..arrow].trim();
    let result = name[arrow + 2..].trim();
    let param = parenthesized_type(param)?;
    if result.is_empty() {
        return None;
    }
    Some(Type::Func(
        Box::new(source_type_name_to_type(param)),
        Box::new(source_type_name_to_type(result)),
    ))
}

fn continuation_storage_type(name: &str) -> Option<Type> {
    let (base, args) = parse_nominal_application(name)?;
    let multi = match base {
        "Cont1" => false,
        "ContN" => true,
        _ => return None,
    };
    if args.len() != 2 {
        return None;
    }
    Some(Type::Continuation {
        multi,
        input: Box::new(source_type_name_to_type(&args[0])),
        answer: Box::new(source_type_name_to_type(&args[1])),
    })
}

fn top_level_arrow(name: &str) -> Option<usize> {
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut chars = name.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '[' => square_depth = square_depth.checked_add(1)?,
            ']' => square_depth = square_depth.checked_sub(1)?,
            '(' => paren_depth = paren_depth.checked_add(1)?,
            ')' => paren_depth = paren_depth.checked_sub(1)?,
            '-' if square_depth == 0
                && paren_depth == 0
                && chars.peek().is_some_and(|(_, next)| *next == '>') =>
            {
                return Some(index)
            }
            _ => {}
        }
    }
    None
}

fn parenthesized_type(name: &str) -> Option<&str> {
    let inner = name.strip_prefix('(')?.strip_suffix(')')?.trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner)
    }
}

fn tuple_intrinsic_type(name: &str) -> Option<Type> {
    let (base, args) = parse_nominal_application(name)?;
    if base != "Tuple" {
        return None;
    }
    Some(Type::Tuple(
        args.iter()
            .map(|arg| source_type_name_to_type(arg))
            .collect(),
    ))
}

fn substitute_type_params(ty: &Type, substitutions: &BTreeMap<String, Type>) -> Type {
    match ty {
        Type::Nominal(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| Type::Nominal(name.clone())),
        Type::Tuple(fields) => Type::Tuple(
            fields
                .iter()
                .map(|field| substitute_type_params(field, substitutions))
                .collect(),
        ),
        Type::Record(fields) => Type::Record(
            fields
                .iter()
                .map(|field| RecordTypeField {
                    name: field.name.clone(),
                    ty: substitute_type_params(&field.ty, substitutions),
                })
                .collect(),
        ),
        Type::Func(param, result) => Type::Func(
            Box::new(substitute_type_params(param, substitutions)),
            Box::new(substitute_type_params(result, substitutions)),
        ),
        Type::Continuation {
            multi,
            input,
            answer,
        } => Type::Continuation {
            multi: *multi,
            input: Box::new(substitute_type_params(input, substitutions)),
            answer: Box::new(substitute_type_params(answer, substitutions)),
        },
        Type::Adt { name, variants } => Type::Adt {
            name: name.clone(),
            variants: variants.clone(),
        },
        Type::Unknown => Type::Unknown,
        Type::I64 => Type::I64,
        Type::Bool => Type::Bool,
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
        Type::Continuation {
            multi,
            input,
            answer,
        } => {
            let kind = if *multi { "ContN" } else { "Cont1" };
            format!(
                "{kind}_{}_{}",
                type_stable_name(input),
                type_stable_name(answer)
            )
        }
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
