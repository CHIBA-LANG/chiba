use crate::alpha::{alpha_expr, AlphaFacts};
use std::collections::BTreeSet;

use crate::ast::{Expr, NamespaceDecl, SourceItem, SourceProgram, UseDecl};
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
use crate::frontend::{parse_source_program, FrontendError, FrontendOutput};
use crate::global::{analyze_global_init, GlobalInitDiagnostic, GlobalInitPlan};
use crate::lambda_lift::{lift_lambdas, LambdaLiftFacts};
use crate::monomorphize::{schedule_monomorphization, MonomorphizationPlan};
use crate::nanopass::PassReport;
use crate::pattern::{analyze_patterns, PatternFacts};
use crate::resolve::{resolve_expr, resolve_expr_with_names, MethodIndex, NameIndex, ResolveFacts};
use crate::specialize::{plan_specialization, SpecializationFacts};
use crate::std_audit::{audit_std_dependencies, StdAuditReport};
use crate::symbol::encode_debug_symbol;
use crate::surface::{
    build_interface_summary, duplicate_constructor_names, duplicate_data_names, project_surface,
    InterfaceSummary, ProjectSurface,
};
use crate::template::{analyze_template, TemplateFacts};
use crate::template_audit::{audit_checked_templates, TemplateAuditReport};
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
    pub template_audit: TemplateAuditReport,
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

#[derive(Clone, Debug)]
pub struct ProgramCompileOutput {
    pub namespace: Option<NamespaceDecl>,
    pub imports: Vec<UseDecl>,
    pub surface: ProjectSurface,
    pub interface: InterfaceSummary,
    pub global_init: GlobalInitPlan,
    pub defs: Vec<ProgramDefOutput>,
    pub diagnostics: Vec<ProgramDiagnostic>,
    pub entry: Option<String>,
    pub backend_link: BackendLinkedBundle,
    pub backend_cache_key: BackendCacheKey,
    pub passes: PassReport,
}

#[derive(Clone, Debug)]
pub struct SourceCompileOutput {
    pub frontend: FrontendOutput,
    pub program: ProgramCompileOutput,
}

#[derive(Clone, Debug)]
pub struct ProgramDefOutput {
    pub name: String,
    pub params: Vec<String>,
    pub output: CompileOutput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramDiagnostic {
    DuplicateDef { name: String },
    DuplicateData { name: String },
    DuplicateConstructor { name: String },
    DuplicateStatic { name: String },
    StaticFunctionNameConflict { name: String },
    StaticInitCycle { cycle: Vec<String> },
    MissingEntry,
    EntryHasParams { name: String, params: Vec<String> },
}

pub fn compile_expr(expr: &Expr) -> CompileOutput {
    compile_expr_with_name_index(expr, NameIndex::default())
}

fn compile_expr_with_name_index(expr: &Expr, names: NameIndex) -> CompileOutput {
    let mut passes = PassReport::default();
    let alpha = passes.record("L1Alpha", "SourceExpr", "AlphaFacts", || alpha_expr(expr));
    let resolve = passes.record("L2Resolve", "AlphaExpr", "ResolveFacts", || {
        if names == NameIndex::default() {
            resolve_expr(&alpha.expr, MethodIndex::default())
        } else {
            resolve_expr_with_names(&alpha.expr, MethodIndex::default(), names)
        }
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
    let template_audit = passes.record(
        "L6TemplateAudit",
        "TemplateFacts+SpecializationFacts+MonomorphizationPlan",
        "TemplateAuditReport",
        || audit_checked_templates(&template, &specialize, &monomorphize),
    );
    let typed = passes.record("L7Typed", "SourceExpr", "TypedExpr", || type_expr(expr));
    let pattern = passes.record("L8PatternElab", "TypedExpr", "PatternFacts", || {
        analyze_patterns(&typed)
    });
    let control = passes.record("L9AnswerControl", "TypedExpr", "ControlFacts", || {
        analyze_control(&typed)
    });
    let usage = passes.record("L10Usage", "AlphaExpr", "UsageFacts", || {
        analyze_alpha_usage(&alpha.expr)
    });
    let cps = passes.record("L11OnePassCps", "TypedExpr", "CpsProgram", || {
        cps_program(&typed)
    });
    let cps_usage = passes.record("L12CpsUsage", "CpsProgram", "CpsUsageFacts", || {
        analyze_cps_usage(&cps)
    });
    let continuation_simplification = passes.record(
        "L13ContSimplify",
        "CpsUsageFacts",
        "ContinuationSimplificationFacts",
        || simplify_continuations(&cps_usage),
    );
    let closure = passes.record("L14Closure", "AlphaExpr", "ClosureFacts", || {
        analyze_alpha_closures(&alpha.expr)
    });
    let lambda_lift = passes.record("L15LambdaLift", "ClosureFacts", "LambdaLiftFacts", || {
        lift_lambdas(&closure)
    });
    let core = passes.record("L16Core", "CpsProgram", "CoreProgram", || {
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
        "L17ClosureCoreUsage",
        "CoreProgram",
        "ClosureCoreUsageFacts",
        || analyze_closure_core_usage(&core),
    );
    let closure_simplification = passes.record(
        "L18ClosureSimplify",
        "ClosureCoreUsageFacts",
        "ClosureSimplificationFacts",
        || simplify_closure_core(&closure_core_usage),
    );
    let usage_audit = passes.record(
        "L19UsageAudit",
        "TypedExpr+UsageFacts+CoreProgram",
        "UsageAuditReport",
        || audit_usage_lowering(expr, &typed, &usage, &control, &core),
    );
    let std_audit = passes.record("L20StdAudit", "CompilerCrate", "StdAuditReport", || {
        audit_std_dependencies()
    });
    let core_validation = passes.record("L21CoreValidate", "CoreProgram", "CoreValidation", || {
        validate_core(&core)
    });
    let backend = passes.record(
        "L22BackendEmit",
        "CoreProgram+CoreValidation",
        "BackendArtifact",
        || emit_wasm_gc(&core, &core_validation),
    );
    let backend_link = passes.record(
        "L23BackendLink",
        "BackendArtifact",
        "BackendLinkedBundle",
        || link_backend_artifacts(vec![backend.clone()]),
    );
    let backend_cache_key = passes.record(
        "L24BackendCacheKey",
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
        &template_audit,
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
        template_audit,
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
    compile_program_bundle(program)
        .defs
        .into_iter()
        .map(|def| def.output)
        .collect()
}

pub fn compile_source_program_bundle(source: &str) -> Result<SourceCompileOutput, FrontendError> {
    let frontend = parse_source_program(source)?;
    let program = compile_program_bundle(&frontend.program);
    Ok(SourceCompileOutput { frontend, program })
}

pub fn compile_program_bundle(program: &SourceProgram) -> ProgramCompileOutput {
    let mut passes = PassReport::default();
    let surface = passes.record("P1ProjectSurface", "SourceProgram", "ProjectSurface", || {
        project_surface(program)
    });
    let interface = passes.record(
        "P2InterfaceSummary",
        "ProjectSurface",
        "InterfaceSummary",
        || build_interface_summary(&surface),
    );
    let diagnostics = passes.record(
        "P3ProgramDiagnostics",
        "ProjectSurface",
        "ProgramDiagnostics",
        || program_surface_diagnostics(&surface),
    );
    let global_init = passes.record(
        "P4GlobalInit",
        "SourceProgram+ProjectSurface",
        "GlobalInitPlan",
        || analyze_global_init(program),
    );
    let defs = passes.record(
        "P5ProgramDefs",
        "SourceProgram+InterfaceSummary",
        "ProgramDefOutput",
        || compile_program_defs(program, &interface),
    );
    let entry = passes.record(
        "P6ProgramEntry",
        "ProgramDefOutput",
        "EntrySelection",
        || select_program_entry(&defs),
    );
    let mut all_diagnostics = diagnostics;
    all_diagnostics.extend(global_init.diagnostics.iter().cloned().map(Into::into));
    if entry.is_none() {
        all_diagnostics.push(ProgramDiagnostic::MissingEntry);
    }
    if let Some(entry_name) = &entry {
        if let Some(def) = defs.iter().find(|def| def.name == *entry_name) {
            if !def.params.is_empty() {
                all_diagnostics.push(ProgramDiagnostic::EntryHasParams {
                    name: def.name.clone(),
                    params: def.params.clone(),
                });
            }
        }
    }
    let backend_link = passes.record(
        "P7ProgramBackendLink",
        "ProgramDefOutput+EntrySelection",
        "BackendLinkedBundle",
        || link_backend_artifacts(program_backend_artifacts(&defs, entry.as_deref())),
    );
    let backend_cache_key = passes.record(
        "P8ProgramBackendCacheKey",
        "BackendLinkedBundle",
        "BackendCacheKey",
        || backend_cache_key(&backend_link, &BackendCacheConfig::default()),
    );
    ProgramCompileOutput {
        namespace: program.namespace.clone(),
        imports: program.imports.clone(),
        surface,
        interface,
        global_init,
        defs,
        diagnostics: all_diagnostics,
        entry,
        backend_link,
        backend_cache_key,
        passes,
    }
}

fn compile_program_defs(
    program: &SourceProgram,
    interface: &InterfaceSummary,
) -> Vec<ProgramDefOutput> {
    let names = NameIndex::from_interface(interface);
    program
        .items
        .iter()
        .filter_map(|item| match item {
            SourceItem::Def {
                name, params, body, ..
            } => Some(ProgramDefOutput {
                name: name.clone(),
                params: params.iter().map(|param| param.name.clone()).collect(),
                output: compile_expr_with_name_index(body, names.clone()),
            }),
            SourceItem::StaticValue { .. } => None,
        })
        .collect()
}

fn program_surface_diagnostics(surface: &ProjectSurface) -> Vec<ProgramDiagnostic> {
    let mut seen = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for def in &surface.defs {
        if !seen.insert(def.name.clone()) {
            diagnostics.push(ProgramDiagnostic::DuplicateDef {
                name: def.name.clone(),
            });
        }
    }
    diagnostics.extend(
        duplicate_data_names(surface)
            .into_iter()
            .map(|name| ProgramDiagnostic::DuplicateData { name }),
    );
    diagnostics.extend(
        duplicate_constructor_names(surface)
            .into_iter()
            .map(|name| ProgramDiagnostic::DuplicateConstructor { name }),
    );
    diagnostics
}

impl From<GlobalInitDiagnostic> for ProgramDiagnostic {
    fn from(diagnostic: GlobalInitDiagnostic) -> Self {
        match diagnostic {
            GlobalInitDiagnostic::DuplicateStatic { name } => {
                ProgramDiagnostic::DuplicateStatic { name }
            }
            GlobalInitDiagnostic::StaticFunctionNameConflict { name } => {
                ProgramDiagnostic::StaticFunctionNameConflict { name }
            }
            GlobalInitDiagnostic::StaticInitCycle { cycle } => {
                ProgramDiagnostic::StaticInitCycle { cycle }
            }
        }
    }
}

fn select_program_entry(defs: &[ProgramDefOutput]) -> Option<String> {
    if defs.iter().any(|def| def.name == "main") {
        Some("main".to_string())
    } else {
        defs.first().map(|def| def.name.clone())
    }
}

fn program_backend_artifacts(
    defs: &[ProgramDefOutput],
    entry: Option<&str>,
) -> Vec<BackendArtifact> {
    let mut entry_exported = false;
    defs.iter()
        .enumerate()
        .map(|(index, def)| {
            let mut artifact = def.output.backend.clone();
            let is_entry = entry == Some(def.name.as_str()) && !entry_exported;
            if is_entry {
                entry_exported = true;
            }
            artifact.wat =
                relabel_program_wat(&artifact.wat, &def_symbol(&def.name, index), is_entry);
            artifact
        })
        .collect()
}

fn relabel_program_wat(wat: &str, symbol: &str, is_entry: bool) -> String {
    if is_entry {
        wat.replace(
            "(func $main (export \"main\")",
            &format!("(func ${symbol} (export \"main\")"),
        )
    } else {
        wat.replace("(func $main (export \"main\")", &format!("(func ${symbol}"))
    }
}

fn def_symbol(name: &str, index: usize) -> String {
    let base = sanitize_program_symbol(name);
    if index == 0 {
        base
    } else {
        format!("{base}__def{index}")
    }
}

fn sanitize_program_symbol(name: &str) -> String {
    encode_debug_symbol(name)
}

impl CompileOutput {
    pub fn render_visual(&self) -> String {
        render_visual_report(&self.visual)
    }
}

impl ProgramCompileOutput {
    pub fn render_summary(&self) -> String {
        let mut out = String::new();
        out.push_str("program:\n");
        out.push_str(&format!(
            "  namespace={}\n",
            self.namespace
                .as_ref()
                .map(NamespaceDecl::dotted)
                .unwrap_or_else(|| "<root>".to_string())
        ));
        out.push_str(&format!(
            "  imports={:?}\n",
            self.imports
                .iter()
                .map(UseDecl::dotted)
                .collect::<Vec<_>>()
        ));
        out.push_str(&format!("  defs={}\n", self.defs.len()));
        out.push_str(&format!("  global-init={:#?}\n", self.global_init));
        out.push_str(&format!("  surface={:#?}\n", self.surface));
        out.push_str(&format!("  interface={:#?}\n", self.interface));
        out.push_str(&format!("  entry={:?}\n", self.entry));
        out.push_str(&format!("  diagnostics={:?}\n", self.diagnostics));
        out.push_str("  passes:\n");
        for event in &self.passes.events {
            out.push_str(&format!(
                "    {}: {} -> {}\n",
                event.name, event.input, event.output
            ));
        }
        out
    }
}

impl SourceCompileOutput {
    pub fn render_summary(&self) -> String {
        let mut out = String::new();
        out.push_str("source-program:\n");
        out.push_str(&format!("  tokens={}\n", self.frontend.tokens.len()));
        out.push_str(&format!(
            "  namespace={}\n",
            self.frontend
                .program
                .namespace
                .as_ref()
                .map(NamespaceDecl::dotted)
                .unwrap_or_else(|| "<root>".to_string())
        ));
        out.push_str(&format!(
            "  imports={:?}\n",
            self.frontend
                .program
                .imports
                .iter()
                .map(UseDecl::dotted)
                .collect::<Vec<_>>()
        ));
        out.push_str(&format!("  items={}\n", self.frontend.program.items.len()));
        out.push_str(&format!(
            "  data={}\n",
            self.frontend.program.data.len()
        ));
        out.push_str(&self.program.render_summary());
        out
    }
}
