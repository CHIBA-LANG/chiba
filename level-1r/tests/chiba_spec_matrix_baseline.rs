use chiba_level1r::backend::BackendTarget;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn chiba_spec_backend_matrix_compiles_marked_fixtures() {
    let spec_root = repo_root().join("chiba-spec");
    let fixtures = collect_chiba_spec_fixtures(&spec_root);
    let mut marked = Vec::new();

    for path in fixtures {
        let source = fs::read_to_string(&path).unwrap();
        let Some(targets) = backend_matrix_targets(&source) else {
            continue;
        };
        let compilable_source = strip_chiba_spec_metadata_comments(&source);
        marked.push(path.clone());
        for target in targets {
            let output =
                chiba_level1r::compile_source_program_bundle_for_target(&compilable_source, target)
                    .unwrap_or_else(|err| {
                        panic!(
                            "{} failed frontend parse for {target:?}: {}",
                            path.strip_prefix(repo_root()).unwrap_or(&path).display(),
                            chiba_level1r::render_frontend_error(&compilable_source, &err)
                        )
                    });
            assert!(
                output.program.diagnostics.is_empty(),
                "{} produced program diagnostics for {target:?}: {:?}",
                path.strip_prefix(repo_root()).unwrap_or(&path).display(),
                output.program.diagnostics
            );
            assert!(
                output
                    .program
                    .defs
                    .iter()
                    .all(|def| def.output.backend.diagnostics.is_empty()),
                "{} produced backend diagnostics for {target:?}: {}",
                path.strip_prefix(repo_root()).unwrap_or(&path).display(),
                render_def_backend_diagnostics(&output.program.defs)
            );
            assert!(
                output.program.backend_link.diagnostics.is_empty(),
                "{} produced backend link diagnostics for {target:?}: {:?}",
                path.strip_prefix(repo_root()).unwrap_or(&path).display(),
                output.program.backend_link.diagnostics
            );
            assert_eq!(
                output.program.backend_link.target,
                target,
                "{} linked wrong backend target",
                path.strip_prefix(repo_root()).unwrap_or(&path).display()
            );
            assert_eq!(
                output.program.backend_cache_key.target,
                target,
                "{} used wrong backend cache target",
                path.strip_prefix(repo_root()).unwrap_or(&path).display()
            );
            if target == BackendTarget::Wasm32NoGc {
                assert_no_wasm_gc_terms(&output.program.backend_link.linked_wat, &path);
            }
        }
    }

    assert!(
        !marked.is_empty(),
        "at least one chiba-spec fixture must opt into // backend-matrix"
    );
}

#[test]
fn chiba_spec_backend_matrix_metadata_uses_known_targets() {
    let spec_root = repo_root().join("chiba-spec");
    let fixtures = collect_chiba_spec_fixtures(&spec_root);

    for path in fixtures {
        let source = fs::read_to_string(&path).unwrap();
        for (line_index, line) in source.lines().enumerate() {
            let Some(raw) = line.trim().strip_prefix("// backend-matrix:") else {
                continue;
            };
            assert!(
                !raw.trim().is_empty(),
                "{}:{} has empty backend matrix",
                path.strip_prefix(repo_root()).unwrap_or(&path).display(),
                line_index + 1
            );
            for target in raw.split(',').map(str::trim) {
                assert!(
                    parse_backend_target(target).is_some(),
                    "{}:{} uses unknown backend target `{target}`",
                    path.strip_prefix(repo_root()).unwrap_or(&path).display(),
                    line_index + 1
                );
            }
        }
    }
}

#[test]
fn chiba_spec_backend_matrix_runner_does_not_spawn_nested_cargo() {
    let source =
        fs::read_to_string(repo_root().join("level-1r/tests/chiba_spec_matrix_baseline.rs"))
            .unwrap();

    let double_quoted = ["Command::new(", "\"cargo\"", ")"].join("");
    let single_quoted = ["Command::new(", "'cargo'", ")"].join("");
    assert!(
        !source.contains(&double_quoted) && !source.contains(&single_quoted),
        "backend matrix tests must use in-process target APIs instead of nested cargo"
    );
}

fn render_def_backend_diagnostics(defs: &[chiba_level1r::ProgramDefOutput]) -> String {
    defs.iter()
        .filter(|def| !def.output.backend.diagnostics.is_empty())
        .map(|def| format!("{}={:?}", def.name, def.output.backend.diagnostics))
        .collect::<Vec<_>>()
        .join(", ")
}

fn strip_chiba_spec_metadata_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn backend_matrix_targets(source: &str) -> Option<Vec<BackendTarget>> {
    source.lines().find_map(|line| {
        let raw = line.trim().strip_prefix("// backend-matrix:")?;
        Some(
            raw.split(',')
                .filter_map(|name| parse_backend_target(name.trim()))
                .collect(),
        )
    })
}

fn parse_backend_target(name: &str) -> Option<BackendTarget> {
    match name {
        "wasm-gc" => Some(BackendTarget::WasmGc),
        "wasm32-nogc" => Some(BackendTarget::Wasm32NoGc),
        "native" => Some(BackendTarget::Native),
        _ => None,
    }
}

fn assert_no_wasm_gc_terms(wat: &str, path: &Path) {
    for term in ["externref", "struct.new", "array.new", "ref.cast", "eqref"] {
        assert!(
            !wat.contains(term),
            "{} wasm32-nogc output contains Wasm-GC term `{term}`",
            path.strip_prefix(repo_root()).unwrap_or(path).display()
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
