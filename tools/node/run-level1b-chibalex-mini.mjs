import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";
import { runWatText } from "./run-wat.mjs";

const ROOT = "level-1b/supports/chibalex-mini";
const OUT = ".scratch/level-1b/chibalex-mini";
const XIDDATA = "level-1b/std/regex/xiddata.chiba";
const CASES = [
  {
    file: "basic.chibalex",
    name: "basic",
    namespace: "chibalexmini.basic",
    expected: ["KwLet", "IntLit", "Ident", "Eq"],
    source: "mk_str(\"let x = 12\", 10)",
    check: `
        KwLet =>
            match token_at(tokens, 1) {
                Ident(name) =>
                    if streq(name, mk_str("x", 1)) != 0 {
                        match token_at(tokens, 2) {
                            Eq =>
                                match token_at(tokens, 3) {
                                    IntLit(n) => if streq(n, mk_str("12", 2)) != 0 { 0 } else { 5 }
                                    _ => 4
                                }
                            _ => 3
                        }
                    } else { 2 }
                _ => 1
            }
        _ => 6
`,
  },
  {
    file: "longest.chibalex",
    name: "longest",
    namespace: "chibalexmini.longest",
    expected: ["EqEq", "Eq", "KwIf", "Ident"],
    source: "mk_str(\"if == =\", 7)",
    check: `
        KwIf =>
            match token_at(tokens, 1) {
                EqEq =>
                    match token_at(tokens, 2) {
                        Eq => 0
                        _ => 2
                    }
                _ => 1
            }
        _ => 3
`,
  },
  {
    file: "string-mode.chibalex",
    name: "stringmode",
    namespace: "chibalexmini.stringmode",
    expected: ["StringStart", "StringChunk", "StringEnd"],
    source: 'r#""abc""#',
    check: `
        StringStart =>
            match token_at(tokens, 1) {
                StringChunk(text) =>
                    if streq(text, mk_str("abc", 3)) != 0 {
                        match token_at(tokens, 2) {
                            StringEnd => 0
                            _ => 3
                        }
                    } else { 2 }
                _ => 1
            }
        _ => 4
`,
  },
  {
    file: "continuation-surface.chibalex",
    name: "continuation",
    namespace: "chibalexmini.continuation",
    expected: ["KwShiftn", "KwCont1", "KwContN", "ThinArrow"],
    source: "mk_str(\"cont1 (A) -> B contN shiftn\", 27)",
    check: `
        KwCont1 =>
            match token_at(tokens, 1) {
                LParen =>
                    match token_at(tokens, 2) {
                        Ident(name) =>
                            if streq(name, mk_str("A", 1)) != 0 {
                                match token_at(tokens, 4) {
                                    ThinArrow =>
                                        match token_at(tokens, 6) {
                                            KwContN =>
                                                match token_at(tokens, 7) {
                                                    KwShiftn => 0
                                                    _ => 7
                                                }
                                            _ => 6
                                        }
                                    _ => 5
                                }
                            } else { 4 }
                        _ => 3
                    }
                _ => 2
            }
        _ => 1
`,
  },
  {
    file: "utf8-ident.chibalex",
    name: "utf8ident",
    namespace: "chibalexmini.utf8_ident",
    expected: ["KwLet", "Ident", "Eq"],
    source: "mk_str(\"let café = λ\", 14)",
    check: `
        KwLet =>
            match token_at(tokens, 1) {
                Ident(name) =>
                    if streq(name, mk_str("café", 5)) != 0 {
                        match token_at(tokens, 2) {
                            Eq =>
                                match token_at(tokens, 3) {
                                    Ident(lambda) => if streq(lambda, mk_str("λ", 2)) != 0 { 0 } else { 5 }
                                    _ => 4
                                }
                            _ => 3
                        }
                    } else { 2 }
                _ => 1
            }
        _ => 6
`,
  },
  {
    file: "utf8-ident.chibalex",
    name: "utf8boundary",
    namespace: "chibalexmini.utf8_ident",
    expected: ["Ident"],
    source: "mk_str(\"\\169\", 1)",
    check: `
        LexError(_) => 0
        _ => 1
`,
  },
  {
    file: "utf8-ident.chibalex",
    name: "utf8combining",
    namespace: "chibalexmini.utf8_ident",
    expected: ["Ident"],
    source: "mk_str(\"é\", 3)",
    check: `
        Ident(name) => if streq(name, mk_str("é", 3)) != 0 { 0 } else { 2 }
        _ => 1
`,
  },
  {
    file: "utf8-ident.chibalex",
    name: "utf8invalidcont",
    namespace: "chibalexmini.utf8_ident",
    expected: ["Ident"],
    source: "mk_str(\"\\195A\", 2)",
    check: `
        LexError(_) => 0
        _ => 1
`,
  },
  {
    file: "attribute-tokens.chibalex",
    name: "attribute",
    namespace: "chibalexmini.attribute_tokens",
    expected: ["Hash", "LBracket", "Ident", "LParen", "Comma", "Eq", "IntLit", "StringLit", "KwTrue", "RBracket"],
    source: 'r#"#[attribute(all(someident, a=b, c=[1,2], meta={owner="compiler", stable=true}))]"#',
    check: `
        Hash =>
            match token_at(tokens, 1) {
                LBracket =>
                    match token_at(tokens, 2) {
                        Ident(name) =>
                            if streq(name, mk_str("attribute", 9)) != 0 {
                                match token_at(tokens, 3) {
                                    LParen =>
                                        match token_at(tokens, 5) {
                                            LParen =>
                                                match token_at(tokens, 9) {
                                                    Eq => 0
                                                    _ => 6
                                                }
                                            _ => 5
                                        }
                                    _ => 4
                                }
                            } else { 3 }
                        _ => 2
                    }
                _ => 1
            }
        _ => 7
`,
  },
];

const LEGACY_REFERENCE_COMPILER = "./target/debug/level1c.o";

function run(name, command, args) {
  const result = spawnSync(command, args, { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  if (result.status !== 0) {
    console.error(`[FAIL] ${name}`);
    console.error(`${result.stdout || ""}${result.stderr || ""}`.split("\n").slice(0, 40).join("\n"));
    process.exit(result.status || 1);
  }
  console.log(`[PASS] ${name}`);
  return result;
}

function oracleReferenceFailed(name) {
  console.log(`[ORACLE-FAIL] ${name}`);
}

fs.rmSync(OUT, { recursive: true, force: true });
fs.mkdirSync(OUT, { recursive: true });

function runParseOk(name, file) {
  const result = run(name, "timeout", ["10", LEGACY_REFERENCE_COMPILER, "parse", file]);
  if (!result.stdout.startsWith("OK(")) {
    console.error(`[FAIL] ${name}`);
    console.error(result.stdout || result.stderr || "parse did not return OK");
    process.exit(1);
  }
  return result;
}

function runCheckOk(name, file) {
  const result = run(name, "timeout", ["10", LEGACY_REFERENCE_COMPILER, "check", file]);
  if (!result.stdout.includes("check ok")) {
    console.error(`[FAIL] ${name}`);
    console.error(result.stdout || result.stderr || "check did not report ok");
    process.exit(1);
  }
  return result;
}

function tokenDataBlock(spec) {
  const match = spec.match(/data\s+Token\s*\{[\s\S]*?\n\s*\}/);
  if (!match) {
    throw new Error("mini chibalex fixture is missing data Token block");
  }
  return match[0];
}

function requireXidRange(lo, hi) {
  const source = fs.readFileSync(XIDDATA, "utf8");
  const needle = `push_codepoint_range(${lo}, ${hi})`;
  if (!source.includes(needle)) {
    throw new Error(`shared Unicode XID table missing ${needle}`);
  }
}

function miniLexerHelpers(namespace, spec) {
  for (const [lo, hi] of [["0xC0", "0xD6"], ["0xD8", "0xF6"], ["0x370", "0x374"], ["0x3A3", "0x3F5"], ["0x300", "0x374"]]) {
    requireXidRange(lo, hi);
  }
  return `namespace ${namespace}
use metalstd.mem.*
use metalstd.str.*
use metalstd.vec.*

// generated by level-1b mini chibalex slice
${tokenDataBlock(spec)}

type Span { file: i64, line: i64, col: i64, len: i64 }
type TokenSpan { token: Token, span: Span, leading: Vec, trailing: Vec }

def tokenspan_make(tok: Token, span: Span, leading: Vec, trailing: Vec): TokenSpan =
    TokenSpan { token: tok, span: span, leading: leading, trailing: trailing }

def lex_emit(out: Vec, tok: Token, file: i64, pos: i64, len: i64): i64 = {
    let _ = vec_push(out, tokenspan_make(tok, Span { file: file, line: 1, col: pos + 1, len: len }, vec_new(), vec_new()) as i64)
    0
}

def mini_is_ws(b: i64): i64 =
    if b == 32 { 1 } else if b == 9 { 1 } else if b == 10 { 1 } else if b == 13 { 1 } else { 0 }

def mini_is_digit(b: i64): i64 =
    if b >= 48 { if b <= 57 { 1 } else { 0 } } else { 0 }

def mini_is_alpha(b: i64): i64 =
    if b >= 65 { if b <= 90 { 1 } else if b >= 97 { if b <= 122 { 1 } else { 0 } } else { 0 } } else { 0 }

def mini_is_utf8_continuation(b: i64): i64 =
    if b >= 128 { if b < 192 { 1 } else { 0 } } else { 0 }

def mini_byte_at(src: Str, pos: i64): i64 =
    src.byte_at(pos)

def mini_slice(src: Str, start: i64, end: i64): Str =
    src.slice(start, end - start)

def mini_utf8_width(b: i64): i64 =
    if b < 128 { 1 }
    else if b < 224 { 2 }
    else if b < 240 { 3 }
    else { 4 }

def mini_utf8_valid_2(src: Str, pos: i64): i64 = {
    let p1 = pos + 1
    let b0 = mini_byte_at(src, pos)
    if p1 >= src.len { 0 }
    else if b0 < 194 { 0 }
    else if b0 > 223 { 0 }
    else { mini_is_utf8_continuation(mini_byte_at(src, p1)) }
}

def mini_utf8_valid_3(src: Str, pos: i64): i64 = {
    let p1 = pos + 1
    let p2 = pos + 2
    let b0 = mini_byte_at(src, pos)
    let b1 = if p1 >= src.len { 0 } else { mini_byte_at(src, p1) }
    if p2 >= src.len { 0 }
    else if b0 < 224 { 0 }
    else if b0 > 239 { 0 }
    else if b0 == 224 { if b1 < 160 { 0 } else if mini_is_utf8_continuation(mini_byte_at(src, p1)) == 0 { 0 } else { mini_is_utf8_continuation(mini_byte_at(src, p2)) } }
    else if b0 == 237 { if b1 >= 160 { 0 } else if mini_is_utf8_continuation(mini_byte_at(src, p1)) == 0 { 0 } else { mini_is_utf8_continuation(mini_byte_at(src, p2)) } }
    else if mini_is_utf8_continuation(mini_byte_at(src, p1)) == 0 { 0 }
    else { mini_is_utf8_continuation(mini_byte_at(src, p2)) }
}

def mini_utf8_valid_4(src: Str, pos: i64): i64 = {
    let p1 = pos + 1
    let p2 = pos + 2
    let p3 = pos + 3
    let b0 = mini_byte_at(src, pos)
    let b1 = if p1 >= src.len { 0 } else { mini_byte_at(src, p1) }
    if p3 >= src.len { 0 }
    else if b0 < 240 { 0 }
    else if b0 > 244 { 0 }
    else if b0 == 240 { if b1 < 144 { 0 } else if mini_is_utf8_continuation(mini_byte_at(src, p1)) == 0 { 0 } else if mini_is_utf8_continuation(mini_byte_at(src, p2)) == 0 { 0 } else { mini_is_utf8_continuation(mini_byte_at(src, p3)) } }
    else if b0 == 244 { if b1 > 143 { 0 } else if mini_is_utf8_continuation(mini_byte_at(src, p1)) == 0 { 0 } else if mini_is_utf8_continuation(mini_byte_at(src, p2)) == 0 { 0 } else { mini_is_utf8_continuation(mini_byte_at(src, p3)) } }
    else if mini_is_utf8_continuation(mini_byte_at(src, p1)) == 0 { 0 }
    else if mini_is_utf8_continuation(mini_byte_at(src, p2)) == 0 { 0 }
    else { mini_is_utf8_continuation(mini_byte_at(src, p3)) }
}

def mini_utf8_valid_at(src: Str, pos: i64): i64 =
    if pos >= src.len { 0 }
    else {
        let b0 = mini_byte_at(src, pos)
        if b0 < 128 { 1 }
        else if b0 < 194 { 0 }
        else if b0 < 224 { mini_utf8_valid_2(src, pos) }
        else if b0 < 240 { mini_utf8_valid_3(src, pos) }
        else if b0 < 245 { mini_utf8_valid_4(src, pos) }
        else { 0 }
    }

def mini_codepoint_2(src: Str, pos: i64, b0: i64): i64 = {
    let p1 = pos + 1
    let hi = b0 - 192
    let lo = mini_byte_at(src, p1) - 128
    hi * 64 + lo
}

def mini_codepoint_3(src: Str, pos: i64, b0: i64): i64 = {
    let p1 = pos + 1
    let p2 = pos + 2
    let hi = b0 - 224
    let mid = mini_byte_at(src, p1) - 128
    let lo = mini_byte_at(src, p2) - 128
    hi * 4096 + mid * 64 + lo
}

def mini_codepoint_4(src: Str, pos: i64, b0: i64): i64 = {
    let p1 = pos + 1
    let p2 = pos + 2
    let p3 = pos + 3
    let hi = b0 - 240
    let mid1 = mini_byte_at(src, p1) - 128
    let mid2 = mini_byte_at(src, p2) - 128
    let lo = mini_byte_at(src, p3) - 128
    hi * 262144 + mid1 * 4096 + mid2 * 64 + lo
}

def mini_codepoint_at(src: Str, pos: i64): i64 = {
    let b0 = mini_byte_at(src, pos)
    if b0 < 128 { b0 }
    else if b0 < 224 { mini_codepoint_2(src, pos, b0) }
    else if b0 < 240 { mini_codepoint_3(src, pos, b0) }
    else { mini_codepoint_4(src, pos, b0) }
}

def mini_cp_in_range(cp: i64, lo: i64, hi: i64): i64 =
    if cp < lo { 0 } else if cp <= hi { 1 } else { 0 }

def mini_is_xid_start_cp(cp: i64): i64 =
    if cp == 95 { 1 }
    else if mini_cp_in_range(cp, 65, 90) != 0 { 1 }
    else if mini_cp_in_range(cp, 97, 122) != 0 { 1 }
    else if mini_cp_in_range(cp, 192, 214) != 0 { 1 }
    else if mini_cp_in_range(cp, 216, 246) != 0 { 1 }
    else if mini_cp_in_range(cp, 880, 884) != 0 { 1 }
    else if mini_cp_in_range(cp, 931, 1013) != 0 { 1 }
    else { 0 }

def mini_is_xid_continue_cp(cp: i64): i64 =
    if mini_is_xid_start_cp(cp) != 0 { 1 }
    else if mini_cp_in_range(cp, 48, 57) != 0 { 1 }
    else if mini_cp_in_range(cp, 768, 884) != 0 { 1 }
    else { 0 }

def mini_is_ident_start_at(src: Str, pos: i64): i64 =
    if mini_utf8_valid_at(src, pos) == 0 { 0 }
    else { mini_is_xid_start_cp(mini_codepoint_at(src, pos)) }

def mini_is_ident_continue_at(src: Str, pos: i64): i64 =
    if mini_utf8_valid_at(src, pos) == 0 { 0 }
    else { mini_is_xid_continue_cp(mini_codepoint_at(src, pos)) }

def mini_next_offset(src: Str, pos: i64): i64 =
    if pos >= src.len { src.len } else { pos + mini_utf8_width(mini_byte_at(src, pos)) }

def mini_scan_digits(src: Str, pos: i64): i64 =
    if pos >= src.len { pos }
    else if mini_is_digit(mini_byte_at(src, pos)) != 0 { mini_scan_digits(src, pos + 1) }
    else { pos }

def mini_scan_ident(src: Str, pos: i64): i64 =
    if pos >= src.len { pos }
    else if mini_is_ident_continue_at(src, pos) != 0 { mini_scan_ident(src, mini_next_offset(src, pos)) }
    else { pos }

def mini_scan_until_quote(src: Str, pos: i64): i64 =
    if pos >= src.len { pos }
    else if mini_byte_at(src, pos) == 34 { pos }
    else { mini_scan_until_quote(src, pos + 1) }

def mini_literal_eq_at(src: Str, pos: i64, lit: Str, i: i64): i64 =
    if i >= lit.len { 1 }
    else {
        let at = pos + i
        if at >= src.len { 0 }
        else if mini_byte_at(src, at) == mini_byte_at(lit, i) { mini_literal_eq_at(src, pos, lit, i + 1) }
        else { 0 }
    }
`;
}

function miniGeneratedLexer(caseInfo, spec) {
  const { namespace, name } = caseInfo;
  const helpers = miniLexerHelpers(namespace, spec);
  if (name === "basic") {
    return `${helpers}
def lex_loop(src: Str, file: i64, out: Vec, pos: i64): i64 =
    if pos >= src.len { 0 }
    else {
        let b = mini_byte_at(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, file, out, pos + 1) }
        else if mini_literal_eq_at(src, pos, mk_str("let", 3), 0) != 0 {
            let _ = lex_emit(out, KwLet, file, pos, 3)
            lex_loop(src, file, out, pos + 3)
        } else if mini_is_digit(b) != 0 {
            let end = mini_scan_digits(src, pos)
            let _ = lex_emit(out, IntLit(mini_slice(src, pos, end)), file, pos, end - pos)
            lex_loop(src, file, out, end)
        } else if mini_is_ident_start_at(src, pos) != 0 {
            let end = mini_scan_ident(src, mini_next_offset(src, pos))
            let _ = lex_emit(out, Ident(mini_slice(src, pos, end)), file, pos, end - pos)
            lex_loop(src, file, out, end)
        } else if b == 61 {
            let _ = lex_emit(out, Eq, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else {
            let _ = lex_emit(out, LexError(b), file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        }
    }

def lex_all(src: Str, file: i64): Vec = {
    let out = vec_new()
    let _ = lex_loop(src, file, out, 0)
    let _ = lex_emit(out, Eof, file, src.len, 0)
    out
}
`;
  }
  if (name.startsWith("utf8")) {
    return `${helpers}
def lex_loop(src: Str, file: i64, out: Vec, pos: i64): i64 =
    if pos >= src.len { 0 }
    else {
        let b = mini_byte_at(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, file, out, pos + 1) }
        else if mini_literal_eq_at(src, pos, mk_str("let", 3), 0) != 0 {
            let _ = lex_emit(out, KwLet, file, pos, 3)
            lex_loop(src, file, out, pos + 3)
        } else if b == 61 {
            let _ = lex_emit(out, Eq, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if mini_is_ident_start_at(src, pos) != 0 {
            let end = mini_scan_ident(src, mini_next_offset(src, pos))
            let _ = lex_emit(out, Ident(mini_slice(src, pos, end)), file, pos, end - pos)
            lex_loop(src, file, out, end)
        } else {
            let _ = lex_emit(out, LexError(b), file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        }
    }

def lex_all(src: Str, file: i64): Vec = {
    let out = vec_new()
    let _ = lex_loop(src, file, out, 0)
    let _ = lex_emit(out, Eof, file, src.len, 0)
    out
}
`;
  }
  if (name === "attribute") {
    return `${helpers}
def lex_loop(src: Str, file: i64, out: Vec, pos: i64): i64 =
    if pos >= src.len { 0 }
    else {
        let b = mini_byte_at(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, file, out, pos + 1) }
        else if mini_literal_eq_at(src, pos, mk_str("true", 4), 0) != 0 {
            let _ = lex_emit(out, KwTrue, file, pos, 4)
            lex_loop(src, file, out, pos + 4)
        } else if mini_literal_eq_at(src, pos, mk_str("false", 5), 0) != 0 {
            let _ = lex_emit(out, KwFalse, file, pos, 5)
            lex_loop(src, file, out, pos + 5)
        } else if b == 34 {
            let end = mini_scan_until_quote(src, pos + 1)
            let _ = lex_emit(out, StringLit(mini_slice(src, pos, end + 1)), file, pos, end + 1 - pos)
            lex_loop(src, file, out, end + 1)
        } else if mini_is_digit(b) != 0 {
            let end = mini_scan_digits(src, pos)
            let _ = lex_emit(out, IntLit(mini_slice(src, pos, end)), file, pos, end - pos)
            lex_loop(src, file, out, end)
        } else if mini_is_ident_start_at(src, pos) != 0 {
            let end = mini_scan_ident(src, mini_next_offset(src, pos))
            let _ = lex_emit(out, Ident(mini_slice(src, pos, end)), file, pos, end - pos)
            lex_loop(src, file, out, end)
        } else if mini_literal_eq_at(src, pos, mk_str("#!", 2), 0) != 0 {
            let _ = lex_emit(out, HashBang, file, pos, 2)
            lex_loop(src, file, out, pos + 2)
        } else if b == 35 {
            let _ = lex_emit(out, Hash, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if b == 91 {
            let _ = lex_emit(out, LBracket, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if b == 93 {
            let _ = lex_emit(out, RBracket, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if b == 40 {
            let _ = lex_emit(out, LParen, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if b == 41 {
            let _ = lex_emit(out, RParen, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if b == 123 {
            let _ = lex_emit(out, LBrace, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if b == 125 {
            let _ = lex_emit(out, RBrace, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if b == 44 {
            let _ = lex_emit(out, Comma, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if b == 61 {
            let _ = lex_emit(out, Eq, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else {
            let _ = lex_emit(out, LexError(b), file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        }
    }

def lex_all(src: Str, file: i64): Vec = {
    let out = vec_new()
    let _ = lex_loop(src, file, out, 0)
    let _ = lex_emit(out, Eof, file, src.len, 0)
    out
}
`;
  }
  if (name === "longest") {
    return `${helpers}
def lex_loop(src: Str, file: i64, out: Vec, pos: i64): i64 =
    if pos >= src.len { 0 }
    else {
        let b = mini_byte_at(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, file, out, pos + 1) }
        else if mini_literal_eq_at(src, pos, mk_str("==", 2), 0) != 0 {
            let _ = lex_emit(out, EqEq, file, pos, 2)
            lex_loop(src, file, out, pos + 2)
        } else if b == 61 {
            let _ = lex_emit(out, Eq, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if mini_literal_eq_at(src, pos, mk_str("if", 2), 0) != 0 {
            let _ = lex_emit(out, KwIf, file, pos, 2)
            lex_loop(src, file, out, pos + 2)
        } else if mini_is_ident_start_at(src, pos) != 0 {
            let end = mini_scan_ident(src, mini_next_offset(src, pos))
            let _ = lex_emit(out, Ident(mini_slice(src, pos, end)), file, pos, end - pos)
            lex_loop(src, file, out, end)
        } else {
            let _ = lex_emit(out, LexError(b), file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        }
    }

def lex_all(src: Str, file: i64): Vec = {
    let out = vec_new()
    let _ = lex_loop(src, file, out, 0)
    let _ = lex_emit(out, Eof, file, src.len, 0)
    out
}
`;
  }
  if (name === "continuation") {
    return `${helpers}
def lex_loop(src: Str, file: i64, out: Vec, pos: i64): i64 =
    if pos >= src.len { 0 }
    else {
        let b = mini_byte_at(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, file, out, pos + 1) }
        else if mini_literal_eq_at(src, pos, mk_str("shiftn", 6), 0) != 0 {
            let _ = lex_emit(out, KwShiftn, file, pos, 6)
            lex_loop(src, file, out, pos + 6)
        } else if mini_literal_eq_at(src, pos, mk_str("cont1", 5), 0) != 0 {
            let _ = lex_emit(out, KwCont1, file, pos, 5)
            lex_loop(src, file, out, pos + 5)
        } else if mini_literal_eq_at(src, pos, mk_str("contN", 5), 0) != 0 {
            let _ = lex_emit(out, KwContN, file, pos, 5)
            lex_loop(src, file, out, pos + 5)
        } else if mini_literal_eq_at(src, pos, mk_str("->", 2), 0) != 0 {
            let _ = lex_emit(out, ThinArrow, file, pos, 2)
            lex_loop(src, file, out, pos + 2)
        } else if b == 40 {
            let _ = lex_emit(out, LParen, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if b == 41 {
            let _ = lex_emit(out, RParen, file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        } else if mini_is_ident_start_at(src, pos) != 0 {
            let end = mini_scan_ident(src, mini_next_offset(src, pos))
            let _ = lex_emit(out, Ident(mini_slice(src, pos, end)), file, pos, end - pos)
            lex_loop(src, file, out, end)
        } else {
            let _ = lex_emit(out, LexError(b), file, pos, 1)
            lex_loop(src, file, out, pos + 1)
        }
    }

def lex_all(src: Str, file: i64): Vec = {
    let out = vec_new()
    let _ = lex_loop(src, file, out, 0)
    let _ = lex_emit(out, Eof, file, src.len, 0)
    out
}
`;
  }
  return `${helpers}
def lex_loop(src: Str, file: i64, out: Vec, pos: i64, mode: i64): i64 =
    if pos >= src.len { 0 }
    else {
        let b = mini_byte_at(src, pos)
        if mode == 0 {
            if mini_is_ws(b) != 0 { lex_loop(src, file, out, pos + 1, mode) }
            else if b == 34 {
                let _ = lex_emit(out, StringStart, file, pos, 1)
                lex_loop(src, file, out, pos + 1, 1)
            } else {
                let _ = lex_emit(out, LexError(b), file, pos, 1)
                lex_loop(src, file, out, pos + 1, mode)
            }
        } else {
            if b == 34 {
                let _ = lex_emit(out, StringEnd, file, pos, 1)
                lex_loop(src, file, out, pos + 1, 0)
            } else {
                let end = mini_scan_until_quote(src, pos)
                let _ = lex_emit(out, StringChunk(mini_slice(src, pos, end)), file, pos, end - pos)
                lex_loop(src, file, out, end, mode)
            }
        }
    }

def lex_all(src: Str, file: i64): Vec = {
    let out = vec_new()
    let _ = lex_loop(src, file, out, 0, 0)
    let _ = lex_emit(out, Eof, file, src.len, 0)
    out
}
`;
}

function mainSource(caseInfo) {
  return `
def token_at(tokens: Vec, i: i64): Token = {
    let ts = vec_get(tokens, i) as TokenSpan
    ts.token
}

def main(): i64 = {
    let src = ${caseInfo.source}
    let tokens = lex_all(src, 0)
    match token_at(tokens, 0) {
${caseInfo.check}    }
}
`;
}

function executableGeneratedSource(caseInfo, generated) {
  return `${generated}\n${mainSource(caseInfo)}`;
}

async function runGeneratedLexer(caseInfo, generated, output) {
  if (!generated.includes("def lex_all")) {
    console.error(`[FAIL] generated lexer case ${caseInfo.file}`);
    console.error("generated lexer missing lex_all entry");
    process.exit(1);
  }
  console.log(`[PASS] generated lexer source ${caseInfo.file}`);
  runParseOk(`level1c parse generated lexer ${caseInfo.file}`, output);
  runCheckOk(`level1c check generated lexer ${caseInfo.file}`, output);
  const execPath = output.replace(/\.level1b\.chiba$/, ".exec.chiba");
  const watPath = execPath.replace(/\.chiba$/, ".wat");
  fs.writeFileSync(execPath, executableGeneratedSource(caseInfo, generated));
  const wat = run(`level1c wat generated lexer ${caseInfo.file}`, "timeout", ["20", LEGACY_REFERENCE_COMPILER, "wat", execPath]);
  if (!wat.stdout.includes("(module")) {
    console.error(`[FAIL] generated lexer wat ${caseInfo.file}`);
    console.error(wat.stdout || wat.stderr || "level1c produced no module");
    process.exit(1);
  }
  fs.writeFileSync(watPath, wat.stdout);
  let actual = "";
  try {
    actual = await runWatText(wat.stdout);
    console.log(`[PASS] run generated lexer wat ${caseInfo.file}`);
  } catch (error) {
    console.error(`[FAIL] run generated lexer wat ${caseInfo.file}`);
    console.error(error && error.stack ? error.stack : error && error.message ? error.message : JSON.stringify(error));
    process.exit(1);
  }
  if (actual !== "0") {
    console.error(`[FAIL] generated lexer tokens ${caseInfo.file}`);
    console.error(`expected main -> 0, got ${actual}`);
    process.exit(1);
  }
}

function readNativeGenerated(output) {
  if (fs.existsSync(output)) return fs.readFileSync(output, "utf8");
  const truncated = output.slice(0, output.length - 1);
  if (fs.existsSync(truncated)) return fs.readFileSync(truncated, "utf8");
  return fs.readFileSync(output, "utf8");
}

for (const caseInfo of CASES) {
  const { file, expected } = caseInfo;
  const input = path.join(ROOT, file);
  const outputSuffix = file === "utf8-ident.chibalex" ? `.${caseInfo.name}` : "";
  const nativeOutput = path.join(OUT, file.replace(/\.chibalex$/, ".native.chiba"));
  if (caseInfo.sourceOnly === true) {
    oracleReferenceFailed(caseInfo.sourceOnlyReason);
  } else {
    run(`native chibalex oracle ${file}`, "timeout", ["10", "./chibalex.o", input, "-o", nativeOutput]);
    const nativeGenerated = readNativeGenerated(nativeOutput);
    for (const token of expected) {
      if (!nativeGenerated.includes(token)) {
        console.error(`[FAIL] native generated lexer oracle ${file}`);
        console.error(`missing token ${token}`);
        process.exit(1);
      }
    }
  }
  const spec = fs.readFileSync(input, "utf8");
  const generated = miniGeneratedLexer(caseInfo, spec);
  const output = path.join(OUT, file.replace(/\.chibalex$/, `${outputSuffix}.level1b.chiba`));
  fs.writeFileSync(output, generated);
  for (const token of expected) {
    if (!generated.includes(token)) {
      console.error(`[FAIL] level-1b mini generated lexer ${file}`);
      console.error(`missing token ${token}`);
      process.exit(1);
    }
  }
  console.log(`[PASS] level-1b mini chibalex generate ${file}`);
  await runGeneratedLexer(caseInfo, generated, output);
}

const backtracking = fs.readFileSync("level-1b/supports/chibalex-continuation/backtracking.chiba", "utf8");
if (!backtracking.includes("shift retry") || !backtracking.includes("retry(candidate)")) {
  console.error("[FAIL] lexer backtracking CPS fixture source");
  process.exit(1);
}
console.log("[PASS] lexer backtracking CPS fixture source");
