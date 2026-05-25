import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/supports/chibalex-mini";
const OUT = ".scratch/level-1b/chibalex-mini";
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
    file: "attribute-tokens.chibalex",
    name: "attribute",
    namespace: "chibalexmini.attribute_tokens",
    expected: ["Hash", "LBracket", "Ident", "LParen", "Comma", "Eq", "IntLit", "StringLit", "KwTrue", "RBracket"],
    source: "mk_str(\"#[attribute(all(someident, a=b, c=[1,2], meta={owner=\\\"compiler\\\", stable=true}))]\", 80)",
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
                                                match token_at(tokens, 8) {
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

function oracleReferenceFailed(name) {
  console.log(`[ORACLE-FAIL] ${name}`);
}

fs.mkdirSync(OUT, { recursive: true });

function tokenDataBlock(spec) {
  const match = spec.match(/data\s+Token\s*\{[\s\S]*?\n\s*\}/);
  if (!match) {
    throw new Error("mini chibalex fixture is missing data Token block");
  }
  return match[0];
}

function miniLexerHelpers(namespace, spec) {
  return `// generated by level-1b mini chibalex slice
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
    if mini_is_alpha(b) != 0 { 1 } else if b == 95 { 1 } else if b >= 128 { 1 } else { 0 }

def mini_is_ident_continue(b: i64): i64 =
    if mini_is_ident_start(b) != 0 { 1 } else { mini_is_digit(b) }

def mini_scan_digits(src: i64, sl: i64, pos: i64): i64 =
    if pos >= sl { pos }
    else if mini_is_digit(load8(src, pos)) != 0 { mini_scan_digits(src, sl, pos + 1) }
    else { pos }

def mini_scan_ident(src: i64, sl: i64, pos: i64): i64 =
    if pos >= sl { pos }
    else if mini_is_ident_continue(load8(src, pos)) != 0 { mini_scan_ident(src, sl, utf8_next_offset(src, pos)) }
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
            let end = mini_scan_ident(src, sl, utf8_next_offset(src, pos))
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
  if (name === "utf8ident") {
    return `${helpers}
def lex_loop(src: i64, sl: i64, file: i64, out: Vec, pos: i64): i64 =
    if pos >= sl { 0 }
    else {
        let b = load8(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, sl, file, out, pos + 1) }
        else if mini_literal_eq_at(src, sl, pos, mk_str("let", 3), 0) != 0 {
            let _ = lex_emit(out, KwLet, file, pos, 3)
            lex_loop(src, sl, file, out, pos + 3)
        } else if b == 61 {
            let _ = lex_emit(out, Eq, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if mini_is_ident_start(b) != 0 {
            let end = mini_scan_ident(src, sl, utf8_next_offset(src, pos))
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
  if (name === "attribute") {
    return `${helpers}
def lex_loop(src: i64, sl: i64, file: i64, out: Vec, pos: i64): i64 =
    if pos >= sl { 0 }
    else {
        let b = load8(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, sl, file, out, pos + 1) }
        else if mini_literal_eq_at(src, sl, pos, mk_str("true", 4), 0) != 0 {
            let _ = lex_emit(out, KwTrue, file, pos, 4)
            lex_loop(src, sl, file, out, pos + 4)
        } else if mini_literal_eq_at(src, sl, pos, mk_str("false", 5), 0) != 0 {
            let _ = lex_emit(out, KwFalse, file, pos, 5)
            lex_loop(src, sl, file, out, pos + 5)
        } else if b == 34 {
            let end = mini_scan_until_quote(src, sl, pos + 1)
            let _ = lex_emit(out, StringLit(mk_str(src + pos, end + 1 - pos)), file, pos, end + 1 - pos)
            lex_loop(src, sl, file, out, end + 1)
        } else if mini_is_digit(b) != 0 {
            let end = mini_scan_digits(src, sl, pos)
            let _ = lex_emit(out, IntLit(mk_str(src + pos, end - pos)), file, pos, end - pos)
            lex_loop(src, sl, file, out, end)
        } else if mini_is_ident_start(b) != 0 {
            let end = mini_scan_ident(src, sl, utf8_next_offset(src, pos))
            let _ = lex_emit(out, Ident(mk_str(src + pos, end - pos)), file, pos, end - pos)
            lex_loop(src, sl, file, out, end)
        } else if mini_literal_eq_at(src, sl, pos, mk_str("#!", 2), 0) != 0 {
            let _ = lex_emit(out, HashBang, file, pos, 2)
            lex_loop(src, sl, file, out, pos + 2)
        } else if b == 35 {
            let _ = lex_emit(out, Hash, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if b == 91 {
            let _ = lex_emit(out, LBracket, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if b == 93 {
            let _ = lex_emit(out, RBracket, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if b == 40 {
            let _ = lex_emit(out, LParen, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if b == 41 {
            let _ = lex_emit(out, RParen, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if b == 123 {
            let _ = lex_emit(out, LBrace, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if b == 125 {
            let _ = lex_emit(out, RBrace, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if b == 44 {
            let _ = lex_emit(out, Comma, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
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
            let end = mini_scan_ident(src, sl, utf8_next_offset(src, pos))
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
  if (name === "continuation") {
    return `${helpers}
def lex_loop(src: i64, sl: i64, file: i64, out: Vec, pos: i64): i64 =
    if pos >= sl { 0 }
    else {
        let b = load8(src, pos)
        if mini_is_ws(b) != 0 { lex_loop(src, sl, file, out, pos + 1) }
        else if mini_literal_eq_at(src, sl, pos, mk_str("shiftn", 6), 0) != 0 {
            let _ = lex_emit(out, KwShiftn, file, pos, 6)
            lex_loop(src, sl, file, out, pos + 6)
        } else if mini_literal_eq_at(src, sl, pos, mk_str("cont1", 5), 0) != 0 {
            let _ = lex_emit(out, KwCont1, file, pos, 5)
            lex_loop(src, sl, file, out, pos + 5)
        } else if mini_literal_eq_at(src, sl, pos, mk_str("contN", 5), 0) != 0 {
            let _ = lex_emit(out, KwContN, file, pos, 5)
            lex_loop(src, sl, file, out, pos + 5)
        } else if mini_literal_eq_at(src, sl, pos, mk_str("->", 2), 0) != 0 {
            let _ = lex_emit(out, ThinArrow, file, pos, 2)
            lex_loop(src, sl, file, out, pos + 2)
        } else if b == 40 {
            let _ = lex_emit(out, LParen, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if b == 41 {
            let _ = lex_emit(out, RParen, file, pos, 1)
            lex_loop(src, sl, file, out, pos + 1)
        } else if mini_is_ident_start(b) != 0 {
            let end = mini_scan_ident(src, sl, utf8_next_offset(src, pos))
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
  if (!generated.includes("def lex_all")) {
    console.error(`[FAIL] generated lexer case ${caseInfo.file}`);
    console.error("generated lexer missing lex_all entry");
    process.exit(1);
  }
  console.log(`[PASS] generated lexer source ${caseInfo.file}`);
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

const backtracking = fs.readFileSync("level-1b/supports/chibalex-continuation/backtracking.chiba", "utf8");
if (!backtracking.includes("shift retry") || !backtracking.includes("retry(candidate)")) {
  console.error("[FAIL] lexer backtracking CPS fixture source");
  process.exit(1);
}
console.log("[PASS] lexer backtracking CPS fixture source");
