use std::fs;
use std::process::Command;

fn write_fixture(name: &str, source: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "chiba-level1r-{}-{name}.chiba",
        std::process::id()
    ));
    fs::write(&path, source).expect("write fixture");
    path
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

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("source-program:"));
    assert!(stdout.contains("namespace=cli.demo"));
    assert!(stdout.contains("program:"));
    assert!(stdout.contains("entry=Some(\"main\")"));
    assert!(stdout.contains("P1ProjectSurface: SourceProgram -> ProjectSurface"));
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

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("== def main =="));
    assert!(stdout.contains("source:"));
    assert!(stdout.contains("typed:"));
    assert!(stdout.contains("cps:"));
    assert!(stdout.contains("backend:"));
    assert!(stdout.contains("nanopass:"));
    assert!(stdout.contains("L11OnePassCps: TypedExpr -> CpsProgram"));
}
