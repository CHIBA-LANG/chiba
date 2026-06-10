use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn level_1r_project_doc_records_rc_to_n_visual_oracle() {
    let doc = fs::read_to_string(repo_root().join("level-1r.md")).unwrap();

    assert!(doc.contains("Rc<T>` corresponds to `N T`"));
    assert!(doc.contains("source / typed / usage / rust-reference"));
}

#[test]
fn todo_records_current_wasm_gc_and_future_no_gc_wasm_targets() {
    let p0_todo = fs::read_to_string(repo_root().join("TODO.level1-b.md")).unwrap();
    let longterm_todo = fs::read_to_string(repo_root().join("TODO.longterm.md")).unwrap();

    assert!(p0_todo.contains("wasm-gc Core/backend rewrite"));
    assert!(p0_todo.contains("meta-level continuation"));
    assert!(longterm_todo.contains("T(expr, k_meta)"));
    assert!(longterm_todo.contains("one-pass"));
}

#[test]
fn p0_audit_covers_every_agents_checkpoint_item() {
    let agents = fs::read_to_string(repo_root().join("AGENTS.md")).unwrap();
    let audit = fs::read_to_string(repo_root().join("P0_AUDIT.md")).unwrap();

    for line in agents.lines().filter(|line| line.starts_with("- [ ] ")) {
        let item = line
            .trim_start_matches("- [ ] ")
            .trim()
            .trim_matches('*')
            .trim();
        assert!(
            audit.contains(item),
            "P0_AUDIT.md is missing AGENTS checkpoint item: {item}"
        );
    }

    assert!(audit.contains("| Performance | Partial |"));
    assert!(audit.contains("| Self-bootstrap | Missing |"));
    assert!(audit.contains("| **并发实例化注册表** | Missing |"));
    assert!(audit.contains("| **增量缓存** | Missing |"));
    assert!(audit.contains("| **namespace 并行区** | Missing |"));
}

#[test]
fn chiba_spec_tree_is_tracked_and_every_fixture_has_expectations() {
    let spec_root = repo_root().join("chiba-spec");
    let fixtures = collect_chiba_spec_fixtures(&spec_root);

    assert!(
        fixtures.len() >= 40,
        "expected broad chiba-spec corpus, got {} fixtures",
        fixtures.len()
    );

    for path in &fixtures {
        let source = fs::read_to_string(path).unwrap();
        let relative = path.strip_prefix(repo_root()).unwrap().display();
        assert!(
            source.contains("// expect:"),
            "{relative} must declare at least one // expect: oracle"
        );
    }

    let corpus = fixtures
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect::<Vec<_>>()
        .join("\n");

    for required in [
        "deeply nested resetn captures ContN",
        "ContN capture includes local, global, parameter, branch, match, tuple, record, ADT payload, closure env",
        "function parameter patterns are source-level language",
        "duplicate-pattern-binding",
        "row member obligation accepts either callable field or receiver method",
        "dyn row package stores payload plus adapter",
        "String indexing uses UTF-8 byte indexing",
        "String.char_at(n) returns rune",
        "no semantic pass may infer syntax, identifier class, namespace, callable, static/global name, tuple, method, or intrinsic identity from string shape",
        "lexer/chibalex/regex tokenization and shared XID tables are the only identifier legality authorities",
    ] {
        assert!(
            corpus.contains(required),
            "chiba-spec corpus is missing required oracle text: {required}"
        );
    }
}

fn collect_chiba_spec_fixtures(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_chiba_spec_fixtures_into(root, &mut out);
    out.sort();
    out
}

fn collect_chiba_spec_fixtures_into(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_chiba_spec_fixtures_into(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("chiba") {
            out.push(path);
        }
    }
}
