use std::fmt::Write;

use crate::alpha::AlphaFacts;
use crate::ast::Expr;
use crate::closure::ClosureFacts;
use crate::control::ControlFacts;
use crate::core::{CoreProgram, CoreValidation};
use crate::cps::CpsProgram;
use crate::cps_usage::{ContinuationSimplificationFacts, CpsUsageFacts};
use crate::lambda_lift::LambdaLiftFacts;
use crate::nanopass::PassReport;
use crate::resolve::ResolveFacts;
use crate::specialize::SpecializationFacts;
use crate::template::TemplateFacts;
use crate::typed::TypedExpr;
use crate::usage::UsageFacts;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualReport {
    pub source: String,
    pub alpha: String,
    pub resolve: String,
    pub template: String,
    pub specialize: String,
    pub typed: String,
    pub control: String,
    pub usage: String,
    pub cps: String,
    pub cps_usage: String,
    pub continuation_simplification: String,
    pub closure: String,
    pub lambda_lift: String,
    pub core: String,
    pub core_validation: String,
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
    writeln!(out, "typed:").unwrap();
    writeln!(out, "  {}", report.typed).unwrap();
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
    writeln!(out, "core-validation:").unwrap();
    writeln!(out, "  {}", report.core_validation).unwrap();
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
    typed: &TypedExpr,
    control: &ControlFacts,
    usage: &UsageFacts,
    cps: &CpsProgram,
    cps_usage: &CpsUsageFacts,
    continuation_simplification: &ContinuationSimplificationFacts,
    closure: &ClosureFacts,
    lambda_lift: &LambdaLiftFacts,
    core: &CoreProgram,
    core_validation: &CoreValidation,
    passes: &PassReport,
) -> VisualReport {
    VisualReport {
        source: format!("{source:?}"),
        alpha: format!("{alpha:#?}"),
        resolve: format!("{resolve:#?}"),
        template: format!("{template:#?}"),
        specialize: format!("{specialize:#?}"),
        typed: format!("{typed:#?}"),
        control: format!("{control:#?}"),
        usage: format!("{usage:#?}"),
        cps: cps.to_string(),
        cps_usage: format!("{cps_usage:#?}"),
        continuation_simplification: format!("{continuation_simplification:#?}"),
        closure: format!("{closure:#?}"),
        lambda_lift: format!("{lambda_lift:#?}"),
        core: format!("{core:#?}"),
        core_validation: format!("{core_validation:#?}"),
        nanopass: render_pass_report(passes),
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
