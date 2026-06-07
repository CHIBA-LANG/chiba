use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../src/regex/xiddata.chiba");
    let input =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../src/regex/xiddata.chiba");
    let output = PathBuf::from(env::var("OUT_DIR").unwrap()).join("xiddata.rs");
    let source = fs::read_to_string(&input).expect("read shared xid data");
    let generated = generate(&source);
    fs::write(&output, generated).expect("write xiddata.rs");
}

fn generate(source: &str) -> String {
    let mut start = Vec::new();
    let mut continue_ranges = Vec::new();
    let mut mode = None;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("def emit_xid_start_ranges") {
            mode = Some(Target::Start);
            continue;
        }
        if trimmed.starts_with("def emit_xid_continue_ranges") {
            mode = Some(Target::Continue);
            continue;
        }
        if !trimmed.contains("emit_codepoint_range(code,") {
            continue;
        }
        let mut parts = trimmed.split(',');
        let _prefix = parts.next().unwrap();
        let lo = parse_hex(parts.next().unwrap());
        let hi = parse_hex(parts.next().unwrap());
        match mode {
            Some(Target::Start) => start.push((lo, hi)),
            Some(Target::Continue) => continue_ranges.push((lo, hi)),
            None => {}
        }
    }

    let mut out = String::new();
    out.push_str("// Auto-generated from src/regex/xiddata.chiba\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("pub const XID_START_RANGES: &[(u32, u32)] = &[\n");
    for (lo, hi) in &start {
        out.push_str(&format!("    (0x{lo:X}, 0x{hi:X}),\n"));
    }
    out.push_str("];\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("pub const XID_CONTINUE_RANGES: &[(u32, u32)] = &[\n");
    for (lo, hi) in &continue_ranges {
        out.push_str(&format!("    (0x{lo:X}, 0x{hi:X}),\n"));
    }
    out.push_str("];\n\n");
    out.push_str("fn contains(ranges: &[(u32, u32)], ch: char) -> bool {\n");
    out.push_str("    let codepoint = ch as u32;\n");
    out.push_str("    let idx = ranges.partition_point(|&(lo, _)| lo <= codepoint);\n");
    out.push_str("    if idx == 0 { false } else { let (lo, hi) = ranges[idx - 1]; lo <= codepoint && codepoint <= hi }\n");
    out.push_str("}\n\n");
    out.push_str("pub fn is_xid_start(ch: char) -> bool { contains(XID_START_RANGES, ch) }\n");
    out.push_str(
        "pub fn is_xid_continue(ch: char) -> bool { contains(XID_CONTINUE_RANGES, ch) }\n",
    );
    out
}

fn parse_hex(raw: &str) -> u32 {
    let trimmed = raw.trim().trim_end_matches(')').trim();
    let digits = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    u32::from_str_radix(digits, 16).expect("hex range")
}

#[derive(Clone, Copy)]
enum Target {
    Start,
    Continue,
}
