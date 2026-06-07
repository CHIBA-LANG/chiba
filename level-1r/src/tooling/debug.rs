use std::fmt::Write;

use crate::alpha::{AlphaDiagnostic, AlphaExprKind, AlphaFacts};
use crate::ast::{
    render_source_binary_op, render_source_expr, render_source_literal, render_source_pattern, Expr,
};
use crate::backend::{
    BackendArtifact, BackendCacheKey, BackendDiagnostic, BackendExternAbi, BackendLinkDiagnostic,
    BackendLinkedBundle, BackendManifest, BackendTarget,
};
use crate::closure::{ClosureFacts, ClosureStorageKind};
use crate::closure_core_usage::ClosureCoreUsageFacts;
use crate::closure_simplify::{
    ClosurePackageDecision, ClosureSimplificationFacts, CodePointerDecision,
    ContinuationPackageDecision, EnvFieldDecision,
};
use crate::control::{ContinuationKind, ControlError, ControlFacts, ReplaySafety};
use crate::core::{
    CallableStorageFact, CallableStorageKind, CompilerIntrinsic, CoreCapturedContinuation,
    CoreDiagnostic, CoreMatchArm, CoreOp, CorePattern, CoreProgram, CoreRecordValueField,
    CoreValidation, CoreValue, LayoutKind, OperatorIntrinsic, OwnershipDecision,
    OwnershipSubjectKind, TargetSpecificCoreTerm,
};
use crate::cps::CpsProgram;
use crate::cps_usage::{
    ContinuationMaterialization, ContinuationSimplificationFacts, CpsUsageDiagnostic, CpsUsageFacts,
};
use crate::lambda_lift::LambdaLiftFacts;
use crate::monomorphize::{MonomorphizationPlan, MonomorphizationStatus};
use crate::nanopass::PassReport;
use crate::pattern::{PatternDiagnostic, PatternFacts};
use crate::resolve::{
    OperatorSurface, ResolveDiagnostic, ResolveFacts, ResolvedCall, ResolvedName,
};
use crate::specialize::{AbiMode, DischargedObligation, SpecializationFacts, SpecializationKey};
use crate::std_audit::{StdAuditDiagnostic, StdAuditReport, StdCapability, StdClassification};
use crate::template::{
    DynAdapterKind, DynRowContract, RowOpenness, RowShape, ShapeType, TemplateDiagnostic,
    TemplateFacts, TemplateObligation, TemplateParamSource,
};
use crate::template_audit::{
    TemplateAuditDiagnostic, TemplateAuditReport, TemplateObligationSource,
};
use crate::typed::{FieldAccessKind, RecordTypeField, Type, TypedExpr, TypedExprKind};
use crate::usage::{UsageFacts, UseCount};
use crate::usage_audit::{RustReferenceOwnership, UsageAuditDiagnostic, UsageAuditReport};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualReport {
    pub source: String,
    pub alpha: String,
    pub resolve: String,
    pub template: String,
    pub specialize: String,
    pub symbol_lineage: String,
    pub monomorphize: String,
    pub template_audit: String,
    pub typed_signature: String,
    pub typed: String,
    pub pattern: String,
    pub control: String,
    pub usage: String,
    pub cps: String,
    pub cps_usage: String,
    pub continuation_simplification: String,
    pub closure: String,
    pub lambda_lift: String,
    pub core: String,
    pub callable_storage: String,
    pub closure_core_usage: String,
    pub closure_simplification: String,
    pub usage_audit: String,
    pub std_audit: String,
    pub core_validation: String,
    pub backend: String,
    pub backend_link: String,
    pub backend_cache_key: String,
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
    writeln!(out, "symbol-lineage:").unwrap();
    for line in report.symbol_lineage.lines() {
        writeln!(out, "  {line}").unwrap();
    }
    writeln!(out, "monomorphize:").unwrap();
    writeln!(out, "  {}", report.monomorphize).unwrap();
    writeln!(out, "template-audit:").unwrap();
    writeln!(out, "  {}", report.template_audit).unwrap();
    writeln!(out, "typed-signature: {}", report.typed_signature).unwrap();
    writeln!(out, "typed:").unwrap();
    writeln!(out, "  {}", report.typed).unwrap();
    writeln!(out, "pattern:").unwrap();
    writeln!(out, "  {}", report.pattern).unwrap();
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
    writeln!(out, "callable-storage:").unwrap();
    for line in report.callable_storage.lines() {
        writeln!(out, "  {line}").unwrap();
    }
    writeln!(out, "closure-core-usage:").unwrap();
    writeln!(out, "  {}", report.closure_core_usage).unwrap();
    writeln!(out, "closure-simplification:").unwrap();
    writeln!(out, "  {}", report.closure_simplification).unwrap();
    writeln!(out, "usage-audit:").unwrap();
    writeln!(out, "  {}", report.usage_audit).unwrap();
    writeln!(out, "std-audit:").unwrap();
    writeln!(out, "  {}", report.std_audit).unwrap();
    writeln!(out, "core-validation:").unwrap();
    writeln!(out, "  {}", report.core_validation).unwrap();
    writeln!(out, "backend:").unwrap();
    writeln!(out, "  {}", report.backend).unwrap();
    writeln!(out, "backend-link:").unwrap();
    writeln!(out, "  {}", report.backend_link).unwrap();
    writeln!(out, "backend-cache-key:").unwrap();
    writeln!(out, "  {}", report.backend_cache_key).unwrap();
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
    monomorphize: &MonomorphizationPlan,
    template_audit: &TemplateAuditReport,
    typed_signature: &str,
    typed: &TypedExpr,
    pattern: &PatternFacts,
    control: &ControlFacts,
    usage: &UsageFacts,
    cps: &CpsProgram,
    cps_usage: &CpsUsageFacts,
    continuation_simplification: &ContinuationSimplificationFacts,
    closure: &ClosureFacts,
    lambda_lift: &LambdaLiftFacts,
    core: &CoreProgram,
    callable_storage: &str,
    closure_core_usage: &ClosureCoreUsageFacts,
    closure_simplification: &ClosureSimplificationFacts,
    usage_audit: &UsageAuditReport,
    std_audit: &StdAuditReport,
    core_validation: &CoreValidation,
    backend: &BackendArtifact,
    backend_link: &BackendLinkedBundle,
    backend_cache_key: &BackendCacheKey,
    passes: &PassReport,
) -> VisualReport {
    VisualReport {
        source: render_source_expr(source),
        alpha: render_alpha_facts(alpha),
        resolve: render_resolve_facts(resolve),
        template: render_template_facts(template),
        specialize: render_specialization_facts(specialize),
        symbol_lineage: render_symbol_lineage(resolve, template, specialize),
        monomorphize: render_monomorphization_plan(monomorphize),
        template_audit: render_template_audit(template_audit),
        typed_signature: typed_signature.to_string(),
        typed: render_typed_expr(typed),
        pattern: render_pattern_facts(pattern),
        control: render_control_facts(control),
        usage: render_usage_facts(usage),
        cps: cps.to_string(),
        cps_usage: render_cps_usage_facts(cps_usage),
        continuation_simplification: render_continuation_simplification(
            continuation_simplification,
        ),
        closure: render_closure_facts(closure),
        lambda_lift: render_lambda_lift_facts(lambda_lift),
        core: render_core_program(core),
        callable_storage: callable_storage.to_string(),
        closure_core_usage: render_closure_core_usage(closure_core_usage),
        closure_simplification: render_closure_simplification(closure_simplification),
        usage_audit: render_usage_audit(usage_audit),
        std_audit: render_std_audit(std_audit),
        core_validation: render_core_validation(core_validation),
        backend: render_backend_artifact(backend),
        backend_link: render_backend_link(backend_link),
        backend_cache_key: render_backend_cache_key(backend_cache_key),
        nanopass: render_pass_report(passes),
    }
}

fn render_core_validation(validation: &CoreValidation) -> String {
    let mut out = String::new();
    writeln!(out, "diagnostics={}", validation.diagnostics.len()).unwrap();
    for diagnostic in &validation.diagnostics {
        writeln!(out, "diagnostic {}", render_core_diagnostic(diagnostic)).unwrap();
    }
    out
}

fn render_core_program(program: &CoreProgram) -> String {
    let mut out = String::new();
    writeln!(out, "ops={}", program.ops.len()).unwrap();
    for (index, op) in program.ops.iter().enumerate() {
        writeln!(out, "op {index} {}", render_core_op(op)).unwrap();
    }
    writeln!(out, "layouts={}", program.layouts.len()).unwrap();
    for layout in &program.layouts {
        writeln!(
            out,
            "layout {} hash={} {}",
            layout.key,
            layout.hash,
            render_layout_kind(&layout.kind)
        )
        .unwrap();
    }
    writeln!(out, "ownership={}", program.ownership.len()).unwrap();
    for fact in &program.ownership {
        writeln!(
            out,
            "ownership {} kind={} decision={}",
            fact.subject,
            render_ownership_subject_kind(fact.kind),
            render_ownership_decision(fact.decision)
        )
        .unwrap();
    }
    writeln!(out, "callable-storage={}", program.callable_storage.len()).unwrap();
    render_callable_storage_lines(&mut out, &program.callable_storage);
    out
}

fn render_core_op(op: &CoreOp) -> String {
    match op {
        CoreOp::ReturnValue(value) => format!("return {}", render_core_value(value)),
        CoreOp::ReturnBranch {
            cond,
            then_value,
            else_value,
        } => format!(
            "return-branch cond={} then={} else={}",
            render_core_value(cond),
            render_core_value(then_value),
            render_core_value(else_value)
        ),
        CoreOp::ReturnMatch { scrutinee, arms } => {
            let arms = arms
                .iter()
                .map(render_core_match_arm)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "return-match scrutinee={} arms=[{}]",
                render_core_value(scrutinee),
                arms
            )
        }
        CoreOp::TailCall { func, args } => {
            let args = args
                .iter()
                .map(render_core_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("tail-call {func} args=[{args}]")
        }
        CoreOp::TailCallResult { binder } => format!("tail-call-result {binder}"),
        CoreOp::DynamicCallableTarget { target } => {
            format!("dynamic-callable-target {target}")
        }
        CoreOp::ExternFunctionTarget {
            target,
            owner,
            symbol,
            abi,
            name,
            signature,
        } => format!(
            "extern-function-target {target} owner={owner} symbol={symbol} abi={} name={name} signature={signature}",
            render_core_extern_abi(*abi)
        ),
        CoreOp::Prompt { kind } => format!("prompt kind={}", render_continuation_kind(*kind)),
        CoreOp::CaptureContinuation {
            binder,
            kind,
            captured,
        } => format!(
            "capture-continuation {binder} kind={} {}",
            render_continuation_kind(*kind),
            render_captured_continuation(captured)
        ),
        CoreOp::DirectMethodTarget { name, target } => {
            format!("direct-method-target {name} -> {target}")
        }
        CoreOp::OperatorTarget {
            protocol,
            target,
            intrinsic,
        } => format!(
            "operator-target protocol={protocol} target={target} intrinsic={}",
            intrinsic.map(render_operator_intrinsic).unwrap_or("none")
        ),
        CoreOp::StaticRowAccess { field, layout } => {
            format!("static-row-access field={field} layout={layout}")
        }
        CoreOp::DynRowAdapterAccess { subject, layout } => {
            format!("dyn-row-adapter-access subject={subject} layout={layout}")
        }
        CoreOp::Branch { cond } => format!("branch cond={cond}"),
        CoreOp::Match {
            scrutinee,
            patterns,
        } => {
            format!(
                "match scrutinee={scrutinee} patterns=[{}]",
                patterns.join(", ")
            )
        }
        CoreOp::TupleConstruct {
            nominal,
            layout,
            fields,
        } => format!(
            "tuple-construct nominal={nominal} layout={layout} fields=[{}]",
            fields.join(", ")
        ),
        CoreOp::TupleFieldGet {
            nominal,
            layout,
            field,
            field_index,
        } => format!(
            "tuple-field-get nominal={nominal} layout={layout} field={field} index={field_index}"
        ),
        CoreOp::RecordConstruct { layout, fields } => {
            format!(
                "record-construct layout={layout} fields=[{}]",
                fields.join(", ")
            )
        }
        CoreOp::RecordUpdate {
            base,
            layout,
            fields,
        } => format!(
            "record-update base={base} layout={layout} fields=[{}]",
            fields.join(", ")
        ),
        CoreOp::RecordFieldGet { layout, field } => {
            format!("record-field-get layout={layout} field={field}")
        }
        CoreOp::RangeFieldGet { field } => {
            format!("range-field-get field={}", field.source_name())
        }
        CoreOp::SliceFieldGet { field } => {
            format!("slice-field-get field={}", field.source_name())
        }
        CoreOp::AdtConstruct {
            data,
            ctor,
            variants,
            args,
        } => format!(
            "adt-construct {data}.{ctor} variants=[{}] args=[{}]",
            variants.join(", "),
            args.join(", ")
        ),
        CoreOp::AdtTupleBridge {
            data,
            ctor,
            tuple_fields,
            tuple_to_adt_intrinsic,
            adt_to_tuple_intrinsic,
        } => format!(
            "adt-tuple-bridge {data}.{ctor} tuple-fields=[{}] tuple-to-adt={} adt-to-tuple={}",
            tuple_fields.join(", "),
            render_compiler_intrinsic(*tuple_to_adt_intrinsic),
            render_compiler_intrinsic(*adt_to_tuple_intrinsic)
        ),
        CoreOp::CompilerIntrinsicUse {
            intrinsic,
            owner_namespace,
            subject,
        } => format!(
            "compiler-intrinsic-use {} owner={} subject={}",
            render_compiler_intrinsic(*intrinsic),
            owner_namespace,
            subject
        ),
        CoreOp::TargetSpecificTerm { term } => {
            format!(
                "target-specific-term {}",
                render_target_specific_core_term(*term)
            )
        }
        CoreOp::LiftedFunction {
            source,
            symbol,
            env_params,
            direct,
        } => format!(
            "lifted-function source={source} symbol={symbol} env=[{}] direct={direct}",
            env_params.join(", ")
        ),
    }
}

fn render_core_match_arm(arm: &CoreMatchArm) -> String {
    format!(
        "{} => {}",
        render_core_pattern(&arm.pattern),
        render_core_value(&arm.value)
    )
}

fn render_captured_continuation(captured: &CoreCapturedContinuation) -> String {
    let ops = captured
        .ops
        .iter()
        .map(render_core_op)
        .collect::<Vec<_>>()
        .join("; ");
    format!("param={} ops=[{}]", captured.param, ops)
}

fn render_core_pattern(pattern: &CorePattern) -> String {
    match pattern {
        CorePattern::Wildcard => "_".to_string(),
        CorePattern::Bind(name) => name.clone(),
        CorePattern::I64(value) => value.to_string(),
        CorePattern::Bool(value) => value.to_string(),
        CorePattern::Constructor { data, ctor, args } => {
            let args = args
                .iter()
                .map(render_core_pattern)
                .collect::<Vec<_>>()
                .join(", ");
            match data {
                Some(data) => format!("{data}.{ctor}({args})"),
                None => format!("{ctor}({args})"),
            }
        }
    }
}

fn render_core_value(value: &CoreValue) -> String {
    match value {
        CoreValue::Unit => "unit".to_string(),
        CoreValue::I64(value) => value.to_string(),
        CoreValue::Bool(value) => value.to_string(),
        CoreValue::Var(name) => name.clone(),
        CoreValue::Tuple { fields } => {
            let fields = fields
                .iter()
                .map(render_core_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({fields})")
        }
        CoreValue::TupleField {
            tuple,
            field,
            field_index,
        } => format!("{}.{}#{}", render_core_value(tuple), field, field_index),
        CoreValue::Range { start, end } => {
            format!("{}..{}", render_core_value(start), render_core_value(end))
        }
        CoreValue::SliceLiteral { items } => {
            let items = items
                .iter()
                .map(render_core_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{items}]")
        }
        CoreValue::Record { fields } => {
            let fields = fields
                .iter()
                .map(render_core_record_value_field)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{fields}}}")
        }
        CoreValue::RecordUpdate { base, fields } => {
            let fields = fields
                .iter()
                .map(render_core_record_value_field)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{} | {fields}}}", render_core_value(base))
        }
        CoreValue::RecordField { record, field } => {
            format!("{}.{}", render_core_value(record), field)
        }
        CoreValue::RangeField { range, field } => {
            format!("{}.{}", render_core_value(range), field.source_name())
        }
        CoreValue::AggregateField {
            kind: _,
            value,
            field,
        } => {
            format!("{}.{}", render_core_value(value), field.source_name())
        }
        CoreValue::TextField {
            kind: _,
            value,
            field,
        } => {
            format!("{}.{}", render_core_value(value), field.source_name())
        }
        CoreValue::AggregateIndex {
            kind: _,
            value,
            index,
        } => {
            format!("{}[{}]", render_core_value(value), render_core_value(index))
        }
        CoreValue::TextIndex {
            kind: _,
            value,
            index,
        } => {
            format!("{}[{}]", render_core_value(value), render_core_value(index))
        }
        CoreValue::VecRuntimeCall { call, args } => {
            let args = args
                .iter()
                .map(render_core_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({args})", call.debug_name())
        }
        CoreValue::Adt {
            data,
            ctor,
            variants,
            args,
        } => {
            let args = args
                .iter()
                .map(render_core_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{data}.{ctor} variants=[{}] args=[{}]",
                variants.join(", "),
                args
            )
        }
        CoreValue::Rendered { debug } => format!("rendered({debug})"),
    }
}

fn render_core_record_value_field(field: &CoreRecordValueField) -> String {
    format!("{}={}", field.name, render_core_value(&field.value))
}

fn render_layout_kind(kind: &LayoutKind) -> String {
    match kind {
        LayoutKind::RowShape(shape) => format!("row-shape {}", render_row_shape(shape)),
        LayoutKind::DynRowPackage(contract) => {
            format!("dyn-row-package {}", render_dyn_contract(contract))
        }
        LayoutKind::ContinuationPackage(env) => {
            format!(
                "continuation-package {}",
                render_continuation_env_layout(env)
            )
        }
        LayoutKind::Cont1StateMachine(env) => {
            format!(
                "cont1-state-machine {}",
                render_continuation_env_layout(env)
            )
        }
        LayoutKind::ClosureEnv(env) => {
            let fields = env
                .fields
                .iter()
                .map(render_closure_env_field)
                .collect::<Vec<_>>()
                .join(", ");
            format!("closure-env closure={} fields=[{}]", env.closure, fields)
        }
        LayoutKind::TupleStruct(tuple) => format!(
            "tuple-struct nominal={} fields=[{}]",
            tuple.nominal,
            tuple.fields.join(", ")
        ),
        LayoutKind::RecordStruct(record) => {
            format!("record-struct fields=[{}]", record.fields.join(", "))
        }
        LayoutKind::AdtShape(adt) => format!(
            "adt-shape data={} variants=[{}]",
            adt.data,
            adt.variants.join(", ")
        ),
    }
}

fn render_continuation_env_layout(env: &crate::core::ContinuationEnvLayout) -> String {
    let fields = env
        .fields
        .iter()
        .map(render_closure_env_field)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "binder={} kind={} fields=[{}] replay={} clone-on-resume={} consumed-state-machine={}",
        env.binder,
        render_continuation_kind(env.kind),
        fields,
        render_replay_safety(env.replay_safety),
        env.clone_on_resume,
        env.consumed_state_machine
    )
}

fn render_closure_env_field(field: &crate::core::ClosureEnvField) -> String {
    format!(
        "{}: usage={} send={}",
        field.name,
        render_usage_color(field.usage),
        render_send_color(field.send)
    )
}

fn render_ownership_subject_kind(kind: OwnershipSubjectKind) -> String {
    match kind {
        OwnershipSubjectKind::Value => "value".to_string(),
        OwnershipSubjectKind::Binder => "binder".to_string(),
        OwnershipSubjectKind::DynRowPackage => "dyn-row-package".to_string(),
        OwnershipSubjectKind::DynRowPayload { send } => {
            format!("dyn-row-payload send={}", render_send_color(send))
        }
        OwnershipSubjectKind::Continuation => "continuation".to_string(),
        OwnershipSubjectKind::SharedSendValue => "shared-send-value".to_string(),
    }
}

fn render_operator_intrinsic(intrinsic: OperatorIntrinsic) -> &'static str {
    match intrinsic {
        OperatorIntrinsic::I64Add => "i64-add",
        OperatorIntrinsic::I64Sub => "i64-sub",
        OperatorIntrinsic::I64Mul => "i64-mul",
        OperatorIntrinsic::I64Div => "i64-div",
    }
}

fn render_target_specific_core_term(term: TargetSpecificCoreTerm) -> &'static str {
    match term {
        TargetSpecificCoreTerm::Wasm => "wasm",
        TargetSpecificCoreTerm::Wat => "wat",
        TargetSpecificCoreTerm::Binaryen => "binaryen",
        TargetSpecificCoreTerm::Funcref => "funcref",
        TargetSpecificCoreTerm::Eqref => "eqref",
    }
}

fn render_alpha_facts(facts: &AlphaFacts) -> String {
    let mut out = String::new();
    writeln!(out, "root={}", render_alpha_expr_kind(&facts.expr.kind)).unwrap();
    writeln!(out, "params={}", facts.param_binders.len()).unwrap();
    for binder in &facts.param_binders {
        writeln!(out, "param {} name={}", binder.id, binder.name).unwrap();
    }
    writeln!(out, "binders={}", facts.binders.len()).unwrap();
    for binder in &facts.binders {
        let namespace = if binder.namespace.is_empty() {
            "<local>".to_string()
        } else {
            binder.namespace.join("::")
        };
        writeln!(
            out,
            "binder {} name={} namespace={}",
            binder.id, binder.name, namespace
        )
        .unwrap();
    }
    writeln!(out, "diagnostics={}", facts.diagnostics.len()).unwrap();
    for diagnostic in &facts.diagnostics {
        writeln!(out, "diagnostic {}", render_alpha_diagnostic(diagnostic)).unwrap();
    }
    out
}

fn render_alpha_expr_kind(kind: &AlphaExprKind) -> &'static str {
    match kind {
        AlphaExprKind::Var(_) => "var",
        AlphaExprKind::Lit(_) => "literal",
        AlphaExprKind::Lambda { .. } => "lambda",
        AlphaExprKind::Call { .. } => "call",
        AlphaExprKind::Tuple(_) => "tuple",
        AlphaExprKind::SliceLiteral(_) => "slice-literal",
        AlphaExprKind::Record(_) => "record",
        AlphaExprKind::RecordUpdate { .. } => "record-update",
        AlphaExprKind::AdtCtor { .. } => "adt-ctor",
        AlphaExprKind::Field { .. } => "field",
        AlphaExprKind::MethodCall { .. } => "method-call",
        AlphaExprKind::Index { .. } => "index",
        AlphaExprKind::Range { .. } => "range",
        AlphaExprKind::Binary { .. } => "binary",
        AlphaExprKind::If { .. } => "if",
        AlphaExprKind::IfLet { .. } => "if-let",
        AlphaExprKind::Match { .. } => "match",
        AlphaExprKind::Nominal { .. } => "nominal",
        AlphaExprKind::Reset { .. } => "reset",
        AlphaExprKind::Shift { .. } => "shift",
    }
}

fn render_alpha_diagnostic(diagnostic: &AlphaDiagnostic) -> String {
    match diagnostic {
        AlphaDiagnostic::UndefinedVar { name } => format!("undefined-var {name}"),
    }
}

fn render_core_diagnostic(diagnostic: &CoreDiagnostic) -> String {
    match diagnostic {
        CoreDiagnostic::TargetSpecificTerm { term } => {
            format!("target-specific core term {term}")
        }
        CoreDiagnostic::LayoutHashMismatch {
            key,
            expected,
            actual,
        } => format!("layout hash mismatch {key}: expected {expected}, actual {actual}"),
        CoreDiagnostic::DuplicateLayoutKey { key } => format!("duplicate layout key {key}"),
        CoreDiagnostic::MissingContNPackage { binder } => {
            format!("missing contn package {binder}")
        }
        CoreDiagnostic::UnsafeContNReplayCapture { binder } => {
            format!("unsafe contn replay capture {binder}")
        }
        CoreDiagnostic::Cont1HasPackageLayout { key } => {
            format!("cont1 has package layout {key}")
        }
        CoreDiagnostic::MissingDynRowLayout { layout } => {
            format!("missing dyn row layout {layout}")
        }
        CoreDiagnostic::DynRowLayoutKindMismatch { layout } => {
            format!("dyn row layout kind mismatch {layout}")
        }
        CoreDiagnostic::MissingStaticRowLayout { layout } => {
            format!("missing static row layout {layout}")
        }
        CoreDiagnostic::StaticRowLayoutKindMismatch { layout } => {
            format!("static row layout kind mismatch {layout}")
        }
        CoreDiagnostic::MissingTupleLayout { layout } => {
            format!("missing tuple layout {layout}")
        }
        CoreDiagnostic::TupleLayoutKindMismatch { layout } => {
            format!("tuple layout kind mismatch {layout}")
        }
        CoreDiagnostic::TupleFieldMissing { layout, field } => {
            format!("tuple field missing {layout}.{field}")
        }
        CoreDiagnostic::MissingRecordLayout { layout } => {
            format!("missing record layout {layout}")
        }
        CoreDiagnostic::RecordLayoutKindMismatch { layout } => {
            format!("record layout kind mismatch {layout}")
        }
        CoreDiagnostic::RecordFieldMissing { layout, field } => {
            format!("record field missing {layout}.{field}")
        }
        CoreDiagnostic::DanglingTailCallTarget { target } => {
            format!("dangling tail-call target {target}")
        }
        CoreDiagnostic::EnvClosureMissingLayout { subject } => {
            format!("env closure missing layout {subject}")
        }
        CoreDiagnostic::ClosureEnvLayoutHasNoFields { layout } => {
            format!("closure env layout has no fields {layout}")
        }
        CoreDiagnostic::SendableCallableContainsContinuation { subject } => {
            format!("sendable callable contains continuation {subject}")
        }
        CoreDiagnostic::Cont1CallableUsedMoreThanOnce { subject } => {
            format!("cont1 callable used more than once {subject}")
        }
        CoreDiagnostic::SharedSendSubjectUsesRc { subject } => {
            format!("shared send subject uses rc {subject}")
        }
        CoreDiagnostic::DynPayloadMustUseDynPackage { subject } => {
            format!("dyn payload must use dyn package {subject}")
        }
        CoreDiagnostic::DuplicateLiftedFunctionSymbol { symbol } => {
            format!("duplicate lifted function symbol {symbol}")
        }
        CoreDiagnostic::LiftedFunctionMissingClosureEnv {
            source,
            expected_layout,
        } => format!("lifted function missing closure env {source}: {expected_layout}"),
        CoreDiagnostic::CompilerIntrinsicOwnerMismatch {
            intrinsic,
            owner_namespace,
            expected_owner_namespace,
        } => format!(
            "compiler intrinsic owner mismatch {}: {owner_namespace} vs {expected_owner_namespace}",
            render_compiler_intrinsic(*intrinsic)
        ),
    }
}

fn render_compiler_intrinsic(intrinsic: CompilerIntrinsic) -> &'static str {
    match intrinsic {
        CompilerIntrinsic::TupleToAdt => "tuple_to_adt",
        CompilerIntrinsic::AdtToTuple => "adt_to_tuple",
    }
}

fn render_backend_artifact(artifact: &BackendArtifact) -> String {
    let mut out = String::new();
    writeln!(out, "target={}", render_backend_target(artifact.target)).unwrap();
    writeln!(out, "wat-lines={}", artifact.wat.lines().count()).unwrap();
    writeln!(
        out,
        "return={}",
        artifact
            .return_value
            .as_ref()
            .map(render_core_value_summary)
            .unwrap_or_else(|| "none".to_string())
    )
    .unwrap();
    render_backend_manifest(&mut out, &artifact.manifest);
    writeln!(out, "diagnostics={}", artifact.diagnostics.len()).unwrap();
    for diagnostic in &artifact.diagnostics {
        writeln!(out, "diagnostic {}", render_backend_diagnostic(diagnostic)).unwrap();
    }
    out
}

fn render_backend_link(bundle: &BackendLinkedBundle) -> String {
    let mut out = String::new();
    writeln!(out, "target={}", render_backend_target(bundle.target)).unwrap();
    writeln!(
        out,
        "linked-wat-lines={}",
        bundle.linked_wat.lines().count()
    )
    .unwrap();
    render_backend_manifest(&mut out, &bundle.manifest);
    writeln!(out, "diagnostics={}", bundle.diagnostics.len()).unwrap();
    for diagnostic in &bundle.diagnostics {
        writeln!(
            out,
            "diagnostic {}",
            render_backend_link_diagnostic(diagnostic)
        )
        .unwrap();
    }
    out
}

fn render_backend_cache_key(key: &BackendCacheKey) -> String {
    let mut out = String::new();
    writeln!(out, "target={}", render_backend_target(key.target)).unwrap();
    writeln!(out, "digest={}", key.digest).unwrap();
    out
}

fn render_core_value_summary(value: &crate::core::CoreValue) -> String {
    match value {
        crate::core::CoreValue::Unit => "unit".to_string(),
        crate::core::CoreValue::I64(value) => format!("i64({value})"),
        crate::core::CoreValue::Bool(value) => format!("bool({value})"),
        crate::core::CoreValue::Var(name) => format!("var({name})"),
        crate::core::CoreValue::Tuple { fields } => {
            format!("tuple/{}", fields.len())
        }
        crate::core::CoreValue::TupleField {
            tuple,
            field,
            field_index,
        } => format!(
            "tuple-field {}.{}#{}",
            render_core_value_summary(tuple),
            field,
            field_index
        ),
        crate::core::CoreValue::Range { start, end } => format!(
            "range({}..{})",
            render_core_value_summary(start),
            render_core_value_summary(end)
        ),
        crate::core::CoreValue::SliceLiteral { items } => {
            format!("slice/{}", items.len())
        }
        crate::core::CoreValue::Record { fields } => {
            format!("record/{}", fields.len())
        }
        crate::core::CoreValue::RecordUpdate { base, fields } => {
            format!(
                "record-update {} +{}",
                render_core_value_summary(base),
                fields.len()
            )
        }
        crate::core::CoreValue::RecordField { record, field } => {
            format!("record-field {}.{field}", render_core_value_summary(record))
        }
        crate::core::CoreValue::RangeField { range, field } => {
            format!(
                "range-field {}.{}",
                render_core_value_summary(range),
                field.source_name()
            )
        }
        crate::core::CoreValue::AggregateField {
            kind: _,
            value,
            field,
        } => {
            format!(
                "slice-field {}.{}",
                render_core_value_summary(value),
                field.source_name()
            )
        }
        crate::core::CoreValue::TextField { kind, value, field } => {
            format!(
                "{}-field {}.{}",
                kind.source_name(),
                render_core_value_summary(value),
                field.source_name()
            )
        }
        crate::core::CoreValue::AggregateIndex {
            kind: _,
            value,
            index,
        } => {
            format!(
                "slice-index {}[{}]",
                render_core_value_summary(value),
                render_core_value_summary(index)
            )
        }
        crate::core::CoreValue::TextIndex { kind, value, index } => {
            format!(
                "{}-index {}[{}]",
                kind.source_name(),
                render_core_value_summary(value),
                render_core_value_summary(index)
            )
        }
        crate::core::CoreValue::VecRuntimeCall { call, args } => {
            format!("{} /{}", call.debug_name(), args.len())
        }
        crate::core::CoreValue::Adt {
            data, ctor, args, ..
        } => {
            format!("adt {data}.{ctor}/{}", args.len())
        }
        crate::core::CoreValue::Rendered { debug } => format!("rendered {debug}"),
    }
}

fn render_backend_manifest(out: &mut String, manifest: &BackendManifest) {
    writeln!(out, "manifest-imports={}", manifest.imports.len()).unwrap();
    for import in &manifest.imports {
        writeln!(
            out,
            "import {} symbol={} module={} name={} signature={}",
            render_backend_extern_abi(import.abi),
            import.final_symbol,
            import.module,
            import.name,
            import.signature_hash
        )
        .unwrap();
    }
    writeln!(out, "manifest-entries={}", manifest.entries.len()).unwrap();
    for entry in &manifest.entries {
        writeln!(
            out,
            "symbol {} source={} origin={} ownership={}",
            entry.final_symbol,
            entry.source_debug_name,
            entry.pass_origin,
            entry
                .ownership
                .map(render_ownership_decision)
                .unwrap_or_else(|| "none".to_string())
        )
        .unwrap();
    }
}

fn render_backend_extern_abi(abi: BackendExternAbi) -> &'static str {
    match abi {
        BackendExternAbi::Wasi => "wasi",
        BackendExternAbi::C => "c",
    }
}

fn render_core_extern_abi(abi: crate::core::CoreExternAbi) -> &'static str {
    match abi {
        crate::core::CoreExternAbi::Wasi => "wasi",
        crate::core::CoreExternAbi::C => "c",
    }
}

fn render_backend_diagnostic(diagnostic: &BackendDiagnostic) -> String {
    match diagnostic {
        BackendDiagnostic::CoreValidationFailed { diagnostics } => {
            format!("core validation failed diagnostics={diagnostics}")
        }
        BackendDiagnostic::UnsupportedI32ReturnValue { value } => {
            format!("unsupported i32 return value {value}")
        }
        BackendDiagnostic::UnsupportedExternImportSignature { symbol, signature } => {
            format!("unsupported extern import signature {symbol} {signature}")
        }
        BackendDiagnostic::Cont1ResumedMoreThanOnce { binder } => {
            format!("cont1 resumed more than once {binder}")
        }
        BackendDiagnostic::UnsupportedContinuationRuntime { op, kind, binder } => {
            format!(
                "unsupported continuation runtime {op} kind={} binder={}",
                render_continuation_kind(*kind),
                binder.as_deref().unwrap_or("none")
            )
        }
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

fn render_backend_target(target: BackendTarget) -> &'static str {
    match target {
        BackendTarget::WasmGc => "wasm-gc",
    }
}

fn render_ownership_decision(decision: OwnershipDecision) -> String {
    match decision {
        OwnershipDecision::StackValue => "stack-value",
        OwnershipDecision::InplaceReuse => "inplace-reuse",
        OwnershipDecision::Rc => "rc",
        OwnershipDecision::Arc => "arc",
        OwnershipDecision::StaticData => "static-data",
        OwnershipDecision::BorrowedView => "borrowed-view",
        OwnershipDecision::DynPackage => "dyn-package",
    }
    .to_string()
}

fn render_cps_usage_facts(facts: &CpsUsageFacts) -> String {
    let mut out = String::new();
    writeln!(out, "continuations={}", facts.continuations.len()).unwrap();
    for (binder, usage) in &facts.continuations {
        writeln!(
            out,
            "continuation {binder} kind={} count={} materialization={}",
            render_continuation_kind(usage.kind),
            render_use_count(usage.count),
            render_continuation_materialization(usage.materialization)
        )
        .unwrap();
    }
    writeln!(out, "function-lambdas={}", facts.function_lambdas.len()).unwrap();
    for (name, count) in &facts.function_lambdas {
        writeln!(
            out,
            "function-lambda {name} count={}",
            render_use_count(*count)
        )
        .unwrap();
    }
    writeln!(
        out,
        "continuation-lambdas={}",
        facts.continuation_lambdas.len()
    )
    .unwrap();
    for (name, count) in &facts.continuation_lambdas {
        writeln!(
            out,
            "continuation-lambda {name} count={}",
            render_use_count(*count)
        )
        .unwrap();
    }
    writeln!(out, "diagnostics={}", facts.diagnostics.len()).unwrap();
    for diagnostic in &facts.diagnostics {
        writeln!(
            out,
            "diagnostic {}",
            render_cps_usage_diagnostic(diagnostic)
        )
        .unwrap();
    }
    out
}

fn render_usage_facts(facts: &UsageFacts) -> String {
    let mut out = String::new();
    writeln!(out, "vars={}", facts.vars.len()).unwrap();
    for (name, count) in &facts.vars {
        writeln!(
            out,
            "var {name} count={} color={}",
            render_use_count(*count),
            render_usage_color(count.color())
        )
        .unwrap();
    }
    writeln!(out, "binders={}", facts.binders.len()).unwrap();
    for (binder, count) in &facts.binders {
        writeln!(
            out,
            "binder {binder} count={} color={}",
            render_use_count(*count),
            render_usage_color(count.color())
        )
        .unwrap();
    }
    out
}

fn render_control_facts(facts: &ControlFacts) -> String {
    let mut out = String::new();
    writeln!(out, "continuations={}", facts.continuations.len()).unwrap();
    for continuation in &facts.continuations {
        writeln!(
            out,
            "continuation {} kind={} input={} answer={} usage={} replay={}",
            continuation.binder,
            render_continuation_kind(continuation.kind),
            render_type(&continuation.input),
            render_type(&continuation.answer),
            render_usage_color(continuation.usage),
            render_replay_safety(continuation.replay_safety)
        )
        .unwrap();
    }
    writeln!(out, "errors={}", facts.errors.len()).unwrap();
    for error in &facts.errors {
        writeln!(out, "error {}", render_control_error(error)).unwrap();
    }
    out
}

fn render_typed_expr(expr: &TypedExpr) -> String {
    let mut out = String::new();
    render_typed_expr_into(expr, 0, &mut out);
    out
}

fn render_typed_expr_into(expr: &TypedExpr, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    writeln!(
        out,
        "{indent}node kind={} type={} usage={} send={}",
        render_typed_expr_kind(&expr.kind),
        render_type(&expr.ty),
        render_usage_color(expr.usage),
        render_send_color(expr.send)
    )
    .unwrap();
    match &expr.kind {
        TypedExprKind::Var(name) => {
            writeln!(out, "{indent}  var {name}").unwrap();
        }
        TypedExprKind::Lit(literal) => {
            writeln!(out, "{indent}  literal {}", render_source_literal(literal)).unwrap();
        }
        TypedExprKind::Lambda {
            param,
            param_ty,
            body,
        } => {
            writeln!(out, "{indent}  param {param}: {}", render_type(param_ty)).unwrap();
            render_typed_child("body", body, depth, out);
        }
        TypedExprKind::Call { callee, args } => {
            render_typed_child("callee", callee, depth, out);
            for (index, arg) in args.iter().enumerate() {
                render_typed_child(&format!("arg {index}"), arg, depth, out);
            }
        }
        TypedExprKind::Tuple { fields, nominal } => {
            writeln!(out, "{indent}  nominal {nominal}").unwrap();
            for (index, field) in fields.iter().enumerate() {
                render_typed_child(&format!("field {index}"), field, depth, out);
            }
        }
        TypedExprKind::SliceLiteral { items, element } => {
            writeln!(out, "{indent}  element {}", render_type(element)).unwrap();
            for (index, item) in items.iter().enumerate() {
                render_typed_child(&format!("item {index}"), item, depth, out);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                render_typed_child(&format!("field {}", field.name), &field.value, depth, out);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            render_typed_child("base", base, depth, out);
            for field in fields {
                render_typed_child(&format!("update {}", field.name), &field.value, depth, out);
            }
        }
        TypedExprKind::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            writeln!(
                out,
                "{indent}  constructor {data}.{ctor} variants=[{}]",
                variants.join(", ")
            )
            .unwrap();
            for (index, arg) in args.iter().enumerate() {
                render_typed_child(&format!("arg {index}"), arg, depth, out);
            }
        }
        TypedExprKind::Field {
            receiver,
            name,
            access,
        } => {
            writeln!(
                out,
                "{indent}  field {name} access={}",
                render_field_access_kind(access)
            )
            .unwrap();
            render_typed_child("receiver", receiver, depth, out);
        }
        TypedExprKind::MethodCall {
            receiver,
            name,
            args,
            builtin,
        } => {
            if let Some(builtin) = builtin {
                writeln!(
                    out,
                    "{indent}  method {name} builtin={}",
                    builtin.debug_name()
                )
                .unwrap();
            } else {
                writeln!(out, "{indent}  method {name}").unwrap();
            }
            render_typed_child("receiver", receiver, depth, out);
            for (index, arg) in args.iter().enumerate() {
                render_typed_child(&format!("arg {index}"), arg, depth, out);
            }
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            render_typed_child("receiver", receiver, depth, out);
            render_typed_child("index", index, depth, out);
        }
        TypedExprKind::Range { start, end } => {
            render_typed_child("start", start, depth, out);
            render_typed_child("end", end, depth, out);
        }
        TypedExprKind::Binary { op, lhs, rhs } => {
            writeln!(out, "{indent}  op {}", render_source_binary_op(*op)).unwrap();
            render_typed_child("lhs", lhs, depth, out);
            render_typed_child("rhs", rhs, depth, out);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            render_typed_child("cond", cond, depth, out);
            render_typed_child("then", then_branch, depth, out);
            render_typed_child("else", else_branch, depth, out);
        }
        TypedExprKind::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            writeln!(out, "{indent}  pattern {}", render_source_pattern(pattern)).unwrap();
            render_typed_child("scrutinee", scrutinee, depth, out);
            render_typed_child("then", then_branch, depth, out);
            render_typed_child("else", else_branch, depth, out);
        }
        TypedExprKind::Match { scrutinee, arms } => {
            render_typed_child("scrutinee", scrutinee, depth, out);
            for (index, arm) in arms.iter().enumerate() {
                writeln!(
                    out,
                    "{indent}  arm {index} pattern {}",
                    render_source_pattern(&arm.pattern)
                )
                .unwrap();
                render_typed_child("body", &arm.body, depth + 1, out);
            }
        }
        TypedExprKind::Nominal { name, expr } => {
            writeln!(out, "{indent}  nominal {name}").unwrap();
            render_typed_child("expr", expr, depth, out);
        }
        TypedExprKind::Reset { multi, body } => {
            let kind = if *multi { "contn" } else { "cont1" };
            writeln!(out, "{indent}  reset-kind {kind}").unwrap();
            render_typed_child("body", body, depth, out);
        }
        TypedExprKind::Shift { binder, body } => {
            writeln!(out, "{indent}  binder {binder}").unwrap();
            render_typed_child("body", body, depth, out);
        }
    }
}

fn render_typed_child(label: &str, expr: &TypedExpr, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    writeln!(out, "{indent}  {label}:").unwrap();
    render_typed_expr_into(expr, depth + 2, out);
}

fn render_typed_expr_kind(kind: &TypedExprKind) -> &'static str {
    match kind {
        TypedExprKind::Var(_) => "var",
        TypedExprKind::Lit(_) => "literal",
        TypedExprKind::Lambda { .. } => "lambda",
        TypedExprKind::Call { .. } => "call",
        TypedExprKind::Tuple { .. } => "tuple",
        TypedExprKind::SliceLiteral { .. } => "slice-literal",
        TypedExprKind::Record { .. } => "record",
        TypedExprKind::RecordUpdate { .. } => "record-update",
        TypedExprKind::AdtCtor { .. } => "adt-ctor",
        TypedExprKind::Field { .. } => "field",
        TypedExprKind::MethodCall { .. } => "method-call",
        TypedExprKind::Index { .. } => "index",
        TypedExprKind::Range { .. } => "range",
        TypedExprKind::Binary { .. } => "binary",
        TypedExprKind::If { .. } => "if",
        TypedExprKind::IfLet { .. } => "if-let",
        TypedExprKind::Match { .. } => "match",
        TypedExprKind::Nominal { .. } => "nominal",
        TypedExprKind::Reset { .. } => "reset",
        TypedExprKind::Shift { .. } => "shift",
    }
}

fn render_field_access_kind(access: &FieldAccessKind) -> String {
    match access {
        FieldAccessKind::RecordOrNominal => "record-or-nominal".to_string(),
        FieldAccessKind::TuplePositionalRow { index } => {
            format!("tuple-positional-row index={index}")
        }
        FieldAccessKind::RangeBoundary { boundary } => {
            format!("range-boundary {}", boundary.source_name())
        }
        FieldAccessKind::AggregateBoundary { kind, boundary } => {
            format!("{}-boundary {}", kind.source_name(), boundary.source_name())
        }
        FieldAccessKind::TextBoundary { kind, boundary } => {
            format!("{}-boundary {}", kind.source_name(), boundary.source_name())
        }
    }
}

fn render_pattern_facts(facts: &PatternFacts) -> String {
    let mut out = String::new();
    writeln!(out, "matches={}", facts.matches.len()).unwrap();
    for (index, fact) in facts.matches.iter().enumerate() {
        let literals = fact
            .covered_literals
            .iter()
            .map(render_source_literal)
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            out,
            "match {index} scrutinee={} literals=[{}] constructors=[{}] wildcard={} exhaustive={}",
            render_type(&fact.scrutinee_type),
            literals,
            fact.covered_constructors.join(", "),
            fact.has_wildcard,
            fact.exhaustive
        )
        .unwrap();
    }
    writeln!(out, "envs={}", facts.envs.len()).unwrap();
    for (index, env) in facts.envs.iter().enumerate() {
        writeln!(
            out,
            "env {index} bindings=[{}] success-binds={} failure-binds={}",
            env.bindings.join(", "),
            env.success_branch_binds,
            env.failure_branch_binds
        )
        .unwrap();
    }
    writeln!(out, "diagnostics={}", facts.diagnostics.len()).unwrap();
    for diagnostic in &facts.diagnostics {
        writeln!(out, "diagnostic {}", render_pattern_diagnostic(diagnostic)).unwrap();
    }
    out
}

fn render_pattern_diagnostic(diagnostic: &PatternDiagnostic) -> String {
    match diagnostic {
        PatternDiagnostic::NonExhaustiveMatch {
            scrutinee_type,
            missing,
        } => {
            let missing = missing
                .iter()
                .map(render_source_pattern)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "non-exhaustive-match scrutinee={} missing=[{}]",
                render_type(scrutinee_type),
                missing
            )
        }
        PatternDiagnostic::DuplicateBinding { name } => {
            format!("duplicate-binding {name}")
        }
        PatternDiagnostic::ChainedAtPattern { name } => {
            format!("chained-at-pattern {name}")
        }
    }
}

fn render_control_error(error: &ControlError) -> String {
    match error {
        ControlError::ShiftOutsideReset { binder } => format!("shift outside reset {binder}"),
        ControlError::UnsafeMultiResumeCapture { binder } => {
            format!("unsafe multi-resume capture {binder}")
        }
    }
}

fn render_replay_safety(replay: ReplaySafety) -> &'static str {
    match replay {
        ReplaySafety::Safe => "safe",
        ReplaySafety::Unsafe => "unsafe",
        ReplaySafety::RollbackRegion => "rollback-region",
    }
}

fn render_type(ty: &Type) -> String {
    match ty {
        Type::Unknown => "_".to_string(),
        Type::I64 => "i64".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Tuple(fields) => {
            let fields = fields
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ");
            format!("Tuple[{fields}]")
        }
        Type::Record(fields) => render_record_type(fields),
        Type::Adt { name, .. } | Type::Nominal(name) => name.clone(),
        Type::Func(param, result) => {
            format!("({}) -> {}", render_type(param), render_type(result))
        }
        Type::Continuation {
            multi,
            input,
            answer,
        } => {
            let name = if *multi { "ContN" } else { "Cont1" };
            format!("{name}[{}, {}]", render_type(input), render_type(answer))
        }
    }
}

fn render_record_type(fields: &[RecordTypeField]) -> String {
    let fields = fields
        .iter()
        .map(|field| format!("{}: {}", field.name, render_type(&field.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{fields}}}")
}

fn render_continuation_simplification(facts: &ContinuationSimplificationFacts) -> String {
    let mut out = String::new();
    writeln!(out, "decisions={}", facts.decisions.len()).unwrap();
    for (binder, decision) in &facts.decisions {
        writeln!(
            out,
            "decision {binder} materialization={}",
            render_continuation_materialization(*decision)
        )
        .unwrap();
    }
    out
}

fn render_closure_facts(facts: &ClosureFacts) -> String {
    let mut out = String::new();
    writeln!(out, "closures={}", facts.closures.len()).unwrap();
    for (index, closure) in facts.closures.iter().enumerate() {
        writeln!(
            out,
            "closure {index} param={} storage={} captures={}",
            closure.param,
            render_closure_storage_kind(closure.storage),
            closure.captures.len()
        )
        .unwrap();
        for capture in &closure.captures {
            let binder = capture
                .binder
                .map(|binder| binder.to_string())
                .unwrap_or_else(|| "none".to_string());
            writeln!(
                out,
                "closure {index} capture {} binder={} usage={}",
                capture.name,
                binder,
                render_usage_color(capture.usage)
            )
            .unwrap();
        }
    }
    out
}

fn render_closure_storage_kind(kind: ClosureStorageKind) -> &'static str {
    match kind {
        ClosureStorageKind::DirectNoCapture => "direct-no-capture",
        ClosureStorageKind::EnvClosure => "env-closure",
    }
}

fn render_closure_core_usage(facts: &ClosureCoreUsageFacts) -> String {
    let mut out = String::new();
    writeln!(out, "closure-packages={}", facts.closure_packages.len()).unwrap();
    for (subject, package) in &facts.closure_packages {
        writeln!(
            out,
            "closure-package {subject} storage={} count={} send={}",
            render_callable_storage_kind(package.storage),
            render_use_count(package.count),
            render_send_color(package.send)
        )
        .unwrap();
    }
    writeln!(out, "env-fields={}", facts.env_fields.len()).unwrap();
    for (key, field) in &facts.env_fields {
        writeln!(
            out,
            "env-field {key} closure={} field={} usage={} send={}",
            field.closure,
            field.field,
            render_usage_color(field.color),
            render_send_color(field.send)
        )
        .unwrap();
    }
    writeln!(out, "code-pointers={}", facts.code_pointers.len()).unwrap();
    for (symbol, pointer) in &facts.code_pointers {
        writeln!(
            out,
            "code-pointer {symbol} source={} count={} direct={}",
            pointer.source,
            render_use_count(pointer.count),
            pointer.known_direct
        )
        .unwrap();
    }
    writeln!(
        out,
        "continuation-packages={}",
        facts.continuation_packages.len()
    )
    .unwrap();
    for (package, usage) in &facts.continuation_packages {
        writeln!(
            out,
            "continuation-package {package} kind={} count={}",
            render_continuation_kind(usage.kind),
            render_use_count(usage.count)
        )
        .unwrap();
    }
    out
}

fn render_callable_storage_kind(kind: CallableStorageKind) -> &'static str {
    match kind {
        CallableStorageKind::DirectFn => "direct-fn",
        CallableStorageKind::NoCaptureClosure => "no-capture-closure",
        CallableStorageKind::EnvClosure => "env-closure",
        CallableStorageKind::BoxedCont1 => "boxed-cont1",
        CallableStorageKind::ContNPackage => "contn-package",
        CallableStorageKind::ErasedCallableAdt => "erased-callable-adt",
    }
}

pub fn render_callable_storage_facts(facts: &[CallableStorageFact]) -> String {
    let mut out = String::new();
    writeln!(out, "entries={}", facts.len()).unwrap();
    render_callable_storage_lines(&mut out, facts);
    out
}

fn render_callable_storage_lines(out: &mut String, facts: &[CallableStorageFact]) {
    for fact in facts {
        writeln!(
            out,
            "callable {} kind={} usage={} send={}",
            fact.subject,
            render_callable_storage_kind(fact.kind),
            render_usage_color(fact.usage),
            render_send_color(fact.send)
        )
        .unwrap();
    }
}

fn render_closure_simplification(facts: &ClosureSimplificationFacts) -> String {
    let mut out = String::new();
    writeln!(out, "closure-packages={}", facts.closure_packages.len()).unwrap();
    for (subject, decision) in &facts.closure_packages {
        writeln!(
            out,
            "closure-package {subject} decision={}",
            render_closure_package_decision(*decision)
        )
        .unwrap();
    }
    writeln!(out, "env-fields={}", facts.env_fields.len()).unwrap();
    for (field, decision) in &facts.env_fields {
        writeln!(
            out,
            "env-field {field} decision={}",
            render_env_field_decision(*decision)
        )
        .unwrap();
    }
    writeln!(out, "code-pointers={}", facts.code_pointers.len()).unwrap();
    for (symbol, decision) in &facts.code_pointers {
        writeln!(
            out,
            "code-pointer {symbol} decision={}",
            render_code_pointer_decision(*decision)
        )
        .unwrap();
    }
    writeln!(
        out,
        "continuation-packages={}",
        facts.continuation_packages.len()
    )
    .unwrap();
    for (package, decision) in &facts.continuation_packages {
        writeln!(
            out,
            "continuation-package {package} decision={}",
            render_continuation_package_decision(*decision)
        )
        .unwrap();
    }
    out
}

fn render_lambda_lift_facts(facts: &LambdaLiftFacts) -> String {
    let mut out = String::new();
    writeln!(out, "functions={}", facts.functions.len()).unwrap();
    for (index, function) in facts.functions.iter().enumerate() {
        writeln!(
            out,
            "function {index} source={} symbol={} env=[{}] direct={}",
            function.source,
            function.symbol,
            function.env_params.join(", "),
            function.direct
        )
        .unwrap();
    }
    out
}

fn render_closure_package_decision(decision: ClosurePackageDecision) -> &'static str {
    match decision {
        ClosurePackageDecision::EraseNoCapture => "erase-no-capture",
        ClosurePackageDecision::DirectifySingleUse => "directify-single-use",
        ClosurePackageDecision::KeepEnvPackage => "keep-env-package",
    }
}

fn render_env_field_decision(decision: EnvFieldDecision) -> &'static str {
    match decision {
        EnvFieldDecision::RemoveDeadField => "remove-dead-field",
        EnvFieldDecision::KeepField => "keep-field",
    }
}

fn render_code_pointer_decision(decision: CodePointerDecision) -> &'static str {
    match decision {
        CodePointerDecision::KnownDirectCall => "known-direct-call",
        CodePointerDecision::KeepIndirectCodePointer => "keep-indirect-code-pointer",
    }
}

fn render_continuation_package_decision(decision: ContinuationPackageDecision) -> &'static str {
    match decision {
        ContinuationPackageDecision::RemoveUnusedPackage => "remove-unused-package",
        ContinuationPackageDecision::KeepRepeatablePackage => "keep-repeatable-package",
    }
}

fn render_usage_audit(report: &UsageAuditReport) -> String {
    let mut out = String::new();
    writeln!(out, "entries={}", report.entries.len()).unwrap();
    for entry in &report.entries {
        writeln!(
            out,
            "entry {} source={} typed={} usage={} rust-ref={} rust-ownership={} ownership={} aligned={}",
            entry.subject,
            entry.source_signature,
            entry.typed_signature,
            entry.usage_signature,
            entry.rust_reference_signature,
            render_rust_reference_ownership(entry.rust_reference_ownership),
            entry
                .ownership
                .map(render_ownership_decision)
                .unwrap_or_else(|| "none".to_string()),
            entry.aligned
        )
        .unwrap();
    }
    writeln!(out, "diagnostics={}", report.diagnostics.len()).unwrap();
    for diagnostic in &report.diagnostics {
        writeln!(
            out,
            "diagnostic {}",
            render_usage_audit_diagnostic(diagnostic)
        )
        .unwrap();
    }
    out
}

fn render_rust_reference_ownership(ownership: RustReferenceOwnership) -> &'static str {
    match ownership {
        RustReferenceOwnership::MoveOnly => "move-only",
        RustReferenceOwnership::SharedRc => "shared-rc",
        RustReferenceOwnership::Obligation => "obligation",
    }
}

fn render_usage_audit_diagnostic(diagnostic: &UsageAuditDiagnostic) -> String {
    match diagnostic {
        UsageAuditDiagnostic::RcRequiresManyUsage { subject, usage } => {
            format!(
                "rc-requires-many-usage {subject} usage={}",
                render_usage_color(*usage)
            )
        }
        UsageAuditDiagnostic::ManyUsageNeedsSharedRustReference {
            subject,
            rust_reference_signature,
        } => format!(
            "many-usage-needs-shared-rust-reference {subject} rust-ref={rust_reference_signature}"
        ),
    }
}

fn render_std_audit(report: &StdAuditReport) -> String {
    let mut out = String::new();
    writeln!(out, "requirements={}", report.requirements.len()).unwrap();
    for requirement in &report.requirements {
        writeln!(
            out,
            "requirement {} classification={} rust={} chiba={} reason={}",
            render_std_capability(requirement.capability),
            render_std_classification(requirement.classification),
            requirement.rust_surface,
            requirement.chiba_surface,
            requirement.reason
        )
        .unwrap();
    }
    writeln!(out, "diagnostics={}", report.diagnostics.len()).unwrap();
    for diagnostic in &report.diagnostics {
        writeln!(
            out,
            "diagnostic {}",
            render_std_audit_diagnostic(diagnostic)
        )
        .unwrap();
    }
    out
}

fn render_std_capability(capability: StdCapability) -> &'static str {
    match capability {
        StdCapability::GrowableSequence => "growable-sequence",
        StdCapability::OrderedMap => "ordered-map",
        StdCapability::OrderedSet => "ordered-set",
        StdCapability::StringBuilder => "string-builder",
        StdCapability::Formatting => "formatting",
        StdCapability::StableHash => "stable-hash",
        StdCapability::Utf8Chars => "utf8-chars",
        StdCapability::RcSharedReference => "rc-shared-reference",
        StdCapability::TimeMeasurement => "time-measurement",
        StdCapability::FileRead => "file-read",
        StdCapability::PathJoin => "path-join",
    }
}

fn render_std_classification(classification: StdClassification) -> &'static str {
    match classification {
        StdClassification::ChibaStdFirstBatch => "chiba-std-first-batch",
        StdClassification::RustImplementationConvenience => "rust-implementation-convenience",
        StdClassification::ChibaUnsafeOrMetalBoundary => "chiba-unsafe-or-metal-boundary",
        StdClassification::CompilerIntrinsic => "compiler-intrinsic",
    }
}

fn render_std_audit_diagnostic(diagnostic: &StdAuditDiagnostic) -> String {
    match diagnostic {
        StdAuditDiagnostic::MissingClassification { capability } => {
            format!(
                "missing-classification {}",
                render_std_capability(*capability)
            )
        }
        StdAuditDiagnostic::EmptyChibaSurface { capability } => {
            format!("empty-chiba-surface {}", render_std_capability(*capability))
        }
    }
}

fn render_cps_usage_diagnostic(diagnostic: &CpsUsageDiagnostic) -> String {
    match diagnostic {
        CpsUsageDiagnostic::Cont1ResumedMoreThanOnce { binder, count } => format!(
            "cont1 resumed more than once {binder} count={}",
            render_use_count(*count)
        ),
    }
}

fn render_continuation_materialization(
    materialization: ContinuationMaterialization,
) -> &'static str {
    match materialization {
        ContinuationMaterialization::Dead => "dead",
        ContinuationMaterialization::InlineSingleUse => "inline-single-use",
        ContinuationMaterialization::OneShotStateMachine => "one-shot-state-machine",
        ContinuationMaterialization::MultiResumePackage => "multi-resume-package",
    }
}

fn render_use_count(count: UseCount) -> &'static str {
    match count {
        UseCount::Zero => "0",
        UseCount::One => "1",
        UseCount::Many => "many",
    }
}

fn render_symbol_lineage(
    resolve: &ResolveFacts,
    template: &TemplateFacts,
    specialize: &SpecializationFacts,
) -> String {
    let mut out = String::new();
    for name in &resolve.resolved_names {
        match name {
            ResolvedName::Function { name, symbol } => {
                writeln!(out, "resolve function {name} -> {symbol}").unwrap();
            }
            ResolvedName::Static { name, symbol } => {
                writeln!(out, "resolve static {name} -> {symbol}").unwrap();
            }
            ResolvedName::Constructor {
                data,
                ctor,
                symbol,
                arity,
            } => {
                writeln!(out, "resolve constructor {data}.{ctor}/{arity} -> {symbol}").unwrap();
            }
        }
    }
    for obligation in &template.obligations {
        match obligation {
            TemplateObligation::Function { name, resolved } => {
                writeln!(out, "template function {name} -> {resolved}").unwrap();
            }
            TemplateObligation::Static { name, resolved } => {
                writeln!(out, "template static {name} -> {resolved}").unwrap();
            }
            TemplateObligation::Constructor {
                data,
                ctor,
                resolved,
                arity,
            } => {
                writeln!(
                    out,
                    "template constructor {data}.{ctor}/{arity} -> {resolved}"
                )
                .unwrap();
            }
            TemplateObligation::Method {
                name,
                resolved: Some(resolved),
                ..
            } => {
                writeln!(out, "template method {name} -> {resolved}").unwrap();
            }
            _ => {}
        }
    }
    for work_item in &specialize.work_items {
        for obligation in &work_item.obligations {
            match obligation {
                DischargedObligation::Function { name, target } => {
                    writeln!(out, "specialize function {name} -> {target}").unwrap();
                }
                DischargedObligation::Static { name, target } => {
                    writeln!(out, "specialize static {name} -> {target}").unwrap();
                }
                DischargedObligation::Constructor {
                    data,
                    ctor,
                    target,
                    arity,
                } => {
                    writeln!(
                        out,
                        "specialize constructor {data}.{ctor}/{arity} -> {target}"
                    )
                    .unwrap();
                }
                DischargedObligation::Method {
                    name,
                    target: Some(target),
                } => {
                    writeln!(out, "specialize method {name} -> {target}").unwrap();
                }
                _ => {}
            }
        }
    }
    if out.is_empty() {
        out.push_str("<empty>\n");
    }
    out
}

fn render_resolve_facts(resolve: &ResolveFacts) -> String {
    let mut out = String::new();
    writeln!(out, "names={}", resolve.resolved_names.len()).unwrap();
    for name in &resolve.resolved_names {
        writeln!(out, "name {}", render_resolved_name(name)).unwrap();
    }
    writeln!(out, "calls={}", resolve.resolved_calls.len()).unwrap();
    for call in &resolve.resolved_calls {
        writeln!(out, "call {}", render_resolved_call(call)).unwrap();
    }
    writeln!(out, "operators={}", resolve.operator_obligations.len()).unwrap();
    for obligation in &resolve.operator_obligations {
        writeln!(
            out,
            "operator {} protocol={} receiver={}",
            render_operator_surface(&obligation.op),
            obligation.protocol,
            obligation.receiver.as_deref().unwrap_or("<unknown>")
        )
        .unwrap();
    }
    writeln!(out, "diagnostics={}", resolve.diagnostics.len()).unwrap();
    for diagnostic in &resolve.diagnostics {
        writeln!(out, "diagnostic {}", render_resolve_diagnostic(diagnostic)).unwrap();
    }
    out
}

fn render_resolved_name(name: &ResolvedName) -> String {
    match name {
        ResolvedName::Function { name, symbol } => format!("function {name} -> {symbol}"),
        ResolvedName::Static { name, symbol } => format!("static {name} -> {symbol}"),
        ResolvedName::Constructor {
            data,
            ctor,
            symbol,
            arity,
        } => format!("constructor {data}.{ctor}/{arity} -> {symbol}"),
    }
}

fn render_resolved_call(call: &ResolvedCall) -> String {
    match call {
        ResolvedCall::FieldCallable { field } => format!("field-callable {field}"),
        ResolvedCall::ReceiverMethod {
            receiver,
            name,
            symbol,
        } => format!("receiver-method {receiver}.{name} -> {symbol}"),
        ResolvedCall::QualifiedCallee { path, symbol } => {
            format!("qualified-callee {path} -> {symbol}")
        }
    }
}

fn render_resolve_diagnostic(diagnostic: &ResolveDiagnostic) -> String {
    match diagnostic {
        ResolveDiagnostic::AmbiguousMethod {
            receiver,
            name,
            candidates,
        } => format!(
            "ambiguous method {receiver}.{name}: [{}]",
            candidates.join(", ")
        ),
        ResolveDiagnostic::MissingMethod { receiver, name } => format!(
            "missing method {}.{}",
            receiver.as_deref().unwrap_or("<unknown>"),
            name
        ),
        ResolveDiagnostic::AmbiguousName { name, candidates } => {
            format!("ambiguous name {name}: [{}]", candidates.join(", "))
        }
        ResolveDiagnostic::FunctionArityMismatch {
            name,
            symbol,
            expected,
            actual,
        } => format!(
            "function arity mismatch {name} -> {symbol}: expected {expected} actual {actual}"
        ),
        ResolveDiagnostic::AmbiguousConstructor {
            data,
            ctor,
            candidates,
        } => format!(
            "ambiguous constructor {data}.{ctor}: [{}]",
            candidates.join(", ")
        ),
        ResolveDiagnostic::MissingConstructor { data, ctor } => {
            format!("missing constructor {data}.{ctor}")
        }
        ResolveDiagnostic::ConstructorArityMismatch {
            data,
            ctor,
            expected,
            actual,
        } => {
            format!("constructor arity mismatch {data}.{ctor}: expected {expected} actual {actual}")
        }
    }
}

fn render_specialization_facts(specialize: &SpecializationFacts) -> String {
    let mut out = String::new();
    writeln!(out, "work-items={}", specialize.work_items.len()).unwrap();
    for (index, item) in specialize.work_items.iter().enumerate() {
        writeln!(
            out,
            "work-item {index} {}",
            render_specialization_key(&item.key)
        )
        .unwrap();
        writeln!(
            out,
            "work-item {index} obligations={}",
            item.obligations.len()
        )
        .unwrap();
        for obligation in &item.obligations {
            writeln!(
                out,
                "work-item {index} obligation {}",
                render_discharged_obligation(obligation)
            )
            .unwrap();
        }
    }
    writeln!(out, "registry-entries={}", specialize.registry.len()).unwrap();
    out
}

fn render_monomorphization_plan(plan: &MonomorphizationPlan) -> String {
    let mut out = String::new();
    writeln!(out, "jobs={}", plan.jobs.len()).unwrap();
    for (index, job) in plan.jobs.iter().enumerate() {
        writeln!(
            out,
            "job {index} artifact={} status={} definition={} call-sites=[{}] {}",
            job.artifact,
            render_monomorphization_status(&job.status),
            job.definition_note,
            job.call_sites.join(", "),
            render_specialization_key(&job.key)
        )
        .unwrap();
    }
    writeln!(out, "duplicates={}", plan.duplicates.len()).unwrap();
    for duplicate in &plan.duplicates {
        writeln!(
            out,
            "duplicate call-site={} joined-artifact={} {}",
            duplicate.call_site,
            duplicate.joined_artifact,
            render_specialization_key(&duplicate.key)
        )
        .unwrap();
    }
    out
}

fn render_monomorphization_status(status: &MonomorphizationStatus) -> String {
    match status {
        MonomorphizationStatus::Scheduled => "scheduled".to_string(),
        MonomorphizationStatus::InProgress => "in-progress".to_string(),
        MonomorphizationStatus::Failed {
            call_site,
            definition_note,
            diagnostic,
        } => format!(
            "failed call-site={call_site} definition={definition_note} diagnostic={diagnostic}"
        ),
    }
}

fn render_specialization_key(key: &SpecializationKey) -> String {
    let params = key
        .template_params
        .iter()
        .map(|param| {
            format!(
                "{}:{}",
                param.name,
                render_template_param_source(param.source)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let instantiations = key
        .explicit_instantiations
        .iter()
        .map(|instantiation| {
            format!(
                "{}[{}]",
                instantiation.callee,
                instantiation.type_args.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let shapes = key
        .normalized_shapes
        .iter()
        .map(render_row_shape)
        .collect::<Vec<_>>()
        .join(", ");
    let dyn_contracts = key
        .dyn_contracts
        .iter()
        .map(render_dyn_contract)
        .collect::<Vec<_>>()
        .join(", ");
    let usage = key
        .capabilities
        .usage
        .iter()
        .copied()
        .map(render_usage_color)
        .collect::<Vec<_>>()
        .join(", ");
    let send = key
        .capabilities
        .send
        .iter()
        .copied()
        .map(render_send_color)
        .collect::<Vec<_>>()
        .join(", ");
    let continuations = key
        .continuations
        .iter()
        .copied()
        .map(render_continuation_kind)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "key generic={} abi={} params=[{}] instantiations=[{}] nominals=[{}] shapes=[{}] usage=[{}] send=[{}] continuations=[{}] dyn=[{}]",
        key.generic_symbol,
        render_abi_mode(key.abi_mode),
        params,
        instantiations,
        key.concrete_nominals.join(", "),
        shapes,
        usage,
        send,
        continuations,
        dyn_contracts
    )
}

fn render_discharged_obligation(obligation: &DischargedObligation) -> String {
    match obligation {
        DischargedObligation::Field { field, shape } => {
            format!("field {field} in {}", render_row_shape(shape))
        }
        DischargedObligation::Method { name, target } => format!(
            "method {name} -> {}",
            target.as_deref().unwrap_or("<pending>")
        ),
        DischargedObligation::Function { name, target } => {
            format!("function {name} -> {target}")
        }
        DischargedObligation::Static { name, target } => {
            format!("static {name} -> {target}")
        }
        DischargedObligation::Constructor {
            data,
            ctor,
            target,
            arity,
        } => format!("constructor {data}.{ctor}/{arity} -> {target}"),
        DischargedObligation::Operator {
            op,
            protocol,
            receiver,
        } => format!(
            "operator {} protocol={} receiver={}",
            render_operator_surface(op),
            protocol,
            receiver.as_deref().unwrap_or("<unknown>")
        ),
        DischargedObligation::DynAdapter { contract } => {
            format!("dyn-adapter {}", render_dyn_contract(contract))
        }
    }
}

fn render_continuation_kind(kind: ContinuationKind) -> &'static str {
    match kind {
        ContinuationKind::Cont1 => "cont1",
        ContinuationKind::ContN => "contn",
    }
}

fn render_abi_mode(mode: AbiMode) -> &'static str {
    match mode {
        AbiMode::Chiba => "chiba",
        AbiMode::Wasi => "wasi",
        AbiMode::Env => "env",
    }
}

fn render_template_facts(template: &TemplateFacts) -> String {
    let mut out = String::new();
    writeln!(out, "params={}", template.explicit_params.len()).unwrap();
    for param in &template.explicit_params {
        writeln!(
            out,
            "param {} source={}",
            param.name,
            render_template_param_source(param.source)
        )
        .unwrap();
    }
    writeln!(
        out,
        "instantiations={}",
        template.explicit_instantiations.len()
    )
    .unwrap();
    for instantiation in &template.explicit_instantiations {
        writeln!(
            out,
            "instantiate {}[{}]",
            instantiation.callee,
            instantiation.type_args.join(", ")
        )
        .unwrap();
    }
    writeln!(out, "row-shapes={}", template.row_shapes.len()).unwrap();
    for shape in &template.row_shapes {
        writeln!(out, "row-shape {}", render_row_shape(shape)).unwrap();
    }
    writeln!(out, "dyn-contracts={}", template.dyn_contracts.len()).unwrap();
    for contract in &template.dyn_contracts {
        writeln!(out, "dyn-contract {}", render_dyn_contract(contract)).unwrap();
    }
    writeln!(out, "obligations={}", template.obligations.len()).unwrap();
    for obligation in &template.obligations {
        writeln!(out, "obligation {}", render_template_obligation(obligation)).unwrap();
    }
    writeln!(out, "diagnostics={}", template.diagnostics.len()).unwrap();
    for diagnostic in &template.diagnostics {
        writeln!(out, "diagnostic {}", render_template_diagnostic(diagnostic)).unwrap();
    }
    out
}

fn render_template_audit(report: &TemplateAuditReport) -> String {
    let mut out = String::new();
    writeln!(out, "obligations={}", report.obligations.len()).unwrap();
    for (index, entry) in report.obligations.iter().enumerate() {
        writeln!(
            out,
            "obligation {index} source={} definition-check={} instantiation-discharge={} monomorphized={} rust-trait-solver={} specialization-key={} explanation={}",
            render_template_obligation_source(entry.source),
            entry.definition_time_check,
            entry.instantiation_time_discharge,
            entry.monomorphized,
            entry.rust_trait_solver_used,
            entry.specialization_key,
            entry.explanation
        )
        .unwrap();
    }
    writeln!(out, "diagnostics={}", report.diagnostics.len()).unwrap();
    for diagnostic in &report.diagnostics {
        writeln!(
            out,
            "diagnostic {}",
            render_template_audit_diagnostic(diagnostic)
        )
        .unwrap();
    }
    out
}

fn render_template_obligation_source(source: TemplateObligationSource) -> &'static str {
    match source {
        TemplateObligationSource::RowShape => "row-shape",
        TemplateObligationSource::Field => "field",
        TemplateObligationSource::Method => "method",
        TemplateObligationSource::Function => "function",
        TemplateObligationSource::Static => "static",
        TemplateObligationSource::Constructor => "constructor",
        TemplateObligationSource::Operator => "operator",
        TemplateObligationSource::DynAdapter => "dyn-adapter",
    }
}

fn render_template_audit_diagnostic(diagnostic: &TemplateAuditDiagnostic) -> String {
    match diagnostic {
        TemplateAuditDiagnostic::MissingDefinitionTimeCheck { source } => format!(
            "missing-definition-time-check {}",
            render_template_obligation_source(*source)
        ),
        TemplateAuditDiagnostic::MissingInstantiationDischarge { source } => format!(
            "missing-instantiation-discharge {}",
            render_template_obligation_source(*source)
        ),
        TemplateAuditDiagnostic::MissingMonomorphization { source } => format!(
            "missing-monomorphization {}",
            render_template_obligation_source(*source)
        ),
        TemplateAuditDiagnostic::RustTraitSolverLeak { source } => format!(
            "rust-trait-solver-leak {}",
            render_template_obligation_source(*source)
        ),
    }
}

fn render_template_param_source(source: TemplateParamSource) -> &'static str {
    match source {
        TemplateParamSource::ExplicitHeader => "explicit-header",
        TemplateParamSource::SyntheticAutoGeneric => "synthetic-auto-generic",
    }
}

fn render_template_obligation(obligation: &TemplateObligation) -> String {
    match obligation {
        TemplateObligation::Field { shape, field } => {
            format!("field {field} in {}", render_row_shape(shape))
        }
        TemplateObligation::Method {
            receiver,
            name,
            resolved,
        } => format!(
            "method {}.{} -> {}",
            receiver.as_deref().unwrap_or("<unknown>"),
            name,
            resolved.as_deref().unwrap_or("<pending>")
        ),
        TemplateObligation::Function { name, resolved } => {
            format!("function {name} -> {resolved}")
        }
        TemplateObligation::Static { name, resolved } => {
            format!("static {name} -> {resolved}")
        }
        TemplateObligation::Constructor {
            data,
            ctor,
            resolved,
            arity,
        } => format!("constructor {data}.{ctor}/{arity} -> {resolved}"),
        TemplateObligation::Operator {
            op,
            protocol,
            receiver,
        } => format!(
            "operator {} protocol={} receiver={}",
            render_operator_surface(op),
            protocol,
            receiver.as_deref().unwrap_or("<unknown>")
        ),
        TemplateObligation::DynAdapter { contract } => {
            format!("dyn-adapter {}", render_dyn_contract(contract))
        }
    }
}

fn render_template_diagnostic(diagnostic: &TemplateDiagnostic) -> String {
    match diagnostic {
        TemplateDiagnostic::ExplicitAutoGenericConflict { param } => {
            format!("explicit-auto-generic-conflict {param}")
        }
        TemplateDiagnostic::ConflictingExplicitInstantiation {
            callee,
            previous_type_args,
            type_args,
        } => format!(
            "conflicting-explicit-instantiation {callee}[{}] vs [{}]",
            previous_type_args.join(", "),
            type_args.join(", ")
        ),
    }
}

fn render_dyn_contract(contract: &DynRowContract) -> String {
    format!(
        "{} payload={} send={} adapter={}",
        render_row_shape(&contract.shape),
        render_usage_color(contract.payload_usage),
        render_send_color(contract.send),
        render_dyn_adapter_kind(contract.adapter)
    )
}

fn render_row_shape(shape: &RowShape) -> String {
    let fields = shape
        .fields
        .iter()
        .map(|field| format!("{}: {}", field.name, render_shape_type(&field.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let marker = match shape.openness {
        RowOpenness::Open => "r | ",
        RowOpenness::Closed => "",
    };
    format!("{{{marker}{fields}}}")
}

fn render_shape_type(ty: &ShapeType) -> String {
    match ty {
        ShapeType::Unknown => "_".to_string(),
        ShapeType::Named(name) => name.clone(),
    }
}

fn render_operator_surface(op: &OperatorSurface) -> String {
    match op {
        OperatorSurface::Binary(op) => render_source_binary_op(*op).to_string(),
        OperatorSurface::Index => "[]".to_string(),
        OperatorSurface::IndexSlice => "[..]".to_string(),
    }
}

fn render_usage_color(color: crate::typed::UsageColor) -> &'static str {
    match color {
        crate::typed::UsageColor::One => "1",
        crate::typed::UsageColor::Many => "N",
        crate::typed::UsageColor::Obligation => "obligation",
    }
}

fn render_send_color(color: crate::typed::SendColor) -> &'static str {
    match color {
        crate::typed::SendColor::Send => "send",
        crate::typed::SendColor::NotSend => "!send",
        crate::typed::SendColor::Obligation => "obligation",
    }
}

fn render_dyn_adapter_kind(kind: DynAdapterKind) -> &'static str {
    match kind {
        DynAdapterKind::StaticToDynPackage => "static-to-dyn-package",
        DynAdapterKind::DynAdapterAccess => "dyn-adapter-access",
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
