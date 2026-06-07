use crate::alpha::{alpha_expr_with_params, AlphaBinder, AlphaFacts};
use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    render_source_expr, Expr, ExternAbi, ExternDecl, MethodReceiver, NamespaceDecl, ParamDecl,
    Pattern, SourceItem, SourceProgram, UseDecl, Visibility,
};
use crate::backend::{
    backend_cache_key, emit_wasm_gc_with_params, link_backend_artifacts, sort_dedup_imports,
    BackendArtifact, BackendCacheConfig, BackendCacheKey, BackendDiagnostic, BackendExternAbi,
    BackendExternImport, BackendLinkDiagnostic, BackendLinkedBundle,
};
use crate::closure::{analyze_alpha_closures, ClosureFacts};
use crate::closure_core_usage::{analyze_closure_core_usage, ClosureCoreUsageFacts};
use crate::closure_simplify::{simplify_closure_core, ClosureSimplificationFacts};
use crate::control::{analyze_control, ControlError, ControlFacts};
use crate::core::{
    lower_core_with_facts, validate_core, CallableStorageFact, CallableStorageKind, CoreExternAbi,
    CoreOp, CoreProgram, CoreValidation, CoreValue,
};
use crate::cps::{cps_program, CpsProgram};
use crate::cps_usage::{
    analyze_cps_usage, simplify_continuations, ContinuationSimplificationFacts, CpsUsageDiagnostic,
    CpsUsageFacts,
};
use crate::debug::{
    render_callable_storage_facts, render_visual_report, visual_report, VisualReport,
};
use crate::frontend::{parse_source_program, FrontendError, FrontendOutput, SourceItemSpan};
use crate::global::{analyze_global_init, GlobalInitDiagnostic, GlobalInitPlan, GlobalStaticId};
use crate::lambda_lift::{lift_lambdas, LambdaLiftFacts};
use crate::monomorphize::{schedule_monomorphization, MonomorphizationPlan};
use crate::nanopass::PassReport;
use crate::pattern::PatternFacts;
use crate::resolve::{
    resolve_expr, resolve_expr_with_names, MethodIndex, NameIndex, ResolveFacts, ResolvedName,
};
use crate::specialize::{plan_specialization, SpecializationFacts};
use crate::std_audit::{audit_std_dependencies, StdAuditReport};
use crate::surface::{
    build_interface_summary, duplicate_constructor_names, duplicate_data_names,
    duplicate_top_level_names, duplicate_type_fields, duplicate_type_names, project_surface,
    InterfaceConstructor, InterfaceSummary, ProjectSurface,
};
use crate::symbol::encode_debug_symbol;
use crate::template::{analyze_template_with_source, TemplateDiagnostic, TemplateFacts};
use crate::template_audit::{audit_checked_templates, TemplateAuditReport};
use crate::typed::{
    source_type_name_to_type, type_expr_with_context, RecordTypeField, Type, TypeContext, TypeEnv,
    TypedExpr,
};
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
    pub typed_signature: TypedSignature,
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypedSignature {
    pub params: Vec<TypedParam>,
    pub return_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedParam {
    pub name: String,
    pub pattern: crate::ast::Pattern,
    pub ty: String,
    pub binding_types: Vec<(String, Type)>,
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
    DuplicateDef {
        name: String,
    },
    DuplicateType {
        name: String,
    },
    DuplicateTypeField {
        type_name: String,
        field: String,
    },
    DuplicateData {
        name: String,
    },
    DuplicateConstructor {
        name: String,
    },
    DuplicateTopLevelName {
        name: String,
    },
    DuplicateStatic {
        name: String,
    },
    StaticFunctionNameConflict {
        name: String,
    },
    StaticInitCycle {
        cycle: Vec<String>,
    },
    InvalidStaticAdtConstructor {
        static_name: String,
        data: String,
        ctor: String,
    },
    UnsupportedStaticInitializer {
        static_name: String,
        expr: String,
    },
    ExplicitAutoGenericConflict {
        def: String,
        param: String,
    },
    ConflictingExplicitInstantiation {
        def: String,
        callee: String,
        previous_type_args: Vec<String>,
        type_args: Vec<String>,
    },
    ShiftOutsideReset {
        def: String,
        binder: String,
    },
    UnsafeMultiResumeCapture {
        def: String,
        binder: String,
    },
    Cont1ResumedMoreThanOnce {
        def: String,
        binder: String,
    },
    MissingEntry,
    EntryHasParams {
        name: String,
        params: Vec<String>,
    },
}

pub fn compile_expr(expr: &Expr) -> CompileOutput {
    let type_aliases = TypeAliasIndex::default();
    compile_expr_with_indexes_and_generics(
        "<expr>",
        expr,
        NameIndex::default(),
        MethodIndex::default(),
        &[],
        &[],
        &None,
        &None,
        &TypeContext::new(),
        "root",
        &type_aliases,
        &[],
    )
}

fn compile_expr_with_indexes_and_generics(
    def_name: &str,
    expr: &Expr,
    names: NameIndex,
    methods: MethodIndex,
    explicit_generics: &[String],
    params: &[ParamDecl],
    return_type: &Option<String>,
    receiver: &Option<MethodReceiver>,
    type_context: &TypeContext,
    current_namespace: &str,
    type_aliases: &TypeAliasIndex,
    extern_functions: &[crate::surface::InterfaceFunction],
) -> CompileOutput {
    let mut passes = PassReport::default();
    let alpha = passes.record("L1Alpha", "SourceExpr", "AlphaFacts", || {
        alpha_expr_with_params(expr, params)
    });
    let resolve = passes.record("L2Resolve", "AlphaExpr", "ResolveFacts", || {
        if names == NameIndex::default() && methods == MethodIndex::default() {
            resolve_expr(&alpha.expr, MethodIndex::default())
        } else {
            resolve_expr_with_names(&alpha.expr, methods.clone(), names.clone())
        }
    });
    let template = passes.record(
        "L3Template",
        "AlphaExpr+ResolveFacts",
        "TemplateFacts",
        || {
            analyze_template_with_source(
                expr,
                explicit_generics,
                params,
                return_type,
                &alpha.expr,
                &resolve,
            )
        },
    );
    let specialize = passes.record(
        "L4Specialize",
        "TemplateFacts",
        "SpecializationFacts",
        || plan_specialization("<expr>", &template),
    );
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
    let typed_signature = passes.record(
        "L7TypedSignature",
        "DefHeader+MethodReceiver",
        "TypedSignature",
        || {
            typed_signature(
                params,
                return_type,
                receiver,
                current_namespace,
                type_aliases,
                type_context,
            )
        },
    );
    let typed_env = typed_signature.type_env();
    let typed = passes.record("L7Typed", "SourceExpr+TypedSignature", "TypedExpr", || {
        type_expr_with_context(expr, &typed_env, type_context)
    });
    let pattern = passes.record(
        "L8PatternElab",
        "TypedExpr+ParamPatterns",
        "PatternFacts",
        || crate::pattern::analyze_patterns_with_params_and_context(&typed, params, type_context),
    );
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
    let explicit_callable_storage =
        explicit_callable_storage_facts(def_name, &typed_signature, &usage, &alpha.param_binders);
    let core = passes.record("L16Core", "CpsProgram", "CoreProgram", || {
        let mut core = lower_core_with_facts(
            &cps,
            &control.continuations,
            &closure,
            &explicit_callable_storage,
            &lambda_lift,
            &specialize,
            &usage,
        );
        attach_extern_function_targets(&mut core, &resolve, extern_functions);
        core
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
        || {
            let param_names = params
                .iter()
                .map(|param| param.name.clone())
                .collect::<Vec<_>>();
            emit_wasm_gc_with_params(&core, &core_validation, &param_names)
        },
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
        &typed_signature.render(def_name),
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
        &render_callable_storage_facts(&core.callable_storage),
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
        typed_signature,
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

pub fn compile_program_with_interface(
    program: &SourceProgram,
    interface: &InterfaceSummary,
) -> Vec<ProgramDefOutput> {
    compile_program_defs(&normalize_pattern_clause_defs(program), interface)
}

pub fn compile_source_program_bundle(source: &str) -> Result<SourceCompileOutput, FrontendError> {
    let frontend = parse_source_program(source)?;
    let mut program = compile_program_bundle(&frontend.program);
    attach_source_item_spans_to_visuals(&mut program, &frontend.item_spans);
    Ok(SourceCompileOutput { frontend, program })
}

fn attach_source_item_spans_to_visuals(
    program: &mut ProgramCompileOutput,
    spans: &[SourceItemSpan],
) {
    for def in &mut program.defs {
        let source_lines = spans
            .iter()
            .filter(|span| span.kind == "def" && span.name == def.name)
            .map(render_source_item_span)
            .collect::<Vec<_>>();
        if source_lines.is_empty() {
            continue;
        }
        def.output.visual.symbol_lineage = format!(
            "{}{}",
            source_lines.join(""),
            def.output.visual.symbol_lineage
        );
    }
}

fn render_source_item_span(span: &SourceItemSpan) -> String {
    format!(
        "source {} {} @ {}:{}..{}:{}\n",
        span.kind,
        span.name,
        span.span.line,
        span.span.column,
        span.span.end_line,
        span.span.end_column
    )
}

pub fn compile_program_bundle(program: &SourceProgram) -> ProgramCompileOutput {
    let normalized_program = normalize_pattern_clause_defs(program);
    let mut passes = PassReport::default();
    let surface = passes.record(
        "P1ProjectSurface",
        "SourceProgram",
        "ProjectSurface",
        || project_surface(&normalized_program),
    );
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
        || analyze_global_init(&normalized_program),
    );
    let mut defs = passes.record(
        "P5ProgramDefs",
        "SourceProgram+InterfaceSummary",
        "ProgramDefOutput",
        || compile_program_defs(&normalized_program, &interface),
    );
    let entry = passes.record(
        "P6ProgramEntry",
        "ProgramDefOutput",
        "EntrySelection",
        || select_program_entry(&defs),
    );
    let mut all_diagnostics = diagnostics;
    all_diagnostics.extend(global_init.diagnostics.iter().cloned().map(Into::into));
    all_diagnostics.extend(template_diagnostics(&defs));
    all_diagnostics.extend(control_diagnostics(&defs));
    all_diagnostics.extend(cps_usage_diagnostics(&defs));
    apply_program_backend_gates(&mut defs);
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
        "ProgramDefOutput+EntrySelection+GlobalInitPlan",
        "BackendLinkedBundle",
        || {
            let mut linked = link_backend_artifacts(program_backend_artifacts(
                &defs,
                entry.as_deref(),
                &global_init,
            ));
            attach_program_extern_imports(&mut linked, &interface);
            lower_global_init_into_linked_wat(linked, &global_init)
        },
    );
    let backend_cache_key = passes.record(
        "P8ProgramBackendCacheKey",
        "BackendLinkedBundle",
        "BackendCacheKey",
        || {
            backend_cache_key(
                &backend_link,
                &backend_cache_config_for_interface(&interface),
            )
        },
    );
    ProgramCompileOutput {
        namespace: normalized_program.namespace.clone(),
        imports: normalized_program.imports.clone(),
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

fn backend_cache_config_for_interface(interface: &InterfaceSummary) -> BackendCacheConfig {
    let mut config = BackendCacheConfig::default();
    config.imports = backend_extern_imports_for_interface(interface);
    config
}

fn attach_extern_function_targets(
    core: &mut CoreProgram,
    resolve: &ResolveFacts,
    extern_functions: &[crate::surface::InterfaceFunction],
) {
    let extern_by_symbol = extern_functions
        .iter()
        .filter(|function| function.extern_decl.is_some())
        .map(|function| (function.symbol.as_str(), function))
        .collect::<BTreeMap<_, _>>();
    let mut facts = resolve
        .resolved_names
        .iter()
        .filter_map(|name| {
            let ResolvedName::Function { name, symbol } = name else {
                return None;
            };
            let function = extern_by_symbol.get(symbol.as_str())?;
            let extern_decl = function.extern_decl.as_ref()?;
            Some(CoreOp::ExternFunctionTarget {
                target: name.clone(),
                owner: function.owner.clone(),
                symbol: function.symbol.clone(),
                abi: core_extern_abi(extern_decl.abi),
                name: extern_decl.symbol.clone(),
                signature: backend_extern_signature_hash(
                    &function.param_types,
                    &function.return_type,
                ),
            })
        })
        .collect::<Vec<_>>();
    facts.sort_by(|left, right| {
        render_core_extern_target_key(left).cmp(&render_core_extern_target_key(right))
    });
    facts.dedup_by(|left, right| {
        render_core_extern_target_key(left) == render_core_extern_target_key(right)
    });
    core.ops.splice(0..0, facts);
}

fn render_core_extern_target_key(op: &CoreOp) -> String {
    match op {
        CoreOp::ExternFunctionTarget {
            target,
            owner,
            symbol,
            abi,
            name,
            signature,
        } => format!(
            "{}|{}|{}|{}|{}|{}",
            target,
            owner,
            symbol,
            render_core_extern_abi(*abi),
            name,
            signature
        ),
        _ => String::new(),
    }
}

fn core_extern_abi(abi: ExternAbi) -> CoreExternAbi {
    match abi {
        ExternAbi::Wasi => CoreExternAbi::Wasi,
        ExternAbi::C => CoreExternAbi::C,
    }
}

fn render_core_extern_abi(abi: CoreExternAbi) -> &'static str {
    match abi {
        CoreExternAbi::Wasi => "wasi",
        CoreExternAbi::C => "c",
    }
}

fn attach_program_extern_imports(bundle: &mut BackendLinkedBundle, interface: &InterfaceSummary) {
    let mut imports = backend_extern_imports_for_interface(interface);
    if imports.is_empty() {
        return;
    }
    let existing_imports = bundle.manifest.imports.clone();
    bundle.manifest.imports.append(&mut imports);
    sort_dedup_imports(&mut bundle.manifest.imports);
    if bundle.diagnostics.is_empty() && !bundle.linked_wat.is_empty() {
        let mut comment_imports = bundle
            .manifest
            .imports
            .iter()
            .filter(|import| !extern_import_runtime_exists(import, &existing_imports))
            .cloned()
            .collect::<Vec<_>>();
        sort_dedup_imports(&mut comment_imports);
        insert_extern_import_comments(&mut bundle.linked_wat, &comment_imports);
    }
}

fn extern_import_runtime_exists(
    import: &BackendExternImport,
    existing_imports: &[BackendExternImport],
) -> bool {
    existing_imports.iter().any(|existing| {
        existing.abi == import.abi
            && existing.module == import.module
            && existing.name == import.name
            && existing.signature_hash == import.signature_hash
    })
}

fn backend_extern_imports_for_interface(interface: &InterfaceSummary) -> Vec<BackendExternImport> {
    let mut imports = interface
        .functions
        .iter()
        .filter_map(|function| {
            let extern_decl = function.extern_decl.as_ref()?;
            Some(BackendExternImport {
                abi: backend_extern_abi(extern_decl.abi),
                final_symbol: encode_debug_symbol(&function.symbol),
                module: backend_extern_module(extern_decl.abi).to_string(),
                name: extern_decl.symbol.clone(),
                signature_hash: backend_extern_signature_hash(
                    &function.param_types,
                    &function.return_type,
                ),
            })
        })
        .collect::<Vec<_>>();
    sort_dedup_imports(&mut imports);
    imports
}

fn insert_extern_import_comments(linked_wat: &mut String, imports: &[BackendExternImport]) {
    let Some(body) = linked_wat.strip_prefix("(module\n") else {
        return;
    };
    let mut next = String::from("(module\n");
    for import in imports {
        next.push_str(&format!(
            "  ;; extern-import {} symbol={} module={} name={} signature={}\n",
            render_backend_extern_abi(import.abi),
            import.final_symbol,
            import.module,
            import.name,
            import.signature_hash
        ));
    }
    next.push_str(body);
    *linked_wat = next;
}

fn render_backend_extern_abi(abi: BackendExternAbi) -> &'static str {
    match abi {
        BackendExternAbi::Wasi => "wasi",
        BackendExternAbi::C => "c",
    }
}

fn backend_extern_abi(abi: ExternAbi) -> BackendExternAbi {
    match abi {
        ExternAbi::Wasi => BackendExternAbi::Wasi,
        ExternAbi::C => BackendExternAbi::C,
    }
}

fn backend_extern_module(abi: ExternAbi) -> &'static str {
    match abi {
        ExternAbi::Wasi => "wasi_snapshot_preview1",
        ExternAbi::C => "env",
    }
}

fn backend_extern_signature_hash(
    params: &[Option<String>],
    return_type: &Option<String>,
) -> String {
    let params = params
        .iter()
        .map(|param| param.as_deref().unwrap_or("_"))
        .collect::<Vec<_>>()
        .join("_");
    format!("{}_to_{}", params, return_type.as_deref().unwrap_or("unit"))
}

fn normalize_pattern_clause_defs(program: &SourceProgram) -> SourceProgram {
    let mut normalized = SourceProgram::with_surface(
        program.namespace.clone(),
        program.imports.clone(),
        program.types.clone(),
        program.data.clone(),
        Vec::new(),
    );
    let mut items_by_key = BTreeMap::<ClauseKey, Vec<SourceItem>>::new();
    for item in &program.items {
        if let Some(key) = def_clause_key(item) {
            items_by_key.entry(key).or_default().push(item.clone());
        }
    }
    let merge_keys = items_by_key
        .iter()
        .filter_map(|(key, items)| clause_group_needs_dispatcher(items).then(|| key.clone()))
        .collect::<BTreeSet<_>>();
    let mut emitted_keys = BTreeSet::<ClauseKey>::new();

    for item in &program.items {
        let Some(key) = def_clause_key(item) else {
            normalized.items.push(item.clone());
            continue;
        };
        if !merge_keys.contains(&key) {
            normalized.items.push(item.clone());
            continue;
        }
        if emitted_keys.insert(key.clone()) {
            normalized
                .items
                .push(merge_clause_items(&items_by_key[&key]));
        }
    }

    normalized
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ClauseKey {
    name: String,
    receiver: Option<String>,
    arity: usize,
}

fn def_clause_key(item: &SourceItem) -> Option<ClauseKey> {
    let SourceItem::Def {
        name,
        receiver,
        params,
        ..
    } = item
    else {
        return None;
    };
    Some(ClauseKey {
        name: name.clone(),
        receiver: receiver.as_ref().map(MethodReceiver::display_name),
        arity: params.len(),
    })
}

fn clause_group_needs_dispatcher(items: &[SourceItem]) -> bool {
    items.len() > 1
        && items.iter().any(|item| match item {
            SourceItem::Def { params, .. } => params
                .iter()
                .any(|param| is_refutable_clause_pattern(&param.pattern)),
            SourceItem::StaticValue { .. } | SourceItem::ExternDef { .. } => false,
        })
}

fn is_refutable_clause_pattern(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Constructor { .. } | Pattern::Lit(_) => true,
        Pattern::Tuple(fields) => fields.iter().any(is_refutable_clause_pattern),
        Pattern::Record(fields) => fields
            .iter()
            .any(|field| is_refutable_clause_pattern(&field.pattern)),
        Pattern::At { pattern, .. } => is_refutable_clause_pattern(pattern),
        Pattern::Wildcard | Pattern::Bind(_) => false,
    }
}

fn merge_clause_items(clauses: &[SourceItem]) -> SourceItem {
    let SourceItem::Def {
        name,
        visibility,
        receiver,
        generics,
        params,
        return_type,
        ..
    } = &clauses[0]
    else {
        unreachable!("clause groups only contain def items");
    };
    let dispatcher_params = (0..params.len())
        .map(|index| {
            let ty = clauses.iter().find_map(|clause| match clause {
                SourceItem::Def { params, .. } => {
                    params.get(index).and_then(|param| param.ty.clone())
                }
                SourceItem::StaticValue { .. } | SourceItem::ExternDef { .. } => None,
            });
            ParamDecl::new(dispatcher_param_name(index), ty)
        })
        .collect::<Vec<_>>();
    let scrutinee = if dispatcher_params.len() == 1 {
        Expr::var(dispatcher_params[0].name.clone())
    } else {
        Expr::tuple(
            dispatcher_params
                .iter()
                .map(|param| Expr::var(param.name.clone()))
                .collect(),
        )
    };
    let arms = clauses
        .iter()
        .map(|clause| match clause {
            SourceItem::Def { params, body, .. } => {
                let pattern = if params.len() == 1 {
                    params[0].pattern.clone()
                } else {
                    Pattern::tuple(params.iter().map(|param| param.pattern.clone()).collect())
                };
                (pattern, body.clone())
            }
            SourceItem::StaticValue { .. } | SourceItem::ExternDef { .. } => {
                unreachable!("clause groups only contain def items")
            }
        })
        .collect();
    SourceItem::Def {
        name: name.clone(),
        visibility: *visibility,
        receiver: receiver.clone(),
        generics: generics.clone(),
        params: dispatcher_params,
        return_type: return_type.clone(),
        body: Expr::match_expr(scrutinee, arms),
    }
}

fn dispatcher_param_name(index: usize) -> String {
    if index == 0 {
        "value".to_string()
    } else {
        format!("value{}", index + 1)
    }
}

fn compile_program_defs(
    program: &SourceProgram,
    interface: &InterfaceSummary,
) -> Vec<ProgramDefOutput> {
    let current_namespace = program
        .namespace
        .as_ref()
        .map(NamespaceDecl::dotted)
        .unwrap_or_else(|| "root".to_string());
    let names = NameIndex::from_interface_for_namespace(interface, &current_namespace);
    let methods = MethodIndex::from_interface_for_namespace(interface, &current_namespace);
    let type_aliases = type_aliases_from_interface(interface);
    let type_context =
        type_context_from_interface(interface, &program.data, &current_namespace, &type_aliases);
    program
        .items
        .iter()
        .filter_map(|item| match item {
            SourceItem::Def {
                name,
                receiver,
                generics,
                params,
                return_type,
                body,
                ..
            } => Some(ProgramDefOutput {
                name: name.clone(),
                params: params.iter().map(|param| param.name.clone()).collect(),
                output: {
                    let mut output = compile_expr_with_indexes_and_generics(
                        name,
                        body,
                        names.clone(),
                        methods.clone(),
                        generics,
                        params,
                        return_type,
                        receiver,
                        &type_context,
                        &current_namespace,
                        &type_aliases,
                        &interface.functions,
                    );
                    output
                        .core
                        .callable_storage
                        .extend(interface_callable_storage_facts(interface));
                    output.visual.callable_storage =
                        render_callable_storage_facts(&output.core.callable_storage);
                    output
                },
            }),
            SourceItem::StaticValue { .. } | SourceItem::ExternDef { .. } => None,
        })
        .collect()
}

impl TypedSignature {
    pub fn render(&self, name: &str) -> String {
        let params = self
            .params
            .iter()
            .map(|param| format!("{}: {}", param.name, param.ty))
            .collect::<Vec<_>>()
            .join(", ");
        match &self.return_type {
            Some(return_type) => format!("def {name}({params}): {return_type}"),
            None => format!("def {name}({params})"),
        }
    }

    pub fn type_env(&self) -> TypeEnv {
        self.params
            .iter()
            .flat_map(|param| param.binding_types.clone())
            .collect()
    }
}

fn explicit_callable_storage_facts(
    def_name: &str,
    signature: &TypedSignature,
    usage: &UsageFacts,
    param_binders: &[AlphaBinder],
) -> Vec<CallableStorageFact> {
    let mut facts = signature
        .params
        .iter()
        .filter_map(|param| match source_type_name_to_type(&param.ty) {
            Type::Continuation { multi, .. } => Some(CallableStorageFact {
                subject: format!("param::{}", param.name),
                kind: if multi {
                    CallableStorageKind::ContNPackage
                } else {
                    CallableStorageKind::BoxedCont1
                },
                usage: if multi {
                    crate::typed::UsageColor::Many
                } else {
                    param_binders
                        .iter()
                        .find(|binder| binder.name == param.name)
                        .and_then(|binder| usage.binders.get(&binder.id).copied())
                        .unwrap_or(crate::usage::UseCount::Zero)
                        .color()
                },
                send: crate::typed::SendColor::NotSend,
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    if let Some(return_type) = &signature.return_type {
        if let Type::Continuation { multi, .. } = source_type_name_to_type(return_type) {
            facts.push(CallableStorageFact {
                subject: format!("return::{def_name}"),
                kind: if multi {
                    CallableStorageKind::ContNPackage
                } else {
                    CallableStorageKind::BoxedCont1
                },
                usage: if multi {
                    crate::typed::UsageColor::Many
                } else {
                    crate::typed::UsageColor::One
                },
                send: crate::typed::SendColor::NotSend,
            });
        }
    }
    facts
}

fn interface_callable_storage_facts(interface: &InterfaceSummary) -> Vec<CallableStorageFact> {
    interface
        .types
        .iter()
        .flat_map(|ty| {
            ty.fields
                .iter()
                .filter_map(|field| match source_type_name_to_type(&field.ty) {
                    Type::Continuation { multi, .. } => Some(CallableStorageFact {
                        subject: format!("type::{}::{}", ty.name, field.name),
                        kind: if multi {
                            CallableStorageKind::ContNPackage
                        } else {
                            CallableStorageKind::BoxedCont1
                        },
                        usage: if multi {
                            crate::typed::UsageColor::Many
                        } else {
                            crate::typed::UsageColor::One
                        },
                        send: crate::typed::SendColor::NotSend,
                    }),
                    _ => None,
                })
        })
        .collect()
}

fn typed_signature(
    params: &[ParamDecl],
    return_type: &Option<String>,
    receiver: &Option<MethodReceiver>,
    current_namespace: &str,
    type_aliases: &TypeAliasIndex,
    type_context: &TypeContext,
) -> TypedSignature {
    TypedSignature {
        params: params
            .iter()
            .map(|param| {
                let ty = resolve_header_type(
                    param.ty.as_deref(),
                    receiver,
                    current_namespace,
                    type_aliases,
                );
                TypedParam {
                    name: param.name.clone(),
                    pattern: param.pattern.clone(),
                    binding_types: type_context
                        .pattern_bindings_for(&param.pattern, &header_type_to_type(&ty)),
                    ty,
                }
            })
            .collect(),
        return_type: return_type
            .as_deref()
            .map(|ty| resolve_header_type(Some(ty), receiver, current_namespace, type_aliases)),
    }
}

fn resolve_header_type(
    ty: Option<&str>,
    receiver: &Option<MethodReceiver>,
    current_namespace: &str,
    type_aliases: &TypeAliasIndex,
) -> String {
    match ty {
        Some("Self") => receiver
            .as_ref()
            .map(MethodReceiver::display_name)
            .unwrap_or_else(|| "Self".to_string()),
        Some(ty) => type_aliases
            .resolve(current_namespace, ty)
            .unwrap_or_else(|| ty.to_string()),
        None => "Unknown".to_string(),
    }
}

fn header_type_to_type(ty: &str) -> Type {
    if ty == "Unknown" {
        Type::Unknown
    } else {
        source_type_name_to_type(ty)
    }
}

fn type_context_from_interface(
    interface: &InterfaceSummary,
    _data_decls: &[crate::ast::DataDecl],
    current_namespace: &str,
    type_aliases: &TypeAliasIndex,
) -> TypeContext {
    let mut context = TypeContext::new();
    for ty in visible_nominal_types(interface, current_namespace) {
        if ty.alias_target.is_some() {
            continue;
        }
        let fields = ty
            .fields
            .iter()
            .map(|field| RecordTypeField {
                name: field.name.clone(),
                ty: header_type_to_type(
                    &type_aliases
                        .resolve(&ty.owner, &field.ty)
                        .unwrap_or_else(|| field.ty.clone()),
                ),
            })
            .collect::<Vec<_>>();
        let display_name = nominal_display_name(&ty.name, &ty.generics);
        context.insert_nominal_row(display_name, fields.clone());
        if !ty.generics.is_empty() {
            context.insert_generic_nominal_row(&ty.name, ty.generics.clone(), fields);
        }
    }
    let data_generics = interface
        .data
        .iter()
        .map(|data| {
            (
                (data.owner.clone(), data.name.clone()),
                data.generics.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for ctor in visible_constructors(interface, current_namespace) {
        let generics = data_generics
            .get(&(ctor.owner.clone(), ctor.data.clone()))
            .cloned()
            .unwrap_or_default();
        context.insert_data_constructor(
            ctor.data.clone(),
            generics,
            ctor.name.clone(),
            ctor.payload_types
                .iter()
                .map(|field| {
                    header_type_to_type(
                        &type_aliases
                            .resolve(&ctor.owner, field)
                            .unwrap_or_else(|| field.clone()),
                    )
                })
                .collect(),
        );
    }
    context
}

fn visible_constructors<'a>(
    interface: &'a InterfaceSummary,
    current_namespace: &str,
) -> Vec<&'a InterfaceConstructor> {
    let mut by_data_ctor = BTreeMap::<(String, String), Vec<&InterfaceConstructor>>::new();
    for ctor in &interface.constructors {
        by_data_ctor
            .entry((ctor.data.clone(), ctor.name.clone()))
            .or_default()
            .push(ctor);
    }
    by_data_ctor
        .into_values()
        .filter_map(|candidates| {
            let local = candidates
                .iter()
                .copied()
                .filter(|candidate| candidate.owner == current_namespace)
                .collect::<Vec<_>>();
            match local.as_slice() {
                [candidate] => Some(*candidate),
                [] => match candidates.as_slice() {
                    [candidate] => Some(*candidate),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

fn visible_nominal_types<'a>(
    interface: &'a InterfaceSummary,
    current_namespace: &str,
) -> Vec<&'a crate::surface::InterfaceType> {
    let mut by_name = BTreeMap::<String, Vec<&crate::surface::InterfaceType>>::new();
    for ty in &interface.types {
        by_name
            .entry(nominal_display_name(&ty.name, &ty.generics))
            .or_default()
            .push(ty);
    }
    by_name
        .into_values()
        .filter_map(|candidates| {
            let local = candidates
                .iter()
                .copied()
                .filter(|candidate| candidate.owner == current_namespace)
                .collect::<Vec<_>>();
            match local.as_slice() {
                [candidate] => Some(*candidate),
                [] => match candidates.as_slice() {
                    [candidate] => Some(*candidate),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TypeAliasIndex {
    by_name: BTreeMap<String, Vec<TypeAliasCandidate>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TypeAliasCandidate {
    owner: String,
    target: String,
}

impl TypeAliasIndex {
    fn resolve(&self, current_namespace: &str, name: &str) -> Option<String> {
        let candidates = self.by_name.get(name)?;
        let local = candidates
            .iter()
            .filter(|candidate| candidate.owner == current_namespace)
            .collect::<Vec<_>>();
        match local.as_slice() {
            [candidate] => Some(candidate.target.clone()),
            [] => match candidates.as_slice() {
                [candidate] => Some(candidate.target.clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

fn type_aliases_from_interface(interface: &InterfaceSummary) -> TypeAliasIndex {
    let mut index = TypeAliasIndex::default();
    for ty in &interface.types {
        let Some(target) = &ty.alias_target else {
            continue;
        };
        index
            .by_name
            .entry(ty.name.clone())
            .or_default()
            .push(TypeAliasCandidate {
                owner: ty.owner.clone(),
                target: target.clone(),
            });
    }
    index
}

fn nominal_display_name(name: &str, generics: &[String]) -> String {
    if generics.is_empty() {
        name.to_string()
    } else {
        format!("{}[{}]", name, generics.join(","))
    }
}

pub fn program_surface_diagnostics(surface: &ProjectSurface) -> Vec<ProgramDiagnostic> {
    let mut seen = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for def in &surface.defs {
        let key = (
            def.owner.clone(),
            def.receiver.as_ref().map(MethodReceiver::display_name),
            def.name.clone(),
        );
        if !seen.insert(key) {
            diagnostics.push(ProgramDiagnostic::DuplicateDef {
                name: def.name.clone(),
            });
        }
    }
    diagnostics.extend(
        duplicate_type_names(surface)
            .into_iter()
            .map(|name| ProgramDiagnostic::DuplicateType { name }),
    );
    diagnostics.extend(
        duplicate_type_fields(surface)
            .into_iter()
            .map(|(type_name, field)| ProgramDiagnostic::DuplicateTypeField { type_name, field }),
    );
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
    diagnostics.extend(
        duplicate_top_level_names(surface)
            .into_iter()
            .map(|name| ProgramDiagnostic::DuplicateTopLevelName { name }),
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
            GlobalInitDiagnostic::InvalidStaticAdtConstructor {
                static_name,
                data,
                ctor,
            } => ProgramDiagnostic::InvalidStaticAdtConstructor {
                static_name,
                data,
                ctor,
            },
            GlobalInitDiagnostic::UnsupportedStaticInitializer { static_name, expr } => {
                ProgramDiagnostic::UnsupportedStaticInitializer { static_name, expr }
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

fn template_diagnostics(defs: &[ProgramDefOutput]) -> Vec<ProgramDiagnostic> {
    defs.iter()
        .flat_map(|def| {
            def.output
                .template
                .diagnostics
                .iter()
                .map(|diagnostic| match diagnostic {
                    TemplateDiagnostic::ExplicitAutoGenericConflict { param } => {
                        ProgramDiagnostic::ExplicitAutoGenericConflict {
                            def: def.name.clone(),
                            param: param.clone(),
                        }
                    }
                    TemplateDiagnostic::ConflictingExplicitInstantiation {
                        callee,
                        previous_type_args,
                        type_args,
                    } => ProgramDiagnostic::ConflictingExplicitInstantiation {
                        def: def.name.clone(),
                        callee: callee.clone(),
                        previous_type_args: previous_type_args.clone(),
                        type_args: type_args.clone(),
                    },
                })
        })
        .collect()
}

fn control_diagnostics(defs: &[ProgramDefOutput]) -> Vec<ProgramDiagnostic> {
    defs.iter()
        .flat_map(|def| {
            def.output
                .control
                .errors
                .iter()
                .map(|diagnostic| match diagnostic {
                    ControlError::ShiftOutsideReset { binder } => {
                        ProgramDiagnostic::ShiftOutsideReset {
                            def: def.name.clone(),
                            binder: binder.clone(),
                        }
                    }
                    ControlError::UnsafeMultiResumeCapture { binder } => {
                        ProgramDiagnostic::UnsafeMultiResumeCapture {
                            def: def.name.clone(),
                            binder: binder.clone(),
                        }
                    }
                })
        })
        .collect()
}

fn cps_usage_diagnostics(defs: &[ProgramDefOutput]) -> Vec<ProgramDiagnostic> {
    defs.iter()
        .flat_map(|def| {
            def.output
                .cps_usage
                .diagnostics
                .iter()
                .map(|diagnostic| match diagnostic {
                    CpsUsageDiagnostic::Cont1ResumedMoreThanOnce { binder, .. } => {
                        ProgramDiagnostic::Cont1ResumedMoreThanOnce {
                            def: def.name.clone(),
                            binder: binder.clone(),
                        }
                    }
                })
        })
        .collect()
}

fn apply_program_backend_gates(defs: &mut [ProgramDefOutput]) {
    for def in defs {
        for diagnostic in &def.output.cps_usage.diagnostics {
            match diagnostic {
                CpsUsageDiagnostic::Cont1ResumedMoreThanOnce { binder, .. } => {
                    def.output.backend.wat.clear();
                    def.output.backend.diagnostics.retain(|diagnostic| {
                        !matches!(
                            diagnostic,
                            BackendDiagnostic::UnsupportedContinuationRuntime {
                                op,
                                kind: crate::control::ContinuationKind::Cont1,
                                binder: Some(existing),
                            } if op == "resume-continuation" && existing == binder
                        )
                    });
                    if !def.output.backend.diagnostics.iter().any(|diagnostic| {
                        matches!(
                            diagnostic,
                            BackendDiagnostic::Cont1ResumedMoreThanOnce { binder: existing }
                                if existing == binder
                        )
                    }) {
                        def.output.backend.diagnostics.push(
                            BackendDiagnostic::Cont1ResumedMoreThanOnce {
                                binder: binder.clone(),
                            },
                        );
                    }
                }
            }
        }
    }
}

fn program_backend_artifacts(
    defs: &[ProgramDefOutput],
    entry: Option<&str>,
    global_init: &GlobalInitPlan,
) -> Vec<BackendArtifact> {
    let mut entry_exported = false;
    let static_names = if global_init.diagnostics.is_empty() {
        global_init
            .statics
            .iter()
            .map(|static_value| {
                (
                    static_value.name.as_str(),
                    global_symbol(&static_value.owner, &static_value.name),
                )
            })
            .collect::<BTreeMap<_, _>>()
    } else {
        BTreeMap::new()
    };
    let mut def_name_counts = BTreeMap::new();
    for def in defs {
        *def_name_counts.entry(def.name.as_str()).or_insert(0usize) += 1;
    }
    defs.iter()
        .enumerate()
        .map(|(index, def)| {
            let mut artifact = def.output.backend.clone();
            let is_entry = entry == Some(def.name.as_str()) && !entry_exported;
            if is_entry {
                entry_exported = true;
            }
            let symbol = def_symbol(
                &def.name,
                index,
                def_name_counts
                    .get(def.name.as_str())
                    .copied()
                    .unwrap_or_default(),
            );
            if let Some(static_wat) =
                static_return_wat_from_fact(&artifact, &symbol, &static_names, is_entry)
            {
                artifact.wat = static_wat;
                artifact.diagnostics.clear();
            }
            artifact.wat = relabel_program_wat(&artifact, &symbol, is_entry);
            artifact
        })
        .collect()
}

fn static_return_wat_from_fact(
    artifact: &BackendArtifact,
    symbol: &str,
    static_symbols: &BTreeMap<&str, String>,
    is_entry: bool,
) -> Option<String> {
    let Some(CoreValue::Var(name)) = artifact.return_value.as_ref() else {
        return None;
    };
    if !is_entry {
        return None;
    }
    let static_symbol = static_symbols.get(name.as_str())?;
    Some(format!(
        "(module\n  (func ${symbol} (export \"main\") (result i32)\n    global.get ${}\n  )\n)\n",
        static_symbol
    ))
}

fn relabel_program_wat(artifact: &BackendArtifact, symbol: &str, is_entry: bool) -> String {
    let mut wat = if is_entry {
        artifact.wat.replace(
            "(func $main (export \"main\")",
            &format!("(func ${symbol} (export \"main\")"),
        )
    } else {
        artifact
            .wat
            .replace("(func $main (export \"main\")", &format!("(func ${symbol}"))
    };
    if !is_entry {
        wat = wat.replace("(func $chiba_tailcall_0", &format!("(func ${symbol}"));
        wat = wat.replace("(func $chiba_return_0", &format!("(func ${symbol}"));
        wat = wat.replace(
            "(func $chiba_tailcall_",
            &format!("(func ${symbol}__chiba_tailcall_"),
        );
        wat = wat.replace(
            "(func $chiba_return_",
            &format!("(func ${symbol}__chiba_return_"),
        );
    } else {
        wat = wat.replace(
            "(func $chiba_tailcall_0 (result i32)",
            &format!("(func ${symbol} (export \"main\") (result i32)"),
        );
        wat = wat.replace(
            "(func $chiba_return_0 (result i32)",
            &format!("(func ${symbol} (export \"main\") (result i32)"),
        );
    }
    wat
}

fn def_symbol(name: &str, index: usize, name_count: usize) -> String {
    let base = sanitize_program_symbol(name);
    if name_count <= 1 || index == 0 {
        base
    } else {
        format!("{base}__def{index}")
    }
}

fn sanitize_program_symbol(name: &str) -> String {
    encode_debug_symbol(name)
}

fn lower_global_init_into_linked_wat(
    mut bundle: BackendLinkedBundle,
    global_init: &GlobalInitPlan,
) -> BackendLinkedBundle {
    if !bundle.diagnostics.is_empty()
        || !global_init.diagnostics.is_empty()
        || global_init.statics.is_empty()
    {
        return bundle;
    }
    let Some(body) = bundle.linked_wat.strip_prefix("(module\n") else {
        return bundle;
    };
    let Some(body) = body.strip_suffix(")\n") else {
        return bundle;
    };

    let mut wat = String::from("(module\n");
    for static_value in &global_init.statics {
        let initializer = global_storage_initializer(&static_value.body);
        wat.push_str(&format!(
            "  (global ${} (mut i32) (i32.const {}))\n",
            global_symbol(&static_value.owner, &static_value.name),
            initializer
        ));
    }
    wat.push_str("  (func $__chiba_init\n");
    for id in &global_init.init_order_ids {
        let Some(static_value) = global_init
            .statics
            .iter()
            .find(|item| item.owner == id.owner && item.name == id.name)
        else {
            continue;
        };
        if const_global_initializer(&static_value.body).is_some() {
            continue;
        }
        wat.push_str(&format!("    ;; init static {}\n", static_value.name));
        if let Err(diagnostic) = render_global_init_expr(
            &mut wat,
            &static_value.name,
            &static_value.body,
            global_init,
        ) {
            bundle.diagnostics.push(diagnostic);
            return bundle;
        }
        wat.push_str(&format!(
            "    global.set ${}\n",
            global_symbol(&static_value.owner, &static_value.name)
        ));
    }
    wat.push_str("  )\n");
    wat.push_str("  (start $__chiba_init)\n");
    wat.push_str(body);
    wat.push_str(")\n");
    bundle.linked_wat = wat;
    bundle
}

fn global_storage_initializer(expr: &Expr) -> i32 {
    const_global_initializer(expr).unwrap_or(RUNTIME_INITIALIZED_GLOBAL_SENTINEL)
}

const RUNTIME_INITIALIZED_GLOBAL_SENTINEL: i32 = 0;

fn const_global_initializer(expr: &Expr) -> Option<i32> {
    match expr {
        Expr::Lit(crate::ast::Literal::I64(value)) => Some(*value as i32),
        Expr::Lit(crate::ast::Literal::Bool(value)) => Some(i32::from(*value)),
        Expr::Binary { op, lhs, rhs } => {
            let lhs = const_global_initializer(lhs)?;
            let rhs = const_global_initializer(rhs)?;
            match op {
                crate::ast::BinaryOp::Add => lhs.checked_add(rhs),
                crate::ast::BinaryOp::Sub => lhs.checked_sub(rhs),
                crate::ast::BinaryOp::Mul => lhs.checked_mul(rhs),
                crate::ast::BinaryOp::Div => match rhs {
                    0 => None,
                    divisor => Some(lhs / divisor),
                },
            }
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => match const_global_initializer(cond)? {
            0 => const_global_initializer(else_branch),
            _ => const_global_initializer(then_branch),
        },
        Expr::AdtCtor { ctor, variants, .. } => variants
            .iter()
            .position(|variant| variant == ctor)
            .map(|tag| tag as i32),
        _ => None,
    }
}

fn render_global_init_expr(
    wat: &mut String,
    static_name: &str,
    expr: &Expr,
    global_init: &GlobalInitPlan,
) -> Result<(), BackendLinkDiagnostic> {
    render_global_init_expr_with_bindings(wat, static_name, expr, global_init, &BTreeMap::new())
}

fn render_global_init_expr_with_bindings(
    wat: &mut String,
    static_name: &str,
    expr: &Expr,
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
) -> Result<(), BackendLinkDiagnostic> {
    match expr {
        Expr::Lit(crate::ast::Literal::I64(value)) => {
            wat.push_str(&format!("    i32.const {}\n", *value as i32));
        }
        Expr::Lit(crate::ast::Literal::Bool(value)) => {
            wat.push_str(&format!("    i32.const {}\n", i32::from(*value)));
        }
        Expr::Var(name) if bindings.contains_key(name) => {
            render_global_init_expr_with_bindings(
                wat,
                static_name,
                bindings.get(name).expect("checked binding"),
                global_init,
                bindings,
            )?;
        }
        Expr::Var(name) => {
            let Some(static_value) = global_init.statics.iter().find(|item| item.name == *name)
            else {
                return Err(
                    BackendLinkDiagnostic::UnsupportedStaticInitializerLowering {
                        static_name: static_name.to_string(),
                        expr: render_source_expr(expr),
                    },
                );
            };
            wat.push_str(&format!(
                "    global.get ${}\n",
                global_symbol(&static_value.owner, &static_value.name)
            ));
        }
        Expr::Binary { op, lhs, rhs } => {
            render_global_init_expr_with_bindings(wat, static_name, lhs, global_init, bindings)?;
            render_global_init_expr_with_bindings(wat, static_name, rhs, global_init, bindings)?;
            wat.push_str(match op {
                crate::ast::BinaryOp::Add => "    i32.add\n",
                crate::ast::BinaryOp::Sub => "    i32.sub\n",
                crate::ast::BinaryOp::Mul => "    i32.mul\n",
                crate::ast::BinaryOp::Div => "    i32.div_s\n",
            });
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            render_global_init_expr_with_bindings(wat, static_name, cond, global_init, bindings)?;
            wat.push_str("    if (result i32)\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                then_branch,
                global_init,
                bindings,
                6,
            )?;
            wat.push_str("    else\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                else_branch,
                global_init,
                bindings,
                6,
            )?;
            wat.push_str("    end\n");
        }
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            render_global_if_let_expr(
                wat,
                static_name,
                pattern,
                scrutinee,
                then_branch,
                else_branch,
                global_init,
                bindings,
                4,
            )?;
        }
        Expr::Match { scrutinee, arms } => {
            render_global_match_expr(wat, static_name, scrutinee, arms, global_init, bindings, 4)?;
        }
        Expr::Field { receiver, name, .. } => {
            if let Some(value) = global_record_field_expr(receiver, name, bindings) {
                render_global_init_expr_with_bindings(
                    wat,
                    static_name,
                    value,
                    global_init,
                    bindings,
                )?;
            } else {
                return Err(
                    BackendLinkDiagnostic::UnsupportedStaticInitializerLowering {
                        static_name: static_name.to_string(),
                        expr: render_source_expr(expr),
                    },
                );
            }
        }
        Expr::AdtCtor { ctor, variants, .. } => {
            let Some(tag) = variants.iter().position(|variant| variant == ctor) else {
                return Err(
                    BackendLinkDiagnostic::UnsupportedStaticInitializerLowering {
                        static_name: static_name.to_string(),
                        expr: render_source_expr(expr),
                    },
                );
            };
            wat.push_str(&format!("    i32.const {tag}\n"));
        }
        other => {
            return Err(
                BackendLinkDiagnostic::UnsupportedStaticInitializerLowering {
                    static_name: static_name.to_string(),
                    expr: render_source_expr(other),
                },
            );
        }
    }
    Ok(())
}

fn render_global_init_expr_nested_with_bindings(
    wat: &mut String,
    static_name: &str,
    expr: &Expr,
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
    indent: usize,
) -> Result<(), BackendLinkDiagnostic> {
    let mut nested = String::new();
    render_global_init_expr_with_bindings(&mut nested, static_name, expr, global_init, bindings)?;
    for line in nested.lines() {
        wat.push_str(&" ".repeat(indent));
        wat.push_str(line.trim_start());
        wat.push('\n');
    }
    Ok(())
}

fn global_record_field_expr<'a>(
    receiver: &'a Expr,
    name: &str,
    bindings: &'a BTreeMap<String, Expr>,
) -> Option<&'a Expr> {
    match receiver {
        Expr::Var(binding) => {
            let value = bindings.get(binding)?;
            global_record_field_expr(value, name, bindings)
        }
        Expr::Record(fields) => fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.value),
        Expr::RecordUpdate { base, fields } => fields
            .iter()
            .rev()
            .find(|field| field.name == name)
            .map(|field| &field.value)
            .or_else(|| global_record_field_expr(base, name, bindings)),
        _ => None,
    }
}

fn render_global_match_expr(
    wat: &mut String,
    static_name: &str,
    scrutinee: &Expr,
    arms: &[crate::ast::MatchArm],
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
    indent: usize,
) -> Result<(), BackendLinkDiagnostic> {
    let Some((first, rest)) = arms.split_first() else {
        push_global_indent(wat, indent);
        wat.push_str("unreachable\n");
        return Ok(());
    };
    render_global_match_arm(
        wat,
        static_name,
        scrutinee,
        first,
        rest,
        global_init,
        bindings,
        indent,
    )
}

fn render_global_if_let_expr(
    wat: &mut String,
    static_name: &str,
    pattern: &Pattern,
    scrutinee: &Expr,
    then_branch: &Expr,
    else_branch: &Expr,
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
    indent: usize,
) -> Result<(), BackendLinkDiagnostic> {
    match global_pattern_bindings(pattern, scrutinee, bindings) {
        Some(GlobalPatternMatch::Always(next_bindings)) => {
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                then_branch,
                global_init,
                &next_bindings,
                indent,
            )?;
        }
        Some(GlobalPatternMatch::Conditional {
            expected,
            bindings: next_bindings,
        }) => {
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                scrutinee,
                global_init,
                bindings,
                indent,
            )?;
            push_global_indent(wat, indent);
            wat.push_str(&format!("i32.const {expected}\n"));
            push_global_indent(wat, indent);
            wat.push_str("i32.eq\n");
            push_global_indent(wat, indent);
            wat.push_str("if (result i32)\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                then_branch,
                global_init,
                &next_bindings,
                indent + 2,
            )?;
            push_global_indent(wat, indent);
            wat.push_str("else\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                else_branch,
                global_init,
                bindings,
                indent + 2,
            )?;
            push_global_indent(wat, indent);
            wat.push_str("end\n");
        }
        None => {
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                else_branch,
                global_init,
                bindings,
                indent,
            )?;
        }
    }
    Ok(())
}

fn render_global_match_arm(
    wat: &mut String,
    static_name: &str,
    scrutinee: &Expr,
    arm: &crate::ast::MatchArm,
    rest: &[crate::ast::MatchArm],
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
    indent: usize,
) -> Result<(), BackendLinkDiagnostic> {
    match global_pattern_bindings(&arm.pattern, scrutinee, bindings) {
        Some(GlobalPatternMatch::Always(next_bindings)) => {
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                &arm.body,
                global_init,
                &next_bindings,
                indent,
            )?;
        }
        Some(GlobalPatternMatch::Conditional {
            expected,
            bindings: next_bindings,
        }) => {
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                scrutinee,
                global_init,
                bindings,
                indent,
            )?;
            push_global_indent(wat, indent);
            wat.push_str(&format!("i32.const {expected}\n"));
            push_global_indent(wat, indent);
            wat.push_str("i32.eq\n");
            push_global_indent(wat, indent);
            wat.push_str("if (result i32)\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                static_name,
                &arm.body,
                global_init,
                &next_bindings,
                indent + 2,
            )?;
            push_global_indent(wat, indent);
            wat.push_str("else\n");
            render_global_match_expr(
                wat,
                static_name,
                scrutinee,
                rest,
                global_init,
                bindings,
                indent + 2,
            )?;
            push_global_indent(wat, indent);
            wat.push_str("end\n");
        }
        None => {
            render_global_match_expr(
                wat,
                static_name,
                scrutinee,
                rest,
                global_init,
                bindings,
                indent,
            )?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GlobalPatternMatch {
    Always(BTreeMap<String, Expr>),
    Conditional {
        expected: i32,
        bindings: BTreeMap<String, Expr>,
    },
}

fn global_pattern_bindings(
    pattern: &Pattern,
    value: &Expr,
    bindings: &BTreeMap<String, Expr>,
) -> Option<GlobalPatternMatch> {
    match pattern {
        Pattern::Wildcard => Some(GlobalPatternMatch::Always(bindings.clone())),
        Pattern::Bind(name) => {
            let mut next = bindings.clone();
            next.insert(name.clone(), value.clone());
            Some(GlobalPatternMatch::Always(next))
        }
        Pattern::Lit(crate::ast::Literal::I64(expected)) => Some(GlobalPatternMatch::Conditional {
            expected: *expected as i32,
            bindings: bindings.clone(),
        }),
        Pattern::Lit(crate::ast::Literal::Rune(expected)) => {
            Some(GlobalPatternMatch::Conditional {
                expected: *expected as i32,
                bindings: bindings.clone(),
            })
        }
        Pattern::Lit(crate::ast::Literal::Bool(expected)) => {
            Some(GlobalPatternMatch::Conditional {
                expected: i32::from(*expected),
                bindings: bindings.clone(),
            })
        }
        Pattern::Lit(crate::ast::Literal::String(_)) => None,
        Pattern::Constructor { data, ctor, args } => {
            let Expr::AdtCtor {
                data: value_data,
                variants,
                args: payloads,
                ..
            } = value
            else {
                return None;
            };
            if data
                .as_ref()
                .is_some_and(|pattern_data| pattern_data != value_data)
            {
                return None;
            }
            if args.len() != payloads.len() {
                return None;
            }
            let expected = variants.iter().position(|variant| variant == ctor)? as i32;
            let mut next = bindings.clone();
            for (pattern, payload) in args.iter().zip(payloads) {
                match global_pattern_bindings(pattern, payload, &next)? {
                    GlobalPatternMatch::Always(updated) => next = updated,
                    GlobalPatternMatch::Conditional { .. } => return None,
                }
            }
            Some(GlobalPatternMatch::Conditional {
                expected,
                bindings: next,
            })
        }
        Pattern::At { name, pattern } => match global_pattern_bindings(pattern, value, bindings)? {
            GlobalPatternMatch::Always(mut next) => {
                next.insert(name.clone(), value.clone());
                Some(GlobalPatternMatch::Always(next))
            }
            GlobalPatternMatch::Conditional {
                expected,
                bindings: mut next,
            } => {
                next.insert(name.clone(), value.clone());
                Some(GlobalPatternMatch::Conditional {
                    expected,
                    bindings: next,
                })
            }
        },
        Pattern::Tuple(_) | Pattern::Record(_) => None,
    }
}

fn push_global_indent(wat: &mut String, indent: usize) {
    wat.push_str(&" ".repeat(indent));
}

fn global_symbol(owner: &str, name: &str) -> String {
    if owner == "root" {
        format!("global__{}", encode_debug_symbol(name))
    } else {
        format!(
            "global__{}__{}",
            encode_debug_symbol(owner),
            encode_debug_symbol(name)
        )
    }
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
            "  imports={}\n",
            render_imports_summary(self.imports.iter().map(UseDecl::dotted))
        ));
        out.push_str(&format!("  defs={}\n", self.defs.len()));
        out.push_str(&render_global_init_summary(&self.global_init));
        out.push_str(&render_project_surface_summary(&self.surface));
        out.push_str(&render_interface_summary_summary(&self.interface));
        out.push_str(&format!("  entry={}\n", render_program_entry(&self.entry)));
        out.push_str(&render_program_diagnostics_summary(&self.diagnostics));
        out.push_str(&render_program_backend_link_summary(&self.backend_link));
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

fn render_program_backend_link_summary(bundle: &BackendLinkedBundle) -> String {
    let mut out = String::new();
    out.push_str("  backend-link:\n");
    out.push_str(&format!(
        "    linked-wat-lines={}\n",
        bundle.linked_wat.lines().count()
    ));
    out.push_str(&format!(
        "    manifest-imports={}\n",
        bundle.manifest.imports.len()
    ));
    for import in &bundle.manifest.imports {
        out.push_str(&format!(
            "      import {} symbol={} module={} name={} signature={}\n",
            render_backend_extern_abi(import.abi),
            import.final_symbol,
            import.module,
            import.name,
            import.signature_hash
        ));
    }
    out.push_str(&format!(
        "    manifest-entries={}\n",
        bundle.manifest.entries.len()
    ));
    out.push_str(&format!("    diagnostics={}\n", bundle.diagnostics.len()));
    for diagnostic in &bundle.diagnostics {
        out.push_str(&format!(
            "      diagnostic {}\n",
            render_backend_link_diagnostic(diagnostic)
        ));
    }
    out
}

fn render_backend_link_diagnostic(diagnostic: &BackendLinkDiagnostic) -> String {
    match diagnostic {
        BackendLinkDiagnostic::ArtifactEmitFailed { artifact_index } => {
            format!("artifact emit failed {artifact_index}")
        }
        BackendLinkDiagnostic::DuplicateFinalSymbol { symbol } => {
            format!("duplicate final symbol {symbol}")
        }
        BackendLinkDiagnostic::UnsupportedStaticInitializerLowering { static_name, expr } => {
            format!("unsupported static initializer lowering {static_name}: {expr}")
        }
        BackendLinkDiagnostic::TargetMismatch {
            artifact_index,
            expected,
            actual,
        } => format!(
            "target mismatch {artifact_index}: expected {}, actual {}",
            render_backend_target(*expected),
            render_backend_target(*actual)
        ),
    }
}

fn render_backend_target(target: crate::backend::BackendTarget) -> &'static str {
    match target {
        crate::backend::BackendTarget::WasmGc => "wasm-gc",
    }
}

fn render_imports_summary(imports: impl Iterator<Item = String>) -> String {
    let imports = imports.collect::<Vec<_>>();
    if imports.is_empty() {
        "<none>".to_string()
    } else {
        imports.join(", ")
    }
}

fn render_project_surface_summary(surface: &ProjectSurface) -> String {
    let mut out = String::new();
    out.push_str("  surface:\n");
    out.push_str(&format!("    namespace={}\n", surface.namespace));
    out.push_str(&format!(
        "    imports={}\n",
        render_imports_summary(surface.imports.iter().cloned())
    ));
    out.push_str(&format!("    defs={}\n", surface.defs.len()));
    for def in &surface.defs {
        out.push_str(&format!(
            "      def {}::{} visibility={} receiver={} generics={} arity={} params={} return={} extern={}\n",
            def.owner,
            def.name,
            render_visibility(def.visibility),
            render_receiver(&def.receiver),
            render_string_values(&def.generics),
            def.arity,
            render_optional_string_values(&def.param_types),
            render_optional_string(&def.return_type),
            render_extern_decl(&def.extern_decl)
        ));
    }
    out.push_str(&format!("    statics={}\n", surface.statics.len()));
    for static_value in &surface.statics {
        out.push_str(&format!(
            "      static {}::{} visibility={} type={}\n",
            static_value.owner,
            static_value.name,
            render_visibility(static_value.visibility),
            render_optional_string(&static_value.ty)
        ));
    }
    out.push_str(&format!("    types={}\n", surface.types.len()));
    for ty in &surface.types {
        out.push_str(&format!(
            "      type {}::{} generics={} alias={} fields={} phantoms={}\n",
            ty.owner,
            ty.name,
            render_string_values(&ty.generics),
            render_optional_string(&ty.alias_target),
            render_surface_type_fields(&ty.fields),
            render_string_values(&ty.phantom_markers)
        ));
    }
    out.push_str(&format!("    data={}\n", surface.data.len()));
    for data in &surface.data {
        out.push_str(&format!(
            "      data {}::{} generics={} variants={}\n",
            data.owner,
            data.name,
            render_string_values(&data.generics),
            render_string_values(&data.variants)
        ));
    }
    out.push_str(&format!(
        "    constructors={}\n",
        surface.constructors.len()
    ));
    for ctor in &surface.constructors {
        out.push_str(&format!(
            "      constructor {}::{}.{} arity={} payloads={}\n",
            ctor.owner,
            ctor.data,
            ctor.name,
            ctor.arity,
            render_string_values(&ctor.payload_types)
        ));
    }
    out
}

fn render_interface_summary_summary(interface: &InterfaceSummary) -> String {
    let mut out = String::new();
    out.push_str("  interface:\n");
    out.push_str(&format!(
        "    namespace={} hash={}\n",
        interface.namespace, interface.stable_hash
    ));
    out.push_str(&format!(
        "    imports={}\n",
        render_imports_summary(interface.imports.iter().cloned())
    ));
    out.push_str(&format!("    functions={}\n", interface.functions.len()));
    for function in &interface.functions {
        out.push_str(&format!(
            "      function {} owner={} source={} visibility={} receiver={} generics={} arity={} params={} return={} extern={}\n",
            function.symbol,
            function.owner,
            function.source_name,
            render_visibility(function.visibility),
            render_receiver(&function.receiver),
            render_string_values(&function.generics),
            function.arity,
            render_optional_string_values(&function.param_types),
            render_optional_string(&function.return_type),
            render_extern_decl(&function.extern_decl)
        ));
    }
    out.push_str(&format!("    statics={}\n", interface.statics.len()));
    for static_value in &interface.statics {
        out.push_str(&format!(
            "      static {} owner={} source={} visibility={} type={}\n",
            static_value.symbol,
            static_value.owner,
            static_value.source_name,
            render_visibility(static_value.visibility),
            render_optional_string(&static_value.ty)
        ));
    }
    out.push_str(&format!("    types={}\n", interface.types.len()));
    for ty in &interface.types {
        out.push_str(&format!(
            "      type {} owner={} source={} generics={} alias={} fields={} phantoms={}\n",
            ty.symbol,
            ty.owner,
            ty.name,
            render_string_values(&ty.generics),
            render_optional_string(&ty.alias_target),
            render_interface_type_fields(&ty.fields),
            render_string_values(&ty.phantom_markers)
        ));
    }
    out.push_str(&format!("    data={}\n", interface.data.len()));
    for data in &interface.data {
        out.push_str(&format!(
            "      data {} owner={} source={} generics={} variants={}\n",
            data.symbol,
            data.owner,
            data.name,
            render_string_values(&data.generics),
            render_string_values(&data.variants)
        ));
    }
    out.push_str(&format!(
        "    constructors={}\n",
        interface.constructors.len()
    ));
    for ctor in &interface.constructors {
        out.push_str(&format!(
            "      constructor {} data={} owner={} source={}.{} arity={} payloads={}\n",
            ctor.symbol,
            ctor.data_symbol,
            ctor.owner,
            ctor.data,
            ctor.name,
            ctor.arity,
            render_string_values(&ctor.payload_types)
        ));
    }
    out
}

fn render_surface_type_fields(fields: &[crate::surface::SurfaceTypeField]) -> String {
    render_string_values(
        &fields
            .iter()
            .map(|field| format!("{}: {}", field.name, field.ty))
            .collect::<Vec<_>>(),
    )
}

fn render_interface_type_fields(fields: &[crate::surface::InterfaceTypeField]) -> String {
    render_string_values(
        &fields
            .iter()
            .map(|field| format!("{}: {}", field.name, field.ty))
            .collect::<Vec<_>>(),
    )
}

fn render_visibility(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Public => "public",
        Visibility::Private => "private",
    }
}

fn render_receiver(receiver: &Option<MethodReceiver>) -> String {
    match receiver {
        Some(receiver) => {
            format!(
                "{}{}",
                receiver.name,
                render_generics_suffix(&receiver.generics)
            )
        }
        None => "<none>".to_string(),
    }
}

fn render_generics_suffix(generics: &[String]) -> String {
    if generics.is_empty() {
        String::new()
    } else {
        format!("[{}]", generics.join(", "))
    }
}

fn render_optional_string(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("<inferred>")
}

fn render_optional_string_values(values: &[Option<String>]) -> String {
    render_string_values(
        &values
            .iter()
            .map(|value| render_optional_string(value).to_string())
            .collect::<Vec<_>>(),
    )
}

fn render_string_values(values: &[String]) -> String {
    if values.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", values.join(", "))
    }
}

fn render_extern_decl(extern_decl: &Option<ExternDecl>) -> String {
    match extern_decl {
        Some(extern_decl) => format!(
            "{}:{}",
            render_extern_abi(extern_decl.abi),
            extern_decl.symbol
        ),
        None => "<none>".to_string(),
    }
}

fn render_extern_abi(abi: ExternAbi) -> &'static str {
    match abi {
        ExternAbi::Wasi => "wasi",
        ExternAbi::C => "c",
    }
}

fn render_program_entry(entry: &Option<String>) -> &str {
    entry.as_deref().unwrap_or("<missing>")
}

fn render_program_diagnostics_summary(diagnostics: &[ProgramDiagnostic]) -> String {
    let mut out = String::new();
    out.push_str(&format!("  diagnostics={}\n", diagnostics.len()));
    for diagnostic in diagnostics {
        out.push_str(&format!("    {}\n", render_program_diagnostic(diagnostic)));
    }
    out
}

fn render_program_diagnostic(diagnostic: &ProgramDiagnostic) -> String {
    match diagnostic {
        ProgramDiagnostic::DuplicateDef { name } => format!("duplicate def {name}"),
        ProgramDiagnostic::DuplicateType { name } => format!("duplicate type {name}"),
        ProgramDiagnostic::DuplicateTypeField { type_name, field } => {
            format!("duplicate type field {type_name}.{field}")
        }
        ProgramDiagnostic::DuplicateData { name } => format!("duplicate data {name}"),
        ProgramDiagnostic::DuplicateConstructor { name } => {
            format!("duplicate constructor {name}")
        }
        ProgramDiagnostic::DuplicateTopLevelName { name } => {
            format!("duplicate top-level name {name}")
        }
        ProgramDiagnostic::DuplicateStatic { name } => format!("duplicate static {name}"),
        ProgramDiagnostic::StaticFunctionNameConflict { name } => {
            format!("static/function name conflict {name}")
        }
        ProgramDiagnostic::StaticInitCycle { cycle } => {
            format!("static init cycle [{}]", cycle.join(" -> "))
        }
        ProgramDiagnostic::InvalidStaticAdtConstructor {
            static_name,
            data,
            ctor,
        } => format!("invalid static ADT constructor {static_name}: {data}.{ctor}"),
        ProgramDiagnostic::UnsupportedStaticInitializer { static_name, expr } => {
            format!("unsupported static initializer {static_name}: {expr}")
        }
        ProgramDiagnostic::ExplicitAutoGenericConflict { def, param } => {
            format!("explicit auto-generic conflict {def}[{param}]")
        }
        ProgramDiagnostic::ConflictingExplicitInstantiation {
            def,
            callee,
            previous_type_args,
            type_args,
        } => format!(
            "conflicting explicit instantiation {def}: {callee}[{}] vs [{}]",
            previous_type_args.join(", "),
            type_args.join(", ")
        ),
        ProgramDiagnostic::ShiftOutsideReset { def, binder } => {
            format!("shift outside reset {def}: {binder}")
        }
        ProgramDiagnostic::UnsafeMultiResumeCapture { def, binder } => {
            format!("unsafe multi-resume capture {def}: {binder}")
        }
        ProgramDiagnostic::Cont1ResumedMoreThanOnce { def, binder } => {
            format!("cont1 resumed more than once {def}: {binder}")
        }
        ProgramDiagnostic::MissingEntry => "missing entry".to_string(),
        ProgramDiagnostic::EntryHasParams { name, params } => {
            format!("entry has params {name}({})", params.join(", "))
        }
    }
}

fn render_global_init_summary(plan: &GlobalInitPlan) -> String {
    let mut out = String::new();
    out.push_str("  global-init:\n");
    out.push_str(&format!("    statics={}\n", plan.statics.len()));
    for static_value in &plan.statics {
        out.push_str(&format!(
            "    static {} ty={} init={}\n",
            render_global_static_id(&GlobalStaticId::new(
                static_value.owner.clone(),
                static_value.name.clone()
            )),
            static_value.ty.as_deref().unwrap_or("<inferred>"),
            render_source_expr(&static_value.body)
        ));
        if !static_value.dependency_ids.is_empty() {
            let deps = static_value
                .dependency_ids
                .iter()
                .map(render_global_static_id)
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("      deps=[{deps}]\n"));
        }
    }
    let init_order = plan
        .init_order_ids
        .iter()
        .map(render_global_static_id)
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!("    init-order=[{init_order}]\n"));
    out.push_str(&format!("    diagnostics={}\n", plan.diagnostics.len()));
    for diagnostic in &plan.diagnostics {
        out.push_str(&format!(
            "      {}\n",
            render_global_init_diagnostic(diagnostic)
        ));
    }
    out
}

fn render_global_static_id(id: &GlobalStaticId) -> String {
    format!("{}::{}", id.owner, id.name)
}

fn render_global_init_diagnostic(diagnostic: &GlobalInitDiagnostic) -> String {
    match diagnostic {
        GlobalInitDiagnostic::DuplicateStatic { name } => format!("duplicate static {name}"),
        GlobalInitDiagnostic::StaticFunctionNameConflict { name } => {
            format!("static/function name conflict {name}")
        }
        GlobalInitDiagnostic::StaticInitCycle { cycle } => {
            format!("static init cycle [{}]", cycle.join(" -> "))
        }
        GlobalInitDiagnostic::InvalidStaticAdtConstructor {
            static_name,
            data,
            ctor,
        } => format!("invalid static ADT constructor {static_name}: {data}.{ctor}"),
        GlobalInitDiagnostic::UnsupportedStaticInitializer { static_name, expr } => {
            format!("unsupported static initializer {static_name}: {expr}")
        }
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
            "  imports={}\n",
            render_imports_summary(self.frontend.program.imports.iter().map(UseDecl::dotted))
        ));
        out.push_str(&format!("  items={}\n", self.frontend.program.items.len()));
        out.push_str("  item-spans:\n");
        for item in &self.frontend.item_spans {
            out.push_str(&format!(
                "    {} {} @ {}:{}..{}:{}\n",
                item.kind,
                item.name,
                item.span.line,
                item.span.column,
                item.span.end_line,
                item.span.end_column
            ));
        }
        out.push_str(&format!("  data={}\n", self.frontend.program.data.len()));
        out.push_str(&self.program.render_summary());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{BackendManifest, BackendTarget};
    use crate::global::GlobalStatic;

    #[test]
    fn global_init_lowering_rejects_internal_unsupported_expr_without_fake_zero() {
        let bundle = BackendLinkedBundle {
            target: BackendTarget::WasmGc,
            linked_wat:
                "(module\n  (func $main (export \"main\") (result i32)\n    i32.const 1)\n)\n"
                    .to_string(),
            manifest: BackendManifest::default(),
            diagnostics: vec![],
        };
        let global_init = GlobalInitPlan {
            statics: vec![GlobalStatic {
                owner: "root".to_string(),
                name: "VALUE".to_string(),
                ty: Some("i64".to_string()),
                body: Expr::call_args(Expr::var("helper"), Vec::new()),
                dependencies: vec![],
                dependency_ids: vec![],
            }],
            init_order: vec!["VALUE".to_string()],
            init_order_ids: vec![crate::global::GlobalStaticId::new("root", "VALUE")],
            diagnostics: vec![],
        };

        let lowered = lower_global_init_into_linked_wat(bundle, &global_init);

        assert_eq!(
            lowered.diagnostics,
            vec![
                BackendLinkDiagnostic::UnsupportedStaticInitializerLowering {
                    static_name: "VALUE".to_string(),
                    expr: "helper()".to_string(),
                }
            ]
        );
        let [BackendLinkDiagnostic::UnsupportedStaticInitializerLowering { expr, .. }] =
            lowered.diagnostics.as_slice()
        else {
            panic!("expected unsupported static initializer diagnostic");
        };
        assert_eq!(expr, "helper()");
        assert!(!lowered.linked_wat.contains("global__VALUE"));
        assert!(!lowered.linked_wat.contains("unsupported static init"));
        assert!(!lowered.linked_wat.contains("(i32.const 0)"));
    }

    #[test]
    fn global_init_lowering_rejects_internal_unknown_adt_ctor_without_fake_tag_zero() {
        let bundle = BackendLinkedBundle {
            target: BackendTarget::WasmGc,
            linked_wat:
                "(module\n  (func $main (export \"main\") (result i32)\n    i32.const 1)\n)\n"
                    .to_string(),
            manifest: BackendManifest::default(),
            diagnostics: vec![],
        };
        let body = Expr::adt_ctor("Option", "Ghost", vec!["None", "Some"], Vec::new());
        let global_init = GlobalInitPlan {
            statics: vec![GlobalStatic {
                owner: "root".to_string(),
                name: "VALUE".to_string(),
                ty: Some("Option".to_string()),
                body: body.clone(),
                dependencies: vec![],
                dependency_ids: vec![],
            }],
            init_order: vec!["VALUE".to_string()],
            init_order_ids: vec![crate::global::GlobalStaticId::new("root", "VALUE")],
            diagnostics: vec![],
        };

        let lowered = lower_global_init_into_linked_wat(bundle, &global_init);

        assert_eq!(
            lowered.diagnostics,
            vec![
                BackendLinkDiagnostic::UnsupportedStaticInitializerLowering {
                    static_name: "VALUE".to_string(),
                    expr: "Option.Ghost()".to_string(),
                }
            ]
        );
        let [BackendLinkDiagnostic::UnsupportedStaticInitializerLowering { expr, .. }] =
            lowered.diagnostics.as_slice()
        else {
            panic!("expected unsupported static initializer diagnostic");
        };
        assert_eq!(expr, "Option.Ghost()");
        assert!(!lowered.linked_wat.contains("global__VALUE"));
        assert!(!lowered.linked_wat.contains("(i32.const 0)"));
    }
}
