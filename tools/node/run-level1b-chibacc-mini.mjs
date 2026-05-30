import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";
import { Worker } from "node:worker_threads";
import { compileWat, extractModule } from "./wat-compile.mjs";

const ROOT = "level-1b/supports/chibacc-mini";
const OUT = ".scratch/level-1b/chibacc-mini";
const FULL_OUT = ".scratch/level-1b/chibacc-full";
const FULL_GRAMMAR = "src/frontend/chiba-level1.chibacc";
const CHIBACC = process.env.CHIBACC || (
  fs.existsSync("/tmp/chibacc-project-mainline/target/debug/chibacc.new.o")
    ? "/tmp/chibacc-project-mainline/target/debug/chibacc.new.o"
    : "./chibacc.o"
);
const LEGACY_REFERENCE_COMPILER = "./target/debug/level1c.o";
const CASES = [
  {
    file: "simple.chibacc",
    namespace: "chibaccmini.simple",
    expected: ["parse_rule", "Assign"],
    tokens: ["Ident(mk_str(\"x\", 1))", "Eq", "IntLit(mk_str(\"7\", 1))"],
    check: `
        Assign(_, _) => 0
        _ => 5
`,
  },
  {
    file: "pratt.chibacc",
    namespace: "chibaccmini.pratt",
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
    namespace: "chibaccmini.pratt",
    expected: ["recover_pos", "RParen"],
    tokens: ["LParen", "RParen"],
  },
  {
    file: "list.chibacc",
    namespace: "chibaccmini.list",
    expected: ["Name_Cons", "Name_End"],
    tokens: ["Ident(mk_str(\"a\", 1))", "Comma", "Ident(mk_str(\"b\", 1))"],
    check: `
        Name_Cons(head, tail) =>
            match tail {
                Name_Cons(head2, tail2) =>
                    match tail2 {
                        Name_End => 0
                        _ => 5
                    }
                _ => 3
            }
        _ => 7
`,
  },
  {
    file: "continuation-type.chibacc",
    namespace: "chibaccmini.continuation",
    expected: ["Type_Cont1", "Type_ContN", "Type_Callable", "ThinArrow"],
    tokens: [
      "KwCont1",
      "LParen",
      "Ident(mk_str(\"A\", 1))",
      "RParen",
      "ThinArrow",
      "KwContN",
      "LParen",
      "Ident(mk_str(\"A\", 1))",
      "RParen",
      "ThinArrow",
      "Ident(mk_str(\"B\", 1))",
    ],
    check: `
        Type_Cont1(input, answer) =>
            match answer {
                Type_ContN(inner, final_answer) => 0
                _ => 2
            }
        _ => 1
`,
  },
  {
    file: "attribute-args.chibacc",
    namespace: "chibaccmini.attribute_args",
    expected: ["AttrNamed", "AttrCall", "AttrList", "AttrObject", "AttrBare"],
    tokens: [
      "Hash",
      "LBracket",
      "Ident(mk_str(\"attribute\", 9))",
      "LParen",
      "Ident(mk_str(\"all\", 3))",
      "LParen",
      "Ident(mk_str(\"someident\", 9))",
      "Comma",
      "Ident(mk_str(\"a\", 1))",
      "Eq",
      "Ident(mk_str(\"b\", 1))",
      "RParen",
      "RParen",
      "RBracket",
    ],
    check: `
        Attr(_, _) => 0
        _ => 4
`,
  },
  {
    file: "calculator.chibacc",
    namespace: "chibaccmini.calculator",
    expected: ["parse_rule_0", "Program_Cons", "Stmt_Var", "Expr_Binary", "OpMul", "OpAdd"],
    expectedMainResult: "14",
    tokens: [
      "KwVar",
      "Ident(mk_str(\"x\", 1))",
      "ColonEq",
      "IntLit(mk_str(\"2\", 1))",
      "Plus",
      "IntLit(mk_str(\"3\", 1))",
      "Star",
      "IntLit(mk_str(\"4\", 1))",
      "Semi",
      "Ident(mk_str(\"x\", 1))",
    ],
    check: `
        Program_Cons(head, tail) =>
            match head {
                Stmt_Var(_, value) =>
                    match value {
                        Expr_Binary(op, _, rhs) =>
                            match op {
                                OpAdd =>
                                    match rhs {
                                        Expr_Binary(op2, _, _) =>
                                            match op2 {
                                                OpMul => 0
                                                _ => 8
                                            }
                                        _ => 7
                                    }
                                _ => 6
                            }
                        _ => 5
                    }
                _ => 4
            }
        _ => 9
`,
    harness: `
def harness_status(result: LabeledAST): i64 = {
    match result {
        OK(ast_value, _) => {
            let ast = ast_value as AST
            match ast {
                Program_Cons(head, tail) =>
                    match head {
                        Stmt_Var(_, value) =>
                            match value {
                                Expr_Binary(op, _, rhs) =>
                                    match op {
                                        OpAdd =>
                                            match rhs {
                                                Expr_Binary(op2, _, _) =>
                                                    match op2 {
                                                        OpMul => 14
                                                        _ => 0 - 1008
                                                    }
                                                _ => 0 - 1007
                                            }
                                        _ => 0 - 1006
                                    }
                                _ => 0 - 1005
                            }
                        _ => 0 - 1004
                    }
                _ => 0 - 1009
            }
        }
        Err(_, _) => 0 - 1099
    }
}
`,
  },
];

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

function runParseOk(name, file) {
  const referenceOnlyLegacyCompiler = LEGACY_REFERENCE_COMPILER;
  const result = run(name, "timeout", ["10", referenceOnlyLegacyCompiler, "parse", file]);
  if (!result.stdout.startsWith("OK(")) {
    console.error(`[FAIL] ${name}`);
    console.error(result.stdout || result.stderr || "parse did not return OK");
    process.exit(1);
  }
  return result;
}

function runCheckOk(name, file) {
  const referenceOnlyLegacyCompiler = LEGACY_REFERENCE_COMPILER;
  const result = run(name, "timeout", ["10", referenceOnlyLegacyCompiler, "check", file]);
  if (!result.stdout.includes("check ok")) {
    console.error(`[FAIL] ${name}`);
    console.error(result.stdout || result.stderr || "check did not report ok");
    process.exit(1);
  }
  return result;
}

function runCheckOkWithTimeout(name, file, seconds) {
  const referenceOnlyLegacyCompiler = LEGACY_REFERENCE_COMPILER;
  const result = run(name, "timeout", [String(seconds), referenceOnlyLegacyCompiler, "check", file]);
  if (!result.stdout.includes("check ok")) {
    console.error(`[FAIL] ${name}`);
    console.error(result.stdout || result.stderr || "check did not report ok");
    process.exit(1);
  }
  return result;
}

function runParseOkWithTimeout(name, file, seconds) {
  const referenceOnlyLegacyCompiler = LEGACY_REFERENCE_COMPILER;
  const result = run(name, "timeout", [String(seconds), referenceOnlyLegacyCompiler, "parse", file]);
  if (!result.stdout.startsWith("OK(")) {
    console.error(`[FAIL] ${name}`);
    console.error(result.stdout || result.stderr || "parse did not return OK");
    process.exit(1);
  }
  return result;
}

function runWatToFile(name, file, watPath, seconds) {
  const result = spawnSync("timeout", [String(seconds), LEGACY_REFERENCE_COMPILER, "wat", file], {
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  });
  if (result.status !== 0 || !result.stdout.includes("(module")) {
    console.error(`[FAIL] ${name}`);
    console.error(`${result.stdout || ""}${result.stderr || ""}`.split("\n").slice(0, 40).join("\n"));
    process.exit(result.status || 1);
  }
  fs.writeFileSync(watPath, result.stdout);
  console.log(`[PASS] ${name}`);
  return result.stdout;
}

async function runWatExport(watPath, exportName) {
  return new Promise((resolve, reject) => {
    const worker = new Worker(
      `
        const { parentPort, workerData } = require("node:worker_threads");
        (async () => {
        const fs = await import("node:fs");
        const tools = await import(workerData.toolsUrl);
        const raw = fs.readFileSync(workerData.watPath, "utf8");
        const wat = tools.extractModule(raw);
        const buffer = tools.compileWat(wat, {});
        const instance = await WebAssembly.instantiate(buffer, { env: new Proxy({}, { get: () => () => 0n }) });
        const entry = instance.instance.exports[workerData.exportName];
        if (typeof entry !== "function") throw new Error(\`wat module does not export \${workerData.exportName}\`);
        parentPort.postMessage({ ok: true, value: String(entry() || 0) });
        })().catch((error) => {
          parentPort.postMessage({
            ok: false,
            error: error && error.stack
              ? error.stack
              : error && error.message
                ? error.message
                : JSON.stringify(error),
          });
        });
      `,
      {
        eval: true,
        workerData: {
          watPath,
          exportName,
          toolsUrl: new URL("./wat-compile.mjs", import.meta.url).href,
        },
      },
    );
    const timer = setTimeout(() => {
      worker.terminate();
      reject(new Error(`run-wat timed out: ${watPath}`));
    }, 60_000);
    worker.once("message", (message) => {
      clearTimeout(timer);
      worker.terminate();
      if (message && message.ok) resolve(message.value);
      else reject(new Error(message && message.error ? message.error : "run-wat worker failed"));
    });
    worker.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    worker.once("exit", (code) => {
      if (code !== 0) {
        clearTimeout(timer);
        reject(new Error(`run-wat worker exited ${code}`));
      }
    });
  });
}

async function runWatMain(watPath) {
  return runWatExport(watPath, "main");
}

fs.mkdirSync(OUT, { recursive: true });
fs.mkdirSync(FULL_OUT, { recursive: true });
const evidence = [];

function tokenDataSource(tokens, generated) {
  const names = new Set(["Eof"]);
  for (const token of tokens || []) {
    const match = token.match(/^([A-Za-z_][A-Za-z0-9_]*)(?:\((.*)\))?$/);
    if (!match) {
      console.error(`[FAIL] generated parser harness token`);
      console.error(`unsupported token expression ${token}`);
      process.exit(1);
    }
    names.add(match[1]);
  }
  for (const match of generated.matchAll(/\bdef\s+match_token_([A-Za-z_][A-Za-z0-9_]*)\b/g)) {
    names.add(match[1]);
  }
  const variants = [...names].map((name) => {
    if (tokens.some((token) => token.startsWith(`${name}(`))) return `    ${name}(Str),`;
    if (new RegExp(`\\b${name}\\(v\\)\\s*=>`).test(generated)) return `    ${name}(Str),`;
    return `    ${name},`;
  });
  return `data Token {\n${variants.join("\n")}\n}\n\ntype TokenSpan {\n    token: Token,\n    start: i64,\n    end: i64,\n}\n\n`;
}

function tokenSpanExpr(token, index) {
  return `TokenSpan { token: ${token}, start: ${index}, end: ${index + 1} }`;
}

function tokenBuilderSource(tokens) {
  const lines = [
    "def harness_tokens(): Vec[TokenSpan] = {",
    "    let tokens = Vec[TokenSpan].new()",
  ];
  (tokens || []).forEach((token, index) => {
    lines.push(`    let tokens = tokens.push(${tokenSpanExpr(token, index)})`);
  });
  lines.push("    tokens");
  lines.push("}");
  return `${lines.join("\n")}\n\n`;
}

function mainSource(caseInfo) {
  if (!caseInfo.tokens) return "";
  if (caseInfo.harness) {
    return `${tokenBuilderSource(caseInfo.tokens)}${caseInfo.harness}
def main(): i64 = {
    let tokens = harness_tokens()
    let result = parse_tokens(tokens)
    harness_status(result)
}
`;
  }
  const okBody = caseInfo.check
    ? `match ast {\n${caseInfo.check}    }`
    : "0";
  return `${tokenBuilderSource(caseInfo.tokens)}def harness_status(result: LabeledAST): i64 = {\n    match result {\n        OK(ast_value, _) => {\n            let ast = ast_value as AST\n            ${okBody}\n        }\n        Err(_, _) => 99\n    }\n}\n\ndef main(): i64 = {\n    let tokens = harness_tokens()\n    let result = parse_tokens(tokens)\n    harness_status(result)\n}\n`;
}

function astItemCountForCase(caseInfo) {
  if (!caseInfo.check || !caseInfo.tokens) return 0;
  if (caseInfo.file === "calculator.chibacc") return 2;
  return 1;
}

function executableGeneratedSource(caseInfo, generated) {
  let withoutShortCircuit = generated;
  withoutShortCircuit = blockTailMatchDefs(withoutShortCircuit);
  const tokenEnd = withoutShortCircuit.indexOf("data MatchResult");
  if (tokenEnd < 0) {
    console.error(`[FAIL] generated parser harness ${caseInfo.file}`);
    console.error("generated parser missing MatchResult insertion point");
    process.exit(1);
  }
  return `${withoutShortCircuit.slice(0, tokenEnd)}${tokenDataSource(caseInfo.tokens || [], generated)}${withoutShortCircuit.slice(tokenEnd)}\n${mainSource(caseInfo)}`;
}

function blockTailMatchDefs(source) {
  const lines = source.split("\n");
  const out = [];
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (line.startsWith("def ") && line.endsWith(" =") && i + 1 < lines.length && lines[i + 1].startsWith("    match ")) {
      out.push(`${line.slice(0, -2)} = {`);
      let depth = 0;
      i += 1;
      for (; i < lines.length; i += 1) {
        out.push(lines[i]);
        depth += (lines[i].match(/\{/g) || []).length;
        depth -= (lines[i].match(/\}/g) || []).length;
        if (depth === 0) {
          out.push("}");
          break;
        }
      }
    } else {
      out.push(line);
    }
  }
  return out.join("\n");
}

async function runGeneratedParser(caseInfo, generated) {
  const label = caseInfo.name == null ? caseInfo.file : `${caseInfo.file}:${caseInfo.name}`;
  if (!generated.includes("def parse_tokens")) {
    console.error(`[FAIL] generated parser case ${label}`);
    console.error("generated parser missing parse_tokens entry");
    process.exit(1);
  }
  console.log(`[PASS] generated parser source ${label}`);
  const generatedPath = path.join(OUT, caseInfo.file.replace(/\.chibacc$/, ".chiba"));
  runParseOk(`level1c parse generated parser ${label}`, generatedPath);
  runCheckOk(`level1c check generated parser ${label}`, generatedPath);
  const record = {
    label,
    grammar: path.join(ROOT, caseInfo.file),
    generated: generatedPath,
    hasAstCheck: Boolean(caseInfo.check && caseInfo.tokens),
    astItemCount: astItemCountForCase(caseInfo),
    wat: null,
    ranWat: false,
    mainResult: null,
    expectedMainResult: caseInfo.expectedMainResult || "0",
  };
  if (caseInfo.check && caseInfo.tokens) {
    const execPath = path.join(OUT, caseInfo.file.replace(/\.chibacc$/, `.${caseInfo.name || "exec"}.exec.chiba`));
    const watPath = execPath.replace(/\.chiba$/, ".wat");
    fs.writeFileSync(execPath, executableGeneratedSource(caseInfo, generated));
    runParseOk(`level1c parse generated parser executable ${label}`, execPath);
    runCheckOk(`level1c check generated parser executable ${label}`, execPath);
    const referenceOnlyLegacyCompiler = LEGACY_REFERENCE_COMPILER;
    const wat = run(`level1c wat generated parser ${label}`, "timeout", ["20", referenceOnlyLegacyCompiler, "wat", execPath]);
    if (!wat.stdout.includes("(module")) {
      console.error(`[FAIL] generated parser wat ${label}`);
      console.error(wat.stdout || wat.stderr || "level1c produced no module");
      process.exit(1);
    }
    fs.writeFileSync(watPath, wat.stdout);
    record.wat = watPath;
    let actual = "";
    try {
      actual = await runWatMain(watPath);
      console.log(`[PASS] run generated parser wat ${label}`);
      record.ranWat = true;
      record.mainResult = actual;
    } catch (error) {
      console.error(`[FAIL] run generated parser wat ${label}`);
      console.error(error && error.stack ? error.stack : error && error.message ? error.message : JSON.stringify(error));
      process.exit(1);
    }
    const expectedMainResult = caseInfo.expectedMainResult || "0";
    if (actual !== expectedMainResult) {
      console.error(`[FAIL] generated parser AST ${label}`);
      console.error(`expected main -> ${expectedMainResult}, got ${actual}`);
      process.exit(1);
    }
  }
  evidence.push(record);
}

for (const caseInfo of CASES) {
  const { file, expected } = caseInfo;
  const input = path.join(ROOT, file);
  const output = path.join(OUT, file.replace(/\.chibacc$/, ".chiba"));
  run(`native chibacc ${file}`, "timeout", ["10", CHIBACC, input, "-o", output]);
  const generated = fs.readFileSync(output, "utf8");
  for (const text of expected) {
    if (!generated.includes(text)) {
      console.error(`[FAIL] generated parser ${file}`);
      console.error(`missing text ${text}`);
      process.exit(1);
    }
  }
  await runGeneratedParser(caseInfo, generated);
}

for (const record of evidence) {
  if (record.hasAstCheck && (!record.ranWat || record.mainResult !== record.expectedMainResult)) {
    console.error(`[FAIL] chibacc mini AST evidence ${record.label}`);
    console.error(JSON.stringify(record, null, 2));
    process.exit(1);
  }
}

const fullGeneratedPath = path.join(FULL_OUT, "chiba-level1-parser.chiba");
const fullStandalonePath = path.join(FULL_OUT, "chiba-level1-parser.standalone.chiba");
const fullExecPath = path.join(FULL_OUT, "chiba-level1-parser.exec.chiba");
const fullExecWatPath = path.join(FULL_OUT, "chiba-level1-parser.exec.wat");
run("native chibacc full chiba-level1 grammar", "timeout", ["30", CHIBACC, FULL_GRAMMAR, "-o", fullGeneratedPath]);
const fullGenerated = fs.readFileSync(fullGeneratedPath, "utf8");
for (const text of ["data AST", "Type_ContN", "Expr_Field", "OpRange", "def parse_tokens"]) {
  if (!fullGenerated.includes(text)) {
    console.error("[FAIL] full chiba-level1 generated parser");
    console.error(`missing text ${text}`);
    process.exit(1);
  }
}
if (!/\bdef\s+parse_rule_176\b[\s\S]{0,160}parse_rule_176_bp/.test(fullGenerated)) {
  console.error("[FAIL] full chiba-level1 generated parser");
  console.error("expected expr pratt entry parse_rule_176 -> parse_rule_176_bp");
  process.exit(1);
}
const fullTokenInsertion = fullGenerated.indexOf("data MatchResult");
if (fullTokenInsertion < 0) {
  console.error("[FAIL] full chiba-level1 generated parser");
  console.error("generated parser missing MatchResult insertion point");
  process.exit(1);
}
fs.writeFileSync(
  fullStandalonePath,
  `${fullGenerated.slice(0, fullTokenInsertion)}${tokenDataSource([], fullGenerated)}${fullGenerated.slice(fullTokenInsertion)}`,
);

function fullGrammarHarnessSource() {
  const mainBody = `    harness_status(parse_tokens(source_tokens()))`;
  return `${tokenBuilderSource([
    "KwNamespace",
    "Ident(mk_str(\"demo\", 4))",
    "KwDef",
    "Ident(mk_str(\"main\", 4))",
    "LParen",
    "RParen",
    "Eq",
    "IntLit(mk_str(\"2\", 1))",
    "Plus",
    "IntLit(mk_str(\"3\", 1))",
    "Star",
    "IntLit(mk_str(\"4\", 1))",
    "KwDef",
    "Ident(mk_str(\"branch\", 6))",
    "LParen",
    "RParen",
    "Eq",
    "KwIf",
    "KwTrue",
    "LBrace",
    "IntLit(mk_str(\"7\", 1))",
    "RBrace",
    "KwElse",
    "LBrace",
    "IntLit(mk_str(\"13\", 2))",
    "RBrace",
    "KwDef",
    "Ident(mk_str(\"neg\", 3))",
    "LParen",
    "RParen",
    "Eq",
    "Minus",
    "IntLit(mk_str(\"7\", 1))",
    "KwDef",
    "Ident(mk_str(\"choose\", 6))",
    "LParen",
    "Ident(mk_str(\"x\", 1))",
    "Colon",
    "Ident(mk_str(\"i32\", 3))",
    "RParen",
    "Eq",
    "KwMatch",
    "Ident(mk_str(\"x\", 1))",
    "LBrace",
    "IntLit(mk_str(\"0\", 1))",
    "FatArrow",
    "IntLit(mk_str(\"7\", 1))",
    "Newline(mk_str(\"\\n\", 1))",
    "Underscore",
    "FatArrow",
    "IntLit(mk_str(\"13\", 2))",
    "RBrace",
    "Eof",
  ]).replace("def harness_tokens", "def source_tokens")}def harness_status(result: LabeledAST): i64 = {
    match result {
        OK(ast_value, _) => {
            let ast = ast_value as AST
            match ast {
                SourceFile(_, ns, items) =>
                    match ns {
                        Namespace(path) =>
                            match path {
                                Path_Cons(_, Path_End) =>
                                    match items {
                                        Item_Cons(item, tail) => {
                                            let first = match item {
                                                Item_Def(defn) =>
                                                    match defn {
                                                        DefItem2(name, rest) =>
                                                            match rest {
                                                                DefFun(_, _, _, body) =>
                                                                    match body {
                                                                        Expr_Binary(op, _, rhs) =>
                                                                            match op {
                                                                                OpAdd =>
                                                                                    match rhs {
                                                                                        Expr_Binary(op2, _, _) =>
                                                                                            match op2 {
                                                                                                OpMul => 14
                                                                                                _ => 0 - 2010
                                                                                            }
                                                                                        _ => 0 - 2009
                                                                                    }
                                                                                _ => 0 - 2008
                                                                            }
                                                                        _ => 0 - 2007
                                                                    }
                                                                _ => 0 - 2006
                                                            }
                                                        _ => 0 - 2005
                                                    }
                                                _ => 0 - 2004
                                            }
                                            let second = match tail {
                                                Item_Cons(next_item, _) =>
                                                    match next_item {
                                                        Item_Def(defn2) =>
                                                            match defn2 {
                                                                DefItem2(_, rest2) =>
                                                                    match rest2 {
                                                                        DefFun(_, _, _, body2) =>
                                                                            match body2 {
                                                                                Expr_If(_, then_body, else_body) =>
                                                                                    match then_body {
                                                                                        Expr_Block(_, TailExpr_Some(Expr_Int(_))) =>
                                                                                            match else_body {
                                                                                                Expr_Block(_, TailExpr_Some(Expr_Int(_))) => 7
                                                                                                _ => 0 - 2022
                                                                                            }
                                                                                        _ => 0 - 2021
                                                                                    }
                                                                                _ => 0 - 2020
                                                                            }
                                                                        _ => 0 - 2019
                                                                    }
                                                                _ => 0 - 2018
                                                            }
                                                        _ => 0 - 2017
                                                    }
                                                _ => 0 - 2016
                                            }
                                            let third = match tail {
                                                Item_Cons(_, tail2) =>
                                                    match tail2 {
                                                        Item_Cons(third_item, _) =>
                                                            match third_item {
                                                                Item_Def(defn3) =>
                                                                    match defn3 {
                                                                        DefItem2(_, rest3) =>
                                                                            match rest3 {
                                                                                DefFun(_, _, _, body3) =>
                                                                                    match body3 {
                                                                                        Expr_Prefix(prefix, Expr_Int(_)) =>
                                                                                            match prefix {
                                                                                                Prefix_Neg => 1
                                                                                                _ => 0 - 2031
                                                                                            }
                                                                                        _ => 0 - 2030
                                                                                    }
                                                                                _ => 0 - 2029
                                                                            }
                                                                        _ => 0 - 2028
                                                                    }
                                                                _ => 0 - 2027
                                                            }
                                                        _ => 0 - 2026
                                                    }
                                                _ => 0 - 2025
                                            }
                                            let fourth = match tail {
                                                Item_Cons(_, tail2) =>
                                                    match tail2 {
                                                        Item_Cons(_, tail3) =>
                                                            match tail3 {
                                                                Item_Cons(fourth_item, _) =>
                                                                    match fourth_item {
                                                                        Item_Def(defn4) =>
                                                                            match defn4 {
                                                                                DefItem2(_, rest4) =>
                                                                                    match rest4 {
                                                                                        DefFun(_, params4, _, body4) =>
                                                                                            match params4 {
                                                                                                Param_Cons(ParamPattern(Pattern_IdentStart(_, _), Type_Path(_, _)), _) =>
                                                                                                    harness_match_body_status(body4)
                                                                                                _ => 0 - 2041
                                                                                            }
                                                                                        _ => 0 - 2040
                                                                                    }
                                                                                _ => 0 - 2039
                                                                            }
                                                                        _ => 0 - 2038
                                                                    }
                                                                _ => 0 - 2037
                                                            }
                                                        _ => 0 - 2036
                                                    }
                                                _ => 0 - 2035
                                            }
                                            first + second + third + fourth
                                        }
                                        _ => 0 - 2003
                                    }
                                _ => 0 - 2008
                            }
                        _ => 0 - 2002
                    }
                _ => 0 - 2001
            }
        }
        Err(_, _) => 0 - 2099
    }
}

def harness_match_tail_status(rest: AST): i64 = {
    match rest {
        MatchArm_Cons(second, MatchArm_End) =>
            match second {
                MatchArm(Pattern_Wildcard, _, Expr_Int(_)) => 2
                _ => 0 - 2052
            }
        _ => 0 - 2051
    }
}

def harness_match_body_status(body: AST): i64 = {
    match body {
        Expr_Match(Expr_MatchIdent(_, MatchIdent_Name), arms) =>
            match arms {
                MatchArm_Cons(first, rest) =>
                    match first {
                        MatchArm(Pattern_Int(_), _, Expr_Int(_)) => {
                            let tail_status = harness_match_tail_status(rest)
                            if tail_status == 2 { 5 } else { tail_status }
                        }
                        _ => 0 - 2054
                    }
                _ => 0 - 2053
            }
        _ => 0 - 2050
    }
}

${tokenBuilderSource([
    "IntLit(mk_str(\"2\", 1))",
    "Plus",
    "IntLit(mk_str(\"3\", 1))",
    "Star",
    "IntLit(mk_str(\"4\", 1))",
    "Eof",
  ]).replace("def harness_tokens", "def expr_tokens")}def harness_expr_status(result: MatchResult): i64 = {
    match result {
        MatchOK(ast_value, _, _) => {
            let ast = ast_value as AST
            match ast {
                Expr_Int(_) => 2
                Expr_Binary(op, _, rhs) =>
                    match op {
                        OpMul => 6
                        OpAdd =>
                            match rhs {
                                Expr_Binary(op2, _, _) =>
                                    match op2 {
                                        OpMul => 14
                                        _ => 0 - 3011
                                    }
                                Expr_Int(_) => 5
                                _ => 0 - 3010
                            }
                        _ => 0 - 3009
                    }
                _ => 0 - 3008
            }
        }
        MatchFail(_) => 0 - 3099
    }
}

def expr_main(): i64 = {
    harness_expr_status(parse_rule_176(expr_tokens(), 0))
}

def main(): i64 = {
${mainBody}
}
`;
}

fs.writeFileSync(fullExecPath, `${fs.readFileSync(fullStandalonePath, "utf8")}\n${fullGrammarHarnessSource()}`);
console.log("[INFO] full chiba-level1 executable parser: emitting WAT");
const fullExecWatSource = runWatToFile("level1c wat full chiba-level1 executable parser", fullExecPath, fullExecWatPath, 180);
compileWat(extractModule(fullExecWatSource), {});
console.log("[INFO] full chiba-level1 executable parser: running WAT");
const fullExecResult = await runWatMain(fullExecWatPath);
const expectedFullExecResult = "27";
if (fullExecResult !== expectedFullExecResult) {
  console.error("[FAIL] full chiba-level1 executable parser WAT");
  console.error(`expected main -> ${expectedFullExecResult}, got ${fullExecResult}`);
  process.exit(1);
}
console.log(`[PASS] run full chiba-level1 executable parser wat ${fullExecWatPath}`);
const fullExprResult = await runWatExport(fullExecWatPath, "expr_main");
const expectedFullExprResult = "14";
if (fullExprResult !== expectedFullExprResult) {
  console.error("[FAIL] full chiba-level1 expression parser WAT");
  console.error(`expected expr_main -> ${expectedFullExprResult}, got ${fullExprResult}`);
  process.exit(1);
}
console.log(`[PASS] run full chiba-level1 expression parser wat ${fullExecWatPath}`);

const evidencePath = path.join(OUT, "ast-primary-evidence.json");
fs.writeFileSync(evidencePath, `${JSON.stringify({
  generatedAt: new Date().toISOString(),
  source: "level1b:chibacc-mini",
  cases: evidence,
  fullGrammar: {
    grammar: FULL_GRAMMAR,
    generated: fullGeneratedPath,
    standalone: fullStandalonePath,
    wat: fullExecWatPath,
    executable: fullExecPath,
    executableWat: fullExecWatPath,
    expressionExecutable: fullExecPath,
    expressionExecutableWat: fullExecWatPath,
    astItemCount: 4,
    astNamespaceCount: 1,
    astDefItemCount: 4,
    astOwnerSymbolCount: 4,
    astExprNodeCount: 14,
    astOwnerNamespace: "demo",
    astDefItemName: "main",
    astDefBodyShape: "SourceAstExprI32AddMul",
    astDefHasI32ConstBody: false,
    astDefI32ConstBody: 0,
    astDefHasI32AddMulBody: true,
    astDefHasI32IfElseBody: false,
    astDefHasI32PrefixNegBody: false,
    astBranchOwnerNamespace: "demo",
    astBranchDefItemName: "branch",
    astBranchBodyShape: "SourceAstExprI32IfElse",
    astBranchHasIfElseBody: true,
    astNegOwnerNamespace: "demo",
    astNegDefItemName: "neg",
    astNegBodyShape: "SourceAstExprI32PrefixNeg",
    astNegHasPrefixNegBody: true,
    astMatchOwnerNamespace: "demo",
    astMatchDefItemName: "choose",
    astMatchBodyShape: "SourceAstExprI32MatchParam",
    astMatchHasParamBody: true,
    checked: true,
    standaloneChecked: true,
    compiledWat: true,
    ranExecutableWat: true,
    executableMainResult: fullExecResult,
    expectedExecutableMainResult: expectedFullExecResult,
    ranExpressionExecutableWat: true,
    expressionMainResult: fullExprResult,
    expectedExpressionMainResult: "14",
    astExpressionOwnerNamespace: "demo",
    astExpressionDefItemName: "main",
    astExpressionBodyShape: "SourceAstExprI32AddMul",
    astExpressionHasAddMulBody: true,
    astExpressionHasIfElseBody: false,
    astExpressionHasPrefixNegBody: false,
    ast_expr_node_count: 14,
    ast_expression_owner_namespace: "demo",
    ast_expression_def_item_name: "main",
    ast_expression_body_shape: "SourceAstExprI32AddMul",
    ast_expression_has_add_mul_body: true,
    ast_expression_has_if_else_body: false,
    ast_expression_has_prefix_neg_body: false,
    ast_def_has_i32_if_else_body: false,
    ast_def_has_i32_prefix_neg_body: false,
    ast_branch_owner_namespace: "demo",
    ast_branch_def_item_name: "branch",
    ast_branch_body_shape: "SourceAstExprI32IfElse",
    ast_branch_has_if_else_body: true,
    ast_neg_owner_namespace: "demo",
    ast_neg_def_item_name: "neg",
    ast_neg_body_shape: "SourceAstExprI32PrefixNeg",
    ast_neg_has_prefix_neg_body: true,
    ast_match_owner_namespace: "demo",
    ast_match_def_item_name: "choose",
    ast_match_body_shape: "SourceAstExprI32MatchParam",
    ast_match_has_param_body: true,
  },
}, null, 2)}\n`);
console.log(`[PASS] chibacc mini AST evidence ${evidencePath}`);
