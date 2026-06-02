use std::fmt::Write;

use crate::alpha::AlphaFacts;
use crate::ast::Expr;
use crate::backend::{BackendArtifact, BackendCacheKey, BackendLinkedBundle};
use crate::closure::ClosureFacts;
use crate::closure_core_usage::ClosureCoreUsageFacts;
use crate::closure_simplify::ClosureSimplificationFacts;
use crate::control::ControlFacts;
use crate::core::{CoreProgram, CoreValidation};
use crate::cps::CpsProgram;
use crate::cps_usage::{ContinuationSimplificationFacts, CpsUsageFacts};
use crate::lambda_lift::LambdaLiftFacts;
use crate::monomorphize::MonomorphizationPlan;
use crate::nanopass::PassReport;
use crate::pattern::PatternFacts;
use crate::resolve::{ResolveFacts, ResolvedName};
use crate::specialize::{DischargedObligation, SpecializationFacts};
use crate::std_audit::StdAuditReport;
use crate::template::{TemplateFacts, TemplateObligation};
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
        source: format!("{source:?}"),
        alpha: format!("{alpha:#?}"),
        resolve: format!("{resolve:#?}"),
        template: format!("{template:#?}"),
        specialize: format!("{specialize:#?}"),
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
