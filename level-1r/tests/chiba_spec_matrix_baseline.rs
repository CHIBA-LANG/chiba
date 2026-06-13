use chiba_level1r::backend::BackendTarget;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

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
        let runtime_oracles = main_runtime_oracles(&source);
        assert!(
            !runtime_oracles.is_empty(),
            "{} backend-matrix fixture must include at least one // expect: runtime main == oracle",
            path.strip_prefix(repo_root()).unwrap_or(&path).display()
        );
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
            for expected in &runtime_oracles {
                let actual = run_wat_export(&output.program.backend_link.linked_wat, "main");
                assert_eq!(
                    actual,
                    *expected,
                    "{} runtime main mismatch for {target:?}",
                    path.strip_prefix(repo_root()).unwrap_or(&path).display()
                );
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

fn main_runtime_oracles(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let raw = line.trim().strip_prefix("// expect: runtime main == ")?;
            Some(raw.trim().to_string())
        })
        .collect()
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

fn run_wat_export(wat: &str, export: &str) -> String {
    static WAT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "level1r-chiba-spec-matrix-{}-{}-{}.wat",
        std::process::id(),
        std::thread::current().name().unwrap_or("test"),
        WAT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, wat).expect("write generated wat fixture");
    wat_runner()
        .lock()
        .expect("wat runner lock")
        .run(&path, export, wat)
}

fn wat_runner() -> &'static Mutex<BatchWatRunner> {
    static RUNNER: OnceLock<Mutex<BatchWatRunner>> = OnceLock::new();
    RUNNER.get_or_init(|| Mutex::new(BatchWatRunner::spawn()))
}

struct BatchWatRunner {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl BatchWatRunner {
    fn spawn() -> Self {
        let mut child = Command::new("node")
            .arg("tools/node/run-wat-batch.mjs")
            .current_dir(repo_root())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn batch wat runner");
        let stdin = child.stdin.take().expect("batch wat runner stdin");
        let stdout = BufReader::new(child.stdout.take().expect("batch wat runner stdout"));
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn run(&mut self, path: &Path, export: &str, wat: &str) -> String {
        if let Some(status) = self.child.try_wait().expect("poll batch wat runner") {
            panic!("batch WAT runner exited before request: {status}");
        }
        writeln!(
            self.stdin,
            "RUN\t{}\t{}",
            path.to_string_lossy(),
            export.replace('\t', "")
        )
        .expect("send batch wat request");
        self.stdin.flush().expect("flush batch wat request");

        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .expect("read batch wat response");
        let line = line.trim_end_matches(['\r', '\n']);
        let Some((status, payload)) = line.split_once('\t') else {
            panic!("malformed batch WAT response: {line}");
        };
        let decoded = decode_base64(payload).expect("decode batch WAT response");
        match status {
            "OK" => decoded,
            "ERR" => panic!("generated WAT failed\nstderr:\n{}\nwat:\n{}", decoded, wat),
            _ => panic!("unknown batch WAT response status: {status}"),
        }
    }
}

fn decode_base64(input: &str) -> Result<String, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4];
    let mut chunk_len = 0;
    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => 64,
            _ => return Err(format!("invalid base64 byte {byte}")),
        };
        chunk[chunk_len] = value;
        chunk_len += 1;
        if chunk_len == 4 {
            if chunk[0] == 64 || chunk[1] == 64 {
                return Err("invalid base64 padding".to_string());
            }
            bytes.push((chunk[0] << 2) | (chunk[1] >> 4));
            if chunk[2] != 64 {
                bytes.push((chunk[1] << 4) | (chunk[2] >> 2));
            }
            if chunk[3] != 64 {
                bytes.push((chunk[2] << 6) | chunk[3]);
            }
            chunk_len = 0;
        }
    }
    if chunk_len != 0 {
        return Err("truncated base64 payload".to_string());
    }
    String::from_utf8(bytes).map_err(|error| error.to_string())
}
