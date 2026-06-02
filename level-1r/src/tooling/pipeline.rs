use crate::alpha::{alpha_expr, AlphaFacts};
use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Expr, MethodReceiver, NamespaceDecl, ParamDecl, Pattern, SourceItem, SourceProgram, UseDecl,
};
use crate::backend::{
    backend_cache_key, emit_wasm_gc_with_params, link_backend_artifacts, BackendArtifact,
    BackendCacheConfig, BackendCacheKey, BackendLinkedBundle,
};
use crate::closure::{analyze_alpha_closures, ClosureFacts};
use crate::closure_core_usage::{analyze_closure_core_usage, ClosureCoreUsageFacts};
use crate::closure_simplify::{simplify_closure_core, ClosureSimplificationFacts};
use crate::control::{analyze_control, ControlFacts};
use crate::core::{lower_core_with_facts, validate_core, CoreProgram, CoreValidation, CoreValue};
use crate::cps::{cps_program, CpsProgram};
use crate::cps_usage::{
    analyze_cps_usage, simplify_continuations, ContinuationSimplificationFacts, CpsUsageFacts,
};
use crate::debug::{render_visual_report, visual_report, VisualReport};
use crate::frontend::{parse_source_program, FrontendError, FrontendOutput, SourceItemSpan};
use crate::global::{analyze_global_init, GlobalInitDiagnostic, GlobalInitPlan};
use crate::lambda_lift::{lift_lambdas, LambdaLiftFacts};
use crate::monomorphize::{schedule_monomorphization, MonomorphizationPlan};
use crate::nanopass::PassReport;
use crate::pattern::PatternFacts;
use crate::resolve::{resolve_expr, resolve_expr_with_names, MethodIndex, NameIndex, ResolveFacts};
use crate::specialize::{plan_specialization, SpecializationFacts};
use crate::std_audit::{audit_std_dependencies, StdAuditReport};
use crate::surface::{
    build_interface_summary, duplicate_constructor_names, duplicate_data_names,
    duplicate_top_level_names, duplicate_type_fields, duplicate_type_names, project_surface,
    InterfaceSummary, ProjectSurface,
};
use crate::symbol::encode_debug_symbol;
use crate::template::{analyze_template_with_source, TemplateFacts};
use crate::template_audit::{audit_checked_templates, TemplateAuditReport};
use crate::typed::{
    type_expr_with_context, RecordTypeField, Type, TypeContext, TypeEnv, TypedExpr,
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
    DuplicateDef { name: String },
    DuplicateType { name: String },
    DuplicateTypeField { type_name: String, field: String },
    DuplicateData { name: String },
    DuplicateConstructor { name: String },
    DuplicateTopLevelName { name: String },
    DuplicateStatic { name: String },
    StaticFunctionNameConflict { name: String },
    StaticInitCycle { cycle: Vec<String> },
    MissingEntry,
    EntryHasParams { name: String, params: Vec<String> },
}

pub fn compile_expr(expr: &Expr) -> CompileOutput {
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
        &BTreeMap::new(),
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
    type_aliases: &BTreeMap<String, String>,
) -> CompileOutput {
    let mut passes = PassReport::default();
    let alpha = passes.record("L1Alpha", "SourceExpr", "AlphaFacts", || alpha_expr(expr));
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
        || typed_signature(params, return_type, receiver, type_aliases, type_context),
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
    let defs = passes.record(
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
            let linked = link_backend_artifacts(program_backend_artifacts(
                &defs,
                entry.as_deref(),
                &global_init,
            ));
            lower_global_init_into_linked_wat(linked, &global_init)
        },
    );
    let backend_cache_key = passes.record(
        "P8ProgramBackendCacheKey",
        "BackendLinkedBundle",
        "BackendCacheKey",
        || backend_cache_key(&backend_link, &BackendCacheConfig::default()),
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
            SourceItem::StaticValue { .. } => false,
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
                SourceItem::StaticValue { .. } => None,
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
            SourceItem::StaticValue { .. } => unreachable!("clause groups only contain def items"),
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
    let names = NameIndex::from_interface(interface);
    let methods = MethodIndex::from_interface(interface);
    let type_aliases = type_aliases_from_interface(interface);
    let type_context = type_context_from_interface(interface, &program.data, &type_aliases);
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
                output: compile_expr_with_indexes_and_generics(
                    name,
                    body,
                    names.clone(),
                    methods.clone(),
                    generics,
                    params,
                    return_type,
                    receiver,
                    &type_context,
                    &type_aliases,
                ),
            }),
            SourceItem::StaticValue { .. } => None,
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

fn typed_signature(
    params: &[ParamDecl],
    return_type: &Option<String>,
    receiver: &Option<MethodReceiver>,
    type_aliases: &BTreeMap<String, String>,
    type_context: &TypeContext,
) -> TypedSignature {
    TypedSignature {
        params: params
            .iter()
            .map(|param| {
                let ty = resolve_header_type(param.ty.as_deref(), receiver, type_aliases);
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
            .map(|ty| resolve_header_type(Some(ty), receiver, type_aliases)),
    }
}

fn resolve_header_type(
    ty: Option<&str>,
    receiver: &Option<MethodReceiver>,
    type_aliases: &BTreeMap<String, String>,
) -> String {
    match ty {
        Some("Self") => receiver
            .as_ref()
            .map(MethodReceiver::display_name)
            .unwrap_or_else(|| "Self".to_string()),
        Some(ty) => type_aliases
            .get(ty)
            .cloned()
            .unwrap_or_else(|| ty.to_string()),
        None => "Unknown".to_string(),
    }
}

fn header_type_to_type(ty: &str) -> Type {
    match ty {
        "Unknown" => Type::Unknown,
        "I64" | "i64" => Type::I64,
        "Bool" | "bool" => Type::Bool,
        ty => Type::Nominal(ty.to_string()),
    }
}

fn type_context_from_interface(
    interface: &InterfaceSummary,
    data_decls: &[crate::ast::DataDecl],
    type_aliases: &BTreeMap<String, String>,
) -> TypeContext {
    let mut context = TypeContext::new();
    for ty in &interface.types {
        if ty.alias_target.is_some() {
            continue;
        }
        let Some(name) = ty.symbol.rsplit("::").next() else {
            continue;
        };
        let fields = ty
            .fields
            .iter()
            .map(|field| RecordTypeField {
                name: field.name.clone(),
                ty: header_type_to_type(&field.ty),
            })
            .collect::<Vec<_>>();
        let display_name = nominal_display_name(name, &ty.generics);
        context.insert_nominal_row(display_name, fields.clone());
        if !ty.generics.is_empty() {
            context.insert_generic_nominal_row(name, ty.generics.clone(), fields);
        }
    }
    for data in data_decls {
        for variant in &data.variants {
            context.insert_data_constructor(
                data.name.clone(),
                data.generics.clone(),
                variant.name.clone(),
                variant
                    .fields
                    .iter()
                    .map(|field| header_type_to_type(resolve_type_alias(field, type_aliases)))
                    .collect(),
            );
        }
    }
    context
}

fn resolve_type_alias<'a>(ty: &'a str, type_aliases: &'a BTreeMap<String, String>) -> &'a str {
    type_aliases.get(ty).map(String::as_str).unwrap_or(ty)
}

fn type_aliases_from_interface(interface: &InterfaceSummary) -> BTreeMap<String, String> {
    interface
        .types
        .iter()
        .filter_map(|ty| {
            let name = ty.symbol.rsplit("::").next()?;
            let target = ty.alias_target.clone()?;
            Some((name.to_string(), target))
        })
        .collect()
}

fn nominal_display_name(name: &str, generics: &[String]) -> String {
    if generics.is_empty() {
        name.to_string()
    } else {
        format!("{}[{}]", name, generics.join(","))
    }
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
    global_init: &GlobalInitPlan,
) -> Vec<BackendArtifact> {
    let mut entry_exported = false;
    let static_names = global_init
        .statics
        .iter()
        .map(|static_value| static_value.name.as_str())
        .collect::<BTreeSet<_>>();
    defs.iter()
        .enumerate()
        .map(|(index, def)| {
            let mut artifact = def.output.backend.clone();
            let is_entry = entry == Some(def.name.as_str()) && !entry_exported;
            if is_entry {
                entry_exported = true;
            }
            artifact.wat = relabel_program_wat(
                &artifact,
                &def_symbol(&def.name, index),
                is_entry,
                &static_names,
            );
            artifact
        })
        .collect()
}

fn relabel_program_wat(
    artifact: &BackendArtifact,
    symbol: &str,
    is_entry: bool,
    static_names: &BTreeSet<&str>,
) -> String {
    let wat = &artifact.wat;
    let relabeled = if is_entry {
        wat.replace(
            "(func $main (export \"main\")",
            &format!("(func ${symbol} (export \"main\")"),
        )
    } else {
        wat.replace("(func $main (export \"main\")", &format!("(func ${symbol}"))
    };
    replace_static_return_from_fact(
        &relabeled,
        artifact.return_value.as_ref(),
        static_names,
        is_entry,
    )
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

fn lower_global_init_into_linked_wat(
    mut bundle: BackendLinkedBundle,
    global_init: &GlobalInitPlan,
) -> BackendLinkedBundle {
    if !bundle.diagnostics.is_empty() || global_init.statics.is_empty() {
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
        let initializer = const_global_initializer(&static_value.body).unwrap_or(0);
        wat.push_str(&format!(
            "  (global ${} (mut i32) (i32.const {}))\n",
            global_symbol(&static_value.name),
            initializer
        ));
    }
    wat.push_str("  (func $__chiba_init\n");
    for name in &global_init.init_order {
        let Some(static_value) = global_init.statics.iter().find(|item| item.name == *name) else {
            continue;
        };
        if const_global_initializer(&static_value.body).is_some() {
            continue;
        }
        wat.push_str(&format!("    ;; init static {}\n", static_value.name));
        render_global_init_expr(&mut wat, &static_value.body, global_init);
        wat.push_str(&format!(
            "    global.set ${}\n",
            global_symbol(&static_value.name)
        ));
    }
    wat.push_str("  )\n");
    wat.push_str("  (start $__chiba_init)\n");
    wat.push_str(body);
    wat.push_str(")\n");
    bundle.linked_wat = wat;
    bundle
}

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
                crate::ast::BinaryOp::Div => (rhs != 0).then(|| lhs / rhs),
            }
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            if const_global_initializer(cond)? != 0 {
                const_global_initializer(then_branch)
            } else {
                const_global_initializer(else_branch)
            }
        }
        Expr::AdtCtor { ctor, variants, .. } => variants
            .iter()
            .position(|variant| variant == ctor)
            .map(|tag| tag as i32),
        _ => None,
    }
}

fn replace_static_return_from_fact(
    wat: &str,
    return_value: Option<&CoreValue>,
    static_names: &BTreeSet<&str>,
    is_entry: bool,
) -> String {
    let Some(CoreValue::Var(name)) = return_value else {
        return wat.to_string();
    };
    if !is_entry || !static_names.contains(name.as_str()) {
        return wat.to_string();
    }
    let Some(const_start) = wat.find("    i32.const 0") else {
        return wat.to_string();
    };
    let const_end = const_start + "    i32.const 0".len();
    let replacement = format!("    global.get ${}", global_symbol(name));
    let mut out = String::new();
    out.push_str(&wat[..const_start]);
    out.push_str(&replacement);
    out.push_str(&wat[const_end..]);
    out
}

fn render_global_init_expr(wat: &mut String, expr: &Expr, global_init: &GlobalInitPlan) {
    render_global_init_expr_with_bindings(wat, expr, global_init, &BTreeMap::new());
}

fn render_global_init_expr_with_bindings(
    wat: &mut String,
    expr: &Expr,
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
) {
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
                bindings.get(name).expect("checked binding"),
                global_init,
                bindings,
            );
        }
        Expr::Var(name) if global_init.statics.iter().any(|item| item.name == *name) => {
            wat.push_str(&format!("    global.get ${}\n", global_symbol(name)));
        }
        Expr::Binary { op, lhs, rhs } => {
            render_global_init_expr_with_bindings(wat, lhs, global_init, bindings);
            render_global_init_expr_with_bindings(wat, rhs, global_init, bindings);
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
            render_global_init_expr_with_bindings(wat, cond, global_init, bindings);
            wat.push_str("    if (result i32)\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                then_branch,
                global_init,
                bindings,
                6,
            );
            wat.push_str("    else\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                else_branch,
                global_init,
                bindings,
                6,
            );
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
                pattern,
                scrutinee,
                then_branch,
                else_branch,
                global_init,
                bindings,
                4,
            );
        }
        Expr::Match { scrutinee, arms } => {
            render_global_match_expr(wat, scrutinee, arms, global_init, bindings, 4);
        }
        Expr::Field { receiver, name } => {
            if let Some(value) = global_record_field_expr(receiver, name, bindings) {
                render_global_init_expr_with_bindings(wat, value, global_init, bindings);
            } else {
                wat.push_str(&format!("    ;; unsupported static init {:?}\n", expr));
                wat.push_str("    i32.const 0\n");
            }
        }
        Expr::AdtCtor { ctor, variants, .. } => {
            let tag = variants
                .iter()
                .position(|variant| variant == ctor)
                .unwrap_or(0);
            wat.push_str(&format!("    i32.const {tag}\n"));
        }
        other => {
            wat.push_str(&format!("    ;; unsupported static init {:?}\n", other));
            wat.push_str("    i32.const 0\n");
        }
    }
}

fn render_global_init_expr_nested_with_bindings(
    wat: &mut String,
    expr: &Expr,
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
    indent: usize,
) {
    let mut nested = String::new();
    render_global_init_expr_with_bindings(&mut nested, expr, global_init, bindings);
    for line in nested.lines() {
        wat.push_str(&" ".repeat(indent));
        wat.push_str(line.trim_start());
        wat.push('\n');
    }
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
    scrutinee: &Expr,
    arms: &[crate::ast::MatchArm],
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
    indent: usize,
) {
    let Some((first, rest)) = arms.split_first() else {
        push_global_indent(wat, indent);
        wat.push_str("unreachable\n");
        return;
    };
    render_global_match_arm(wat, scrutinee, first, rest, global_init, bindings, indent);
}

fn render_global_if_let_expr(
    wat: &mut String,
    pattern: &Pattern,
    scrutinee: &Expr,
    then_branch: &Expr,
    else_branch: &Expr,
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
    indent: usize,
) {
    match global_pattern_bindings(pattern, scrutinee, bindings) {
        Some(GlobalPatternMatch::Always(next_bindings)) => {
            render_global_init_expr_nested_with_bindings(
                wat,
                then_branch,
                global_init,
                &next_bindings,
                indent,
            );
        }
        Some(GlobalPatternMatch::Conditional {
            expected,
            bindings: next_bindings,
        }) => {
            render_global_init_expr_nested_with_bindings(
                wat,
                scrutinee,
                global_init,
                bindings,
                indent,
            );
            push_global_indent(wat, indent);
            wat.push_str(&format!("i32.const {expected}\n"));
            push_global_indent(wat, indent);
            wat.push_str("i32.eq\n");
            push_global_indent(wat, indent);
            wat.push_str("if (result i32)\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                then_branch,
                global_init,
                &next_bindings,
                indent + 2,
            );
            push_global_indent(wat, indent);
            wat.push_str("else\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                else_branch,
                global_init,
                bindings,
                indent + 2,
            );
            push_global_indent(wat, indent);
            wat.push_str("end\n");
        }
        None => {
            render_global_init_expr_nested_with_bindings(
                wat,
                else_branch,
                global_init,
                bindings,
                indent,
            );
        }
    }
}

fn render_global_match_arm(
    wat: &mut String,
    scrutinee: &Expr,
    arm: &crate::ast::MatchArm,
    rest: &[crate::ast::MatchArm],
    global_init: &GlobalInitPlan,
    bindings: &BTreeMap<String, Expr>,
    indent: usize,
) {
    match global_pattern_bindings(&arm.pattern, scrutinee, bindings) {
        Some(GlobalPatternMatch::Always(next_bindings)) => {
            render_global_init_expr_nested_with_bindings(
                wat,
                &arm.body,
                global_init,
                &next_bindings,
                indent,
            );
        }
        Some(GlobalPatternMatch::Conditional {
            expected,
            bindings: next_bindings,
        }) => {
            render_global_init_expr_nested_with_bindings(
                wat,
                scrutinee,
                global_init,
                bindings,
                indent,
            );
            push_global_indent(wat, indent);
            wat.push_str(&format!("i32.const {expected}\n"));
            push_global_indent(wat, indent);
            wat.push_str("i32.eq\n");
            push_global_indent(wat, indent);
            wat.push_str("if (result i32)\n");
            render_global_init_expr_nested_with_bindings(
                wat,
                &arm.body,
                global_init,
                &next_bindings,
                indent + 2,
            );
            push_global_indent(wat, indent);
            wat.push_str("else\n");
            render_global_match_expr(wat, scrutinee, rest, global_init, bindings, indent + 2);
            push_global_indent(wat, indent);
            wat.push_str("end\n");
        }
        None => render_global_match_expr(wat, scrutinee, rest, global_init, bindings, indent),
    }
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
        Pattern::Lit(crate::ast::Literal::Bool(expected)) => {
            Some(GlobalPatternMatch::Conditional {
                expected: i32::from(*expected),
                bindings: bindings.clone(),
            })
        }
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

fn global_symbol(name: &str) -> String {
    format!("global__{}", encode_debug_symbol(name))
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
            self.imports.iter().map(UseDecl::dotted).collect::<Vec<_>>()
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
