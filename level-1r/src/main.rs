use std::{env, fs, process};

use chiba_level1r::backend::BackendTarget;

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut visual = false;
    let mut target = BackendTarget::WasmGc;
    let mut input = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--visual" => visual = true,
            "--target" => {
                let Some(value) = args.next() else {
                    return Err("--target requires a backend target".to_string());
                };
                target = parse_backend_target(&value)?;
            }
            _ if arg.starts_with("--target=") => {
                let value = arg.trim_start_matches("--target=");
                target = parse_backend_target(value)?;
            }
            "-h" | "--help" => {
                print_usage();
                return Ok(());
            }
            _ if input.is_none() => input = Some(arg),
            _ => return Err(format!("unexpected argument: {arg}")),
        }
    }

    let Some(input) = input else {
        print_usage();
        return Ok(());
    };
    let source = fs::read_to_string(&input).map_err(|error| format!("{input}: {error}"))?;
    let output = chiba_level1r::compile_source_program_bundle_for_target(&source, target).map_err(
        |error| {
            format!(
                "{input}: {}",
                chiba_level1r::render_frontend_error(&source, &error)
            )
        },
    )?;

    if visual {
        for def in &output.program.defs {
            println!("== def {} ==", def.name);
            print!("{}", def.output.render_visual());
        }
    } else {
        print!("{}", output.render_summary());
    }
    Ok(())
}

fn parse_backend_target(value: &str) -> Result<BackendTarget, String> {
    match value {
        "wasm-gc" => Ok(BackendTarget::WasmGc),
        "wasm32-nogc" => Ok(BackendTarget::Wasm32NoGc),
        "native" => Ok(BackendTarget::Native),
        _ => Err(format!(
            "unknown backend target `{value}`; expected wasm-gc, wasm32-nogc, or native"
        )),
    }
}

fn print_usage() {
    println!(
        "usage: chiba-level1r [--visual] [--target <wasm-gc|wasm32-nogc|native>] <source.chiba>"
    );
}
