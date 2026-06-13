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
fn agents_records_current_p0_step1_and_step2_work() {
    let agents = fs::read_to_string(repo_root().join("AGENTS.md")).unwrap();

    assert!(agents.contains("## P0 Step 1：非性能剩余落地"));
    assert!(agents.contains("## P0 Step 2：编译加速 / 并行 / 增量"));

    for required in [
        "checked generics / template 完整化",
        "continuation / callable / capability 安全矩阵",
        "ownership / layout / runtime lowering 完整化",
        "symbol manifest / debug map / namespace ownership",
        "multi-namespace backend / link / ABI",
        "no-GC wasm 或共享 target-neutral lowering 证明",
        "legacy lowering shortcut 清理",
        "稳定 timing / threshold gate",
        "ProjectSurface / InterfaceSummary 增量缓存",
        "并发实例化注册表",
        "namespace / body 并行区",
        "specialization 并行区",
    ] {
        assert!(
            agents.contains(required),
            "AGENTS.md is missing current P0 work item: {required}"
        );
    }

    let step1 = agents
        .split("## P0 Step 1：非性能剩余落地")
        .nth(1)
        .and_then(|rest| rest.split("## P0 Step 2：编译加速 / 并行 / 增量").next())
        .unwrap();
    let step2 = agents
        .split("## P0 Step 2：编译加速 / 并行 / 增量")
        .nth(1)
        .unwrap();

    assert!(step1.contains("non-escaping") || step1.contains("非逃逸") || step1.contains("语义"));
    assert!(step2.contains("性能优化不能通过削弱 executable WAT 覆盖"));
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

#[test]
fn chiba_spec_oracles_use_stable_categories() {
    let spec_root = repo_root().join("chiba-spec");
    let fixtures = collect_chiba_spec_fixtures(&spec_root);
    let readme = fs::read_to_string(spec_root.join("README.md")).unwrap();

    assert!(readme.contains("// expect: mixed-outcomes"));
    assert!(readme.contains("// expect: warning ..."));
    assert!(readme.contains("// backend-matrix: wasm-gc, wasm32-nogc"));

    let forbidden = [
        "aggregate-file",
        "reject or",
        " or missing-",
        " unless ",
        "future runner",
        "current level-1",
        "when support matrix permits",
    ];

    for path in &fixtures {
        let source = fs::read_to_string(path).unwrap();
        let relative = path.strip_prefix(repo_root()).unwrap().display();
        for (line_index, line) in source.lines().enumerate() {
            if !line.contains("// expect:") {
                continue;
            }
            for pattern in forbidden {
                assert!(
                    !line.contains(pattern),
                    "{relative}:{} contains unstable oracle wording `{pattern}`: {line}",
                    line_index + 1
                );
            }
            let oracle = line.split_once("// expect:").unwrap().1.trim();
            let kind = oracle.split_whitespace().next().unwrap_or("");
            assert!(
                matches!(
                    kind,
                    "accept" | "reject" | "warning" | "runtime" | "fact" | "mixed-outcomes"
                ),
                "{relative}:{} uses unknown oracle kind `{kind}`",
                line_index + 1
            );
            if kind == "reject" {
                assert!(
                    oracle.split_whitespace().nth(1).is_some(),
                    "{relative}:{} reject oracle must include a stable diagnostic category",
                    line_index + 1
                );
            }
        }
    }
}

#[test]
fn chiba_spec_marks_current_backend_matrix_fixtures() {
    let spec_root = repo_root().join("chiba-spec");
    let fixtures = collect_chiba_spec_fixtures(&spec_root);
    let marked = fixtures
        .iter()
        .filter(|path| {
            fs::read_to_string(path)
                .unwrap()
                .contains("// backend-matrix:")
        })
        .collect::<Vec<_>>();

    assert!(
        !marked.is_empty(),
        "at least one chiba-spec fixture must opt into current backend matrix"
    );
}

#[test]
fn aggregate_declaration_examples_use_equals_before_body() {
    let root = repo_root();
    let mut files = Vec::new();

    collect_files_with_extensions(&root.join("chiba-spec"), &["chiba", "md"], &mut files);
    collect_files_with_extensions(
        &root
            .join("../chiba-org-web/src/content/chiba-level1-spec")
            .canonicalize()
            .unwrap(),
        &["md", "chiba"],
        &mut files,
    );
    collect_files_with_extensions(
        &root
            .join("../chiba-org-web/src/content/type_system")
            .canonicalize()
            .unwrap(),
        &["md", "chiba"],
        &mut files,
    );
    collect_files_with_extensions(&root.join("level-1r/src"), &["rs"], &mut files);
    collect_files_with_extensions(&root.join("level-1r/tests"), &["rs"], &mut files);
    files.push(root.join("AGENTS.md"));

    let mut bad = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        for (line_index, line) in source.lines().enumerate() {
            if aggregate_decl_line_without_equals(line) {
                let relative = path.strip_prefix(&root).unwrap_or(&path);
                bad.push(format!("{}:{}", relative.display(), line_index + 1));
            }
        }
    }

    assert!(
        bad.is_empty(),
        "aggregate declarations must use `type/data/union Name = {{ ... }}`: {}",
        bad.join(", ")
    );
}

#[test]
fn non_lexer_passes_do_not_use_ascii_shape_semantics() {
    let root = repo_root();
    let src = root.join("level-1r/src");
    let mut files = Vec::new();
    collect_files_with_extensions(&src, &["rs"], &mut files);

    let allowed_paths = [
        "level-1r/src/chibalex/",
        "level-1r/src/frontend/",
        "level-1r/src/regex/",
        "level-1r/src/tooling/symbol.rs",
    ];
    let forbidden = [
        ".is_ascii",
        "is_ascii_",
        "to_ascii_",
        "make_ascii_",
        "to_uppercase()",
        "to_lowercase()",
    ];
    let mut bad = Vec::new();

    for path in files {
        let relative = path.strip_prefix(&root).unwrap();
        let relative_text = relative.to_string_lossy();
        if allowed_paths
            .iter()
            .any(|allowed| relative_text.starts_with(allowed))
        {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        for (line_index, line) in source.lines().enumerate() {
            if forbidden.iter().any(|pattern| line.contains(pattern)) {
                bad.push(format!("{}:{}", relative.display(), line_index + 1));
            }
        }
    }

    assert!(
        bad.is_empty(),
        "semantic/backend/lowering passes must not infer semantics from ASCII string shape: {}",
        bad.join(", ")
    );
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

fn collect_files_with_extensions(root: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files_with_extensions(&path, extensions, out);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| extensions.contains(&ext))
        {
            out.push(path);
        }
    }
}

fn aggregate_decl_line_without_equals(line: &str) -> bool {
    let trimmed = line.trim_start_matches(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '"' | '`' | '\'' | '-' | '*' | '>' | ':' | '(' | '[' | '{'
            )
    });
    let Some(rest) = trimmed
        .strip_prefix("type ")
        .or_else(|| trimmed.strip_prefix("data "))
        .or_else(|| trimmed.strip_prefix("union "))
    else {
        return false;
    };
    let Some(body_start) = rest.find('{') else {
        return false;
    };
    let header = rest[..body_start].trim();
    if header.is_empty()
        || header.contains('`')
        || header.contains("{}")
        || header.contains("::")
        || !header
            .chars()
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_alphabetic())
    {
        return false;
    }
    !header.contains('=')
}
