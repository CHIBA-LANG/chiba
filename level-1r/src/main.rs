use std::{env, fs, process};

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut visual = false;
    let mut input = None;
    for arg in args {
        match arg.as_str() {
            "--visual" => visual = true,
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
    let output = chiba_level1r::compile_source_program_bundle(&source)
        .map_err(|error| {
            format!(
                "{input}: {}",
                chiba_level1r::render_frontend_error(&source, &error)
            )
        })?;

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

fn print_usage() {
    println!("usage: chiba-level1r [--visual] <source.chiba>");
}
