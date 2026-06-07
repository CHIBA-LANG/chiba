use chiba_level1r::ast::{
    DataDecl, DataVariant, ExternAbi, ExternDecl, MethodReceiver, NamespaceDecl, ParamDecl,
    SourceItem, SourceProgram, TypeDecl, TypeField, UseDecl, Visibility,
};
use chiba_level1r::core::CoreOp;
use chiba_level1r::pattern::PatternDiagnostic;
use chiba_level1r::typed::{AggregateKind, Type, TypedExprKind};
use chiba_level1r::{
    build_interface_summary, compile_program, compile_program_bundle,
    compile_program_with_interface, compile_source_program_bundle, parse_source_program,
    project_surface_many, Expr, ProgramCompileOutput, ProgramDiagnostic,
};
use std::process::Command;

fn def(name: &str, params: Vec<&str>, body: Expr) -> SourceItem {
    SourceItem::Def {
        receiver: None,
        generics: Vec::new(),
        name: name.to_string(),
        visibility: Visibility::Public,
        params: params.into_iter().map(ParamDecl::untyped).collect(),
        return_type: None,
        body,
    }
}

fn static_value(name: &str, ty: Option<&str>, body: Expr) -> SourceItem {
    SourceItem::StaticValue {
        name: name.to_string(),
        ty: ty.map(str::to_string),
        visibility: Visibility::Public,
        body,
    }
}

fn extern_def(name: &str, abi: ExternAbi, symbol: &str) -> SourceItem {
    SourceItem::extern_def(
        name,
        Vec::new(),
        Vec::new(),
        Some("i64".to_string()),
        ExternDecl::new(abi, symbol),
    )
}

fn assert_backend_link_clean(bundle: &ProgramCompileOutput, def_index: usize) {
    assert!(
        bundle.backend_link.diagnostics.is_empty(),
        "{}",
        render_backend_failure_context(bundle, Some(def_index))
    );
}

fn assert_backend_link_clean_all(bundle: &ProgramCompileOutput) {
    assert!(
        bundle.backend_link.diagnostics.is_empty(),
        "{}",
        render_backend_failure_context(bundle, None)
    );
}

fn render_backend_failure_context(
    bundle: &ProgramCompileOutput,
    def_index: Option<usize>,
) -> String {
    let mut out = String::new();
    out.push_str(&bundle.render_summary());
    match def_index {
        Some(index) => {
            out.push_str("def visual:\n");
            out.push_str(&bundle.defs[index].output.render_visual());
        }
        None => {
            out.push_str("defs visual:\n");
            for def in &bundle.defs {
                out.push_str(&format!("def {}\n", def.name));
                out.push_str(&def.output.render_visual());
            }
        }
    }
    out.push_str("linked wat:\n");
    out.push_str(&bundle.backend_link.linked_wat);
    out
}

fn typed_expr_kind_name(kind: &TypedExprKind) -> &'static str {
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
        TypedExprKind::Assign { .. } => "assign",
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

#[test]
fn compile_program_keeps_legacy_per_def_outputs() {
    let program = SourceProgram::new(vec![
        def("one", vec![], Expr::i64(1)),
        def("two", vec![], Expr::i64(2)),
    ]);

    let outputs = compile_program(&program);

    assert_eq!(outputs.len(), 2);
    assert!(outputs[0].backend.wat.contains("i32.const 1"));
    assert!(outputs[1].backend.wat.contains("i32.const 2"));
}

#[test]
fn program_typed_binary_i64_result_is_not_unknown() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::binary(
            chiba_level1r::ast::BinaryOp::Add,
            Expr::i64(2),
            Expr::i64(3),
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
}

#[test]
fn program_typed_lambda_call_uses_function_return_type() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::call(Expr::lambda("x", Expr::i64(7)), Expr::i64(1)),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
}

#[test]
fn typed_multi_arg_call_steps_through_curried_function_type() {
    let mut env = chiba_level1r::typed::TypeEnv::new();
    env.insert(
        "f".to_string(),
        Type::Func(
            Box::new(Type::I64),
            Box::new(Type::Func(Box::new(Type::Bool), Box::new(Type::I64))),
        ),
    );

    let typed = chiba_level1r::typed::type_expr_with_context(
        &Expr::call_args(Expr::var("f"), vec![Expr::i64(1), Expr::bool(true)]),
        &env,
        &chiba_level1r::typed::TypeContext::new(),
    );

    assert_eq!(typed.ty, Type::I64);
}

#[test]
fn typed_partial_call_preserves_remaining_function_type() {
    let remaining = Type::Func(Box::new(Type::Bool), Box::new(Type::I64));
    let mut env = chiba_level1r::typed::TypeEnv::new();
    env.insert(
        "f".to_string(),
        Type::Func(Box::new(Type::I64), Box::new(remaining.clone())),
    );

    let typed = chiba_level1r::typed::type_expr_with_context(
        &Expr::call(Expr::var("f"), Expr::i64(1)),
        &env,
        &chiba_level1r::typed::TypeContext::new(),
    );

    assert_eq!(typed.ty, remaining);
}

#[test]
fn typed_continuation_storage_call_returns_answer_type() {
    let output =
        chiba_level1r::compile_source_program_bundle("def main(k: contN (i64) -> bool) = k(1)")
            .expect("compile source");
    let main = &output.program.defs[0].output;

    assert_eq!(main.typed.ty, Type::Bool);
    assert!(main.cps.to_string().contains("k(1,"));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::DynamicCallableTarget { target }
                if target == "k"
        )
    }));
    assert!(main.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "k" && args == &vec![chiba_level1r::core::CoreValue::I64(1)]
        )
    }));
    assert!(main.core.callable_storage.iter().any(|fact| {
        fact.subject == "param::k"
            && fact.kind == chiba_level1r::core::CallableStorageKind::ContNPackage
            && fact.send == chiba_level1r::typed::SendColor::NotSend
    }));
    assert!(main.core_validation.diagnostics.is_empty());
}

#[test]
fn typed_continuation_storage_return_enters_callable_storage() {
    let output =
        chiba_level1r::compile_source_program_bundle("def main(): contN (i64) -> bool = 0")
            .expect("compile source");
    let main = &output.program.defs[0].output;

    assert!(main.core.callable_storage.iter().any(|fact| {
        fact.subject == "return::main"
            && fact.kind == chiba_level1r::core::CallableStorageKind::ContNPackage
            && fact.send == chiba_level1r::typed::SendColor::NotSend
    }));
}

#[test]
fn typed_continuation_boxed_storage_param_enters_callable_storage() {
    let output =
        chiba_level1r::compile_source_program_bundle("def main(k: cont1 (i64) -> bool) = 0")
            .expect("compile source");
    let main = &output.program.defs[0].output;

    assert!(main.core.callable_storage.iter().any(|fact| {
        fact.subject == "param::k"
            && fact.kind == chiba_level1r::core::CallableStorageKind::BoxedCont1
            && fact.send == chiba_level1r::typed::SendColor::NotSend
    }));
}

#[test]
fn source_def_params_enter_alpha_scope_and_usage_by_binder_id() {
    let output = chiba_level1r::compile_source_program_bundle("def main(x: i64) = x + x")
        .expect("compile source");
    let main = &output.program.defs[0].output;

    assert_eq!(main.alpha.param_binders.len(), 1);
    let param = &main.alpha.param_binders[0];
    assert_eq!(param.name, "x");
    assert!(main.alpha.diagnostics.is_empty());
    assert_eq!(
        main.usage.binders.get(&param.id).copied(),
        Some(chiba_level1r::usage::UseCount::Many)
    );
    assert_eq!(main.usage.vars.get("x").copied(), None);
    assert!(main.visual.alpha.contains("params=1"));
    assert!(main.visual.alpha.contains("param %0 name=x"));
}

#[test]
fn typed_cont1_storage_param_repeated_call_is_rejected_by_core() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main(k: cont1 (i64) -> i64) = k(1) + k(2)",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert!(main.core.callable_storage.iter().any(|fact| {
        fact.subject == "param::k"
            && fact.kind == chiba_level1r::core::CallableStorageKind::BoxedCont1
            && fact.usage == chiba_level1r::typed::UsageColor::Many
            && fact.send == chiba_level1r::typed::SendColor::NotSend
    }));
    assert_eq!(
        main.core_validation.diagnostics,
        vec![
            chiba_level1r::core::CoreDiagnostic::Cont1CallableUsedMoreThanOnce {
                subject: "param::k".to_string()
            }
        ]
    );
    assert_eq!(
        main.backend.diagnostics,
        vec![chiba_level1r::backend::BackendDiagnostic::CoreValidationFailed { diagnostics: 1 }]
    );
    assert_eq!(main.backend.wat, "");
    assert_eq!(bundle.backend_link.linked_wat, "");
    assert_eq!(
        bundle.backend_link.diagnostics,
        vec![
            chiba_level1r::backend::BackendLinkDiagnostic::ArtifactEmitFailed { artifact_index: 0 }
        ]
    );
}

#[test]
fn typed_cont1_storage_param_usage_ignores_shadowing_local_binder() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main(k: cont1 (i64) -> i64) = (k: I64): I64 => k + k",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    assert!(main.core.callable_storage.iter().any(|fact| {
        fact.subject == "param::k"
            && fact.kind == chiba_level1r::core::CallableStorageKind::BoxedCont1
            && fact.usage == chiba_level1r::typed::UsageColor::One
            && fact.send == chiba_level1r::typed::SendColor::NotSend
    }));
    assert!(!main.core_validation.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            chiba_level1r::core::CoreDiagnostic::Cont1CallableUsedMoreThanOnce { subject }
                if subject == "param::k"
        )
    }));
}

#[test]
fn program_cont1_single_resume_lowers_to_executable_wat() {
    let output =
        chiba_level1r::compile_source_program_bundle("def main() = reset { shift k { k(7) } }")
            .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert!(main.backend.wat.contains(";; prompt kind=cont1"));
    assert!(main
        .backend
        .wat
        .contains(";; resume-cont binder=k kind=cont1 result=w0"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "7");
}

#[test]
fn program_cont1_repeated_resume_is_rejected_by_backend() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = reset { shift k { k(1) + k(2) } }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::Cont1ResumedMoreThanOnce {
            def: "main".to_string(),
            binder: "k".to_string()
        }]
    );
    assert_eq!(
        main.backend.diagnostics,
        vec![
            chiba_level1r::backend::BackendDiagnostic::Cont1ResumedMoreThanOnce {
                binder: "k".to_string()
            }
        ]
    );
    assert_eq!(main.backend.wat, "");
    assert_eq!(bundle.backend_link.linked_wat, "");
    assert_eq!(
        bundle.backend_link.diagnostics,
        vec![
            chiba_level1r::backend::BackendLinkDiagnostic::ArtifactEmitFailed { artifact_index: 0 }
        ]
    );
}

#[test]
fn program_contn_repeated_resume_lowers_to_executable_wat() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = resetn { shift retry { retry(1) + retry(2) } }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert!(main.backend.wat.contains(";; prompt kind=contn"));
    assert!(main
        .backend
        .wat
        .contains(";; resume-cont binder=retry kind=contn result=w0"));
    assert!(main
        .backend
        .wat
        .contains(";; resume-cont binder=retry kind=contn result=w1"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn program_contn_capture_under_function_call_is_replay_unsafe() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def f(x: I64): I64 = x
def main() = resetn { f(shift retry { retry(1) }) }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main def")
        .output;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::UnsafeMultiResumeCapture {
            def: "main".to_string(),
            binder: "retry".to_string()
        }]
    );
    assert_eq!(
        main.control.errors,
        vec![
            chiba_level1r::control::ControlError::UnsafeMultiResumeCapture {
                binder: "retry".to_string()
            }
        ]
    );
    assert_eq!(
        main.control.continuations[0].replay_safety,
        chiba_level1r::control::ReplaySafety::Unsafe
    );
    assert_eq!(
        main.core_validation.diagnostics,
        vec![
            chiba_level1r::core::CoreDiagnostic::UnsafeContNReplayCapture {
                binder: "retry".to_string()
            }
        ]
    );
    assert_eq!(
        bundle.backend_link.diagnostics,
        vec![
            chiba_level1r::backend::BackendLinkDiagnostic::ArtifactEmitFailed { artifact_index: 1 }
        ]
    );
}

#[test]
fn program_contn_capture_under_pure_builtin_method_is_replay_safe() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main(): rune = resetn { \"abc\".char_at(shift retry { retry(1) + retry(2) }) }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(main.control.errors, vec![]);
    assert_eq!(
        main.control.continuations[0].replay_safety,
        chiba_level1r::control::ReplaySafety::Safe
    );
    assert_backend_link_clean_all(&bundle);
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "197");
}

#[test]
fn program_contn_capture_under_string_concat_is_replay_safe() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main(): i64 = resetn { \"a\".concat(shift retry { retry(\"b\") + retry(\"c\") }).bytes_len() }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(main.control.errors, vec![]);
    assert_eq!(
        main.control.continuations[0].replay_safety,
        chiba_level1r::control::ReplaySafety::Safe
    );
    assert_backend_link_clean_all(&bundle);
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "4");
}

#[test]
fn program_contn_capture_under_unsafe_ref_builtin_is_replay_unsafe() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = resetn { UnsafeRef.new(shift retry { retry(1) }) }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::UnsafeMultiResumeCapture {
            def: "main".to_string(),
            binder: "retry".to_string()
        }]
    );
    assert_eq!(
        main.control.errors,
        vec![
            chiba_level1r::control::ControlError::UnsafeMultiResumeCapture {
                binder: "retry".to_string()
            }
        ]
    );
    assert_eq!(
        main.control.continuations[0].replay_safety,
        chiba_level1r::control::ReplaySafety::Unsafe
    );
}

#[test]
fn program_shift_outside_reset_reaches_program_diagnostics() {
    let output = chiba_level1r::compile_source_program_bundle("def main() = shift k { k(1) }")
        .expect("compile source");
    let bundle = output.program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::ShiftOutsideReset {
            def: "main".to_string(),
            binder: "k".to_string()
        }]
    );
    assert_eq!(
        bundle.defs[0].output.control.errors,
        vec![chiba_level1r::control::ControlError::ShiftOutsideReset {
            binder: "k".to_string()
        }]
    );
}

#[test]
fn program_contn_repeated_resume_replays_captured_reset_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = resetn { 10 + shift retry { retry(1) + retry(2) } }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(
        main.backend
            .wat
            .matches(";; resume-cont binder=retry kind=contn result=")
            .count(),
        2
    );
    assert_eq!(
        main.backend
            .wat
            .matches(";; captured-tailcall op_add")
            .count(),
        2
    );
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "23");
}

#[test]
fn program_cont1_repeated_resume_is_rejected_before_program_backend_link() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = reset { 10 + shift k { k(1) + k(2) } }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::Cont1ResumedMoreThanOnce {
            def: "main".to_string(),
            binder: "k".to_string()
        }]
    );
    assert_eq!(
        main.backend.diagnostics,
        vec![
            chiba_level1r::backend::BackendDiagnostic::Cont1ResumedMoreThanOnce {
                binder: "k".to_string()
            }
        ]
    );
    assert_eq!(main.backend.wat, "");
    assert_eq!(
        bundle.backend_link.diagnostics,
        vec![
            chiba_level1r::backend::BackendLinkDiagnostic::ArtifactEmitFailed { artifact_index: 0 }
        ]
    );
    assert_eq!(bundle.backend_link.linked_wat, "");
}

#[test]
fn program_reset_shift_answer_type_uses_captured_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = reset { 10 + shift k { k(7) } }",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    assert_eq!(main.typed.ty, Type::I64);
    assert_eq!(main.control.continuations[0].input, Type::I64);
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert_eq!(run_wat_text(&output.program.backend_link.linked_wat), "17");
}

#[test]
fn program_cont1_single_resume_replays_captured_branch_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = reset { if shift k { k(true) } { 11 } else { 22 } }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(main.control.continuations[0].input, Type::Bool);
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert!(main
        .backend
        .wat
        .contains(";; resume-cont binder=k kind=cont1"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "11");
}

#[test]
fn program_contn_repeated_resume_replays_captured_branch_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = resetn { if shift retry { retry(true) + retry(false) } { 11 } else { 22 } }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(main.control.continuations[0].input, Type::Bool);
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert_eq!(
        main.backend
            .wat
            .matches(";; resume-cont binder=retry kind=contn result=")
            .count(),
        2
    );
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "33");
}

#[test]
fn program_cont1_single_resume_replays_captured_match_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = reset { match shift k { k(1) } { 1 => 11, _ => 22 } }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(main.control.continuations[0].input, Type::I64);
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert!(main
        .backend
        .wat
        .contains(";; resume-cont binder=k kind=cont1"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "11");
}

#[test]
fn program_contn_repeated_resume_replays_captured_match_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = resetn { match shift retry { retry(1) + retry(2) } { 1 => 11, _ => 22 } }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(main.control.continuations[0].input, Type::I64);
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert_eq!(
        main.backend
            .wat
            .matches(";; resume-cont binder=retry kind=contn result=")
            .count(),
        2
    );
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "33");
}

#[test]
fn program_cont1_single_resume_replays_captured_tuple_field_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = reset { (shift k { k((4, true)) })._2 }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(
        main.control.continuations[0].input,
        Type::Tuple(vec![Type::I64, Type::Bool])
    );
    assert_eq!(main.control.continuations[0].answer, Type::Bool);
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_cont1_single_resume_replays_captured_record_field_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = reset { (shift k { k({x: 4, y: true}) }).x }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "4");
}

#[test]
fn program_contn_repeated_resume_replays_captured_tuple_field_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = resetn { (shift retry { retry((4, false)) + retry((7, true)) })._1 }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(
        main.control.continuations[0].input,
        Type::Tuple(vec![Type::I64, Type::Bool])
    );
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert_eq!(
        main.backend
            .wat
            .matches(";; resume-cont binder=retry kind=contn result=")
            .count(),
        2
    );
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "11");
}

#[test]
fn program_contn_repeated_resume_replays_captured_record_field_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = resetn { (shift retry { retry({x: 4, y: false}) + retry({x: 7, y: true}) }).x }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert_eq!(
        main.backend
            .wat
            .matches(";; resume-cont binder=retry kind=contn result=")
            .count(),
        2
    );
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "11");
}

#[test]
fn program_contn_repeated_resume_replays_captured_record_update_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main() = resetn { { shift retry { retry({x: 4, y: false}) + retry({x: 7, y: true}) } | x: 9 }.x }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert_eq!(
        main.backend
            .wat
            .matches(";; resume-cont binder=retry kind=contn result=")
            .count(),
        2
    );
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "18");
}

#[test]
fn program_contn_repeated_resume_replays_captured_adt_match_context() {
    let output = chiba_level1r::compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def main() = resetn { match shift retry { retry(Option.Some(7)) + retry(Option.None) } { Option.Some(value) => value, Option.None => 0 } }",
    )
    .expect("compile source");
    let bundle = output.program;
    let main = &bundle.defs[0].output;

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert_eq!(main.backend.diagnostics, vec![]);
    assert_eq!(
        main.control.continuations[0].input,
        Type::Nominal("Option[i64]".to_string())
    );
    assert_eq!(main.control.continuations[0].answer, Type::I64);
    assert_eq!(
        main.backend
            .wat
            .matches(";; resume-cont binder=retry kind=contn result=")
            .count(),
        2
    );
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "7");
}

#[test]
fn interface_type_continuation_field_enters_callable_storage() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "RetryBox",
            Vec::new(),
            vec![TypeField::new("retry", "ContN[i64,bool]")],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert!(bundle.defs[0]
        .output
        .core
        .callable_storage
        .iter()
        .any(|fact| {
            fact.subject == "type::RetryBox::retry"
                && fact.kind == chiba_level1r::core::CallableStorageKind::ContNPackage
                && fact.send == chiba_level1r::typed::SendColor::NotSend
        }));
    assert!(bundle.defs[0]
        .output
        .render_visual()
        .contains("callable-storage:"));
    assert!(bundle.defs[0]
        .output
        .render_visual()
        .contains("type::RetryBox::retry"));
    assert!(bundle.defs[0]
        .output
        .render_visual()
        .contains("kind=contn-package"));
    assert!(!bundle.defs[0]
        .output
        .render_visual()
        .contains("ContNPackage"));
    assert!(!bundle.defs[0]
        .output
        .render_visual()
        .contains("CallableStorageFact"));
}

#[test]
fn program_bundle_selects_main_and_links_def_artifacts() {
    let program = SourceProgram::new(vec![
        def("helper", vec![], Expr::i64(1)),
        def("main", vec![], Expr::i64(7)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.entry, Some("main".to_string()));
    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(bundle.defs.len(), 2);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $helper (result i32)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main (export \"main\") (result i32)"));
    assert!(bundle.backend_link.linked_wat.contains("i32.const 1"));
    assert!(bundle.backend_link.linked_wat.contains("i32.const 7"));
}

#[test]
fn program_branch_return_lowers_to_executable_wat_if() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::if_else(Expr::bool(true), Expr::i64(1), Expr::i64(2)),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnBranch {
                cond: chiba_level1r::core::CoreValue::Bool(true),
                then_value: chiba_level1r::core::CoreValue::I64(1),
                else_value: chiba_level1r::core::CoreValue::I64(2),
            }
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains("if (result i32)"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_literal_match_return_lowers_to_executable_wat_if_chain() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::match_expr(
            Expr::i64(0),
            vec![
                (chiba_level1r::ast::Pattern::lit_i64(0), Expr::i64(7)),
                (chiba_level1r::ast::Pattern::wildcard(), Expr::i64(9)),
            ],
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnMatch {
                scrutinee: chiba_level1r::core::CoreValue::I64(0),
                arms,
            } if arms.len() == 2
                && arms[0].pattern == chiba_level1r::core::CorePattern::I64(0)
                && arms[0].value == chiba_level1r::core::CoreValue::I64(7)
                && arms[1].pattern == chiba_level1r::core::CorePattern::Wildcard
                && arms[1].value == chiba_level1r::core::CoreValue::I64(9)
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; core-return match scrutinee=0 arms=2"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "7");
}

#[test]
fn program_tuple_field_return_lowers_to_executable_wat_value() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::field(Expr::tuple(vec![Expr::i64(4), Expr::bool(true)]), "_2"),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnValue(
                chiba_level1r::core::CoreValue::TupleField { tuple, field, field_index }
            ) if field == "_2"
                && *field_index == 1
                && matches!(
                    tuple.as_ref(),
                    chiba_level1r::core::CoreValue::Tuple { fields }
                        if fields == &vec![
                            chiba_level1r::core::CoreValue::I64(4),
                            chiba_level1r::core::CoreValue::Bool(true),
                        ]
                )
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; tuple-field layout=tuple::Tuple2_I64_Bool field=_2"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_record_field_return_lowers_to_executable_wat_value() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::field(
            Expr::record(vec![("x", Expr::i64(4)), ("y", Expr::bool(true))]),
            "y",
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnValue(
                chiba_level1r::core::CoreValue::RecordField { record, field }
            ) if field == "y"
                && matches!(
                    record.as_ref(),
                    chiba_level1r::core::CoreValue::Record { fields }
                        if fields.iter().any(|field|
                            field.name == "x"
                                && field.value == chiba_level1r::core::CoreValue::I64(4)
                        )
                        && fields.iter().any(|field|
                            field.name == "y"
                                && field.value == chiba_level1r::core::CoreValue::Bool(true)
                        )
                )
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; record-field layout=record::x+y field=y"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_range_field_return_lowers_to_executable_wat_value() {
    let start = compile_source_program_bundle("def main() = (3..9).start")
        .expect("compile range start")
        .program;
    let end = compile_source_program_bundle("def main() = (3..9).end")
        .expect("compile range end")
        .program;

    assert_backend_link_clean_all(&start);
    assert_backend_link_clean_all(&end);
    let start_main = start
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("start main");
    let end_main = end
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("end main");

    assert_eq!(start_main.output.typed.ty, Type::I64);
    assert_eq!(end_main.output.typed.ty, Type::I64);
    assert!(start_main.output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::RangeFieldGet {
                field: chiba_level1r::core::RangeField::Start
            }
        )
    }));
    assert!(end_main.output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::RangeFieldGet {
                field: chiba_level1r::core::RangeField::End
            }
        )
    }));
    assert!(matches!(
        start_main.output.backend.return_value,
        Some(chiba_level1r::core::CoreValue::RangeField {
            field: chiba_level1r::core::RangeField::Start,
            ..
        })
    ));
    assert!(matches!(
        end_main.output.backend.return_value,
        Some(chiba_level1r::core::CoreValue::RangeField {
            field: chiba_level1r::core::RangeField::End,
            ..
        })
    ));
    assert_eq!(run_wat_text(&start.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&end.backend_link.linked_wat), "9");
}

#[test]
fn program_range_return_lowers_to_executable_externref_handle() {
    let bundle = compile_source_program_bundle("def main() = 3..9")
        .expect("compile range return")
        .program;

    assert_backend_link_clean_all(&bundle);
    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main");

    assert_eq!(main.output.typed.ty, Type::Nominal("Range".to_string()));
    assert!(matches!(
        main.output.backend.return_value,
        Some(chiba_level1r::core::CoreValue::Range { .. })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.range_i64_new\" (func $std_range_i64_new (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main (export \"main\") (result externref)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_range_i64_new"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "range/3/9");
}

#[test]
fn program_slice_literal_len_lowers_to_executable_wat_value() {
    let bundle = compile_source_program_bundle("def main() = [1, 2, 3].len")
        .expect("compile slice len")
        .program;

    assert_backend_link_clean_all(&bundle);
    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main");

    assert_eq!(main.output.typed.ty, Type::I64);
    assert!(main.output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::SliceFieldGet {
                field: chiba_level1r::core::SliceField::Len
            }
        )
    }));
    assert!(matches!(
        main.output.backend.return_value,
        Some(chiba_level1r::core::CoreValue::AggregateField {
            kind: AggregateKind::Slice,
            field: chiba_level1r::core::SliceField::Len,
            ..
        })
    ));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn program_slice_literal_return_lowers_to_executable_externref_handle() {
    let bundle = compile_source_program_bundle("def main() = [1, 2, 3]")
        .expect("compile slice literal")
        .program;

    assert_backend_link_clean_all(&bundle);
    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main");

    assert_eq!(
        main.output.typed.ty,
        Type::Nominal("Slice[i64]".to_string())
    );
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_i64_literal_3\" (func $std_slice_i64_literal_3 (param i32) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main (export \"main\") (result externref)"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "slice/3");
}

#[test]
fn program_bool_slice_literal_lowers_to_executable_values() {
    let len = compile_source_program_bundle("def main(): i64 = [true, false].len")
        .expect("compile bool slice literal len")
        .program;
    let first = compile_source_program_bundle("def main(): bool = [true, false][0]")
        .expect("compile bool slice literal first")
        .program;
    let returned = compile_source_program_bundle("def main() = [true, false]")
        .expect("compile bool slice literal return")
        .program;

    for bundle in [&len, &first, &returned] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.slice_i64_literal_2\" (func $std_slice_i64_literal_2 (param i32) (param i32) (result externref)))"
        ));
    }
    assert_eq!(
        returned.defs[0].output.typed.ty,
        Type::Nominal("Slice[bool]".to_string())
    );
    assert_eq!(first.defs[0].output.typed.ty, Type::Bool);
    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "2");
    assert_eq!(run_wat_text(&first.backend_link.linked_wat), "1");
    assert_eq!(run_wat_text(&returned.backend_link.linked_wat), "slice/2");
}

#[test]
fn program_string_slice_literal_lowers_to_executable_values() {
    let len =
        compile_source_program_bundle("def main(): i64 = [String.from(\"hé\")][0].bytes_len()")
            .expect("compile string slice literal element len")
            .program;
    let returned = compile_source_program_bundle("def main() = [String.from(\"hé\")]")
        .expect("compile string slice literal return")
        .program;

    for bundle in [&len, &returned] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.slice_externref_literal_1\" (func $std_slice_externref_literal_1 (param externref) (result externref)))"
        ));
    }
    assert_eq!(
        returned.defs[0].output.typed.ty,
        Type::Nominal("Slice[String]".to_string())
    );
    assert_eq!(len.defs[0].output.typed.ty, Type::I64);
    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&returned.backend_link.linked_wat), "slice/1");
}

#[test]
fn source_slice_param_len_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: Slice[i64]): i64 = s.len")
        .expect("compile slice param len")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_i64_len\" (func $std_slice_i64_len (param externref) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main (export \"main\") (param $s externref) (result i32)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_len"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_slice_param_index_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: Slice[i64]): i64 = s[1]")
        .expect("compile slice param index")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::AggregateIndex {
            kind: AggregateKind::Slice,
            ..
        })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_i64_get\" (func $std_slice_i64_get (param externref) (param i32) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main (export \"main\") (param $s externref) (result i32)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_slice_param_bool_index_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: Slice[bool]): bool = s[0]")
        .expect("compile bool slice param index")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Bool);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::AggregateIndex {
            kind: AggregateKind::Slice,
            ..
        })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_i64_get\" (func $std_slice_i64_get (param externref) (param i32) (result i32)))"
    ));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn source_slice_param_range_slice_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: Slice[i64]): i64 = s[1..3].len")
        .expect("compile slice param range slice")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_i64_slice\" (func $std_slice_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_slice"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_slice_param_bool_range_slice_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: Slice[bool]): bool = s[0..2][0]")
        .expect("compile bool slice param range slice")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Bool);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_i64_slice\" (func $std_slice_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_slice"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn source_slice_param_string_index_lowers_to_executable_externref_runtime_import() {
    let bundle =
        compile_source_program_bundle("def main(s: Slice[String]): i64 = s[0].bytes_len()")
            .expect("compile string slice param index")
            .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::BuiltinRuntimeCall { .. })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_get\" (func $std_slice_get (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_get"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_slice_param_string_range_slice_lowers_to_executable_externref_runtime_import() {
    let bundle =
        compile_source_program_bundle("def main(s: Slice[String]): i64 = s[0..2][0].char_at(0)")
            .expect("compile string slice param range slice")
            .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Rune);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_i64_slice\" (func $std_slice_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_get\" (func $std_slice_get (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_slice"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_get"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "104");
}

#[test]
fn source_array_param_len_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(a: Array[i64]): i64 = a.len")
        .expect("compile array param len")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["a".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.array_i64_len\" (func $std_array_i64_len (param externref) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main (export \"main\") (param $a externref) (result i32)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_array_i64_len"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_array_param_index_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(a: Array[i64]): i64 = a[1]")
        .expect("compile array param index")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["a".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::AggregateIndex {
            kind: AggregateKind::Array,
            ..
        })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.array_i64_get\" (func $std_array_i64_get (param externref) (param i32) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_array_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_array_param_bool_index_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(a: Array[bool]): bool = a[0]")
        .expect("compile bool array param index")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["a".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Bool);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::AggregateIndex {
            kind: AggregateKind::Array,
            ..
        })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.array_i64_get\" (func $std_array_i64_get (param externref) (param i32) (result i32)))"
    ));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn source_array_param_range_slice_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(a: Array[i64]): i64 = a[0..2][1]")
        .expect("compile array param range slice")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["a".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.array_i64_slice\" (func $std_array_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_array_i64_slice"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_array_param_bool_range_slice_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(a: Array[bool]): bool = a[0..2][0]")
        .expect("compile bool array param range slice")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["a".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Bool);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.array_i64_slice\" (func $std_array_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_array_i64_slice"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn source_array_param_string_index_lowers_to_executable_externref_runtime_import() {
    let bundle =
        compile_source_program_bundle("def main(a: Array[String]): i64 = a[0].bytes_len()")
            .expect("compile string array param index")
            .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["a".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.array_get\" (func $std_array_get (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_array_get"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("call $std_array_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_array_param_string_range_slice_lowers_to_executable_externref_runtime_import() {
    let bundle =
        compile_source_program_bundle("def main(a: Array[String]): i64 = a[0..2][0].char_at(0)")
            .expect("compile string array param range slice")
            .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["a".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Rune);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.array_i64_slice\" (func $std_array_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_get\" (func $std_slice_get (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_array_i64_slice"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_get"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "104");
}

#[test]
fn source_vec_param_len_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(v: Vec[i64]): i64 = v.len")
        .expect("compile vec param len")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["v".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_i64_len\" (func $std_vec_i64_len (param externref) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main (export \"main\") (param $v externref) (result i32)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_i64_len"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_vec_param_index_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(v: Vec[i64]): i64 = v[1]")
        .expect("compile vec param index")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["v".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::AggregateIndex {
            kind: AggregateKind::Vec,
            ..
        })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_i64_get\" (func $std_vec_i64_get (param externref) (param i32) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_vec_param_bool_index_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(v: Vec[bool]): bool = v[0]")
        .expect("compile bool vec param index")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["v".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Bool);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::AggregateIndex {
            kind: AggregateKind::Vec,
            ..
        })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_i64_get\" (func $std_vec_i64_get (param externref) (param i32) (result i32)))"
    ));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn source_vec_param_range_slice_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(v: Vec[i64]): i64 = v[0..2][1]")
        .expect("compile vec param range slice")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["v".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_i64_slice\" (func $std_vec_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_i64_slice"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_vec_param_bool_range_slice_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(v: Vec[bool]): bool = v[0..2][0]")
        .expect("compile bool vec param range slice")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["v".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Bool);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_i64_slice\" (func $std_vec_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_i64_slice"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn source_vec_param_string_index_lowers_to_executable_externref_runtime_import() {
    let bundle = compile_source_program_bundle("def main(v: Vec[String]): i64 = v[0].bytes_len()")
        .expect("compile string vec param index")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["v".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_get\" (func $std_vec_get (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains("call $std_vec_get"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_vec_param_string_range_slice_lowers_to_executable_externref_runtime_import() {
    let bundle =
        compile_source_program_bundle("def main(v: Vec[String]): rune = v[0..2][0].char_at(0)")
            .expect("compile string vec param range slice")
            .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["v".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Rune);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_i64_slice\" (func $std_vec_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_get\" (func $std_slice_get (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_i64_slice"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_get"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "104");
}

#[test]
fn source_builtin_externref_params_can_return_executable_handles() {
    let slice = compile_source_program_bundle("def main(s: Slice[i64]): Slice[i64] = s")
        .expect("compile slice param passthrough")
        .program;
    let array = compile_source_program_bundle("def main(a: Array[i64]): Array[i64] = a")
        .expect("compile array param passthrough")
        .program;
    let vec = compile_source_program_bundle("def main(v: Vec[String]): Vec[String] = v")
        .expect("compile vec param passthrough")
        .program;
    let string = compile_source_program_bundle("def main(s: String): String = s")
        .expect("compile string param passthrough")
        .program;
    let str_view = compile_source_program_bundle("def main(s: str): str = s")
        .expect("compile str param passthrough")
        .program;
    let cstr = compile_source_program_bundle("def main(s: cstr): cstr = s")
        .expect("compile cstr param passthrough")
        .program;
    let range = compile_source_program_bundle("def main(r: Range): Range = r")
        .expect("compile range param passthrough")
        .program;

    for (bundle, param) in [
        (&slice, "s"),
        (&array, "a"),
        (&vec, "v"),
        (&string, "s"),
        (&str_view, "s"),
        (&cstr, "s"),
        (&range, "r"),
    ] {
        assert_eq!(
            bundle.diagnostics,
            vec![ProgramDiagnostic::EntryHasParams {
                name: "main".to_string(),
                params: vec![param.to_string()]
            }]
        );
        assert_backend_link_clean(bundle, 0);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("(result externref)"));
    }
    assert_eq!(
        slice.defs[0].output.typed.ty,
        Type::Nominal("Slice[i64]".to_string())
    );
    assert_eq!(
        array.defs[0].output.typed.ty,
        Type::Nominal("Array[i64]".to_string())
    );
    assert_eq!(
        vec.defs[0].output.typed.ty,
        Type::Nominal("Vec[String]".to_string())
    );
    assert_eq!(
        string.defs[0].output.typed.ty,
        Type::Nominal("String".to_string())
    );
    assert_eq!(
        str_view.defs[0].output.typed.ty,
        Type::Nominal("str".to_string())
    );
    assert_eq!(
        cstr.defs[0].output.typed.ty,
        Type::Nominal("cstr".to_string())
    );
    assert_eq!(
        range.defs[0].output.typed.ty,
        Type::Nominal("Range".to_string())
    );

    assert_eq!(run_wat_text(&slice.backend_link.linked_wat), "slice/3");
    assert_eq!(run_wat_text(&array.backend_link.linked_wat), "slice/3");
    assert_eq!(run_wat_text(&vec.backend_link.linked_wat), "slice/3");
    assert_eq!(run_wat_text(&string.backend_link.linked_wat), "slice/3");
    assert_eq!(run_wat_text(&str_view.backend_link.linked_wat), "slice/3");
    assert_eq!(run_wat_text(&cstr.backend_link.linked_wat), "slice/3");
    assert_eq!(run_wat_text(&range.backend_link.linked_wat), "slice/3");
}

#[test]
fn source_vec_new_push_freeze_index_lowers_to_executable_runtime_imports() {
    let bundle = compile_source_program_bundle("def main(): i64 = Vec.new().push(7).freeze()[0]")
        .expect("compile vec new push freeze index")
        .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::AggregateIndex {
            kind: AggregateKind::Slice,
            ..
        })
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(import \"env\" \"std.vec_new\" (func $std_vec_new (result externref)))"));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_push\" (func $std_vec_push (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_freeze\" (func $std_vec_freeze (param externref) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains("call $std_vec_new"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_push"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_freeze"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "7");
}

#[test]
fn source_vec_new_push_preserves_bool_element_type_through_freeze_index() {
    let bundle =
        compile_source_program_bundle("def main(): bool = Vec.new().push(true).freeze()[0]")
            .expect("compile bool vec new push freeze index")
            .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Bool);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::AggregateIndex {
            kind: AggregateKind::Slice,
            ..
        })
    ));
    assert!(bundle.backend_link.linked_wat.contains("call $std_vec_new"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_push"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_freeze"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn source_vec_push_conflicting_element_type_does_not_keep_i64() {
    let bundle =
        compile_source_program_bundle("def main() = Vec.new().push(1).push(false).freeze()[0]")
            .expect("compile conflicting vec push element type")
            .program;

    assert_eq!(bundle.defs[0].output.typed.ty, Type::Unknown);
}

#[test]
fn source_vec_new_push_freeze_range_slice_lowers_to_executable_runtime_imports() {
    let bundle = compile_source_program_bundle(
        "def main(): i64 = Vec.new().push(5).push(8).freeze()[1..2][0]",
    )
    .expect("compile vec new push freeze range slice")
    .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_i64_slice\" (func $std_slice_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_slice"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "8");
}

#[test]
fn source_vec_new_push_preserves_bool_element_type_through_freeze_range_slice() {
    let bundle = compile_source_program_bundle(
        "def main(): bool = Vec.new().push(false).push(true).freeze()[1..2][0]",
    )
    .expect("compile bool vec new push freeze range slice")
    .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Bool);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.slice_i64_slice\" (func $std_slice_i64_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_slice"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn source_vec_new_push_preserves_string_element_type_through_freeze_index() {
    let bundle = compile_source_program_bundle(
        "def main(): i64 = Vec.new().push(String.from(\"hé\")).freeze()[0].bytes_len()",
    )
    .expect("compile string vec new push freeze index")
    .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.vec_push_externref\" (func $std_vec_push_externref (param externref) (param externref) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_push_externref"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_vec_new_push_preserves_string_element_type_through_freeze_range_slice() {
    let bundle = compile_source_program_bundle(
        "def main(): i64 = Vec.new().push(String.from(\"x\")).push(String.from(\"hé\")).freeze()[1..2][0].bytes_len()",
    )
    .expect("compile string vec new push freeze range slice")
    .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_vec_push_externref"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_slice"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_slice_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_aggregate_and_ref_builtins_can_return_executable_externref_handles() {
    let vec_i64 = compile_source_program_bundle("def main(): Vec[i64] = Vec.new().push(7)")
        .expect("compile vec i64 return")
        .program;
    let vec_string = compile_source_program_bundle(
        "def main(): Vec[String] = Vec.new().push(String.from(\"hé\"))",
    )
    .expect("compile vec string return")
    .program;
    let frozen_string = compile_source_program_bundle(
        "def main(): Slice[String] = Vec.new().push(String.from(\"hé\")).freeze()",
    )
    .expect("compile frozen string vec return")
    .program;
    let slice_slice = compile_source_program_bundle("def main(): Slice[i64] = [1, 2, 3][1..3]")
        .expect("compile slice slice return")
        .program;
    let ref_string =
        compile_source_program_bundle("def main(): Ref[String] = Ref.new(String.from(\"hé\"))")
            .expect("compile ref string return")
            .program;
    let ref_slice =
        compile_source_program_bundle("def main(): Ref[Slice[i64]] = Ref.new([1, 2, 3])")
            .expect("compile ref slice return")
            .program;
    let unsafe_ref_range =
        compile_source_program_bundle("def main(): UnsafeRef[Range] = UnsafeRef.new(3..9)")
            .expect("compile unsafe ref range return")
            .program;

    for bundle in [
        &vec_i64,
        &vec_string,
        &frozen_string,
        &slice_slice,
        &ref_string,
        &ref_slice,
        &unsafe_ref_range,
    ] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("(func $main (export \"main\") (result externref)"));
    }
    assert_eq!(
        vec_i64.defs[0].output.typed.ty,
        Type::Nominal("Vec[i64]".to_string())
    );
    assert_eq!(
        vec_string.defs[0].output.typed.ty,
        Type::Nominal("Vec[String]".to_string())
    );
    assert_eq!(
        frozen_string.defs[0].output.typed.ty,
        Type::Nominal("Slice[String]".to_string())
    );
    assert_eq!(
        slice_slice.defs[0].output.typed.ty,
        Type::Nominal("Slice[i64]".to_string())
    );
    assert_eq!(
        ref_string.defs[0].output.typed.ty,
        Type::Nominal("Ref[String]".to_string())
    );
    assert_eq!(
        ref_slice.defs[0].output.typed.ty,
        Type::Nominal("Ref[Slice[i64]]".to_string())
    );
    assert_eq!(
        unsafe_ref_range.defs[0].output.typed.ty,
        Type::Nominal("UnsafeRef[Range]".to_string())
    );
    assert!(vec_i64
        .backend_link
        .linked_wat
        .contains("call $std_vec_push"));
    assert!(vec_string
        .backend_link
        .linked_wat
        .contains("call $std_vec_push_externref"));
    assert!(frozen_string
        .backend_link
        .linked_wat
        .contains("call $std_vec_freeze"));
    assert!(slice_slice
        .backend_link
        .linked_wat
        .contains("call $std_slice_i64_slice"));
    assert!(ref_string
        .backend_link
        .linked_wat
        .contains("call $std_ref_new_externref"));
    assert!(ref_slice
        .backend_link
        .linked_wat
        .contains("call $std_ref_new_externref"));
    assert!(unsafe_ref_range
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_new_externref"));

    assert_eq!(run_wat_text(&vec_i64.backend_link.linked_wat), "slice/1");
    assert_eq!(run_wat_text(&vec_string.backend_link.linked_wat), "slice/1");
    assert_eq!(
        run_wat_text(&frozen_string.backend_link.linked_wat),
        "slice/1"
    );
    assert_eq!(
        run_wat_text(&slice_slice.backend_link.linked_wat),
        "slice/2"
    );
    assert_eq!(
        run_wat_text(&ref_string.backend_link.linked_wat),
        "ref/bytes/3"
    );
    assert_eq!(
        run_wat_text(&ref_slice.backend_link.linked_wat),
        "ref/slice/3"
    );
    assert_eq!(
        run_wat_text(&unsafe_ref_range.backend_link.linked_wat),
        "ref/range/3/9"
    );
}

fn assert_invalid_assignment_target(source: &str, target_type: &str) {
    let bundle = compile_source_program_bundle(source)
        .expect("compile invalid assignment target")
        .program;

    assert!(
        bundle
            .diagnostics
            .contains(&ProgramDiagnostic::InvalidAssignmentTarget {
                def: "main".to_string(),
                target_type: target_type.to_string(),
            }),
        "{}",
        bundle.render_summary()
    );
    assert_eq!(bundle.defs[0].output.typed.ty, Type::Unknown);
}

#[test]
fn source_assignment_to_immutable_builtin_values_is_program_diagnostic() {
    assert_invalid_assignment_target("def main(s: Slice[i64]): i64 = s := 1", "Slice[i64]");
    assert_invalid_assignment_target("def main(a: Array[i64]): i64 = a := 1", "Array[i64]");
    assert_invalid_assignment_target("def main(v: Vec[i64]): i64 = v := 1", "Vec[i64]");
    assert_invalid_assignment_target("def main(s: str): i64 = s := 1", "str");
    assert_invalid_assignment_target("def main(s: String): i64 = s := 1", "String");
}

#[test]
fn source_assignment_to_builtin_index_results_is_program_diagnostic() {
    assert_invalid_assignment_target("def main(s: Slice[i64]): i64 = s[0] := 9", "i64");
    assert_invalid_assignment_target("def main(a: Array[i64]): i64 = a[0] := 9", "i64");
    assert_invalid_assignment_target("def main(v: Vec[i64]): i64 = v[0] := 9", "i64");
    assert_invalid_assignment_target("def main(s: str): i64 = s[0] := 9", "i64");
    assert_invalid_assignment_target("def main(s: String): i64 = s[0] := 9", "i64");
}

#[test]
fn source_ref_new_set_get_lowers_to_executable_runtime_imports() {
    let bundle = compile_source_program_bundle("def main(): i64 = Ref.new(1).set(4).get()")
        .expect("compile ref new set get")
        .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.ref_new\" (func $std_ref_new (param i32) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.ref_set\" (func $std_ref_set (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.ref_get\" (func $std_ref_get (param externref) (result i32)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains("call $std_ref_new"));
    assert!(bundle.backend_link.linked_wat.contains("call $std_ref_set"));
    assert!(bundle.backend_link.linked_wat.contains("call $std_ref_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "4");
}

#[test]
fn source_ref_deref_sugar_lowers_to_executable_get_runtime_import() {
    let bundle = compile_source_program_bundle("def main(): i64 = Ref.new(9).*")
        .expect("compile ref deref sugar")
        .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains("call $std_ref_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "9");
}

#[test]
fn source_ref_assignment_sugar_lowers_to_executable_set_runtime_import() {
    let bundle = compile_source_program_bundle("def main(): i64 = (Ref.new(1) := 4).*")
        .expect("compile ref assignment sugar")
        .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains("call $std_ref_set"));
    assert!(bundle.backend_link.linked_wat.contains("call $std_ref_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "4");
}

#[test]
fn source_ref_new_preserves_bool_element_type_through_get_and_set() {
    let get = compile_source_program_bundle("def main(): bool = Ref.new(true).get()")
        .expect("compile bool ref get")
        .program;
    let set = compile_source_program_bundle("def main(): bool = (Ref.new(false) := true).*")
        .expect("compile bool ref set")
        .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        let main = &bundle.defs[0].output;
        assert_eq!(main.typed.ty, Type::Bool);
        assert!(bundle.backend_link.linked_wat.contains("call $std_ref_new"));
        assert!(bundle.backend_link.linked_wat.contains("call $std_ref_get"));
    }
    assert!(set.backend_link.linked_wat.contains("call $std_ref_set"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "1");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "1");
}

#[test]
fn source_ref_new_preserves_string_element_type_through_get_and_set() {
    let get = compile_source_program_bundle(
        "def main(): i64 = Ref.new(String.from(\"hé\")).get().bytes_len()",
    )
    .expect("compile string ref get")
    .program;
    let set = compile_source_program_bundle(
        "def main(): i64 = (Ref.new(String.from(\"x\")) := String.from(\"hé\")).*.bytes_len()",
    )
    .expect("compile string ref set")
    .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        let main = &bundle.defs[0].output;
        assert_eq!(main.typed.ty, Type::I64);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_get_externref"));
    }
    assert!(set
        .backend_link
        .linked_wat
        .contains("call $std_ref_set_externref"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "3");
}

#[test]
fn source_ref_new_preserves_text_views_through_get_and_set() {
    let str_view = compile_source_program_bundle(
        "def main(): rune = Ref.new(String.from(\"hé\").as_str()).get().char_at(0)",
    )
    .expect("compile str view ref get")
    .program;
    let cstr_get = compile_source_program_bundle(
        "def main(): i64 = Ref.new(String.from(\"hé\").to_cstr()).get()[3]",
    )
    .expect("compile cstr ref get")
    .program;
    let cstr_set = compile_source_program_bundle(
        "def main(): i64 = (Ref.new(c\"x\") := String.from(\"hé\").to_cstr()).*.bytes_len()",
    )
    .expect("compile cstr ref set")
    .program;

    for bundle in [&str_view, &cstr_get, &cstr_set] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_get_externref"));
    }
    assert!(cstr_set
        .backend_link
        .linked_wat
        .contains("call $std_ref_set_externref"));
    assert_eq!(run_wat_text(&str_view.backend_link.linked_wat), "104");
    assert_eq!(run_wat_text(&cstr_get.backend_link.linked_wat), "0");
    assert_eq!(run_wat_text(&cstr_set.backend_link.linked_wat), "3");
}

#[test]
fn source_ref_new_preserves_vec_string_value_through_get_and_set() {
    let get = compile_source_program_bundle(
        "def main(): i64 = Ref.new(Vec.new().push(String.from(\"x\"))).get().push(String.from(\"hé\")).freeze()[1].bytes_len()",
    )
    .expect("compile string vec ref get")
    .program;
    let set = compile_source_program_bundle(
        "def main(): rune = (Ref.new(Vec.new().push(String.from(\"x\"))) := Vec.new().push(String.from(\"hi\"))).*.freeze()[0].char_at(0)",
    )
    .expect("compile string vec ref set")
    .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_get_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_vec_push_externref"));
    }
    assert!(set
        .backend_link
        .linked_wat
        .contains("call $std_ref_set_externref"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "104");
}

#[test]
fn source_ref_new_preserves_slice_string_value_through_get_and_set() {
    let get = compile_source_program_bundle(
        "def main(): i64 = Ref.new([String.from(\"hé\")]).get()[0].bytes_len()",
    )
    .expect("compile string slice ref get")
    .program;
    let set = compile_source_program_bundle(
        "def main(): rune = (Ref.new([String.from(\"x\")]) := [String.from(\"hi\")]).*[0].char_at(0)",
    )
    .expect("compile string slice ref set")
    .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_get_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_slice_get"));
    }
    assert!(set
        .backend_link
        .linked_wat
        .contains("call $std_ref_set_externref"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "104");
}

#[test]
fn source_ref_new_preserves_range_value_through_get_and_set() {
    let get = compile_source_program_bundle("def main(): i64 = Ref.new(3..9).get().end")
        .expect("compile range ref get")
        .program;
    let set = compile_source_program_bundle("def main(): i64 = (Ref.new(1..2) := 5..8).*.start")
        .expect("compile range ref set")
        .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_ref_get_externref"));
    }
    assert!(set
        .backend_link
        .linked_wat
        .contains("call $std_ref_set_externref"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "9");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "5");
}

#[test]
fn source_assignment_to_non_ref_is_program_diagnostic() {
    let bundle = compile_source_program_bundle("def main(): i64 = 1 := 4")
        .expect("compile invalid assignment target")
        .program;

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::InvalidAssignmentTarget {
            def: "main".to_string(),
            target_type: "i64".to_string(),
        }));
    assert_eq!(bundle.defs[0].output.typed.ty, Type::Unknown);
}

#[test]
fn source_unsafe_ref_new_set_get_lowers_to_executable_runtime_imports() {
    let bundle = compile_source_program_bundle("def main(): i64 = (UnsafeRef.new(2) := 8).*")
        .expect("compile unsafe ref new set get")
        .program;

    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.unsafe_ref_new\" (func $std_unsafe_ref_new (param i32) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.unsafe_ref_set\" (func $std_unsafe_ref_set (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.unsafe_ref_get\" (func $std_unsafe_ref_get (param externref) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_new"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_set"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_get"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "8");
}

#[test]
fn source_unsafe_ref_new_preserves_bool_element_type_through_get_and_set() {
    let get = compile_source_program_bundle("def main(): bool = UnsafeRef.new(true).get()")
        .expect("compile bool unsafe ref get")
        .program;
    let set = compile_source_program_bundle("def main(): bool = (UnsafeRef.new(false) := true).*")
        .expect("compile bool unsafe ref set")
        .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        let main = &bundle.defs[0].output;
        assert_eq!(main.typed.ty, Type::Bool);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_new"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_get"));
    }
    assert!(set
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_set"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "1");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "1");
}

#[test]
fn source_unsafe_ref_new_preserves_string_element_type_through_get_and_set() {
    let get = compile_source_program_bundle(
        "def main(): i64 = UnsafeRef.new(String.from(\"hé\")).get().bytes_len()",
    )
    .expect("compile string unsafe ref get")
    .program;
    let set = compile_source_program_bundle(
        "def main(): i64 = (UnsafeRef.new(String.from(\"x\")) := String.from(\"hé\")).*.bytes_len()",
    )
    .expect("compile string unsafe ref set")
    .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        let main = &bundle.defs[0].output;
        assert_eq!(main.typed.ty, Type::I64);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_get_externref"));
    }
    assert!(set
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_set_externref"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "3");
}

#[test]
fn source_unsafe_ref_new_preserves_text_views_through_get_and_set() {
    let str_view = compile_source_program_bundle(
        "def main(): rune = UnsafeRef.new(String.from(\"hé\").as_str()).get().char_at(0)",
    )
    .expect("compile str view unsafe ref get")
    .program;
    let cstr_get = compile_source_program_bundle(
        "def main(): i64 = UnsafeRef.new(String.from(\"hé\").to_cstr()).get()[3]",
    )
    .expect("compile cstr unsafe ref get")
    .program;
    let cstr_set = compile_source_program_bundle(
        "def main(): i64 = (UnsafeRef.new(c\"x\") := String.from(\"hé\").to_cstr()).*.bytes_len()",
    )
    .expect("compile cstr unsafe ref set")
    .program;

    for bundle in [&str_view, &cstr_get, &cstr_set] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_get_externref"));
    }
    assert!(cstr_set
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_set_externref"));
    assert_eq!(run_wat_text(&str_view.backend_link.linked_wat), "104");
    assert_eq!(run_wat_text(&cstr_get.backend_link.linked_wat), "0");
    assert_eq!(run_wat_text(&cstr_set.backend_link.linked_wat), "3");
}

#[test]
fn source_unsafe_ref_new_preserves_vec_string_value_through_get_and_set() {
    let get = compile_source_program_bundle(
        "def main(): i64 = UnsafeRef.new(Vec.new().push(String.from(\"x\"))).get().push(String.from(\"hé\")).freeze()[1].bytes_len()",
    )
    .expect("compile string vec unsafe ref get")
    .program;
    let set = compile_source_program_bundle(
        "def main(): rune = (UnsafeRef.new(Vec.new().push(String.from(\"x\"))) := Vec.new().push(String.from(\"hi\"))).*.freeze()[0].char_at(0)",
    )
    .expect("compile string vec unsafe ref set")
    .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_get_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_vec_push_externref"));
    }
    assert!(set
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_set_externref"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "104");
}

#[test]
fn source_unsafe_ref_new_preserves_slice_string_value_through_get_and_set() {
    let get = compile_source_program_bundle(
        "def main(): i64 = UnsafeRef.new([String.from(\"hé\")]).get()[0].bytes_len()",
    )
    .expect("compile string slice unsafe ref get")
    .program;
    let set = compile_source_program_bundle(
        "def main(): rune = (UnsafeRef.new([String.from(\"x\")]) := [String.from(\"hi\")]).*[0].char_at(0)",
    )
    .expect("compile string slice unsafe ref set")
    .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_get_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_slice_get"));
    }
    assert!(set
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_set_externref"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "104");
}

#[test]
fn source_unsafe_ref_new_preserves_range_value_through_get_and_set() {
    let get = compile_source_program_bundle("def main(): i64 = UnsafeRef.new(3..9).get().end")
        .expect("compile range unsafe ref get")
        .program;
    let set =
        compile_source_program_bundle("def main(): i64 = (UnsafeRef.new(1..2) := 5..8).*.start")
            .expect("compile range unsafe ref set")
            .program;

    for bundle in [&get, &set] {
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_new_externref"));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_unsafe_ref_get_externref"));
    }
    assert!(set
        .backend_link
        .linked_wat
        .contains("call $std_unsafe_ref_set_externref"));
    assert_eq!(run_wat_text(&get.backend_link.linked_wat), "9");
    assert_eq!(run_wat_text(&set.backend_link.linked_wat), "5");
}

#[test]
fn source_str_param_len_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: str): i64 = s.len")
        .expect("compile str param len")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.str_i64_len\" (func $std_str_i64_len (param externref) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main (export \"main\") (param $s externref) (result i32)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_str_i64_len"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_str_param_len_methods_lower_to_executable_runtime_import() {
    let len = compile_source_program_bundle("def main(s: str): i64 = s.len()")
        .expect("compile str param len method")
        .program;
    let bytes_len = compile_source_program_bundle("def main(s: str): i64 = s.bytes_len()")
        .expect("compile str param bytes_len method")
        .program;

    for bundle in [&len, &bytes_len] {
        assert!(bundle
            .diagnostics
            .contains(&ProgramDiagnostic::EntryHasParams {
                name: "main".to_string(),
                params: vec!["s".to_string()]
            }));
        assert_backend_link_clean(bundle, 0);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.str_i64_len\" (func $std_str_i64_len (param externref) (result i32)))"
        ));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_str_i64_len"));
        assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
    }
}

#[test]
fn source_str_param_index_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: str): i64 = s[1]")
        .expect("compile str param index")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::TextIndex {
            kind: chiba_level1r::typed::TextKind::Str,
            ..
        })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.str_i64_byte_at\" (func $std_str_i64_byte_at (param externref) (param i32) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_str_i64_byte_at"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_str_param_range_slice_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: str): i64 = s[1..3].bytes_len()")
        .expect("compile str param range slice")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.str_slice\" (func $std_str_slice (param externref) (param i32) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_str_slice"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_str_param_rune_methods_lower_to_executable_runtime_import() {
    let rune_len = compile_source_program_bundle("def main(s: str): i64 = s.rune_len()")
        .expect("compile str param rune_len")
        .program;
    let char_at = compile_source_program_bundle("def main(s: str): rune = s.char_at(0)")
        .expect("compile str param char_at")
        .program;

    for bundle in [&rune_len, &char_at] {
        assert_eq!(
            bundle.diagnostics,
            vec![ProgramDiagnostic::EntryHasParams {
                name: "main".to_string(),
                params: vec!["s".to_string()]
            }]
        );
        assert_backend_link_clean(bundle, 0);
    }
    assert!(rune_len.backend_link.linked_wat.contains(
        "(import \"env\" \"std.str_rune_len\" (func $std_str_rune_len (param externref) (result i32)))"
    ));
    assert!(char_at.backend_link.linked_wat.contains(
        "(import \"env\" \"std.str_char_at\" (func $std_str_char_at (param externref) (param i32) (result i32)))"
    ));

    assert_eq!(run_wat_text(&rune_len.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&char_at.backend_link.linked_wat), "1");
}

#[test]
fn source_string_param_len_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: String): i64 = s.len")
        .expect("compile string param len")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.string_i64_len\" (func $std_string_i64_len (param externref) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main (export \"main\") (param $s externref) (result i32)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_string_i64_len"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn source_string_param_len_methods_lower_to_executable_runtime_import() {
    let len = compile_source_program_bundle("def main(s: String): i64 = s.len()")
        .expect("compile string param len method")
        .program;
    let bytes_len = compile_source_program_bundle("def main(s: String): i64 = s.bytes_len()")
        .expect("compile string param bytes_len method")
        .program;

    for bundle in [&len, &bytes_len] {
        assert!(bundle
            .diagnostics
            .contains(&ProgramDiagnostic::EntryHasParams {
                name: "main".to_string(),
                params: vec!["s".to_string()]
            }));
        assert_backend_link_clean(bundle, 0);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.string_i64_len\" (func $std_string_i64_len (param externref) (result i32)))"
        ));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_string_i64_len"));
        assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
    }
}

#[test]
fn source_string_param_index_lowers_to_executable_runtime_import() {
    let bundle = compile_source_program_bundle("def main(s: String): i64 = s[1]")
        .expect("compile string param index")
        .program;

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["s".to_string()]
        }]
    );
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(matches!(
        main.backend.return_value,
        Some(chiba_level1r::core::CoreValue::TextIndex {
            kind: chiba_level1r::typed::TextKind::String,
            ..
        })
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.string_i64_byte_at\" (func $std_string_i64_byte_at (param externref) (param i32) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_string_i64_byte_at"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_string_param_range_slice_lowers_to_executable_runtime_import() {
    let len = compile_source_program_bundle("def main(s: String): i64 = s[1..3].bytes_len()")
        .expect("compile string param range slice len")
        .program;
    let byte = compile_source_program_bundle("def main(s: String): i64 = s[1..3][0]")
        .expect("compile string param range slice byte")
        .program;

    for bundle in [&len, &byte] {
        assert_eq!(
            bundle.diagnostics,
            vec![ProgramDiagnostic::EntryHasParams {
                name: "main".to_string(),
                params: vec!["s".to_string()]
            }]
        );
        assert_backend_link_clean(bundle, 0);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.string_slice\" (func $std_string_slice (param externref) (param i32) (param i32) (result externref)))"
        ));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_string_slice"));
    }
    assert_eq!(len.defs[0].output.typed.ty, Type::I64);
    assert_eq!(byte.defs[0].output.typed.ty, Type::I64);
    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "2");
    assert_eq!(run_wat_text(&byte.backend_link.linked_wat), "2");
}

#[test]
fn source_string_param_rune_methods_lower_to_executable_runtime_import() {
    let rune_len = compile_source_program_bundle("def main(s: String): i64 = s.rune_len()")
        .expect("compile string param rune_len")
        .program;
    let char_at = compile_source_program_bundle("def main(s: String): rune = s.char_at(0)")
        .expect("compile string param char_at")
        .program;

    for bundle in [&rune_len, &char_at] {
        assert_eq!(
            bundle.diagnostics,
            vec![ProgramDiagnostic::EntryHasParams {
                name: "main".to_string(),
                params: vec!["s".to_string()]
            }]
        );
        assert_backend_link_clean(bundle, 0);
    }
    assert!(rune_len.backend_link.linked_wat.contains(
        "(import \"env\" \"std.string_rune_len\" (func $std_string_rune_len (param externref) (result i32)))"
    ));
    assert!(char_at.backend_link.linked_wat.contains(
        "(import \"env\" \"std.string_char_at\" (func $std_string_char_at (param externref) (param i32) (result i32)))"
    ));

    assert_eq!(run_wat_text(&rune_len.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&char_at.backend_link.linked_wat), "1");
}

#[test]
fn source_string_literal_index_keeps_utf8_byte_semantics() {
    let bundle = compile_source_program_bundle("def main(): i64 = \"é\"[0]")
        .expect("compile string literal byte index")
        .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_string_i64_byte_at"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "195");
}

#[test]
fn source_string_literal_range_slice_keeps_utf8_byte_semantics() {
    let len = compile_source_program_bundle("def main(): i64 = \"éx\"[0..2].bytes_len()")
        .expect("compile string literal range slice len")
        .program;
    let byte = compile_source_program_bundle("def main(): i64 = \"éx\"[0..2][0]")
        .expect("compile string literal range slice byte")
        .program;

    for bundle in [&len, &byte] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean(bundle, 0);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_string_slice"));
    }
    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "2");
    assert_eq!(run_wat_text(&byte.backend_link.linked_wat), "195");
}

#[test]
fn source_raw_string_literal_keeps_escape_and_interpolation_text() {
    let len = compile_source_program_bundle("def main(): i64 = r\"\\n${x}\".bytes_len()")
        .expect("compile raw string literal len")
        .program;
    let quote = compile_source_program_bundle("def main(): i64 = r#\"say \"hi\"\"#[4]")
        .expect("compile raw hash string literal quote byte")
        .program;

    for bundle in [&len, &quote] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean(bundle, 0);
        assert!(
            bundle
                .backend_link
                .linked_wat
                .contains("call $std_string_i64_byte_at")
                || bundle
                    .backend_link
                    .linked_wat
                    .contains("call $std_string_i64_len")
        );
    }
    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "6");
    assert_eq!(run_wat_text(&quote.backend_link.linked_wat), "34");
}

#[test]
fn source_string_from_lowers_to_executable_owned_text_runtime_import() {
    let bundle = compile_source_program_bundle("def main(): i64 = String.from(\"é\").bytes_len()")
        .expect("compile string from bytes_len")
        .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.str_to_string\" (func $std_str_to_string (param externref) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_str_to_string"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_string_i64_len"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_text_builtins_can_return_executable_externref_handles() {
    let owned = compile_source_program_bundle("def main(): String = String.from(\"hé\")")
        .expect("compile owned string return")
        .program;
    let concat = compile_source_program_bundle("def main(): String = \"h\".concat(\"é\")")
        .expect("compile string concat return")
        .program;
    let built = compile_source_program_bundle("def main(): String = String.new().push_rune('é')")
        .expect("compile string builder return")
        .program;
    let view = compile_source_program_bundle("def main(): str = String.from(\"hé\").as_str()")
        .expect("compile str view return")
        .program;
    let string_slice = compile_source_program_bundle("def main(): String = \"héx\"[1..3]")
        .expect("compile string slice return")
        .program;
    let cstr = compile_source_program_bundle("def main(): cstr = String.from(\"hé\").to_cstr()")
        .expect("compile cstr return")
        .program;
    let cstr_slice = compile_source_program_bundle("def main(): cstr = c\"héx\"[1..3]")
        .expect("compile cstr slice return")
        .program;

    for bundle in [
        &owned,
        &concat,
        &built,
        &view,
        &string_slice,
        &cstr,
        &cstr_slice,
    ] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("(func $main (export \"main\") (result externref)"));
    }
    assert_eq!(
        owned.defs[0].output.typed.ty,
        Type::Nominal("String".to_string())
    );
    assert_eq!(
        concat.defs[0].output.typed.ty,
        Type::Nominal("String".to_string())
    );
    assert_eq!(
        built.defs[0].output.typed.ty,
        Type::Nominal("String".to_string())
    );
    assert_eq!(
        view.defs[0].output.typed.ty,
        Type::Nominal("str".to_string())
    );
    assert_eq!(
        string_slice.defs[0].output.typed.ty,
        Type::Nominal("String".to_string())
    );
    assert_eq!(
        cstr.defs[0].output.typed.ty,
        Type::Nominal("cstr".to_string())
    );
    assert_eq!(
        cstr_slice.defs[0].output.typed.ty,
        Type::Nominal("cstr".to_string())
    );
    assert!(owned
        .backend_link
        .linked_wat
        .contains("call $std_str_to_string"));
    assert!(concat
        .backend_link
        .linked_wat
        .contains("call $std_string_concat"));
    assert!(built
        .backend_link
        .linked_wat
        .contains("call $std_string_push_rune"));
    assert!(view
        .backend_link
        .linked_wat
        .contains("call $std_string_as_str"));
    assert!(string_slice
        .backend_link
        .linked_wat
        .contains("call $std_string_slice"));
    assert!(cstr
        .backend_link
        .linked_wat
        .contains("call $std_string_to_cstr"));
    assert!(cstr_slice
        .backend_link
        .linked_wat
        .contains("call $std_cstr_slice"));

    assert_eq!(run_wat_text(&owned.backend_link.linked_wat), "bytes/3");
    assert_eq!(run_wat_text(&concat.backend_link.linked_wat), "bytes/3");
    assert_eq!(run_wat_text(&built.backend_link.linked_wat), "bytes/2");
    assert_eq!(run_wat_text(&view.backend_link.linked_wat), "bytes/3");
    assert_eq!(
        run_wat_text(&string_slice.backend_link.linked_wat),
        "bytes/2"
    );
    assert_eq!(run_wat_text(&cstr.backend_link.linked_wat), "cstr/4");
    assert_eq!(run_wat_text(&cstr_slice.backend_link.linked_wat), "cstr/3");
}

#[test]
fn source_string_as_str_lowers_to_executable_view_runtime_import() {
    let len =
        compile_source_program_bundle("def main(): i64 = String.from(\"hé\").as_str().bytes_len()")
            .expect("compile string as_str len")
            .program;
    let byte = compile_source_program_bundle("def main(): i64 = String.from(\"hé\").as_str()[1]")
        .expect("compile string as_str byte")
        .program;
    let runes =
        compile_source_program_bundle("def main(): i64 = String.from(\"hé\").as_str().rune_len()")
            .expect("compile string as_str rune_len")
            .program;

    for bundle in [&len, &byte, &runes] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean_all(bundle);
        assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.string_as_str\" (func $std_string_as_str (param externref) (result externref)))"
        ));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_string_as_str"));
    }
    assert!(len.backend_link.linked_wat.contains(
        "(import \"env\" \"std.str_i64_len\" (func $std_str_i64_len (param externref) (result i32)))"
    ));
    assert!(byte.backend_link.linked_wat.contains(
        "(import \"env\" \"std.str_i64_byte_at\" (func $std_str_i64_byte_at (param externref) (param i32) (result i32)))"
    ));
    assert!(runes.backend_link.linked_wat.contains(
        "(import \"env\" \"std.str_rune_len\" (func $std_str_rune_len (param externref) (result i32)))"
    ));

    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&byte.backend_link.linked_wat), "195");
    assert_eq!(run_wat_text(&runes.backend_link.linked_wat), "2");
}

#[test]
fn source_string_interpolation_lowers_to_executable_concat_runtime_imports() {
    let len =
        compile_source_program_bundle("def main(name: str): i64 = \"a ${name} b\".bytes_len()")
            .expect("compile string interpolation len")
            .program;
    let byte = compile_source_program_bundle("def main(name: str): i64 = \"a ${name} b\"[2]")
        .expect("compile string interpolation byte")
        .program;

    for bundle in [&len, &byte] {
        assert_eq!(
            bundle.diagnostics,
            vec![ProgramDiagnostic::EntryHasParams {
                name: "main".to_string(),
                params: vec!["name".to_string()]
            }]
        );
        assert_backend_link_clean_all(bundle);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.string_concat\" (func $std_string_concat (param externref) (param externref) (result externref)))"
        ));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_string_concat"));
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.str_to_string\" (func $std_str_to_string (param externref) (result externref)))"
        ));
    }
    assert_eq!(len.defs[0].output.typed.ty, Type::I64);
    assert_eq!(byte.defs[0].output.typed.ty, Type::I64);
    assert!(byte.backend_link.linked_wat.contains(
        "(import \"env\" \"std.string_i64_byte_at\" (func $std_string_i64_byte_at (param externref) (param i32) (result i32)))"
    ));
    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "7");
    assert_eq!(run_wat_text(&byte.backend_link.linked_wat), "1");
}

#[test]
fn source_string_concat_lowers_to_executable_runtime_import() {
    let len = compile_source_program_bundle("def main(): i64 = \"a\".concat(\"é\").bytes_len()")
        .expect("compile string concat len")
        .program;
    let byte = compile_source_program_bundle("def main(): i64 = \"a\".concat(\"é\")[1]")
        .expect("compile string concat byte")
        .program;
    let rune = compile_source_program_bundle("def main(): rune = \"a\".concat(\"é\").char_at(1)")
        .expect("compile string concat char_at")
        .program;

    for bundle in [&len, &byte, &rune] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean_all(bundle);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.string_concat\" (func $std_string_concat (param externref) (param externref) (result externref)))"
        ));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_string_concat"));
    }
    assert_eq!(len.defs[0].output.typed.ty, Type::I64);
    assert_eq!(byte.defs[0].output.typed.ty, Type::I64);
    assert_eq!(rune.defs[0].output.typed.ty, Type::Rune);
    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&byte.backend_link.linked_wat), "195");
    assert_eq!(run_wat_text(&rune.backend_link.linked_wat), "233");
}

#[test]
fn source_string_from_keeps_byte_index_and_char_at_semantics() {
    let byte = compile_source_program_bundle("def main(): i64 = String.from(\"é\")[0]")
        .expect("compile string from byte index")
        .program;
    let rune = compile_source_program_bundle("def main(): rune = String.from(\"é\").char_at(0)")
        .expect("compile string from char_at")
        .program;

    assert_eq!(byte.diagnostics, vec![]);
    assert_eq!(rune.diagnostics, vec![]);
    assert_backend_link_clean_all(&byte);
    assert_backend_link_clean_all(&rune);
    assert_eq!(byte.defs[0].output.typed.ty, Type::I64);
    assert_eq!(rune.defs[0].output.typed.ty, Type::Rune);

    assert_eq!(run_wat_text(&byte.backend_link.linked_wat), "195");
    assert_eq!(run_wat_text(&rune.backend_link.linked_wat), "233");
}

#[test]
fn source_text_rune_len_lowers_to_executable_runtime_import() {
    let string_runes = compile_source_program_bundle("def main(): i64 = \"éx\".rune_len()")
        .expect("compile string rune_len")
        .program;
    let string_bytes = compile_source_program_bundle("def main(): i64 = \"éx\".bytes_len()")
        .expect("compile string bytes_len")
        .program;
    let owned_runes =
        compile_source_program_bundle("def main(): i64 = String.from(\"你好a\").rune_len()")
            .expect("compile owned string rune_len")
            .program;
    let cstr_runes = compile_source_program_bundle("def main(): i64 = c\"hé\".rune_len()")
        .expect("compile cstr rune_len")
        .program;

    for bundle in [&string_runes, &owned_runes, &cstr_runes] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean_all(bundle);
        assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
    }
    assert!(string_runes.backend_link.linked_wat.contains(
        "(import \"env\" \"std.string_rune_len\" (func $std_string_rune_len (param externref) (result i32)))"
    ));
    assert!(owned_runes.backend_link.linked_wat.contains(
        "(import \"env\" \"std.string_rune_len\" (func $std_string_rune_len (param externref) (result i32)))"
    ));
    assert!(cstr_runes.backend_link.linked_wat.contains(
        "(import \"env\" \"std.cstr_rune_len\" (func $std_cstr_rune_len (param externref) (result i32)))"
    ));

    assert_eq!(run_wat_text(&string_runes.backend_link.linked_wat), "2");
    assert_eq!(run_wat_text(&string_bytes.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&owned_runes.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&cstr_runes.backend_link.linked_wat), "2");
}

#[test]
fn source_string_to_cstr_lowers_to_executable_abi_text_runtime_import() {
    let len = compile_source_program_bundle(
        "def main(): i64 = String.from(\"hé\").to_cstr().bytes_len()",
    )
    .expect("compile string to_cstr len")
    .program;
    let nul = compile_source_program_bundle("def main(): i64 = String.from(\"hé\").to_cstr()[3]")
        .expect("compile string to_cstr nul byte")
        .program;

    for bundle in [&len, &nul] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean_all(bundle);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.string_to_cstr\" (func $std_string_to_cstr (param externref) (result externref)))"
        ));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_string_to_cstr"));
    }
    assert_eq!(len.defs[0].output.typed.ty, Type::I64);
    assert_eq!(nul.defs[0].output.typed.ty, Type::I64);
    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&nul.backend_link.linked_wat), "0");
}

#[test]
fn source_cstr_range_slice_lowers_to_executable_runtime_import() {
    let literal_len = compile_source_program_bundle("def main(): i64 = c\"héx\"[1..3].bytes_len()")
        .expect("compile cstr literal range slice len")
        .program;
    let literal_nul = compile_source_program_bundle("def main(): i64 = c\"héx\"[1..3][2]")
        .expect("compile cstr literal range slice nul")
        .program;
    let converted_byte =
        compile_source_program_bundle("def main(): i64 = String.from(\"héx\").to_cstr()[1..3][0]")
            .expect("compile string to cstr range slice byte")
            .program;
    let param_len = compile_source_program_bundle("def main(s: cstr): i64 = s[0..2].bytes_len()")
        .expect("compile cstr param range slice len")
        .program;

    for bundle in [&literal_len, &literal_nul, &converted_byte, &param_len] {
        if bundle.defs[0].params.is_empty() {
            assert_eq!(bundle.diagnostics, vec![]);
        } else {
            assert_eq!(
                bundle.diagnostics,
                vec![ProgramDiagnostic::EntryHasParams {
                    name: "main".to_string(),
                    params: vec!["s".to_string()]
                }]
            );
        }
        assert_backend_link_clean_all(bundle);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.cstr_slice\" (func $std_cstr_slice (param externref) (param i32) (param i32) (result externref)))"
        ));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("call $std_cstr_slice"));
    }
    assert_eq!(literal_len.defs[0].output.typed.ty, Type::I64);
    assert_eq!(converted_byte.defs[0].output.typed.ty, Type::I64);
    assert_eq!(run_wat_text(&literal_len.backend_link.linked_wat), "2");
    assert_eq!(run_wat_text(&literal_nul.backend_link.linked_wat), "0");
    assert_eq!(run_wat_text(&converted_byte.backend_link.linked_wat), "195");
    assert_eq!(run_wat_text(&param_len.backend_link.linked_wat), "2");
}

#[test]
fn source_cstr_literal_lowers_to_executable_abi_text_runtime_import() {
    let len = compile_source_program_bundle("def main(): i64 = c\"hé\".bytes_len()")
        .expect("compile cstr literal len")
        .program;
    let nul = compile_source_program_bundle("def main(): i64 = c\"hé\"[3]")
        .expect("compile cstr literal nul byte")
        .program;

    for bundle in [&len, &nul] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean_all(bundle);
        assert!(bundle.backend_link.linked_wat.contains(
            "(import \"env\" \"std.cstr_literal_3\" (func $std_cstr_literal_3 (param i32) (param i32) (param i32) (result externref)))"
        ));
    }
    assert!(len.backend_link.linked_wat.contains(
        "(import \"env\" \"std.cstr_i64_len\" (func $std_cstr_i64_len (param externref) (result i32)))"
    ));
    assert!(nul.backend_link.linked_wat.contains(
        "(import \"env\" \"std.cstr_i64_byte_at\" (func $std_cstr_i64_byte_at (param externref) (param i32) (result i32)))"
    ));

    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&nul.backend_link.linked_wat), "0");
}

#[test]
fn source_cstr_param_methods_lower_to_executable_runtime_import() {
    let len = compile_source_program_bundle("def main(s: cstr): i64 = s.bytes_len()")
        .expect("compile cstr param bytes_len")
        .program;
    let byte = compile_source_program_bundle("def main(s: cstr): i64 = s[1]")
        .expect("compile cstr param byte index")
        .program;
    let rune_len = compile_source_program_bundle("def main(s: cstr): i64 = s.rune_len()")
        .expect("compile cstr param rune_len")
        .program;
    let char_at = compile_source_program_bundle("def main(s: cstr): rune = s.char_at(0)")
        .expect("compile cstr param char_at")
        .program;

    for bundle in [&len, &byte, &rune_len, &char_at] {
        assert_eq!(
            bundle.diagnostics,
            vec![ProgramDiagnostic::EntryHasParams {
                name: "main".to_string(),
                params: vec!["s".to_string()]
            }]
        );
        assert_backend_link_clean(bundle, 0);
    }
    assert!(len.backend_link.linked_wat.contains(
        "(import \"env\" \"std.cstr_i64_len\" (func $std_cstr_i64_len (param externref) (result i32)))"
    ));
    assert!(byte.backend_link.linked_wat.contains(
        "(import \"env\" \"std.cstr_i64_byte_at\" (func $std_cstr_i64_byte_at (param externref) (param i32) (result i32)))"
    ));
    assert!(rune_len.backend_link.linked_wat.contains(
        "(import \"env\" \"std.cstr_rune_len\" (func $std_cstr_rune_len (param externref) (result i32)))"
    ));
    assert!(char_at.backend_link.linked_wat.contains(
        "(import \"env\" \"std.cstr_char_at\" (func $std_cstr_char_at (param externref) (param i32) (result i32)))"
    ));

    assert_eq!(run_wat_text(&len.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&byte.backend_link.linked_wat), "2");
    assert_eq!(run_wat_text(&rune_len.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&char_at.backend_link.linked_wat), "1");
}

#[test]
fn source_string_char_at_returns_rune() {
    let bundle = compile_source_program_bundle("def main(): rune = \"é\".char_at(0)")
        .expect("compile string char_at")
        .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Rune);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.string_char_at\" (func $std_string_char_at (param externref) (param i32) (result i32)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_string_char_at"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "233");
}

#[test]
fn source_string_new_push_rune_len_lowers_to_executable_runtime_imports() {
    let bundle = compile_source_program_bundle("def main(): i64 = String.new().push_rune('é').len")
        .expect("compile string builder push_rune len")
        .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(import \"env\" \"std.string_new\" (func $std_string_new (result externref)))"));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.string_push_rune\" (func $std_string_push_rune (param externref) (param i32) (result externref)))"
    ));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_string_new"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_string_push_rune"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_string_new_push_rune_bytes_len_method_lowers_to_executable_runtime_imports() {
    let bundle =
        compile_source_program_bundle("def main(): i64 = String.new().push_rune('é').bytes_len()")
            .expect("compile string builder push_rune bytes_len")
            .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::I64);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_string_push_rune"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("call $std_string_i64_len"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn source_string_new_push_rune_keeps_byte_index_and_char_at_semantics() {
    let byte = compile_source_program_bundle("def main(): i64 = String.new().push_rune('é')[0]")
        .expect("compile string builder push_rune byte index")
        .program;
    let rune =
        compile_source_program_bundle("def main(): rune = String.new().push_rune('é').char_at(0)")
            .expect("compile string builder push_rune char_at")
            .program;

    assert_eq!(byte.diagnostics, vec![]);
    assert_eq!(rune.diagnostics, vec![]);
    assert_backend_link_clean_all(&byte);
    assert_backend_link_clean_all(&rune);
    assert_eq!(byte.defs[0].output.typed.ty, Type::I64);
    assert_eq!(rune.defs[0].output.typed.ty, Type::Rune);

    assert_eq!(run_wat_text(&byte.backend_link.linked_wat), "195");
    assert_eq!(run_wat_text(&rune.backend_link.linked_wat), "233");
}

#[test]
fn source_rune_literal_lowers_to_executable_scalar() {
    let bundle = compile_source_program_bundle("def main(): rune = 'é'")
        .expect("compile rune literal")
        .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    let main = &bundle.defs[0].output;
    assert_eq!(main.typed.ty, Type::Rune);

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "233");
}

#[test]
fn program_record_update_field_return_lowers_to_executable_wat_value() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::field(
            Expr::record_update(
                Expr::record(vec![("x", Expr::i64(4)), ("y", Expr::bool(true))]),
                vec![("x", Expr::i64(9))],
            ),
            "x",
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnValue(
                chiba_level1r::core::CoreValue::RecordField { record, field }
            ) if field == "x"
                && matches!(
                    record.as_ref(),
                    chiba_level1r::core::CoreValue::Record { fields }
                        if fields.iter().any(|field|
                            field.name == "x"
                                && field.value == chiba_level1r::core::CoreValue::I64(9)
                        )
                        && fields.iter().any(|field|
                            field.name == "y"
                                && field.value == chiba_level1r::core::CoreValue::Bool(true)
                        )
                )
        )
    }));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "9");
}

#[test]
fn program_zero_arg_tailcall_lowers_to_executable_direct_call_wat() {
    let program = SourceProgram::new(vec![
        def("helper", vec![], Expr::i64(7)),
        def(
            "main",
            vec![],
            Expr::call_args(Expr::var("helper"), Vec::new()),
        ),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    assert!(bundle.defs[1].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "helper" && args.is_empty()
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; tailcall helper args=[]"));
    assert!(bundle.backend_link.linked_wat.contains("call $helper"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "7");
}

#[test]
fn program_literal_arg_tailcall_lowers_to_executable_direct_call_wat() {
    let program = SourceProgram::new(vec![
        def("helper", vec!["x"], Expr::var("x")),
        def(
            "main",
            vec![],
            Expr::call_args(Expr::var("helper"), vec![Expr::i64(9)]),
        ),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    assert!(bundle.defs[1].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "helper"
                    && args == &vec![chiba_level1r::core::CoreValue::I64(9)]
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $helper (param $x i32) (result i32)"));
    assert!(bundle.backend_link.linked_wat.contains("i32.const 9"));
    assert!(bundle.backend_link.linked_wat.contains("call $helper"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "9");
}

#[test]
fn source_pipe_default_call_lowers_to_executable_direct_call_wat() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def id(x: i64): i64 = x
def main(): i64 = 1 |> id",
    )
    .expect("compile source");
    let bundle = &output.program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    assert!(bundle.defs[1].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "id"
                    && args == &vec![chiba_level1r::core::CoreValue::I64(1)]
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains("call $id"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn source_pipe_placeholder_reuses_input_in_executable_wat() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def add(x: i64, y: i64): i64 = x + y
def main(): i64 = 2 |> add(_, _)",
    )
    .expect("compile source");
    let bundle = &output.program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    assert!(bundle.defs[1].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::TailCall { func, args }
                if func == "add"
                    && args == &vec![
                        chiba_level1r::core::CoreValue::I64(2),
                        chiba_level1r::core::CoreValue::I64(2)
                    ]
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains("call $add"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "4");
}

#[test]
fn source_arithmetic_operators_lower_to_executable_intrinsic_wat() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def sub(x: i64, y: i64): i64 = x - y
def mul(x: i64, y: i64): i64 = x * y
def div(x: i64, y: i64): i64 = x / y
def main(): i64 = div(mul(sub(10, 4), 3), 2)",
    )
    .expect("compile source");
    let bundle = &output.program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(bundle);
    assert!(bundle.backend_link.linked_wat.contains("i32.sub"));
    assert!(bundle.backend_link.linked_wat.contains("i32.mul"));
    assert!(bundle.backend_link.linked_wat.contains("i32.div_s"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "9");
}

#[test]
fn program_param_branch_lowers_to_executable_wat() {
    let program = SourceProgram::new(vec![
        def(
            "helper",
            vec!["x"],
            Expr::if_else(Expr::var("x"), Expr::i64(11), Expr::i64(22)),
        ),
        def(
            "main",
            vec![],
            Expr::call_args(Expr::var("helper"), vec![Expr::i64(0)]),
        ),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnBranch {
                cond: chiba_level1r::core::CoreValue::Var(name),
                then_value: chiba_level1r::core::CoreValue::I64(11),
                else_value: chiba_level1r::core::CoreValue::I64(22),
            } if name == "x"
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $helper (param $x i32) (result i32)"));
    assert!(bundle.backend_link.linked_wat.contains("local.get $x"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "22");
}

#[test]
fn program_param_literal_match_lowers_to_executable_wat() {
    let program = SourceProgram::new(vec![
        def(
            "helper",
            vec!["x"],
            Expr::match_expr(
                Expr::var("x"),
                vec![
                    (chiba_level1r::ast::Pattern::lit_i64(1), Expr::i64(10)),
                    (chiba_level1r::ast::Pattern::wildcard(), Expr::i64(30)),
                ],
            ),
        ),
        def(
            "main",
            vec![],
            Expr::call_args(Expr::var("helper"), vec![Expr::i64(1)]),
        ),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnMatch {
                scrutinee: chiba_level1r::core::CoreValue::Var(name),
                arms,
            } if name == "x"
                && arms.len() == 2
                && arms[0].pattern == chiba_level1r::core::CorePattern::I64(1)
                && arms[0].value == chiba_level1r::core::CoreValue::I64(10)
                && arms[1].pattern == chiba_level1r::core::CorePattern::Wildcard
                && arms[1].value == chiba_level1r::core::CoreValue::I64(30)
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $helper (param $x i32) (result i32)"));
    assert!(bundle.backend_link.linked_wat.contains("local.get $x"));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "10");
}

#[test]
fn program_adt_constructor_return_lowers_to_executable_tag_wat() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(1)]),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnValue(
                chiba_level1r::core::CoreValue::Adt {
                    data,
                    ctor,
                    variants,
                    args,
                }
            ) if data == "Option"
                && ctor == "Some"
                && variants == &vec!["None".to_string(), "Some".to_string()]
                && args == &vec![chiba_level1r::core::CoreValue::I64(1)]
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; adt data=Option ctor=Some args=1"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_branch_can_return_executable_adt_constructor_tag() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::if_else(
            Expr::bool(false),
            Expr::adt_ctor("Option", "None", vec!["None", "Some"], Vec::new()),
            Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(1)]),
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnBranch {
                then_value: chiba_level1r::core::CoreValue::Adt {
                    ctor: then_ctor,
                    ..
                },
                else_value: chiba_level1r::core::CoreValue::Adt {
                    ctor: else_ctor,
                    ..
                },
                ..
            } if then_ctor == "None" && else_ctor == "Some"
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains("if (result i32)"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn program_zero_arg_adt_match_lowers_to_executable_tag_match_wat() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::match_expr(
            Expr::adt_ctor("Option", "None", vec!["None", "Some"], Vec::new()),
            vec![
                (
                    chiba_level1r::ast::Pattern::qualified_ctor("Option", "Some", Vec::new()),
                    Expr::i64(9),
                ),
                (
                    chiba_level1r::ast::Pattern::qualified_ctor("Option", "None", Vec::new()),
                    Expr::i64(4),
                ),
            ],
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnMatch {
                scrutinee: chiba_level1r::core::CoreValue::Adt {
                    data,
                    ctor,
                    variants,
                    ..
                },
                arms,
            } if data == "Option"
                && ctor == "None"
                && variants == &vec!["None".to_string(), "Some".to_string()]
                && arms.len() == 2
                && arms[0].pattern
                    == chiba_level1r::core::CorePattern::Constructor {
                        data: Some("Option".to_string()),
                        ctor: "Some".to_string(),
                        args: Vec::new(),
                    }
                && arms[1].pattern
                    == chiba_level1r::core::CorePattern::Constructor {
                        data: Some("Option".to_string()),
                        ctor: "None".to_string(),
                        args: Vec::new(),
                    }
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains(";; core-return match scrutinee=Option.None() arms=2"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "4");
}

#[test]
fn program_adt_payload_match_lowers_to_executable_payload_wat() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::match_expr(
            Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(8)]),
            vec![
                (
                    chiba_level1r::ast::Pattern::qualified_ctor(
                        "Option",
                        "Some",
                        vec![chiba_level1r::ast::Pattern::bind("value")],
                    ),
                    Expr::var("value"),
                ),
                (
                    chiba_level1r::ast::Pattern::qualified_ctor("Option", "None", Vec::new()),
                    Expr::i64(0),
                ),
            ],
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    assert!(bundle.defs[0].output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ReturnMatch {
                scrutinee: chiba_level1r::core::CoreValue::Adt {
                    data,
                    ctor,
                    variants,
                    args,
                },
                arms,
            } if data == "Option"
                && ctor == "Some"
                && variants == &vec!["None".to_string(), "Some".to_string()]
                && args == &vec![chiba_level1r::core::CoreValue::I64(8)]
                && arms.len() == 2
                && arms[0].pattern
                    == chiba_level1r::core::CorePattern::Constructor {
                        data: Some("Option".to_string()),
                        ctor: "Some".to_string(),
                        args: vec![chiba_level1r::core::CorePattern::Bind("value".to_string())],
                    }
                && arms[0].value == chiba_level1r::core::CoreValue::Var("value".to_string())
        )
    }));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "8");
}

#[test]
fn program_adt_payload_literal_pattern_checks_payload_before_arm_body() {
    let program = SourceProgram::new(vec![def(
        "main",
        vec![],
        Expr::match_expr(
            Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(8)]),
            vec![
                (
                    chiba_level1r::ast::Pattern::qualified_ctor(
                        "Option",
                        "Some",
                        vec![chiba_level1r::ast::Pattern::lit_i64(9)],
                    ),
                    Expr::i64(90),
                ),
                (
                    chiba_level1r::ast::Pattern::qualified_ctor(
                        "Option",
                        "Some",
                        vec![chiba_level1r::ast::Pattern::bind("value")],
                    ),
                    Expr::var("value"),
                ),
                (
                    chiba_level1r::ast::Pattern::qualified_ctor("Option", "None", Vec::new()),
                    Expr::i64(0),
                ),
            ],
        ),
    )]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean(&bundle, 0);
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "8");
}

#[test]
fn program_bundle_reports_duplicate_defs_and_entry_params() {
    let program = SourceProgram::new(vec![
        def("main", vec!["x"], Expr::var("x")),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.entry, Some("main".to_string()));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateDef {
            name: "main".to_string(),
        }));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::EntryHasParams {
            name: "main".to_string(),
            params: vec!["x".to_string()],
        }));
    assert_eq!(
        bundle
            .backend_link
            .linked_wat
            .matches("(export \"main\")")
            .count(),
        1
    );
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main__def1 (result i32)"));
}

#[test]
fn program_surface_reports_extern_and_def_duplicate_in_same_namespace() {
    let program = SourceProgram::new(vec![
        extern_def("fd_write", ExternAbi::Wasi, "fd_write"),
        def("fd_write", vec![], Expr::i64(0)),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateDef {
            name: "fd_write".to_string(),
        }));
}

#[test]
fn program_backend_cache_key_includes_source_extern_imports() {
    let wasi = compile_source_program_bundle(
        r#"
def fd_write(fd: i64): i64 = extern "wasi" "fd_write"
def main() = 0
"#,
    )
    .expect("compile wasi extern")
    .program;
    let env = compile_source_program_bundle(
        r#"
def fd_write(fd: i64): i64 = extern "C" "fd_write"
def main() = 0
"#,
    )
    .expect("compile env extern")
    .program;
    let env_lower = compile_source_program_bundle(
        r#"
def fd_write(fd: i64): i64 = extern "c" "fd_write"
def main() = 0
"#,
    )
    .expect("compile lowercase env extern")
    .program;
    let renamed = compile_source_program_bundle(
        r#"
def fd_write(fd: i64): i64 = extern "wasi" "proc_exit"
def main() = 0
"#,
    )
    .expect("compile renamed extern")
    .program;

    assert_ne!(wasi.backend_cache_key, env.backend_cache_key);
    assert_eq!(env.backend_cache_key, env_lower.backend_cache_key);
    assert_ne!(wasi.backend_cache_key, renamed.backend_cache_key);
    assert_eq!(wasi.backend_link.manifest.imports.len(), 1);
    assert_eq!(
        wasi.backend_link.manifest.imports[0].module,
        "wasi_snapshot_preview1"
    );
    assert_eq!(wasi.backend_link.manifest.imports[0].name, "fd_write");
    assert_eq!(
        wasi.backend_link.manifest.imports[0].signature_hash,
        "i64_to_i64"
    );
    assert!(wasi.backend_link.linked_wat.contains(
        ";; extern-import wasi symbol=root__fd_write module=wasi_snapshot_preview1 name=fd_write signature=i64_to_i64"
    ));
    assert!(env.backend_link.linked_wat.contains(
        ";; extern-import c symbol=root__fd_write module=env name=fd_write signature=i64_to_i64"
    ));
    assert_eq!(
        env.backend_link.linked_wat,
        env_lower.backend_link.linked_wat
    );
    assert!(wasi
        .render_summary()
        .contains("import wasi symbol=root__fd_write module=wasi_snapshot_preview1 name=fd_write signature=i64_to_i64"));
    assert!(wasi.diagnostics.is_empty());
    assert!(env.diagnostics.is_empty());
    assert!(renamed.diagnostics.is_empty());
}

#[test]
fn program_extern_c_call_lowers_to_executable_env_import_wat() {
    let bundle = compile_source_program_bundle(
        r#"
def inc(value: i64): i64 = extern "C" "level1r_inc"
def main() = inc(8)
"#,
    )
    .expect("compile extern call")
    .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main def");
    assert!(main.output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ExternFunctionTarget {
                target,
                abi: chiba_level1r::core::CoreExternAbi::C,
                name,
                signature,
                ..
            } if target == "inc" && name == "level1r_inc" && signature == "i64_to_i64"
        )
    }));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(import \"env\" \"level1r_inc\" (func $inc (param i32) (result i32)))"));
    assert!(bundle.backend_link.linked_wat.contains(
        ";; extern-import c symbol=inc module=env name=level1r_inc signature=i64_to_i64"
    ));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "9");
}

#[test]
fn program_extern_c_multi_arg_call_lowers_to_executable_env_import_wat() {
    let bundle = compile_source_program_bundle(
        r#"
def add(left: i64, right: i64): i64 = extern "C" "level1r_add"
def main() = add(8, 5)
"#,
    )
    .expect("compile extern multi arg call")
    .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main def");
    assert!(main.output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ExternFunctionTarget {
                target,
                abi: chiba_level1r::core::CoreExternAbi::C,
                name,
                signature,
                ..
            } if target == "add" && name == "level1r_add" && signature == "i64_i64_to_i64"
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"level1r_add\" (func $add (param i32) (param i32) (result i32)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        ";; extern-import c symbol=add module=env name=level1r_add signature=i64_i64_to_i64"
    ));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "13");
}

#[test]
fn program_extern_c_cstr_arg_lowers_to_executable_externref_import_wat() {
    let bundle = compile_source_program_bundle(
        r#"
def c_len(value: cstr): i64 = extern "C" "level1r_cstr_len"
def main(): i64 = c_len(String.from("hé").to_cstr())
"#,
    )
    .expect("compile cstr extern call")
    .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main def");
    assert!(main.output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ExternFunctionTarget {
                target,
                abi: chiba_level1r::core::CoreExternAbi::C,
                name,
                signature,
                ..
            } if target == "c_len" && name == "level1r_cstr_len" && signature == "externref_to_i64"
        )
    }));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"level1r_cstr_len\" (func $c_len (param externref) (result i32)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        ";; extern-import c symbol=c_len module=env name=level1r_cstr_len signature=externref_to_i64"
    ));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn program_extern_c_cstr_literal_arg_lowers_to_executable_externref_import_wat() {
    let bundle = compile_source_program_bundle(
        r#"
def c_len(value: cstr): i64 = extern "C" "level1r_cstr_len"
def main(): i64 = c_len(c"hé")
"#,
    )
    .expect("compile cstr literal extern call")
    .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"level1r_cstr_len\" (func $c_len (param externref) (result i32)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"std.cstr_literal_3\" (func $std_cstr_literal_3 (param i32) (param i32) (param i32) (result externref)))"
    ));

    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "3");
}

#[test]
fn program_extern_c_builtin_handle_args_lower_to_executable_externref_import_wat() {
    let slice = compile_source_program_bundle(
        r#"
def handle_len(value: Slice[i64]): i64 = extern "C" "level1r_externref_len"
def main(): i64 = handle_len([1, 2, 3])
"#,
    )
    .expect("compile slice extern arg")
    .program;
    let string = compile_source_program_bundle(
        r#"
def handle_len(value: String): i64 = extern "C" "level1r_externref_len"
def main(): i64 = handle_len(String.from("hé"))
"#,
    )
    .expect("compile string extern arg")
    .program;
    let range = compile_source_program_bundle(
        r#"
def range_end(value: Range): i64 = extern "C" "level1r_range_end"
def main(): i64 = range_end(3..9)
"#,
    )
    .expect("compile range extern arg")
    .program;
    let cell = compile_source_program_bundle(
        r#"
def cell_len(value: Ref[String]): i64 = extern "C" "level1r_ref_cell_len"
def main(): i64 = cell_len(Ref.new(String.from("hé")))
"#,
    )
    .expect("compile ref extern arg")
    .program;

    for (bundle, symbol) in [
        (&slice, "handle_len"),
        (&string, "handle_len"),
        (&range, "range_end"),
        (&cell, "cell_len"),
    ] {
        assert_eq!(bundle.diagnostics, vec![]);
        assert_backend_link_clean_all(bundle);
        assert!(bundle
            .backend_link
            .linked_wat
            .contains(&format!("(func ${symbol} (param externref) (result i32))")));
        assert!(bundle
            .backend_link
            .linked_wat
            .contains("signature=externref_to_i64"));
    }
    assert_eq!(run_wat_text(&slice.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&string.backend_link.linked_wat), "3");
    assert_eq!(run_wat_text(&range.backend_link.linked_wat), "9");
    assert_eq!(run_wat_text(&cell.backend_link.linked_wat), "3");
}

#[test]
fn program_extern_c_builtin_handle_return_lowers_to_executable_externref_import_wat() {
    let bundle = compile_source_program_bundle(
        r#"
def id(value: String): String = extern "C" "level1r_identity_externref"
def main(): String = id(String.from("hé"))
"#,
    )
    .expect("compile string extern return")
    .program;

    assert_eq!(bundle.diagnostics, vec![]);
    assert_backend_link_clean_all(&bundle);
    assert!(bundle.backend_link.linked_wat.contains(
        "(import \"env\" \"level1r_identity_externref\" (func $id (param externref) (result externref)))"
    ));
    assert!(bundle.backend_link.linked_wat.contains(
        ";; extern-import c symbol=id module=env name=level1r_identity_externref signature=externref_to_externref"
    ));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "bytes/3");
}

#[test]
fn program_extern_c_unsupported_signature_is_backend_diagnostic() {
    let bundle = compile_source_program_bundle(
        r#"
def first(pair: Tuple[i64, i64]): i64 = extern "C" "level1r_first"
def main() = first((8, 5))
"#,
    )
    .expect("compile unsupported extern signature")
    .program;

    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main def");
    assert!(main.output.core.ops.iter().any(|op| {
        matches!(
            op,
            chiba_level1r::core::CoreOp::ExternFunctionTarget {
                target,
                signature,
                ..
            } if target == "first" && signature == "Tuple[i64,i64]_to_i64"
        )
    }));
    assert_eq!(
        main.output.backend.diagnostics,
        vec![
            chiba_level1r::backend::BackendDiagnostic::UnsupportedExternImportSignature {
                symbol: "first".to_string(),
                signature: "Tuple[i64,i64]_to_i64".to_string(),
            }
        ]
    );
    assert!(main.output.backend.wat.is_empty());
    assert!(bundle.backend_link.diagnostics.contains(
        &chiba_level1r::backend::BackendLinkDiagnostic::ArtifactEmitFailed { artifact_index: 0 }
    ));
}

#[test]
fn program_extern_target_does_not_ignore_local_shadowing() {
    let bundle = compile_source_program_bundle(
        r#"
def inc(value: i64): i64 = extern "C" "level1r_inc"
def main(inc: (i64) -> i64) = inc(8)
"#,
    )
    .expect("compile shadowing extern call")
    .program;

    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main def");
    assert!(!main
        .output
        .core
        .ops
        .iter()
        .any(|op| { matches!(op, chiba_level1r::core::CoreOp::ExternFunctionTarget { .. }) }));
    assert!(!main
        .output
        .backend
        .wat
        .contains("(import \"env\" \"level1r_inc\""));
}

#[test]
fn program_surface_allows_same_function_name_across_owner_namespaces() {
    let left = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["left".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![def("shared", vec![], Expr::i64(1))],
    );
    let right = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["right".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![extern_def("shared", ExternAbi::C, "shared")],
    );
    let surface = project_surface_many(&[right, left]);
    let diagnostics = chiba_level1r::pipeline::program_surface_diagnostics(&surface);

    assert_eq!(
        surface
            .defs
            .iter()
            .map(|def| format!("{}::{}", def.owner, def.name))
            .collect::<Vec<_>>(),
        vec!["left::shared".to_string(), "right::shared".to_string()]
    );
    assert!(!diagnostics.contains(&ProgramDiagnostic::DuplicateDef {
        name: "shared".to_string(),
    }));
}

#[test]
fn program_surface_allows_same_top_level_names_across_owner_namespaces() {
    let left = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["left".to_string()])),
        Vec::new(),
        vec![TypeDecl::new(
            "Shared",
            Vec::new(),
            vec![TypeField::new("value", "i64")],
        )],
        vec![DataDecl::new(
            "Payload",
            Vec::new(),
            vec![DataVariant::new("Same", Vec::new())],
        )],
        vec![
            static_value("CONFIG", Some("i64"), Expr::i64(1)),
            def("main", vec![], Expr::i64(0)),
        ],
    );
    let right = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["right".to_string()])),
        Vec::new(),
        vec![TypeDecl::new(
            "Shared",
            Vec::new(),
            vec![TypeField::new("value", "bool")],
        )],
        vec![DataDecl::new(
            "Payload",
            Vec::new(),
            vec![DataVariant::new("Same", Vec::new())],
        )],
        vec![
            static_value("CONFIG", Some("i64"), Expr::i64(2)),
            def("main", vec![], Expr::i64(0)),
        ],
    );
    let surface = project_surface_many(&[right, left]);
    let diagnostics = chiba_level1r::pipeline::program_surface_diagnostics(&surface);

    assert_eq!(
        surface
            .types
            .iter()
            .map(|ty| format!("{}::{}", ty.owner, ty.name))
            .collect::<Vec<_>>(),
        vec!["left::Shared".to_string(), "right::Shared".to_string()]
    );
    assert_eq!(
        surface
            .data
            .iter()
            .map(|data| format!("{}::{}", data.owner, data.name))
            .collect::<Vec<_>>(),
        vec!["left::Payload".to_string(), "right::Payload".to_string()]
    );
    assert_eq!(
        surface
            .statics
            .iter()
            .map(|static_value| format!("{}::{}", static_value.owner, static_value.name))
            .collect::<Vec<_>>(),
        vec!["left::CONFIG".to_string(), "right::CONFIG".to_string()]
    );
    assert!(!diagnostics.contains(&ProgramDiagnostic::DuplicateType {
        name: "Shared".to_string(),
    }));
    assert!(!diagnostics.contains(&ProgramDiagnostic::DuplicateData {
        name: "Payload".to_string(),
    }));
    assert!(
        !diagnostics.contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "Shared".to_string(),
        })
    );
    assert!(
        !diagnostics.contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "Payload".to_string(),
        })
    );
    assert!(
        !diagnostics.contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "CONFIG".to_string(),
        })
    );
    assert!(
        !diagnostics.contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "main".to_string(),
        })
    );
}

#[test]
fn pattern_clause_defs_lower_to_single_dispatcher_without_duplicate_def() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            vec!["T".to_string()],
            vec![
                DataVariant::new("Some", vec!["T".to_string()]),
                DataVariant::new("None", Vec::new()),
            ],
        )],
        vec![
            SourceItem::Def {
                receiver: None,
                generics: vec!["T".to_string()],
                name: "unwrap_or_zero".to_string(),
                visibility: Visibility::Public,
                params: vec![ParamDecl::pattern(
                    chiba_level1r::ast::Pattern::ctor(
                        "Some",
                        vec![chiba_level1r::ast::Pattern::bind("x")],
                    ),
                    Some("Option[i64]".to_string()),
                )],
                return_type: Some("i64".to_string()),
                body: Expr::var("x"),
            },
            SourceItem::Def {
                receiver: None,
                generics: vec!["T".to_string()],
                name: "unwrap_or_zero".to_string(),
                visibility: Visibility::Public,
                params: vec![ParamDecl::pattern(
                    chiba_level1r::ast::Pattern::ctor("None", Vec::new()),
                    Some("Option[i64]".to_string()),
                )],
                return_type: Some("i64".to_string()),
                body: Expr::i64(0),
            },
        ],
    );

    let bundle = compile_program_bundle(&program);

    assert!(!bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateDef {
            name: "unwrap_or_zero".to_string(),
        }));
    assert_eq!(bundle.defs.len(), 1);
    assert_eq!(bundle.defs[0].name, "unwrap_or_zero");
    assert_eq!(bundle.defs[0].params, vec!["value".to_string()]);
    assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
    assert_eq!(bundle.defs[0].output.pattern.matches.len(), 1);
    assert!(bundle.defs[0].output.pattern.matches[0].exhaustive);
}

#[test]
fn source_pattern_clause_defs_compile_through_dispatcher() {
    let output = chiba_level1r::compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def unwrap_or_zero(Some(x): Option[i64]): i64 = x
def unwrap_or_zero(None: Option[i64]): i64 = 0",
    )
    .expect("compile source");

    assert_eq!(output.frontend.program.items.len(), 2);
    assert_eq!(output.program.defs.len(), 1);
    assert_eq!(output.program.defs[0].name, "unwrap_or_zero");
    assert_eq!(output.program.defs[0].params, vec!["value".to_string()]);
    assert_eq!(output.program.defs[0].output.typed.ty, Type::I64);
    assert_eq!(output.program.defs[0].output.pattern.matches.len(), 1);
    assert!(output.program.defs[0].output.pattern.matches[0].exhaustive);
}

#[test]
fn source_non_exhaustive_match_reaches_pattern_diagnostic_and_visual() {
    let output = chiba_level1r::compile_source_program_bundle(
        "def main(flag: bool): i64 = match flag {
true => 1
}",
    )
    .expect("compile source");
    let main = &output.program.defs[0].output;

    assert_eq!(main.pattern.matches.len(), 1);
    assert!(!main.pattern.matches[0].exhaustive);
    assert_eq!(
        main.pattern.diagnostics,
        vec![PatternDiagnostic::NonExhaustiveMatch {
            scrutinee_type: Type::Bool,
            missing: vec![chiba_level1r::ast::Pattern::lit_bool(false)],
        }]
    );
    let visual = main.render_visual();
    assert!(visual.contains("pattern:"));
    assert!(visual.contains("diagnostic non-exhaustive-match scrutinee=bool missing=[false]"));
    assert!(!visual.contains("NonExhaustiveMatch"));
    assert!(!visual.contains("scrutinee_type"));
}

#[test]
fn source_pattern_clause_defs_merge_wildcard_fallback() {
    let output = chiba_level1r::compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def unwrap_or_zero(Some(x): Option[i64]): i64 = x
def unwrap_or_zero(_: Option[i64]): i64 = 0",
    )
    .expect("compile source");

    assert_eq!(output.frontend.program.items.len(), 2);
    assert!(!output
        .program
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateDef {
            name: "unwrap_or_zero".to_string(),
        }));
    assert_eq!(output.program.defs.len(), 1);
    assert_eq!(output.program.defs[0].name, "unwrap_or_zero");
    assert_eq!(output.program.defs[0].params, vec!["value".to_string()]);
    assert_eq!(output.program.defs[0].output.typed.ty, Type::I64);
    assert_eq!(output.program.defs[0].output.pattern.matches.len(), 1);
    assert!(output.program.defs[0].output.pattern.matches[0].exhaustive);
}

#[test]
fn source_pattern_clause_defs_merge_binding_fallback() {
    let output = chiba_level1r::compile_source_program_bundle(
        "data Option[T] = { Some(T), None }
def unwrap_or_zero(Some(x): Option[i64]): i64 = x
def unwrap_or_zero(other: Option[i64]): i64 = 0",
    )
    .expect("compile source");

    assert_eq!(output.frontend.program.items.len(), 2);
    assert!(!output
        .program
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateDef {
            name: "unwrap_or_zero".to_string(),
        }));
    assert_eq!(output.program.defs.len(), 1);
    assert_eq!(output.program.defs[0].name, "unwrap_or_zero");
    assert_eq!(output.program.defs[0].params, vec!["value".to_string()]);
    assert_eq!(output.program.defs[0].output.typed.ty, Type::I64);
    assert_eq!(output.program.defs[0].output.pattern.matches.len(), 1);
    assert!(output.program.defs[0].output.pattern.matches[0].exhaustive);
}

#[test]
fn program_surface_and_interface_summary_preserve_owner_namespace() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec![
            "parser".to_string(),
            "core".to_string(),
        ])),
        vec![UseDecl::new(
            vec!["std".to_string(), "regex".to_string()],
            false,
        )],
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            vec!["T".to_string()],
            vec![
                DataVariant::new("Some", vec!["T".to_string()]),
                DataVariant::new("None", Vec::new()),
            ],
        )],
        vec![def("main", vec![], Expr::i64(7))],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.namespace, "parser.core");
    assert_eq!(bundle.surface.imports, vec!["std.regex".to_string()]);
    assert_eq!(bundle.surface.defs[0].owner, "parser.core");
    assert_eq!(
        bundle.surface.defs[0].param_types,
        Vec::<Option<String>>::new()
    );
    assert_eq!(bundle.surface.defs[0].return_type, None);
    assert_eq!(bundle.surface.data[0].owner, "parser.core");
    assert_eq!(bundle.surface.constructors.len(), 2);
    assert_eq!(bundle.interface.namespace, "parser.core");
    assert_eq!(bundle.interface.functions[0].symbol, "parser.core::main");
    assert_eq!(
        bundle.interface.functions[0].param_types,
        Vec::<Option<String>>::new()
    );
    assert_eq!(bundle.interface.functions[0].return_type, None);
    assert_eq!(bundle.interface.data[0].symbol, "parser.core::Option");
    assert_eq!(
        bundle.interface.constructors[0].symbol,
        "parser.core::Option.Some"
    );
    assert_eq!(bundle.interface.constructors[0].arity, 1);
    assert_eq!(bundle.interface.constructors[1].arity, 0);
    assert_eq!(bundle.interface.imports, vec!["std.regex".to_string()]);
    assert_eq!(bundle.interface.stable_hash.len(), 16);

    let summary = bundle.render_summary();
    assert!(summary.contains("surface:"));
    assert!(summary.contains("interface:"));
    assert!(summary.contains("namespace=parser.core"));
    assert!(summary.contains("constructor parser.core::Option.Some"));
    assert!(summary.contains("parser.core::Option.Some"));
    assert!(!summary.contains("surface=ProjectSurface"));
    assert!(!summary.contains("interface=InterfaceSummary"));
}

#[test]
fn private_def_and_static_visibility_reach_surface_and_interface() {
    let output = chiba_level1r::compile_source_program_bundle(
        "namespace demo.math
private def hidden(): i64 = 0
private def SECRET: i64 = 1
def public_value(): i64 = SECRET",
    )
    .expect("compile source");

    assert_eq!(output.program.surface.defs[0].name, "hidden");
    assert_eq!(
        output.program.surface.defs[0].visibility,
        Visibility::Private
    );
    assert_eq!(output.program.surface.defs[1].name, "public_value");
    assert_eq!(
        output.program.surface.defs[1].visibility,
        Visibility::Public
    );
    assert_eq!(output.program.surface.statics[0].name, "SECRET");
    assert_eq!(
        output.program.surface.statics[0].visibility,
        Visibility::Private
    );
    assert_eq!(
        output.program.interface.functions[0].visibility,
        Visibility::Private
    );
    assert_eq!(
        output.program.interface.functions[1].visibility,
        Visibility::Public
    );
    assert_eq!(
        output.program.interface.statics[0].visibility,
        Visibility::Private
    );
    let summary = output.program.render_summary();
    assert!(summary.contains("def demo.math::hidden visibility=private"));
    assert!(summary.contains("def demo.math::public_value visibility=public"));
    assert!(summary.contains("static demo.math::SECRET visibility=private"));
    assert!(summary
        .contains("function demo.math::hidden owner=demo.math source=hidden visibility=private"));
}

#[test]
fn project_surface_many_merges_namespaces_deterministically() {
    let lexer_program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["lexer".to_string()])),
        vec![UseDecl::new(
            vec!["std".to_string(), "text".to_string()],
            false,
        )],
        vec![TypeDecl::alias("TokenId", Vec::new(), "i64")],
        Vec::new(),
        vec![def("scan", vec![], Expr::i64(1))],
    );
    let parser_program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        vec![UseDecl::new(vec!["lexer".to_string()], true)],
        Vec::new(),
        vec![DataDecl::new(
            "Ast",
            Vec::new(),
            vec![DataVariant::new("Node", vec!["TokenId".to_string()])],
        )],
        vec![def("parse", vec![], Expr::i64(2))],
    );

    let forward = project_surface_many(&[lexer_program.clone(), parser_program.clone()]);
    let reverse = project_surface_many(&[parser_program, lexer_program]);

    assert_eq!(forward, reverse);
    assert_eq!(forward.namespace, "<project>");
    assert_eq!(
        forward.imports,
        vec!["lexer.*".to_string(), "std.text".to_string()]
    );
    assert_eq!(
        forward
            .defs
            .iter()
            .map(|def| format!("{}::{}", def.owner, def.name))
            .collect::<Vec<_>>(),
        vec!["lexer::scan".to_string(), "parser::parse".to_string()]
    );
    assert_eq!(forward.types[0].owner, "lexer");
    assert_eq!(forward.data[0].owner, "parser");
    assert_eq!(forward.constructors[0].owner, "parser");

    let forward_summary = build_interface_summary(&forward);
    let reverse_summary = build_interface_summary(&reverse);
    assert_eq!(forward_summary, reverse_summary);
    assert_eq!(forward_summary.functions[0].symbol, "lexer::scan");
    assert_eq!(forward_summary.functions[1].symbol, "parser::parse");
    assert_eq!(forward_summary.types[0].symbol, "lexer::TokenId");
    assert_eq!(forward_summary.types[0].owner, "lexer");
    assert_eq!(forward_summary.types[0].name, "TokenId");
    assert_eq!(forward_summary.data[0].symbol, "parser::Ast");
    assert_eq!(forward_summary.constructors[0].symbol, "parser::Ast.Node");
}

#[test]
fn interface_summary_preserves_function_signature_types() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec![
            "parser".to_string(),
            "core".to_string(),
        ])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "id".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("x", Some("I64".to_string()))],
            return_type: Some("I64".to_string()),
            body: Expr::var("x"),
        }],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.defs[0].arity, 1);
    assert_eq!(
        bundle.surface.defs[0].param_types,
        vec![Some("I64".to_string())]
    );
    assert_eq!(bundle.surface.defs[0].return_type, Some("I64".to_string()));
    assert_eq!(bundle.interface.functions[0].symbol, "parser.core::id");
    assert_eq!(
        bundle.interface.functions[0].param_types,
        vec![Some("I64".to_string())]
    );
    assert_eq!(
        bundle.interface.functions[0].return_type,
        Some("I64".to_string())
    );
    let summary = bundle.render_summary();
    assert!(summary.contains("function parser.core::id"));
    assert!(summary.contains("params=[I64]"));
    assert!(summary.contains("return=I64"));
    assert!(!summary.contains("param_types"));
    assert!(!summary.contains("return_type"));
}

#[test]
fn interface_summary_hash_ignores_function_body_changes() {
    let one = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "parse".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("input", Some("Token".to_string()))],
            return_type: Some("Ast".to_string()),
            body: Expr::i64(1),
        }],
    );
    let two = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "parse".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("input", Some("Token".to_string()))],
            return_type: Some("Ast".to_string()),
            body: Expr::i64(2),
        }],
    );

    assert_eq!(
        compile_program_bundle(&one).interface.stable_hash,
        compile_program_bundle(&two).interface.stable_hash
    );
}

#[test]
fn interface_summary_hash_keeps_owner_namespace_identity() {
    let parser = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["parser".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![def("shared", vec![], Expr::i64(1))],
    );
    let lexer = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["lexer".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![def("shared", vec![], Expr::i64(1))],
    );

    assert_ne!(
        compile_program_bundle(&parser).interface.stable_hash,
        compile_program_bundle(&lexer).interface.stable_hash
    );
}

#[test]
fn interface_summary_preserves_explicit_checked_template_params() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec![
            "parser".to_string(),
            "core".to_string(),
        ])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: vec!["T".to_string()],
            name: "id".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("x", Some("T".to_string()))],
            return_type: Some("T".to_string()),
            body: Expr::var("x"),
        }],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.defs[0].generics, vec!["T".to_string()]);
    assert_eq!(
        bundle.interface.functions[0].generics,
        vec!["T".to_string()]
    );
    assert_eq!(
        bundle.defs[0].output.template.explicit_params,
        vec![chiba_level1r::TemplateParam {
            name: "T".to_string(),
            source: chiba_level1r::TemplateParamSource::ExplicitHeader,
        }]
    );
    assert_eq!(
        bundle.defs[0].output.specialize.work_items[0]
            .key
            .template_params,
        bundle.defs[0].output.template.explicit_params
    );
}

#[test]
fn interface_summary_preserves_row_style_type_decl_shape() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec![
            "parser".to_string(),
            "core".to_string(),
        ])),
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            vec!["T".to_string()],
            vec![TypeField::new("value", "T")],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.types[0].owner, "parser.core");
    assert_eq!(bundle.surface.types[0].name, "Box");
    assert_eq!(bundle.surface.types[0].generics, vec!["T".to_string()]);
    assert_eq!(bundle.surface.types[0].fields[0].name, "value");
    assert_eq!(bundle.surface.types[0].fields[0].ty, "T");
    assert_eq!(bundle.interface.types[0].symbol, "parser.core::Box");
    assert_eq!(bundle.interface.types[0].owner, "parser.core");
    assert_eq!(bundle.interface.types[0].name, "Box");
    assert_eq!(bundle.interface.types[0].fields[0].name, "value");
    assert_eq!(bundle.interface.types[0].fields[0].ty, "T");
    assert!(bundle.render_summary().contains("parser.core::Box"));
}

#[test]
fn interface_summary_preserves_type_alias_target() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec![
            "parser".to_string(),
            "core".to_string(),
        ])),
        Vec::new(),
        vec![TypeDecl::alias("UserId", Vec::new(), "i64")],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.interface.types[0].symbol, "parser.core::UserId");
    assert_eq!(bundle.interface.types[0].name, "UserId");
    assert_eq!(
        bundle.interface.types[0].alias_target,
        Some("i64".to_string())
    );
    let summary = bundle.render_summary();
    assert!(summary.contains("type parser.core::UserId"));
    assert!(summary.contains("alias=i64"));
    assert!(!summary.contains("alias_target"));
}

#[test]
fn typed_signature_resolves_type_alias_headers() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::alias("UserId", Vec::new(), "i64")],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "id".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("value", Some("UserId".to_string()))],
            return_type: Some("UserId".to_string()),
            body: Expr::var("value"),
        }],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.defs[0]
            .output
            .typed_signature
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect::<Vec<_>>(),
        vec![("value".to_string(), "i64".to_string())]
    );
    assert_eq!(
        bundle.defs[0].output.typed_signature.return_type,
        Some("i64".to_string())
    );
    assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
}

#[test]
fn typed_signature_resolves_local_alias_when_project_has_same_alias_name() {
    let current = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["current".to_string()])),
        Vec::new(),
        vec![TypeDecl::alias("UserId", Vec::new(), "i64")],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "id".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("value", Some("UserId".to_string()))],
            return_type: Some("UserId".to_string()),
            body: Expr::var("value"),
        }],
    );
    let imported = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["imported".to_string()])),
        Vec::new(),
        vec![TypeDecl::alias("UserId", Vec::new(), "bool")],
        Vec::new(),
        Vec::new(),
    );
    let interface =
        build_interface_summary(&project_surface_many(&[imported.clone(), current.clone()]));
    let output = compile_program_with_interface(&current, &interface);

    assert_eq!(
        output[0].output.typed_signature.params[0].ty,
        "i64".to_string()
    );
    assert_eq!(
        output[0].output.typed_signature.return_type,
        Some("i64".to_string())
    );
    assert_eq!(output[0].output.typed.ty, Type::I64);
}

#[test]
fn typed_signature_does_not_guess_alias_when_owner_is_ambiguous() {
    let consumer = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["consumer".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "id".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("value", Some("UserId".to_string()))],
            return_type: Some("UserId".to_string()),
            body: Expr::var("value"),
        }],
    );
    let left = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["left".to_string()])),
        Vec::new(),
        vec![TypeDecl::alias("UserId", Vec::new(), "i64")],
        Vec::new(),
        Vec::new(),
    );
    let right = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["right".to_string()])),
        Vec::new(),
        vec![TypeDecl::alias("UserId", Vec::new(), "bool")],
        Vec::new(),
        Vec::new(),
    );
    let interface = build_interface_summary(&project_surface_many(&[
        right.clone(),
        consumer.clone(),
        left.clone(),
    ]));
    let output = compile_program_with_interface(&consumer, &interface);

    assert_eq!(
        output[0].output.typed_signature.params[0].ty,
        "UserId".to_string()
    );
    assert_eq!(
        output[0].output.typed_signature.return_type,
        Some("UserId".to_string())
    );
    assert_eq!(
        output[0].output.typed.ty,
        Type::Nominal("UserId".to_string())
    );
}

#[test]
fn typed_context_resolves_local_nominal_row_when_project_has_same_type_name() {
    let current = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["current".to_string()])),
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            Vec::new(),
            vec![TypeField::new("value", "i64")],
        )],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "read".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("box", Some("Box".to_string()))],
            return_type: None,
            body: Expr::field(Expr::var("box"), "value"),
        }],
    );
    let imported = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["imported".to_string()])),
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            Vec::new(),
            vec![TypeField::new("value", "bool")],
        )],
        Vec::new(),
        Vec::new(),
    );
    let interface = build_interface_summary(&project_surface_many(&[imported, current.clone()]));
    let output = compile_program_with_interface(&current, &interface);

    assert_eq!(output[0].output.typed.ty, Type::I64);
}

#[test]
fn typed_context_does_not_guess_nominal_row_when_owner_is_ambiguous() {
    let consumer = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["consumer".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "read".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("box", Some("Box".to_string()))],
            return_type: None,
            body: Expr::field(Expr::var("box"), "value"),
        }],
    );
    let left = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["left".to_string()])),
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            Vec::new(),
            vec![TypeField::new("value", "i64")],
        )],
        Vec::new(),
        Vec::new(),
    );
    let right = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["right".to_string()])),
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            Vec::new(),
            vec![TypeField::new("value", "bool")],
        )],
        Vec::new(),
        Vec::new(),
    );
    let interface =
        build_interface_summary(&project_surface_many(&[right, consumer.clone(), left]));
    let output = compile_program_with_interface(&consumer, &interface);

    assert_eq!(output[0].output.typed.ty, Type::Unknown);
}

#[test]
fn typed_context_uses_unique_external_constructor_payload_from_interface() {
    let consumer = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["consumer".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "unwrap".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::pattern(
                chiba_level1r::ast::Pattern::ctor(
                    "Some",
                    vec![chiba_level1r::ast::Pattern::bind("x")],
                ),
                Some("Option[i64]".to_string()),
            )],
            return_type: None,
            body: Expr::var("x"),
        }],
    );
    let library = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["library".to_string()])),
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            vec!["T".to_string()],
            vec![
                DataVariant::new("Some", vec!["T".to_string()]),
                DataVariant::new("None", Vec::new()),
            ],
        )],
        Vec::new(),
    );
    let interface = build_interface_summary(&project_surface_many(&[library, consumer.clone()]));
    let output = compile_program_with_interface(&consumer, &interface);

    assert_eq!(
        output[0].output.typed_signature.params[0].binding_types,
        vec![("x".to_string(), Type::I64)]
    );
    assert_eq!(output[0].output.typed.ty, Type::I64);
}

#[test]
fn typed_context_resolves_local_constructor_payload_when_project_has_same_data_name() {
    let current = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["current".to_string()])),
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            Vec::new(),
            vec![DataVariant::new("Some", vec!["i64".to_string()])],
        )],
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "unwrap".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::pattern(
                chiba_level1r::ast::Pattern::ctor(
                    "Some",
                    vec![chiba_level1r::ast::Pattern::bind("x")],
                ),
                Some("Option".to_string()),
            )],
            return_type: None,
            body: Expr::var("x"),
        }],
    );
    let imported = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["imported".to_string()])),
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            Vec::new(),
            vec![DataVariant::new("Some", vec!["bool".to_string()])],
        )],
        Vec::new(),
    );
    let interface = build_interface_summary(&project_surface_many(&[imported, current.clone()]));
    let output = compile_program_with_interface(&current, &interface);

    assert_eq!(
        output[0].output.typed_signature.params[0].binding_types,
        vec![("x".to_string(), Type::I64)]
    );
    assert_eq!(output[0].output.typed.ty, Type::I64);
}

#[test]
fn typed_context_does_not_guess_constructor_payload_when_owner_is_ambiguous() {
    let consumer = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["consumer".to_string()])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "unwrap".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::pattern(
                chiba_level1r::ast::Pattern::ctor(
                    "Some",
                    vec![chiba_level1r::ast::Pattern::bind("x")],
                ),
                Some("Option[i64]".to_string()),
            )],
            return_type: None,
            body: Expr::var("x"),
        }],
    );
    let left = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["left".to_string()])),
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            Vec::new(),
            vec![DataVariant::new("Some", vec!["i64".to_string()])],
        )],
        Vec::new(),
    );
    let right = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec!["right".to_string()])),
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Option",
            Vec::new(),
            vec![DataVariant::new("Some", vec!["bool".to_string()])],
        )],
        Vec::new(),
    );
    let interface =
        build_interface_summary(&project_surface_many(&[right, consumer.clone(), left]));
    let output = compile_program_with_interface(&consumer, &interface);

    assert_eq!(
        output[0].output.typed_signature.params[0].binding_types,
        vec![("x".to_string(), Type::Unknown)]
    );
    assert_eq!(output[0].output.typed.ty, Type::Unknown);
}

#[test]
fn interface_summary_splits_type_fields_from_phantom_markers() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "User",
            Vec::new(),
            vec![
                TypeField::new("id", "i64"),
                TypeField::new("_", "PhantomUser"),
                TypeField::new("_", "AuditMarker"),
            ],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.interface.types[0].fields.len(), 1);
    assert_eq!(bundle.interface.types[0].fields[0].name, "id");
    assert_eq!(
        bundle.interface.types[0].phantom_markers,
        vec!["PhantomUser".to_string(), "AuditMarker".to_string()]
    );
    let summary = bundle.render_summary();
    assert!(summary.contains("fields=[id: i64]"));
    assert!(summary.contains("phantoms=[PhantomUser, AuditMarker]"));
    assert!(!summary.contains("phantom_markers"));
}

#[test]
fn interface_summary_preserves_method_style_receiver_and_self_surface() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec![
            "parser".to_string(),
            "core".to_string(),
        ])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::method_def(
            MethodReceiver::new("Box", vec!["T".to_string()]),
            "update",
            vec![
                ParamDecl::new("self", Some("Self".to_string())),
                ParamDecl::new("value", Some("T".to_string())),
            ],
            Some("Self".to_string()),
            Expr::var("self"),
        )],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.surface.defs[0].receiver,
        Some(MethodReceiver::new("Box", vec!["T".to_string()]))
    );
    assert_eq!(
        bundle.interface.functions[0].receiver,
        Some(MethodReceiver::new("Box", vec!["T".to_string()]))
    );
    assert_eq!(bundle.interface.functions[0].source_name, "update");
    assert_eq!(
        bundle.interface.functions[0].symbol,
        "parser.core::Box[T].update"
    );
    assert_eq!(
        bundle.interface.functions[0].param_types,
        vec![Some("Self".to_string()), Some("T".to_string())]
    );
    assert_eq!(
        bundle.interface.functions[0].return_type,
        Some("Self".to_string())
    );
    assert_eq!(
        bundle.defs[0]
            .output
            .typed_signature
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("self".to_string(), "Box[T]".to_string()),
            ("value".to_string(), "T".to_string()),
        ]
    );
    assert_eq!(
        bundle.defs[0].output.typed_signature.return_type,
        Some("Box[T]".to_string())
    );
    assert!(bundle.defs[0]
        .output
        .render_visual()
        .contains("typed-signature: def update(self: Box[T], value: T): Box[T]"));
    assert!(matches!(
        &bundle.defs[0].output.typed.kind,
        TypedExprKind::Var(name) if name == "self"
    ));
    assert_eq!(
        bundle.defs[0].output.typed.ty,
        Type::Nominal("Box[T]".to_string())
    );
}

#[test]
fn method_self_record_update_preserves_nominal_receiver_type_in_body() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec![
            "parser".to_string(),
            "core".to_string(),
        ])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![SourceItem::method_def(
            MethodReceiver::new("Box", vec!["T".to_string()]),
            "update",
            vec![
                ParamDecl::new("self", Some("Self".to_string())),
                ParamDecl::new("value", Some("T".to_string())),
            ],
            Some("Self".to_string()),
            Expr::record_update(Expr::var("self"), vec![("value", Expr::var("value"))]),
        )],
    );

    let bundle = compile_program_bundle(&program);
    let typed = &bundle.defs[0].output.typed;

    assert_eq!(typed.ty, Type::Nominal("Box[T]".to_string()));
    match &typed.kind {
        TypedExprKind::RecordUpdate { base, fields } => {
            assert_eq!(base.ty, Type::Nominal("Box[T]".to_string()));
            assert_eq!(fields[0].name, "value");
            assert_eq!(fields[0].value.ty, Type::Nominal("T".to_string()));
        }
        other => panic!(
            "expected typed record update, got {}",
            typed_expr_kind_name(other)
        ),
    }
}

#[test]
fn source_method_self_with_generics_reaches_typed_and_core_record_update() {
    let parsed = parse_source_program(
        "type Box[T] = { value: T }
def Box[T].update(self: Self, value: T): Self = { self | value: value }
def main() = 0",
    )
    .expect("parse source");

    let bundle = compile_program_bundle(&parsed.program);

    assert_eq!(bundle.diagnostics, vec![]);
    let update = &bundle
        .defs
        .iter()
        .find(|def| def.name == "update")
        .expect("update def")
        .output;

    assert_eq!(
        update.typed_signature.render("update"),
        "def update(self: Box[T], value: T): Box[T]"
    );
    assert_eq!(update.typed.ty, Type::Nominal("Box[T]".to_string()));
    match &update.typed.kind {
        TypedExprKind::RecordUpdate { base, fields } => {
            assert_eq!(base.ty, Type::Nominal("Box[T]".to_string()));
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "value");
            assert_eq!(fields[0].value.ty, Type::Nominal("T".to_string()));
        }
        other => panic!(
            "expected typed record update, got {}",
            typed_expr_kind_name(other)
        ),
    }
    assert!(update.core.ops.iter().any(|op| {
        matches!(
            op,
            CoreOp::RecordUpdate { base, layout, fields }
                if base == "self"
                    && layout == "record::value"
                    && fields == &vec!["value".to_string()]
        )
    }));
}

#[test]
fn method_self_field_access_uses_row_style_type_shape() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec![
            "parser".to_string(),
            "core".to_string(),
        ])),
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            vec!["T".to_string()],
            vec![TypeField::new("value", "T")],
        )],
        Vec::new(),
        vec![SourceItem::method_def(
            MethodReceiver::new("Box", vec!["T".to_string()]),
            "get",
            vec![ParamDecl::new("self", Some("Self".to_string()))],
            Some("T".to_string()),
            Expr::field(Expr::var("self"), "value"),
        )],
    );

    let bundle = compile_program_bundle(&program);
    let typed = &bundle.defs[0].output.typed;

    assert_eq!(typed.ty, Type::Nominal("T".to_string()));
    match &typed.kind {
        TypedExprKind::Field { receiver, name, .. } => {
            assert_eq!(receiver.ty, Type::Nominal("Box[T]".to_string()));
            assert_eq!(name, "value");
        }
        other => panic!(
            "expected typed field access, got {}",
            typed_expr_kind_name(other)
        ),
    }
}

#[test]
fn nominal_row_field_access_substitutes_concrete_type_arguments() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            vec!["T".to_string()],
            vec![TypeField::new("value", "T")],
        )],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "main".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new("box", Some("Box[i64]".to_string()))],
            return_type: Some("i64".to_string()),
            body: Expr::field(Expr::var("box"), "value"),
        }],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.defs[0].output.typed.ty, Type::I64);
}

#[test]
fn nominal_row_field_access_preserves_nested_type_arguments() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![
            TypeDecl::new(
                "Pair",
                vec!["A".to_string(), "B".to_string()],
                vec![TypeField::new("left", "A"), TypeField::new("right", "B")],
            ),
            TypeDecl::new(
                "Box",
                vec!["T".to_string()],
                vec![TypeField::new("value", "T")],
            ),
        ],
        Vec::new(),
        vec![SourceItem::Def {
            receiver: None,
            generics: Vec::new(),
            name: "main".to_string(),
            visibility: Visibility::Public,
            params: vec![ParamDecl::new(
                "box",
                Some("Box[Pair[i64,bool]]".to_string()),
            )],
            return_type: Some("Pair[i64,bool]".to_string()),
            body: Expr::field(Expr::var("box"), "value"),
        }],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.defs[0].output.typed.ty,
        Type::Nominal("Pair[i64,bool]".to_string())
    );
}

#[test]
fn auto_generic_is_lowering_fact_not_source_surface_generic() {
    let program = SourceProgram::new(vec![def("id", vec!["x"], Expr::var("x"))]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.defs[0].generics, Vec::<String>::new());
    assert_eq!(bundle.interface.functions[0].generics, Vec::<String>::new());
    assert_eq!(
        bundle.defs[0].output.template.explicit_params,
        vec![chiba_level1r::TemplateParam {
            name: "T_x".to_string(),
            source: chiba_level1r::TemplateParamSource::SyntheticAutoGeneric,
        }]
    );
    assert_eq!(
        bundle.defs[0].output.specialize.work_items[0]
            .key
            .template_params,
        bundle.defs[0].output.template.explicit_params
    );
}

#[test]
fn program_surface_and_interface_preserve_static_values_separately_from_functions() {
    let program = SourceProgram::new(vec![
        static_value("ONE", Some("i64"), Expr::i64(1)),
        def("main", vec![], Expr::var("ONE")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.surface.defs.len(), 1);
    assert_eq!(bundle.surface.defs[0].name, "main");
    assert_eq!(bundle.surface.statics.len(), 1);
    assert_eq!(bundle.surface.statics[0].name, "ONE");
    assert_eq!(bundle.surface.statics[0].ty, Some("i64".to_string()));
    assert_eq!(bundle.interface.functions[0].symbol, "root::main");
    assert_eq!(bundle.interface.statics[0].symbol, "root::ONE");
    assert_eq!(bundle.interface.statics[0].ty, Some("i64".to_string()));
    assert_eq!(bundle.defs.len(), 1);
    assert_eq!(bundle.defs[0].name, "main");
}

#[test]
fn global_init_preserves_static_owner_namespace_in_linked_symbol() {
    let program = SourceProgram::with_surface(
        Some(NamespaceDecl::new(vec![
            "compiler".to_string(),
            "core".to_string(),
        ])),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![
            static_value("CONFIG", Some("i64"), Expr::i64(8)),
            def("main", vec![], Expr::var("CONFIG")),
        ],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(bundle.global_init.statics[0].owner, "compiler.core");
    assert_eq!(bundle.global_init.statics[0].dependency_ids, Vec::new());
    assert_eq!(
        bundle
            .global_init
            .init_order_ids
            .iter()
            .map(|id| format!("{}::{}", id.owner, id.name))
            .collect::<Vec<_>>(),
        vec!["compiler.core::CONFIG".to_string()]
    );
    assert_eq!(bundle.interface.statics[0].symbol, "compiler.core::CONFIG");
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__compiler_core__CONFIG (mut i32) (i32.const 8)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("global.get $global__compiler_core__CONFIG"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("global.get $global__CONFIG"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "8");
}

#[test]
fn global_init_allows_ordered_and_forward_static_dependencies() {
    let program = SourceProgram::new(vec![
        static_value("THREE", Some("i64"), Expr::var("TWO")),
        static_value("ONE", Some("i64"), Expr::i64(1)),
        static_value(
            "TWO",
            Some("i64"),
            Expr::binary(
                chiba_level1r::ast::BinaryOp::Add,
                Expr::var("ONE"),
                Expr::i64(1),
            ),
        ),
        def("main", vec![], Expr::var("THREE")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(
        bundle
            .global_init
            .statics
            .iter()
            .map(|static_value| {
                (
                    static_value.name.as_str(),
                    static_value.dependencies.as_slice(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("THREE", &["TWO".to_string()][..]),
            ("ONE", &[][..]),
            ("TWO", &["ONE".to_string()][..]),
        ]
    );
    assert_eq!(
        bundle
            .global_init
            .statics
            .iter()
            .map(|static_value| {
                (
                    static_value.name.as_str(),
                    static_value
                        .dependency_ids
                        .iter()
                        .map(|id| format!("{}::{}", id.owner, id.name))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("THREE", vec!["root::TWO".to_string()]),
            ("ONE", Vec::<String>::new()),
            ("TWO", vec!["root::ONE".to_string()]),
        ]
    );
    assert_eq!(
        bundle.global_init.init_order,
        vec!["ONE".to_string(), "TWO".to_string(), "THREE".to_string()]
    );
    assert_eq!(
        bundle
            .global_init
            .init_order_ids
            .iter()
            .map(|id| format!("{}::{}", id.owner, id.name))
            .collect::<Vec<_>>(),
        vec![
            "root::ONE".to_string(),
            "root::TWO".to_string(),
            "root::THREE".to_string(),
        ]
    );
    assert_eq!(
        bundle.defs[0].output.backend.return_value,
        Some(chiba_level1r::core::CoreValue::Var("THREE".to_string()))
    );
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__ONE (mut i32) (i32.const 1)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__TWO (mut i32) (i32.const 0)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__THREE (mut i32) (i32.const 0)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $__chiba_init"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(start $__chiba_init)"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("global.set $global__ONE"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("global.get $global__ONE"));
    assert!(bundle.backend_link.linked_wat.contains("i32.add"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("global.set $global__TWO"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("global.set $global__THREE"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("global.get $global__THREE"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "2");
}

#[test]
fn global_init_lowers_pure_const_expressions_into_global_initializers() {
    let program = SourceProgram::new(vec![
        static_value(
            "FORTY_TWO",
            Some("i64"),
            Expr::binary(
                chiba_level1r::ast::BinaryOp::Add,
                Expr::i64(40),
                Expr::i64(2),
            ),
        ),
        static_value(
            "CHOSEN",
            Some("i64"),
            Expr::if_else(Expr::bool(false), Expr::i64(1), Expr::i64(3)),
        ),
        def("main", vec![], Expr::var("FORTY_TWO")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__FORTY_TWO (mut i32) (i32.const 42)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__CHOSEN (mut i32) (i32.const 3)"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("global.set $global__FORTY_TWO"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("global.set $global__CHOSEN"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "42");
}

#[test]
fn global_init_lowers_adt_constructor_tag_into_global_initializer() {
    let program = SourceProgram::new(vec![
        static_value(
            "CHOICE",
            Some("Option"),
            Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(1)]),
        ),
        def("main", vec![], Expr::var("CHOICE")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(global $global__CHOICE (mut i32) (i32.const 1)"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("global.set $global__CHOICE"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "1");
}

#[test]
fn global_init_rejects_unknown_adt_constructor_instead_of_faking_tag_zero() {
    let program = SourceProgram::new(vec![
        static_value(
            "CHOICE",
            Some("Option"),
            Expr::adt_ctor("Option", "Ghost", vec!["None", "Some"], vec![]),
        ),
        def("main", vec![], Expr::var("CHOICE")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::InvalidStaticAdtConstructor {
            static_name: "CHOICE".to_string(),
            data: "Option".to_string(),
            ctor: "Ghost".to_string(),
        }]
    );
    assert!(!bundle.backend_link.linked_wat.contains("global__CHOICE"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("(global $global__CHOICE (mut i32) (i32.const 0)"));
}

#[test]
fn global_init_rejects_unsupported_initializer_instead_of_faking_i32_zero() {
    let program = SourceProgram::new(vec![
        def("helper", vec![], Expr::i64(9)),
        static_value(
            "VALUE",
            Some("i64"),
            Expr::call_args(Expr::var("helper"), Vec::new()),
        ),
        def("main", vec![], Expr::var("VALUE")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::UnsupportedStaticInitializer {
            static_name: "VALUE".to_string(),
            expr: "helper()".to_string(),
        }]
    );
    let summary = bundle.render_summary();
    assert!(summary.contains("unsupported static initializer VALUE: helper()"));
    assert!(!summary.contains("Call {"));
    assert!(!summary.contains("Var("));
    assert!(!bundle.backend_link.linked_wat.contains("global__VALUE"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("unsupported static init"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("(global $global__VALUE (mut i32) (i32.const 0)"));
}

#[test]
fn global_init_rejects_unknown_static_reference_before_backend_lowering() {
    let program = SourceProgram::new(vec![
        static_value("VALUE", Some("i64"), Expr::var("MISSING")),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::UnsupportedStaticInitializer {
            static_name: "VALUE".to_string(),
            expr: "MISSING".to_string(),
        }]
    );
    let summary = bundle.render_summary();
    assert!(summary.contains("unsupported static initializer VALUE: MISSING"));
    assert!(!summary.contains("Var("));
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert!(!bundle.backend_link.linked_wat.contains("global__VALUE"));
}

#[test]
fn global_init_lowers_adt_match_into_executable_initializer() {
    let program = SourceProgram::new(vec![
        static_value(
            "MATCHED",
            Some("i64"),
            Expr::match_expr(
                Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(5)]),
                vec![
                    (
                        chiba_level1r::ast::Pattern::qualified_ctor(
                            "Option",
                            "Some",
                            vec![chiba_level1r::ast::Pattern::bind("value")],
                        ),
                        Expr::var("value"),
                    ),
                    (
                        chiba_level1r::ast::Pattern::qualified_ctor("Option", "None", Vec::new()),
                        Expr::i64(0),
                    ),
                ],
            ),
        ),
        def("main", vec![], Expr::var("MATCHED")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $__chiba_init"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("global.set $global__MATCHED"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "5");
}

#[test]
fn global_init_if_let_binder_does_not_leak_to_else_initializer_branch() {
    let program = SourceProgram::new(vec![
        static_value(
            "PICKED",
            Some("i64"),
            Expr::if_let(
                chiba_level1r::ast::Pattern::qualified_ctor(
                    "Option",
                    "Some",
                    vec![chiba_level1r::ast::Pattern::bind("value")],
                ),
                Expr::adt_ctor("Option", "None", vec!["None", "Some"], Vec::new()),
                Expr::var("value"),
                Expr::var("value"),
            ),
        ),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::UnsupportedStaticInitializer {
            static_name: "PICKED".to_string(),
            expr: "value".to_string(),
        }]
    );
    let summary = bundle.render_summary();
    assert!(summary.contains("unsupported static initializer PICKED: value"));
    assert!(!summary.contains("Var("));
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert!(!bundle.backend_link.linked_wat.contains("global__PICKED"));
}

#[test]
fn global_init_pattern_binder_does_not_create_static_dependency() {
    let program = SourceProgram::new(vec![
        static_value("value", Some("i64"), Expr::i64(1)),
        static_value(
            "MATCHED",
            Some("i64"),
            Expr::match_expr(
                Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(6)]),
                vec![
                    (
                        chiba_level1r::ast::Pattern::qualified_ctor(
                            "Option",
                            "Some",
                            vec![chiba_level1r::ast::Pattern::bind("value")],
                        ),
                        Expr::var("value"),
                    ),
                    (
                        chiba_level1r::ast::Pattern::qualified_ctor("Option", "None", Vec::new()),
                        Expr::i64(0),
                    ),
                ],
            ),
        ),
        def("main", vec![], Expr::var("MATCHED")),
    ]);

    let bundle = compile_program_bundle(&program);
    let matched = bundle
        .global_init
        .statics
        .iter()
        .find(|static_value| static_value.name == "MATCHED")
        .expect("MATCHED static");

    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(matched.dependencies, Vec::<String>::new());
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "6");
}

#[test]
fn global_init_if_let_binder_does_not_create_static_dependency() {
    let program = SourceProgram::new(vec![
        static_value("value", Some("i64"), Expr::i64(1)),
        static_value(
            "PICKED",
            Some("i64"),
            Expr::if_let(
                chiba_level1r::ast::Pattern::qualified_ctor(
                    "Option",
                    "Some",
                    vec![chiba_level1r::ast::Pattern::bind("value")],
                ),
                Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(7)]),
                Expr::var("value"),
                Expr::i64(0),
            ),
        ),
        def("main", vec![], Expr::var("PICKED")),
    ]);

    let bundle = compile_program_bundle(&program);
    let picked = bundle
        .global_init
        .statics
        .iter()
        .find(|static_value| static_value.name == "PICKED")
        .expect("PICKED static");

    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(picked.dependencies, Vec::<String>::new());
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "7");
}

#[test]
fn global_init_lambda_param_does_not_create_static_dependency() {
    let program = SourceProgram::new(vec![
        static_value("value", Some("i64"), Expr::i64(1)),
        static_value(
            "LAMBDA",
            Some("i64"),
            Expr::call(Expr::lambda("value", Expr::var("value")), Expr::i64(9)),
        ),
        def("main", vec![], Expr::var("value")),
    ]);

    let bundle = compile_program_bundle(&program);
    let lambda = bundle
        .global_init
        .statics
        .iter()
        .find(|static_value| static_value.name == "LAMBDA")
        .expect("LAMBDA static");

    assert_eq!(lambda.dependencies, Vec::<String>::new());
}

#[test]
fn global_init_lowers_record_field_access_into_executable_initializer() {
    let program = SourceProgram::new(vec![
        static_value(
            "PICKED",
            Some("i64"),
            Expr::field(
                Expr::record(vec![("x", Expr::i64(1)), ("y", Expr::i64(7))]),
                "y",
            ),
        ),
        def("main", vec![], Expr::var("PICKED")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $__chiba_init"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("global.set $global__PICKED"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("unsupported static init"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "7");
}

#[test]
fn global_init_rejects_aggregate_static_initializer_instead_of_faking_i32_zero() {
    let program = SourceProgram::new(vec![
        static_value(
            "BOX",
            None,
            Expr::record(vec![("value", Expr::i64(13)), ("ignored", Expr::i64(1))]),
        ),
        def("main", vec![], Expr::var("BOX")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.diagnostics,
        vec![ProgramDiagnostic::UnsupportedStaticInitializer {
            static_name: "BOX".to_string(),
            expr: "{value: 13, ignored: 1}".to_string(),
        }]
    );
    let summary = bundle.render_summary();
    assert!(summary.contains("unsupported static initializer BOX: {value: 13, ignored: 1}"));
    assert!(!summary.contains("RecordField"));
    assert!(!bundle.backend_link.linked_wat.contains("global__BOX"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("unsupported static init"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("(global $global__BOX (mut i32) (i32.const 0)"));
}

#[test]
fn global_init_lowers_record_update_field_access_into_executable_initializer() {
    let program = SourceProgram::new(vec![
        static_value(
            "PICKED",
            Some("i64"),
            Expr::field(
                Expr::record_update(
                    Expr::record(vec![("x", Expr::i64(1)), ("y", Expr::i64(2))]),
                    vec![("x", Expr::i64(9))],
                ),
                "x",
            ),
        ),
        def("main", vec![], Expr::var("PICKED")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $__chiba_init"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("global.set $global__PICKED"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("unsupported static init"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "9");
}

#[test]
fn global_init_lowers_if_let_adt_payload_into_executable_initializer() {
    let program = SourceProgram::new(vec![
        static_value(
            "PICKED",
            Some("i64"),
            Expr::if_let(
                chiba_level1r::ast::Pattern::qualified_ctor(
                    "Option",
                    "Some",
                    vec![chiba_level1r::ast::Pattern::bind("value")],
                ),
                Expr::adt_ctor("Option", "Some", vec!["None", "Some"], vec![Expr::i64(6)]),
                Expr::var("value"),
                Expr::i64(0),
            ),
        ),
        def("main", vec![], Expr::var("PICKED")),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $__chiba_init"));
    assert!(bundle.backend_link.linked_wat.contains("if (result i32)"));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("unsupported static init"));
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "6");
}

fn run_wat_text(wat: &str) -> String {
    run_wat_export(wat, "main")
}

fn run_wat_export(wat: &str, export: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "level1r-global-init-{}-{}.wat",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::write(&path, wat).expect("write generated wat fixture");
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("level-1r parent repo root");
    let output = Command::new("node")
        .arg("tools/node/run-wat.mjs")
        .arg(&path)
        .arg("--invoke")
        .arg(export)
        .current_dir(repo_root)
        .output()
        .expect("run generated wat");
    assert!(
        output.status.success(),
        "generated WAT failed\nstdout:\n{}\nstderr:\n{}\nwat:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        wat
    );
    String::from_utf8(output.stdout)
        .expect("wat stdout utf8")
        .trim()
        .to_string()
}

#[test]
fn global_init_reports_cycles_and_duplicate_static_names() {
    let program = SourceProgram::new(vec![
        static_value("A", None, Expr::var("B")),
        static_value("B", None, Expr::var("A")),
        static_value("A", None, Expr::i64(0)),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateStatic {
            name: "A".to_string(),
        }));
    assert!(bundle.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            ProgramDiagnostic::StaticInitCycle { cycle }
                if cycle == &vec!["A".to_string(), "B".to_string(), "A".to_string()]
        )
    }));
}

#[test]
fn global_init_reports_self_referential_static_cycle() {
    let program = SourceProgram::new(vec![
        static_value("A", None, Expr::var("A")),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert_eq!(
        bundle.global_init.statics[0].dependencies,
        vec!["A".to_string()]
    );
    assert!(bundle.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            ProgramDiagnostic::StaticInitCycle { cycle }
                if cycle == &vec!["A".to_string(), "A".to_string()]
        )
    }));
    assert!(!bundle
        .backend_link
        .linked_wat
        .contains("global.get $global__A"));
}

#[test]
fn global_init_reports_static_function_name_conflict() {
    let program = SourceProgram::new(vec![
        static_value("main", Some("i64"), Expr::i64(1)),
        def("main", vec![], Expr::i64(0)),
    ]);

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::StaticFunctionNameConflict {
            name: "main".to_string(),
        }));
}

#[test]
fn program_surface_reports_duplicate_data_and_constructor_names() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![
            DataDecl::new(
                "A",
                Vec::new(),
                vec![
                    DataVariant::new("Same", Vec::new()),
                    DataVariant::new("Same", Vec::new()),
                ],
            ),
            DataDecl::new("A", Vec::new(), vec![DataVariant::new("Other", Vec::new())]),
        ],
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateData {
            name: "A".to_string(),
        }));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateConstructor {
            name: "Same".to_string(),
        }));
}

#[test]
fn program_surface_reports_duplicate_type_names() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![
            TypeDecl::new("Box", Vec::new(), vec![TypeField::new("value", "i64")]),
            TypeDecl::new("Box", Vec::new(), vec![TypeField::new("other", "i64")]),
        ],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateType {
            name: "Box".to_string(),
        }));
}

#[test]
fn program_surface_reports_duplicate_ordinary_type_fields() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            Vec::new(),
            vec![
                TypeField::new("value", "i64"),
                TypeField::new("value", "bool"),
                TypeField::new("_", "PhantomA"),
                TypeField::new("_", "PhantomB"),
            ],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateTypeField {
            type_name: "Box".to_string(),
            field: "value".to_string(),
        }));
    assert!(!bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateTypeField {
            type_name: "Box".to_string(),
            field: "_".to_string(),
        }));
}

#[test]
fn phantom_type_fields_are_not_available_for_field_access() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "Box",
            Vec::new(),
            vec![
                TypeField::new("_", "PhantomA"),
                TypeField::new("_", "PhantomB"),
            ],
        )],
        Vec::new(),
        vec![
            SourceItem::Def {
                receiver: None,
                generics: Vec::new(),
                name: "probe".to_string(),
                visibility: Visibility::Public,
                params: vec![ParamDecl::new("box", Some("Box".to_string()))],
                return_type: None,
                body: Expr::field(Expr::var("box"), "_"),
            },
            def("main", vec![], Expr::i64(0)),
        ],
    );

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(bundle.defs[0].output.typed.ty, Type::Unknown);
}

#[test]
fn program_surface_reports_type_topdef_name_conflicts() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![
            TypeDecl::new("Thing", Vec::new(), vec![TypeField::new("value", "i64")]),
            TypeDecl::new("Config", Vec::new(), vec![TypeField::new("value", "i64")]),
            TypeDecl::new("Global", Vec::new(), vec![TypeField::new("value", "i64")]),
        ],
        vec![DataDecl::new(
            "Thing",
            Vec::new(),
            vec![DataVariant::new("Made", Vec::new())],
        )],
        vec![
            def("Config", vec![], Expr::i64(0)),
            static_value("Global", Some("i64"), Expr::i64(1)),
        ],
    );

    let bundle = compile_program_bundle(&program);

    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "Thing".to_string(),
        }));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "Config".to_string(),
        }));
    assert!(bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateTopLevelName {
            name: "Global".to_string(),
        }));
}

#[test]
fn program_surface_allows_same_constructor_name_across_different_data() {
    let program = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![
            DataDecl::new(
                "Left",
                Vec::new(),
                vec![DataVariant::new("Same", Vec::new())],
            ),
            DataDecl::new(
                "Right",
                Vec::new(),
                vec![DataVariant::new("Same", Vec::new())],
            ),
        ],
        vec![def("main", vec![], Expr::i64(0))],
    );

    let bundle = compile_program_bundle(&program);

    assert!(!bundle
        .diagnostics
        .contains(&ProgramDiagnostic::DuplicateConstructor {
            name: "Same".to_string(),
        }));
    assert_eq!(
        bundle
            .interface
            .constructors
            .iter()
            .map(|ctor| ctor.symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["root::Left.Same", "root::Right.Same"]
    );
}

#[test]
fn interface_summary_hash_changes_when_constructor_arity_changes() {
    let one_field = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Box",
            Vec::new(),
            vec![DataVariant::new("Wrap", vec!["I64".to_string()])],
        )],
        vec![def("main", vec![], Expr::i64(0))],
    );
    let two_fields = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Box",
            Vec::new(),
            vec![DataVariant::new(
                "Wrap",
                vec!["I64".to_string(), "Bool".to_string()],
            )],
        )],
        vec![def("main", vec![], Expr::i64(0))],
    );

    let one_hash = compile_program_bundle(&one_field).interface.stable_hash;
    let two_hash = compile_program_bundle(&two_fields).interface.stable_hash;

    assert_ne!(one_hash, two_hash);
}

#[test]
fn interface_summary_hash_changes_when_constructor_payload_type_changes() {
    let i64_payload = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Box",
            Vec::new(),
            vec![DataVariant::new("Wrap", vec!["I64".to_string()])],
        )],
        vec![def("main", vec![], Expr::i64(0))],
    );
    let bool_payload = SourceProgram::with_surface(
        None,
        Vec::new(),
        Vec::new(),
        vec![DataDecl::new(
            "Box",
            Vec::new(),
            vec![DataVariant::new("Wrap", vec!["Bool".to_string()])],
        )],
        vec![def("main", vec![], Expr::i64(0))],
    );

    let i64_hash = compile_program_bundle(&i64_payload).interface.stable_hash;
    let bool_hash = compile_program_bundle(&bool_payload).interface.stable_hash;

    assert_ne!(i64_hash, bool_hash);
}

#[test]
fn interface_summary_hash_changes_when_type_phantom_marker_changes() {
    let user_marker = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "User",
            Vec::new(),
            vec![
                TypeField::new("id", "i64"),
                TypeField::new("_", "UserMarker"),
            ],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );
    let admin_marker = SourceProgram::with_surface(
        None,
        Vec::new(),
        vec![TypeDecl::new(
            "User",
            Vec::new(),
            vec![
                TypeField::new("id", "i64"),
                TypeField::new("_", "AdminMarker"),
            ],
        )],
        Vec::new(),
        vec![def("main", vec![], Expr::i64(0))],
    );

    let user_hash = compile_program_bundle(&user_marker).interface.stable_hash;
    let admin_hash = compile_program_bundle(&admin_marker).interface.stable_hash;

    assert_ne!(user_hash, admin_hash);
}

#[test]
fn program_bundle_reports_missing_entry_for_empty_program() {
    let program = SourceProgram::default();

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.entry, None);
    assert_eq!(bundle.diagnostics, vec![ProgramDiagnostic::MissingEntry]);
    assert!(bundle.backend_link.linked_wat.starts_with("(module\n"));
}

#[test]
fn program_summary_contains_program_level_nanopass_events() {
    let program = SourceProgram::new(vec![def("main", vec![], Expr::i64(7))]);

    let bundle = compile_program_bundle(&program);
    let summary = bundle.render_summary();

    assert!(summary.contains("program:"));
    assert!(summary.contains("namespace=<root>"));
    assert!(summary.contains("imports=<none>"));
    assert!(summary.contains("defs=1"));
    assert!(summary.contains("entry=main"));
    assert!(summary.contains("P1ProjectSurface: SourceProgram -> ProjectSurface"));
    assert!(summary.contains("P2InterfaceSummary: ProjectSurface -> InterfaceSummary"));
    assert!(summary.contains("P3ProgramDiagnostics: ProjectSurface -> ProgramDiagnostics"));
    assert!(summary.contains("P4GlobalInit: SourceProgram+ProjectSurface -> GlobalInitPlan"));
    assert!(summary.contains("P5ProgramDefs: SourceProgram+InterfaceSummary -> ProgramDefOutput"));
    assert!(summary.contains("P6ProgramEntry: ProgramDefOutput -> EntrySelection"));
    assert!(summary.contains(
        "P7ProgramBackendLink: ProgramDefOutput+EntrySelection+GlobalInitPlan -> BackendLinkedBundle"
    ));
    assert!(summary.contains("P8ProgramBackendCacheKey: BackendLinkedBundle -> BackendCacheKey"));
    assert!(summary.contains("global-init:"));
    assert!(summary.contains("statics=0"));
    assert!(summary.contains("init-order=[]"));
    assert!(summary.contains("diagnostics=0"));
    assert!(!summary.contains("global-init=GlobalInitPlan"));
    assert!(!summary.contains("entry=Some"));
    assert!(!summary.contains("imports=[]"));
    assert!(!summary.contains("body:"));
    assert!(!summary.contains("Lit("));
}

#[test]
fn program_summary_renders_global_init_without_source_ast_debug() {
    let program = SourceProgram::new(vec![
        static_value(
            "TWO",
            Some("i64"),
            Expr::binary(
                chiba_level1r::ast::BinaryOp::Add,
                Expr::i64(1),
                Expr::i64(1),
            ),
        ),
        static_value("THREE", Some("i64"), Expr::var("TWO")),
        def("main", vec![], Expr::var("THREE")),
    ]);

    let bundle = compile_program_bundle(&program);
    let summary = bundle.render_summary();

    assert!(summary.contains("global-init:"));
    assert!(summary.contains("static root::TWO ty=i64 init=1 + 1"));
    assert!(summary.contains("static root::THREE ty=i64 init=TWO"));
    assert!(summary.contains("deps=[root::TWO]"));
    assert!(summary.contains("init-order=[root::TWO, root::THREE]"));
    assert!(!summary.contains("Binary {"));
    assert!(!summary.contains("Lit("));
    assert!(!summary.contains("Var("));
}
