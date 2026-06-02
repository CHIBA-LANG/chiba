use crate::alpha::{alpha_expr, AlphaFacts};
use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    DataDecl, Expr, MethodReceiver, NamespaceDecl, ParamDecl, Pattern, SourceItem, SourceProgram,
    UseDecl,
};
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
use crate::pattern::PatternFacts;
use crate::resolve::{resolve_expr, resolve_expr_with_names, MethodIndex, NameIndex, ResolveFacts};
use crate::specialize::{plan_specialization, SpecializationFacts};
use crate::std_audit::{audit_std_dependencies, StdAuditReport};
use crate::symbol::encode_debug_symbol;
use crate::surface::{
    build_interface_summary, duplicate_constructor_names, duplicate_data_names,
    duplicate_top_level_names, duplicate_type_fields, duplicate_type_names, project_surface,
    InterfaceSummary, ProjectSurface,
};
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
        &PatternTypeEnv::default(),
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
    pattern_types: &PatternTypeEnv,
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
    let template = passes.record("L3Template", "AlphaExpr+ResolveFacts", "TemplateFacts", || {
        analyze_template_with_source(
            expr,
            explicit_generics,
            params,
            return_type,
            &alpha.expr,
            &resolve,
        )
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
    let typed_signature = passes.record(
        "L7TypedSignature",
        "DefHeader+MethodReceiver",
        "TypedSignature",
        || typed_signature(params, return_type, receiver, type_aliases, pattern_types),
    );
    let typed_env = typed_signature.type_env();
    let typed = passes.record("L7Typed", "SourceExpr+TypedSignature", "TypedExpr", || {
        type_expr_with_context(expr, &typed_env, type_context)
    });
    let pattern = passes.record("L8PatternElab", "TypedExpr+ParamPatterns", "PatternFacts", || {
        crate::pattern::analyze_patterns_with_params(&typed, params)
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
    let methods = MethodIndex::from_interface(interface);
    let type_context = type_context_from_interface(interface);
    let type_aliases = type_aliases_from_interface(interface);
    let pattern_types = PatternTypeEnv::from_data(&program.data, &type_aliases);
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
                    &pattern_types,
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
    pattern_types: &PatternTypeEnv,
) -> TypedSignature {
    TypedSignature {
        params: params
            .iter()
            .map(|param| {
                let ty = resolve_header_type(param.ty.as_deref(), receiver, type_aliases);
                TypedParam {
                    name: param.name.clone(),
                    pattern: param.pattern.clone(),
                    binding_types: pattern_types.bindings_for(&param.pattern, &ty),
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

#[derive(Clone, Debug, Default)]
struct PatternTypeEnv {
    data_generics: BTreeMap<String, Vec<String>>,
    constructors: BTreeMap<(String, String), Vec<Type>>,
}

impl PatternTypeEnv {
    fn from_data(data: &[DataDecl], type_aliases: &BTreeMap<String, String>) -> Self {
        let mut env = Self::default();
        for decl in data {
            env.data_generics.insert(decl.name.clone(), decl.generics.clone());
            for variant in &decl.variants {
                env.constructors.insert(
                    (decl.name.clone(), variant.name.clone()),
                    variant
                        .fields
                        .iter()
                        .map(|field| header_type_to_type_resolved(field, type_aliases))
                        .collect(),
                );
            }
        }
        env
    }

    fn bindings_for(&self, pattern: &Pattern, subject_ty: &str) -> Vec<(String, Type)> {
        let subject = header_type_to_type(subject_ty);
        let subject_data = nominal_type_name(&subject).map(|name| nominal_base_name(name).to_string());
        let substitutions = nominal_type_name(&subject)
            .and_then(|nominal| self.substitutions_for_subject(nominal))
            .unwrap_or_default();
        self.bindings_for_pattern(pattern, subject, subject_data.as_deref(), &substitutions)
    }

    fn bindings_for_pattern(
        &self,
        pattern: &Pattern,
        ty: Type,
        subject_data: Option<&str>,
        substitutions: &BTreeMap<String, Type>,
    ) -> Vec<(String, Type)> {
        match pattern {
            Pattern::Bind(name) => vec![(name.clone(), ty)],
            Pattern::Tuple(fields) => fields
                .iter()
                .enumerate()
                .flat_map(|(index, field)| {
                    let field_ty = match &ty {
                        Type::Tuple(types) => types.get(index).cloned().unwrap_or(Type::Unknown),
                        _ => Type::Unknown,
                    };
                    self.bindings_for_pattern(field, field_ty, subject_data, substitutions)
                })
                .collect(),
            Pattern::Record(fields) => fields
                .iter()
                .flat_map(|field| {
                    let field_ty = match &ty {
                        Type::Record(types) => types
                            .iter()
                            .find(|typed| typed.name == field.name)
                            .map(|typed| typed.ty.clone())
                            .unwrap_or(Type::Unknown),
                        _ => Type::Unknown,
                    };
                    self.bindings_for_pattern(&field.pattern, field_ty, subject_data, substitutions)
                })
                .collect(),
            Pattern::Constructor { data, ctor, args } => {
                let ctor_data = data.as_deref().or(subject_data);
                let payloads = ctor_data
                    .and_then(|data| self.constructors.get(&(data.to_string(), ctor.clone())))
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                args.iter()
                    .enumerate()
                    .flat_map(|(index, arg)| {
                        let field_ty = payloads
                            .get(index)
                            .map(|field| substitute_pattern_type(field, substitutions))
                            .unwrap_or(Type::Unknown);
                        self.bindings_for_pattern(arg, field_ty, subject_data, substitutions)
                    })
                    .collect()
            }
            Pattern::At { name, pattern } => {
                let mut bindings =
                    self.bindings_for_pattern(pattern, ty.clone(), subject_data, substitutions);
                bindings.push((name.clone(), ty));
                bindings
            }
            Pattern::Wildcard | Pattern::Lit(_) => Vec::new(),
        }
    }

    fn substitutions_for_subject(&self, subject: &str) -> Option<BTreeMap<String, Type>> {
        let (base, args) = parse_nominal_application_local(subject)?;
        let generics = self.data_generics.get(base)?;
        if generics.len() != args.len() {
            return None;
        }
        Some(
            generics
                .iter()
                .cloned()
                .zip(args.into_iter().map(|arg| header_type_to_type(&arg)))
                .collect(),
        )
    }
}

fn header_type_to_type_resolved(ty: &str, type_aliases: &BTreeMap<String, String>) -> Type {
    header_type_to_type(type_aliases.get(ty).map(String::as_str).unwrap_or(ty))
}

fn nominal_type_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Nominal(name) => Some(name),
        _ => None,
    }
}

fn nominal_base_name(name: &str) -> &str {
    name.find('[').map(|open| &name[..open]).unwrap_or(name)
}

fn substitute_pattern_type(ty: &Type, substitutions: &BTreeMap<String, Type>) -> Type {
    match ty {
        Type::Nominal(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| Type::Nominal(name.clone())),
        Type::Tuple(fields) => Type::Tuple(
            fields
                .iter()
                .map(|field| substitute_pattern_type(field, substitutions))
                .collect(),
        ),
        Type::Record(fields) => Type::Record(
            fields
                .iter()
                .map(|field| RecordTypeField {
                    name: field.name.clone(),
                    ty: substitute_pattern_type(&field.ty, substitutions),
                })
                .collect(),
        ),
        Type::Func(param, result) => Type::Func(
            Box::new(substitute_pattern_type(param, substitutions)),
            Box::new(substitute_pattern_type(result, substitutions)),
        ),
        Type::Adt { name, variants } => Type::Adt {
            name: name.clone(),
            variants: variants.clone(),
        },
        Type::Unknown => Type::Unknown,
        Type::I64 => Type::I64,
        Type::Bool => Type::Bool,
    }
}

fn parse_nominal_application_local(name: &str) -> Option<(&str, Vec<String>)> {
    let open = name.find('[')?;
    let close = name.rfind(']')?;
    if close <= open || close != name.len() - 1 {
        return None;
    }
    let base = &name[..open];
    let args = &name[open + 1..close];
    let args = if args.trim().is_empty() {
        Vec::new()
    } else {
        args.split(',').map(|arg| arg.trim().to_string()).collect()
    };
    Some((base, args))
}

fn type_context_from_interface(interface: &InterfaceSummary) -> TypeContext {
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
    context
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
            .map(|(type_name, field)| ProgramDiagnostic::DuplicateTypeField {
                type_name,
                field,
            }),
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
