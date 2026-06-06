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
    SliceLiteral {
        items: Vec<TypedExpr>,
        element: Box<Type>,
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
        access: FieldAccessKind,
    },
    MethodCall {
        receiver: Box<TypedExpr>,
        name: String,
        args: Vec<TypedExpr>,
    },
    Index {
        receiver: Box<TypedExpr>,
        index: Box<TypedExpr>,
        access: IndexAccessKind,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldAccessKind {
    RecordOrNominal,
    TuplePositionalRow { index: usize },
    RangeBoundary { boundary: RangeBoundary },
    SliceBoundary { boundary: SliceBoundary },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexAccessKind {
    Operator,
    SliceElement { element: Type },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeBoundary {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliceBoundary {
    Len,
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
        let mut context = Self::default();
        context.insert_nominal_row(
            "Range",
            vec![
                RecordTypeField {
                    name: "start".to_string(),
                    ty: Type::I64,
                },
                RecordTypeField {
                    name: "end".to_string(),
                    ty: Type::I64,
                },
            ],
        );
        context.insert_generic_nominal_row(
            "Slice",
            vec!["T".to_string()],
            vec![RecordTypeField {
                name: "len".to_string(),
                ty: Type::I64,
            }],
        );
        context
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
        let ParsedTypeHeader::Nominal { base, args } = parse_type_header(nominal)? else {
            return None;
        };
        let decl = self.generic_nominal_rows.get(&base)?;
        if decl.generics.len() != args.len() {
            return None;
        }
        let substitutions = decl
            .generics
            .iter()
            .cloned()
            .zip(args.iter().map(ParsedTypeHeader::to_type))
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

    fn instantiated_data_constructor_type(
        &self,
        data: &str,
        ctor: &str,
        args: &[TypedExpr],
    ) -> Option<Type> {
        let generics = self.data_generics.get(data)?;
        let payloads = self
            .constructors
            .get(&(data.to_string(), ctor.to_string()))?;
        if payloads.len() != args.len() {
            return None;
        }
        let mut substitutions = BTreeMap::new();
        for (payload, arg) in payloads.iter().zip(args) {
            collect_payload_substitutions(payload, &arg.ty, generics, &mut substitutions)?;
        }
        if substitutions.len() != generics.len() {
            return None;
        }
        Some(Type::Nominal(render_data_instance_type(
            data,
            generics,
            &substitutions,
        )))
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
            _ => Type::Unknown,
        }
    }

    fn substitutions_for_subject(&self, subject: &str) -> Option<BTreeMap<String, Type>> {
        let ParsedTypeHeader::Nominal { base, args } = parse_type_header(subject)? else {
            return None;
        };
        let generics = self.data_generics.get(&base)?;
        if generics.len() != args.len() {
            return None;
        }
        Some(
            generics
                .iter()
                .cloned()
                .zip(args.iter().map(ParsedTypeHeader::to_type))
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
    let typed = type_expr_with_context_and_controls(expr, env, context, &mut Vec::new());
    refine_continuation_types(typed, context)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ContinuationTypeBoundary {
    multi: bool,
}

fn type_expr_with_context_and_controls(
    expr: &Expr,
    env: &TypeEnv,
    context: &TypeContext,
    controls: &mut Vec<ContinuationTypeBoundary>,
) -> TypedExpr {
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
            let body = type_expr_with_context_and_controls(body, &env, context, controls);
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
            let callee = type_expr_with_context_and_controls(callee, env, context, controls);
            let args = args
                .iter()
                .map(|arg| type_expr_with_context_and_controls(arg, env, context, controls))
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
        Expr::Instantiate { callee, .. } => {
            type_expr_with_context_and_controls(callee, env, context, controls)
        }
        Expr::Tuple(fields) => {
            let fields: Vec<_> = fields
                .iter()
                .map(|field| type_expr_with_context_and_controls(field, env, context, controls))
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
        Expr::SliceLiteral(items) => {
            let items = items
                .iter()
                .map(|item| type_expr_with_context_and_controls(item, env, context, controls))
                .collect::<Vec<_>>();
            let element = slice_element_type(&items);
            typed(
                TypedExprKind::SliceLiteral {
                    items,
                    element: Box::new(element.clone()),
                },
                Type::Nominal(render_source_type_application(
                    "Slice",
                    &[source_type_name_for_type(&element)],
                )),
            )
        }
        Expr::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| TypedRecordField {
                    name: field.name.clone(),
                    value: type_expr_with_context_and_controls(
                        &field.value,
                        env,
                        context,
                        controls,
                    ),
                })
                .collect::<Vec<_>>();
            let ty = Type::Record(record_type_fields(&fields));
            typed(TypedExprKind::Record { fields }, ty)
        }
        Expr::RecordUpdate { base, fields } => {
            let base = type_expr_with_context_and_controls(base, env, context, controls);
            let fields = fields
                .iter()
                .map(|field| TypedRecordField {
                    name: field.name.clone(),
                    value: type_expr_with_context_and_controls(
                        &field.value,
                        env,
                        context,
                        controls,
                    ),
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
                .map(|arg| type_expr_with_context_and_controls(arg, env, context, controls))
                .collect::<Vec<_>>();
            let ty = context
                .instantiated_data_constructor_type(data, ctor, &args)
                .unwrap_or_else(|| Type::Adt {
                    name: data.clone(),
                    variants: canonical_variants(variants),
                });
            typed(
                TypedExprKind::AdtCtor {
                    data: data.clone(),
                    ctor: ctor.clone(),
                    variants: canonical_variants(variants),
                    args,
                },
                ty,
            )
        }
        Expr::Field { receiver, name } => {
            let receiver = type_expr_with_context_and_controls(receiver, env, context, controls);
            let access = field_access_kind(&receiver.ty, name);
            let ty = match access {
                FieldAccessKind::TuplePositionalRow { index } => {
                    tuple_field_type(&receiver.ty, index)
                }
                FieldAccessKind::RangeBoundary { .. } => Some(Type::I64),
                FieldAccessKind::SliceBoundary { .. } => Some(Type::I64),
                FieldAccessKind::RecordOrNominal => record_field_type(&receiver.ty, name)
                    .or_else(|| context.nominal_field_type(&receiver.ty, name)),
            }
            .unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::Field {
                    receiver: Box::new(receiver),
                    name: name.clone(),
                    access,
                },
                ty,
            )
        }
        Expr::MethodCall {
            receiver,
            name,
            args,
        } => {
            let receiver = type_expr_with_context_and_controls(receiver, env, context, controls);
            let args = args
                .iter()
                .map(|arg| type_expr_with_context_and_controls(arg, env, context, controls))
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
            let receiver = type_expr_with_context_and_controls(receiver, env, context, controls);
            let index = type_expr_with_context_and_controls(index, env, context, controls);
            let access = index_access_kind(&receiver.ty, &index.ty);
            let ty = index_result_type(&access);
            typed(
                TypedExprKind::Index {
                    receiver: Box::new(receiver),
                    index: Box::new(index),
                    access,
                },
                ty,
            )
        }
        Expr::Range { start, end } => {
            let start = type_expr_with_context_and_controls(start, env, context, controls);
            let end = type_expr_with_context_and_controls(end, env, context, controls);
            typed(
                TypedExprKind::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                },
                Type::Nominal("Range".to_string()),
            )
        }
        Expr::Binary { op, lhs, rhs } => {
            let lhs = type_expr_with_context_and_controls(lhs, env, context, controls);
            let rhs = type_expr_with_context_and_controls(rhs, env, context, controls);
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
            let cond = type_expr_with_context_and_controls(cond, env, context, controls);
            let then_branch =
                type_expr_with_context_and_controls(then_branch, env, context, controls);
            let else_branch =
                type_expr_with_context_and_controls(else_branch, env, context, controls);
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
            let scrutinee = type_expr_with_context_and_controls(scrutinee, env, context, controls);
            let mut then_env = env.clone();
            then_env.extend(context.pattern_bindings_for(pattern, &scrutinee.ty));
            let then_branch =
                type_expr_with_context_and_controls(then_branch, &then_env, context, controls);
            let else_branch =
                type_expr_with_context_and_controls(else_branch, env, context, controls);
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
            let scrutinee = type_expr_with_context_and_controls(scrutinee, env, context, controls);
            let arms: Vec<_> = arms
                .iter()
                .map(|arm| {
                    let mut arm_env = env.clone();
                    arm_env.extend(context.pattern_bindings_for(&arm.pattern, &scrutinee.ty));
                    TypedMatchArm {
                        pattern: arm.pattern.clone(),
                        body: type_expr_with_context_and_controls(
                            &arm.body, &arm_env, context, controls,
                        ),
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
            let expr = type_expr_with_context_and_controls(expr, env, context, controls);
            typed(
                TypedExprKind::Nominal {
                    name: name.clone(),
                    expr: Box::new(expr),
                },
                Type::Nominal(name.clone()),
            )
        }
        Expr::Reset { multi, body } => {
            controls.push(ContinuationTypeBoundary { multi: *multi });
            let body = type_expr_with_context_and_controls(body, env, context, controls);
            controls.pop();
            typed(
                TypedExprKind::Reset {
                    multi: *multi,
                    body: Box::new(body.clone()),
                },
                body.ty,
            )
        }
        Expr::Shift { binder, body } => {
            let mut env = env.clone();
            if let Some(boundary) = controls.last() {
                env.insert(
                    binder.clone(),
                    Type::Continuation {
                        multi: boundary.multi,
                        input: Box::new(Type::Unknown),
                        answer: Box::new(Type::Unknown),
                    },
                );
            }
            let body = type_expr_with_context_and_controls(body, &env, context, controls);
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

fn refine_continuation_types(expr: TypedExpr, context: &TypeContext) -> TypedExpr {
    match expr.kind {
        TypedExprKind::Var(name) => typed(TypedExprKind::Var(name), expr.ty),
        TypedExprKind::Lit(lit) => typed(TypedExprKind::Lit(lit), expr.ty),
        TypedExprKind::Lambda {
            param,
            param_ty,
            body,
        } => {
            let body = refine_continuation_types(*body, context);
            let ty = Type::Func(Box::new(param_ty.clone()), Box::new(body.ty.clone()));
            typed(
                TypedExprKind::Lambda {
                    param,
                    param_ty,
                    body: Box::new(body),
                },
                ty,
            )
        }
        TypedExprKind::Call { callee, args } => {
            let callee = refine_continuation_types(*callee, context);
            let args = args
                .into_iter()
                .map(|arg| refine_continuation_types(arg, context))
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
        TypedExprKind::Tuple { fields, nominal: _ } => {
            let fields = fields
                .into_iter()
                .map(|field| refine_continuation_types(field, context))
                .collect::<Vec<_>>();
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
        TypedExprKind::SliceLiteral { items, .. } => {
            let items = items
                .into_iter()
                .map(|item| refine_continuation_types(item, context))
                .collect::<Vec<_>>();
            let element = slice_element_type(&items);
            typed(
                TypedExprKind::SliceLiteral {
                    items,
                    element: Box::new(element.clone()),
                },
                Type::Nominal(render_source_type_application(
                    "Slice",
                    &[source_type_name_for_type(&element)],
                )),
            )
        }
        TypedExprKind::Record { fields } => {
            let fields = fields
                .into_iter()
                .map(|field| TypedRecordField {
                    name: field.name,
                    value: refine_continuation_types(field.value, context),
                })
                .collect::<Vec<_>>();
            let ty = Type::Record(record_type_fields(&fields));
            typed(TypedExprKind::Record { fields }, ty)
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            let base = refine_continuation_types(*base, context);
            let fields = fields
                .into_iter()
                .map(|field| TypedRecordField {
                    name: field.name,
                    value: refine_continuation_types(field.value, context),
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
        TypedExprKind::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            let args = args
                .into_iter()
                .map(|arg| refine_continuation_types(arg, context))
                .collect::<Vec<_>>();
            let ty = context
                .instantiated_data_constructor_type(&data, &ctor, &args)
                .unwrap_or(expr.ty);
            typed(
                TypedExprKind::AdtCtor {
                    data,
                    ctor,
                    variants,
                    args,
                },
                ty,
            )
        }
        TypedExprKind::Field {
            receiver,
            name,
            access: _,
        } => {
            let receiver = refine_continuation_types(*receiver, context);
            let access = field_access_kind(&receiver.ty, &name);
            let ty = match access {
                FieldAccessKind::TuplePositionalRow { index } => {
                    tuple_field_type(&receiver.ty, index)
                }
                FieldAccessKind::RangeBoundary { .. } => Some(Type::I64),
                FieldAccessKind::SliceBoundary { .. } => Some(Type::I64),
                FieldAccessKind::RecordOrNominal => record_field_type(&receiver.ty, &name)
                    .or_else(|| context.nominal_field_type(&receiver.ty, &name)),
            }
            .unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::Field {
                    receiver: Box::new(receiver),
                    name,
                    access,
                },
                ty,
            )
        }
        TypedExprKind::MethodCall {
            receiver,
            name,
            args,
        } => {
            let receiver = refine_continuation_types(*receiver, context);
            let args = args
                .into_iter()
                .map(|arg| refine_continuation_types(arg, context))
                .collect::<Vec<_>>();
            let ty = field_callable_result_type(&receiver.ty, &name, args.len(), context);
            typed(
                TypedExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    name,
                    args,
                },
                ty,
            )
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            let receiver = refine_continuation_types(*receiver, context);
            let index = refine_continuation_types(*index, context);
            let access = index_access_kind(&receiver.ty, &index.ty);
            let ty = index_result_type(&access);
            typed(
                TypedExprKind::Index {
                    receiver: Box::new(receiver),
                    index: Box::new(index),
                    access,
                },
                ty,
            )
        }
        TypedExprKind::Range { start, end } => {
            let start = refine_continuation_types(*start, context);
            let end = refine_continuation_types(*end, context);
            typed(
                TypedExprKind::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                },
                Type::Nominal("Range".to_string()),
            )
        }
        TypedExprKind::Binary { op, lhs, rhs } => {
            let lhs = refine_continuation_types(*lhs, context);
            let rhs = refine_continuation_types(*rhs, context);
            let ty = binary_result_type(&lhs.ty, &rhs.ty);
            typed(
                TypedExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                ty,
            )
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond = refine_continuation_types(*cond, context);
            let then_branch = refine_continuation_types(*then_branch, context);
            let else_branch = refine_continuation_types(*else_branch, context);
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
        TypedExprKind::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            let scrutinee = refine_continuation_types(*scrutinee, context);
            let then_branch = refine_continuation_types(*then_branch, context);
            let else_branch = refine_continuation_types(*else_branch, context);
            let ty = common_type(&then_branch.ty, &else_branch.ty);
            typed(
                TypedExprKind::IfLet {
                    pattern,
                    scrutinee: Box::new(scrutinee),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                },
                ty,
            )
        }
        TypedExprKind::Match { scrutinee, arms } => {
            let scrutinee = refine_continuation_types(*scrutinee, context);
            let arms = arms
                .into_iter()
                .map(|arm| {
                    let mut body = refine_continuation_types(arm.body, context);
                    let bindings = context
                        .pattern_bindings_for(&arm.pattern, &scrutinee.ty)
                        .into_iter()
                        .collect::<TypeEnv>();
                    body = refine_pattern_binding_types(body, &bindings, context);
                    TypedMatchArm {
                        pattern: arm.pattern,
                        body,
                    }
                })
                .collect::<Vec<_>>();
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
        TypedExprKind::Nominal { name, expr } => {
            let expr = refine_continuation_types(*expr, context);
            typed(
                TypedExprKind::Nominal {
                    name: name.clone(),
                    expr: Box::new(expr),
                },
                Type::Nominal(name),
            )
        }
        TypedExprKind::Reset { multi, body } => {
            let body = refine_continuation_types(*body, context);
            let ty = body.ty.clone();
            typed(
                TypedExprKind::Reset {
                    multi,
                    body: Box::new(body),
                },
                ty,
            )
        }
        TypedExprKind::Shift { binder, body } => {
            let body = refine_continuation_types(*body, context);
            let input = typed_resume_input_type(&binder, &body);
            let body = rewrite_continuation_callee_input(body, &binder, &input);
            let ty = if input == Type::Unknown {
                body.ty.clone()
            } else {
                input
            };
            typed(
                TypedExprKind::Shift {
                    binder,
                    body: Box::new(body),
                },
                ty,
            )
        }
    }
}

fn typed_resume_input_type(binder: &str, body: &TypedExpr) -> Type {
    let mut inputs = Vec::new();
    collect_typed_resume_inputs(binder, body, &mut inputs);
    inputs
        .into_iter()
        .reduce(|left, right| common_type(&left, &right))
        .unwrap_or(Type::Unknown)
}

fn refine_pattern_binding_types(
    expr: TypedExpr,
    bindings: &TypeEnv,
    context: &TypeContext,
) -> TypedExpr {
    match expr.kind {
        TypedExprKind::Var(name) => {
            let ty = bindings.get(&name).cloned().unwrap_or(expr.ty);
            typed(TypedExprKind::Var(name), ty)
        }
        TypedExprKind::Lit(lit) => typed(TypedExprKind::Lit(lit), expr.ty),
        TypedExprKind::Lambda {
            param,
            param_ty,
            body,
        } => {
            let mut nested = bindings.clone();
            nested.remove(&param);
            let body = refine_pattern_binding_types(*body, &nested, context);
            let ty = Type::Func(Box::new(param_ty.clone()), Box::new(body.ty.clone()));
            typed(
                TypedExprKind::Lambda {
                    param,
                    param_ty,
                    body: Box::new(body),
                },
                ty,
            )
        }
        TypedExprKind::Call { callee, args } => {
            let callee = refine_pattern_binding_types(*callee, bindings, context);
            let args = args
                .into_iter()
                .map(|arg| refine_pattern_binding_types(arg, bindings, context))
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
        TypedExprKind::Tuple { fields, nominal: _ } => {
            let fields = fields
                .into_iter()
                .map(|field| refine_pattern_binding_types(field, bindings, context))
                .collect::<Vec<_>>();
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
        TypedExprKind::SliceLiteral { items, .. } => {
            let items = items
                .into_iter()
                .map(|item| refine_pattern_binding_types(item, bindings, context))
                .collect::<Vec<_>>();
            let element = slice_element_type(&items);
            typed(
                TypedExprKind::SliceLiteral {
                    items,
                    element: Box::new(element.clone()),
                },
                Type::Nominal(render_source_type_application(
                    "Slice",
                    &[source_type_name_for_type(&element)],
                )),
            )
        }
        TypedExprKind::Record { fields } => {
            let fields = fields
                .into_iter()
                .map(|field| TypedRecordField {
                    name: field.name,
                    value: refine_pattern_binding_types(field.value, bindings, context),
                })
                .collect::<Vec<_>>();
            let ty = Type::Record(record_type_fields(&fields));
            typed(TypedExprKind::Record { fields }, ty)
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            let base = refine_pattern_binding_types(*base, bindings, context);
            let fields = fields
                .into_iter()
                .map(|field| TypedRecordField {
                    name: field.name,
                    value: refine_pattern_binding_types(field.value, bindings, context),
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
        TypedExprKind::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            let args = args
                .into_iter()
                .map(|arg| refine_pattern_binding_types(arg, bindings, context))
                .collect::<Vec<_>>();
            typed(
                TypedExprKind::AdtCtor {
                    data: data.clone(),
                    ctor,
                    variants: variants.clone(),
                    args,
                },
                Type::Adt {
                    name: data,
                    variants,
                },
            )
        }
        TypedExprKind::Field {
            receiver,
            name,
            access: _,
        } => {
            let receiver = refine_pattern_binding_types(*receiver, bindings, context);
            let access = field_access_kind(&receiver.ty, &name);
            let ty = match access {
                FieldAccessKind::TuplePositionalRow { index } => {
                    tuple_field_type(&receiver.ty, index)
                }
                FieldAccessKind::RangeBoundary { .. } => Some(Type::I64),
                FieldAccessKind::SliceBoundary { .. } => Some(Type::I64),
                FieldAccessKind::RecordOrNominal => record_field_type(&receiver.ty, &name)
                    .or_else(|| context.nominal_field_type(&receiver.ty, &name)),
            }
            .unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::Field {
                    receiver: Box::new(receiver),
                    name,
                    access,
                },
                ty,
            )
        }
        TypedExprKind::MethodCall {
            receiver,
            name,
            args,
        } => {
            let receiver = refine_pattern_binding_types(*receiver, bindings, context);
            let args = args
                .into_iter()
                .map(|arg| refine_pattern_binding_types(arg, bindings, context))
                .collect::<Vec<_>>();
            let ty = field_callable_result_type(&receiver.ty, &name, args.len(), context);
            typed(
                TypedExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    name,
                    args,
                },
                ty,
            )
        }
        TypedExprKind::Index {
            receiver,
            index,
            access,
        } => {
            let receiver = refine_pattern_binding_types(*receiver, bindings, context);
            let index = refine_pattern_binding_types(*index, bindings, context);
            let access = match access {
                IndexAccessKind::SliceElement { .. } => index_access_kind(&receiver.ty, &index.ty),
                IndexAccessKind::Operator => IndexAccessKind::Operator,
            };
            let ty = match access {
                IndexAccessKind::SliceElement { .. } => index_result_type(&access),
                IndexAccessKind::Operator => expr.ty,
            };
            typed(
                TypedExprKind::Index {
                    receiver: Box::new(receiver),
                    index: Box::new(index),
                    access,
                },
                ty,
            )
        }
        TypedExprKind::Range { start, end } => typed(
            TypedExprKind::Range {
                start: Box::new(refine_pattern_binding_types(*start, bindings, context)),
                end: Box::new(refine_pattern_binding_types(*end, bindings, context)),
            },
            Type::Nominal("Range".to_string()),
        ),
        TypedExprKind::Binary { op, lhs, rhs } => {
            let lhs = refine_pattern_binding_types(*lhs, bindings, context);
            let rhs = refine_pattern_binding_types(*rhs, bindings, context);
            let ty = binary_result_type(&lhs.ty, &rhs.ty);
            typed(
                TypedExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                ty,
            )
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond = refine_pattern_binding_types(*cond, bindings, context);
            let then_branch = refine_pattern_binding_types(*then_branch, bindings, context);
            let else_branch = refine_pattern_binding_types(*else_branch, bindings, context);
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
        TypedExprKind::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            let scrutinee = refine_pattern_binding_types(*scrutinee, bindings, context);
            let then_branch = refine_pattern_binding_types(*then_branch, bindings, context);
            let else_branch = refine_pattern_binding_types(*else_branch, bindings, context);
            let ty = common_type(&then_branch.ty, &else_branch.ty);
            typed(
                TypedExprKind::IfLet {
                    pattern,
                    scrutinee: Box::new(scrutinee),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                },
                ty,
            )
        }
        TypedExprKind::Match { scrutinee, arms } => {
            let scrutinee = refine_pattern_binding_types(*scrutinee, bindings, context);
            let arms = arms
                .into_iter()
                .map(|arm| TypedMatchArm {
                    pattern: arm.pattern,
                    body: refine_pattern_binding_types(arm.body, bindings, context),
                })
                .collect::<Vec<_>>();
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
        TypedExprKind::Nominal { name, expr } => typed(
            TypedExprKind::Nominal {
                name: name.clone(),
                expr: Box::new(refine_pattern_binding_types(*expr, bindings, context)),
            },
            Type::Nominal(name),
        ),
        TypedExprKind::Reset { multi, body } => {
            let body = refine_pattern_binding_types(*body, bindings, context);
            let ty = body.ty.clone();
            typed(
                TypedExprKind::Reset {
                    multi,
                    body: Box::new(body),
                },
                ty,
            )
        }
        TypedExprKind::Shift { binder, body } => typed(
            TypedExprKind::Shift {
                binder,
                body: Box::new(refine_pattern_binding_types(*body, bindings, context)),
            },
            expr.ty,
        ),
    }
}

fn collect_typed_resume_inputs(binder: &str, expr: &TypedExpr, inputs: &mut Vec<Type>) {
    match &expr.kind {
        TypedExprKind::Call { callee, args } => {
            if is_typed_resume_callee(binder, callee) {
                match args.as_slice() {
                    [arg] => inputs.push(arg.ty.clone()),
                    _ => inputs.push(Type::Unknown),
                }
            }
            collect_typed_resume_inputs(binder, callee, inputs);
            for arg in args {
                collect_typed_resume_inputs(binder, arg, inputs);
            }
        }
        TypedExprKind::Lambda { body, .. }
        | TypedExprKind::Nominal { expr: body, .. }
        | TypedExprKind::Reset { body, .. }
        | TypedExprKind::Shift { body, .. } => collect_typed_resume_inputs(binder, body, inputs),
        TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                collect_typed_resume_inputs(binder, field, inputs);
            }
        }
        TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                collect_typed_resume_inputs(binder, item, inputs);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                collect_typed_resume_inputs(binder, &field.value, inputs);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            collect_typed_resume_inputs(binder, base, inputs);
            for field in fields {
                collect_typed_resume_inputs(binder, &field.value, inputs);
            }
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_typed_resume_inputs(binder, arg, inputs);
            }
        }
        TypedExprKind::Field { receiver, .. } => {
            collect_typed_resume_inputs(binder, receiver, inputs);
        }
        TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_typed_resume_inputs(binder, receiver, inputs);
            for arg in args {
                collect_typed_resume_inputs(binder, arg, inputs);
            }
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            collect_typed_resume_inputs(binder, receiver, inputs);
            collect_typed_resume_inputs(binder, index, inputs);
        }
        TypedExprKind::Range { start, end } => {
            collect_typed_resume_inputs(binder, start, inputs);
            collect_typed_resume_inputs(binder, end, inputs);
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            collect_typed_resume_inputs(binder, lhs, inputs);
            collect_typed_resume_inputs(binder, rhs, inputs);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_typed_resume_inputs(binder, cond, inputs);
            collect_typed_resume_inputs(binder, then_branch, inputs);
            collect_typed_resume_inputs(binder, else_branch, inputs);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_typed_resume_inputs(binder, scrutinee, inputs);
            collect_typed_resume_inputs(binder, then_branch, inputs);
            collect_typed_resume_inputs(binder, else_branch, inputs);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_typed_resume_inputs(binder, scrutinee, inputs);
            for arm in arms {
                collect_typed_resume_inputs(binder, &arm.body, inputs);
            }
        }
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
    }
}

fn rewrite_continuation_callee_input(expr: TypedExpr, binder: &str, input: &Type) -> TypedExpr {
    match expr.kind {
        TypedExprKind::Var(name) if name == binder => {
            if let Type::Continuation {
                multi,
                input: _,
                answer,
            } = expr.ty
            {
                typed(
                    TypedExprKind::Var(name),
                    Type::Continuation {
                        multi,
                        input: Box::new(input.clone()),
                        answer,
                    },
                )
            } else {
                typed(TypedExprKind::Var(name), expr.ty)
            }
        }
        TypedExprKind::Var(name) => typed(TypedExprKind::Var(name), expr.ty),
        TypedExprKind::Lit(lit) => typed(TypedExprKind::Lit(lit), expr.ty),
        TypedExprKind::Lambda {
            param,
            param_ty,
            body,
        } => typed(
            TypedExprKind::Lambda {
                param,
                param_ty,
                body: Box::new(rewrite_continuation_callee_input(*body, binder, input)),
            },
            expr.ty,
        ),
        TypedExprKind::Call { callee, args } => typed(
            TypedExprKind::Call {
                callee: Box::new(rewrite_continuation_callee_input(*callee, binder, input)),
                args: args
                    .into_iter()
                    .map(|arg| rewrite_continuation_callee_input(arg, binder, input))
                    .collect(),
            },
            expr.ty,
        ),
        TypedExprKind::Tuple { fields, nominal } => typed(
            TypedExprKind::Tuple {
                nominal,
                fields: fields
                    .into_iter()
                    .map(|field| rewrite_continuation_callee_input(field, binder, input))
                    .collect(),
            },
            expr.ty,
        ),
        TypedExprKind::SliceLiteral { items, element } => typed(
            TypedExprKind::SliceLiteral {
                items: items
                    .into_iter()
                    .map(|item| rewrite_continuation_callee_input(item, binder, input))
                    .collect(),
                element,
            },
            expr.ty,
        ),
        TypedExprKind::Record { fields } => typed(
            TypedExprKind::Record {
                fields: fields
                    .into_iter()
                    .map(|field| TypedRecordField {
                        name: field.name,
                        value: rewrite_continuation_callee_input(field.value, binder, input),
                    })
                    .collect(),
            },
            expr.ty,
        ),
        TypedExprKind::RecordUpdate { base, fields } => typed(
            TypedExprKind::RecordUpdate {
                base: Box::new(rewrite_continuation_callee_input(*base, binder, input)),
                fields: fields
                    .into_iter()
                    .map(|field| TypedRecordField {
                        name: field.name,
                        value: rewrite_continuation_callee_input(field.value, binder, input),
                    })
                    .collect(),
            },
            expr.ty,
        ),
        TypedExprKind::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => typed(
            TypedExprKind::AdtCtor {
                data,
                ctor,
                variants,
                args: args
                    .into_iter()
                    .map(|arg| rewrite_continuation_callee_input(arg, binder, input))
                    .collect(),
            },
            expr.ty,
        ),
        TypedExprKind::Field {
            receiver,
            name,
            access,
        } => typed(
            TypedExprKind::Field {
                receiver: Box::new(rewrite_continuation_callee_input(*receiver, binder, input)),
                name,
                access,
            },
            expr.ty,
        ),
        TypedExprKind::MethodCall {
            receiver,
            name,
            args,
        } => typed(
            TypedExprKind::MethodCall {
                receiver: Box::new(rewrite_continuation_callee_input(*receiver, binder, input)),
                name,
                args: args
                    .into_iter()
                    .map(|arg| rewrite_continuation_callee_input(arg, binder, input))
                    .collect(),
            },
            expr.ty,
        ),
        TypedExprKind::Index {
            receiver,
            index,
            access,
        } => typed(
            TypedExprKind::Index {
                receiver: Box::new(rewrite_continuation_callee_input(*receiver, binder, input)),
                index: Box::new(rewrite_continuation_callee_input(*index, binder, input)),
                access,
            },
            expr.ty,
        ),
        TypedExprKind::Range { start, end } => typed(
            TypedExprKind::Range {
                start: Box::new(rewrite_continuation_callee_input(*start, binder, input)),
                end: Box::new(rewrite_continuation_callee_input(*end, binder, input)),
            },
            expr.ty,
        ),
        TypedExprKind::Binary { op, lhs, rhs } => typed(
            TypedExprKind::Binary {
                op,
                lhs: Box::new(rewrite_continuation_callee_input(*lhs, binder, input)),
                rhs: Box::new(rewrite_continuation_callee_input(*rhs, binder, input)),
            },
            expr.ty,
        ),
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => typed(
            TypedExprKind::If {
                cond: Box::new(rewrite_continuation_callee_input(*cond, binder, input)),
                then_branch: Box::new(rewrite_continuation_callee_input(
                    *then_branch,
                    binder,
                    input,
                )),
                else_branch: Box::new(rewrite_continuation_callee_input(
                    *else_branch,
                    binder,
                    input,
                )),
            },
            expr.ty,
        ),
        TypedExprKind::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => typed(
            TypedExprKind::IfLet {
                pattern,
                scrutinee: Box::new(rewrite_continuation_callee_input(*scrutinee, binder, input)),
                then_branch: Box::new(rewrite_continuation_callee_input(
                    *then_branch,
                    binder,
                    input,
                )),
                else_branch: Box::new(rewrite_continuation_callee_input(
                    *else_branch,
                    binder,
                    input,
                )),
            },
            expr.ty,
        ),
        TypedExprKind::Match { scrutinee, arms } => typed(
            TypedExprKind::Match {
                scrutinee: Box::new(rewrite_continuation_callee_input(*scrutinee, binder, input)),
                arms: arms
                    .into_iter()
                    .map(|arm| TypedMatchArm {
                        pattern: arm.pattern,
                        body: rewrite_continuation_callee_input(arm.body, binder, input),
                    })
                    .collect(),
            },
            expr.ty,
        ),
        TypedExprKind::Nominal { name, expr: body } => typed(
            TypedExprKind::Nominal {
                name,
                expr: Box::new(rewrite_continuation_callee_input(*body, binder, input)),
            },
            expr.ty,
        ),
        TypedExprKind::Reset { multi, body } => typed(
            TypedExprKind::Reset {
                multi,
                body: Box::new(rewrite_continuation_callee_input(*body, binder, input)),
            },
            expr.ty,
        ),
        TypedExprKind::Shift {
            binder: shift_binder,
            body,
        } => typed(
            TypedExprKind::Shift {
                binder: shift_binder,
                body: Box::new(rewrite_continuation_callee_input(*body, binder, input)),
            },
            expr.ty,
        ),
    }
}

fn is_typed_resume_callee(binder: &str, callee: &TypedExpr) -> bool {
    matches!(
        (&callee.kind, &callee.ty),
        (
            TypedExprKind::Var(name),
            Type::Continuation {
                input: _,
                answer: _,
                ..
            }
        ) if name == binder
    )
}

pub(crate) fn common_type(left: &Type, right: &Type) -> Type {
    if left == right {
        left.clone()
    } else if adt_nominal_instance_matches(left, right) {
        right.clone()
    } else if adt_nominal_instance_matches(right, left) {
        left.clone()
    } else {
        Type::Unknown
    }
}

fn adt_nominal_instance_matches(adt: &Type, nominal: &Type) -> bool {
    let Type::Adt { name, .. } = adt else {
        return false;
    };
    let Type::Nominal(nominal) = nominal else {
        return false;
    };
    parse_type_header(nominal).is_some_and(
        |header| matches!(header, ParsedTypeHeader::Nominal { base, .. } if base == *name),
    )
}

fn binary_result_type(lhs: &Type, rhs: &Type) -> Type {
    match (lhs, rhs) {
        (Type::I64, Type::I64) => Type::I64,
        _ => Type::Unknown,
    }
}

fn index_access_kind(receiver: &Type, index: &Type) -> IndexAccessKind {
    if index != &Type::I64 {
        return IndexAccessKind::Operator;
    }
    let Type::Nominal(name) = receiver else {
        return IndexAccessKind::Operator;
    };
    let Some(ParsedTypeHeader::Nominal { base, args }) = parse_type_header(name) else {
        return IndexAccessKind::Operator;
    };
    match (base.as_str(), args.as_slice()) {
        ("Slice", [element]) => IndexAccessKind::SliceElement {
            element: element.to_type(),
        },
        _ => IndexAccessKind::Operator,
    }
}

fn index_result_type(access: &IndexAccessKind) -> Type {
    match access {
        IndexAccessKind::SliceElement { element } => element.clone(),
        IndexAccessKind::Operator => Type::Unknown,
    }
}

fn call_result_type(callee: &Type, arity: usize) -> Type {
    if let Type::Continuation { answer, .. } = callee {
        return if arity == 1 {
            answer.as_ref().clone()
        } else {
            Type::Unknown
        };
    }
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

fn tuple_field_type(receiver: &Type, index: usize) -> Option<Type> {
    let Type::Tuple(fields) = receiver else {
        return None;
    };
    fields.get(index).cloned()
}

fn slice_element_type(items: &[TypedExpr]) -> Type {
    let Some((first, rest)) = items.split_first() else {
        return Type::Unknown;
    };
    if rest.iter().all(|item| item.ty == first.ty) {
        first.ty.clone()
    } else {
        Type::Unknown
    }
}

fn field_access_kind(receiver: &Type, name: &str) -> FieldAccessKind {
    if matches!(receiver, Type::Nominal(nominal) if nominal == "Range") {
        if let Some(boundary) = RangeBoundary::from_source_name(name) {
            return FieldAccessKind::RangeBoundary { boundary };
        }
    }
    if nominal_base_name_for_type(receiver) == Some("Slice") {
        if let Some(boundary) = SliceBoundary::from_source_name(name) {
            return FieldAccessKind::SliceBoundary { boundary };
        }
    }
    if let Type::Tuple(fields) = receiver {
        if let Some(index) = tuple_row_fields(fields)
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.index)
        {
            return FieldAccessKind::TuplePositionalRow { index };
        }
    }
    FieldAccessKind::RecordOrNominal
}

impl RangeBoundary {
    pub fn from_source_name(name: &str) -> Option<Self> {
        match name {
            "start" => Some(Self::Start),
            "end" => Some(Self::End),
            _ => None,
        }
    }

    pub fn source_name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

impl SliceBoundary {
    pub fn from_source_name(name: &str) -> Option<Self> {
        match name {
            "len" => Some(Self::Len),
            _ => None,
        }
    }

    pub fn source_name(self) -> &'static str {
        match self {
            Self::Len => "len",
        }
    }
}

fn nominal_base_name_for_type(ty: &Type) -> Option<&str> {
    let Type::Nominal(name) = ty else {
        return None;
    };
    Some(nominal_base_name(name))
}

fn tuple_row_fields(fields: &[Type]) -> Vec<TupleRowField> {
    fields
        .iter()
        .enumerate()
        .map(|(index, _)| TupleRowField {
            name: format!("_{}", index + 1),
            index,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TupleRowField {
    name: String,
    index: usize,
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

fn nominal_type_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Nominal(name) => Some(name),
        _ => None,
    }
}

fn nominal_base_name(name: &str) -> &str {
    nominal_header_base_range(name)
        .and_then(|range| name.get(range))
        .unwrap_or(name)
}

pub(crate) fn source_type_name_to_type(name: &str) -> Type {
    parse_type_header(name)
        .map(|header| header.to_type())
        .unwrap_or_else(|| Type::Nominal(name.to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParsedTypeHeader {
    ScalarI64,
    ScalarBool,
    Nominal {
        base: String,
        args: Vec<ParsedTypeHeader>,
    },
    Tuple {
        args: Vec<ParsedTypeHeader>,
    },
    Continuation {
        multi: bool,
        input: Box<ParsedTypeHeader>,
        answer: Box<ParsedTypeHeader>,
    },
    Callable {
        param: Box<ParsedTypeHeader>,
        result: Box<ParsedTypeHeader>,
    },
}

impl ParsedTypeHeader {
    fn to_type(&self) -> Type {
        match self {
            ParsedTypeHeader::ScalarI64 => Type::I64,
            ParsedTypeHeader::ScalarBool => Type::Bool,
            ParsedTypeHeader::Nominal { base, args } => {
                if args.is_empty() {
                    Type::Nominal(base.clone())
                } else {
                    Type::Nominal(render_nominal_type_header(base, args))
                }
            }
            ParsedTypeHeader::Tuple { args } => {
                Type::Tuple(args.iter().map(ParsedTypeHeader::to_type).collect())
            }
            ParsedTypeHeader::Continuation {
                multi,
                input,
                answer,
            } => Type::Continuation {
                multi: *multi,
                input: Box::new(input.to_type()),
                answer: Box::new(answer.to_type()),
            },
            ParsedTypeHeader::Callable { param, result } => {
                Type::Func(Box::new(param.to_type()), Box::new(result.to_type()))
            }
        }
    }
}

fn parse_type_header(name: &str) -> Option<ParsedTypeHeader> {
    let name = name.trim();
    match name {
        "I64" | "i64" => return Some(ParsedTypeHeader::ScalarI64),
        "Bool" | "bool" => return Some(ParsedTypeHeader::ScalarBool),
        "" => return None,
        _ => {}
    }

    if let Some(callable) = parse_callable_type_header(name) {
        return Some(callable);
    }

    let Some((base, args)) = parse_application_type_header(name)? else {
        return Some(ParsedTypeHeader::Nominal {
            base: name.to_string(),
            args: Vec::new(),
        });
    };

    let parsed_args = args
        .iter()
        .map(|arg| parse_type_header(arg))
        .collect::<Option<Vec<_>>>()?;

    match (base.as_str(), parsed_args.as_slice()) {
        ("Tuple", _) => Some(ParsedTypeHeader::Tuple { args: parsed_args }),
        ("Cont1", [input, answer]) => Some(ParsedTypeHeader::Continuation {
            multi: false,
            input: Box::new(input.clone()),
            answer: Box::new(answer.clone()),
        }),
        ("ContN", [input, answer]) => Some(ParsedTypeHeader::Continuation {
            multi: true,
            input: Box::new(input.clone()),
            answer: Box::new(answer.clone()),
        }),
        ("Cont1" | "ContN", _) => None,
        _ => Some(ParsedTypeHeader::Nominal {
            base,
            args: parsed_args,
        }),
    }
}

fn parse_callable_type_header(name: &str) -> Option<ParsedTypeHeader> {
    let arrow = top_level_arrow(name)?;
    let param = parenthesized_type(name[..arrow].trim())?;
    let result = name[arrow + 2..].trim();
    if result.is_empty() {
        return None;
    }
    Some(ParsedTypeHeader::Callable {
        param: Box::new(parse_type_header(param)?),
        result: Box::new(parse_type_header(result)?),
    })
}

fn parse_application_type_header(name: &str) -> Option<Option<(String, Vec<String>)>> {
    let Some(range) = nominal_header_base_range(name) else {
        return Some(None);
    };
    let base = name.get(range)?.to_string();
    let args = application_type_arg_slice(name)?;
    let args = if args.trim().is_empty() {
        Vec::new()
    } else {
        split_type_header_args(args)?
    };
    Some(Some((base, args)))
}

fn nominal_header_base_range(name: &str) -> Option<std::ops::Range<usize>> {
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    for (index, ch) in name.char_indices() {
        match ch {
            '[' if paren_depth == 0 => {
                if square_depth == 0 {
                    return Some(0..index);
                }
                square_depth = square_depth.checked_add(1)?;
            }
            ']' if paren_depth == 0 => square_depth = square_depth.checked_sub(1)?,
            '(' => paren_depth = paren_depth.checked_add(1)?,
            ')' => paren_depth = paren_depth.checked_sub(1)?,
            _ => {}
        }
    }
    None
}

fn application_type_arg_slice(name: &str) -> Option<&str> {
    let open = nominal_header_base_range(name)?.end;
    let close = name.strip_suffix(']')?;
    close.get(open + 1..)
}

fn split_type_header_args(args: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in args.char_indices() {
        match ch {
            '[' => square_depth = square_depth.checked_add(1)?,
            ']' => square_depth = square_depth.checked_sub(1)?,
            '(' => paren_depth = paren_depth.checked_add(1)?,
            ')' => paren_depth = paren_depth.checked_sub(1)?,
            ',' if square_depth == 0 && paren_depth == 0 => {
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
    if square_depth != 0 || paren_depth != 0 {
        return None;
    }
    let arg = args[start..].trim();
    if arg.is_empty() {
        return None;
    }
    out.push(arg.to_string());
    Some(out)
}

fn render_nominal_type_header(base: &str, args: &[ParsedTypeHeader]) -> String {
    let mut rendered = String::from(base);
    rendered.push('[');
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            rendered.push(',');
        }
        rendered.push_str(&render_type_header(arg));
    }
    rendered.push(']');
    rendered
}

fn render_type_header(header: &ParsedTypeHeader) -> String {
    match header {
        ParsedTypeHeader::ScalarI64 => "i64".to_string(),
        ParsedTypeHeader::ScalarBool => "bool".to_string(),
        ParsedTypeHeader::Nominal { base, args } => {
            if args.is_empty() {
                base.clone()
            } else {
                render_nominal_type_header(base, args)
            }
        }
        ParsedTypeHeader::Tuple { args } => render_nominal_type_header("Tuple", args),
        ParsedTypeHeader::Continuation {
            multi,
            input,
            answer,
        } => {
            let base = if *multi { "ContN" } else { "Cont1" };
            render_nominal_type_header(base, &[input.as_ref().clone(), answer.as_ref().clone()])
        }
        ParsedTypeHeader::Callable { param, result } => {
            format!(
                "({}) -> {}",
                render_type_header(param),
                render_type_header(result)
            )
        }
    }
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

fn collect_payload_substitutions(
    payload: &Type,
    actual: &Type,
    generics: &[String],
    substitutions: &mut BTreeMap<String, Type>,
) -> Option<()> {
    match payload {
        Type::Nominal(name) if generics.iter().any(|generic| generic == name) => {
            match substitutions.get(name) {
                Some(existing) if existing != actual => None,
                Some(_) => Some(()),
                None => {
                    substitutions.insert(name.clone(), actual.clone());
                    Some(())
                }
            }
        }
        Type::Tuple(payload_fields) => {
            let Type::Tuple(actual_fields) = actual else {
                return Some(());
            };
            if payload_fields.len() != actual_fields.len() {
                return None;
            }
            for (payload, actual) in payload_fields.iter().zip(actual_fields) {
                collect_payload_substitutions(payload, actual, generics, substitutions)?;
            }
            Some(())
        }
        Type::Record(payload_fields) => {
            let Type::Record(actual_fields) = actual else {
                return Some(());
            };
            for payload_field in payload_fields {
                let actual_field = field_type(actual_fields, &payload_field.name)?;
                collect_payload_substitutions(
                    &payload_field.ty,
                    &actual_field,
                    generics,
                    substitutions,
                )?;
            }
            Some(())
        }
        Type::Func(payload_param, payload_result) => {
            let Type::Func(actual_param, actual_result) = actual else {
                return Some(());
            };
            collect_payload_substitutions(payload_param, actual_param, generics, substitutions)?;
            collect_payload_substitutions(payload_result, actual_result, generics, substitutions)
        }
        Type::Continuation {
            input: payload_input,
            answer: payload_answer,
            ..
        } => {
            let Type::Continuation {
                input: actual_input,
                answer: actual_answer,
                ..
            } = actual
            else {
                return Some(());
            };
            collect_payload_substitutions(payload_input, actual_input, generics, substitutions)?;
            collect_payload_substitutions(payload_answer, actual_answer, generics, substitutions)
        }
        Type::Adt { .. } | Type::Unknown | Type::I64 | Type::Bool | Type::Nominal(_) => Some(()),
    }
}

fn render_data_instance_type(
    data: &str,
    generics: &[String],
    substitutions: &BTreeMap<String, Type>,
) -> String {
    let args = generics
        .iter()
        .map(|generic| {
            substitutions
                .get(generic)
                .map(source_type_name_for_type)
                .unwrap_or_else(|| generic.clone())
        })
        .collect::<Vec<_>>();
    render_source_type_application(data, &args)
}

fn render_source_type_application(base: &str, args: &[String]) -> String {
    let mut rendered = String::from(base);
    rendered.push('[');
    rendered.push_str(&args.join(","));
    rendered.push(']');
    rendered
}

fn source_type_name_for_type(ty: &Type) -> String {
    match ty {
        Type::I64 => "i64".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Nominal(name) => name.clone(),
        Type::Tuple(fields) => {
            let args = fields
                .iter()
                .map(source_type_name_for_type)
                .collect::<Vec<_>>();
            render_source_type_application("Tuple", &args)
        }
        Type::Continuation {
            multi,
            input,
            answer,
        } => {
            let base = if *multi { "ContN" } else { "Cont1" };
            let args = vec![
                source_type_name_for_type(input),
                source_type_name_for_type(answer),
            ];
            render_source_type_application(base, &args)
        }
        Type::Func(param, result) => {
            format!(
                "({}) -> {}",
                source_type_name_for_type(param),
                source_type_name_for_type(result)
            )
        }
        Type::Record(_) | Type::Adt { .. } | Type::Unknown => type_stable_name(ty),
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
