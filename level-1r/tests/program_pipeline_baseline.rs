use chiba_level1r::ast::{SourceItem, SourceProgram};
use chiba_level1r::{compile_program, compile_program_bundle, Expr, ProgramDiagnostic};

fn def(name: &str, params: Vec<&str>, body: Expr) -> SourceItem {
    SourceItem::Def {
        name: name.to_string(),
        params: params.into_iter().map(str::to_string).collect(),
        body,
    }
}

#[test]
fn compile_program_keeps_legacy_per_def_outputs() {
    let program = SourceProgram {
        items: vec![def("one", vec![], Expr::i64(1)), def("two", vec![], Expr::i64(2))],
    };

    let outputs = compile_program(&program);

    assert_eq!(outputs.len(), 2);
    assert!(outputs[0].backend.wat.contains("i32.const 1"));
    assert!(outputs[1].backend.wat.contains("i32.const 2"));
}

#[test]
fn program_bundle_selects_main_and_links_def_artifacts() {
    let program = SourceProgram {
        items: vec![
            def("helper", vec![], Expr::i64(1)),
            def("main", vec![], Expr::i64(7)),
        ],
    };

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.entry, Some("main".to_string()));
    assert_eq!(bundle.diagnostics, vec![]);
    assert_eq!(bundle.defs.len(), 2);
    assert!(bundle.backend_link.diagnostics.is_empty());
    assert!(bundle.backend_link.linked_wat.contains("(func $helper (result i32)"));
    assert!(bundle
        .backend_link
        .linked_wat
        .contains("(func $main__def1 (export \"main\") (result i32)"));
    assert!(bundle.backend_link.linked_wat.contains("i32.const 1"));
    assert!(bundle.backend_link.linked_wat.contains("i32.const 7"));
}

#[test]
fn program_bundle_reports_duplicate_defs_and_entry_params() {
    let program = SourceProgram {
        items: vec![
            def("main", vec!["x"], Expr::var("x")),
            def("main", vec![], Expr::i64(0)),
        ],
    };

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
    assert_eq!(bundle.backend_link.linked_wat.matches("(export \"main\")").count(), 1);
    assert!(bundle.backend_link.linked_wat.contains("(func $main__def1 (result i32)"));
}

#[test]
fn program_bundle_reports_missing_entry_for_empty_program() {
    let program = SourceProgram { items: vec![] };

    let bundle = compile_program_bundle(&program);

    assert_eq!(bundle.entry, None);
    assert_eq!(bundle.diagnostics, vec![ProgramDiagnostic::MissingEntry]);
    assert!(bundle.backend_link.linked_wat.starts_with("(module\n"));
}

#[test]
fn program_summary_contains_program_level_nanopass_events() {
    let program = SourceProgram {
        items: vec![def("main", vec![], Expr::i64(7))],
    };

    let bundle = compile_program_bundle(&program);
    let summary = bundle.render_summary();

    assert!(summary.contains("program:"));
    assert!(summary.contains("defs=1"));
    assert!(summary.contains("entry=Some(\"main\")"));
    assert!(summary.contains("P1ProgramSurface: SourceProgram -> ProgramDiagnostics"));
    assert!(summary.contains("P2ProgramDefs: SourceProgram -> ProgramDefOutput"));
    assert!(summary.contains("P3ProgramEntry: ProgramDefOutput -> EntrySelection"));
    assert!(summary.contains(
        "P4ProgramBackendLink: ProgramDefOutput+EntrySelection -> BackendLinkedBundle"
    ));
    assert!(summary.contains("P5ProgramBackendCacheKey: BackendLinkedBundle -> BackendCacheKey"));
}
