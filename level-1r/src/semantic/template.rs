use crate::alpha::{AlphaExpr, AlphaExprKind};
use crate::ast::{render_source_expr, Expr, ParamDecl};
use crate::resolve::{
    OperatorObligation, OperatorSurface, ResolveFacts, ResolvedCall, ResolvedName,
};
use crate::typed::{source_type_name_to_type, RecordTypeField, SendColor, Type, UsageColor};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateFacts {
    pub explicit_params: Vec<TemplateParam>,
    pub explicit_instantiations: Vec<TemplateInstantiation>,
    pub row_shapes: Vec<RowShape>,
    pub obligations: Vec<TemplateObligation>,
    pub dyn_contracts: Vec<DynRowContract>,
    pub diagnostics: Vec<TemplateDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplateParam {
    pub name: String,
    pub source: TemplateParamSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TemplateParamSource {
    ExplicitHeader,
    SyntheticAutoGeneric,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TemplateDiagnostic {
    ExplicitAutoGenericConflict {
        param: String,
    },
    ConflictingExplicitInstantiation {
        callee: String,
        previous_type_args: Vec<String>,
        type_args: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplateInstantiation {
    pub callee: String,
    pub type_args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowField {
    pub name: String,
    pub ty: ShapeType,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShapeType {
    Unknown,
    Named(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowShape {
    pub openness: RowOpenness,
    pub fields: Vec<RowField>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RowOpenness {
    Open,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TemplateObligation {
    Field {
        shape: RowShape,
        field: String,
    },
    Method {
        receiver: Option<String>,
        name: String,
        resolved: Option<String>,
    },
    Function {
        name: String,
        resolved: String,
    },
    Static {
        name: String,
        resolved: String,
    },
    Constructor {
        data: String,
        ctor: String,
        resolved: String,
        arity: usize,
    },
    Operator {
        op: OperatorSurface,
        protocol: String,
        receiver: Option<String>,
    },
    DynAdapter {
        contract: DynRowContract,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DynRowContract {
    pub shape: RowShape,
    pub payload_usage: UsageColor,
    pub send: SendColor,
    pub adapter: DynAdapterKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DynAdapterKind {
    StaticToDynPackage,
    DynAdapterAccess,
}

pub fn analyze_template(expr: &AlphaExpr, resolve: &ResolveFacts) -> TemplateFacts {
    let mut facts = TemplateFacts::default();
    collect_expr_obligations(expr, &mut facts);
    collect_resolve_obligations(resolve, &mut facts);
    facts
}

pub fn analyze_template_with_source(
    source: &Expr,
    explicit_params: &[String],
    source_params: &[ParamDecl],
    return_type: &Option<String>,
    expr: &AlphaExpr,
    resolve: &ResolveFacts,
) -> TemplateFacts {
    let mut facts = analyze_template(expr, resolve);
    facts.explicit_params = explicit_params
        .iter()
        .map(|name| TemplateParam {
            name: name.clone(),
            source: TemplateParamSource::ExplicitHeader,
        })
        .collect();
    collect_auto_template_params(source, source_params, return_type, &mut facts);
    collect_source_dyn_row_contracts(source_params, return_type, &mut facts);
    collect_source_instantiations(source, &mut facts);
    facts
}

fn collect_source_dyn_row_contracts(
    source_params: &[ParamDecl],
    return_type: &Option<String>,
    facts: &mut TemplateFacts,
) {
    for ty in source_params
        .iter()
        .filter_map(|param| param.ty.as_deref())
        .chain(return_type.as_deref())
    {
        collect_dyn_row_contract_from_type(&source_type_name_to_type(ty), facts);
    }
}

fn collect_dyn_row_contract_from_type(ty: &Type, facts: &mut TemplateFacts) {
    match ty {
        Type::DynRow(fields) => {
            let contract = dyn_row_contract_from_record_fields(fields);
            push_dyn_contract_once(facts, contract.clone());
            push_obligation_once(facts, TemplateObligation::DynAdapter { contract });
        }
        Type::Func(param, result) => {
            collect_dyn_row_contract_from_type(param, facts);
            collect_dyn_row_contract_from_type(result, facts);
        }
        Type::Tuple(fields) => {
            for field in fields {
                collect_dyn_row_contract_from_type(field, facts);
            }
        }
        Type::Record(fields) => {
            for field in fields {
                collect_dyn_row_contract_from_type(&field.ty, facts);
            }
        }
        Type::Continuation { input, answer, .. } => {
            collect_dyn_row_contract_from_type(input, facts);
            collect_dyn_row_contract_from_type(answer, facts);
        }
        Type::Unknown
        | Type::I64
        | Type::Rune
        | Type::Bool
        | Type::Adt { .. }
        | Type::Nominal(_) => {}
    }
}

fn dyn_row_contract_from_record_fields(fields: &[RecordTypeField]) -> DynRowContract {
    dyn_row_contract(
        fields
            .iter()
            .map(|field| (field.name.as_str(), shape_type_for_type(&field.ty)))
            .collect(),
    )
}

fn shape_type_for_type(ty: &Type) -> ShapeType {
    match ty {
        Type::Unknown => ShapeType::Unknown,
        _ => ShapeType::Named(crate::typed::source_type_name_for_type(ty)),
    }
}

fn push_dyn_contract_once(facts: &mut TemplateFacts, contract: DynRowContract) {
    if !facts.dyn_contracts.contains(&contract) {
        facts.dyn_contracts.push(contract);
    }
}

fn push_obligation_once(facts: &mut TemplateFacts, obligation: TemplateObligation) {
    if !facts.obligations.contains(&obligation) {
        facts.obligations.push(obligation);
    }
}

fn collect_auto_template_params(
    source: &Expr,
    source_params: &[ParamDecl],
    return_type: &Option<String>,
    facts: &mut TemplateFacts,
) {
    for param in source_params.iter().filter(|param| param.ty.is_none()) {
        for binding in param.pattern.bindings() {
            push_template_param_once(
                facts,
                TemplateParam {
                    name: format!("T_{binding}"),
                    source: TemplateParamSource::SyntheticAutoGeneric,
                },
            );
        }
    }

    if return_type.is_none() && source_needs_auto_return_param(source, source_params) {
        push_template_param_once(
            facts,
            TemplateParam {
                name: "T_return".to_string(),
                source: TemplateParamSource::SyntheticAutoGeneric,
            },
        );
    }
}

fn source_needs_auto_return_param(source: &Expr, source_params: &[ParamDecl]) -> bool {
    match source {
        Expr::Lit(_) => false,
        Expr::Var(name) => !source_params
            .iter()
            .any(|param| param.ty.is_none() && param.pattern.bindings().contains(name)),
        Expr::Tuple(fields) => fields
            .iter()
            .any(|field| source_needs_auto_return_param(field, source_params)),
        Expr::SliceLiteral(items) => items
            .iter()
            .any(|item| source_needs_auto_return_param(item, source_params)),
        Expr::Record(fields) => fields
            .iter()
            .any(|field| source_needs_auto_return_param(&field.value, source_params)),
        Expr::RecordUpdate { .. }
        | Expr::AdtCtor { .. }
        | Expr::Field { .. }
        | Expr::MethodCall { .. }
        | Expr::Assign { .. }
        | Expr::Index { .. }
        | Expr::Range { .. }
        | Expr::Binary { .. }
        | Expr::If { .. }
        | Expr::IfLet { .. }
        | Expr::Match { .. }
        | Expr::Nominal { .. }
        | Expr::Reset { .. }
        | Expr::Shift { .. }
        | Expr::Lambda { .. }
        | Expr::Call { .. }
        | Expr::Instantiate { .. } => true,
    }
}

fn push_template_param_once(facts: &mut TemplateFacts, param: TemplateParam) {
    if facts.explicit_params.iter().any(|existing| {
        existing.name == param.name
            && existing.source != param.source
            && !facts
                .diagnostics
                .contains(&TemplateDiagnostic::ExplicitAutoGenericConflict {
                    param: param.name.clone(),
                })
    }) {
        facts
            .diagnostics
            .push(TemplateDiagnostic::ExplicitAutoGenericConflict {
                param: param.name.clone(),
            });
    }
    if !facts
        .explicit_params
        .iter()
        .any(|existing| existing == &param)
    {
        facts.explicit_params.push(param);
    }
}

fn collect_source_instantiations(expr: &Expr, facts: &mut TemplateFacts) {
    match expr {
        Expr::Var(_) | Expr::Lit(_) => {}
        Expr::Lambda { body, .. } => collect_source_instantiations(body, facts),
        Expr::Call { callee, args } => {
            collect_source_instantiations(callee, facts);
            for arg in args {
                collect_source_instantiations(arg, facts);
            }
        }
        Expr::Instantiate { callee, type_args } => {
            let instantiation = TemplateInstantiation {
                callee: source_callee_name(callee),
                type_args: type_args.clone(),
            };
            if let Some(previous) = facts.explicit_instantiations.iter().find(|previous| {
                previous.callee == instantiation.callee
                    && previous.type_args != instantiation.type_args
            }) {
                let diagnostic = TemplateDiagnostic::ConflictingExplicitInstantiation {
                    callee: instantiation.callee.clone(),
                    previous_type_args: previous.type_args.clone(),
                    type_args: instantiation.type_args.clone(),
                };
                if !facts.diagnostics.contains(&diagnostic) {
                    facts.diagnostics.push(diagnostic);
                }
            }
            facts.explicit_instantiations.push(instantiation);
            collect_source_instantiations(callee, facts);
        }
        Expr::Tuple(fields) => {
            for field in fields {
                collect_source_instantiations(field, facts);
            }
        }
        Expr::SliceLiteral(items) => {
            for item in items {
                collect_source_instantiations(item, facts);
            }
        }
        Expr::Record(fields) => {
            for field in fields {
                collect_source_instantiations(&field.value, facts);
            }
        }
        Expr::RecordUpdate { base, fields } => {
            collect_source_instantiations(base, facts);
            for field in fields {
                collect_source_instantiations(&field.value, facts);
            }
        }
        Expr::AdtCtor { args, .. } => {
            for arg in args {
                collect_source_instantiations(arg, facts);
            }
        }
        Expr::Field { receiver, .. } => collect_source_instantiations(receiver, facts),
        Expr::MethodCall { receiver, args, .. } => {
            collect_source_instantiations(receiver, facts);
            for arg in args {
                collect_source_instantiations(arg, facts);
            }
        }
        Expr::Assign { target, value } => {
            collect_source_instantiations(target, facts);
            collect_source_instantiations(value, facts);
        }
        Expr::Index { receiver, index } => {
            collect_source_instantiations(receiver, facts);
            collect_source_instantiations(index, facts);
        }
        Expr::Range { start, end } => {
            collect_source_instantiations(start, facts);
            collect_source_instantiations(end, facts);
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_source_instantiations(lhs, facts);
            collect_source_instantiations(rhs, facts);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_source_instantiations(cond, facts);
            collect_source_instantiations(then_branch, facts);
            collect_source_instantiations(else_branch, facts);
        }
        Expr::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_source_instantiations(scrutinee, facts);
            collect_source_instantiations(then_branch, facts);
            collect_source_instantiations(else_branch, facts);
        }
        Expr::Match { scrutinee, arms } => {
            collect_source_instantiations(scrutinee, facts);
            for arm in arms {
                collect_source_instantiations(&arm.body, facts);
            }
        }
        Expr::Nominal { expr, .. } => collect_source_instantiations(expr, facts),
        Expr::Reset { body, .. } | Expr::Shift { body, .. } => {
            collect_source_instantiations(body, facts);
        }
    }
}

fn source_callee_name(expr: &Expr) -> String {
    match expr {
        Expr::Var(name) => name.clone(),
        Expr::Field { receiver, name, .. } | Expr::MethodCall { receiver, name, .. } => {
            format!("{}.{}", source_callee_name(receiver), name)
        }
        Expr::Assign { target, .. } => {
            format!("{}.:=", source_callee_name(target))
        }
        Expr::Instantiate { callee, type_args } => {
            format!("{}[{}]", source_callee_name(callee), type_args.join(","))
        }
        other => render_source_expr(other),
    }
}

pub fn canonical_open_row(fields: Vec<(&str, ShapeType)>) -> RowShape {
    canonical_row(RowOpenness::Open, fields)
}

pub fn canonical_closed_row(fields: Vec<(&str, ShapeType)>) -> RowShape {
    canonical_row(RowOpenness::Closed, fields)
}

pub fn dyn_row_contract(fields: Vec<(&str, ShapeType)>) -> DynRowContract {
    DynRowContract {
        shape: canonical_open_row(fields),
        payload_usage: UsageColor::Many,
        send: SendColor::Obligation,
        adapter: DynAdapterKind::StaticToDynPackage,
    }
}

pub fn row_shape_key(shape: &RowShape) -> String {
    let mut text = match shape.openness {
        RowOpenness::Open => "open".to_string(),
        RowOpenness::Closed => "closed".to_string(),
    };
    for field in &shape.fields {
        text.push('|');
        text.push_str(&field.name);
        text.push(':');
        text.push_str(shape_type_key(&field.ty));
    }
    text
}

pub fn dyn_row_contract_key(contract: &DynRowContract) -> String {
    format!(
        "{};usage={};send={};adapter={}",
        row_shape_key(&contract.shape),
        usage_key(contract.payload_usage),
        send_key(contract.send),
        dyn_adapter_key(contract.adapter),
    )
}

fn shape_type_key(ty: &ShapeType) -> &str {
    match ty {
        ShapeType::Unknown => "?",
        ShapeType::Named(name) => name,
    }
}

fn usage_key(color: UsageColor) -> &'static str {
    match color {
        UsageColor::One => "1",
        UsageColor::Many => "N",
        UsageColor::Obligation => "obligation",
    }
}

fn send_key(color: SendColor) -> &'static str {
    match color {
        SendColor::Send => "send",
        SendColor::NotSend => "!send",
        SendColor::Obligation => "obligation",
    }
}

fn dyn_adapter_key(adapter: DynAdapterKind) -> &'static str {
    match adapter {
        DynAdapterKind::StaticToDynPackage => "static-to-dyn-package",
        DynAdapterKind::DynAdapterAccess => "dyn-adapter-access",
    }
}

fn canonical_row(openness: RowOpenness, fields: Vec<(&str, ShapeType)>) -> RowShape {
    let mut fields = fields
        .into_iter()
        .map(|(name, ty)| RowField {
            name: name.to_string(),
            ty,
        })
        .collect::<Vec<_>>();
    fields.sort();
    fields.dedup_by(|a, b| a.name == b.name);
    RowShape { openness, fields }
}

fn collect_expr_obligations(expr: &AlphaExpr, facts: &mut TemplateFacts) {
    match &expr.kind {
        AlphaExprKind::Var(_) | AlphaExprKind::Lit(_) => {}
        AlphaExprKind::Lambda { body, .. } => collect_expr_obligations(body, facts),
        AlphaExprKind::Call { callee, args } => {
            collect_expr_obligations(callee, facts);
            for arg in args {
                collect_expr_obligations(arg, facts);
            }
        }
        AlphaExprKind::Tuple(fields) => {
            for field in fields {
                collect_expr_obligations(field, facts);
            }
        }
        AlphaExprKind::SliceLiteral(items) => {
            for item in items {
                collect_expr_obligations(item, facts);
            }
        }
        AlphaExprKind::Record(fields) => {
            let shape = canonical_closed_row(
                fields
                    .iter()
                    .map(|field| (field.name.as_str(), ShapeType::Unknown))
                    .collect(),
            );
            facts.row_shapes.push(shape);
            for field in fields {
                collect_expr_obligations(&field.value, facts);
            }
        }
        AlphaExprKind::RecordUpdate { base, fields } => {
            let shape = canonical_open_row(
                fields
                    .iter()
                    .map(|field| (field.name.as_str(), ShapeType::Unknown))
                    .collect(),
            );
            facts.row_shapes.push(shape);
            collect_expr_obligations(base, facts);
            for field in fields {
                collect_expr_obligations(&field.value, facts);
            }
        }
        AlphaExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_expr_obligations(arg, facts);
            }
        }
        AlphaExprKind::Field { receiver, name, .. } => {
            let shape = canonical_open_row(vec![(name.as_str(), ShapeType::Unknown)]);
            facts.row_shapes.push(shape.clone());
            facts.obligations.push(TemplateObligation::Field {
                shape,
                field: name.clone(),
            });
            collect_expr_obligations(receiver, facts);
        }
        AlphaExprKind::MethodCall { receiver, args, .. } => {
            collect_expr_obligations(receiver, facts);
            for arg in args {
                collect_expr_obligations(arg, facts);
            }
        }
        AlphaExprKind::Assign { target, value } => {
            collect_expr_obligations(target, facts);
            collect_expr_obligations(value, facts);
        }
        AlphaExprKind::Index { receiver, index } => {
            collect_expr_obligations(receiver, facts);
            collect_expr_obligations(index, facts);
        }
        AlphaExprKind::Range { start, end } => {
            collect_expr_obligations(start, facts);
            collect_expr_obligations(end, facts);
        }
        AlphaExprKind::Binary { lhs, rhs, .. } => {
            collect_expr_obligations(lhs, facts);
            collect_expr_obligations(rhs, facts);
        }
        AlphaExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_expr_obligations(cond, facts);
            collect_expr_obligations(then_branch, facts);
            collect_expr_obligations(else_branch, facts);
        }
        AlphaExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_obligations(scrutinee, facts);
            collect_expr_obligations(then_branch, facts);
            collect_expr_obligations(else_branch, facts);
        }
        AlphaExprKind::Match { scrutinee, arms } => {
            collect_expr_obligations(scrutinee, facts);
            for arm in arms {
                collect_expr_obligations(&arm.body, facts);
            }
        }
        AlphaExprKind::Nominal { expr, .. } => collect_expr_obligations(expr, facts),
        AlphaExprKind::Reset { body, .. } | AlphaExprKind::Shift { body, .. } => {
            collect_expr_obligations(body, facts);
        }
    }
}

fn collect_resolve_obligations(resolve: &ResolveFacts, facts: &mut TemplateFacts) {
    for name in &resolve.resolved_names {
        match name {
            ResolvedName::Function { name, symbol } => {
                facts.obligations.push(TemplateObligation::Function {
                    name: name.clone(),
                    resolved: symbol.clone(),
                });
            }
            ResolvedName::Static { name, symbol } => {
                facts.obligations.push(TemplateObligation::Static {
                    name: name.clone(),
                    resolved: symbol.clone(),
                });
            }
            ResolvedName::Constructor {
                data,
                ctor,
                symbol,
                arity,
            } => facts.obligations.push(TemplateObligation::Constructor {
                data: data.clone(),
                ctor: ctor.clone(),
                resolved: symbol.clone(),
                arity: *arity,
            }),
        }
    }

    for call in &resolve.resolved_calls {
        match call {
            ResolvedCall::FieldCallable { field } => {
                let shape = canonical_open_row(vec![(field.as_str(), ShapeType::Unknown)]);
                facts.row_shapes.push(shape.clone());
                facts.obligations.push(TemplateObligation::Field {
                    shape,
                    field: field.clone(),
                });
            }
            ResolvedCall::ReceiverMethod {
                receiver,
                name,
                symbol,
            } => facts.obligations.push(TemplateObligation::Method {
                receiver: Some(receiver.clone()),
                name: name.clone(),
                resolved: Some(symbol.clone()),
            }),
            ResolvedCall::QualifiedCallee { path, symbol } => {
                facts.obligations.push(TemplateObligation::Method {
                    receiver: None,
                    name: path.clone(),
                    resolved: Some(symbol.clone()),
                });
            }
        }
    }

    for OperatorObligation {
        op,
        protocol,
        receiver,
    } in &resolve.operator_obligations
    {
        facts.obligations.push(TemplateObligation::Operator {
            op: op.clone(),
            protocol: protocol.clone(),
            receiver: receiver.clone(),
        });
    }
}
