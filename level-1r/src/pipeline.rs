use crate::ast::{Expr, SourceItem, SourceProgram};
use crate::closure::{analyze_closures, ClosureFacts};
use crate::control::{analyze_control, ControlFacts};
use crate::core::{lower_core, CoreProgram};
use crate::cps::{cps_program, CpsProgram};
use crate::debug::{render_visual_report, visual_report, VisualReport};
use crate::nanopass::PassReport;
use crate::typed::{type_expr, TypedExpr};
use crate::usage::{analyze_usage, UsageFacts};

#[derive(Clone, Debug)]
pub struct CompileOutput {
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
    let typed = passes.record("L2Typed", "SourceExpr", "TypedExpr", || type_expr(expr));
    let control = passes.record("L3AnswerControl", "TypedExpr", "ControlFacts", || {
        analyze_control(&typed)
    });
    let usage = passes.record("L4Usage", "TypedExpr", "UsageFacts", || {
        analyze_usage(&typed)
    });
    let cps = passes.record("L5OnePassCps", "TypedExpr", "CpsProgram", || {
        cps_program(&typed)
    });
    let closure = passes.record("L6Closure", "TypedExpr", "ClosureFacts", || {
        analyze_closures(&typed)
    });
    let core = passes.record("L7Core", "CpsProgram", "CoreProgram", || {
        lower_core(&cps, &control.continuations)
    });
    let visual = visual_report(
        expr, &typed, &control, &usage, &cps, &closure, &core, &passes,
    );
    CompileOutput {
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
