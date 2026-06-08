use std::process::Command;

use chiba_level1r::ast::Pattern;
use chiba_level1r::core::{CompilerIntrinsic, CoreOp, LayoutKind};
use chiba_level1r::pattern::PatternDiagnostic;
use chiba_level1r::typed::{type_expr, Type, TypedExprKind};
use chiba_level1r::{compile_expr, Expr};

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
        TypedExprKind::DynRowPackage { .. } => "dyn-row-package",
        TypedExprKind::DynRowField { .. } => "dyn-row-field",
        TypedExprKind::AdtCtor { .. } => "adt-ctor",
        TypedExprKind::AdtToTuple { .. } => "adt-to-tuple",
        TypedExprKind::TupleToAdt { .. } => "tuple-to-adt",
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

fn option_some(value: Expr) -> Expr {
    Expr::adt_ctor("Option", "Some", vec!["Some", "None"], vec![value])
}

fn option_none() -> Expr {
    Expr::adt_ctor("Option", "None", vec!["Some", "None"], vec![])
}

fn run_wat_text(wat: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "level1r-adt-{}-{}.wat",
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
        .arg("main")
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
fn adt_constructor_expression_carries_known_data_shape() {
    let typed = type_expr(&option_some(Expr::i64(1)));

    assert_eq!(
        typed.ty,
        Type::Adt {
            name: "Option".to_string(),
            variants: vec!["None".to_string(), "Some".to_string()],
        }
    );
    match typed.kind {
        TypedExprKind::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            assert_eq!(data, "Option");
            assert_eq!(ctor, "Some");
            assert_eq!(variants, vec!["None".to_string(), "Some".to_string()]);
            assert_eq!(args.len(), 1);
        }
        other => panic!(
            "expected ADT ctor expression, got {}",
            typed_expr_kind_name(&other)
        ),
    }
}

#[test]
fn adt_constructor_reaches_cps_core_and_target_neutral_layout() {
    let output = compile_expr(&option_some(Expr::i64(1)));

    assert_eq!(output.cps.to_string(), "halt Option.Some(1)");
    assert!(output.core.ops.contains(&CoreOp::AdtConstruct {
        data: "Option".to_string(),
        ctor: "Some".to_string(),
        variants: vec!["None".to_string(), "Some".to_string()],
        args: vec!["1".to_string()],
    }));
    assert!(output.core.ops.contains(&CoreOp::AdtTupleBridge {
        data: "Option".to_string(),
        ctor: "Some".to_string(),
        tuple_fields: vec![":Some".to_string(), "1".to_string()],
        tuple_to_adt_intrinsic: CompilerIntrinsic::TupleToAdt,
        adt_to_tuple_intrinsic: CompilerIntrinsic::AdtToTuple,
    }));
    assert_eq!(
        CompilerIntrinsic::TupleToAdt.owner_namespace(),
        "compiler.intrinsic"
    );
    assert!(output.core.ops.contains(&CoreOp::CompilerIntrinsicUse {
        intrinsic: CompilerIntrinsic::TupleToAdt,
        owner_namespace: "compiler.intrinsic".to_string(),
        subject: "Option.Some".to_string(),
    }));
    assert!(output.core.ops.contains(&CoreOp::CompilerIntrinsicUse {
        intrinsic: CompilerIntrinsic::AdtToTuple,
        owner_namespace: "compiler.intrinsic".to_string(),
        subject: "Option.Some".to_string(),
    }));
    assert!(output.core.layouts.iter().any(|layout| {
        matches!(
            &layout.kind,
            LayoutKind::AdtShape(shape)
                if shape.data == "Option"
                    && shape.variants == vec!["None".to_string(), "Some".to_string()]
        )
    }));
    assert_eq!(output.core_validation.diagnostics, vec![]);
    assert!(output.render_visual().contains("Option.Some"));
    assert!(output.backend.wat.contains(";; adt-tuple-bridge"));
    assert!(output
        .backend
        .wat
        .contains("compiler.intrinsic.tuple_to_adt"));
    assert!(output
        .backend
        .wat
        .contains("compiler.intrinsic.adt_to_tuple"));
    assert!(output.backend.wat.contains(";; compiler-intrinsic"));
}

#[test]
fn adt_to_tuple_intrinsic_surface_extracts_payload_through_tuple_field() {
    let output = chiba_level1r::compile_source_program_bundle(
        r#"
data Option[T] = { Some(T), None }
def main(): i64 = adt_to_tuple(Option.Some(41))._2
"#,
    )
    .expect("compile source");
    let bundle = output.program;
    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main def");

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(main.output.core_validation.diagnostics.is_empty());
    assert!(
        main.output.backend.diagnostics.is_empty(),
        "{:?}",
        main.output.backend.diagnostics
    );
    assert!(main
        .output
        .visual
        .typed
        .contains("node kind=adt-to-tuple type=Tuple[_, i64]"));
    assert!(
        main.output.core.ops.iter().any(|op| matches!(
            op,
            CoreOp::CompilerIntrinsicUse {
                intrinsic: CompilerIntrinsic::AdtToTuple,
                owner_namespace,
                subject,
            } if owner_namespace == "compiler.intrinsic" && subject.contains("Option.Some")
        )),
        "{:?}",
        main.output.core.ops
    );
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "41");
}

#[test]
fn adt_tuple_bridge_roundtrips_back_to_adt_and_executes_match() {
    let output = chiba_level1r::compile_source_program_bundle(
        r#"
data Option[T] = { Some(T), None }
def main(): i64 = match tuple_to_adt(adt_to_tuple(Option.Some(41))) { Some(value) => value, None => 0 }
"#,
    )
    .expect("compile source");
    let bundle = output.program;
    let main = bundle
        .defs
        .iter()
        .find(|def| def.name == "main")
        .expect("main def");

    assert_eq!(bundle.diagnostics, vec![]);
    assert!(main.output.core_validation.diagnostics.is_empty());
    assert!(
        main.output.backend.diagnostics.is_empty(),
        "{:?}",
        main.output.backend.diagnostics
    );
    assert!(
        main.output
            .visual
            .typed
            .contains("node kind=tuple-to-adt type=Option[i64]"),
        "{}",
        main.output.visual.typed
    );
    assert!(
        main.output.core.ops.iter().any(|op| matches!(
            op,
            CoreOp::CompilerIntrinsicUse {
                intrinsic: CompilerIntrinsic::TupleToAdt,
                owner_namespace,
                subject,
            } if owner_namespace == "compiler.intrinsic" && subject.contains("adt_to_tuple")
        )),
        "{:?}",
        main.output.core.ops
    );
    assert_eq!(run_wat_text(&bundle.backend_link.linked_wat), "41");
}

#[test]
fn constructor_pattern_collects_nested_bindings() {
    let output = compile_expr(&Expr::if_let(
        Pattern::ctor("Some", vec![Pattern::bind("value")]),
        option_some(Expr::i64(1)),
        Expr::var("value"),
        Expr::i64(0),
    ));

    assert_eq!(output.pattern.envs.len(), 1);
    assert_eq!(output.pattern.envs[0].bindings, vec!["value".to_string()]);
    assert!(output.cps.to_string().contains("Some(value) => join"));
}

#[test]
fn adt_match_is_exhaustive_when_all_known_variants_are_covered() {
    let output = compile_expr(&Expr::match_expr(
        option_some(Expr::i64(1)),
        vec![
            (
                Pattern::ctor("Some", vec![Pattern::bind("value")]),
                Expr::var("value"),
            ),
            (Pattern::ctor("None", vec![]), Expr::i64(0)),
        ],
    ));

    assert_eq!(output.pattern.diagnostics, vec![]);
    assert_eq!(output.pattern.matches.len(), 1);
    let fact = &output.pattern.matches[0];
    assert_eq!(
        fact.covered_constructors,
        vec!["Some".to_string(), "None".to_string()]
    );
    assert_eq!(fact.exhaustive, true);
}

#[test]
fn adt_match_reports_missing_constructor_with_qualified_pattern() {
    let output = compile_expr(&Expr::match_expr(
        option_some(Expr::i64(1)),
        vec![(
            Pattern::ctor("Some", vec![Pattern::bind("value")]),
            Expr::var("value"),
        )],
    ));

    assert_eq!(
        output.pattern.diagnostics,
        vec![PatternDiagnostic::NonExhaustiveMatch {
            scrutinee_type: Type::Adt {
                name: "Option".to_string(),
                variants: vec!["None".to_string(), "Some".to_string()],
            },
            missing: vec![Pattern::qualified_ctor("Option", "None", vec![])],
        }]
    );
}

#[test]
fn qualified_constructor_pattern_displays_stably() {
    let output = compile_expr(&Expr::match_expr(
        option_none(),
        vec![
            (
                Pattern::qualified_ctor("Option", "Some", vec![Pattern::bind("value")]),
                Expr::var("value"),
            ),
            (
                Pattern::qualified_ctor("Option", "None", vec![]),
                Expr::i64(0),
            ),
        ],
    ));

    assert!(output
        .cps
        .to_string()
        .contains("Option.Some(value) => join"));
    assert!(output.cps.to_string().contains("Option.None => join"));
    assert_eq!(output.pattern.diagnostics, vec![]);
}
