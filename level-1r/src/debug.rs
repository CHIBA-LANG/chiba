use std::fmt::Write;

use crate::alpha::AlphaFacts;
use crate::ast::Expr;
use crate::closure::ClosureFacts;
use crate::control::ControlFacts;
use crate::core::CoreProgram;
use crate::cps::CpsProgram;
use crate::nanopass::PassReport;
use crate::typed::TypedExpr;
use crate::usage::UsageFacts;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualReport {
    pub source: String,
    pub alpha: String,
    pub typed: String,
    pub control: String,
    pub usage: String,
    pub cps: String,
    pub closure: String,
    pub core: String,
    pub nanopass: String,
}

pub fn render_visual_report(report: &VisualReport) -> String {
    let mut out = String::new();
    writeln!(out, "source:").unwrap();
    writeln!(out, "  {}", report.source).unwrap();
    writeln!(out, "alpha:").unwrap();
    writeln!(out, "  {}", report.alpha).unwrap();
    writeln!(out, "typed:").unwrap();
    writeln!(out, "  {}", report.typed).unwrap();
    writeln!(out, "control:").unwrap();
    writeln!(out, "  {}", report.control).unwrap();
    writeln!(out, "usage:").unwrap();
    writeln!(out, "  {}", report.usage).unwrap();
    writeln!(out, "cps:").unwrap();
    writeln!(out, "  {}", report.cps).unwrap();
    writeln!(out, "closure:").unwrap();
    writeln!(out, "  {}", report.closure).unwrap();
    writeln!(out, "core:").unwrap();
    writeln!(out, "  {}", report.core).unwrap();
    writeln!(out, "nanopass:").unwrap();
    for event in report.nanopass.lines() {
        writeln!(out, "  {event}").unwrap();
    }
    out
}

pub fn visual_report(
    source: &Expr,
    alpha: &AlphaFacts,
    typed: &TypedExpr,
    control: &ControlFacts,
    usage: &UsageFacts,
    cps: &CpsProgram,
    closure: &ClosureFacts,
    core: &CoreProgram,
    passes: &PassReport,
) -> VisualReport {
    VisualReport {
        source: format!("{source:?}"),
        alpha: format!("{alpha:#?}"),
        typed: format!("{typed:#?}"),
        control: format!("{control:#?}"),
        usage: format!("{usage:#?}"),
        cps: cps.to_string(),
        closure: format!("{closure:#?}"),
        core: format!("{core:#?}"),
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
