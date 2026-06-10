use std::fs;
use std::path::PathBuf;

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

    assert!(audit.contains("| Performance | Missing |"));
    assert!(audit.contains("| Self-bootstrap | Missing |"));
    assert!(audit.contains("| **并发实例化注册表** | Missing |"));
    assert!(audit.contains("| **增量缓存** | Missing |"));
    assert!(audit.contains("| **namespace 并行区** | Missing |"));
}
