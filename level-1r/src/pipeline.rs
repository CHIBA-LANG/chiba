use crate::alpha::{alpha_expr, AlphaFacts};
use crate::ast::{Expr, SourceItem, SourceProgram};
use crate::backend::{
    backend_cache_key, emit_wasm_gc, link_backend_artifacts, BackendArtifact, BackendCacheConfig,
    BackendCacheKey, BackendLinkedBundle,
};
use crate::closure::{analyze_alpha_closures, ClosureFacts};
use crate::closure_core_usage::{analyze_closure_core_usage, ClosureCoreUsageFacts};
use crate::closure_simplify::{simplify_closure_core, ClosureSimplificationFacts};
use crate::control::{analyze_control, ControlFacts};
use crate::core::{lower_core_with_facts, validate_core, CoreProgram, CoreValidation};
use crate::cps::{cps_program, CpsProgram};
use crate::cps_usage::{
    analyze_cps_usage, simplify_continuations, ContinuationSimplificationFacts, CpsUsageFacts,
};
use crate::debug::{render_visual_report, visual_report, VisualReport};
use crate::lambda_lift::{lift_lambdas, LambdaLiftFacts};
use crate::monomorphize::{schedule_monomorphization, MonomorphizationPlan};
use crate::nanopass::PassReport;
use crate::pattern::{analyze_patterns, PatternFacts};
use crate::resolve::{resolve_expr, MethodIndex, ResolveFacts};
use crate::specialize::{plan_specialization, SpecializationFacts};
use crate::std_audit::{audit_std_dependencies, StdAuditReport};
use crate::template::{analyze_template, TemplateFacts};
use crate::typed::{type_expr, TypedExpr};
use crate::usage::{analyze_alpha_usage, UsageFacts};
use crate::usage_audit::{audit_usage_lowering, UsageAuditReport};

#[derive(Clone, Debug)]
pub struct CompileOutput {
    pub alpha: AlphaFacts,
    pub resolve: ResolveFacts,
    pub template: TemplateFacts,
    pub specialize: SpecializationFacts,
    pub monomorphize: MonomorphizationPlan,
    pub typed: TypedExpr,
    pub pattern: PatternFacts,
    pub control: ControlFacts,
    pub usage: UsageFacts,
    pub cps: CpsProgram,
    pub cps_usage: CpsUsageFacts,
    pub continuation_simplification: ContinuationSimplificationFacts,
    pub closure: ClosureFacts,
    pub lambda_lift: LambdaLiftFacts,
    pub core: CoreProgram,
    pub closure_core_usage: ClosureCoreUsageFacts,
    pub closure_simplification: ClosureSimplificationFacts,
    pub usage_audit: UsageAuditReport,
    pub std_audit: StdAuditReport,
    pub core_validation: CoreValidation,
    pub backend: BackendArtifact,
    pub backend_link: BackendLinkedBundle,
    pub backend_cache_key: BackendCacheKey,
    pub passes: PassReport,
    pub visual: VisualReport,
}

pub fn compile_expr(expr: &Expr) -> CompileOutput {
    let mut passes = PassReport::default();
    let alpha = passes.record("L1Alpha", "SourceExpr", "AlphaFacts", || alpha_expr(expr));
    let resolve = passes.record("L2Resolve", "AlphaExpr", "ResolveFacts", || {
        resolve_expr(&alpha.expr, MethodIndex::default())
    });
    let template = passes.record("L3Template", "AlphaExpr+ResolveFacts", "TemplateFacts", || {
        analyze_template(&alpha.expr, &resolve)
    });
    let specialize = passes.record("L4Specialize", "TemplateFacts", "SpecializationFacts", || {
        plan_specialization("<expr>", &template)
    });
    let monomorphize = passes.record(
        "L5Monomorphize",
        "SpecializationFacts",
        "MonomorphizationPlan",
        || schedule_monomorphization(&specialize),
    );
    let typed = passes.record("L6Typed", "SourceExpr", "TypedExpr", || type_expr(expr));
    let pattern = passes.record("L7PatternElab", "TypedExpr", "PatternFacts", || {
        analyze_patterns(&typed)
    });
    let control = passes.record("L8AnswerControl", "TypedExpr", "ControlFacts", || {
        analyze_control(&typed)
    });
    let usage = passes.record("L9Usage", "AlphaExpr", "UsageFacts", || {
        analyze_alpha_usage(&alpha.expr)
    });
    let cps = passes.record("L10OnePassCps", "TypedExpr", "CpsProgram", || {
        cps_program(&typed)
    });
    let cps_usage = passes.record("L11CpsUsage", "CpsProgram", "CpsUsageFacts", || {
        analyze_cps_usage(&cps)
    });
    let continuation_simplification = passes.record(
        "L12ContSimplify",
        "CpsUsageFacts",
        "ContinuationSimplificationFacts",
        || simplify_continuations(&cps_usage),
    );
    let closure = passes.record("L13Closure", "AlphaExpr", "ClosureFacts", || {
        analyze_alpha_closures(&alpha.expr)
    });
    let lambda_lift = passes.record("L14LambdaLift", "ClosureFacts", "LambdaLiftFacts", || {
        lift_lambdas(&closure)
    });
    let core = passes.record("L15Core", "CpsProgram", "CoreProgram", || {
        lower_core_with_facts(
            &cps,
            &control.continuations,
            &closure,
            &lambda_lift,
            &specialize,
            &usage,
        )
    });
    let closure_core_usage = passes.record(
        "L16ClosureCoreUsage",
        "CoreProgram",
        "ClosureCoreUsageFacts",
        || analyze_closure_core_usage(&core),
    );
    let closure_simplification = passes.record(
        "L17ClosureSimplify",
        "ClosureCoreUsageFacts",
        "ClosureSimplificationFacts",
        || simplify_closure_core(&closure_core_usage),
    );
    let usage_audit = passes.record(
        "L18UsageAudit",
        "TypedExpr+UsageFacts+CoreProgram",
        "UsageAuditReport",
        || audit_usage_lowering(expr, &typed, &usage, &control, &core),
    );
    let std_audit = passes.record("L19StdAudit", "CompilerCrate", "StdAuditReport", || {
        audit_std_dependencies()
    });
    let core_validation = passes.record("L20CoreValidate", "CoreProgram", "CoreValidation", || {
        validate_core(&core)
    });
    let backend = passes.record(
        "L21BackendEmit",
        "CoreProgram+CoreValidation",
        "BackendArtifact",
        || emit_wasm_gc(&core, &core_validation),
    );
    let backend_link = passes.record(
        "L22BackendLink",
        "BackendArtifact",
        "BackendLinkedBundle",
        || link_backend_artifacts(vec![backend.clone()]),
    );
    let backend_cache_key = passes.record(
        "L23BackendCacheKey",
        "BackendLinkedBundle",
        "BackendCacheKey",
        || backend_cache_key(&backend_link, &BackendCacheConfig::default()),
    );
    let visual = visual_report(
        expr,
        &alpha,
        &resolve,
        &template,
        &specialize,
        &monomorphize,
        &typed,
        &pattern,
        &control,
        &usage,
        &cps,
        &cps_usage,
        &continuation_simplification,
        &closure,
        &lambda_lift,
        &core,
        &closure_core_usage,
        &closure_simplification,
        &usage_audit,
        &std_audit,
        &core_validation,
        &backend,
        &backend_link,
        &backend_cache_key,
        &passes,
    );
    CompileOutput {
        alpha,
        resolve,
        template,
        specialize,
        monomorphize,
        typed,
        pattern,
        control,
        usage,
        cps,
        cps_usage,
        continuation_simplification,
        closure,
        lambda_lift,
        core,
        closure_core_usage,
        closure_simplification,
        usage_audit,
        std_audit,
        core_validation,
        backend,
        backend_link,
        backend_cache_key,
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
