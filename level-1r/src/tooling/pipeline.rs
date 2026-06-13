use crate::alpha::{alpha_expr_with_params, AlphaBinder, AlphaFacts};
use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    generic_param_decls_from_names, render_source_binary_op, render_source_expr, Expr, ExternAbi,
    ExternDecl, GenericBoundDecl, GenericParamDecl, ItemAttr, MethodReceiver, NamespaceDecl,
    ParamDecl, Pattern, SourceItem, SourceProgram, UseDecl, Visibility,
};
use crate::backend::{
    backend_cache_key, emit_wasm_gc_with_param_abi, link_backend_artifacts, sort_dedup_imports,
    BackendAdtPayloadAbi, BackendAdtTagAbi, BackendArtifact, BackendCacheConfig, BackendCacheKey,
    BackendCallableAbi, BackendCallableArgExpansion, BackendDiagnostic, BackendDynRowParamFieldAbi,
    BackendDynRowParamMethodAbi, BackendExternAbi, BackendExternImport, BackendLinkDiagnostic,
    BackendLinkedBundle, BackendParamAbi, BackendReturnedCallableAbi, BackendValueKind,
};
use crate::closure::{analyze_alpha_closures_with_params, ClosureFacts};
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
use crate::global::{
    analyze_global_init, GlobalInitDiagnostic, GlobalInitPlan, GlobalStatic, GlobalStaticId,
};
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
    nominal_base_name_for_type, nominal_type_args_for_type, source_type_name_for_type,
    source_type_name_to_type, type_expr_with_context, type_expr_with_expected, IndexAccessKind,
    ReceiverMethodSummary, RecordTypeField, Type, TypeContext, TypeEnv, TypedExpr, TypedExprKind,
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
    pub receiver: Option<String>,
    pub entry: bool,
    pub params: Vec<String>,
    pub output: CompileOutput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramDiagnostic {
    InvalidSourceFileHeader {
        reason: String,
    },
    DuplicateNamespace {
        namespace: String,
    },
    MultipleEntries {
        names: Vec<String>,
    },
    EntryOnNonFunction {
        name: String,
    },
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
    ExplicitInstantiationArityMismatch {
        def: String,
        callee: String,
        expected: usize,
        actual: usize,
    },
    ExplicitInstantiationOfNonTemplate {
        def: String,
        callee: String,
    },
    DefinitionReturnTypeMismatch {
        def: String,
        expected: String,
        actual: String,
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
    NonSendCallablePassedToSendCallable {
        def: String,
        callee: String,
        arg: String,
    },
    InvalidAssignmentTarget {
        def: String,
        target_type: String,
    },
    DynRowCoercionFailed {
        def: String,
        expected: String,
        actual: String,
    },
    RowMemberCallableUnsatisfied {
        def: String,
        callee: String,
        field: String,
        actual: String,
    },
    NonCallableFieldForCallableRowMember {
        def: String,
        callee: String,
        field: String,
        actual: String,
        field_type: String,
    },
    MethodSignatureMismatchForRowMember {
        def: String,
        callee: String,
        field: String,
        actual: String,
    },
    AmbiguousReceiverMethod {
        receiver: String,
        name: String,
        candidates: Vec<String>,
    },
    AmbiguousOperatorResolution {
        receiver: String,
        op: String,
        candidates: Vec<String>,
    },
    InvalidOperatorOperands {
        def: String,
        op: String,
        lhs: String,
        rhs: String,
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
        &[],
        &None,
        &None,
        &TypeContext::new(),
        "root",
        &type_aliases,
        &[],
        &[],
        &[],
        &[],
        &[],
        &TypeEnv::new(),
    )
}

fn compile_expr_with_indexes_and_generics(
    def_name: &str,
    expr: &Expr,
    names: NameIndex,
    methods: MethodIndex,
    explicit_generics: &[String],
    explicit_generic_params: &[GenericParamDecl],
    params: &[ParamDecl],
    return_type: &Option<String>,
    receiver: &Option<MethodReceiver>,
    type_context: &TypeContext,
    current_namespace: &str,
    type_aliases: &TypeAliasIndex,
    interface_functions: &[crate::surface::InterfaceFunction],
    interface_types: &[crate::surface::InterfaceType],
    interface_data: &[crate::surface::InterfaceData],
    interface_constructors: &[crate::surface::InterfaceConstructor],
    interface_statics: &[crate::surface::InterfaceStatic],
    function_env: &TypeEnv,
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
                explicit_generic_param_decls(explicit_generics, explicit_generic_params).as_slice(),
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
    let typed_env = typed_signature.type_env(function_env);
    let typed = passes.record("L7Typed", "SourceExpr+TypedSignature", "TypedExpr", || {
        typed_signature
            .return_type
            .as_deref()
            .map(source_type_name_to_type)
            .map(|expected| type_expr_with_expected(expr, &typed_env, type_context, &expected))
            .unwrap_or_else(|| type_expr_with_context(expr, &typed_env, type_context))
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
        analyze_alpha_closures_with_params(&alpha.expr, &alpha.param_binders)
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
        attach_extern_function_targets(&mut core, &resolve, interface_functions);
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
            let param_abi = backend_param_abi(
                &typed_signature,
                interface_functions,
                interface_types,
                interface_data,
                interface_constructors,
                interface_statics,
            );
            emit_wasm_gc_with_param_abi(&core, &core_validation, &param_names, &param_abi)
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

fn explicit_generic_param_decls(
    explicit_generics: &[String],
    explicit_generic_params: &[GenericParamDecl],
) -> Vec<GenericParamDecl> {
    if explicit_generic_params.is_empty() && !explicit_generics.is_empty() {
        generic_param_decls_from_names(explicit_generics)
    } else {
        explicit_generic_params.to_vec()
    }
}

pub fn compile_program(program: &SourceProgram) -> Vec<CompileOutput> {
    compile_program_bundle(program)
        .defs
        .into_iter()
        .map(|def| def.output)
        .collect()
}

pub fn compile_program_bundle_with_interface(
    program: &SourceProgram,
    interface: &InterfaceSummary,
) -> ProgramCompileOutput {
    compile_program_bundle_internal(program, Some(interface.clone()))
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
    compile_program_bundle_internal(program, None)
}

fn compile_program_bundle_internal(
    program: &SourceProgram,
    interface_override: Option<InterfaceSummary>,
) -> ProgramCompileOutput {
    let initial_program = normalize_pattern_clause_defs(program);
    let initial_surface = project_surface(&initial_program);
    let initial_interface = interface_override
        .clone()
        .unwrap_or_else(|| build_interface_summary(&initial_surface));
    let current_namespace = initial_program
        .namespace
        .as_ref()
        .map(NamespaceDecl::dotted)
        .unwrap_or_else(|| "root".to_string());
    let normalized_program = specialize_row_callable_call_sites(
        &initial_program,
        &initial_interface,
        &current_namespace,
    );
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
        || interface_override.unwrap_or_else(|| build_interface_summary(&surface)),
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
    let checked_interface = interface_without_invalid_static_initializers(&interface, &global_init);
    let mut defs = passes.record(
        "P5ProgramDefs",
        "SourceProgram+InterfaceSummary",
        "ProgramDefOutput",
        || compile_program_defs(&normalized_program, &checked_interface),
    );
    refresh_program_backends_with_lifted_callables(&mut defs, &checked_interface);
    let entry = passes.record(
        "P6ProgramEntry",
        "ProgramDefOutput",
        "EntrySelection",
        || select_program_entry(&defs),
    );
    let mut all_diagnostics = diagnostics;
    let receiver_ambiguity_diagnostics =
        receiver_method_ambiguity_diagnostics(&checked_interface, &normalized_program);
    all_diagnostics.extend(ambiguous_operator_resolution_diagnostics(
        &receiver_ambiguity_diagnostics,
    ));
    all_diagnostics.extend(receiver_ambiguity_diagnostics);
    all_diagnostics.extend(global_init.diagnostics.iter().cloned().map(Into::into));
    all_diagnostics.extend(template_diagnostics(&defs));
    all_diagnostics.extend(explicit_instantiation_diagnostics(&defs));
    all_diagnostics.extend(definition_return_type_diagnostics(&defs));
    all_diagnostics.extend(control_diagnostics(&defs));
    all_diagnostics.extend(cps_usage_diagnostics(&defs));
    let send_diagnostics = send_callable_diagnostics(&defs);
    all_diagnostics.extend(send_diagnostics.iter().cloned());
    all_diagnostics.extend(assignment_diagnostics(&defs));
    all_diagnostics.extend(dyn_row_coercion_diagnostics(&defs));
    all_diagnostics.extend(row_member_callable_diagnostics(
        &normalized_program,
        &checked_interface,
        &current_namespace,
    ));
    all_diagnostics.extend(operator_operand_diagnostics(&defs));
    apply_program_backend_gates(&mut defs, &send_diagnostics);
    if entry.is_none() {
        all_diagnostics.push(ProgramDiagnostic::MissingEntry);
    }
    all_diagnostics.extend(entry_diagnostics(&defs, &surface));
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
                &interface,
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

#[derive(Clone, Debug)]
struct RowCallableDef {
    param: String,
    field: String,
    args: Vec<Expr>,
}

fn specialize_row_callable_call_sites(
    program: &SourceProgram,
    interface: &InterfaceSummary,
    current_namespace: &str,
) -> SourceProgram {
    let row_callables = program
        .items
        .iter()
        .filter_map(row_callable_def)
        .collect::<BTreeMap<_, _>>();
    if row_callables.is_empty() {
        return program.clone();
    }

    let mut specialized = program.clone();
    let row_member_records = row_callable_member_records(interface, current_namespace);
    specialized.items = program
        .items
        .iter()
        .map(|item| specialize_row_callable_item(item, &row_callables, &row_member_records))
        .collect();
    specialized
}

#[derive(Clone, Debug, Default)]
struct RowCallableMemberRecords {
    members: BTreeMap<String, BTreeMap<String, Vec<RowCallableMemberRecord>>>,
}

#[derive(Clone, Debug)]
enum RowCallableMemberRecord {
    Field { ty: Type },
    ReceiverMethod { param_tys: Vec<Type> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BackendDynRowMemberAbi {
    Field(BackendDynRowParamFieldAbi),
    ReceiverMethod(BackendDynRowParamMethodAbi),
}

type DynRowMethodTargets = BTreeMap<String, BTreeMap<String, Vec<BackendDynRowParamMethodAbi>>>;

fn row_callable_member_records(
    interface: &InterfaceSummary,
    current_namespace: &str,
) -> RowCallableMemberRecords {
    let mut records = RowCallableMemberRecords::default();
    for ty in &interface.types {
        for field in &ty.fields {
            records
                .members
                .entry(ty.name.clone())
                .or_default()
                .entry(field.name.clone())
                .or_default()
                .push(RowCallableMemberRecord::Field {
                    ty: source_type_name_to_type(&field.ty),
                });
        }
    }
    for method in visible_receiver_methods(interface, current_namespace) {
        let Some(receiver) = &method.receiver else {
            continue;
        };
        records
            .members
            .entry(receiver.display_name())
            .or_default()
            .entry(method.source_name.clone())
            .or_default()
            .push(RowCallableMemberRecord::ReceiverMethod {
                param_tys: method
                    .param_types
                    .iter()
                    .skip(1)
                    .map(|ty| {
                        ty.as_deref()
                            .map(source_type_name_to_type)
                            .unwrap_or(Type::Unknown)
                    })
                    .collect(),
            });
    }
    records
}

fn row_callable_def(item: &SourceItem) -> Option<(String, RowCallableDef)> {
    let SourceItem::Def {
        name,
        receiver: None,
        generic_params,
        params,
        body,
        ..
    } = item
    else {
        return None;
    };
    if params.len() != 1 {
        return None;
    }
    let Pattern::Bind(param_name) = &params[0].pattern else {
        return None;
    };
    let Expr::MethodCall {
        receiver,
        name: field,
        args,
    } = body
    else {
        return None;
    };
    if !matches!(receiver.as_ref(), Expr::Var(receiver_name) if receiver_name == param_name) {
        return None;
    }
    if !row_callable_has_supported_generic_surface(params, generic_params, field) {
        return None;
    }
    Some((
        name.clone(),
        RowCallableDef {
            param: param_name.clone(),
            field: field.clone(),
            args: args.clone(),
        },
    ))
}

fn row_callable_has_supported_generic_surface(
    params: &[ParamDecl],
    generic_params: &[GenericParamDecl],
    field: &str,
) -> bool {
    if params[0].ty.is_none() {
        return generic_params.is_empty();
    }
    let Some(param_ty) = params[0].ty.as_deref() else {
        return false;
    };
    generic_params.iter().any(|generic| {
        generic.name == param_ty
            && match &generic.bound {
                Some(GenericBoundDecl::OpenRow(fields)) => {
                    fields.iter().any(|candidate| candidate.name == field)
                }
                None => false,
            }
    })
}

fn specialize_row_callable_item(
    item: &SourceItem,
    row_callables: &BTreeMap<String, RowCallableDef>,
    member_records: &RowCallableMemberRecords,
) -> SourceItem {
    match item {
        SourceItem::Def {
            name,
            attrs,
            visibility,
            receiver,
            generics,
            generic_params,
            params,
            return_type,
            body,
        } => SourceItem::Def {
            name: name.clone(),
            attrs: attrs.clone(),
            visibility: *visibility,
            receiver: receiver.clone(),
            generics: generics.clone(),
            generic_params: generic_params.clone(),
            params: params.clone(),
            return_type: return_type.clone(),
            body: specialize_row_callable_expr(body, row_callables, member_records),
        },
        SourceItem::StaticValue {
            name,
            attrs,
            visibility,
            ty,
            body,
        } => SourceItem::StaticValue {
            name: name.clone(),
            attrs: attrs.clone(),
            visibility: *visibility,
            ty: ty.clone(),
            body: specialize_row_callable_expr(body, row_callables, member_records),
        },
        SourceItem::ExternDef { .. } => item.clone(),
    }
}

fn specialize_row_callable_expr(
    expr: &Expr,
    row_callables: &BTreeMap<String, RowCallableDef>,
    member_records: &RowCallableMemberRecords,
) -> Expr {
    if let Expr::Call { callee, args } = expr {
        if let Expr::Var(callee_name) = callee.as_ref() {
            if let Some(row_callable) = row_callables.get(callee_name) {
                if let [arg] = args.as_slice() {
                    if row_callable_arg_has_callable_field(arg, row_callable, member_records) {
                        return Expr::MethodCall {
                            receiver: Box::new(specialize_row_callable_expr(
                                arg,
                                row_callables,
                                member_records,
                            )),
                            name: row_callable.field.clone(),
                            args: row_callable
                                .args
                                .iter()
                                .map(|arg| {
                                    substitute_row_callable_param(
                                        arg,
                                        &row_callable.param,
                                        args.first().expect("checked one row callable arg"),
                                        row_callables,
                                        member_records,
                                    )
                                })
                                .collect(),
                        };
                    }
                }
            }
        }
    }

    match expr {
        Expr::Var(_) | Expr::Lit(_) => expr.clone(),
        Expr::Lambda {
            param,
            param_ty,
            return_ty,
            body,
        } => Expr::Lambda {
            param: param.clone(),
            param_ty: param_ty.clone(),
            return_ty: return_ty.clone(),
            body: Box::new(specialize_row_callable_expr(
                body,
                row_callables,
                member_records,
            )),
        },
        Expr::Call { callee, args } => Expr::Call {
            callee: Box::new(specialize_row_callable_expr(
                callee,
                row_callables,
                member_records,
            )),
            args: args
                .iter()
                .map(|arg| specialize_row_callable_expr(arg, row_callables, member_records))
                .collect(),
        },
        Expr::Instantiate { callee, type_args } => Expr::Instantiate {
            callee: Box::new(specialize_row_callable_expr(
                callee,
                row_callables,
                member_records,
            )),
            type_args: type_args.clone(),
        },
        Expr::Tuple(fields) => Expr::Tuple(
            fields
                .iter()
                .map(|field| specialize_row_callable_expr(field, row_callables, member_records))
                .collect(),
        ),
        Expr::SliceLiteral(items) => Expr::SliceLiteral(
            items
                .iter()
                .map(|item| specialize_row_callable_expr(item, row_callables, member_records))
                .collect(),
        ),
        Expr::Record(fields) => Expr::Record(
            fields
                .iter()
                .map(|field| crate::ast::RecordField {
                    name: field.name.clone(),
                    value: specialize_row_callable_expr(
                        &field.value,
                        row_callables,
                        member_records,
                    ),
                })
                .collect(),
        ),
        Expr::RecordUpdate { base, fields } => Expr::RecordUpdate {
            base: Box::new(specialize_row_callable_expr(
                base,
                row_callables,
                member_records,
            )),
            fields: fields
                .iter()
                .map(|field| crate::ast::RecordField {
                    name: field.name.clone(),
                    value: specialize_row_callable_expr(
                        &field.value,
                        row_callables,
                        member_records,
                    ),
                })
                .collect(),
        },
        Expr::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => Expr::AdtCtor {
            data: data.clone(),
            ctor: ctor.clone(),
            variants: variants.clone(),
            args: args
                .iter()
                .map(|arg| specialize_row_callable_expr(arg, row_callables, member_records))
                .collect(),
        },
        Expr::Field { receiver, name } => Expr::Field {
            receiver: Box::new(specialize_row_callable_expr(
                receiver,
                row_callables,
                member_records,
            )),
            name: name.clone(),
        },
        Expr::MethodCall {
            receiver,
            name,
            args,
        } => Expr::MethodCall {
            receiver: Box::new(specialize_row_callable_expr(
                receiver,
                row_callables,
                member_records,
            )),
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| specialize_row_callable_expr(arg, row_callables, member_records))
                .collect(),
        },
        Expr::Assign { target, value } => Expr::Assign {
            target: Box::new(specialize_row_callable_expr(
                target,
                row_callables,
                member_records,
            )),
            value: Box::new(specialize_row_callable_expr(
                value,
                row_callables,
                member_records,
            )),
        },
        Expr::Index { receiver, index } => Expr::Index {
            receiver: Box::new(specialize_row_callable_expr(
                receiver,
                row_callables,
                member_records,
            )),
            index: Box::new(specialize_row_callable_expr(
                index,
                row_callables,
                member_records,
            )),
        },
        Expr::Range { start, end } => Expr::Range {
            start: Box::new(specialize_row_callable_expr(
                start,
                row_callables,
                member_records,
            )),
            end: Box::new(specialize_row_callable_expr(
                end,
                row_callables,
                member_records,
            )),
        },
        Expr::Binary { op, lhs, rhs } => Expr::Binary {
            op: *op,
            lhs: Box::new(specialize_row_callable_expr(
                lhs,
                row_callables,
                member_records,
            )),
            rhs: Box::new(specialize_row_callable_expr(
                rhs,
                row_callables,
                member_records,
            )),
        },
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => Expr::If {
            cond: Box::new(specialize_row_callable_expr(
                cond,
                row_callables,
                member_records,
            )),
            then_branch: Box::new(specialize_row_callable_expr(
                then_branch,
                row_callables,
                member_records,
            )),
            else_branch: Box::new(specialize_row_callable_expr(
                else_branch,
                row_callables,
                member_records,
            )),
        },
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => Expr::IfLet {
            pattern: pattern.clone(),
            scrutinee: Box::new(specialize_row_callable_expr(
                scrutinee,
                row_callables,
                member_records,
            )),
            then_branch: Box::new(specialize_row_callable_expr(
                then_branch,
                row_callables,
                member_records,
            )),
            else_branch: Box::new(specialize_row_callable_expr(
                else_branch,
                row_callables,
                member_records,
            )),
        },
        Expr::Match { scrutinee, arms } => Expr::Match {
            scrutinee: Box::new(specialize_row_callable_expr(
                scrutinee,
                row_callables,
                member_records,
            )),
            arms: arms
                .iter()
                .map(|arm| crate::ast::MatchArm {
                    pattern: arm.pattern.clone(),
                    body: specialize_row_callable_expr(&arm.body, row_callables, member_records),
                })
                .collect(),
        },
        Expr::Nominal { name, expr } => Expr::Nominal {
            name: name.clone(),
            expr: Box::new(specialize_row_callable_expr(
                expr,
                row_callables,
                member_records,
            )),
        },
        Expr::Reset { multi, body } => Expr::Reset {
            multi: *multi,
            body: Box::new(specialize_row_callable_expr(
                body,
                row_callables,
                member_records,
            )),
        },
        Expr::Shift { binder, body } => Expr::Shift {
            binder: binder.clone(),
            body: Box::new(specialize_row_callable_expr(
                body,
                row_callables,
                member_records,
            )),
        },
    }
}

fn substitute_row_callable_param(
    expr: &Expr,
    param: &str,
    replacement: &Expr,
    row_callables: &BTreeMap<String, RowCallableDef>,
    member_records: &RowCallableMemberRecords,
) -> Expr {
    match expr {
        Expr::Var(name) if name == param => {
            specialize_row_callable_expr(replacement, row_callables, member_records)
        }
        Expr::Lambda {
            param: lambda_param,
            param_ty,
            return_ty,
            body,
        } if lambda_param == param => Expr::Lambda {
            param: lambda_param.clone(),
            param_ty: param_ty.clone(),
            return_ty: return_ty.clone(),
            body: body.clone(),
        },
        Expr::Lambda {
            param: lambda_param,
            param_ty,
            return_ty,
            body,
        } => Expr::Lambda {
            param: lambda_param.clone(),
            param_ty: param_ty.clone(),
            return_ty: return_ty.clone(),
            body: Box::new(substitute_row_callable_param(
                body,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::Call { callee, args } => Expr::Call {
            callee: Box::new(substitute_row_callable_param(
                callee,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            args: args
                .iter()
                .map(|arg| {
                    substitute_row_callable_param(
                        arg,
                        param,
                        replacement,
                        row_callables,
                        member_records,
                    )
                })
                .collect(),
        },
        Expr::MethodCall {
            receiver,
            name,
            args,
        } => Expr::MethodCall {
            receiver: Box::new(substitute_row_callable_param(
                receiver,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| {
                    substitute_row_callable_param(
                        arg,
                        param,
                        replacement,
                        row_callables,
                        member_records,
                    )
                })
                .collect(),
        },
        Expr::Field { receiver, name } => Expr::Field {
            receiver: Box::new(substitute_row_callable_param(
                receiver,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            name: name.clone(),
        },
        Expr::Binary { op, lhs, rhs } => Expr::Binary {
            op: *op,
            lhs: Box::new(substitute_row_callable_param(
                lhs,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            rhs: Box::new(substitute_row_callable_param(
                rhs,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::Instantiate { callee, type_args } => Expr::Instantiate {
            callee: Box::new(substitute_row_callable_param(
                callee,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            type_args: type_args.clone(),
        },
        Expr::Tuple(fields) => Expr::Tuple(
            fields
                .iter()
                .map(|field| {
                    substitute_row_callable_param(
                        field,
                        param,
                        replacement,
                        row_callables,
                        member_records,
                    )
                })
                .collect(),
        ),
        Expr::SliceLiteral(items) => Expr::SliceLiteral(
            items
                .iter()
                .map(|item| {
                    substitute_row_callable_param(
                        item,
                        param,
                        replacement,
                        row_callables,
                        member_records,
                    )
                })
                .collect(),
        ),
        Expr::Record(fields) => Expr::Record(
            fields
                .iter()
                .map(|field| crate::ast::RecordField {
                    name: field.name.clone(),
                    value: substitute_row_callable_param(
                        &field.value,
                        param,
                        replacement,
                        row_callables,
                        member_records,
                    ),
                })
                .collect(),
        ),
        Expr::RecordUpdate { base, fields } => Expr::RecordUpdate {
            base: Box::new(substitute_row_callable_param(
                base,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            fields: fields
                .iter()
                .map(|field| crate::ast::RecordField {
                    name: field.name.clone(),
                    value: substitute_row_callable_param(
                        &field.value,
                        param,
                        replacement,
                        row_callables,
                        member_records,
                    ),
                })
                .collect(),
        },
        Expr::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => Expr::AdtCtor {
            data: data.clone(),
            ctor: ctor.clone(),
            variants: variants.clone(),
            args: args
                .iter()
                .map(|arg| {
                    substitute_row_callable_param(
                        arg,
                        param,
                        replacement,
                        row_callables,
                        member_records,
                    )
                })
                .collect(),
        },
        Expr::Assign { target, value } => Expr::Assign {
            target: Box::new(substitute_row_callable_param(
                target,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            value: Box::new(substitute_row_callable_param(
                value,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::Index { receiver, index } => Expr::Index {
            receiver: Box::new(substitute_row_callable_param(
                receiver,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            index: Box::new(substitute_row_callable_param(
                index,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::Range { start, end } => Expr::Range {
            start: Box::new(substitute_row_callable_param(
                start,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            end: Box::new(substitute_row_callable_param(
                end,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => Expr::If {
            cond: Box::new(substitute_row_callable_param(
                cond,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            then_branch: Box::new(substitute_row_callable_param(
                then_branch,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            else_branch: Box::new(substitute_row_callable_param(
                else_branch,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => Expr::IfLet {
            pattern: pattern.clone(),
            scrutinee: Box::new(substitute_row_callable_param(
                scrutinee,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            then_branch: Box::new(substitute_row_callable_param(
                then_branch,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            else_branch: Box::new(substitute_row_callable_param(
                else_branch,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::Match { scrutinee, arms } => Expr::Match {
            scrutinee: Box::new(substitute_row_callable_param(
                scrutinee,
                param,
                replacement,
                row_callables,
                member_records,
            )),
            arms: arms
                .iter()
                .map(|arm| crate::ast::MatchArm {
                    pattern: arm.pattern.clone(),
                    body: substitute_row_callable_param(
                        &arm.body,
                        param,
                        replacement,
                        row_callables,
                        member_records,
                    ),
                })
                .collect(),
        },
        Expr::Nominal { name, expr } => Expr::Nominal {
            name: name.clone(),
            expr: Box::new(substitute_row_callable_param(
                expr,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::Reset { multi, body } => Expr::Reset {
            multi: *multi,
            body: Box::new(substitute_row_callable_param(
                body,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::Shift { binder, body } if binder == param => Expr::Shift {
            binder: binder.clone(),
            body: body.clone(),
        },
        Expr::Shift { binder, body } => Expr::Shift {
            binder: binder.clone(),
            body: Box::new(substitute_row_callable_param(
                body,
                param,
                replacement,
                row_callables,
                member_records,
            )),
        },
        Expr::Var(_) | Expr::Lit(_) => {
            specialize_row_callable_expr(expr, row_callables, member_records)
        }
    }
}

fn row_callable_arg_has_callable_field(
    arg: &Expr,
    row_callable: &RowCallableDef,
    member_records: &RowCallableMemberRecords,
) -> bool {
    row_callable_arg_check(arg, row_callable, member_records).is_satisfied()
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RowCallableArgCheck {
    Satisfied,
    NonCallableField { field_type: Type },
    SignatureMismatch,
    MissingMember,
}

impl RowCallableArgCheck {
    fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied)
    }
}

fn row_callable_arg_check(
    arg: &Expr,
    row_callable: &RowCallableDef,
    member_records: &RowCallableMemberRecords,
) -> RowCallableArgCheck {
    match arg {
        Expr::Record(fields) => fields
            .iter()
            .find(|candidate| candidate.name == row_callable.field)
            .map(|candidate| {
                if expr_accepts_row_callable_args(&candidate.value, &row_callable.args) {
                    RowCallableArgCheck::Satisfied
                } else {
                    source_expr_static_type(&candidate.value)
                        .map(|field_type| RowCallableArgCheck::NonCallableField { field_type })
                        .unwrap_or(RowCallableArgCheck::SignatureMismatch)
                }
            })
            .unwrap_or(RowCallableArgCheck::MissingMember),
        Expr::Nominal { name, .. } => member_records
            .members
            .get(name)
            .and_then(|members| members.get(&row_callable.field))
            .map(|members| row_member_records_callable_check(members, row_callable))
            .unwrap_or(RowCallableArgCheck::MissingMember),
        _ => RowCallableArgCheck::MissingMember,
    }
}

fn row_member_records_callable_check(
    members: &[RowCallableMemberRecord],
    row_callable: &RowCallableDef,
) -> RowCallableArgCheck {
    let field_members = members
        .iter()
        .filter_map(|member| match member {
            RowCallableMemberRecord::Field { ty } => Some(ty),
            RowCallableMemberRecord::ReceiverMethod { .. } => None,
        })
        .collect::<Vec<_>>();
    if !field_members.is_empty() {
        if field_members
            .iter()
            .any(|ty| type_accepts_row_callable_args(ty, &row_callable.args))
        {
            return RowCallableArgCheck::Satisfied;
        }
        return RowCallableArgCheck::NonCallableField {
            field_type: field_members
                .first()
                .map(|ty| (*ty).clone())
                .unwrap_or(Type::Unknown),
        };
    }
    if members.iter().any(|member| match member {
        RowCallableMemberRecord::Field { .. } => false,
        RowCallableMemberRecord::ReceiverMethod { param_tys } => {
            row_callable_args_match_types(&row_callable.args, param_tys)
        }
    }) {
        RowCallableArgCheck::Satisfied
    } else if members
        .iter()
        .any(|member| matches!(member, RowCallableMemberRecord::ReceiverMethod { .. }))
    {
        RowCallableArgCheck::SignatureMismatch
    } else {
        RowCallableArgCheck::MissingMember
    }
}

fn expr_is_callable_value(expr: &Expr) -> bool {
    matches!(expr, Expr::Lambda { .. } | Expr::Var(_))
}

fn expr_accepts_row_callable_args(expr: &Expr, args: &[Expr]) -> bool {
    if !expr_is_callable_value(expr) {
        return false;
    }
    callable_expr_param_types(expr)
        .map(|param_tys| row_callable_args_match_types(args, &param_tys))
        .unwrap_or(true)
}

fn callable_expr_param_types(expr: &Expr) -> Option<Vec<Type>> {
    match expr {
        Expr::Lambda { param_ty, .. } => param_ty
            .as_deref()
            .map(source_type_name_to_type)
            .map(|ty| vec![ty]),
        _ => None,
    }
}

fn type_is_callable_storage(ty: &Type) -> bool {
    matches!(ty, Type::Func(..) | Type::Continuation { .. })
}

fn row_member_callable_diagnostics(
    program: &SourceProgram,
    interface: &InterfaceSummary,
    current_namespace: &str,
) -> Vec<ProgramDiagnostic> {
    let row_callables = program
        .items
        .iter()
        .filter_map(row_callable_def)
        .collect::<BTreeMap<_, _>>();
    if row_callables.is_empty() {
        return Vec::new();
    }
    let member_records = row_callable_member_records(interface, current_namespace);
    let mut diagnostics = Vec::new();
    for item in &program.items {
        let SourceItem::Def { name, body, .. } = item else {
            continue;
        };
        collect_row_member_callable_diagnostics(
            name,
            body,
            &row_callables,
            &member_records,
            &mut diagnostics,
        );
    }
    diagnostics
}

fn collect_row_member_callable_diagnostics(
    def: &str,
    expr: &Expr,
    row_callables: &BTreeMap<String, RowCallableDef>,
    member_records: &RowCallableMemberRecords,
    diagnostics: &mut Vec<ProgramDiagnostic>,
) {
    match expr {
        Expr::Call { callee, args } => {
            if let Expr::Var(callee_name) = callee.as_ref() {
                if let Some(row_callable) = row_callables.get(callee_name) {
                    if let [arg] = args.as_slice() {
                        match row_callable_arg_check(arg, row_callable, member_records) {
                            RowCallableArgCheck::Satisfied => {}
                            RowCallableArgCheck::NonCallableField { field_type } => {
                                diagnostics.push(
                                    ProgramDiagnostic::NonCallableFieldForCallableRowMember {
                                        def: def.to_string(),
                                        callee: callee_name.clone(),
                                        field: row_callable.field.clone(),
                                        actual: render_source_expr(arg),
                                        field_type: program_type_name(&field_type),
                                    },
                                );
                            }
                            RowCallableArgCheck::SignatureMismatch => {
                                diagnostics.push(
                                    ProgramDiagnostic::MethodSignatureMismatchForRowMember {
                                        def: def.to_string(),
                                        callee: callee_name.clone(),
                                        field: row_callable.field.clone(),
                                        actual: render_source_expr(arg),
                                    },
                                );
                            }
                            RowCallableArgCheck::MissingMember => {
                                diagnostics.push(ProgramDiagnostic::RowMemberCallableUnsatisfied {
                                    def: def.to_string(),
                                    callee: callee_name.clone(),
                                    field: row_callable.field.clone(),
                                    actual: render_source_expr(arg),
                                });
                            }
                        }
                    }
                }
            }
            collect_row_member_callable_diagnostics(
                def,
                callee,
                row_callables,
                member_records,
                diagnostics,
            );
            for arg in args {
                collect_row_member_callable_diagnostics(
                    def,
                    arg,
                    row_callables,
                    member_records,
                    diagnostics,
                );
            }
        }
        Expr::Lambda { body, .. }
        | Expr::Nominal { expr: body, .. }
        | Expr::Reset { body, .. }
        | Expr::Shift { body, .. } => {
            collect_row_member_callable_diagnostics(
                def,
                body,
                row_callables,
                member_records,
                diagnostics,
            );
        }
        Expr::Tuple(fields) | Expr::SliceLiteral(fields) => {
            for field in fields {
                collect_row_member_callable_diagnostics(
                    def,
                    field,
                    row_callables,
                    member_records,
                    diagnostics,
                );
            }
        }
        Expr::Record(fields) => {
            for field in fields {
                collect_row_member_callable_diagnostics(
                    def,
                    &field.value,
                    row_callables,
                    member_records,
                    diagnostics,
                );
            }
        }
        Expr::RecordUpdate { base, fields } => {
            collect_row_member_callable_diagnostics(
                def,
                base,
                row_callables,
                member_records,
                diagnostics,
            );
            for field in fields {
                collect_row_member_callable_diagnostics(
                    def,
                    &field.value,
                    row_callables,
                    member_records,
                    diagnostics,
                );
            }
        }
        Expr::AdtCtor { args, .. } => {
            for arg in args {
                collect_row_member_callable_diagnostics(
                    def,
                    arg,
                    row_callables,
                    member_records,
                    diagnostics,
                );
            }
        }
        Expr::Field { receiver, .. } => {
            collect_row_member_callable_diagnostics(
                def,
                receiver,
                row_callables,
                member_records,
                diagnostics,
            );
        }
        Expr::Index { receiver, index } => {
            collect_row_member_callable_diagnostics(
                def,
                receiver,
                row_callables,
                member_records,
                diagnostics,
            );
            collect_row_member_callable_diagnostics(
                def,
                index,
                row_callables,
                member_records,
                diagnostics,
            );
        }
        Expr::MethodCall { receiver, args, .. } => {
            collect_row_member_callable_diagnostics(
                def,
                receiver,
                row_callables,
                member_records,
                diagnostics,
            );
            for arg in args {
                collect_row_member_callable_diagnostics(
                    def,
                    arg,
                    row_callables,
                    member_records,
                    diagnostics,
                );
            }
        }
        Expr::Assign { target, value } => {
            collect_row_member_callable_diagnostics(
                def,
                target,
                row_callables,
                member_records,
                diagnostics,
            );
            collect_row_member_callable_diagnostics(
                def,
                value,
                row_callables,
                member_records,
                diagnostics,
            );
        }
        Expr::Range { start, end }
        | Expr::Binary {
            lhs: start,
            rhs: end,
            ..
        } => {
            collect_row_member_callable_diagnostics(
                def,
                start,
                row_callables,
                member_records,
                diagnostics,
            );
            collect_row_member_callable_diagnostics(
                def,
                end,
                row_callables,
                member_records,
                diagnostics,
            );
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_row_member_callable_diagnostics(
                def,
                cond,
                row_callables,
                member_records,
                diagnostics,
            );
            collect_row_member_callable_diagnostics(
                def,
                then_branch,
                row_callables,
                member_records,
                diagnostics,
            );
            collect_row_member_callable_diagnostics(
                def,
                else_branch,
                row_callables,
                member_records,
                diagnostics,
            );
        }
        Expr::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_row_member_callable_diagnostics(
                def,
                scrutinee,
                row_callables,
                member_records,
                diagnostics,
            );
            collect_row_member_callable_diagnostics(
                def,
                then_branch,
                row_callables,
                member_records,
                diagnostics,
            );
            collect_row_member_callable_diagnostics(
                def,
                else_branch,
                row_callables,
                member_records,
                diagnostics,
            );
        }
        Expr::Match { scrutinee, arms } => {
            collect_row_member_callable_diagnostics(
                def,
                scrutinee,
                row_callables,
                member_records,
                diagnostics,
            );
            for arm in arms {
                collect_row_member_callable_diagnostics(
                    def,
                    &arm.body,
                    row_callables,
                    member_records,
                    diagnostics,
                );
            }
        }
        Expr::Instantiate { callee, .. } => {
            collect_row_member_callable_diagnostics(
                def,
                callee,
                row_callables,
                member_records,
                diagnostics,
            );
        }
        Expr::Var(_) | Expr::Lit(_) => {}
    }
}

fn type_accepts_row_callable_args(ty: &Type, args: &[Expr]) -> bool {
    if !type_is_callable_storage(ty) {
        return false;
    }
    callable_param_types(ty)
        .map(|param_tys| row_callable_args_match_types(args, &param_tys))
        .unwrap_or(true)
}

fn callable_param_types(ty: &Type) -> Option<Vec<Type>> {
    match ty {
        Type::Func(param, result, _) => {
            let mut params = vec![param.as_ref().clone()];
            if let Some(mut rest) = callable_param_types(result) {
                params.append(&mut rest);
            }
            Some(params)
        }
        Type::Continuation { input, .. } => Some(vec![input.as_ref().clone()]),
        _ => None,
    }
}

fn row_callable_args_match_types(args: &[Expr], param_tys: &[Type]) -> bool {
    args.len() == param_tys.len()
        && args
            .iter()
            .zip(param_tys)
            .all(|(arg, ty)| row_callable_arg_matches_type(arg, ty))
}

fn row_callable_arg_matches_type(arg: &Expr, ty: &Type) -> bool {
    match source_expr_static_type(arg) {
        Some(actual) => actual == *ty || matches!(ty, Type::Unknown),
        None => true,
    }
}

fn source_expr_static_type(expr: &Expr) -> Option<Type> {
    match expr {
        Expr::Lit(crate::ast::Literal::I64(_)) => Some(Type::I64),
        Expr::Lit(crate::ast::Literal::Bool(_)) => Some(Type::Bool),
        Expr::Lit(crate::ast::Literal::Rune(_)) => Some(Type::Rune),
        _ => None,
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
                result_type: function
                    .return_type
                    .as_deref()
                    .map(source_type_name_to_type),
                result_ref_cell_lane: function
                    .return_type
                    .as_deref()
                    .map(source_type_name_to_type)
                    .and_then(|ty| core_ref_cell_lane_for_type(&ty)),
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
            ..
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
        .map(|param| backend_extern_type_name(param.as_deref()))
        .collect::<Vec<_>>()
        .join("_");
    format!(
        "{}_to_{}",
        params,
        backend_extern_type_name(return_type.as_deref())
    )
}

fn backend_extern_type_name(source: Option<&str>) -> String {
    match source.map(source_type_name_to_type) {
        Some(ty) if backend_value_kind_for_type(&ty) == Some(BackendValueKind::ExternRef) => {
            "externref".to_string()
        }
        _ => match source {
            Some("i64") => "i64".to_string(),
            Some("bool") => "bool".to_string(),
            Some("Unit") | Some("unit") | None => "unit".to_string(),
            Some(source) => source.to_string(),
        },
    }
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
        attrs,
        visibility,
        receiver,
        generics,
        generic_params,
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
        attrs: attrs.clone(),
        visibility: *visibility,
        receiver: receiver.clone(),
        generics: generics.clone(),
        generic_params: generic_params.clone(),
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
    let function_env = value_type_env_from_interface(interface, &current_namespace, &type_aliases);
    program
        .items
        .iter()
        .filter_map(|item| match item {
            SourceItem::Def {
                name,
                attrs,
                receiver,
                generics,
                generic_params,
                params,
                return_type,
                body,
                ..
            } => Some(ProgramDefOutput {
                name: name.clone(),
                receiver: receiver.as_ref().map(MethodReceiver::display_name),
                entry: attrs.contains(&ItemAttr::Entry),
                params: params.iter().map(|param| param.name.clone()).collect(),
                output: {
                    let mut output = compile_expr_with_indexes_and_generics(
                        name,
                        body,
                        names.clone(),
                        methods.clone(),
                        generics,
                        generic_params,
                        params,
                        return_type,
                        receiver,
                        &type_context,
                        &current_namespace,
                        &type_aliases,
                        &interface.functions,
                        &interface.types,
                        &interface.data,
                        &interface.constructors,
                        &interface.statics,
                        &function_env,
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

fn refresh_program_backends_with_lifted_callables(
    defs: &mut [ProgramDefOutput],
    interface: &InterfaceSummary,
) {
    let lifted = program_lifted_callable_abis(defs);
    let returned = program_returned_callable_abis(defs);
    let dyn_contracts = program_dyn_row_contract_param_types(defs);
    let dyn_method_targets = program_dyn_row_method_targets(defs);
    let dyn_function_abis = program_dyn_row_contract_function_abis(
        defs,
        interface,
        &dyn_contracts,
        &dyn_method_targets,
        &returned,
    );
    for def in defs {
        let mut extra_functions = lifted.clone();
        extra_functions.extend(dyn_function_abis.clone());
        let param_abi = backend_param_abi_with_extra_functions(
            &def.output.typed_signature,
            &interface.functions,
            &interface.types,
            &interface.data,
            &interface.constructors,
            &interface.statics,
            extra_functions,
            &returned,
            dyn_contracts.get(&def.name),
            dyn_method_targets.get(&def.name),
        );
        let backend = emit_wasm_gc_with_param_abi(
            &def.output.core,
            &def.output.core_validation,
            &def.params,
            &param_abi,
        );
        let backend_link = link_backend_artifacts(vec![backend.clone()]);
        let backend_cache_key = backend_cache_key(&backend_link, &BackendCacheConfig::default());
        def.output.backend = backend;
        def.output.backend_link = backend_link;
        def.output.backend_cache_key = backend_cache_key;
        def.output.visual.backend = crate::debug::render_backend_artifact(&def.output.backend);
        def.output.visual.backend_link =
            crate::debug::render_backend_link(&def.output.backend_link);
        def.output.visual.backend_cache_key =
            crate::debug::render_backend_cache_key(&def.output.backend_cache_key);
    }
}

fn program_dyn_row_contract_param_types(
    defs: &[ProgramDefOutput],
) -> BTreeMap<String, BTreeMap<String, Type>> {
    defs.iter()
        .filter_map(|def| {
            let contracts = dyn_row_contract_param_types_for_expr(&def.output.typed);
            (!contracts.is_empty()).then_some((def.name.clone(), contracts))
        })
        .collect()
}

fn dyn_row_contract_param_types_for_expr(expr: &TypedExpr) -> BTreeMap<String, Type> {
    let mut contracts = BTreeMap::new();
    collect_dyn_row_contract_param_types(expr, &mut contracts);
    contracts
}

fn collect_dyn_row_contract_param_types(expr: &TypedExpr, contracts: &mut BTreeMap<String, Type>) {
    match &expr.kind {
        TypedExprKind::DynRowPackage { payload, fields } => {
            if let TypedExprKind::Var(param) = &payload.kind {
                if fields.iter().any(|field| {
                    matches!(
                        field.source,
                        crate::typed::DynRowFieldSource::ContractObligation { .. }
                    )
                }) {
                    contracts.insert(param.clone(), expr.ty.clone());
                }
            }
            collect_dyn_row_contract_param_types(payload, contracts);
        }
        TypedExprKind::Lambda { body, .. } => collect_dyn_row_contract_param_types(body, contracts),
        TypedExprKind::Call { callee, args } => {
            collect_dyn_row_contract_param_types(callee, contracts);
            for arg in args {
                collect_dyn_row_contract_param_types(arg, contracts);
            }
        }
        TypedExprKind::Tuple { fields, .. } | TypedExprKind::SliceLiteral { items: fields, .. } => {
            for field in fields {
                collect_dyn_row_contract_param_types(field, contracts);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                collect_dyn_row_contract_param_types(&field.value, contracts);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            collect_dyn_row_contract_param_types(base, contracts);
            for field in fields {
                collect_dyn_row_contract_param_types(&field.value, contracts);
            }
        }
        TypedExprKind::DynRowField { package, .. }
        | TypedExprKind::Field {
            receiver: package, ..
        }
        | TypedExprKind::Nominal { expr: package, .. }
        | TypedExprKind::Reset { body: package, .. }
        | TypedExprKind::Shift { body: package, .. } => {
            collect_dyn_row_contract_param_types(package, contracts);
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_dyn_row_contract_param_types(arg, contracts);
            }
        }
        TypedExprKind::AdtToTuple { value, .. } => {
            collect_dyn_row_contract_param_types(value, contracts);
        }
        TypedExprKind::TupleToAdt { value, .. } => {
            collect_dyn_row_contract_param_types(value, contracts);
        }
        TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_dyn_row_contract_param_types(receiver, contracts);
            for arg in args {
                collect_dyn_row_contract_param_types(arg, contracts);
            }
        }
        TypedExprKind::Assign { target, value, .. } => {
            collect_dyn_row_contract_param_types(target, contracts);
            collect_dyn_row_contract_param_types(value, contracts);
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            collect_dyn_row_contract_param_types(receiver, contracts);
            collect_dyn_row_contract_param_types(index, contracts);
        }
        TypedExprKind::Range { start, end }
        | TypedExprKind::Binary {
            lhs: start,
            rhs: end,
            ..
        } => {
            collect_dyn_row_contract_param_types(start, contracts);
            collect_dyn_row_contract_param_types(end, contracts);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_dyn_row_contract_param_types(cond, contracts);
            collect_dyn_row_contract_param_types(then_branch, contracts);
            collect_dyn_row_contract_param_types(else_branch, contracts);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_dyn_row_contract_param_types(scrutinee, contracts);
            collect_dyn_row_contract_param_types(then_branch, contracts);
            collect_dyn_row_contract_param_types(else_branch, contracts);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_dyn_row_contract_param_types(scrutinee, contracts);
            for arm in arms {
                collect_dyn_row_contract_param_types(&arm.body, contracts);
            }
        }
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
    }
}

fn program_dyn_row_method_targets(defs: &[ProgramDefOutput]) -> DynRowMethodTargets {
    let signatures = defs
        .iter()
        .map(|def| (def.name.clone(), def.output.typed_signature.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut targets = BTreeMap::new();
    for def in defs {
        collect_dyn_row_method_targets(&def.output.typed, &signatures, &mut targets);
    }
    targets
}

fn collect_dyn_row_method_targets(
    expr: &TypedExpr,
    signatures: &BTreeMap<String, TypedSignature>,
    targets: &mut DynRowMethodTargets,
) {
    if let TypedExprKind::Call { callee, args } = &expr.kind {
        if let TypedExprKind::Var(callee_name) = &callee.kind {
            if let Some(signature) = signatures.get(callee_name) {
                for (param, arg) in signature.params.iter().zip(args) {
                    collect_dyn_row_arg_method_targets(callee_name, &param.name, arg, targets);
                }
            }
        }
    }
    match &expr.kind {
        TypedExprKind::DynRowPackage { payload, .. } => {
            collect_dyn_row_method_targets(payload, signatures, targets);
        }
        TypedExprKind::Lambda { body, .. } => {
            collect_dyn_row_method_targets(body, signatures, targets)
        }
        TypedExprKind::Call { callee, args } => {
            collect_dyn_row_method_targets(callee, signatures, targets);
            for arg in args {
                collect_dyn_row_method_targets(arg, signatures, targets);
            }
        }
        TypedExprKind::Tuple { fields, .. } | TypedExprKind::SliceLiteral { items: fields, .. } => {
            for field in fields {
                collect_dyn_row_method_targets(field, signatures, targets);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                collect_dyn_row_method_targets(&field.value, signatures, targets);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            collect_dyn_row_method_targets(base, signatures, targets);
            for field in fields {
                collect_dyn_row_method_targets(&field.value, signatures, targets);
            }
        }
        TypedExprKind::DynRowField { package, .. }
        | TypedExprKind::Field {
            receiver: package, ..
        }
        | TypedExprKind::Nominal { expr: package, .. }
        | TypedExprKind::Reset { body: package, .. }
        | TypedExprKind::Shift { body: package, .. } => {
            collect_dyn_row_method_targets(package, signatures, targets);
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_dyn_row_method_targets(arg, signatures, targets);
            }
        }
        TypedExprKind::AdtToTuple { value, .. } | TypedExprKind::TupleToAdt { value, .. } => {
            collect_dyn_row_method_targets(value, signatures, targets);
        }
        TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_dyn_row_method_targets(receiver, signatures, targets);
            for arg in args {
                collect_dyn_row_method_targets(arg, signatures, targets);
            }
        }
        TypedExprKind::Assign { target, value, .. } => {
            collect_dyn_row_method_targets(target, signatures, targets);
            collect_dyn_row_method_targets(value, signatures, targets);
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            collect_dyn_row_method_targets(receiver, signatures, targets);
            collect_dyn_row_method_targets(index, signatures, targets);
        }
        TypedExprKind::Range { start, end }
        | TypedExprKind::Binary {
            lhs: start,
            rhs: end,
            ..
        } => {
            collect_dyn_row_method_targets(start, signatures, targets);
            collect_dyn_row_method_targets(end, signatures, targets);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_dyn_row_method_targets(cond, signatures, targets);
            collect_dyn_row_method_targets(then_branch, signatures, targets);
            collect_dyn_row_method_targets(else_branch, signatures, targets);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_dyn_row_method_targets(scrutinee, signatures, targets);
            collect_dyn_row_method_targets(then_branch, signatures, targets);
            collect_dyn_row_method_targets(else_branch, signatures, targets);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_dyn_row_method_targets(scrutinee, signatures, targets);
            for arm in arms {
                collect_dyn_row_method_targets(&arm.body, signatures, targets);
            }
        }
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
    }
}

fn collect_dyn_row_arg_method_targets(
    callee: &str,
    param: &str,
    arg: &TypedExpr,
    targets: &mut DynRowMethodTargets,
) {
    let TypedExprKind::DynRowPackage { fields, .. } = &arg.kind else {
        return;
    };
    let methods = fields
        .iter()
        .filter_map(|field| {
            let crate::typed::DynRowFieldSource::ReceiverMethod { symbol, .. } = &field.source
            else {
                return None;
            };
            Some(BackendDynRowParamMethodAbi {
                field: field.name.clone(),
                target: symbol.clone(),
            })
        })
        .collect::<Vec<_>>();
    if methods.is_empty() {
        return;
    }
    let entry = targets
        .entry(callee.to_string())
        .or_insert_with(BTreeMap::new)
        .entry(param.to_string())
        .or_insert_with(Vec::new);
    for method in methods {
        if !entry.contains(&method) {
            entry.push(method);
        }
    }
}

fn program_dyn_row_contract_function_abis(
    defs: &[ProgramDefOutput],
    interface: &InterfaceSummary,
    contracts: &BTreeMap<String, BTreeMap<String, Type>>,
    method_targets: &DynRowMethodTargets,
    returned_callables: &BTreeMap<String, BackendReturnedCallableAbi>,
) -> BTreeMap<String, BackendCallableAbi> {
    let function_abis = backend_callable_abis_for_interface(
        &interface.functions,
        &interface.types,
        &interface.data,
        &interface.constructors,
        returned_callables,
    );
    defs.iter()
        .filter_map(|def| {
            let param_contracts = contracts.get(&def.name);
            let param_method_targets = method_targets.get(&def.name);
            if param_contracts.is_none() && param_method_targets.is_none() {
                return None;
            }
            let base = function_abis.get(&def.name)?;
            let expansions = def
                .output
                .typed_signature
                .params
                .iter()
                .zip(base.arg_expansions.iter())
                .map(|(param, expansion)| {
                    let mut expanded = param_contracts
                        .and_then(|contracts| contracts.get(&param.name))
                        .and_then(|ty| dyn_row_contract_arg_expansion(ty, &interface.functions))
                        .unwrap_or_else(|| expansion.clone());
                    if let BackendCallableArgExpansion::DynRow { needs_payload, .. } = &mut expanded
                    {
                        if param_method_targets
                            .and_then(|targets| targets.get(&param.name))
                            .is_some_and(|targets| !targets.is_empty())
                        {
                            *needs_payload = true;
                        }
                    }
                    expanded
                })
                .collect::<Vec<_>>();
            let params = expansions
                .iter()
                .flat_map(backend_arg_expansion_param_kinds)
                .collect::<Vec<_>>();
            Some((
                def.name.clone(),
                BackendCallableAbi {
                    params,
                    arg_expansions: expansions,
                    result: base.result,
                    result_ref_cell_lane: base.result_ref_cell_lane,
                    return_callable: base.return_callable.clone(),
                    return_dyn_row_methods: base.return_dyn_row_methods.clone(),
                },
            ))
        })
        .collect()
}

fn dyn_row_contract_arg_expansion(
    ty: &Type,
    functions: &[crate::surface::InterfaceFunction],
) -> Option<BackendCallableArgExpansion> {
    let Type::DynRow(fields) = ty else {
        return None;
    };
    Some(backend_dyn_row_arg_expansion(fields, functions))
}

fn backend_dyn_row_arg_expansion(
    fields: &[RecordTypeField],
    functions: &[crate::surface::InterfaceFunction],
) -> BackendCallableArgExpansion {
    let members = backend_dyn_row_member_abis(&Type::DynRow(fields.to_vec()), functions);
    BackendCallableArgExpansion::DynRow {
        fields: backend_dyn_row_field_members(&members),
        needs_payload: backend_dyn_row_members_need_payload(&members),
    }
}

fn backend_arg_expansion_param_kinds(
    expansion: &BackendCallableArgExpansion,
) -> Vec<BackendValueKind> {
    match expansion {
        BackendCallableArgExpansion::Direct(kind) => vec![*kind],
        BackendCallableArgExpansion::AdtTag { payloads, .. } => {
            let mut params = vec![BackendValueKind::I32];
            params.extend(payloads.iter().flat_map(backend_adt_payload_param_kinds));
            params
        }
        BackendCallableArgExpansion::Callable { env, .. } => {
            let mut params = vec![BackendValueKind::I32];
            params.extend(env.iter().copied());
            params
        }
        BackendCallableArgExpansion::DynRow {
            fields,
            needs_payload,
        } => {
            let mut params = Vec::new();
            if *needs_payload {
                params.push(BackendValueKind::I32);
            }
            params.extend(fields.iter().map(|field| field.kind));
            params
        }
        BackendCallableArgExpansion::StaticRowFields(fields) => {
            fields.iter().map(|field| field.kind).collect()
        }
    }
}

fn backend_adt_payload_param_kinds(payload: &BackendAdtPayloadAbi) -> Vec<BackendValueKind> {
    let mut kinds = vec![payload.kind];
    if let Some(nested) = &payload.nested {
        kinds.extend(
            nested
                .payloads
                .iter()
                .flat_map(backend_adt_payload_param_kinds),
        );
    }
    kinds
}

fn program_lifted_callable_abis(defs: &[ProgramDefOutput]) -> BTreeMap<String, BackendCallableAbi> {
    defs.iter()
        .flat_map(|def| def.output.core.ops.iter())
        .filter_map(|op| {
            let CoreOp::LiftedFunction {
                symbol,
                env_params,
                param,
                body,
                ..
            } = op
            else {
                return None;
            };
            if param.is_none() || body.is_empty() {
                return None;
            }
            let mut params = vec![BackendValueKind::I32; env_params.len()];
            params.push(BackendValueKind::I32);
            Some((
                symbol.clone(),
                BackendCallableAbi {
                    params,
                    arg_expansions: vec![BackendCallableArgExpansion::Direct(
                        BackendValueKind::I32,
                    )],
                    result: Some(BackendValueKind::I32),
                    result_ref_cell_lane: None,
                    return_callable: None,
                    return_dyn_row_methods: Vec::new(),
                },
            ))
        })
        .collect()
}

fn program_returned_callable_abis(
    defs: &[ProgramDefOutput],
) -> BTreeMap<String, BackendReturnedCallableAbi> {
    defs.iter()
        .filter_map(|def| {
            let returned = def.output.core.ops.iter().find_map(|op| {
                let CoreOp::ReturnValue(CoreValue::LiftedFunction { symbol, .. }) = op else {
                    return None;
                };
                let env = def
                    .output
                    .core
                    .ops
                    .iter()
                    .find_map(|op| {
                        let CoreOp::LiftedFunction {
                            symbol: candidate,
                            env_params,
                            ..
                        } = op
                        else {
                            return None;
                        };
                        (candidate == symbol).then(|| vec![BackendValueKind::I32; env_params.len()])
                    })
                    .unwrap_or_default();
                Some(BackendReturnedCallableAbi {
                    target: symbol.clone(),
                    env,
                })
            })?;
            Some((def.name.clone(), returned))
        })
        .collect()
}

fn interface_without_invalid_static_initializers(
    interface: &InterfaceSummary,
    global_init: &GlobalInitPlan,
) -> InterfaceSummary {
    let invalid = global_init
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic {
            GlobalInitDiagnostic::UnsupportedStaticInitializer { static_name, .. }
            | GlobalInitDiagnostic::InvalidStaticAdtConstructor { static_name, .. } => {
                Some(static_name.as_str())
            }
            GlobalInitDiagnostic::DuplicateStatic { .. }
            | GlobalInitDiagnostic::StaticFunctionNameConflict { .. }
            | GlobalInitDiagnostic::StaticInitCycle { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    if invalid.is_empty() {
        return interface.clone();
    }
    let mut filtered = interface.clone();
    filtered
        .statics
        .retain(|static_value| !invalid.contains(static_value.source_name.as_str()));
    filtered
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

    pub fn type_env(&self, function_env: &TypeEnv) -> TypeEnv {
        function_env
            .iter()
            .map(|(name, ty)| (name.clone(), ty.clone()))
            .chain(
                self.params
                    .iter()
                    .flat_map(|param| param.binding_types.clone()),
            )
            .collect()
    }
}

fn function_type_env_from_interface(
    interface: &InterfaceSummary,
    current_namespace: &str,
    type_aliases: &TypeAliasIndex,
) -> TypeEnv {
    interface
        .functions
        .iter()
        .filter(|function| function.receiver.is_none())
        .filter(|function| function.owner == current_namespace)
        .map(|function| {
            (
                function.source_name.clone(),
                function_type_from_interface(function, current_namespace, type_aliases),
            )
        })
        .collect()
}

fn value_type_env_from_interface(
    interface: &InterfaceSummary,
    current_namespace: &str,
    type_aliases: &TypeAliasIndex,
) -> TypeEnv {
    let mut env = function_type_env_from_interface(interface, current_namespace, type_aliases);
    for static_value in &interface.statics {
        if static_value.owner != current_namespace {
            continue;
        }
        let Some(ty) = static_value.ty.as_deref() else {
            continue;
        };
        let resolved = resolve_header_type(Some(ty), &None, current_namespace, type_aliases);
        env.insert(
            static_value.source_name.clone(),
            source_type_name_to_type(&resolved),
        );
    }
    env
}

fn function_type_from_interface(
    function: &crate::surface::InterfaceFunction,
    current_namespace: &str,
    type_aliases: &TypeAliasIndex,
) -> Type {
    let result = function
        .return_type
        .as_deref()
        .map(|ty| resolve_header_type(Some(ty), &None, current_namespace, type_aliases))
        .map(|ty| source_type_name_to_type(&ty))
        .unwrap_or(Type::Unknown);
    function
        .param_types
        .iter()
        .rev()
        .fold(result, |result, param| {
            Type::Func(
                Box::new(
                    param
                        .as_deref()
                        .map(|ty| {
                            resolve_header_type(Some(ty), &None, current_namespace, type_aliases)
                        })
                        .map(|ty| source_type_name_to_type(&ty))
                        .unwrap_or(Type::Unknown),
                ),
                Box::new(result),
                crate::typed::SendColor::Obligation,
            )
        })
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
            Type::Continuation { multi, .. } => Some(continuation_storage_fact(
                format!("param::{}", param.name),
                multi,
                if multi {
                    crate::typed::UsageColor::Many
                } else {
                    param_binders
                        .iter()
                        .find(|binder| binder.name == param.name)
                        .and_then(|binder| usage.binders.get(&binder.id).copied())
                        .unwrap_or(crate::usage::UseCount::Zero)
                        .color()
                },
            )),
            Type::Func(_, _, send) => Some(callable_storage_fact(
                format!("param::{}", param.name),
                send,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    if let Some(return_type) = &signature.return_type {
        match source_type_name_to_type(return_type) {
            Type::Continuation { multi, .. } => {
                facts.push(continuation_storage_fact(
                    format!("return::{def_name}"),
                    multi,
                    if multi {
                        crate::typed::UsageColor::Many
                    } else {
                        crate::typed::UsageColor::One
                    },
                ));
            }
            Type::Func(_, _, send) => {
                facts.push(callable_storage_fact(format!("return::{def_name}"), send));
            }
            _ => {}
        }
    }
    facts
}

fn continuation_storage_fact(
    subject: String,
    multi: bool,
    usage: crate::typed::UsageColor,
) -> CallableStorageFact {
    CallableStorageFact {
        subject,
        kind: if multi {
            CallableStorageKind::ContNPackage
        } else {
            CallableStorageKind::BoxedCont1
        },
        usage,
        send: crate::typed::SendColor::NotSend,
    }
}

fn callable_storage_fact(subject: String, send: crate::typed::SendColor) -> CallableStorageFact {
    CallableStorageFact {
        subject,
        kind: CallableStorageKind::ErasedCallableAdt,
        usage: crate::typed::UsageColor::Many,
        send,
    }
}

fn interface_callable_storage_facts(interface: &InterfaceSummary) -> Vec<CallableStorageFact> {
    interface
        .types
        .iter()
        .flat_map(|ty| {
            ty.fields
                .iter()
                .filter_map(|field| match source_type_name_to_type(&field.ty) {
                    Type::Continuation { multi, .. } => Some(continuation_storage_fact(
                        format!("type::{}::{}", ty.name, field.name),
                        multi,
                        if multi {
                            crate::typed::UsageColor::Many
                        } else {
                            crate::typed::UsageColor::One
                        },
                    )),
                    Type::Func(_, _, send) => Some(callable_storage_fact(
                        format!("type::{}::{}", ty.name, field.name),
                        send,
                    )),
                    _ => None,
                })
        })
        .collect()
}

fn backend_param_abi(
    signature: &TypedSignature,
    functions: &[crate::surface::InterfaceFunction],
    types: &[crate::surface::InterfaceType],
    data: &[crate::surface::InterfaceData],
    constructors: &[crate::surface::InterfaceConstructor],
    statics: &[crate::surface::InterfaceStatic],
) -> BackendParamAbi {
    backend_param_abi_with_extra_functions(
        signature,
        functions,
        types,
        data,
        constructors,
        statics,
        BTreeMap::new(),
        &BTreeMap::new(),
        None,
        None,
    )
}

fn backend_param_abi_with_extra_functions(
    signature: &TypedSignature,
    functions: &[crate::surface::InterfaceFunction],
    types: &[crate::surface::InterfaceType],
    data: &[crate::surface::InterfaceData],
    constructors: &[crate::surface::InterfaceConstructor],
    statics: &[crate::surface::InterfaceStatic],
    extra_functions: BTreeMap<String, BackendCallableAbi>,
    returned_callables: &BTreeMap<String, BackendReturnedCallableAbi>,
    extra_dyn_param_types: Option<&BTreeMap<String, Type>>,
    extra_dyn_param_methods: Option<&BTreeMap<String, Vec<BackendDynRowParamMethodAbi>>>,
) -> BackendParamAbi {
    let mut function_abis = backend_callable_abis_for_interface(
        functions,
        types,
        data,
        constructors,
        returned_callables,
    );
    function_abis.extend(extra_functions);
    let mut dyn_row_param_members =
        backend_dyn_row_param_members_for_signature(signature, types, functions);
    if let Some(extra_dyn_param_types) = extra_dyn_param_types {
        dyn_row_param_members.extend(backend_dyn_row_param_members_for_param_types(
            extra_dyn_param_types,
            functions,
        ));
    }
    if let Some(extra_dyn_param_methods) = extra_dyn_param_methods {
        for (param, methods) in extra_dyn_param_methods {
            let entry = dyn_row_param_members
                .entry(param.clone())
                .or_insert_with(Vec::new);
            for method in methods {
                let member = BackendDynRowMemberAbi::ReceiverMethod(method.clone());
                if !entry.contains(&member) {
                    entry.push(member);
                }
            }
        }
    }
    let dyn_row_param_fields = backend_dyn_row_param_field_members_by_param(&dyn_row_param_members);
    let dyn_row_param_methods =
        backend_dyn_row_param_method_members_by_param(&dyn_row_param_members);
    BackendParamAbi {
        params: signature
            .params
            .iter()
            .filter_map(|param| {
                let ty = source_type_name_to_type(&param.ty);
                match ty {
                    Type::Func(..) => Some(BackendValueKind::I32),
                    ty => backend_value_kind_for_type(&ty)
                        .or_else(|| backend_storage_value_kind_for_type(&ty)),
                }
                .map(|kind| (param.name.clone(), kind))
            })
            .collect(),
        callable_params: backend_callable_params_for_signature(signature),
        functions: function_abis,
        statics: backend_static_abis_for_interface(statics),
        static_ref_cell_lanes: backend_static_ref_cell_lanes_for_interface(statics),
        adt_tag_params: backend_adt_tag_params_for_signature(signature, data, constructors),
        dyn_row_param_fields,
        dyn_row_param_methods,
        aggregate_element_lanes: backend_aggregate_element_lanes_for_signature(signature)
            .into_iter()
            .chain(backend_static_aggregate_element_lanes_for_interface(
                statics,
            ))
            .collect(),
    }
}

fn backend_adt_tag_params_for_signature(
    signature: &TypedSignature,
    data: &[crate::surface::InterfaceData],
    constructors: &[crate::surface::InterfaceConstructor],
) -> BTreeMap<String, BackendAdtTagAbi> {
    signature
        .params
        .iter()
        .filter_map(|param| {
            let ty = source_type_name_to_type(&param.ty);
            let abi = match ty {
                Type::Adt { .. } | Type::Nominal(_) => interface_data_for_type(data, &ty)
                    .map(|data| backend_adt_tag_abi_for_type(data, constructors, &ty))?,
                _ => return None,
            };
            Some((param.name.clone(), abi))
        })
        .collect()
}

fn backend_adt_tag_abi_for_data(
    data: &crate::surface::InterfaceData,
    constructors: &[crate::surface::InterfaceConstructor],
) -> BackendAdtTagAbi {
    backend_adt_tag_abi_for_type(data, constructors, &Type::Nominal(data.name.clone()))
}

fn backend_adt_tag_abi_for_type(
    data: &crate::surface::InterfaceData,
    constructors: &[crate::surface::InterfaceConstructor],
    ty: &Type,
) -> BackendAdtTagAbi {
    backend_adt_tag_abi_for_type_with_depth(data, constructors, ty, 4)
}

fn backend_adt_tag_abi_for_type_with_depth(
    data: &crate::surface::InterfaceData,
    constructors: &[crate::surface::InterfaceConstructor],
    ty: &Type,
    depth: usize,
) -> BackendAdtTagAbi {
    let substitutions = adt_type_substitutions(data, ty);
    BackendAdtTagAbi {
        data: data.name.clone(),
        variants: data.variants.clone(),
        payloads: constructors
            .iter()
            .filter(|ctor| ctor.data_symbol == data.symbol)
            .flat_map(|ctor| {
                ctor.payload_types
                    .iter()
                    .enumerate()
                    .filter_map(|(index, payload)| {
                        let ty = substitute_backend_type_params(
                            &source_type_name_to_type(payload),
                            &substitutions,
                        );
                        backend_adt_payload_abi_for_type(
                            data,
                            constructors,
                            &ctor.name,
                            index,
                            &ty,
                            depth,
                        )
                    })
            })
            .collect(),
    }
}

fn backend_adt_payload_abi_for_type(
    data: &crate::surface::InterfaceData,
    constructors: &[crate::surface::InterfaceConstructor],
    variant: &str,
    index: usize,
    ty: &Type,
    depth: usize,
) -> Option<BackendAdtPayloadAbi> {
    if let Some(kind) = backend_storage_value_kind_for_type(ty) {
        return Some(BackendAdtPayloadAbi {
            variant: variant.to_string(),
            index,
            kind,
            nested: None,
        });
    }
    if depth == 0 || interface_data_for_type(std::slice::from_ref(data), ty).is_none() {
        return None;
    }
    Some(BackendAdtPayloadAbi {
        variant: variant.to_string(),
        index,
        kind: BackendValueKind::I32,
        nested: Some(Box::new(backend_adt_tag_abi_for_type_with_depth(
            data,
            constructors,
            ty,
            depth - 1,
        ))),
    })
}

fn adt_type_substitutions(
    data: &crate::surface::InterfaceData,
    ty: &Type,
) -> BTreeMap<String, Type> {
    let Some(base) = nominal_base_name_for_type(ty) else {
        return BTreeMap::new();
    };
    if base != data.name && base != data.symbol {
        return BTreeMap::new();
    }
    data.generics
        .iter()
        .cloned()
        .zip(nominal_type_args_for_type(ty).unwrap_or_default())
        .collect()
}

fn substitute_backend_type_params(ty: &Type, substitutions: &BTreeMap<String, Type>) -> Type {
    match ty {
        Type::Nominal(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| Type::Nominal(name.clone())),
        Type::Tuple(fields) => Type::Tuple(
            fields
                .iter()
                .map(|field| substitute_backend_type_params(field, substitutions))
                .collect(),
        ),
        Type::Record(fields) => Type::Record(
            fields
                .iter()
                .map(|field| RecordTypeField {
                    name: field.name.clone(),
                    ty: substitute_backend_type_params(&field.ty, substitutions),
                })
                .collect(),
        ),
        Type::DynRow(fields) => Type::DynRow(
            fields
                .iter()
                .map(|field| RecordTypeField {
                    name: field.name.clone(),
                    ty: substitute_backend_type_params(&field.ty, substitutions),
                })
                .collect(),
        ),
        Type::Func(param, result, send) => Type::Func(
            Box::new(substitute_backend_type_params(param, substitutions)),
            Box::new(substitute_backend_type_params(result, substitutions)),
            *send,
        ),
        Type::Continuation {
            input,
            answer,
            multi,
        } => Type::Continuation {
            input: Box::new(substitute_backend_type_params(input, substitutions)),
            answer: Box::new(substitute_backend_type_params(answer, substitutions)),
            multi: *multi,
        },
        Type::Adt { name, variants } => Type::Adt {
            name: name.clone(),
            variants: variants.clone(),
        },
        Type::Unknown | Type::I64 | Type::Rune | Type::Bool => ty.clone(),
    }
}

fn backend_adt_arg_expansion(abi: &BackendAdtTagAbi) -> BackendCallableArgExpansion {
    BackendCallableArgExpansion::AdtTag {
        data: abi.data.clone(),
        variants: abi.variants.clone(),
        payloads: abi.payloads.clone(),
    }
}

fn interface_data_for_type<'a>(
    data: &'a [crate::surface::InterfaceData],
    ty: &Type,
) -> Option<&'a crate::surface::InterfaceData> {
    let base = nominal_base_name_for_type(ty)?;
    data.iter()
        .find(|item| base == item.name || base == item.symbol)
}

fn backend_callable_params_for_signature(
    signature: &TypedSignature,
) -> BTreeMap<String, BackendCallableAbi> {
    signature
        .params
        .iter()
        .filter_map(|param| {
            let Type::Func(input, result, _) = source_type_name_to_type(&param.ty) else {
                return None;
            };
            let input_kind = backend_value_kind_for_type(&input)
                .or_else(|| backend_storage_value_kind_for_type(&input))?;
            let result_kind = backend_value_kind_for_type(&result)
                .or_else(|| backend_storage_value_kind_for_type(&result));
            Some((
                param.name.clone(),
                BackendCallableAbi {
                    params: vec![BackendValueKind::I32, input_kind],
                    arg_expansions: vec![BackendCallableArgExpansion::Direct(input_kind)],
                    result: result_kind,
                    result_ref_cell_lane: None,
                    return_callable: None,
                    return_dyn_row_methods: Vec::new(),
                },
            ))
        })
        .collect()
}

fn backend_dyn_row_param_members_for_signature(
    signature: &TypedSignature,
    types: &[crate::surface::InterfaceType],
    functions: &[crate::surface::InterfaceFunction],
) -> BTreeMap<String, Vec<BackendDynRowMemberAbi>> {
    let static_row_members = backend_static_row_members_by_type(types);
    signature
        .params
        .iter()
        .filter_map(|param| {
            let ty = source_type_name_to_type(&param.ty);
            let members = match ty {
                Type::DynRow(_) => backend_dyn_row_member_abis(&ty, functions),
                Type::Record(fields) => backend_dyn_row_field_member_abis(&fields),
                Type::Nominal(name)
                    if !backend_nominal_is_externref(&Type::Nominal(name.clone())) =>
                {
                    static_row_members.get(&name).cloned().unwrap_or_default()
                }
                _ => Vec::new(),
            };
            (!members.is_empty()).then_some((param.name.clone(), members))
        })
        .collect()
}

fn backend_dyn_row_field_abis(fields: &[RecordTypeField]) -> Vec<BackendDynRowParamFieldAbi> {
    backend_dyn_row_field_member_abis(fields)
        .into_iter()
        .filter_map(|member| match member {
            BackendDynRowMemberAbi::Field(field) => Some(field),
            BackendDynRowMemberAbi::ReceiverMethod(_) => None,
        })
        .collect()
}

fn backend_dyn_row_field_member_abis(fields: &[RecordTypeField]) -> Vec<BackendDynRowMemberAbi> {
    fields
        .iter()
        .filter_map(|field| {
            backend_storage_value_kind_for_type(&field.ty).map(|kind| {
                BackendDynRowMemberAbi::Field(BackendDynRowParamFieldAbi {
                    field: field.name.clone(),
                    kind,
                })
            })
        })
        .collect()
}

fn backend_static_row_members_by_type(
    types: &[crate::surface::InterfaceType],
) -> BTreeMap<String, Vec<BackendDynRowMemberAbi>> {
    types
        .iter()
        .filter(|ty| ty.alias_target.is_none())
        .filter_map(|ty| {
            let members = ty
                .fields
                .iter()
                .filter_map(|field| {
                    let field_ty = source_type_name_to_type(&field.ty);
                    backend_storage_value_kind_for_type(&field_ty).map(|kind| {
                        BackendDynRowMemberAbi::Field(BackendDynRowParamFieldAbi {
                            field: field.name.clone(),
                            kind,
                        })
                    })
                })
                .collect::<Vec<_>>();
            (!members.is_empty()).then_some((nominal_display_name(&ty.name, &ty.generics), members))
        })
        .collect()
}

fn backend_static_row_fields_by_type(
    types: &[crate::surface::InterfaceType],
) -> BTreeMap<String, Vec<BackendDynRowParamFieldAbi>> {
    backend_static_row_members_by_type(types)
        .into_iter()
        .filter_map(|(ty, members)| {
            let fields = backend_dyn_row_field_members(&members);
            (!fields.is_empty()).then_some((ty, fields))
        })
        .collect()
}

fn backend_dyn_row_param_members_for_param_types(
    param_types: &BTreeMap<String, Type>,
    functions: &[crate::surface::InterfaceFunction],
) -> BTreeMap<String, Vec<BackendDynRowMemberAbi>> {
    param_types
        .iter()
        .filter_map(|(param, ty)| {
            let Type::DynRow(_) = ty else {
                return None;
            };
            let members = backend_dyn_row_member_abis(ty, functions);
            (!members.is_empty()).then_some((param.clone(), members))
        })
        .collect()
}

fn backend_dyn_row_methods_for_type(
    ty: &Type,
    functions: &[crate::surface::InterfaceFunction],
) -> Vec<BackendDynRowParamMethodAbi> {
    backend_dyn_row_method_members(&backend_dyn_row_member_abis(ty, functions))
}

fn backend_dyn_row_member_abis(
    ty: &Type,
    functions: &[crate::surface::InterfaceFunction],
) -> Vec<BackendDynRowMemberAbi> {
    let Type::DynRow(fields) = ty else {
        return Vec::new();
    };
    let mut members = backend_dyn_row_field_member_abis(fields);
    members.extend(fields.iter().filter_map(|field| {
        let Type::Func(..) = field.ty else {
            return None;
        };
        receiver_method_for_dyn_row_field(field, functions).map(|function| {
            BackendDynRowMemberAbi::ReceiverMethod(BackendDynRowParamMethodAbi {
                field: field.name.clone(),
                target: function.symbol.clone(),
            })
        })
    }));
    members
}

fn backend_dyn_row_field_members(
    members: &[BackendDynRowMemberAbi],
) -> Vec<BackendDynRowParamFieldAbi> {
    members
        .iter()
        .filter_map(|member| match member {
            BackendDynRowMemberAbi::Field(field) => Some(field.clone()),
            BackendDynRowMemberAbi::ReceiverMethod(_) => None,
        })
        .collect()
}

fn backend_dyn_row_method_members(
    members: &[BackendDynRowMemberAbi],
) -> Vec<BackendDynRowParamMethodAbi> {
    members
        .iter()
        .filter_map(|member| match member {
            BackendDynRowMemberAbi::Field(_) => None,
            BackendDynRowMemberAbi::ReceiverMethod(method) => Some(method.clone()),
        })
        .collect()
}

fn backend_dyn_row_members_need_payload(members: &[BackendDynRowMemberAbi]) -> bool {
    members
        .iter()
        .any(|member| matches!(member, BackendDynRowMemberAbi::ReceiverMethod(_)))
}

fn backend_dyn_row_param_field_members_by_param(
    members_by_param: &BTreeMap<String, Vec<BackendDynRowMemberAbi>>,
) -> BTreeMap<String, Vec<BackendDynRowParamFieldAbi>> {
    members_by_param
        .iter()
        .filter_map(|(param, members)| {
            let fields = backend_dyn_row_field_members(members);
            (!fields.is_empty()).then_some((param.clone(), fields))
        })
        .collect()
}

fn backend_dyn_row_param_method_members_by_param(
    members_by_param: &BTreeMap<String, Vec<BackendDynRowMemberAbi>>,
) -> BTreeMap<String, Vec<BackendDynRowParamMethodAbi>> {
    members_by_param
        .iter()
        .filter_map(|(param, members)| {
            let methods = backend_dyn_row_method_members(members);
            (!methods.is_empty()).then_some((param.clone(), methods))
        })
        .collect()
}

fn receiver_method_for_dyn_row_field<'a>(
    field: &RecordTypeField,
    functions: &'a [crate::surface::InterfaceFunction],
) -> Option<&'a crate::surface::InterfaceFunction> {
    let mut matches = functions
        .iter()
        .filter(|function| function.receiver.is_some())
        .filter(|function| {
            function.source_name == field.name && dyn_row_method_field_type(function) == field.ty
        });
    let method = matches.next()?;
    matches.next().is_none().then_some(method)
}

fn dyn_row_method_field_type(function: &crate::surface::InterfaceFunction) -> Type {
    let result = function
        .return_type
        .as_deref()
        .map(source_type_name_to_type)
        .unwrap_or(Type::Unknown);
    let params = function
        .param_types
        .iter()
        .skip(1)
        .map(|param| {
            param
                .as_deref()
                .map(source_type_name_to_type)
                .unwrap_or(Type::Unknown)
        })
        .collect::<Vec<_>>();
    Type::Func(
        Box::new(method_bound_param_type(&params)),
        Box::new(result),
        crate::typed::SendColor::Obligation,
    )
}

fn method_bound_param_type(params: &[Type]) -> Type {
    match params {
        [] => Type::Nominal("Unit".to_string()),
        [single] => single.clone(),
        many => Type::Tuple(many.to_vec()),
    }
}

fn backend_aggregate_element_lanes_for_signature(
    signature: &TypedSignature,
) -> BTreeMap<String, BackendValueKind> {
    signature
        .params
        .iter()
        .filter_map(|param| {
            let ty = source_type_name_to_type(&param.ty);
            aggregate_element_lane_for_type(&ty).map(|lane| (param.name.clone(), lane))
        })
        .collect()
}

fn backend_static_aggregate_element_lanes_for_interface(
    statics: &[crate::surface::InterfaceStatic],
) -> BTreeMap<String, BackendValueKind> {
    statics
        .iter()
        .filter_map(|static_value| {
            let ty = static_value.ty.as_deref().map(source_type_name_to_type)?;
            aggregate_element_lane_for_type(&ty)
                .map(|lane| (static_value.source_name.clone(), lane))
        })
        .collect()
}

fn backend_static_abis_for_interface(
    statics: &[crate::surface::InterfaceStatic],
) -> BTreeMap<String, BackendValueKind> {
    statics
        .iter()
        .filter_map(|static_value| {
            let ty = static_value.ty.as_deref().map(source_type_name_to_type)?;
            backend_value_kind_for_type(&ty)
                .or_else(|| backend_storage_value_kind_for_type(&ty))
                .map(|kind| (static_value.source_name.clone(), kind))
        })
        .collect()
}

fn backend_static_ref_cell_lanes_for_interface(
    statics: &[crate::surface::InterfaceStatic],
) -> BTreeMap<String, crate::core::CoreRefCellLane> {
    statics
        .iter()
        .filter_map(|static_value| {
            let ty = static_value.ty.as_deref().map(source_type_name_to_type)?;
            core_ref_cell_lane_for_type(&ty).map(|lane| (static_value.source_name.clone(), lane))
        })
        .collect()
}

fn backend_callable_abis_for_interface(
    functions: &[crate::surface::InterfaceFunction],
    types: &[crate::surface::InterfaceType],
    data: &[crate::surface::InterfaceData],
    constructors: &[crate::surface::InterfaceConstructor],
    returned_callables: &BTreeMap<String, BackendReturnedCallableAbi>,
) -> BTreeMap<String, BackendCallableAbi> {
    let mut abis = BTreeMap::new();
    let static_row_fields = backend_static_row_fields_by_type(types);
    let data_abis = data
        .iter()
        .map(|data| {
            (
                data.name.clone(),
                backend_adt_tag_abi_for_data(data, constructors),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for function in functions.iter() {
        let arg_expansions = function
            .param_types
            .iter()
            .map(|param| {
                let ty = param.as_deref().map(source_type_name_to_type);
                match &ty {
                    Some(Type::DynRow(fields)) => backend_dyn_row_arg_expansion(&fields, functions),
                    Some(Type::Adt { .. }) => {
                        interface_data_for_type(data, ty.as_ref().expect("matched Some"))
                            .map(|data| {
                                backend_adt_tag_abi_for_type(
                                    data,
                                    constructors,
                                    ty.as_ref().expect("matched Some"),
                                )
                            })
                            .map(|abi| backend_adt_arg_expansion(&abi))
                            .unwrap_or(BackendCallableArgExpansion::Direct(BackendValueKind::I32))
                    }
                    Some(Type::Nominal(_))
                        if ty
                            .as_ref()
                            .and_then(nominal_base_name_for_type)
                            .is_some_and(|name| {
                                data_abis.contains_key(name)
                                    || data.iter().any(|data| data.symbol == name)
                            }) =>
                    {
                        interface_data_for_type(data, ty.as_ref().expect("matched Some"))
                            .map(|data| {
                                backend_adt_tag_abi_for_type(
                                    data,
                                    constructors,
                                    ty.as_ref().expect("matched Some"),
                                )
                            })
                            .map(|abi| backend_adt_arg_expansion(&abi))
                            .unwrap_or(BackendCallableArgExpansion::Direct(BackendValueKind::I32))
                    }
                    Some(Type::Record(fields)) => BackendCallableArgExpansion::StaticRowFields(
                        backend_dyn_row_field_abis(&fields),
                    ),
                    Some(Type::Nominal(ref name))
                        if static_row_fields.contains_key(name)
                            && !backend_nominal_is_externref(&Type::Nominal(name.clone())) =>
                    {
                        BackendCallableArgExpansion::StaticRowFields(
                            static_row_fields.get(name).cloned().unwrap_or_default(),
                        )
                    }
                    Some(Type::Func(input, result, _)) => BackendCallableArgExpansion::Callable {
                        params: backend_value_kind_for_type(&input)
                            .or_else(|| backend_storage_value_kind_for_type(&input))
                            .into_iter()
                            .collect(),
                        env: vec![BackendValueKind::I32],
                        result: backend_value_kind_for_type(&result)
                            .or_else(|| backend_storage_value_kind_for_type(&result)),
                    },
                    Some(ty) => BackendCallableArgExpansion::Direct(
                        backend_value_kind_for_type(&ty)
                            .or_else(|| backend_storage_value_kind_for_type(&ty))
                            .unwrap_or(BackendValueKind::I32),
                    ),
                    None => BackendCallableArgExpansion::Direct(BackendValueKind::I32),
                }
            })
            .collect::<Vec<_>>();
        let params = arg_expansions
            .iter()
            .flat_map(|expansion| match expansion {
                BackendCallableArgExpansion::Direct(kind) => vec![*kind],
                BackendCallableArgExpansion::AdtTag { payloads, .. } => {
                    let mut params = vec![BackendValueKind::I32];
                    params.extend(payloads.iter().flat_map(backend_adt_payload_param_kinds));
                    params
                }
                BackendCallableArgExpansion::Callable { env, .. } => {
                    let mut params = vec![BackendValueKind::I32];
                    params.extend(env.iter().copied());
                    params
                }
                BackendCallableArgExpansion::DynRow {
                    fields,
                    needs_payload,
                } => {
                    let mut params = Vec::new();
                    if *needs_payload {
                        params.push(BackendValueKind::I32);
                    }
                    params.extend(fields.iter().map(|field| field.kind));
                    params
                }
                BackendCallableArgExpansion::StaticRowFields(fields) => {
                    fields.iter().map(|field| field.kind).collect()
                }
            })
            .collect();
        let result_type = function
            .return_type
            .as_deref()
            .map(source_type_name_to_type);
        let result = result_type.as_ref().and_then(|ty| {
            backend_value_kind_for_type(ty).or_else(|| backend_storage_value_kind_for_type(ty))
        });
        let result_ref_cell_lane = result_type.as_ref().and_then(core_ref_cell_lane_for_type);
        let abi = BackendCallableAbi {
            params,
            arg_expansions,
            result,
            result_ref_cell_lane,
            return_callable: returned_callables.get(&function.source_name).cloned(),
            return_dyn_row_methods: result_type
                .as_ref()
                .map(|ty| backend_dyn_row_methods_for_type(ty, functions))
                .unwrap_or_default(),
        };
        if function.receiver.is_some() {
            abis.insert(function.symbol.clone(), abi);
        } else {
            abis.insert(function.source_name.clone(), abi);
        }
    }
    abis
}

fn backend_value_kind_for_type(ty: &Type) -> Option<BackendValueKind> {
    match ty {
        Type::Unknown | Type::I64 | Type::Rune | Type::Bool => None,
        Type::Nominal(_) if backend_nominal_is_externref(ty) => Some(BackendValueKind::ExternRef),
        Type::Tuple(_)
        | Type::Record(_)
        | Type::DynRow(_)
        | Type::Adt { .. }
        | Type::Nominal(_)
        | Type::Func(..)
        | Type::Continuation { .. } => None,
    }
}

fn backend_storage_value_kind_for_type(ty: &Type) -> Option<BackendValueKind> {
    match ty {
        Type::I64 | Type::Rune | Type::Bool => Some(BackendValueKind::I32),
        Type::Nominal(_) if backend_nominal_is_externref(ty) => Some(BackendValueKind::ExternRef),
        _ => None,
    }
}

fn core_ref_cell_lane_for_type(ty: &Type) -> Option<crate::core::CoreRefCellLane> {
    match nominal_base_name_for_type(ty) {
        Some("Ref" | "UnsafeRef") => {
            let inner = ref_cell_type_arg(ty)?;
            if backend_value_kind_for_type(&inner) == Some(BackendValueKind::ExternRef)
                || matches!(
                    inner,
                    Type::Tuple(_)
                        | Type::Record(_)
                        | Type::DynRow(_)
                        | Type::Adt { .. }
                        | Type::Func(..)
                        | Type::Continuation { .. }
                )
            {
                Some(crate::core::CoreRefCellLane::ExternRef)
            } else {
                Some(crate::core::CoreRefCellLane::I32)
            }
        }
        _ => None,
    }
}

fn aggregate_element_lane_for_type(ty: &Type) -> Option<BackendValueKind> {
    match nominal_base_name_for_type(ty) {
        Some("Slice" | "Array" | "Vec") => {
            let element = nominal_type_args_for_type(ty)?.into_iter().next()?;
            Some(
                backend_value_kind_for_type(&element)
                    .filter(|kind| *kind == BackendValueKind::ExternRef)
                    .unwrap_or(BackendValueKind::I32),
            )
        }
        _ => None,
    }
}

fn ref_cell_type_arg(ty: &Type) -> Option<Type> {
    nominal_type_args_for_type(ty)?.into_iter().next()
}

fn backend_nominal_is_externref(ty: &Type) -> bool {
    matches!(
        nominal_base_name_for_type(ty),
        Some("Slice" | "Array" | "Vec" | "str" | "String" | "cstr" | "Range" | "Ref" | "UnsafeRef")
    )
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
    for method in visible_receiver_methods(interface, current_namespace) {
        let Some(receiver) = &method.receiver else {
            continue;
        };
        let param_tys = method
            .param_types
            .iter()
            .skip(1)
            .map(|param| {
                param
                    .as_deref()
                    .map(|ty| {
                        resolve_header_type(Some(ty), &method.receiver, &method.owner, type_aliases)
                    })
                    .map(|ty| header_type_to_type(&ty))
                    .unwrap_or(Type::Unknown)
            })
            .collect::<Vec<_>>();
        let result_ty = method
            .return_type
            .as_deref()
            .map(|ty| resolve_header_type(Some(ty), &method.receiver, &method.owner, type_aliases))
            .map(|ty| header_type_to_type(&ty))
            .unwrap_or(Type::Unknown);
        context.insert_receiver_method(ReceiverMethodSummary {
            receiver: receiver.display_name(),
            name: method.source_name.clone(),
            symbol: method.symbol.clone(),
            runtime_target: method.source_name.clone(),
            param_tys,
            result_ty,
        });
    }
    context
}

fn visible_receiver_methods<'a>(
    interface: &'a InterfaceSummary,
    current_namespace: &str,
) -> Vec<&'a crate::surface::InterfaceFunction> {
    let mut by_receiver_name =
        BTreeMap::<(String, String), Vec<&crate::surface::InterfaceFunction>>::new();
    for function in &interface.functions {
        let Some(receiver) = &function.receiver else {
            continue;
        };
        if function.visibility == Visibility::Private && function.owner != current_namespace {
            continue;
        }
        by_receiver_name
            .entry((receiver.display_name(), function.source_name.clone()))
            .or_default()
            .push(function);
    }
    by_receiver_name
        .into_values()
        .flat_map(|candidates| {
            let local = candidates
                .iter()
                .copied()
                .filter(|candidate| candidate.owner == current_namespace)
                .collect::<Vec<_>>();
            if local.is_empty() {
                candidates
            } else {
                local
            }
        })
        .collect()
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
    if matches!(&surface.namespace_path, Some(path) if path.is_empty()) {
        diagnostics.push(ProgramDiagnostic::InvalidSourceFileHeader {
            reason: "empty namespace path".to_string(),
        });
    }
    diagnostics.extend(
        surface
            .duplicate_namespaces
            .iter()
            .cloned()
            .map(|namespace| ProgramDiagnostic::DuplicateNamespace { namespace }),
    );
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

fn receiver_method_ambiguity_diagnostics(
    interface: &InterfaceSummary,
    program: &SourceProgram,
) -> Vec<ProgramDiagnostic> {
    let current_namespace = program
        .namespace
        .as_ref()
        .map(NamespaceDecl::dotted)
        .unwrap_or_else(|| "root".to_string());
    let mut by_receiver_name =
        BTreeMap::<(String, String), Vec<&crate::surface::InterfaceFunction>>::new();
    for function in &interface.functions {
        let Some(receiver) = &function.receiver else {
            continue;
        };
        if function.visibility == Visibility::Private && function.owner != current_namespace {
            continue;
        }
        by_receiver_name
            .entry((receiver.display_name(), function.source_name.clone()))
            .or_default()
            .push(function);
    }
    by_receiver_name
        .into_iter()
        .filter_map(|((receiver, name), candidates)| {
            let local = candidates
                .iter()
                .copied()
                .filter(|candidate| candidate.owner == current_namespace)
                .collect::<Vec<_>>();
            let visible = if local.is_empty() { candidates } else { local };
            (visible.len() > 1).then(|| ProgramDiagnostic::AmbiguousReceiverMethod {
                receiver,
                name,
                candidates: visible
                    .iter()
                    .map(|candidate| candidate.symbol.clone())
                    .collect(),
            })
        })
        .collect()
}

fn ambiguous_operator_resolution_diagnostics(
    diagnostics: &[ProgramDiagnostic],
) -> Vec<ProgramDiagnostic> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| {
            let ProgramDiagnostic::AmbiguousReceiverMethod {
                receiver,
                name,
                candidates,
            } = diagnostic
            else {
                return None;
            };
            operator_protocol_source(name).map(|op| {
                ProgramDiagnostic::AmbiguousOperatorResolution {
                    receiver: receiver.clone(),
                    op: op.to_string(),
                    candidates: candidates.clone(),
                }
            })
        })
        .collect()
}

fn operator_protocol_source(protocol: &str) -> Option<&'static str> {
    match protocol {
        "op_add" => Some("+"),
        "op_sub" => Some("-"),
        "op_mul" => Some("*"),
        "op_div" => Some("/"),
        "op_index" => Some("[]"),
        "op_index_slice" => Some("[..]"),
        _ => None,
    }
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
    let explicit = defs
        .iter()
        .filter(|def| def.entry)
        .map(|def| def.name.clone())
        .collect::<Vec<_>>();
    if let [name] = explicit.as_slice() {
        Some(name.clone())
    } else if !explicit.is_empty() {
        None
    } else if defs.iter().any(|def| def.name == "main") {
        Some("main".to_string())
    } else {
        defs.first().map(|def| def.name.clone())
    }
}

fn entry_diagnostics(
    defs: &[ProgramDefOutput],
    surface: &ProjectSurface,
) -> Vec<ProgramDiagnostic> {
    let mut diagnostics = Vec::new();
    let explicit = defs
        .iter()
        .filter(|def| def.entry)
        .map(|def| def.name.clone())
        .collect::<Vec<_>>();
    if explicit.len() > 1 {
        diagnostics.push(ProgramDiagnostic::MultipleEntries { names: explicit });
    }
    diagnostics.extend(
        surface
            .statics
            .iter()
            .filter(|static_value| static_value.entry)
            .map(|static_value| ProgramDiagnostic::EntryOnNonFunction {
                name: static_value.name.clone(),
            }),
    );
    diagnostics
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

fn explicit_instantiation_diagnostics(defs: &[ProgramDefOutput]) -> Vec<ProgramDiagnostic> {
    let mut template_arities = BTreeMap::new();
    for def in defs {
        if def.receiver.is_none() {
            template_arities.insert(def.name.clone(), def.output.template.explicit_params.len());
        }
    }

    let mut diagnostics = Vec::new();
    for def in defs {
        for instantiation in &def.output.template.explicit_instantiations {
            let Some(expected) = template_arities.get(&instantiation.callee) else {
                continue;
            };
            let actual = instantiation.type_args.len();
            if *expected == 0 {
                diagnostics.push(ProgramDiagnostic::ExplicitInstantiationOfNonTemplate {
                    def: def.name.clone(),
                    callee: instantiation.callee.clone(),
                });
            } else if *expected != actual {
                diagnostics.push(ProgramDiagnostic::ExplicitInstantiationArityMismatch {
                    def: def.name.clone(),
                    callee: instantiation.callee.clone(),
                    expected: *expected,
                    actual,
                });
            }
        }
    }
    diagnostics
}

fn definition_return_type_diagnostics(defs: &[ProgramDefOutput]) -> Vec<ProgramDiagnostic> {
    defs.iter()
        .filter_map(|def| {
            let expected = def.output.typed_signature.return_type.as_deref()?;
            let expected_ty = source_type_name_to_type(expected);
            let actual_ty = &def.output.typed.ty;
            if definition_return_types_match(&expected_ty, actual_ty) {
                return None;
            }
            Some(ProgramDiagnostic::DefinitionReturnTypeMismatch {
                def: def.name.clone(),
                expected: source_type_name_for_type(&expected_ty),
                actual: source_type_name_for_type(actual_ty),
            })
        })
        .collect()
}

fn definition_return_types_match(expected: &Type, actual: &Type) -> bool {
    matches!(expected, Type::Unknown)
        || matches!(actual, Type::Unknown)
        || expected == actual
        || matches!(expected, Type::DynRow(_))
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

fn send_callable_diagnostics(defs: &[ProgramDefOutput]) -> Vec<ProgramDiagnostic> {
    let signatures = send_callable_signatures(defs);
    if signatures.is_empty() {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    for def in defs {
        collect_send_callable_diagnostics(
            &def.name,
            &def.output.typed,
            &signatures,
            &mut diagnostics,
        );
    }
    diagnostics
}

fn send_callable_signatures(defs: &[ProgramDefOutput]) -> BTreeMap<String, Vec<bool>> {
    defs.iter()
        .filter_map(|def| {
            let send_params = def
                .output
                .typed_signature
                .params
                .iter()
                .map(|param| {
                    matches!(
                        source_type_name_to_type(&param.ty),
                        Type::Func(_, _, crate::typed::SendColor::Send)
                    )
                })
                .collect::<Vec<_>>();
            send_params
                .iter()
                .any(|send| *send)
                .then_some((def.name.clone(), send_params))
        })
        .collect()
}

fn collect_send_callable_diagnostics(
    def: &str,
    expr: &TypedExpr,
    signatures: &BTreeMap<String, Vec<bool>>,
    diagnostics: &mut Vec<ProgramDiagnostic>,
) {
    match &expr.kind {
        crate::typed::TypedExprKind::Call { callee, args } => {
            if let crate::typed::TypedExprKind::Var(callee_name) = &callee.kind {
                if let Some(send_params) = signatures.get(callee_name) {
                    for (index, arg) in args.iter().enumerate() {
                        if send_params.get(index).copied().unwrap_or(false)
                            && !callable_arg_is_sendable(arg)
                        {
                            diagnostics.push(
                                ProgramDiagnostic::NonSendCallablePassedToSendCallable {
                                    def: def.to_string(),
                                    callee: callee_name.clone(),
                                    arg: send_callable_arg_name(arg),
                                },
                            );
                        }
                    }
                }
            }
            collect_send_callable_diagnostics(def, callee, signatures, diagnostics);
            for arg in args {
                collect_send_callable_diagnostics(def, arg, signatures, diagnostics);
            }
        }
        crate::typed::TypedExprKind::Lambda { body, .. }
        | crate::typed::TypedExprKind::Nominal { expr: body, .. }
        | crate::typed::TypedExprKind::Reset { body, .. }
        | crate::typed::TypedExprKind::Shift { body, .. } => {
            collect_send_callable_diagnostics(def, body, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                collect_send_callable_diagnostics(def, field, signatures, diagnostics);
            }
        }
        crate::typed::TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                collect_send_callable_diagnostics(def, item, signatures, diagnostics);
            }
        }
        crate::typed::TypedExprKind::Record { fields } => {
            for field in fields {
                collect_send_callable_diagnostics(def, &field.value, signatures, diagnostics);
            }
        }
        crate::typed::TypedExprKind::RecordUpdate { base, fields } => {
            collect_send_callable_diagnostics(def, base, signatures, diagnostics);
            for field in fields {
                collect_send_callable_diagnostics(def, &field.value, signatures, diagnostics);
            }
        }
        crate::typed::TypedExprKind::DynRowPackage { payload, .. } => {
            collect_send_callable_diagnostics(def, payload, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::DynRowField { package, .. } => {
            collect_send_callable_diagnostics(def, package, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_send_callable_diagnostics(def, arg, signatures, diagnostics);
            }
        }
        crate::typed::TypedExprKind::AdtToTuple { value, .. } => {
            collect_send_callable_diagnostics(def, value, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::TupleToAdt { value, .. } => {
            collect_send_callable_diagnostics(def, value, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::Field { receiver, .. } => {
            collect_send_callable_diagnostics(def, receiver, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_send_callable_diagnostics(def, receiver, signatures, diagnostics);
            for arg in args {
                collect_send_callable_diagnostics(def, arg, signatures, diagnostics);
            }
        }
        crate::typed::TypedExprKind::Assign { target, value, .. } => {
            collect_send_callable_diagnostics(def, target, signatures, diagnostics);
            collect_send_callable_diagnostics(def, value, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::Index {
            receiver, index, ..
        } => {
            collect_send_callable_diagnostics(def, receiver, signatures, diagnostics);
            collect_send_callable_diagnostics(def, index, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::Range { start, end } => {
            collect_send_callable_diagnostics(def, start, signatures, diagnostics);
            collect_send_callable_diagnostics(def, end, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::Binary { lhs, rhs, .. } => {
            collect_send_callable_diagnostics(def, lhs, signatures, diagnostics);
            collect_send_callable_diagnostics(def, rhs, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_send_callable_diagnostics(def, cond, signatures, diagnostics);
            collect_send_callable_diagnostics(def, then_branch, signatures, diagnostics);
            collect_send_callable_diagnostics(def, else_branch, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_send_callable_diagnostics(def, scrutinee, signatures, diagnostics);
            collect_send_callable_diagnostics(def, then_branch, signatures, diagnostics);
            collect_send_callable_diagnostics(def, else_branch, signatures, diagnostics);
        }
        crate::typed::TypedExprKind::Match { scrutinee, arms } => {
            collect_send_callable_diagnostics(def, scrutinee, signatures, diagnostics);
            for arm in arms {
                collect_send_callable_diagnostics(def, &arm.body, signatures, diagnostics);
            }
        }
        crate::typed::TypedExprKind::Var(_) | crate::typed::TypedExprKind::Lit(_) => {}
    }
}

fn callable_arg_is_sendable(arg: &TypedExpr) -> bool {
    match &arg.kind {
        crate::typed::TypedExprKind::Lambda { body, .. } => !typed_expr_has_capture(body),
        crate::typed::TypedExprKind::Shift { .. } => false,
        _ => !matches!(arg.ty, Type::Continuation { .. }),
    }
}

fn typed_expr_has_capture(expr: &TypedExpr) -> bool {
    let mut free = BTreeSet::new();
    collect_typed_free_vars(expr, &mut BTreeSet::new(), &mut free);
    !free.is_empty()
}

fn collect_typed_free_vars(
    expr: &TypedExpr,
    locals: &mut BTreeSet<String>,
    free: &mut BTreeSet<String>,
) {
    match &expr.kind {
        crate::typed::TypedExprKind::Var(name) => {
            if !locals.contains(name) {
                free.insert(name.clone());
            }
        }
        crate::typed::TypedExprKind::Lambda { param, body, .. } => {
            locals.insert(param.clone());
            collect_typed_free_vars(body, locals, free);
            locals.remove(param);
        }
        crate::typed::TypedExprKind::Call { callee, args } => {
            collect_typed_free_vars(callee, locals, free);
            for arg in args {
                collect_typed_free_vars(arg, locals, free);
            }
        }
        crate::typed::TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                collect_typed_free_vars(field, locals, free);
            }
        }
        crate::typed::TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                collect_typed_free_vars(item, locals, free);
            }
        }
        crate::typed::TypedExprKind::Record { fields } => {
            for field in fields {
                collect_typed_free_vars(&field.value, locals, free);
            }
        }
        crate::typed::TypedExprKind::RecordUpdate { base, fields } => {
            collect_typed_free_vars(base, locals, free);
            for field in fields {
                collect_typed_free_vars(&field.value, locals, free);
            }
        }
        crate::typed::TypedExprKind::DynRowPackage { payload, .. } => {
            collect_typed_free_vars(payload, locals, free);
        }
        crate::typed::TypedExprKind::DynRowField { package, .. } => {
            collect_typed_free_vars(package, locals, free);
        }
        crate::typed::TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_typed_free_vars(arg, locals, free);
            }
        }
        crate::typed::TypedExprKind::AdtToTuple { value, .. } => {
            collect_typed_free_vars(value, locals, free);
        }
        crate::typed::TypedExprKind::TupleToAdt { value, .. } => {
            collect_typed_free_vars(value, locals, free);
        }
        crate::typed::TypedExprKind::Field { receiver, .. } => {
            collect_typed_free_vars(receiver, locals, free);
        }
        crate::typed::TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_typed_free_vars(receiver, locals, free);
            for arg in args {
                collect_typed_free_vars(arg, locals, free);
            }
        }
        crate::typed::TypedExprKind::Assign { target, value, .. } => {
            collect_typed_free_vars(target, locals, free);
            collect_typed_free_vars(value, locals, free);
        }
        crate::typed::TypedExprKind::Index {
            receiver, index, ..
        } => {
            collect_typed_free_vars(receiver, locals, free);
            collect_typed_free_vars(index, locals, free);
        }
        crate::typed::TypedExprKind::Range { start, end } => {
            collect_typed_free_vars(start, locals, free);
            collect_typed_free_vars(end, locals, free);
        }
        crate::typed::TypedExprKind::Binary { lhs, rhs, .. } => {
            collect_typed_free_vars(lhs, locals, free);
            collect_typed_free_vars(rhs, locals, free);
        }
        crate::typed::TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_typed_free_vars(cond, locals, free);
            collect_typed_free_vars(then_branch, locals, free);
            collect_typed_free_vars(else_branch, locals, free);
        }
        crate::typed::TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_typed_free_vars(scrutinee, locals, free);
            collect_typed_free_vars(then_branch, locals, free);
            collect_typed_free_vars(else_branch, locals, free);
        }
        crate::typed::TypedExprKind::Match { scrutinee, arms } => {
            collect_typed_free_vars(scrutinee, locals, free);
            for arm in arms {
                collect_typed_free_vars(&arm.body, locals, free);
            }
        }
        crate::typed::TypedExprKind::Nominal { expr, .. }
        | crate::typed::TypedExprKind::Reset { body: expr, .. }
        | crate::typed::TypedExprKind::Shift { body: expr, .. } => {
            collect_typed_free_vars(expr, locals, free);
        }
        crate::typed::TypedExprKind::Lit(_) => {}
    }
}

fn send_callable_arg_name(arg: &TypedExpr) -> String {
    match &arg.kind {
        crate::typed::TypedExprKind::Var(name) => name.clone(),
        crate::typed::TypedExprKind::Lambda { .. } => "lambda".to_string(),
        crate::typed::TypedExprKind::Shift { binder, .. } => binder.clone(),
        _ => program_type_name(&arg.ty),
    }
}

fn assignment_diagnostics(defs: &[ProgramDefOutput]) -> Vec<ProgramDiagnostic> {
    let mut diagnostics = Vec::new();
    for def in defs {
        collect_assignment_diagnostics(&def.name, &def.output.typed, &mut diagnostics);
    }
    diagnostics
}

fn collect_assignment_diagnostics(
    def: &str,
    expr: &TypedExpr,
    diagnostics: &mut Vec<ProgramDiagnostic>,
) {
    match &expr.kind {
        crate::typed::TypedExprKind::Assign {
            target,
            value,
            builtin,
        } => {
            if !matches!(
                builtin,
                Some(crate::typed::BuiltinMethodCall::RefSet)
                    | Some(crate::typed::BuiltinMethodCall::UnsafeRefSet)
            ) {
                diagnostics.push(ProgramDiagnostic::InvalidAssignmentTarget {
                    def: def.to_string(),
                    target_type: program_type_name(&target.ty),
                });
            }
            collect_assignment_diagnostics(def, target, diagnostics);
            collect_assignment_diagnostics(def, value, diagnostics);
        }
        crate::typed::TypedExprKind::Lambda { body, .. }
        | crate::typed::TypedExprKind::Nominal { expr: body, .. }
        | crate::typed::TypedExprKind::Reset { body, .. }
        | crate::typed::TypedExprKind::Shift { body, .. } => {
            collect_assignment_diagnostics(def, body, diagnostics);
        }
        crate::typed::TypedExprKind::Call { callee, args } => {
            collect_assignment_diagnostics(def, callee, diagnostics);
            for arg in args {
                collect_assignment_diagnostics(def, arg, diagnostics);
            }
        }
        crate::typed::TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                collect_assignment_diagnostics(def, field, diagnostics);
            }
        }
        crate::typed::TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                collect_assignment_diagnostics(def, item, diagnostics);
            }
        }
        crate::typed::TypedExprKind::Record { fields } => {
            for field in fields {
                collect_assignment_diagnostics(def, &field.value, diagnostics);
            }
        }
        crate::typed::TypedExprKind::RecordUpdate { base, fields } => {
            collect_assignment_diagnostics(def, base, diagnostics);
            for field in fields {
                collect_assignment_diagnostics(def, &field.value, diagnostics);
            }
        }
        crate::typed::TypedExprKind::DynRowPackage { payload, .. } => {
            collect_assignment_diagnostics(def, payload, diagnostics);
        }
        crate::typed::TypedExprKind::DynRowField { package, .. } => {
            collect_assignment_diagnostics(def, package, diagnostics);
        }
        crate::typed::TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_assignment_diagnostics(def, arg, diagnostics);
            }
        }
        crate::typed::TypedExprKind::AdtToTuple { value, .. } => {
            collect_assignment_diagnostics(def, value, diagnostics);
        }
        crate::typed::TypedExprKind::TupleToAdt { value, .. } => {
            collect_assignment_diagnostics(def, value, diagnostics);
        }
        crate::typed::TypedExprKind::Field { receiver, .. } => {
            collect_assignment_diagnostics(def, receiver, diagnostics);
        }
        crate::typed::TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_assignment_diagnostics(def, receiver, diagnostics);
            for arg in args {
                collect_assignment_diagnostics(def, arg, diagnostics);
            }
        }
        crate::typed::TypedExprKind::Index {
            receiver, index, ..
        } => {
            collect_assignment_diagnostics(def, receiver, diagnostics);
            collect_assignment_diagnostics(def, index, diagnostics);
        }
        crate::typed::TypedExprKind::Range { start, end } => {
            collect_assignment_diagnostics(def, start, diagnostics);
            collect_assignment_diagnostics(def, end, diagnostics);
        }
        crate::typed::TypedExprKind::Binary { lhs, rhs, .. } => {
            collect_assignment_diagnostics(def, lhs, diagnostics);
            collect_assignment_diagnostics(def, rhs, diagnostics);
        }
        crate::typed::TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_assignment_diagnostics(def, cond, diagnostics);
            collect_assignment_diagnostics(def, then_branch, diagnostics);
            collect_assignment_diagnostics(def, else_branch, diagnostics);
        }
        crate::typed::TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_assignment_diagnostics(def, scrutinee, diagnostics);
            collect_assignment_diagnostics(def, then_branch, diagnostics);
            collect_assignment_diagnostics(def, else_branch, diagnostics);
        }
        crate::typed::TypedExprKind::Match { scrutinee, arms } => {
            collect_assignment_diagnostics(def, scrutinee, diagnostics);
            for arm in arms {
                collect_assignment_diagnostics(def, &arm.body, diagnostics);
            }
        }
        crate::typed::TypedExprKind::Var(_) | crate::typed::TypedExprKind::Lit(_) => {}
    }
}

fn dyn_row_coercion_diagnostics(defs: &[ProgramDefOutput]) -> Vec<ProgramDiagnostic> {
    let signatures = defs
        .iter()
        .map(|def| (def.name.clone(), def.output.typed_signature.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Vec::new();
    for def in defs {
        if let Some(expected) = def
            .output
            .typed_signature
            .return_type
            .as_deref()
            .map(source_type_name_to_type)
        {
            collect_dyn_row_coercion_diagnostics(
                &def.name,
                &def.output.typed,
                &expected,
                &mut diagnostics,
            );
        }
        collect_call_dyn_row_coercion_diagnostics(
            &def.name,
            &def.output.typed,
            &signatures,
            &mut diagnostics,
        );
    }
    diagnostics
}

fn collect_dyn_row_coercion_diagnostics(
    def: &str,
    expr: &TypedExpr,
    expected: &Type,
    diagnostics: &mut Vec<ProgramDiagnostic>,
) {
    if matches!(expected, Type::DynRow(_))
        && !matches!(expr.kind, TypedExprKind::DynRowPackage { .. })
        && expr.ty != *expected
    {
        diagnostics.push(ProgramDiagnostic::DynRowCoercionFailed {
            def: def.to_string(),
            expected: program_type_name(expected),
            actual: program_type_name(&expr.ty),
        });
    }
}

fn collect_call_dyn_row_coercion_diagnostics(
    def: &str,
    expr: &TypedExpr,
    signatures: &BTreeMap<String, TypedSignature>,
    diagnostics: &mut Vec<ProgramDiagnostic>,
) {
    match &expr.kind {
        TypedExprKind::Call { callee, args } => {
            if let TypedExprKind::Var(callee_name) = &callee.kind {
                if let Some(signature) = signatures.get(callee_name) {
                    for (arg, param) in args.iter().zip(&signature.params) {
                        let expected = source_type_name_to_type(&param.ty);
                        collect_dyn_row_coercion_diagnostics(def, arg, &expected, diagnostics);
                    }
                }
            }
            collect_call_dyn_row_coercion_diagnostics(def, callee, signatures, diagnostics);
            for arg in args {
                collect_call_dyn_row_coercion_diagnostics(def, arg, signatures, diagnostics);
            }
        }
        TypedExprKind::Lambda { body, .. }
        | TypedExprKind::Nominal { expr: body, .. }
        | TypedExprKind::Reset { body, .. }
        | TypedExprKind::Shift { body, .. } => {
            collect_call_dyn_row_coercion_diagnostics(def, body, signatures, diagnostics);
        }
        TypedExprKind::Tuple { fields, .. } | TypedExprKind::SliceLiteral { items: fields, .. } => {
            for field in fields {
                collect_call_dyn_row_coercion_diagnostics(def, field, signatures, diagnostics);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                collect_call_dyn_row_coercion_diagnostics(
                    def,
                    &field.value,
                    signatures,
                    diagnostics,
                );
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            collect_call_dyn_row_coercion_diagnostics(def, base, signatures, diagnostics);
            for field in fields {
                collect_call_dyn_row_coercion_diagnostics(
                    def,
                    &field.value,
                    signatures,
                    diagnostics,
                );
            }
        }
        TypedExprKind::DynRowPackage { payload, .. } => {
            collect_call_dyn_row_coercion_diagnostics(def, payload, signatures, diagnostics);
        }
        TypedExprKind::DynRowField { package, .. } => {
            collect_call_dyn_row_coercion_diagnostics(def, package, signatures, diagnostics);
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_call_dyn_row_coercion_diagnostics(def, arg, signatures, diagnostics);
            }
        }
        TypedExprKind::AdtToTuple { value, .. } | TypedExprKind::TupleToAdt { value, .. } => {
            collect_call_dyn_row_coercion_diagnostics(def, value, signatures, diagnostics);
        }
        TypedExprKind::Field { receiver, .. } => {
            collect_call_dyn_row_coercion_diagnostics(def, receiver, signatures, diagnostics);
        }
        TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_call_dyn_row_coercion_diagnostics(def, receiver, signatures, diagnostics);
            for arg in args {
                collect_call_dyn_row_coercion_diagnostics(def, arg, signatures, diagnostics);
            }
        }
        TypedExprKind::Assign { target, value, .. } => {
            collect_call_dyn_row_coercion_diagnostics(def, target, signatures, diagnostics);
            collect_call_dyn_row_coercion_diagnostics(def, value, signatures, diagnostics);
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            collect_call_dyn_row_coercion_diagnostics(def, receiver, signatures, diagnostics);
            collect_call_dyn_row_coercion_diagnostics(def, index, signatures, diagnostics);
        }
        TypedExprKind::Range { start, end }
        | TypedExprKind::Binary {
            lhs: start,
            rhs: end,
            ..
        } => {
            collect_call_dyn_row_coercion_diagnostics(def, start, signatures, diagnostics);
            collect_call_dyn_row_coercion_diagnostics(def, end, signatures, diagnostics);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_call_dyn_row_coercion_diagnostics(def, cond, signatures, diagnostics);
            collect_call_dyn_row_coercion_diagnostics(def, then_branch, signatures, diagnostics);
            collect_call_dyn_row_coercion_diagnostics(def, else_branch, signatures, diagnostics);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_call_dyn_row_coercion_diagnostics(def, scrutinee, signatures, diagnostics);
            collect_call_dyn_row_coercion_diagnostics(def, then_branch, signatures, diagnostics);
            collect_call_dyn_row_coercion_diagnostics(def, else_branch, signatures, diagnostics);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_call_dyn_row_coercion_diagnostics(def, scrutinee, signatures, diagnostics);
            for arm in arms {
                collect_call_dyn_row_coercion_diagnostics(def, &arm.body, signatures, diagnostics);
            }
        }
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
    }
}

fn operator_operand_diagnostics(defs: &[ProgramDefOutput]) -> Vec<ProgramDiagnostic> {
    let mut diagnostics = Vec::new();
    for def in defs {
        collect_operator_operand_diagnostics(&def.name, &def.output.typed, &mut diagnostics);
    }
    diagnostics
}

fn collect_operator_operand_diagnostics(
    def: &str,
    expr: &TypedExpr,
    diagnostics: &mut Vec<ProgramDiagnostic>,
) {
    match &expr.kind {
        TypedExprKind::Binary {
            op,
            lhs,
            rhs,
            receiver_method,
        } => {
            if receiver_method.is_none()
                && (binary_operand_is_concrete_non_i64(&lhs.ty)
                    || binary_operand_is_concrete_non_i64(&rhs.ty))
            {
                diagnostics.push(ProgramDiagnostic::InvalidOperatorOperands {
                    def: def.to_string(),
                    op: render_source_binary_op(*op).to_string(),
                    lhs: program_type_name(&lhs.ty),
                    rhs: program_type_name(&rhs.ty),
                });
            }
            collect_operator_operand_diagnostics(def, lhs, diagnostics);
            collect_operator_operand_diagnostics(def, rhs, diagnostics);
        }
        TypedExprKind::Lambda { body, .. }
        | TypedExprKind::Nominal { expr: body, .. }
        | TypedExprKind::Reset { body, .. }
        | TypedExprKind::Shift { body, .. } => {
            collect_operator_operand_diagnostics(def, body, diagnostics);
        }
        TypedExprKind::Call { callee, args } => {
            collect_operator_operand_diagnostics(def, callee, diagnostics);
            for arg in args {
                collect_operator_operand_diagnostics(def, arg, diagnostics);
            }
        }
        TypedExprKind::Tuple { fields, .. } | TypedExprKind::SliceLiteral { items: fields, .. } => {
            for field in fields {
                collect_operator_operand_diagnostics(def, field, diagnostics);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                collect_operator_operand_diagnostics(def, &field.value, diagnostics);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            collect_operator_operand_diagnostics(def, base, diagnostics);
            for field in fields {
                collect_operator_operand_diagnostics(def, &field.value, diagnostics);
            }
        }
        TypedExprKind::DynRowPackage { payload, .. } => {
            collect_operator_operand_diagnostics(def, payload, diagnostics);
        }
        TypedExprKind::DynRowField { package, .. } => {
            collect_operator_operand_diagnostics(def, package, diagnostics);
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                collect_operator_operand_diagnostics(def, arg, diagnostics);
            }
        }
        TypedExprKind::AdtToTuple { value, .. } | TypedExprKind::TupleToAdt { value, .. } => {
            collect_operator_operand_diagnostics(def, value, diagnostics);
        }
        TypedExprKind::Field { receiver, .. } => {
            collect_operator_operand_diagnostics(def, receiver, diagnostics);
        }
        TypedExprKind::MethodCall { receiver, args, .. } => {
            collect_operator_operand_diagnostics(def, receiver, diagnostics);
            for arg in args {
                collect_operator_operand_diagnostics(def, arg, diagnostics);
            }
        }
        TypedExprKind::Assign { target, value, .. } => {
            collect_operator_operand_diagnostics(def, target, diagnostics);
            collect_operator_operand_diagnostics(def, value, diagnostics);
        }
        TypedExprKind::Index {
            receiver,
            index,
            access,
            receiver_method: _,
        } => {
            collect_index_operator_diagnostics(def, receiver, index, access, diagnostics);
            collect_operator_operand_diagnostics(def, receiver, diagnostics);
            collect_operator_operand_diagnostics(def, index, diagnostics);
        }
        TypedExprKind::Range { start, end } => {
            collect_operator_operand_diagnostics(def, start, diagnostics);
            collect_operator_operand_diagnostics(def, end, diagnostics);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_operator_operand_diagnostics(def, cond, diagnostics);
            collect_operator_operand_diagnostics(def, then_branch, diagnostics);
            collect_operator_operand_diagnostics(def, else_branch, diagnostics);
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_operator_operand_diagnostics(def, scrutinee, diagnostics);
            collect_operator_operand_diagnostics(def, then_branch, diagnostics);
            collect_operator_operand_diagnostics(def, else_branch, diagnostics);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            collect_operator_operand_diagnostics(def, scrutinee, diagnostics);
            for arm in arms {
                collect_operator_operand_diagnostics(def, &arm.body, diagnostics);
            }
        }
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
    }
}

fn binary_operand_is_concrete_non_i64(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Bool | Type::Rune | Type::Tuple(_) | Type::Record(_) | Type::DynRow(_)
    )
}

fn collect_index_operator_diagnostics(
    def: &str,
    receiver: &TypedExpr,
    index: &TypedExpr,
    access: &IndexAccessKind,
    diagnostics: &mut Vec<ProgramDiagnostic>,
) {
    if !matches!(access, IndexAccessKind::Operator) {
        if let TypedExprKind::Range { start, end } = &index.kind {
            if start.ty != Type::I64 || end.ty != Type::I64 {
                diagnostics.push(ProgramDiagnostic::InvalidOperatorOperands {
                    def: def.to_string(),
                    op: "[..]".to_string(),
                    lhs: program_type_name(&receiver.ty),
                    rhs: program_type_name(&index.ty),
                });
            }
        }
        return;
    }

    if builtin_indexable_type(&receiver.ty) && invalid_builtin_index_type(index) {
        diagnostics.push(ProgramDiagnostic::InvalidOperatorOperands {
            def: def.to_string(),
            op: index_operator_name(index),
            lhs: program_type_name(&receiver.ty),
            rhs: program_type_name(&index.ty),
        });
    }
}

fn builtin_indexable_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Nominal(name)
            if matches!(
                nominal_base_name_for_type(&Type::Nominal(name.clone())).as_deref(),
                Some("Slice" | "Array" | "Vec" | "str" | "String" | "cstr")
            )
    )
}

fn invalid_builtin_index_type(index: &TypedExpr) -> bool {
    match &index.kind {
        TypedExprKind::Range { start, end } => start.ty != Type::I64 || end.ty != Type::I64,
        _ => index.ty != Type::I64,
    }
}

fn index_operator_name(index: &TypedExpr) -> String {
    if matches!(index.kind, TypedExprKind::Range { .. }) {
        "[..]".to_string()
    } else {
        "[]".to_string()
    }
}

fn program_type_name(ty: &Type) -> String {
    match ty {
        Type::Unknown => "Unknown".to_string(),
        Type::I64 => "i64".to_string(),
        Type::Rune => "rune".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Nominal(name) => name.clone(),
        Type::Adt { name, .. } => name.clone(),
        Type::Tuple(fields) => format!(
            "Tuple[{}]",
            fields
                .iter()
                .map(program_type_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Record(fields) => format!(
            "{{{}}}",
            fields
                .iter()
                .map(|field| format!("{}: {}", field.name, program_type_name(&field.ty)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::DynRow(fields) => format!(
            "dyn {{{}}}",
            fields
                .iter()
                .map(|field| format!("{}: {}", field.name, program_type_name(&field.ty)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Func(arg, ret, send) => {
            let rendered = format!("({}) -> {}", program_type_name(arg), program_type_name(ret));
            if *send == crate::typed::SendColor::Send {
                format!("({rendered}) send")
            } else {
                rendered
            }
        }
        Type::Continuation {
            multi,
            input,
            answer,
        } => {
            let kind = if *multi { "contN" } else { "cont1" };
            format!(
                "{kind} ({}) -> {}",
                program_type_name(input),
                program_type_name(answer)
            )
        }
    }
}

fn apply_program_backend_gates(defs: &mut [ProgramDefOutput], diagnostics: &[ProgramDiagnostic]) {
    for def in defs {
        if diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic,
                ProgramDiagnostic::NonSendCallablePassedToSendCallable { def: name, .. }
                    if name == &def.name
            )
        }) {
            def.output.backend.wat.clear();
            def.output
                .backend
                .diagnostics
                .push(BackendDiagnostic::CoreValidationFailed { diagnostics: 1 });
            def.output.backend_link = link_backend_artifacts(vec![def.output.backend.clone()]);
            def.output.backend_cache_key =
                backend_cache_key(&def.output.backend_link, &BackendCacheConfig::default());
            def.output.visual.backend = crate::debug::render_backend_artifact(&def.output.backend);
            def.output.visual.backend_link =
                crate::debug::render_backend_link(&def.output.backend_link);
        }
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
    interface: &InterfaceSummary,
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
        .filter(|(_, def)| {
            entry == Some(def.name.as_str())
                || !is_uninstantiated_row_callable_template(def)
                || defs.iter().any(|caller| {
                    caller.name != def.name
                        && typed_expr_calls_def(&caller.output.typed, def, interface)
                })
        })
        .map(|(index, def)| {
            let mut artifact = def.output.backend.clone();
            let is_entry = entry == Some(def.name.as_str()) && !entry_exported;
            if is_entry {
                entry_exported = true;
            }
            let symbol = program_backend_def_symbol(
                def,
                interface,
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

fn is_uninstantiated_row_callable_template(def: &ProgramDefOutput) -> bool {
    !def.output.typed_signature.params.is_empty() && is_row_callable_template_core(def)
}

fn is_row_callable_template_core(def: &ProgramDefOutput) -> bool {
    !def.output.template.obligations.is_empty()
        && def.output.template.obligations.iter().any(|obligation| {
            matches!(
                obligation,
                crate::template::TemplateObligation::Field { .. }
            )
        })
        && def
            .output
            .core
            .ops
            .iter()
            .any(|op| matches!(op, CoreOp::DynamicCallableTarget { .. }))
}

fn typed_expr_calls_def(
    expr: &TypedExpr,
    target: &ProgramDefOutput,
    interface: &InterfaceSummary,
) -> bool {
    let target_method_symbol = interface
        .functions
        .iter()
        .find(|function| {
            function.receiver.is_some()
                && function.source_name == target.name
                && function.receiver.as_ref().map(MethodReceiver::display_name) == target.receiver
        })
        .map(|function| function.symbol.as_str());
    match &expr.kind {
        TypedExprKind::Call { callee, args } => {
            matches!(callee.kind, TypedExprKind::Var(ref callee_name) if callee_name == &target.name)
                || typed_expr_calls_def(callee, target, interface)
                || args
                    .iter()
                    .any(|arg| typed_expr_calls_def(arg, target, interface))
        }
        TypedExprKind::Lambda { body, .. }
        | TypedExprKind::Nominal { expr: body, .. }
        | TypedExprKind::Reset { body, .. }
        | TypedExprKind::Shift { body, .. } => typed_expr_calls_def(body, target, interface),
        TypedExprKind::Tuple { fields, .. } | TypedExprKind::SliceLiteral { items: fields, .. } => {
            fields
                .iter()
                .any(|field| typed_expr_calls_def(field, target, interface))
        }
        TypedExprKind::Record { fields } => fields
            .iter()
            .any(|field| typed_expr_calls_def(&field.value, target, interface)),
        TypedExprKind::RecordUpdate { base, fields } => {
            typed_expr_calls_def(base, target, interface)
                || fields
                    .iter()
                    .any(|field| typed_expr_calls_def(&field.value, target, interface))
        }
        TypedExprKind::DynRowPackage { payload, fields } => {
            typed_expr_calls_def(payload, target, interface)
                || fields.iter().any(|field| {
                    matches!(
                        &field.source,
                        crate::typed::DynRowFieldSource::ReceiverMethod { symbol, .. }
                            if Some(symbol.as_str()) == target_method_symbol
                    )
                })
        }
        TypedExprKind::DynRowField { package, .. }
        | TypedExprKind::Field {
            receiver: package, ..
        } => typed_expr_calls_def(package, target, interface),
        TypedExprKind::AdtCtor { args, .. } => args
            .iter()
            .any(|arg| typed_expr_calls_def(arg, target, interface)),
        TypedExprKind::AdtToTuple { value, .. } | TypedExprKind::TupleToAdt { value, .. } => {
            typed_expr_calls_def(value, target, interface)
        }
        TypedExprKind::MethodCall { receiver, args, .. } => {
            typed_expr_calls_def(receiver, target, interface)
                || args
                    .iter()
                    .any(|arg| typed_expr_calls_def(arg, target, interface))
        }
        TypedExprKind::Assign {
            target: assign_target,
            value,
            ..
        } => {
            typed_expr_calls_def(assign_target, target, interface)
                || typed_expr_calls_def(value, target, interface)
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            typed_expr_calls_def(receiver, target, interface)
                || typed_expr_calls_def(index, target, interface)
        }
        TypedExprKind::Range { start, end } => {
            typed_expr_calls_def(start, target, interface)
                || typed_expr_calls_def(end, target, interface)
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            typed_expr_calls_def(lhs, target, interface)
                || typed_expr_calls_def(rhs, target, interface)
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            typed_expr_calls_def(cond, target, interface)
                || typed_expr_calls_def(then_branch, target, interface)
                || typed_expr_calls_def(else_branch, target, interface)
        }
        TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            typed_expr_calls_def(scrutinee, target, interface)
                || typed_expr_calls_def(then_branch, target, interface)
                || typed_expr_calls_def(else_branch, target, interface)
        }
        TypedExprKind::Match { scrutinee, arms } => {
            typed_expr_calls_def(scrutinee, target, interface)
                || arms
                    .iter()
                    .any(|arm| typed_expr_calls_def(&arm.body, target, interface))
        }
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => false,
    }
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

fn program_backend_def_symbol(
    def: &ProgramDefOutput,
    interface: &InterfaceSummary,
    index: usize,
    name_count: usize,
) -> String {
    interface
        .functions
        .iter()
        .find(|function| {
            function.receiver.is_some()
                && function.source_name == def.name
                && function.receiver.as_ref().map(MethodReceiver::display_name) == def.receiver
        })
        .map(|function| sanitize_program_symbol(&function.symbol))
        .unwrap_or_else(|| def_symbol(&def.name, index, name_count))
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
    let (_, body_without_imports) = split_module_imports(body);

    let mut wat = String::from("(module\n");
    let mut imports = bundle.manifest.imports.clone();
    imports.extend(global_init_runtime_imports(global_init));
    sort_dedup_imports(&mut imports);
    bundle.manifest.imports = imports.clone();
    for import in &imports {
        render_program_extern_import_wat(&mut wat, import);
    }
    for static_value in &global_init.statics {
        match global_static_value_kind(static_value) {
            BackendValueKind::ExternRef => {
                wat.push_str(&format!(
                    "  (global ${} (mut externref) (ref.null extern))\n",
                    global_symbol(&static_value.owner, &static_value.name)
                ));
            }
            BackendValueKind::I32 => {
                let initializer = global_storage_initializer(&static_value.body);
                wat.push_str(&format!(
                    "  (global ${} (mut i32) (i32.const {}))\n",
                    global_symbol(&static_value.owner, &static_value.name),
                    initializer
                ));
            }
        }
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
        if global_static_value_kind(static_value) == BackendValueKind::I32
            && const_global_initializer(&static_value.body).is_some()
        {
            continue;
        }
        wat.push_str(&format!("    ;; init static {}\n", static_value.name));
        let init_result = match global_static_value_kind(static_value) {
            BackendValueKind::ExternRef => render_global_init_expr_externref(
                &mut wat,
                &static_value.name,
                &static_value.body,
                global_init,
            ),
            BackendValueKind::I32 => render_global_init_expr(
                &mut wat,
                &static_value.name,
                &static_value.body,
                global_init,
            ),
        };
        if let Err(diagnostic) = init_result {
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
    wat.push_str(&body_without_imports);
    wat.push_str(")\n");
    bundle.linked_wat = wat;
    bundle
}

fn split_module_imports(body: &str) -> (String, String) {
    let mut imports = String::new();
    let mut rest = String::new();
    for line in body.lines() {
        if line.trim_start().starts_with("(import ") {
            imports.push_str(line);
            imports.push('\n');
        } else {
            rest.push_str(line);
            rest.push('\n');
        }
    }
    (imports, rest)
}

fn global_static_value_kind(static_value: &GlobalStatic) -> BackendValueKind {
    static_value
        .ty
        .as_deref()
        .map(source_type_name_to_type)
        .and_then(|ty| backend_value_kind_for_type(&ty))
        .unwrap_or(BackendValueKind::I32)
}

fn global_init_runtime_imports(global_init: &GlobalInitPlan) -> Vec<BackendExternImport> {
    let mut imports = Vec::new();
    for static_value in &global_init.statics {
        collect_global_init_runtime_imports(&static_value.body, &mut imports);
    }
    sort_dedup_imports(&mut imports);
    imports
}

fn collect_global_init_runtime_imports(expr: &Expr, imports: &mut Vec<BackendExternImport>) {
    match expr {
        Expr::MethodCall {
            receiver,
            name,
            args,
        } if global_string_from_literal(receiver, name, args).is_some() => {
            if let Some(text) = global_string_from_literal(receiver, name, args) {
                imports.push(global_text_literal_import(text));
                imports.push(global_builtin_import(
                    "std_str_to_string",
                    "std.str_to_string",
                    "externref_to_externref",
                ));
            }
        }
        Expr::MethodCall {
            receiver,
            name,
            args,
        } => {
            collect_global_init_runtime_imports(receiver, imports);
            for arg in args {
                collect_global_init_runtime_imports(arg, imports);
            }
            match (receiver.as_ref(), name.as_str(), args.as_slice()) {
                (Expr::Var(type_name), "new", []) if type_name == "String" => {
                    imports.push(global_builtin_import(
                        "std_string_new",
                        "std.string_new",
                        "_to_externref",
                    ));
                }
                (_, "concat", [_]) => {
                    imports.push(global_builtin_import(
                        "std_string_concat",
                        "std.string_concat",
                        "externref_externref_to_externref",
                    ));
                }
                (_, "as_str", []) => {
                    imports.push(global_builtin_import(
                        "std_string_as_str",
                        "std.string_as_str",
                        "externref_to_externref",
                    ));
                }
                (_, "to_cstr", []) => {
                    imports.push(global_builtin_import(
                        "std_string_to_cstr",
                        "std.string_to_cstr",
                        "externref_to_externref",
                    ));
                }
                (_, "push_rune", [_]) => {
                    imports.push(global_builtin_import(
                        "std_string_push_rune",
                        "std.string_push_rune",
                        "externref_i64_to_externref",
                    ));
                }
                (Expr::Var(type_name), "new", []) if type_name == "Vec" => {
                    imports.push(global_builtin_import(
                        "std_vec_new",
                        "std.vec_new",
                        "_to_externref",
                    ));
                }
                (_, "push", [item]) => {
                    imports.push(global_vec_push_import(item));
                }
                (_, "freeze", []) => {
                    imports.push(global_builtin_import(
                        "std_vec_freeze",
                        "std.vec_freeze",
                        "externref_to_externref",
                    ));
                }
                (Expr::Var(type_name), "new", [value])
                    if type_name == "Ref" || type_name == "UnsafeRef" =>
                {
                    imports.push(global_ref_new_import(type_name, value));
                }
                _ => {}
            }
        }
        Expr::Lit(crate::ast::Literal::String(text)) => {
            imports.push(global_text_literal_import(text));
        }
        Expr::Lit(crate::ast::Literal::CStr(text)) => {
            imports.push(global_cstr_literal_import(text));
        }
        Expr::Range { start, end } => {
            collect_global_init_runtime_imports(start, imports);
            collect_global_init_runtime_imports(end, imports);
            imports.push(global_builtin_import(
                "std_range_i64_new",
                "std.range_i64_new",
                "i64_i64_to_externref",
            ));
        }
        Expr::SliceLiteral(items) => {
            for item in items {
                collect_global_init_runtime_imports(item, imports);
            }
            imports.push(global_slice_literal_import(items));
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_global_init_runtime_imports(cond, imports);
            collect_global_init_runtime_imports(then_branch, imports);
            collect_global_init_runtime_imports(else_branch, imports);
        }
        Expr::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_global_init_runtime_imports(scrutinee, imports);
            collect_global_init_runtime_imports(then_branch, imports);
            collect_global_init_runtime_imports(else_branch, imports);
        }
        Expr::Match { scrutinee, arms } => {
            collect_global_init_runtime_imports(scrutinee, imports);
            for arm in arms {
                collect_global_init_runtime_imports(&arm.body, imports);
            }
        }
        _ => {}
    }
}

fn global_text_literal_import(text: &str) -> BackendExternImport {
    let arity = text.as_bytes().len();
    global_builtin_import(
        &format!("std_string_literal_{arity}"),
        &format!("std.string_literal_{arity}"),
        &format!(
            "{}_to_externref",
            std::iter::repeat("i64")
                .take(arity)
                .collect::<Vec<_>>()
                .join("_")
        ),
    )
}

fn global_slice_literal_import(items: &[Expr]) -> BackendExternImport {
    let lane = if items.iter().all(global_static_expr_is_i32) {
        "i64"
    } else {
        "externref"
    };
    let arity = items.len();
    global_builtin_import(
        &format!(
            "std_slice_{}_literal_{arity}",
            if lane == "i64" { "i64" } else { "externref" }
        ),
        &format!(
            "std.slice_{}_literal_{arity}",
            if lane == "i64" { "i64" } else { "externref" }
        ),
        &format!(
            "{}_to_externref",
            std::iter::repeat(lane)
                .take(arity)
                .collect::<Vec<_>>()
                .join("_")
        ),
    )
}

fn global_vec_push_import(item: &Expr) -> BackendExternImport {
    if global_static_expr_is_i32(item) {
        global_builtin_import("std_vec_push", "std.vec_push", "externref_i64_to_externref")
    } else {
        global_builtin_import(
            "std_vec_push_externref",
            "std.vec_push_externref",
            "externref_externref_to_externref",
        )
    }
}

fn global_ref_new_import(type_name: &str, value: &Expr) -> BackendExternImport {
    let base = if type_name == "UnsafeRef" {
        "std.unsafe_ref_new"
    } else {
        "std.ref_new"
    };
    if global_static_expr_is_i32(value) {
        global_builtin_import(&base.replace('.', "_"), base, "i64_to_externref")
    } else {
        let name = format!("{base}_externref");
        global_builtin_import(&name.replace('.', "_"), &name, "externref_to_externref")
    }
}

fn global_cstr_literal_import(text: &str) -> BackendExternImport {
    let arity = text.as_bytes().len() + 1;
    global_builtin_import(
        &format!("std_cstr_literal_{arity}"),
        &format!("std.cstr_literal_{arity}"),
        &format!(
            "{}_to_externref",
            std::iter::repeat("i64")
                .take(arity)
                .collect::<Vec<_>>()
                .join("_")
        ),
    )
}

fn global_static_expr_is_i32(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Lit(
            crate::ast::Literal::I64(_)
                | crate::ast::Literal::Rune(_)
                | crate::ast::Literal::Bool(_)
        )
    )
}

fn global_builtin_import(
    final_symbol: &str,
    name: &str,
    signature_hash: &str,
) -> BackendExternImport {
    BackendExternImport {
        abi: BackendExternAbi::C,
        final_symbol: final_symbol.to_string(),
        module: "env".to_string(),
        name: name.to_string(),
        signature_hash: signature_hash.to_string(),
    }
}

fn render_program_extern_import_wat(wat: &mut String, import: &BackendExternImport) {
    let Some((params, result)) = program_extern_import_wat_signature(&import.signature_hash) else {
        return;
    };
    wat.push_str(&format!(
        "  (import \"{}\" \"{}\" (func ${}",
        import.module, import.name, import.final_symbol
    ));
    for param in params {
        wat.push_str(&format!(" (param {param})"));
    }
    if let Some(result) = result {
        wat.push_str(&format!(" (result {result})"));
    }
    wat.push_str("))\n");
}

fn program_extern_import_wat_signature(
    signature: &str,
) -> Option<(Vec<&'static str>, Option<&'static str>)> {
    let (params, result) = signature.split_once("_to_")?;
    let params = if params.is_empty() {
        Vec::new()
    } else {
        params
            .split('_')
            .map(program_extern_scalar_wat_type)
            .collect::<Option<Vec<_>>>()?
    };
    let result = match result {
        "unit" | "Unit" => None,
        "externref" => Some("externref"),
        scalar => Some(program_extern_scalar_wat_type(scalar)?),
    };
    Some((params, result))
}

fn program_extern_scalar_wat_type(scalar: &str) -> Option<&'static str> {
    match scalar {
        "i64" | "I64" | "bool" | "Bool" => Some("i32"),
        "externref" => Some("externref"),
        _ => None,
    }
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

fn render_global_init_expr_externref(
    wat: &mut String,
    static_name: &str,
    expr: &Expr,
    global_init: &GlobalInitPlan,
) -> Result<(), BackendLinkDiagnostic> {
    match expr {
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
            Ok(())
        }
        Expr::Lit(crate::ast::Literal::String(text)) => {
            render_global_text_literal(wat, text);
            Ok(())
        }
        Expr::Lit(crate::ast::Literal::CStr(text)) => {
            render_global_cstr_literal(wat, text);
            Ok(())
        }
        Expr::MethodCall {
            receiver,
            name,
            args,
        } => render_global_init_method_call_externref(
            wat,
            static_name,
            receiver,
            name,
            args,
            global_init,
        ),
        Expr::Range { start, end } => {
            render_global_init_expr_with_bindings(
                wat,
                static_name,
                start,
                &GlobalInitPlan::default(),
                &BTreeMap::new(),
            )?;
            render_global_init_expr_with_bindings(
                wat,
                static_name,
                end,
                &GlobalInitPlan::default(),
                &BTreeMap::new(),
            )?;
            wat.push_str("    call $std_range_i64_new\n");
            Ok(())
        }
        Expr::SliceLiteral(items) => {
            let i32_lane = items.iter().all(global_static_expr_is_i32);
            for item in items {
                if i32_lane {
                    render_global_init_expr_with_bindings(
                        wat,
                        static_name,
                        item,
                        &GlobalInitPlan::default(),
                        &BTreeMap::new(),
                    )?;
                } else {
                    render_global_init_expr_externref(wat, static_name, item, global_init)?;
                }
            }
            wat.push_str(&format!(
                "    call $std_slice_{}_literal_{}\n",
                if i32_lane { "i64" } else { "externref" },
                items.len()
            ));
            Ok(())
        }
        _ => Err(
            BackendLinkDiagnostic::UnsupportedStaticInitializerLowering {
                static_name: static_name.to_string(),
                expr: render_source_expr(expr),
            },
        ),
    }
}

fn render_global_init_method_call_externref(
    wat: &mut String,
    static_name: &str,
    receiver: &Expr,
    name: &str,
    args: &[Expr],
    global_init: &GlobalInitPlan,
) -> Result<(), BackendLinkDiagnostic> {
    if let Some(text) = global_string_from_literal(receiver, name, args) {
        render_global_text_literal(wat, text);
        wat.push_str("    call $std_str_to_string\n");
        return Ok(());
    }
    match (receiver, name, args) {
        (Expr::Var(type_name), "new", []) if type_name == "String" => {
            wat.push_str("    call $std_string_new\n");
        }
        (_, "concat", [arg]) => {
            render_global_init_expr_externref(wat, static_name, receiver, global_init)?;
            render_global_init_expr_externref(wat, static_name, arg, global_init)?;
            wat.push_str("    call $std_string_concat\n");
        }
        (_, "as_str", []) => {
            render_global_init_expr_externref(wat, static_name, receiver, global_init)?;
            wat.push_str("    call $std_string_as_str\n");
        }
        (_, "to_cstr", []) => {
            render_global_init_expr_externref(wat, static_name, receiver, global_init)?;
            wat.push_str("    call $std_string_to_cstr\n");
        }
        (_, "push_rune", [rune]) => {
            render_global_init_expr_externref(wat, static_name, receiver, global_init)?;
            render_global_init_expr_with_bindings(
                wat,
                static_name,
                rune,
                &GlobalInitPlan::default(),
                &BTreeMap::new(),
            )?;
            wat.push_str("    call $std_string_push_rune\n");
        }
        (Expr::Var(type_name), "new", []) if type_name == "Vec" => {
            wat.push_str("    call $std_vec_new\n");
        }
        (_, "push", [item]) => {
            render_global_init_expr_externref(wat, static_name, receiver, global_init)?;
            if global_static_expr_is_i32(item) {
                render_global_init_expr_with_bindings(
                    wat,
                    static_name,
                    item,
                    &GlobalInitPlan::default(),
                    &BTreeMap::new(),
                )?;
                wat.push_str("    call $std_vec_push\n");
            } else {
                render_global_init_expr_externref(wat, static_name, item, global_init)?;
                wat.push_str("    call $std_vec_push_externref\n");
            }
        }
        (_, "freeze", []) => {
            render_global_init_expr_externref(wat, static_name, receiver, global_init)?;
            wat.push_str("    call $std_vec_freeze\n");
        }
        (Expr::Var(type_name), "new", [value])
            if type_name == "Ref" || type_name == "UnsafeRef" =>
        {
            let prefix = if type_name == "UnsafeRef" {
                "std_unsafe_ref_new"
            } else {
                "std_ref_new"
            };
            if global_static_expr_is_i32(value) {
                render_global_init_expr_with_bindings(
                    wat,
                    static_name,
                    value,
                    &GlobalInitPlan::default(),
                    &BTreeMap::new(),
                )?;
                wat.push_str(&format!("    call ${prefix}\n"));
            } else {
                render_global_init_expr_externref(wat, static_name, value, global_init)?;
                wat.push_str(&format!("    call ${prefix}_externref\n"));
            }
        }
        _ => {
            return Err(
                BackendLinkDiagnostic::UnsupportedStaticInitializerLowering {
                    static_name: static_name.to_string(),
                    expr: render_source_expr(&Expr::MethodCall {
                        receiver: Box::new(receiver.clone()),
                        name: name.to_string(),
                        args: args.to_vec(),
                    }),
                },
            );
        }
    }
    Ok(())
}

fn render_global_text_literal(wat: &mut String, text: &str) {
    for byte in text.as_bytes() {
        wat.push_str(&format!("    i32.const {}\n", *byte as i32));
    }
    wat.push_str(&format!(
        "    call $std_string_literal_{}\n",
        text.as_bytes().len()
    ));
}

fn render_global_cstr_literal(wat: &mut String, text: &str) {
    for byte in text.as_bytes() {
        wat.push_str(&format!("    i32.const {}\n", *byte as i32));
    }
    wat.push_str("    i32.const 0\n");
    wat.push_str(&format!(
        "    call $std_cstr_literal_{}\n",
        text.as_bytes().len() + 1
    ));
}

fn global_string_from_literal<'a>(
    receiver: &Expr,
    name: &str,
    args: &'a [Expr],
) -> Option<&'a str> {
    match (receiver, name, args) {
        (Expr::Var(type_name), "from", [Expr::Lit(crate::ast::Literal::String(text))])
            if type_name == "String" =>
        {
            Some(text.as_str())
        }
        _ => None,
    }
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
        Expr::Lit(crate::ast::Literal::Rune(value)) => {
            wat.push_str(&format!("    i32.const {value}\n"));
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
        Pattern::Lit(crate::ast::Literal::String(_) | crate::ast::Literal::CStr(_)) => None,
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
                "    {}: {} -> {} ({}us)\n",
                event.name,
                event.input,
                event.output,
                event.elapsed.as_micros()
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
    for entry in &bundle.manifest.entries {
        out.push_str(&format!(
            "      symbol {} source={} source-path={} owner={} item={} specialization={} layout={} origin={} role={} stable-id={} ownership={}\n",
            entry.final_symbol,
            entry.source_debug_name,
            entry.source.source_path.as_deref().unwrap_or("none"),
            entry.source.owner_namespace.as_deref().unwrap_or("none"),
            entry.source.item_path.as_deref().unwrap_or("none"),
            entry.source.specialization_key.as_deref().unwrap_or("none"),
            entry.source.layout_key.as_deref().unwrap_or("none"),
            entry.pass_origin,
            entry.lowering_role,
            entry.stable_id,
            entry
                .ownership
                .map(render_ownership_decision)
                .unwrap_or("none")
        ));
    }
    out.push_str(&format!("    diagnostics={}\n", bundle.diagnostics.len()));
    for diagnostic in &bundle.diagnostics {
        out.push_str(&format!(
            "      diagnostic {}\n",
            render_backend_link_diagnostic(diagnostic)
        ));
    }
    out
}

fn render_ownership_decision(decision: crate::core::OwnershipDecision) -> &'static str {
    match decision {
        crate::core::OwnershipDecision::StackValue => "stack",
        crate::core::OwnershipDecision::InplaceReuse => "inplace-reuse",
        crate::core::OwnershipDecision::Rc => "rc",
        crate::core::OwnershipDecision::Arc => "arc",
        crate::core::OwnershipDecision::StaticData => "static-data",
        crate::core::OwnershipDecision::BorrowedView => "borrowed-view",
        crate::core::OwnershipDecision::DynPackage => "dyn-package",
    }
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
        "    namespaces={}\n",
        render_string_values(&surface.namespaces)
    ));
    out.push_str(&format!(
        "    duplicate-namespaces={}\n",
        render_string_values(&surface.duplicate_namespaces)
    ));
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
        ProgramDiagnostic::InvalidSourceFileHeader { reason } => {
            format!("invalid source file header: {reason}")
        }
        ProgramDiagnostic::DuplicateNamespace { namespace } => {
            format!("duplicate namespace {namespace}")
        }
        ProgramDiagnostic::MultipleEntries { names } => {
            format!("multiple entries [{}]", names.join(", "))
        }
        ProgramDiagnostic::EntryOnNonFunction { name } => {
            format!("entry on non-function {name}")
        }
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
        ProgramDiagnostic::ExplicitInstantiationArityMismatch {
            def,
            callee,
            expected,
            actual,
        } => format!(
            "explicit instantiation arity mismatch {def}: {callee} expected {expected} got {actual}"
        ),
        ProgramDiagnostic::ExplicitInstantiationOfNonTemplate { def, callee } => {
            format!("explicit instantiation of non-template {def}: {callee}")
        }
        ProgramDiagnostic::DefinitionReturnTypeMismatch {
            def,
            expected,
            actual,
        } => {
            format!("definition return type mismatch {def}: expected {expected} got {actual}")
        }
        ProgramDiagnostic::ShiftOutsideReset { def, binder } => {
            format!("shift outside reset {def}: {binder}")
        }
        ProgramDiagnostic::UnsafeMultiResumeCapture { def, binder } => {
            format!("unsafe multi-resume capture {def}: {binder}")
        }
        ProgramDiagnostic::Cont1ResumedMoreThanOnce { def, binder } => {
            format!("cont1 resumed more than once {def}: {binder}")
        }
        ProgramDiagnostic::NonSendCallablePassedToSendCallable { def, callee, arg } => {
            format!("non-send callable passed to send callable {def}: {callee}({arg})")
        }
        ProgramDiagnostic::InvalidAssignmentTarget { def, target_type } => {
            format!("invalid assignment target {def}: {target_type}")
        }
        ProgramDiagnostic::DynRowCoercionFailed {
            def,
            expected,
            actual,
        } => format!("dyn row coercion failed {def}: expected {expected}, actual {actual}"),
        ProgramDiagnostic::RowMemberCallableUnsatisfied {
            def,
            callee,
            field,
            actual,
        } => {
            format!("row member callable unsatisfied {def}: {callee}.{field} for {actual}")
        }
        ProgramDiagnostic::NonCallableFieldForCallableRowMember {
            def,
            callee,
            field,
            actual,
            field_type,
        } => format!(
            "non-callable field for callable row member {def}: {callee}.{field} for {actual} has {field_type}"
        ),
        ProgramDiagnostic::MethodSignatureMismatchForRowMember {
            def,
            callee,
            field,
            actual,
        } => format!(
            "method signature mismatch for row member {def}: {callee}.{field} for {actual}"
        ),
        ProgramDiagnostic::AmbiguousReceiverMethod {
            receiver,
            name,
            candidates,
        } => format!(
            "ambiguous receiver method {receiver}.{name}: [{}]",
            candidates.join(", ")
        ),
        ProgramDiagnostic::AmbiguousOperatorResolution {
            receiver,
            op,
            candidates,
        } => format!(
            "ambiguous operator resolution {receiver} {op}: [{}]",
            candidates.join(", ")
        ),
        ProgramDiagnostic::InvalidOperatorOperands { def, op, lhs, rhs } => {
            format!("invalid operator operands {def}: {lhs} {op} {rhs}")
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
