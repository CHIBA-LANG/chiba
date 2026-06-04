use std::fmt::Write;

use crate::alpha::AlphaFacts;
use crate::ast::{render_source_binary_op, render_source_expr, Expr};
use crate::backend::{BackendArtifact, BackendCacheKey, BackendLinkedBundle};
use crate::closure::ClosureFacts;
use crate::closure_core_usage::ClosureCoreUsageFacts;
use crate::closure_simplify::ClosureSimplificationFacts;
use crate::control::{ContinuationKind, ControlFacts};
use crate::core::{CoreProgram, CoreValidation};
use crate::cps::CpsProgram;
use crate::cps_usage::{ContinuationSimplificationFacts, CpsUsageFacts};
use crate::lambda_lift::LambdaLiftFacts;
use crate::monomorphize::MonomorphizationPlan;
use crate::nanopass::PassReport;
use crate::pattern::PatternFacts;
use crate::resolve::{
    OperatorSurface, ResolveDiagnostic, ResolveFacts, ResolvedCall, ResolvedName,
};
use crate::specialize::{AbiMode, DischargedObligation, SpecializationFacts, SpecializationKey};
use crate::std_audit::StdAuditReport;
use crate::template::{
    DynAdapterKind, DynRowContract, RowOpenness, RowShape, ShapeType, TemplateDiagnostic,
    TemplateFacts, TemplateObligation, TemplateParamSource,
};
use crate::template_audit::TemplateAuditReport;
use crate::typed::TypedExpr;
use crate::usage::UsageFacts;
use crate::usage_audit::UsageAuditReport;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualReport {
    pub source: String,
    pub alpha: String,
    pub resolve: String,
    pub template: String,
    pub specialize: String,
    pub symbol_lineage: String,
    pub monomorphize: String,
    pub template_audit: String,
    pub typed_signature: String,
    pub typed: String,
    pub pattern: String,
    pub control: String,
    pub usage: String,
    pub cps: String,
    pub cps_usage: String,
    pub continuation_simplification: String,
    pub closure: String,
    pub lambda_lift: String,
    pub core: String,
    pub callable_storage: String,
    pub closure_core_usage: String,
    pub closure_simplification: String,
    pub usage_audit: String,
    pub std_audit: String,
    pub core_validation: String,
    pub backend: String,
    pub backend_link: String,
    pub backend_cache_key: String,
    pub nanopass: String,
}

pub fn render_visual_report(report: &VisualReport) -> String {
    let mut out = String::new();
    writeln!(out, "source:").unwrap();
    writeln!(out, "  {}", report.source).unwrap();
    writeln!(out, "alpha:").unwrap();
    writeln!(out, "  {}", report.alpha).unwrap();
    writeln!(out, "resolve:").unwrap();
    writeln!(out, "  {}", report.resolve).unwrap();
    writeln!(out, "template:").unwrap();
    writeln!(out, "  {}", report.template).unwrap();
    writeln!(out, "specialize:").unwrap();
    writeln!(out, "  {}", report.specialize).unwrap();
    writeln!(out, "symbol-lineage:").unwrap();
    for line in report.symbol_lineage.lines() {
        writeln!(out, "  {line}").unwrap();
    }
    writeln!(out, "monomorphize:").unwrap();
    writeln!(out, "  {}", report.monomorphize).unwrap();
    writeln!(out, "template-audit:").unwrap();
    writeln!(out, "  {}", report.template_audit).unwrap();
    writeln!(out, "typed-signature: {}", report.typed_signature).unwrap();
    writeln!(out, "typed:").unwrap();
    writeln!(out, "  {}", report.typed).unwrap();
    writeln!(out, "pattern:").unwrap();
    writeln!(out, "  {}", report.pattern).unwrap();
    writeln!(out, "control:").unwrap();
    writeln!(out, "  {}", report.control).unwrap();
    writeln!(out, "usage:").unwrap();
    writeln!(out, "  {}", report.usage).unwrap();
    writeln!(out, "cps:").unwrap();
    writeln!(out, "  {}", report.cps).unwrap();
    writeln!(out, "cps-usage:").unwrap();
    writeln!(out, "  {}", report.cps_usage).unwrap();
    writeln!(out, "continuation-simplification:").unwrap();
    writeln!(out, "  {}", report.continuation_simplification).unwrap();
    writeln!(out, "closure:").unwrap();
    writeln!(out, "  {}", report.closure).unwrap();
    writeln!(out, "lambda-lift:").unwrap();
    writeln!(out, "  {}", report.lambda_lift).unwrap();
    writeln!(out, "core:").unwrap();
    writeln!(out, "  {}", report.core).unwrap();
    writeln!(out, "callable-storage:").unwrap();
    for line in report.callable_storage.lines() {
        writeln!(out, "  {line}").unwrap();
    }
    writeln!(out, "closure-core-usage:").unwrap();
    writeln!(out, "  {}", report.closure_core_usage).unwrap();
    writeln!(out, "closure-simplification:").unwrap();
    writeln!(out, "  {}", report.closure_simplification).unwrap();
    writeln!(out, "usage-audit:").unwrap();
    writeln!(out, "  {}", report.usage_audit).unwrap();
    writeln!(out, "std-audit:").unwrap();
    writeln!(out, "  {}", report.std_audit).unwrap();
    writeln!(out, "core-validation:").unwrap();
    writeln!(out, "  {}", report.core_validation).unwrap();
    writeln!(out, "backend:").unwrap();
    writeln!(out, "  {}", report.backend).unwrap();
    writeln!(out, "backend-link:").unwrap();
    writeln!(out, "  {}", report.backend_link).unwrap();
    writeln!(out, "backend-cache-key:").unwrap();
    writeln!(out, "  {}", report.backend_cache_key).unwrap();
    writeln!(out, "nanopass:").unwrap();
    for event in report.nanopass.lines() {
        writeln!(out, "  {event}").unwrap();
    }
    out
}

pub fn visual_report(
    source: &Expr,
    alpha: &AlphaFacts,
    resolve: &ResolveFacts,
    template: &TemplateFacts,
    specialize: &SpecializationFacts,
    monomorphize: &MonomorphizationPlan,
    template_audit: &TemplateAuditReport,
    typed_signature: &str,
    typed: &TypedExpr,
    pattern: &PatternFacts,
    control: &ControlFacts,
    usage: &UsageFacts,
    cps: &CpsProgram,
    cps_usage: &CpsUsageFacts,
    continuation_simplification: &ContinuationSimplificationFacts,
    closure: &ClosureFacts,
    lambda_lift: &LambdaLiftFacts,
    core: &CoreProgram,
    callable_storage: &str,
    closure_core_usage: &ClosureCoreUsageFacts,
    closure_simplification: &ClosureSimplificationFacts,
    usage_audit: &UsageAuditReport,
    std_audit: &StdAuditReport,
    core_validation: &CoreValidation,
    backend: &BackendArtifact,
    backend_link: &BackendLinkedBundle,
    backend_cache_key: &BackendCacheKey,
    passes: &PassReport,
) -> VisualReport {
    VisualReport {
        source: render_source_expr(source),
        alpha: format!("{alpha:#?}"),
        resolve: render_resolve_facts(resolve),
        template: render_template_facts(template),
        specialize: render_specialization_facts(specialize),
        symbol_lineage: render_symbol_lineage(resolve, template, specialize),
        monomorphize: format!("{monomorphize:#?}"),
        template_audit: format!("{template_audit:#?}"),
        typed_signature: typed_signature.to_string(),
        typed: format!("{typed:#?}"),
        pattern: format!("{pattern:#?}"),
        control: format!("{control:#?}"),
        usage: format!("{usage:#?}"),
        cps: cps.to_string(),
        cps_usage: format!("{cps_usage:#?}"),
        continuation_simplification: format!("{continuation_simplification:#?}"),
        closure: format!("{closure:#?}"),
        lambda_lift: format!("{lambda_lift:#?}"),
        core: format!("{core:#?}"),
        callable_storage: callable_storage.to_string(),
        closure_core_usage: format!("{closure_core_usage:#?}"),
        closure_simplification: format!("{closure_simplification:#?}"),
        usage_audit: format!("{usage_audit:#?}"),
        std_audit: format!("{std_audit:#?}"),
        core_validation: format!("{core_validation:#?}"),
        backend: format!("{backend:#?}"),
        backend_link: format!("{backend_link:#?}"),
        backend_cache_key: format!("{backend_cache_key:#?}"),
        nanopass: render_pass_report(passes),
    }
}

fn render_symbol_lineage(
    resolve: &ResolveFacts,
    template: &TemplateFacts,
    specialize: &SpecializationFacts,
) -> String {
    let mut out = String::new();
    for name in &resolve.resolved_names {
        match name {
            ResolvedName::Function { name, symbol } => {
                writeln!(out, "resolve function {name} -> {symbol}").unwrap();
            }
            ResolvedName::Static { name, symbol } => {
                writeln!(out, "resolve static {name} -> {symbol}").unwrap();
            }
            ResolvedName::Constructor {
                data,
                ctor,
                symbol,
                arity,
            } => {
                writeln!(out, "resolve constructor {data}.{ctor}/{arity} -> {symbol}").unwrap();
            }
        }
    }
    for obligation in &template.obligations {
        match obligation {
            TemplateObligation::Function { name, resolved } => {
                writeln!(out, "template function {name} -> {resolved}").unwrap();
            }
            TemplateObligation::Static { name, resolved } => {
                writeln!(out, "template static {name} -> {resolved}").unwrap();
            }
            TemplateObligation::Constructor {
                data,
                ctor,
                resolved,
                arity,
            } => {
                writeln!(
                    out,
                    "template constructor {data}.{ctor}/{arity} -> {resolved}"
                )
                .unwrap();
            }
            TemplateObligation::Method {
                name,
                resolved: Some(resolved),
                ..
            } => {
                writeln!(out, "template method {name} -> {resolved}").unwrap();
            }
            _ => {}
        }
    }
    for work_item in &specialize.work_items {
        for obligation in &work_item.obligations {
            match obligation {
                DischargedObligation::Function { name, target } => {
                    writeln!(out, "specialize function {name} -> {target}").unwrap();
                }
                DischargedObligation::Static { name, target } => {
                    writeln!(out, "specialize static {name} -> {target}").unwrap();
                }
                DischargedObligation::Constructor {
                    data,
                    ctor,
                    target,
                    arity,
                } => {
                    writeln!(
                        out,
                        "specialize constructor {data}.{ctor}/{arity} -> {target}"
                    )
                    .unwrap();
                }
                DischargedObligation::Method {
                    name,
                    target: Some(target),
                } => {
                    writeln!(out, "specialize method {name} -> {target}").unwrap();
                }
                _ => {}
            }
        }
    }
    if out.is_empty() {
        out.push_str("<empty>\n");
    }
    out
}

fn render_resolve_facts(resolve: &ResolveFacts) -> String {
    let mut out = String::new();
    writeln!(out, "names={}", resolve.resolved_names.len()).unwrap();
    for name in &resolve.resolved_names {
        writeln!(out, "name {}", render_resolved_name(name)).unwrap();
    }
    writeln!(out, "calls={}", resolve.resolved_calls.len()).unwrap();
    for call in &resolve.resolved_calls {
        writeln!(out, "call {}", render_resolved_call(call)).unwrap();
    }
    writeln!(out, "operators={}", resolve.operator_obligations.len()).unwrap();
    for obligation in &resolve.operator_obligations {
        writeln!(
            out,
            "operator {} protocol={} receiver={}",
            render_operator_surface(&obligation.op),
            obligation.protocol,
            obligation.receiver.as_deref().unwrap_or("<unknown>")
        )
        .unwrap();
    }
    writeln!(out, "diagnostics={}", resolve.diagnostics.len()).unwrap();
    for diagnostic in &resolve.diagnostics {
        writeln!(out, "diagnostic {}", render_resolve_diagnostic(diagnostic)).unwrap();
    }
    out
}

fn render_resolved_name(name: &ResolvedName) -> String {
    match name {
        ResolvedName::Function { name, symbol } => format!("function {name} -> {symbol}"),
        ResolvedName::Static { name, symbol } => format!("static {name} -> {symbol}"),
        ResolvedName::Constructor {
            data,
            ctor,
            symbol,
            arity,
        } => format!("constructor {data}.{ctor}/{arity} -> {symbol}"),
    }
}

fn render_resolved_call(call: &ResolvedCall) -> String {
    match call {
        ResolvedCall::FieldCallable { field } => format!("field-callable {field}"),
        ResolvedCall::ReceiverMethod {
            receiver,
            name,
            symbol,
        } => format!("receiver-method {receiver}.{name} -> {symbol}"),
        ResolvedCall::QualifiedCallee { path, symbol } => {
            format!("qualified-callee {path} -> {symbol}")
        }
    }
}

fn render_resolve_diagnostic(diagnostic: &ResolveDiagnostic) -> String {
    match diagnostic {
        ResolveDiagnostic::AmbiguousMethod {
            receiver,
            name,
            candidates,
        } => format!(
            "ambiguous method {receiver}.{name}: [{}]",
            candidates.join(", ")
        ),
        ResolveDiagnostic::MissingMethod { receiver, name } => format!(
            "missing method {}.{}",
            receiver.as_deref().unwrap_or("<unknown>"),
            name
        ),
        ResolveDiagnostic::AmbiguousName { name, candidates } => {
            format!("ambiguous name {name}: [{}]", candidates.join(", "))
        }
        ResolveDiagnostic::FunctionArityMismatch {
            name,
            symbol,
            expected,
            actual,
        } => format!(
            "function arity mismatch {name} -> {symbol}: expected {expected} actual {actual}"
        ),
        ResolveDiagnostic::AmbiguousConstructor {
            data,
            ctor,
            candidates,
        } => format!(
            "ambiguous constructor {data}.{ctor}: [{}]",
            candidates.join(", ")
        ),
        ResolveDiagnostic::MissingConstructor { data, ctor } => {
            format!("missing constructor {data}.{ctor}")
        }
        ResolveDiagnostic::ConstructorArityMismatch {
            data,
            ctor,
            expected,
            actual,
        } => {
            format!("constructor arity mismatch {data}.{ctor}: expected {expected} actual {actual}")
        }
    }
}

fn render_specialization_facts(specialize: &SpecializationFacts) -> String {
    let mut out = String::new();
    writeln!(out, "work-items={}", specialize.work_items.len()).unwrap();
    for (index, item) in specialize.work_items.iter().enumerate() {
        writeln!(
            out,
            "work-item {index} {}",
            render_specialization_key(&item.key)
        )
        .unwrap();
        writeln!(
            out,
            "work-item {index} obligations={}",
            item.obligations.len()
        )
        .unwrap();
        for obligation in &item.obligations {
            writeln!(
                out,
                "work-item {index} obligation {}",
                render_discharged_obligation(obligation)
            )
            .unwrap();
        }
    }
    writeln!(out, "registry-entries={}", specialize.registry.len()).unwrap();
    out
}

fn render_specialization_key(key: &SpecializationKey) -> String {
    let params = key
        .template_params
        .iter()
        .map(|param| {
            format!(
                "{}:{}",
                param.name,
                render_template_param_source(param.source)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let instantiations = key
        .explicit_instantiations
        .iter()
        .map(|instantiation| {
            format!(
                "{}[{}]",
                instantiation.callee,
                instantiation.type_args.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let shapes = key
        .normalized_shapes
        .iter()
        .map(render_row_shape)
        .collect::<Vec<_>>()
        .join(", ");
    let dyn_contracts = key
        .dyn_contracts
        .iter()
        .map(render_dyn_contract)
        .collect::<Vec<_>>()
        .join(", ");
    let usage = key
        .capabilities
        .usage
        .iter()
        .copied()
        .map(render_usage_color)
        .collect::<Vec<_>>()
        .join(", ");
    let send = key
        .capabilities
        .send
        .iter()
        .copied()
        .map(render_send_color)
        .collect::<Vec<_>>()
        .join(", ");
    let continuations = key
        .continuations
        .iter()
        .copied()
        .map(render_continuation_kind)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "key generic={} abi={} params=[{}] instantiations=[{}] nominals=[{}] shapes=[{}] usage=[{}] send=[{}] continuations=[{}] dyn=[{}]",
        key.generic_symbol,
        render_abi_mode(key.abi_mode),
        params,
        instantiations,
        key.concrete_nominals.join(", "),
        shapes,
        usage,
        send,
        continuations,
        dyn_contracts
    )
}

fn render_discharged_obligation(obligation: &DischargedObligation) -> String {
    match obligation {
        DischargedObligation::Field { field, shape } => {
            format!("field {field} in {}", render_row_shape(shape))
        }
        DischargedObligation::Method { name, target } => format!(
            "method {name} -> {}",
            target.as_deref().unwrap_or("<pending>")
        ),
        DischargedObligation::Function { name, target } => {
            format!("function {name} -> {target}")
        }
        DischargedObligation::Static { name, target } => {
            format!("static {name} -> {target}")
        }
        DischargedObligation::Constructor {
            data,
            ctor,
            target,
            arity,
        } => format!("constructor {data}.{ctor}/{arity} -> {target}"),
        DischargedObligation::Operator {
            op,
            protocol,
            receiver,
        } => format!(
            "operator {} protocol={} receiver={}",
            render_operator_surface(op),
            protocol,
            receiver.as_deref().unwrap_or("<unknown>")
        ),
        DischargedObligation::DynAdapter { contract } => {
            format!("dyn-adapter {}", render_dyn_contract(contract))
        }
    }
}

fn render_continuation_kind(kind: ContinuationKind) -> &'static str {
    match kind {
        ContinuationKind::Cont1 => "cont1",
        ContinuationKind::ContN => "contn",
    }
}

fn render_abi_mode(mode: AbiMode) -> &'static str {
    match mode {
        AbiMode::Chiba => "chiba",
        AbiMode::Wasi => "wasi",
        AbiMode::Env => "env",
    }
}

fn render_template_facts(template: &TemplateFacts) -> String {
    let mut out = String::new();
    writeln!(out, "params={}", template.explicit_params.len()).unwrap();
    for param in &template.explicit_params {
        writeln!(
            out,
            "param {} source={}",
            param.name,
            render_template_param_source(param.source)
        )
        .unwrap();
    }
    writeln!(
        out,
        "instantiations={}",
        template.explicit_instantiations.len()
    )
    .unwrap();
    for instantiation in &template.explicit_instantiations {
        writeln!(
            out,
            "instantiate {}[{}]",
            instantiation.callee,
            instantiation.type_args.join(", ")
        )
        .unwrap();
    }
    writeln!(out, "row-shapes={}", template.row_shapes.len()).unwrap();
    for shape in &template.row_shapes {
        writeln!(out, "row-shape {}", render_row_shape(shape)).unwrap();
    }
    writeln!(out, "dyn-contracts={}", template.dyn_contracts.len()).unwrap();
    for contract in &template.dyn_contracts {
        writeln!(out, "dyn-contract {}", render_dyn_contract(contract)).unwrap();
    }
    writeln!(out, "obligations={}", template.obligations.len()).unwrap();
    for obligation in &template.obligations {
        writeln!(out, "obligation {}", render_template_obligation(obligation)).unwrap();
    }
    writeln!(out, "diagnostics={}", template.diagnostics.len()).unwrap();
    for diagnostic in &template.diagnostics {
        writeln!(out, "diagnostic {}", render_template_diagnostic(diagnostic)).unwrap();
    }
    out
}

fn render_template_param_source(source: TemplateParamSource) -> &'static str {
    match source {
        TemplateParamSource::ExplicitHeader => "explicit-header",
        TemplateParamSource::SyntheticAutoGeneric => "synthetic-auto-generic",
    }
}

fn render_template_obligation(obligation: &TemplateObligation) -> String {
    match obligation {
        TemplateObligation::Field { shape, field } => {
            format!("field {field} in {}", render_row_shape(shape))
        }
        TemplateObligation::Method {
            receiver,
            name,
            resolved,
        } => format!(
            "method {}.{} -> {}",
            receiver.as_deref().unwrap_or("<unknown>"),
            name,
            resolved.as_deref().unwrap_or("<pending>")
        ),
        TemplateObligation::Function { name, resolved } => {
            format!("function {name} -> {resolved}")
        }
        TemplateObligation::Static { name, resolved } => {
            format!("static {name} -> {resolved}")
        }
        TemplateObligation::Constructor {
            data,
            ctor,
            resolved,
            arity,
        } => format!("constructor {data}.{ctor}/{arity} -> {resolved}"),
        TemplateObligation::Operator {
            op,
            protocol,
            receiver,
        } => format!(
            "operator {} protocol={} receiver={}",
            render_operator_surface(op),
            protocol,
            receiver.as_deref().unwrap_or("<unknown>")
        ),
        TemplateObligation::DynAdapter { contract } => {
            format!("dyn-adapter {}", render_dyn_contract(contract))
        }
    }
}

fn render_template_diagnostic(diagnostic: &TemplateDiagnostic) -> String {
    match diagnostic {
        TemplateDiagnostic::ExplicitAutoGenericConflict { param } => {
            format!("explicit-auto-generic-conflict {param}")
        }
        TemplateDiagnostic::ConflictingExplicitInstantiation {
            callee,
            previous_type_args,
            type_args,
        } => format!(
            "conflicting-explicit-instantiation {callee}[{}] vs [{}]",
            previous_type_args.join(", "),
            type_args.join(", ")
        ),
    }
}

fn render_dyn_contract(contract: &DynRowContract) -> String {
    format!(
        "{} payload={} send={} adapter={}",
        render_row_shape(&contract.shape),
        render_usage_color(contract.payload_usage),
        render_send_color(contract.send),
        render_dyn_adapter_kind(contract.adapter)
    )
}

fn render_row_shape(shape: &RowShape) -> String {
    let fields = shape
        .fields
        .iter()
        .map(|field| format!("{}: {}", field.name, render_shape_type(&field.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let marker = match shape.openness {
        RowOpenness::Open => "r | ",
        RowOpenness::Closed => "",
    };
    format!("{{{marker}{fields}}}")
}

fn render_shape_type(ty: &ShapeType) -> String {
    match ty {
        ShapeType::Unknown => "_".to_string(),
        ShapeType::Named(name) => name.clone(),
    }
}

fn render_operator_surface(op: &OperatorSurface) -> String {
    match op {
        OperatorSurface::Binary(op) => render_source_binary_op(*op).to_string(),
        OperatorSurface::Index => "[]".to_string(),
        OperatorSurface::IndexSlice => "[..]".to_string(),
    }
}

fn render_usage_color(color: crate::typed::UsageColor) -> &'static str {
    match color {
        crate::typed::UsageColor::One => "1",
        crate::typed::UsageColor::Many => "N",
        crate::typed::UsageColor::Obligation => "obligation",
    }
}

fn render_send_color(color: crate::typed::SendColor) -> &'static str {
    match color {
        crate::typed::SendColor::Send => "send",
        crate::typed::SendColor::NotSend => "!send",
        crate::typed::SendColor::Obligation => "obligation",
    }
}

fn render_dyn_adapter_kind(kind: DynAdapterKind) -> &'static str {
    match kind {
        DynAdapterKind::StaticToDynPackage => "static-to-dyn-package",
        DynAdapterKind::DynAdapterAccess => "dyn-adapter-access",
    }
}

fn render_pass_report(report: &PassReport) -> String {
    let mut out = String::new();
    for event in &report.events {
        writeln!(
            out,
            "{}: {} -> {} ({}us)",
            event.name,
            event.input,
            event.output,
            event.elapsed.as_micros()
        )
        .unwrap();
    }
    out
}
