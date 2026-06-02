use crate::alpha::{alpha_expr, AlphaFacts};
use crate::ast::{Expr, SourceItem, SourceProgram};
use crate::closure::{analyze_alpha_closures, ClosureFacts};
use crate::control::{analyze_control, ControlFacts};
use crate::core::{lower_core, CoreProgram};
use crate::cps::{cps_program, CpsProgram};
use crate::debug::{render_visual_report, visual_report, VisualReport};
use crate::nanopass::PassReport;
use crate::resolve::{resolve_expr, MethodIndex, ResolveFacts};
use crate::typed::{type_expr, TypedExpr};
use crate::usage::{analyze_alpha_usage, UsageFacts};

#[derive(Clone, Debug)]
pub struct CompileOutput {
    pub alpha: AlphaFacts,
    pub resolve: ResolveFacts,
    pub typed: TypedExpr,
    pub control: ControlFacts,
    pub usage: UsageFacts,
    pub cps: CpsProgram,
    pub closure: ClosureFacts,
    pub core: CoreProgram,
    pub passes: PassReport,
    pub visual: VisualReport,
}

pub fn compile_expr(expr: &Expr) -> CompileOutput {
    let mut passes = PassReport::default();
    let alpha = passes.record("L1Alpha", "SourceExpr", "AlphaFacts", || alpha_expr(expr));
    let resolve = passes.record("L2Resolve", "AlphaExpr", "ResolveFacts", || {
        resolve_expr(&alpha.expr, MethodIndex::default())
    });
    let typed = passes.record("L3Typed", "SourceExpr", "TypedExpr", || type_expr(expr));
    let control = passes.record("L4AnswerControl", "TypedExpr", "ControlFacts", || {
        analyze_control(&typed)
    });
    let usage = passes.record("L5Usage", "AlphaExpr", "UsageFacts", || {
        analyze_alpha_usage(&alpha.expr)
    });
    let cps = passes.record("L6OnePassCps", "TypedExpr", "CpsProgram", || {
        cps_program(&typed)
    });
    let closure = passes.record("L7Closure", "AlphaExpr", "ClosureFacts", || {
        analyze_alpha_closures(&alpha.expr)
    });
    let core = passes.record("L8Core", "CpsProgram", "CoreProgram", || {
        lower_core(&cps, &control.continuations)
    });
    let visual = visual_report(
        expr, &alpha, &resolve, &typed, &control, &usage, &cps, &closure, &core, &passes,
    );
    CompileOutput {
        alpha,
        resolve,
        typed,
        control,
        usage,
        cps,
        closure,
        core,
        passes,
        visual,
    }
}

pub fn compile_program(program: &SourceProgram) -> Vec<CompileOutput> {
    program
        .items
        .iter()
        .map(|item| match item {
            SourceItem::Def { body, .. } => compile_expr(body),
        })
        .collect()
}

impl CompileOutput {
    pub fn render_visual(&self) -> String {
        render_visual_report(&self.visual)
    }
}
