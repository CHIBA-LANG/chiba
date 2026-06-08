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
    DynRowPackage {
        payload: Box<TypedExpr>,
        fields: Vec<TypedDynRowField>,
    },
    DynRowField {
        package: Box<TypedExpr>,
        name: String,
    },
    AdtCtor {
        data: String,
        ctor: String,
        variants: Vec<String>,
        args: Vec<TypedExpr>,
    },
    AdtToTuple {
        value: Box<TypedExpr>,
        tuple_nominal: String,
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
        builtin: Option<BuiltinMethodCall>,
    },
    Assign {
        target: Box<TypedExpr>,
        value: Box<TypedExpr>,
        builtin: Option<BuiltinMethodCall>,
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
pub struct TypedDynRowField {
    pub name: String,
    pub source: DynRowFieldSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DynRowFieldSource {
    Field,
    ContractObligation {
        ty: Type,
    },
    ReceiverMethod {
        symbol: String,
        runtime_target: String,
        param_ty: Type,
        result_ty: Type,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldAccessKind {
    RecordOrNominal,
    TuplePositionalRow {
        index: usize,
    },
    RangeBoundary {
        boundary: RangeBoundary,
    },
    AggregateBoundary {
        kind: AggregateKind,
        boundary: AggregateBoundary,
    },
    TextBoundary {
        kind: TextKind,
        boundary: TextBoundary,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexAccessKind {
    Operator,
    AggregateElement { kind: AggregateKind, element: Type },
    AggregateSlice { kind: AggregateKind, element: Type },
    TextByte { kind: TextKind },
    TextSlice { kind: TextKind },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggregateKind {
    Slice,
    Array,
    Vec,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextKind {
    Str,
    String,
    CStr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeBoundary {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggregateBoundary {
    Len,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBoundary {
    Len,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinMethodCall {
    VecNew,
    VecPush,
    VecFreeze,
    StringNew,
    StringFrom,
    StringConcat,
    StringAsStr,
    StringToCStr,
    StringPushRune,
    TextLen { kind: TextKind },
    TextRuneLen { kind: TextKind },
    TextCharAt { kind: TextKind },
    RefNew,
    RefGet,
    RefSet,
    UnsafeRefNew,
    UnsafeRefGet,
    UnsafeRefSet,
}

impl BuiltinMethodCall {
    pub fn debug_name(self) -> &'static str {
        match self {
            Self::VecNew => "builtin.vec.new",
            Self::VecPush => "builtin.vec.push",
            Self::VecFreeze => "builtin.vec.freeze",
            Self::StringNew => "builtin.string.new",
            Self::StringFrom => "builtin.string.from",
            Self::StringConcat => "builtin.string.concat",
            Self::StringAsStr => "builtin.string.as_str",
            Self::StringToCStr => "builtin.string.to_cstr",
            Self::StringPushRune => "builtin.string.push_rune",
            Self::TextLen { kind } => match kind {
                TextKind::Str => "builtin.str.len",
                TextKind::String => "builtin.string.len",
                TextKind::CStr => "builtin.cstr.len",
            },
            Self::TextRuneLen { kind } => match kind {
                TextKind::Str => "builtin.str.rune_len",
                TextKind::String => "builtin.string.rune_len",
                TextKind::CStr => "builtin.cstr.rune_len",
            },
            Self::TextCharAt { kind } => match kind {
                TextKind::Str => "builtin.str.char_at",
                TextKind::String => "builtin.string.char_at",
                TextKind::CStr => "builtin.cstr.char_at",
            },
            Self::RefNew => "builtin.ref.new",
            Self::RefGet => "builtin.ref.get",
            Self::RefSet => "builtin.ref.set",
            Self::UnsafeRefNew => "builtin.unsafe_ref.new",
            Self::UnsafeRefGet => "builtin.unsafe_ref.get",
            Self::UnsafeRefSet => "builtin.unsafe_ref.set",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Unknown,
    I64,
    Rune,
    Bool,
    Tuple(Vec<Type>),
    Record(Vec<RecordTypeField>),
    DynRow(Vec<RecordTypeField>),
    Adt {
        name: String,
        variants: Vec<String>,
    },
    Nominal(String),
    Func(Box<Type>, Box<Type>, SendColor),
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedTypeFieldHeader {
    name: String,
    ty: ParsedTypeHeader,
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
    receiver_methods: BTreeMap<(String, String), ReceiverMethodSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NominalRowDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<RecordTypeField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiverMethodSummary {
    pub receiver: String,
    pub name: String,
    pub symbol: String,
    pub runtime_target: String,
    pub param_tys: Vec<Type>,
    pub result_ty: Type,
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

    pub fn insert_receiver_method(&mut self, method: ReceiverMethodSummary) {
        self.receiver_methods
            .insert((method.receiver.clone(), method.name.clone()), method);
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

    pub fn receiver_method(&self, receiver: &Type, name: &str) -> Option<&ReceiverMethodSummary> {
        let Type::Nominal(nominal) = receiver else {
            return None;
        };
        self.receiver_methods
            .get(&(nominal.clone(), name.to_string()))
            .or_else(|| {
                let base = nominal_base_name(nominal);
                self.receiver_methods
                    .get(&(base.to_string(), name.to_string()))
            })
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

pub fn type_expr_with_expected(
    expr: &Expr,
    env: &TypeEnv,
    context: &TypeContext,
    expected: &Type,
) -> TypedExpr {
    let typed = type_expr_with_context(expr, env, context);
    coerce_expected(typed, expected, context)
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
        Expr::Lit(Literal::Rune(value)) => {
            typed(TypedExprKind::Lit(Literal::Rune(*value)), Type::Rune)
        }
        Expr::Lit(Literal::Bool(value)) => {
            typed(TypedExprKind::Lit(Literal::Bool(*value)), Type::Bool)
        }
        Expr::Lit(Literal::String(value)) => typed(
            TypedExprKind::Lit(Literal::String(value.clone())),
            Type::Nominal("String".to_string()),
        ),
        Expr::Lit(Literal::CStr(value)) => typed(
            TypedExprKind::Lit(Literal::CStr(value.clone())),
            Type::Nominal("cstr".to_string()),
        ),
        Expr::Lambda { param, body } => {
            let param_ty = Type::Unknown;
            let mut env = env.clone();
            env.insert(param.clone(), param_ty.clone());
            let body = type_expr_with_context_and_controls(body, &env, context, controls);
            let ty = Type::Func(
                Box::new(param_ty.clone()),
                Box::new(body.ty.clone()),
                SendColor::Obligation,
            );
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
            if let Some(expr) = type_compiler_intrinsic_call(callee, args, env, context, controls) {
                return expr;
            }
            let callee = type_expr_with_context_and_controls(callee, env, context, controls);
            let args = args
                .iter()
                .map(|arg| type_expr_with_context_and_controls(arg, env, context, controls))
                .collect::<Vec<_>>();
            let args = coerce_call_args(&callee.ty, args, context);
            let ty = dyn_row_method_call_result_type(&callee.ty, args.len());
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
            if matches!(receiver.ty, Type::DynRow(_)) {
                let ty = dyn_row_field_type(&receiver.ty, name).unwrap_or(Type::Unknown);
                return typed(
                    TypedExprKind::DynRowField {
                        package: Box::new(receiver),
                        name: name.clone(),
                    },
                    ty,
                );
            }
            let access = field_access_kind(&receiver.ty, name);
            let ty = match access {
                FieldAccessKind::TuplePositionalRow { index } => {
                    tuple_field_type(&receiver.ty, index)
                }
                FieldAccessKind::RangeBoundary { .. } => Some(Type::I64),
                FieldAccessKind::AggregateBoundary { .. } => Some(Type::I64),
                FieldAccessKind::TextBoundary { .. } => Some(Type::I64),
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
            if matches!(receiver.ty, Type::DynRow(_)) {
                let field_ty = dyn_row_field_type(&receiver.ty, name).unwrap_or(Type::Unknown);
                let callee = typed(
                    TypedExprKind::DynRowField {
                        package: Box::new(receiver),
                        name: name.clone(),
                    },
                    field_ty,
                );
                let ty = dyn_row_method_call_result_type(&callee.ty, args.len());
                return typed(
                    TypedExprKind::Call {
                        callee: Box::new(callee),
                        args,
                    },
                    ty,
                );
            }
            if let Some((access, field_ty)) =
                field_callable_callee_type(&receiver.ty, name, context)
            {
                let callee = typed(
                    TypedExprKind::Field {
                        receiver: Box::new(receiver),
                        name: name.clone(),
                        access,
                    },
                    field_ty,
                );
                let ty = call_result_type(&callee.ty, args.len());
                return typed(
                    TypedExprKind::Call {
                        callee: Box::new(callee),
                        args,
                    },
                    ty,
                );
            }
            let builtin = builtin_method_call(&receiver, name, &args);
            let ty = builtin
                .and_then(|builtin| builtin_method_result_type(builtin, &receiver.ty, &args))
                .unwrap_or_else(|| {
                    field_callable_result_type(&receiver.ty, name, args.len(), context)
                });
            typed(
                TypedExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    name: name.clone(),
                    args,
                    builtin,
                },
                ty,
            )
        }
        Expr::Assign { target, value } => {
            let target = type_expr_with_context_and_controls(target, env, context, controls);
            let value = type_expr_with_context_and_controls(value, env, context, controls);
            let args = [value.clone()];
            let builtin = match builtin_method_call(&target, "set", &args) {
                Some(BuiltinMethodCall::RefSet) => Some(BuiltinMethodCall::RefSet),
                Some(BuiltinMethodCall::UnsafeRefSet) => Some(BuiltinMethodCall::UnsafeRefSet),
                _ => None,
            };
            let ty = builtin
                .and_then(|builtin| builtin_method_result_type(builtin, &target.ty, &args))
                .unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::Assign {
                    target: Box::new(target),
                    value: Box::new(value),
                    builtin,
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

fn type_compiler_intrinsic_call(
    callee: &Expr,
    args: &[Expr],
    env: &TypeEnv,
    context: &TypeContext,
    controls: &mut Vec<ContinuationTypeBoundary>,
) -> Option<TypedExpr> {
    if compiler_intrinsic_callee_name(callee)? != "adt_to_tuple" || args.len() != 1 {
        return None;
    }
    let value = type_expr_with_context_and_controls(&args[0], env, context, controls);
    let field_types = adt_tuple_field_types(&value);
    let tuple_nominal = tuple_nominal_name(&field_types);
    Some(typed(
        TypedExprKind::AdtToTuple {
            value: Box::new(value),
            tuple_nominal,
        },
        Type::Tuple(field_types),
    ))
}

fn compiler_intrinsic_callee_name(callee: &Expr) -> Option<&str> {
    match callee {
        Expr::Var(name) => Some(name.as_str()),
        Expr::Instantiate { callee, .. } => compiler_intrinsic_callee_name(callee),
        _ => None,
    }
}

fn adt_tuple_field_types(value: &TypedExpr) -> Vec<Type> {
    let mut fields = vec![Type::Unknown];
    match &value.kind {
        TypedExprKind::AdtCtor { args, .. } => {
            fields.extend(args.iter().map(|arg| arg.ty.clone()));
        }
        _ => fields.push(Type::Unknown),
    }
    fields
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
            let ty = Type::Func(
                Box::new(param_ty.clone()),
                Box::new(body.ty.clone()),
                SendColor::Obligation,
            );
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
            let ty = dyn_row_method_call_result_type(&callee.ty, args.len());
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
        TypedExprKind::DynRowPackage { payload, fields } => {
            let payload = refine_continuation_types(*payload, context);
            typed(
                TypedExprKind::DynRowPackage {
                    payload: Box::new(payload),
                    fields,
                },
                expr.ty,
            )
        }
        TypedExprKind::DynRowField { package, name } => {
            let package = refine_continuation_types(*package, context);
            let ty = dyn_row_field_type(&package.ty, &name).unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::DynRowField {
                    package: Box::new(package),
                    name,
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
        TypedExprKind::AdtToTuple {
            value,
            tuple_nominal: _,
        } => {
            let value = refine_continuation_types(*value, context);
            let field_types = adt_tuple_field_types(&value);
            typed(
                TypedExprKind::AdtToTuple {
                    value: Box::new(value),
                    tuple_nominal: tuple_nominal_name(&field_types),
                },
                Type::Tuple(field_types),
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
                FieldAccessKind::AggregateBoundary { .. } => Some(Type::I64),
                FieldAccessKind::TextBoundary { .. } => Some(Type::I64),
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
            ..
        } => {
            let receiver = refine_continuation_types(*receiver, context);
            let args = args
                .into_iter()
                .map(|arg| refine_continuation_types(arg, context))
                .collect::<Vec<_>>();
            let builtin = builtin_method_call(&receiver, &name, &args);
            let ty = builtin
                .and_then(|builtin| builtin_method_result_type(builtin, &receiver.ty, &args))
                .unwrap_or_else(|| {
                    field_callable_result_type(&receiver.ty, &name, args.len(), context)
                });
            typed(
                TypedExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    name,
                    args,
                    builtin,
                },
                ty,
            )
        }
        TypedExprKind::Assign { target, value, .. } => {
            let target = refine_continuation_types(*target, context);
            let value = refine_continuation_types(*value, context);
            let args = [value.clone()];
            let builtin = match builtin_method_call(&target, "set", &args) {
                Some(BuiltinMethodCall::RefSet) => Some(BuiltinMethodCall::RefSet),
                Some(BuiltinMethodCall::UnsafeRefSet) => Some(BuiltinMethodCall::UnsafeRefSet),
                _ => None,
            };
            let ty = builtin
                .and_then(|builtin| builtin_method_result_type(builtin, &target.ty, &args))
                .unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::Assign {
                    target: Box::new(target),
                    value: Box::new(value),
                    builtin,
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
            let ty = Type::Func(
                Box::new(param_ty.clone()),
                Box::new(body.ty.clone()),
                SendColor::Obligation,
            );
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
            let ty = dyn_row_method_call_result_type(&callee.ty, args.len());
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
        TypedExprKind::DynRowPackage { payload, fields } => {
            let payload = refine_pattern_binding_types(*payload, bindings, context);
            typed(
                TypedExprKind::DynRowPackage {
                    payload: Box::new(payload),
                    fields,
                },
                expr.ty,
            )
        }
        TypedExprKind::DynRowField { package, name } => {
            let package = refine_pattern_binding_types(*package, bindings, context);
            let ty = dyn_row_field_type(&package.ty, &name).unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::DynRowField {
                    package: Box::new(package),
                    name,
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
        TypedExprKind::AdtToTuple {
            value,
            tuple_nominal: _,
        } => {
            let value = refine_pattern_binding_types(*value, bindings, context);
            let field_types = adt_tuple_field_types(&value);
            typed(
                TypedExprKind::AdtToTuple {
                    value: Box::new(value),
                    tuple_nominal: tuple_nominal_name(&field_types),
                },
                Type::Tuple(field_types),
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
                FieldAccessKind::AggregateBoundary { .. } => Some(Type::I64),
                FieldAccessKind::TextBoundary { .. } => Some(Type::I64),
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
            ..
        } => {
            let receiver = refine_pattern_binding_types(*receiver, bindings, context);
            let args = args
                .into_iter()
                .map(|arg| refine_pattern_binding_types(arg, bindings, context))
                .collect::<Vec<_>>();
            let builtin = builtin_method_call(&receiver, &name, &args);
            let ty = builtin
                .and_then(|builtin| builtin_method_result_type(builtin, &receiver.ty, &args))
                .unwrap_or_else(|| {
                    field_callable_result_type(&receiver.ty, &name, args.len(), context)
                });
            typed(
                TypedExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    name,
                    args,
                    builtin,
                },
                ty,
            )
        }
        TypedExprKind::Assign { target, value, .. } => {
            let target = refine_pattern_binding_types(*target, bindings, context);
            let value = refine_pattern_binding_types(*value, bindings, context);
            let args = [value.clone()];
            let builtin = match builtin_method_call(&target, "set", &args) {
                Some(BuiltinMethodCall::RefSet) => Some(BuiltinMethodCall::RefSet),
                Some(BuiltinMethodCall::UnsafeRefSet) => Some(BuiltinMethodCall::UnsafeRefSet),
                _ => None,
            };
            let ty = builtin
                .and_then(|builtin| builtin_method_result_type(builtin, &target.ty, &args))
                .unwrap_or(Type::Unknown);
            typed(
                TypedExprKind::Assign {
                    target: Box::new(target),
                    value: Box::new(value),
                    builtin,
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
                IndexAccessKind::AggregateElement { .. }
                | IndexAccessKind::AggregateSlice { .. } => {
                    index_access_kind(&receiver.ty, &index.ty)
                }
                IndexAccessKind::TextByte { .. } | IndexAccessKind::TextSlice { .. } => {
                    index_access_kind(&receiver.ty, &index.ty)
                }
                IndexAccessKind::Operator => IndexAccessKind::Operator,
            };
            let ty = match access {
                IndexAccessKind::AggregateElement { .. }
                | IndexAccessKind::AggregateSlice { .. }
                | IndexAccessKind::TextByte { .. }
                | IndexAccessKind::TextSlice { .. } => index_result_type(&access),
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
        TypedExprKind::DynRowPackage { payload, .. } => {
            collect_typed_resume_inputs(binder, payload, inputs);
        }
        TypedExprKind::DynRowField { package, .. } => {
            collect_typed_resume_inputs(binder, package, inputs);
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_typed_resume_inputs(binder, arg, inputs);
            }
        }
        TypedExprKind::AdtToTuple { value, .. } => {
            collect_typed_resume_inputs(binder, value, inputs);
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
        TypedExprKind::Assign { target, value, .. } => {
            collect_typed_resume_inputs(binder, target, inputs);
            collect_typed_resume_inputs(binder, value, inputs);
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
        TypedExprKind::DynRowPackage { payload, fields } => typed(
            TypedExprKind::DynRowPackage {
                payload: Box::new(rewrite_continuation_callee_input(*payload, binder, input)),
                fields,
            },
            expr.ty,
        ),
        TypedExprKind::DynRowField { package, name } => typed(
            TypedExprKind::DynRowField {
                package: Box::new(rewrite_continuation_callee_input(*package, binder, input)),
                name,
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
        TypedExprKind::AdtToTuple {
            value,
            tuple_nominal,
        } => typed(
            TypedExprKind::AdtToTuple {
                value: Box::new(rewrite_continuation_callee_input(*value, binder, input)),
                tuple_nominal,
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
            ..
        } => {
            let receiver = rewrite_continuation_callee_input(*receiver, binder, input);
            let args = args
                .into_iter()
                .map(|arg| rewrite_continuation_callee_input(arg, binder, input))
                .collect::<Vec<_>>();
            let builtin = builtin_method_call(&receiver, &name, &args);
            typed(
                TypedExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    name,
                    args,
                    builtin,
                },
                expr.ty,
            )
        }
        TypedExprKind::Assign {
            target,
            value,
            builtin,
        } => typed(
            TypedExprKind::Assign {
                target: Box::new(rewrite_continuation_callee_input(*target, binder, input)),
                value: Box::new(rewrite_continuation_callee_input(*value, binder, input)),
                builtin,
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
    let is_range_index = matches!(index, Type::Nominal(name) if name == "Range");
    if index != &Type::I64 && !is_range_index {
        return IndexAccessKind::Operator;
    }
    let Type::Nominal(name) = receiver else {
        return IndexAccessKind::Operator;
    };
    let Some(ParsedTypeHeader::Nominal { base, args }) = parse_type_header(name) else {
        return IndexAccessKind::Operator;
    };
    match (base.as_str(), args.as_slice()) {
        ("Slice", [element]) if is_range_index => IndexAccessKind::AggregateSlice {
            kind: AggregateKind::Slice,
            element: element.to_type(),
        },
        ("Slice", [element]) => IndexAccessKind::AggregateElement {
            kind: AggregateKind::Slice,
            element: element.to_type(),
        },
        ("Array", [element]) if is_range_index => IndexAccessKind::AggregateSlice {
            kind: AggregateKind::Array,
            element: element.to_type(),
        },
        ("Array", [element]) => IndexAccessKind::AggregateElement {
            kind: AggregateKind::Array,
            element: element.to_type(),
        },
        ("Vec", [element]) if is_range_index => IndexAccessKind::AggregateSlice {
            kind: AggregateKind::Vec,
            element: element.to_type(),
        },
        ("Vec", [element]) => IndexAccessKind::AggregateElement {
            kind: AggregateKind::Vec,
            element: element.to_type(),
        },
        ("str", []) if is_range_index => IndexAccessKind::TextSlice {
            kind: TextKind::Str,
        },
        ("str", []) => IndexAccessKind::TextByte {
            kind: TextKind::Str,
        },
        ("String", []) if is_range_index => IndexAccessKind::TextSlice {
            kind: TextKind::String,
        },
        ("String", []) => IndexAccessKind::TextByte {
            kind: TextKind::String,
        },
        ("cstr", []) if is_range_index => IndexAccessKind::TextSlice {
            kind: TextKind::CStr,
        },
        ("cstr", []) => IndexAccessKind::TextByte {
            kind: TextKind::CStr,
        },
        _ => IndexAccessKind::Operator,
    }
}

fn index_result_type(access: &IndexAccessKind) -> Type {
    match access {
        IndexAccessKind::AggregateElement { element, .. } => element.clone(),
        IndexAccessKind::AggregateSlice { element, .. } => {
            Type::Nominal(format!("Slice[{}]", source_type_name_for_type(element)))
        }
        IndexAccessKind::TextByte { .. } => Type::I64,
        IndexAccessKind::TextSlice { kind } => match kind {
            TextKind::Str => Type::Nominal("str".to_string()),
            TextKind::String => Type::Nominal("String".to_string()),
            TextKind::CStr => Type::Nominal("cstr".to_string()),
        },
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
            Type::Func(_, result, _) => current = result,
            _ => return Type::Unknown,
        }
    }
    current.clone()
}

fn dyn_row_method_call_result_type(callee: &Type, arity: usize) -> Type {
    match (callee, arity) {
        (Type::Func(param, result, _), 0) if matches!(param.as_ref(), Type::Nominal(name) if name == "Unit") => {
            result.as_ref().clone()
        }
        _ => call_result_type(callee, arity),
    }
}

fn coerce_call_args(callee: &Type, args: Vec<TypedExpr>, context: &TypeContext) -> Vec<TypedExpr> {
    let expected = call_param_types(callee, args.len());
    args.into_iter()
        .enumerate()
        .map(|(index, arg)| match expected.get(index) {
            Some(expected) => coerce_expected(arg, expected, context),
            None => arg,
        })
        .collect()
}

fn call_param_types(callee: &Type, arity: usize) -> Vec<Type> {
    let mut params = Vec::new();
    let mut current = callee;
    for _ in 0..arity {
        match current {
            Type::Func(param, result, _) => {
                params.push(param.as_ref().clone());
                current = result;
            }
            Type::Continuation { input, .. } => {
                params.push(input.as_ref().clone());
                break;
            }
            _ => break,
        }
    }
    params
}

fn coerce_expected(expr: TypedExpr, expected: &Type, context: &TypeContext) -> TypedExpr {
    match expected {
        Type::DynRow(fields) if !matches!(expr.ty, Type::DynRow(_)) => {
            let Some(adapter_fields) = dyn_row_adapter_fields(&expr.ty, fields, context) else {
                return expr;
            };
            typed(
                TypedExprKind::DynRowPackage {
                    payload: Box::new(expr),
                    fields: adapter_fields,
                },
                expected.clone(),
            )
        }
        _ => expr,
    }
}

fn dyn_row_adapter_fields(
    payload: &Type,
    fields: &[RecordTypeField],
    context: &TypeContext,
) -> Option<Vec<TypedDynRowField>> {
    if matches!(payload, Type::Unknown) {
        return Some(
            fields
                .iter()
                .map(|field| TypedDynRowField {
                    name: field.name.clone(),
                    source: DynRowFieldSource::ContractObligation {
                        ty: field.ty.clone(),
                    },
                })
                .collect(),
        );
    }
    let mut adapters = Vec::new();
    for field in fields {
        let adapter = if let Some(field_ty) = record_field_type(payload, &field.name)
            .or_else(|| context.nominal_field_type(payload, &field.name))
        {
            if field_ty != field.ty && field.ty != Type::Unknown {
                return None;
            }
            DynRowFieldSource::Field
        } else {
            let method = context.receiver_method(payload, &field.name)?;
            let method_ty = bound_method_type(method);
            if method_ty != field.ty && field.ty != Type::Unknown {
                return None;
            }
            DynRowFieldSource::ReceiverMethod {
                symbol: method.symbol.clone(),
                runtime_target: method.runtime_target.clone(),
                param_ty: bound_method_param_type(method),
                result_ty: method.result_ty.clone(),
            }
        };
        adapters.push(TypedDynRowField {
            name: field.name.clone(),
            source: adapter,
        });
    }
    Some(adapters)
}

fn bound_method_type(method: &ReceiverMethodSummary) -> Type {
    Type::Func(
        Box::new(bound_method_param_type(method)),
        Box::new(method.result_ty.clone()),
        SendColor::Obligation,
    )
}

fn bound_method_param_type(method: &ReceiverMethodSummary) -> Type {
    match method.param_tys.as_slice() {
        [] => Type::Nominal("Unit".to_string()),
        [single] => single.clone(),
        params => Type::Tuple(params.to_vec()),
    }
}

fn dyn_row_field_type(receiver: &Type, name: &str) -> Option<Type> {
    let Type::DynRow(fields) = receiver else {
        return None;
    };
    field_type(fields, name)
}

fn field_callable_result_type(
    receiver: &Type,
    name: &str,
    arity: usize,
    context: &TypeContext,
) -> Type {
    field_callable_callee_type(receiver, name, context)
        .map(|(_, ty)| call_result_type(&ty, arity))
        .unwrap_or(Type::Unknown)
}

fn field_callable_callee_type(
    receiver: &Type,
    name: &str,
    context: &TypeContext,
) -> Option<(FieldAccessKind, Type)> {
    let access = field_access_kind(receiver, name);
    let field_ty = match access {
        FieldAccessKind::TuplePositionalRow { index } => tuple_field_type(receiver, index),
        FieldAccessKind::RangeBoundary { .. }
        | FieldAccessKind::AggregateBoundary { .. }
        | FieldAccessKind::TextBoundary { .. } => None,
        FieldAccessKind::RecordOrNominal => {
            record_field_type(receiver, name).or_else(|| context.nominal_field_type(receiver, name))
        }
    }?;
    matches!(field_ty, Type::Func(_, _, _) | Type::Continuation { .. })
        .then_some((access, field_ty))
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
    if let Some(kind) = aggregate_kind_for_type(receiver) {
        if let Some(boundary) = AggregateBoundary::from_source_name(name) {
            return FieldAccessKind::AggregateBoundary { kind, boundary };
        }
    }
    if let Some(kind) = text_kind_for_type(receiver) {
        if let Some(boundary) = TextBoundary::from_source_name(name) {
            return FieldAccessKind::TextBoundary { kind, boundary };
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

fn builtin_method_call(
    receiver: &TypedExpr,
    name: &str,
    args: &[TypedExpr],
) -> Option<BuiltinMethodCall> {
    if matches!(
        (&receiver.kind, name, args),
        (TypedExprKind::Var(type_name), "new", []) if type_name == "Vec"
    ) {
        return Some(BuiltinMethodCall::VecNew);
    }
    if matches!(
        (&receiver.kind, name, args),
        (TypedExprKind::Var(type_name), "new", []) if type_name == "String"
    ) {
        return Some(BuiltinMethodCall::StringNew);
    }
    if matches!(
        (&receiver.kind, name, args),
        (TypedExprKind::Var(type_name), "from", [arg])
            if type_name == "String" && text_kind_for_type(&arg.ty).is_some()
    ) {
        return Some(BuiltinMethodCall::StringFrom);
    }
    if matches!(
        (&receiver.kind, name, args),
        (TypedExprKind::Var(type_name), "new", [_]) if type_name == "Ref"
    ) {
        return Some(BuiltinMethodCall::RefNew);
    }
    if matches!(
        (&receiver.kind, name, args),
        (TypedExprKind::Var(type_name), "new", [_]) if type_name == "UnsafeRef"
    ) {
        return Some(BuiltinMethodCall::UnsafeRefNew);
    }
    if let Some(kind) = text_kind_for_type(&receiver.ty) {
        if matches!((kind, name, args), (TextKind::String, "concat", [arg]) if text_kind_for_type(&arg.ty).is_some())
        {
            return Some(BuiltinMethodCall::StringConcat);
        }
        if matches!((kind, name, args), (TextKind::String, "to_cstr", [])) {
            return Some(BuiltinMethodCall::StringToCStr);
        }
        if matches!((kind, name, args), (TextKind::String, "as_str", [])) {
            return Some(BuiltinMethodCall::StringAsStr);
        }
        if matches!((kind, name, args), (TextKind::String, "push_rune", [_])) {
            return Some(BuiltinMethodCall::StringPushRune);
        }
        if matches!((name, args), ("len" | "bytes_len", [])) {
            return Some(BuiltinMethodCall::TextLen { kind });
        }
        if matches!((name, args), ("rune_len", [])) {
            return Some(BuiltinMethodCall::TextRuneLen { kind });
        }
        if matches!((name, args), ("char_at", [_])) {
            return Some(BuiltinMethodCall::TextCharAt { kind });
        }
    }
    if ref_element_type(&receiver.ty).is_some() {
        return match (name, args) {
            ("get", []) => Some(BuiltinMethodCall::RefGet),
            ("set", [_]) => Some(BuiltinMethodCall::RefSet),
            _ => None,
        };
    }
    if unsafe_ref_element_type(&receiver.ty).is_some() {
        return match (name, args) {
            ("get", []) => Some(BuiltinMethodCall::UnsafeRefGet),
            ("set", [_]) => Some(BuiltinMethodCall::UnsafeRefSet),
            _ => None,
        };
    }
    if aggregate_kind_for_type(&receiver.ty) != Some(AggregateKind::Vec) {
        return None;
    }
    match (name, args) {
        ("push", [_]) => Some(BuiltinMethodCall::VecPush),
        ("freeze", []) => Some(BuiltinMethodCall::VecFreeze),
        _ => None,
    }
}

fn builtin_method_result_type(
    builtin: BuiltinMethodCall,
    receiver: &Type,
    args: &[TypedExpr],
) -> Option<Type> {
    match builtin {
        BuiltinMethodCall::VecNew => Some(Type::Nominal("Vec[Unknown]".to_string())),
        BuiltinMethodCall::VecPush => Some(vec_push_result_type(receiver, args)),
        BuiltinMethodCall::VecFreeze => vec_element_type(receiver).map(|element| {
            Type::Nominal(format!("Slice[{}]", source_type_name_for_type(&element)))
        }),
        BuiltinMethodCall::StringNew => Some(Type::Nominal("String".to_string())),
        BuiltinMethodCall::StringFrom => Some(Type::Nominal("String".to_string())),
        BuiltinMethodCall::StringConcat => Some(Type::Nominal("String".to_string())),
        BuiltinMethodCall::StringAsStr => Some(Type::Nominal("str".to_string())),
        BuiltinMethodCall::StringToCStr => Some(Type::Nominal("cstr".to_string())),
        BuiltinMethodCall::StringPushRune => Some(receiver.clone()),
        BuiltinMethodCall::TextLen { .. } => Some(Type::I64),
        BuiltinMethodCall::TextRuneLen { .. } => Some(Type::I64),
        BuiltinMethodCall::TextCharAt { .. } => Some(Type::Rune),
        BuiltinMethodCall::RefNew => args
            .first()
            .map(|arg| Type::Nominal(format!("Ref[{}]", source_type_name_for_type(&arg.ty)))),
        BuiltinMethodCall::RefGet => ref_element_type(receiver),
        BuiltinMethodCall::RefSet => Some(receiver.clone()),
        BuiltinMethodCall::UnsafeRefNew => args
            .first()
            .map(|arg| Type::Nominal(format!("UnsafeRef[{}]", source_type_name_for_type(&arg.ty)))),
        BuiltinMethodCall::UnsafeRefGet => unsafe_ref_element_type(receiver),
        BuiltinMethodCall::UnsafeRefSet => Some(receiver.clone()),
    }
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

impl AggregateKind {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::Slice => "slice",
            Self::Array => "array",
            Self::Vec => "vec",
        }
    }

    pub fn runtime_prefix(self) -> &'static str {
        match self {
            Self::Slice => "slice",
            Self::Array => "array",
            Self::Vec => "vec",
        }
    }
}

fn aggregate_kind_for_type(receiver: &Type) -> Option<AggregateKind> {
    match nominal_base_name_for_type(receiver)? {
        "Slice" => Some(AggregateKind::Slice),
        "Array" => Some(AggregateKind::Array),
        "Vec" => Some(AggregateKind::Vec),
        _ => None,
    }
}

fn vec_element_type(receiver: &Type) -> Option<Type> {
    let Type::Nominal(name) = receiver else {
        return None;
    };
    let ParsedTypeHeader::Nominal { base, args } = parse_type_header(name)? else {
        return None;
    };
    match (base.as_str(), args.as_slice()) {
        ("Vec", [element]) => Some(element.to_type()),
        _ => None,
    }
}

fn vec_push_result_type(receiver: &Type, args: &[TypedExpr]) -> Type {
    let Some(item) = args.first() else {
        return receiver.clone();
    };
    let Some(current) = vec_element_type(receiver) else {
        return receiver.clone();
    };
    let element = if current == item.ty {
        current
    } else if current == Type::Unknown {
        item.ty.clone()
    } else {
        Type::Unknown
    };
    Type::Nominal(render_source_type_application(
        "Vec",
        &[source_type_name_for_type(&element)],
    ))
}

fn ref_element_type(receiver: &Type) -> Option<Type> {
    let Type::Nominal(name) = receiver else {
        return None;
    };
    let ParsedTypeHeader::Nominal { base, args } = parse_type_header(name)? else {
        return None;
    };
    match (base.as_str(), args.as_slice()) {
        ("Ref", [element]) => Some(element.to_type()),
        _ => None,
    }
}

fn unsafe_ref_element_type(receiver: &Type) -> Option<Type> {
    let Type::Nominal(name) = receiver else {
        return None;
    };
    let ParsedTypeHeader::Nominal { base, args } = parse_type_header(name)? else {
        return None;
    };
    match (base.as_str(), args.as_slice()) {
        ("UnsafeRef", [element]) => Some(element.to_type()),
        _ => None,
    }
}

impl TextKind {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::Str => "str",
            Self::String => "String",
            Self::CStr => "cstr",
        }
    }

    pub fn runtime_prefix(self) -> &'static str {
        match self {
            Self::Str => "str",
            Self::String => "string",
            Self::CStr => "cstr",
        }
    }
}

fn text_kind_for_type(receiver: &Type) -> Option<TextKind> {
    match nominal_base_name_for_type(receiver)? {
        "str" => Some(TextKind::Str),
        "String" => Some(TextKind::String),
        "cstr" => Some(TextKind::CStr),
        _ => None,
    }
}

impl AggregateBoundary {
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

impl TextBoundary {
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

pub(crate) fn nominal_base_name_for_type(ty: &Type) -> Option<&str> {
    let Type::Nominal(name) = ty else {
        return None;
    };
    Some(nominal_base_name(name))
}

pub(crate) fn nominal_type_args_for_type(ty: &Type) -> Option<Vec<Type>> {
    let Type::Nominal(name) = ty else {
        return None;
    };
    let ParsedTypeHeader::Nominal { args, .. } = parse_type_header(name)? else {
        return None;
    };
    Some(args.into_iter().map(|arg| arg.to_type()).collect())
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
    Unknown,
    ScalarI64,
    ScalarRune,
    ScalarBool,
    Nominal {
        base: String,
        args: Vec<ParsedTypeHeader>,
    },
    Tuple {
        args: Vec<ParsedTypeHeader>,
    },
    DynRow {
        fields: Vec<ParsedTypeFieldHeader>,
    },
    Continuation {
        multi: bool,
        input: Box<ParsedTypeHeader>,
        answer: Box<ParsedTypeHeader>,
    },
    Callable {
        param: Box<ParsedTypeHeader>,
        result: Box<ParsedTypeHeader>,
        send: SendColor,
    },
}

impl ParsedTypeHeader {
    fn to_type(&self) -> Type {
        match self {
            ParsedTypeHeader::Unknown => Type::Unknown,
            ParsedTypeHeader::ScalarI64 => Type::I64,
            ParsedTypeHeader::ScalarRune => Type::Rune,
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
            ParsedTypeHeader::DynRow { fields } => Type::DynRow(canonical_fields(
                fields
                    .iter()
                    .map(|field| RecordTypeField {
                        name: field.name.clone(),
                        ty: field.ty.to_type(),
                    })
                    .collect(),
            )),
            ParsedTypeHeader::Continuation {
                multi,
                input,
                answer,
            } => Type::Continuation {
                multi: *multi,
                input: Box::new(input.to_type()),
                answer: Box::new(answer.to_type()),
            },
            ParsedTypeHeader::Callable {
                param,
                result,
                send,
            } => Type::Func(Box::new(param.to_type()), Box::new(result.to_type()), *send),
        }
    }
}

fn parse_type_header(name: &str) -> Option<ParsedTypeHeader> {
    let name = name.trim();
    match name {
        "Unknown" | "unknown" => return Some(ParsedTypeHeader::Unknown),
        "I64" | "i64" => return Some(ParsedTypeHeader::ScalarI64),
        "Rune" | "rune" => return Some(ParsedTypeHeader::ScalarRune),
        "Bool" | "bool" => return Some(ParsedTypeHeader::ScalarBool),
        "" => return None,
        _ => {}
    }

    if let Some(callable) = parse_callable_type_header(name) {
        return Some(callable);
    }

    if let Some(grouped) = parse_grouped_type_header(name) {
        return Some(grouped);
    }

    if let Some(fields) = parse_dyn_row_type_header(name) {
        return Some(ParsedTypeHeader::DynRow { fields });
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

fn parse_dyn_row_type_header(name: &str) -> Option<Vec<ParsedTypeFieldHeader>> {
    let body = name.strip_prefix("dyn")?.trim_start();
    let body = body.strip_prefix('{')?.strip_suffix('}')?.trim();
    if body.is_empty() {
        return Some(Vec::new());
    }
    split_type_header_args(body)?
        .into_iter()
        .map(|field| {
            let colon = top_level_colon(&field)?;
            let name = field[..colon].trim();
            let ty = field[colon + 1..].trim();
            if name.is_empty() || ty.is_empty() {
                return None;
            }
            Some(ParsedTypeFieldHeader {
                name: name.to_string(),
                ty: parse_type_header(ty)?,
            })
        })
        .collect()
}

fn top_level_colon(text: &str) -> Option<usize> {
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' => square_depth = square_depth.checked_add(1)?,
            ']' => square_depth = square_depth.checked_sub(1)?,
            '(' => paren_depth = paren_depth.checked_add(1)?,
            ')' => paren_depth = paren_depth.checked_sub(1)?,
            '{' => brace_depth = brace_depth.checked_add(1)?,
            '}' => brace_depth = brace_depth.checked_sub(1)?,
            ':' if square_depth == 0 && paren_depth == 0 && brace_depth == 0 => {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn parse_callable_type_header(name: &str) -> Option<ParsedTypeHeader> {
    let (name, send) = strip_send_callable_suffix(name);
    let arrow = top_level_arrow(name)?;
    let param = parenthesized_type(name[..arrow].trim())?;
    let result = name[arrow + 2..].trim();
    if result.is_empty() {
        return None;
    }
    Some(ParsedTypeHeader::Callable {
        param: Box::new(parse_type_header(param)?),
        result: Box::new(parse_type_header(result)?),
        send,
    })
}

fn parse_grouped_type_header(name: &str) -> Option<ParsedTypeHeader> {
    let (name, send) = strip_send_callable_suffix(name);
    let inner = parenthesized_type(name)?;
    let mut parsed = parse_type_header(inner)?;
    if send == SendColor::Send {
        let ParsedTypeHeader::Callable {
            send: callable_send,
            ..
        } = &mut parsed
        else {
            return None;
        };
        *callable_send = SendColor::Send;
    }
    Some(parsed)
}

fn strip_send_callable_suffix(name: &str) -> (&str, SendColor) {
    let Some(prefix) = name.strip_suffix(" send") else {
        return (name, SendColor::Obligation);
    };
    (prefix.trim_end(), SendColor::Send)
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
    let mut brace_depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in args.char_indices() {
        match ch {
            '[' => square_depth = square_depth.checked_add(1)?,
            ']' => square_depth = square_depth.checked_sub(1)?,
            '(' => paren_depth = paren_depth.checked_add(1)?,
            ')' => paren_depth = paren_depth.checked_sub(1)?,
            '{' => brace_depth = brace_depth.checked_add(1)?,
            '}' => brace_depth = brace_depth.checked_sub(1)?,
            ',' if square_depth == 0 && paren_depth == 0 && brace_depth == 0 => {
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
    if square_depth != 0 || paren_depth != 0 || brace_depth != 0 {
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
        ParsedTypeHeader::Unknown => "Unknown".to_string(),
        ParsedTypeHeader::ScalarI64 => "i64".to_string(),
        ParsedTypeHeader::ScalarRune => "rune".to_string(),
        ParsedTypeHeader::ScalarBool => "bool".to_string(),
        ParsedTypeHeader::Nominal { base, args } => {
            if args.is_empty() {
                base.clone()
            } else {
                render_nominal_type_header(base, args)
            }
        }
        ParsedTypeHeader::Tuple { args } => render_nominal_type_header("Tuple", args),
        ParsedTypeHeader::DynRow { fields } => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, render_type_header(&field.ty)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("dyn {{{fields}}}")
        }
        ParsedTypeHeader::Continuation {
            multi,
            input,
            answer,
        } => {
            let base = if *multi { "ContN" } else { "Cont1" };
            render_nominal_type_header(base, &[input.as_ref().clone(), answer.as_ref().clone()])
        }
        ParsedTypeHeader::Callable {
            param,
            result,
            send,
        } => {
            let mut rendered = format!(
                "({}) -> {}",
                render_type_header(param),
                render_type_header(result)
            );
            if *send == SendColor::Send {
                rendered = format!("({rendered}) send");
            }
            rendered
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
        Type::DynRow(fields) => Type::DynRow(
            fields
                .iter()
                .map(|field| RecordTypeField {
                    name: field.name.clone(),
                    ty: substitute_type_params(&field.ty, substitutions),
                })
                .collect(),
        ),
        Type::Func(param, result, send) => Type::Func(
            Box::new(substitute_type_params(param, substitutions)),
            Box::new(substitute_type_params(result, substitutions)),
            *send,
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
        Type::Rune => Type::Rune,
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
        Type::DynRow(payload_fields) => {
            let Type::DynRow(actual_fields) = actual else {
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
        Type::Func(payload_param, payload_result, _) => {
            let Type::Func(actual_param, actual_result, _) = actual else {
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
        Type::Adt { .. }
        | Type::Unknown
        | Type::I64
        | Type::Rune
        | Type::Bool
        | Type::Nominal(_) => Some(()),
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

pub(crate) fn source_type_name_for_type(ty: &Type) -> String {
    match ty {
        Type::I64 => "i64".to_string(),
        Type::Rune => "rune".to_string(),
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
        Type::Func(param, result, send) => {
            let rendered = format!(
                "({}) -> {}",
                source_type_name_for_type(param),
                source_type_name_for_type(result)
            );
            if *send == SendColor::Send {
                format!("({rendered}) send")
            } else {
                rendered
            }
        }
        Type::Record(_) | Type::Adt { .. } | Type::Unknown => type_stable_name(ty),
        Type::DynRow(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, source_type_name_for_type(&field.ty)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("dyn {{{fields}}}")
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
        Type::Rune => "Rune".to_string(),
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
        Type::DynRow(fields) => {
            let mut name = "DynRow".to_string();
            for field in fields {
                name.push('_');
                name.push_str(&sanitize_type_name(&field.name));
                name.push('_');
                name.push_str(&type_stable_name(&field.ty));
            }
            name
        }
        Type::Nominal(name) => format!("Nominal{}", sanitize_type_name(name)),
        Type::Func(arg, ret, send) => format!(
            "Fn_{}_{}_{}",
            type_stable_name(arg),
            type_stable_name(ret),
            send_color_stable_name(*send)
        ),
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

fn send_color_stable_name(send: SendColor) -> &'static str {
    match send {
        SendColor::Send => "send",
        SendColor::NotSend => "notsend",
        SendColor::Obligation => "obligation",
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
