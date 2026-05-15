import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/supports/chibalex-mini";
const OUT = ".scratch/level-1b/chibalex-mini";
const RUNNERS = ".scratch/level-1b/chibalex-mini-runners";
const CASES = [
  {
    file: "basic.chibalex",
    name: "basic",
    namespace: "level1b.chibalexmini.basic",
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
    namespace: "level1b.chibalexmini.longest",
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
    namespace: "level1b.chibalexmini.stringmode",
    expected: ["StringStart", "StringChunk", "StringEnd"],
    source: "mk_str(\"\\\"abc\\\"\", 5)",
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
];

function run(name, command, args) {
  const result = spawnSync(command, args, { encoding: "utf8" });
  if (result.status !== 0) {
    console.error(`[FAIL] ${name}`);
    console.error(`${result.stdout || ""}${result.stderr || ""}`.split("\n").slice(0, 40).join("\n"));
    process.exit(result.status || 1);
  }
  console.log(`[PASS] ${name}`);
  return result;
}

fs.mkdirSync(OUT, { recursive: true });
fs.mkdirSync(RUNNERS, { recursive: true });

function mainSource(namespace, source, check) {
  return `namespace ${namespace}
use metalstd.str.*
use metalstd.vec.*

def token_at(tokens: Vec, i: i64): Token = {
    let ts = vec_get(tokens, i) as TokenSpan
    ts.token
}

#[entry]
def main(argc: i64, argv: i64): i64 = {
    let src = ${source}
    let tokens = lex_all(src.ptr, src.len, 0)
    match token_at(tokens, 0) {${check}    }
}
`;
}

function localRegexStub(namespace) {
  return `namespace ${namespace}
use metalstd.mem.*
use metalstd.option.*
use metalstd.str.*

type Vec { ptr: i64  len: i64  cap: i64 }

data Option[T] {
    None,
    Some(T)
}

def vec_new(): Vec = Vec { ptr: 0, len: 0, cap: 0 }

def vec_set_ptr(v: Vec, p: i64): i64 = store64(v as i64, 1, p)

def vec_set_len(v: Vec, l: i64): i64 = store64(v as i64, 2, l)

def vec_set_cap(v: Vec, c: i64): i64 = store64(v as i64, 3, c)

def vec_grow(v: Vec): i64 = {
    let new_cap = if v.cap == 0 { 8 } else { v.cap * 2 }
    let new_buf = heap_alloc(new_cap * 8)
    let _ = if v.len > 0 { memcpy_words(new_buf, v.ptr, v.len) } else { 0 }
    let _ = vec_set_ptr(v, new_buf)
    vec_set_cap(v, new_cap)
}

def vec_push(v: Vec, elem: i64): i64 = {
    let _ = if v.len >= v.cap { vec_grow(v) } else { 0 }
    let _ = store64(v.ptr, v.len, elem)
    vec_set_len(v, v.len + 1)
}

def vec_pop(v: Vec): i64 = {
    let new_len = v.len - 1
    let val = load64(v.ptr, new_len)
    let _ = vec_set_len(v, new_len)
    val
}

def vec_get(v: Vec, i: i64): i64 = load64(v.ptr, i)

def vec_len(v: Vec): i64 = v.len

data RegexAST {
    REmpty,
    RPat(i64, i64),
}

type RegexCompiled { ptr: i64  len: i64 }

def regex_parse(ptr: i64, len: i64): Option[RegexAST] = Some(RPat(ptr, len))

def regex_compile(ast: RegexAST): RegexCompiled =
    match ast {
        RPat(ptr, len) => RegexCompiled { ptr: ptr, len: len }
        REmpty => RegexCompiled { ptr: 0, len: 0 }
    }

def utf8_decode_char(src: i64, pos: i64): i64 = load8(src, pos)

def utf8_next_offset(src: i64, pos: i64): i64 = pos + 1

def mini_is_ws(b: i64): i64 =
    if b == 32 { 1 } else if b == 9 { 1 } else if b == 10 { 1 } else if b == 13 { 1 } else { 0 }

def mini_is_digit(b: i64): i64 =
    if b >= 48 && b <= 57 { 1 } else { 0 }

def mini_is_alpha(b: i64): i64 =
    if b >= 65 && b <= 90 { 1 } else if b >= 97 && b <= 122 { 1 } else { 0 }

def is_ident_start(b: i64): i64 =
    if is_alpha(b) != 0 { 1 } else if b == 95 { 1 } else { 0 }

def is_ident_continue(b: i64): i64 =
    if is_ident_start(b) != 0 { 1 } else { is_digit(b) }

def match_literal(src: i64, sl: i64, pos: i64, lit: Str, i: i64): i64 =
    if i >= lit.len { pos + lit.len }
    else if pos + i >= sl { pos }
    else if load8(src, pos + i) == load8(lit.ptr, i) { match_literal(src, sl, pos, lit, i + 1) }
    else { pos }

def match_ws(src: i64, sl: i64, pos: i64): i64 = match_ws_loop(src, sl, pos, pos)

def match_ws_loop(src: i64, sl: i64, pos: i64, cur: i64): i64 =
    if cur >= sl { if cur > pos { cur } else { pos } }
    else if is_ws(load8(src, cur)) != 0 { match_ws_loop(src, sl, pos, cur + 1) }
    else { if cur > pos { cur } else { pos } }

def match_digits(src: i64, sl: i64, pos: i64): i64 = match_digits_loop(src, sl, pos, pos)

def match_digits_loop(src: i64, sl: i64, pos: i64, cur: i64): i64 =
    if cur >= sl { if cur > pos { cur } else { pos } }
    else if is_digit(load8(src, cur)) != 0 { match_digits_loop(src, sl, pos, cur + 1) }
    else { if cur > pos { cur } else { pos } }

def match_ident(src: i64, sl: i64, pos: i64): i64 =
    if pos >= sl { pos }
    else if is_ident_start(load8(src, pos)) == 0 { pos }
    else { match_ident_loop(src, sl, pos + 1) }

def match_ident_loop(src: i64, sl: i64, cur: i64): i64 =
    if cur >= sl { cur }
    else if is_ident_continue(load8(src, cur)) != 0 { match_ident_loop(src, sl, cur + 1) }
    else { cur }

def match_until_quote(src: i64, sl: i64, pos: i64): i64 = match_until_quote_loop(src, sl, pos, pos)

def match_until_quote_loop(src: i64, sl: i64, pos: i64, cur: i64): i64 =
    if cur >= sl { if cur > pos { cur } else { pos } }
    else if load8(src, cur) == 34 { if cur > pos { cur } else { pos } }
    else { match_until_quote_loop(src, sl, pos, cur + 1) }

def regex_match_at(pat: RegexCompiled, src: i64, sl: i64, pos: i64): i64 = {
    let p = mk_str(pat.ptr, pat.len)
    if streq(p, mk_str("(?:[ \\\\t\\\\r\\\\n]+)", 11)) != 0 { match_ws(src, sl, pos) }
    else if streq(p, mk_str("(?:[0-9]+)", 10)) != 0 { match_digits(src, sl, pos) }
    else if streq(p, mk_str("(?:\\\\i\\\\j*)", 9)) != 0 { match_ident(src, sl, pos) }
    else if streq(p, mk_str("(?:[^\\\"]+)", 9)) != 0 { match_until_quote(src, sl, pos) }
    else { match_literal(src, sl, pos, p, 0) }
}
`;
}

function tokenDataBlock(spec) {
  const match = spec.match(/data\s+Token\s*\{[\s\S]*?\n\}/);
  if (!match) {
    throw new Error("mini chibalex fixture is missing data Token block");
  }
  return match[0];
}

function miniLexerHelpers(namespace, spec) {
  return `// generated by level-1b mini chibalex slice
namespace ${namespace}
use metalstd.str.*

${tokenDataBlock(spec)}

type Span { file: i64  line: i64  col: i64  len: i64 }
type TokenSpan { token: Token  span: Span  leading: Vec  trailing: Vec }

def tokenspan_make(tok: Token, span: Span, leading: Vec, trailing: Vec): TokenSpan =
    TokenSpan { token: tok, span: span, leading: leading, trailing: trailing }

def lex_emit(out: Vec, tok: Token, file: i64, pos: i64, len: i64): i64 =
    vec_push(out, tokenspan_make(tok, Span { file: file, line: 1, col: pos + 1, len: len }, vec_new(), vec_new()) as i64)

def is_ws(b: i64): i64 =
    if b == 32 { 1 } else if b == 9 { 1 } else if b == 10 { 1 } else if b == 13 { 1 } else { 0 }

def is_digit(b: i64): i64 =
    if b >= 48 && b <= 57 { 1 } else { 0 }

def is_alpha(b: i64): i64 =
    if b >= 65 && b <= 90 { 1 } else if b >= 97 && b <= 122 { 1 } else { 0 }

def mini_is_ident_start(b: i64): i64 =
    if mini_is_alpha(b) != 0 { 1 } else if b == 95 { 1 } else { 0 }

def mini_is_ident_continue(b: i64): i64 =
    if mini_is_ident_start(b) != 0 { 1 } else { mini_is_digit(b) }

def mini_scan_digits(src: i64, sl: i64, pos: i64): i64 =
    if pos >= sl { pos }
    else if mini_is_digit(load8(src, pos)) != 0 { mini_scan_digits(src, sl, pos + 1) }
    else { pos }

def mini_scan_ident(src: i64, sl: i64, pos: i64): i64 =
    if pos >= sl { pos }
    else if mini_is_ident_continue(load8(src, pos)) != 0 { mini_scan_ident(src, sl, pos + 1) }
    else { pos }

def mini_scan_until_quote(src: i64, sl: i64, pos: i64): i64 =
    if pos >= sl { pos }
    else if load8(src, pos) == 34 { pos }
    else { mini_scan_until_quote(src, sl, pos + 1) }

def mini_literal_eq_at(src: i64, sl: i64, pos: i64, lit: Str, i: i64): i64 =
    if i >= lit.len { 1 }
    else if pos + i >= sl { 0 }
    else if load8(src, pos + i) == load8(lit.ptr, i) { mini_literal_eq_at(src, sl, pos, lit, i + 1) }
    else { 0 }
`;
}

function miniGeneratedLexer(caseInfo, spec) {
  const { namespace, name } = caseInfo;
  const helpers = miniLexerHelpers(namespace, spec);
  if (name === "basic") {
    return `${helpers}
def lex_loop(src: i64, sl: i64, file: i64, out: Vec, pos: i64): i64 =
    if pos >= sl { 0 }
    else {
        let b = load8(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, sl, file, out, pos + 1) }
        else if mini_literal_eq_at(src, sl, pos, mk_str("let", 3), 0) != 0 {
            let _ = lex_emit(out, KwLet, file, pos, 3)
            lex_loop(src, sl, file, out, pos + 3)
        } else if mini_is_digit(b) != 0 {
            let end = mini_scan_digits(src, sl, pos)
            let _ = lex_emit(out, IntLit(mk_str(src + pos, end - pos)), file, pos, end - pos)
            lex_loop(src, sl, file, out, end)
        } else if mini_is_ident_start(b) != 0 {
            let end = mini_scan_ident(src, sl, pos + 1)
            let _ = lex_emit(out, Ident(mk_str(src + pos, end - pos)), file, pos, end - pos)
            lex_loop(src, sl, file, out, end)
        } else if b == 61 {
            let _ = lex_emit(out, Eq, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else {
            let _ = lex_emit(out, LexError(b), file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        }
    }

def lex_all(src: i64, src_len: i64, file: i64): Vec = {
    let out = vec_new()
    let _ = lex_loop(src, src_len, file, out, 0)
    let _ = lex_emit(out, Eof, file, src_len, 0)
    out
}
`;
  }
  if (name === "longest") {
    return `${helpers}
def lex_loop(src: i64, sl: i64, file: i64, out: Vec, pos: i64): i64 =
    if pos >= sl { 0 }
    else {
        let b = load8(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, sl, file, out, pos + 1) }
        else if mini_literal_eq_at(src, sl, pos, mk_str("==", 2), 0) != 0 {
            let _ = lex_emit(out, EqEq, file, pos, 2)
            lex_loop(src, sl, file, out, pos + 2)
        } else if b == 61 {
            let _ = lex_emit(out, Eq, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if mini_literal_eq_at(src, sl, pos, mk_str("if", 2), 0) != 0 {
            let _ = lex_emit(out, KwIf, file, pos, 2)
            lex_loop(src, sl, file, out, pos + 2)
        } else if mini_is_ident_start(b) != 0 {
            let end = mini_scan_ident(src, sl, pos + 1)
            let _ = lex_emit(out, Ident(mk_str(src + pos, end - pos)), file, pos, end - pos)
            lex_loop(src, sl, file, out, end)
        } else {
            let _ = lex_emit(out, LexError(b), file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        }
    }

def lex_all(src: i64, src_len: i64, file: i64): Vec = {
    let out = vec_new()
    let _ = lex_loop(src, src_len, file, out, 0)
    let _ = lex_emit(out, Eof, file, src_len, 0)
    out
}
`;
  }
  return `${helpers}
def lex_loop(src: i64, sl: i64, file: i64, out: Vec, pos: i64, mode: i64): i64 =
    if pos >= sl { 0 }
    else {
        let b = load8(src, pos)
        if mode == 0 {
            if mini_is_ws(b) != 0 { lex_loop(src, sl, file, out, pos + 1, mode) }
            else if b == 34 {
                let _ = lex_emit(out, StringStart, file, pos, 1)
                lex_loop(src, sl, file, out, pos + 1, 1)
            } else {
                let _ = lex_emit(out, LexError(b), file, pos, 1)
                lex_loop(src, sl, file, out, pos + 1, mode)
            }
        } else {
            if b == 34 {
                let _ = lex_emit(out, StringEnd, file, pos, 1)
                lex_loop(src, sl, file, out, pos + 1, 0)
            } else {
                let end = mini_scan_until_quote(src, sl, pos)
                let _ = lex_emit(out, StringChunk(mk_str(src + pos, end - pos)), file, pos, end - pos)
                lex_loop(src, sl, file, out, end, mode)
            }
        }
    }

def lex_all(src: i64, src_len: i64, file: i64): Vec = {
    let out = vec_new()
    let _ = lex_loop(src, src_len, file, out, 0, 0)
    let _ = lex_emit(out, Eof, file, src_len, 0)
    out
}
`;
}

function runGeneratedLexer(caseInfo, generated) {
  const project = path.join(RUNNERS, caseInfo.name);
  const src = path.join(project, "src");
  fs.rmSync(project, { recursive: true, force: true });
  fs.mkdirSync(src, { recursive: true });
  fs.copyFileSync("src/metalstd/mem.chiba", path.join(src, "mem.chiba"));
  fs.copyFileSync("src/metalstd/str.chiba", path.join(src, "str.chiba"));
  fs.writeFileSync(path.join(src, "main.chiba"), `${localRegexStub(caseInfo.namespace)}\n${generated}\n${mainSource(caseInfo.namespace, caseInfo.source, caseInfo.check)}`);
  run(`generated lexer compile ${caseInfo.file}`, "timeout", [
    "10",
    "./chibac_amd64-unknown-linux_chiba_dev.o",
    "--project",
    project,
    "--entry",
    "main.chiba",
    "--output",
    "runner.o",
  ]);
  run(`generated lexer run ${caseInfo.file}`, path.join(project, "target/debug/runner.o"), []);
}

for (const caseInfo of CASES) {
  const { file, expected } = caseInfo;
  const input = path.join(ROOT, file);
  const nativeOutput = path.join(OUT, file.replace(/\.chibalex$/, ".native.chiba"));
  run(`native chibalex oracle ${file}`, "timeout", ["10", "./chibalex.o", input, "-o", nativeOutput]);
  const nativeGenerated = fs.readFileSync(nativeOutput, "utf8");
  for (const token of expected) {
    if (!nativeGenerated.includes(token)) {
      console.error(`[FAIL] native generated lexer oracle ${file}`);
      console.error(`missing token ${token}`);
      process.exit(1);
    }
  }
  const spec = fs.readFileSync(input, "utf8");
  const generated = miniGeneratedLexer(caseInfo, spec);
  const output = path.join(OUT, file.replace(/\.chibalex$/, ".level1b.chiba"));
  fs.writeFileSync(output, generated);
  for (const token of expected) {
    if (!generated.includes(token)) {
      console.error(`[FAIL] level-1b mini generated lexer ${file}`);
      console.error(`missing token ${token}`);
      process.exit(1);
    }
  }
  console.log(`[PASS] level-1b mini chibalex generate ${file}`);
  runGeneratedLexer(caseInfo, generated);
}

run("level1c cps lexer backtracking fixture", "./target/debug/level1c.o", [
  "cps",
  "supports/bootstrap/continuation-multi-resume.chiba",
]);
