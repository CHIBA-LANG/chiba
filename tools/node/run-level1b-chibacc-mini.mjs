import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/supports/chibacc-mini";
const OUT = ".scratch/level-1b/chibacc-mini";
const RUNNERS = ".scratch/level-1b/chibacc-mini-runners";
const CASES = [
  {
    file: "simple.chibacc",
    namespace: "level1b.chibaccmini.simple",
    expected: ["parse_rule", "Assign"],
    tokens: ["Ident(mk_str(\"x\", 1))", "Eq", "IntLit(mk_str(\"7\", 1))"],
    check: `
        Assign(name, value) =>
            if streq(name, mk_str("x", 1)) != 0 {
                if streq(value, mk_str("7", 1)) != 0 { 0 } else { 3 }
            } else { 4 }
        _ => 5
`,
  },
  {
    file: "pratt.chibacc",
    namespace: "level1b.chibaccmini.pratt",
    expected: ["parse_rule_0_bp", "Expr_Binary", "OpAdd"],
    tokens: ["IntLit(mk_str(\"1\", 1))", "Plus", "IntLit(mk_str(\"2\", 1))"],
    check: `
        Expr_Binary(op, lhs, rhs) =>
            match op {
                OpAdd => 0
                _ => 3
            }
        _ => 4
`,
  },
  {
    name: "recover",
    file: "pratt.chibacc",
    namespace: "level1b.chibaccmini.pratt",
    expected: ["recover_pos", "RParen"],
    tokens: ["LParen", "RParen"],
  },
  {
    file: "list.chibacc",
    namespace: "level1b.chibaccmini.list",
    expected: ["Name_Cons", "Name_End"],
    tokens: ["Ident(mk_str(\"a\", 1))", "Comma", "Ident(mk_str(\"b\", 1))"],
    check: `
        Name_Cons(head, tail) =>
            if streq(head, mk_str("a", 1)) != 0 {
                match tail {
                    Name_Cons(head2, tail2) =>
                        if streq(head2, mk_str("b", 1)) != 0 {
                            match tail2 {
                                Name_End => 0
                                _ => 5
                            }
                        } else { 4 }
                    _ => 3
                }
            } else { 6 }
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

fs.mkdirSync(OUT, { recursive: true });
fs.mkdirSync(RUNNERS, { recursive: true });

function tokenPrelude(namespace) {
  return `namespace ${namespace}
use metalstd.str.*
use metalstd.vec.*

data Token {
    Eof,
    Ident(Str),
    IntLit(Str),
    Eq,
    Plus,
    Minus,
    Comma,
    LParen,
    RParen,
}

data Option[T] {
    None,
    Some(T)
}

type Span { file: i64  line: i64  col: i64  len: i64 }
type TokenSpan { token: Token  span: Span  leading: Vec  trailing: Vec }

def span0(): Span = Span { file: 0, line: 0, col: 0, len: 0 }

def tokenspan_make(tok: Token, span: Span, leading: Vec, trailing: Vec): TokenSpan =
    TokenSpan { token: tok, span: span, leading: leading, trailing: trailing }
`;
}

function mainSource(namespace, tokens, check) {
  const pushes = tokens
    .map((token) => `    let _ = push_token(tokens, ${token})`)
    .join("\n");
  const resultCheck =
    check == null
      ? `    match parse_tokens(tokens) {
        Err(fail, errors) =>
            match fail {
                Some(ast) => 0
                None => 3
            }
        OK(ast, errors) => 2
    }`
      : `    match parse_tokens(tokens) {
        OK(ast, errors) => {
            let node = ast as AST
            match node {${check}            }
        }
        Err(fail, errors) => 2
    }`;
  return `namespace ${namespace}
use metalstd.str.*
use metalstd.vec.*

def push_token(tokens: Vec, tok: Token): i64 =
    vec_push(tokens, tokenspan_make(tok, span0(), vec_new(), vec_new()))

#[entry]
def main(argc: i64, argv: i64): i64 = {
    let tokens = vec_new()
${pushes}
${resultCheck}
}
`;
}

function runGeneratedParser(caseInfo, generated) {
  const name = caseInfo.name ?? caseInfo.file.replace(/\.chibacc$/, "");
  const label = caseInfo.name == null ? caseInfo.file : `${caseInfo.file}:${caseInfo.name}`;
  const project = path.join(RUNNERS, name);
  const src = path.join(project, "src");
  fs.rmSync(project, { recursive: true, force: true });
  fs.mkdirSync(src, { recursive: true });
  fs.cpSync("src/metalstd", path.join(src, "metalstd"), { recursive: true });
  for (const file of fs.readdirSync("src/metalstd")) {
    if (file.endsWith(".chiba")) {
      fs.copyFileSync(path.join("src/metalstd", file), path.join(src, file));
    }
  }
  fs.writeFileSync(
    path.join(src, "main.chiba"),
    `${tokenPrelude(caseInfo.namespace)}\n${generated}\n${mainSource(caseInfo.namespace, caseInfo.tokens, caseInfo.check)}`,
  );
  run(`generated parser compile ${label}`, "timeout", [
    "10",
    "./chibac_amd64-unknown-linux_chiba_dev.o",
    "--project",
    project,
    "--entry",
    "main.chiba",
    "--output",
    "runner.o",
  ]);
  run(`generated parser run ${label}`, path.join(project, "target/debug/runner.o"), []);
}

for (const caseInfo of CASES) {
  const { file, expected } = caseInfo;
  const input = path.join(ROOT, file);
  const output = path.join(OUT, file.replace(/\.chibacc$/, ".chiba"));
  run(`native chibacc ${file}`, "timeout", ["10", "./chibacc.o", input, "-o", output]);
  const generated = fs.readFileSync(output, "utf8");
  for (const text of expected) {
    if (!generated.includes(text)) {
      console.error(`[FAIL] generated parser ${file}`);
      console.error(`missing text ${text}`);
      process.exit(1);
    }
  }
  runGeneratedParser(caseInfo, generated);
}
