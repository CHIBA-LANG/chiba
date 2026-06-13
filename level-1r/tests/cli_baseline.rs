use std::fs;
use std::process::{Command, Output};

fn write_fixture(name: &str, source: &str) -> std::path::PathBuf {
    let path =
        std::env::temp_dir().join(format!("chiba-level1r-{}-{name}.chiba", std::process::id()));
    fs::write(&path, source).expect("write fixture");
    path
}

fn assert_cli_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        render_cli_output_context(output)
    );
}

fn assert_cli_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "{}",
        render_cli_output_context(output)
    );
}

fn render_cli_output_context(output: &Output) -> String {
    format!(
        "status={}\nstdout:\n{}\nstderr:\n{}",
        output
            .status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "signal".to_string()),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn cli_compiles_source_file_to_program_summary() {
    let source = write_fixture(
        "summary",
        "namespace cli.demo
def main() = 7",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_chiba-level1r"))
        .arg(&source)
        .output()
        .expect("run cli");

    assert_cli_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("source-program:"));
    assert!(stdout.contains("namespace=cli.demo"));
    assert!(stdout.contains("item-spans:"));
    assert!(stdout.contains("def main @ 2:1..2:15"));
    assert!(stdout.contains("program:"));
    assert!(stdout.contains("entry=main"));
    assert!(!stdout.contains("entry=Some"));
    assert!(stdout.contains("P1ProjectSurface: SourceProgram -> ProjectSurface"));
}

#[test]
fn cli_can_compile_source_for_wasm32_nogc_target() {
    let source = write_fixture(
        "wasm32-nogc-target",
        "namespace cli.target
def main(): i64 = 40 + 2",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_chiba-level1r"))
        .arg("--target")
        .arg("wasm32-nogc")
        .arg(&source)
        .output()
        .expect("run cli");

    assert_cli_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("namespace=cli.target"));
    assert!(stdout.contains("backend-link:"));
    assert!(stdout.contains("target=wasm32-nogc"));
    assert!(stdout.contains("backend-cache-key:"));
    assert!(!stdout.contains("externref"));
    assert!(!stdout.contains("struct.new"));
}

#[test]
fn cli_rejects_unknown_backend_target() {
    let source = write_fixture("bad-target", "def main(): i64 = 1");

    let output = Command::new(env!("CARGO_BIN_EXE_chiba-level1r"))
        .arg("--target=wat")
        .arg(&source)
        .output()
        .expect("run cli");

    assert_cli_failure(&output);
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("unknown backend target `wat`; expected wasm-gc, wasm32-nogc, or native"),
        "{stderr}"
    );
}

#[test]
fn cli_visual_flag_prints_per_def_nanopass_report() {
    let source = write_fixture(
        "visual",
        "def main() = match tag {
0 => 1
_ => 2
}",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_chiba-level1r"))
        .arg("--visual")
        .arg(&source)
        .output()
        .expect("run cli");

    assert_cli_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("== def main =="));
    assert!(stdout.contains("source:"));
    assert!(stdout.contains("source def main @ 1:1..4:2"));
    assert!(stdout.contains("typed:"));
    assert!(stdout.contains("cps:"));
    assert!(stdout.contains("backend:"));
    assert!(stdout.contains("nanopass:"));
    assert!(stdout.contains("L11OnePassCps: TypedExpr -> CpsProgram"));
}

#[test]
fn cli_visual_prints_callable_storage_for_continuation_type_fields() {
    let source = write_fixture(
        "visual-continuation-storage",
        "type ParserState = { retry: contN (i64) -> bool }
def main() = 0",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_chiba-level1r"))
        .arg("--visual")
        .arg(&source)
        .output()
        .expect("run cli");

    assert_cli_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("callable-storage:"));
    assert!(stdout.contains("type::ParserState::retry"));
    assert!(stdout.contains("kind=contn-package"));
    assert!(!stdout.contains("ContNPackage"));
    assert!(!stdout.contains("CallableStorageFact"));
}

#[test]
fn cli_summary_reports_control_diagnostics() {
    let source = write_fixture("control-diagnostic", "def main() = shift k { k(1) }");

    let output = Command::new(env!("CARGO_BIN_EXE_chiba-level1r"))
        .arg(&source)
        .output()
        .expect("run cli");

    assert_cli_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("program:"));
    assert!(stdout.contains("diagnostics=1"));
    assert!(stdout.contains("shift outside reset main: k"), "{stdout}");
}

#[test]
fn cli_summary_reports_cont1_multi_resume_diagnostic() {
    let source = write_fixture(
        "cont1-multi-resume-diagnostic",
        "def main() = reset { shift k { k(1) + k(2) } }",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_chiba-level1r"))
        .arg(&source)
        .output()
        .expect("run cli");

    assert_cli_success(&output);
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("program:"));
    assert!(stdout.contains("diagnostics=1"));
    assert!(
        stdout.contains("cont1 resumed more than once main: k"),
        "{stdout}"
    );
}

#[test]
fn cli_reports_frontend_errors_with_source_location() {
    let source = write_fixture("error", "def main() = match tag { 0 => 1 _ => 2 }");

    let output = Command::new(env!("CARGO_BIN_EXE_chiba-level1r"))
        .arg(&source)
        .output()
        .expect("run cli");

    assert_cli_failure(&output);
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("frontend error at 1:33"), "{stderr}");
    assert!(
        stderr.contains("unexpected token Ident `_`, expected Comma"),
        "{stderr}"
    );
    assert!(
        stderr.contains("def main() = match tag { 0 => 1 _ => 2 }"),
        "{stderr}"
    );
    assert!(
        stderr.contains("                                ^"),
        "{stderr}"
    );
}
