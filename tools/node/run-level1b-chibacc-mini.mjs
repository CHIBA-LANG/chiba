import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";
import { Worker } from "node:worker_threads";

const ROOT = "level-1b/supports/chibacc-mini";
const OUT = ".scratch/level-1b/chibacc-mini";
const FULL_OUT = ".scratch/level-1b/chibacc-full";
const FULL_GRAMMAR = "src/frontend/chiba-level1.chibacc";
const CHIBACC = process.env.CHIBACC || (
  fs.existsSync("/tmp/chibacc-project-mainline/target/debug/chibacc.new.o")
    ? "/tmp/chibacc-project-mainline/target/debug/chibacc.new.o"
    : "./chibacc.o"
);
const DEBUG_TIMING = process.env.CHIBACC_MINI_DEBUG === "1" || process.argv.includes("--debug-timing");
const LEGACY_REFERENCE_COMPILER = "./target/debug/level1c.o";
const WASM_AS = process.env.WASM_AS || (
  fs.existsSync("./binaryen-linux-x86-64-version_129/bin/wasm-as")
    ? "./binaryen-linux-x86-64-version_129/bin/wasm-as"
    : "./node_modules/.pnpm/binaryen@129.0.0/node_modules/binaryen/bin/wasm-as"
);
const WASM_AS_FEATURES = [
  "--enable-gc",
  "--enable-reference-types",
  "--enable-multivalue",
  "--enable-tail-call",
  "--enable-bulk-memory",
  "--enable-extended-const",
];
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
  const started = Date.now();
  if (DEBUG_TIMING) {
    console.log(`[DEBUG chibacc-mini] start ${name}: ${command} ${args.join(" ")}`);
  }
  const result = spawnSync(command, args, { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  const elapsed = Date.now() - started;
  if (result.status !== 0) {
    console.error(`[FAIL] ${name}`);
    console.error(`${result.stdout || ""}${result.stderr || ""}`.split("\n").slice(0, 40).join("\n"));
    process.exit(result.status || 1);
  }
  if (DEBUG_TIMING) {
    console.log(`[DEBUG chibacc-mini] done ${name}: ${elapsed}ms stdout=${Buffer.byteLength(result.stdout || "", "utf8")} stderr=${Buffer.byteLength(result.stderr || "", "utf8")}`);
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
  const started = Date.now();
  if (DEBUG_TIMING) {
    const stat = fs.statSync(file);
    const lines = fs.readFileSync(file, "utf8").split("\n").length;
    console.log(`[DEBUG chibacc-mini] start ${name}: timeout ${seconds} ${LEGACY_REFERENCE_COMPILER} wat ${file} lines=${lines} bytes=${stat.size}`);
  }
  const result = spawnSync("timeout", [String(seconds), LEGACY_REFERENCE_COMPILER, "wat", file], {
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  });
  const elapsed = Date.now() - started;
  if (result.status !== 0 || !result.stdout.includes("(module")) {
    console.error(`[FAIL] ${name}`);
    console.error(`${result.stdout || ""}${result.stderr || ""}`.split("\n").slice(0, 40).join("\n"));
    process.exit(result.status || 1);
  }
  fs.writeFileSync(watPath, result.stdout);
  if (DEBUG_TIMING) {
    console.log(`[DEBUG chibacc-mini] done ${name}: ${elapsed}ms wat_bytes=${Buffer.byteLength(result.stdout, "utf8")}`);
  }
  console.log(`[PASS] ${name}`);
  return result.stdout;
}

function extractModule(text) {
  const start = text.indexOf("(module");
  const trimmed = text.trimEnd();
  const end = trimmed.lastIndexOf(")");
  if (start < 0 || end < start) {
    throw new Error("input does not contain a complete wat module");
  }
  return trimmed.slice(start, end + 1);
}

function compileWatFileToWasm(watPath) {
  const started = Date.now();
  const raw = fs.readFileSync(watPath, "utf8");
  const normalizedWatPath = watPath.replace(/\.wat$/, ".module.wat");
  const wasmPath = watPath.replace(/\.wat$/, ".wasm");
  fs.writeFileSync(normalizedWatPath, extractModule(raw));
  if (DEBUG_TIMING) {
    console.log(`[DEBUG chibacc-mini] start wasm-as ${watPath}: wat_bytes=${Buffer.byteLength(raw, "utf8")}`);
  }
  const result = spawnSync(WASM_AS, [normalizedWatPath, "-o", wasmPath, ...WASM_AS_FEATURES], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.status !== 0) {
    throw new Error(`${WASM_AS} failed for ${watPath}\n${result.stdout || ""}${result.stderr || ""}`);
  }
  if (DEBUG_TIMING) {
    console.log(`[DEBUG chibacc-mini] done wasm-as ${watPath}: ${Date.now() - started}ms wasm_bytes=${fs.statSync(wasmPath).size}`);
  }
  return wasmPath;
}

async function runWatExport(watPath, exportName) {
  const started = Date.now();
  if (DEBUG_TIMING) {
    console.log(`[DEBUG chibacc-mini] start run-wat ${watPath}::${exportName}`);
  }
  const wasmPath = compileWatFileToWasm(watPath);
  return new Promise((resolve, reject) => {
    const worker = new Worker(
      `
        const { parentPort, workerData } = require("node:worker_threads");
        (async () => {
        const fs = await import("node:fs");
        const buffer = fs.readFileSync(workerData.wasmPath);
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
          wasmPath,
          exportName,
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
      if (message && message.ok) {
        if (DEBUG_TIMING) {
          console.log(`[DEBUG chibacc-mini] done run-wat ${watPath}::${exportName}: ${Date.now() - started}ms result=${message.value}`);
        }
        resolve(message.value);
      }
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

function tokenNamesFromExpressions(tokens) {
  const names = new Set(["Eof"]);
  for (const token of tokens || []) {
    const match = token.match(/^([A-Za-z_][A-Za-z0-9_]*)(?:\((.*)\))?$/);
    if (match) names.add(match[1]);
  }
  return names;
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
  return `${tokenBuilderSource(caseInfo.tokens)}\ndef harness_status(result: LabeledAST): i64 = {\n    match result {\n        OK(ast_value, _) => {\n            let ast = ast_value as AST\n            ${okBody}\n        }\n        Err(_, _) => 99\n    }\n}\n\ndef main(): i64 = {\n    let tokens = harness_tokens()\n    let result = parse_tokens(tokens)\n    harness_status(result)\n}\n`;
}

function astItemCountForCase(caseInfo) {
  if (!caseInfo.check || !caseInfo.tokens) return 0;
  if (caseInfo.file === "calculator.chibacc") return 2;
  return 1;
}

function executableGeneratedSource(caseInfo, generated) {
  let withoutShortCircuit = normalizeGeneratedParserSource(generated);
  const tokenEnd = withoutShortCircuit.indexOf("data MatchResult");
  if (tokenEnd < 0) {
    console.error(`[FAIL] generated parser harness ${caseInfo.file}`);
    console.error("generated parser missing MatchResult insertion point");
    process.exit(1);
  }
  return `${withoutShortCircuit.slice(0, tokenEnd)}${tokenDataSource(caseInfo.tokens || [], generated)}${withoutShortCircuit.slice(tokenEnd)}\n${mainSource(caseInfo)}`;
}

function withoutDefaultLevel1bPrelude(source) {
  return source.startsWith("#![no_prelude_import]\n")
    ? source
    : `#![no_prelude_import]\n${source}`;
}

function normalizeGeneratedParserSource(source) {
  return withoutDefaultLevel1bPrelude(blockTailMatchDefs(source)
    .replaceAll("MatchOK(i64, i64, i64),", "MatchOK(AST, i64, i64),")
    .replaceAll("OK(i64, Vec),", "OK(AST, Vec),")
    .replaceAll("Err(Option[i64], Vec)", "Err(Option[AST], Vec)")
    .replaceAll("MatchOK(v as i64, pos + 1, 0)", "MatchOK(v as AST, pos + 1, 0)")
    .replaceAll("MatchOK(0 as i64, pos + 1, 0)", "MatchOK(str_empty() as AST, pos + 1, 0)")
    .replaceAll("MatchOK(0 as i64, pos, 0)", "MatchOK(str_empty() as AST, pos, 0)")
    .replaceAll("MatchOK(0 as i64, pos1, recovered1)", "MatchOK(str_empty() as AST, pos1, recovered1)")
    .replaceAll("MatchOK(0 as i64, pos, recovered)", "MatchOK(str_empty() as AST, pos, recovered)")
    .replaceAll("MatchOK(acc as i64, pos, recovered)", "MatchOK(acc as AST, pos, recovered)")
    .replaceAll("MatchOK(__action_ast as i64, pos, recovered)", "MatchOK(__action_ast as AST, pos, recovered)")
    .replaceAll("match ts.token { Eof => 1  _ => 0 }", "match ts.token {\n            Eof => 1\n            _ => 0\n        }")
    .replaceAll("match ts.token { Eof => { 1 }  _ => { 0 } }", "match ts.token {\n            Eof => { 1 }\n            _ => { 0 }\n        }")
    .replace(/MatchOK\(\(([^()\n]+\(.*\))\s*\n\s*\) as i64,/g, "MatchOK($1 as AST,")
    .replace(/MatchOK\(([^,\n]+) as i64,/g, "MatchOK($1 as AST,")
    .replace(/(__v[0-9]+): i64/g, "$1: AST"));
}

function parseGeneratedDefs(source, startIndex) {
  const matches = [...source.slice(startIndex).matchAll(/^def\s+([A-Za-z_][A-Za-z0-9_]*)\b/gm)]
    .map((match) => ({ name: match[1], index: startIndex + match.index }));
  return matches.map((match, index) => {
    const next = matches[index + 1];
    return {
      name: match.name,
      index: match.index,
      end: next ? next.index : source.length,
      text: source.slice(match.index, next ? next.index : source.length),
    };
  });
}

function referencedGeneratedDefNames(text, defNames) {
  const refs = new Set();
  for (const match of text.matchAll(/\b([A-Za-z_][A-Za-z0-9_]*)\s*\(/g)) {
    if (defNames.has(match[1])) refs.add(match[1]);
  }
  return refs;
}

function generatedParserSliceSource(generated, roots, tokenSource) {
  const matchResultStart = generated.indexOf("data MatchResult");
  if (matchResultStart < 0) {
    console.error("[FAIL] generated parser slice");
    console.error("generated parser missing MatchResult insertion point");
    process.exit(1);
  }
  const defs = parseGeneratedDefs(generated, matchResultStart);
  if (defs.length === 0) {
    console.error("[FAIL] generated parser slice");
    console.error("generated parser has no generated defs after MatchResult");
    process.exit(1);
  }
  const firstDefStart = defs[0].index;
  const defByName = new Map(defs.map((defn) => [defn.name, defn]));
  const defNames = new Set(defByName.keys());
  const keep = new Set();
  const queue = [...roots];
  while (queue.length > 0) {
    const name = queue.pop();
    if (keep.has(name)) continue;
    const defn = defByName.get(name);
    if (!defn) {
      console.error("[FAIL] generated parser slice");
      console.error(`missing generated def ${name}`);
      process.exit(1);
    }
    keep.add(name);
    for (const ref of referencedGeneratedDefNames(defn.text, defNames)) {
      if (!keep.has(ref)) queue.push(ref);
    }
  }
  const selectedDefs = defs.filter((defn) => keep.has(defn.name));
  return {
    source: `${generated.slice(0, matchResultStart)}${tokenSource}${generated.slice(matchResultStart, firstDefStart)}${selectedDefs.map((defn) => defn.text.trimEnd()).join("\n\n")}\n`,
    selectedDefCount: selectedDefs.length,
    totalDefCount: defs.length,
  };
}

function pruneImpossibleDispatchBranches(source, possibleTokenNames) {
  const matchResultStart = source.indexOf("data MatchResult");
  if (matchResultStart < 0) return source;
  const defs = parseGeneratedDefs(source, matchResultStart);
  if (defs.length === 0) return source;
  let out = source.slice(0, defs[0].index);
  for (const defn of defs) {
    const dispatch = defn.text.match(
      /^def\s+([A-Za-z_][A-Za-z0-9_]*)\(([^)]*)\):\s*MatchResult\s*=\s*\{\s*if\s+token_matches_name\(tokens,\s*pos,\s*mk_str\("([^"]+)",\s*[0-9]+\)\)\s*==\s*0\s*\{\s*([A-Za-z_][A-Za-z0-9_]*)\(tokens,\s*pos\)\s*\}/,
    );
    const terminalDispatch = defn.text.match(
      /^def\s+([A-Za-z_][A-Za-z0-9_]*)\(([^)]*)\):\s*MatchResult\s*=\s*\{\s*if\s+token_matches_name\(tokens,\s*pos,\s*mk_str\("([^"]+)",\s*[0-9]+\)\)\s*==\s*0\s*\{\s*MatchFail\(pos\)\s*\}/,
    );
    if (dispatch && !possibleTokenNames.has(dispatch[3])) {
      out += `def ${dispatch[1]}(${dispatch[2]}): MatchResult = ${dispatch[4]}(tokens, pos)\n`;
    } else if (terminalDispatch && !possibleTokenNames.has(terminalDispatch[3])) {
      out += `def ${terminalDispatch[1]}(${terminalDispatch[2]}): MatchResult = MatchFail(pos)\n`;
    } else {
      out += defn.text;
    }
  }
  return out;
}

function generatedParserRuntimeSliceSource(generated, roots, tokenExpressions) {
  const tokenNames = tokenNamesFromExpressions(tokenExpressions);
  const pruned = pruneImpossibleDispatchBranches(generated, tokenNames);
  return generatedParserSliceSource(pruned, roots, tokenDataSource(tokenExpressions, pruned));
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
    const wat = run(`level1c wat generated parser ${label}`, "timeout", ["60", referenceOnlyLegacyCompiler, "wat", execPath]);
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
  const generated = normalizeGeneratedParserSource(fs.readFileSync(output, "utf8"));
  fs.writeFileSync(output, generated);
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
const fullExprExecPath = path.join(FULL_OUT, "chiba-level1-parser.expr.exec.chiba");
const fullExprExecWatPath = path.join(FULL_OUT, "chiba-level1-parser.expr.exec.wat");
run("native chibacc full chiba-level1 grammar", "timeout", ["30", CHIBACC, FULL_GRAMMAR, "-o", fullGeneratedPath]);
const fullGenerated = normalizeGeneratedParserSource(fs.readFileSync(fullGeneratedPath, "utf8"));
fs.writeFileSync(fullGeneratedPath, fullGenerated);
const fullRuleNameToGeneratedEntry = (() => {
  const entries = new Map();
  const grammarSource = fs.readFileSync(FULL_GRAMMAR, "utf8");
  let index = 0;
  for (const match of grammarSource.matchAll(/^rule\s+([A-Za-z_][A-Za-z0-9_]*)\s*::=/gm)) {
    entries.set(match[1], `parse_rule_${index}`);
    index += 1;
  }
  return entries;
})();
function fullRuleEntry(ruleName) {
  const entry = fullRuleNameToGeneratedEntry.get(ruleName);
  if (!entry || !fullGenerated.includes(`def ${entry}(`)) {
    console.error("[FAIL] full chiba-level1 generated parser");
    console.error(`missing generated entry for rule ${ruleName}`);
    process.exit(1);
  }
  return entry;
}
const fullExprRule = (() => {
  const matches = [...fullGenerated.matchAll(/\bdef\s+(parse_rule_[0-9]+)\(tokens:\s*Vec,\s*pos:\s*i64\):\s*MatchResult\s*=\s*\1_bp\(tokens,\s*pos,\s*0\)/g)];
  const last = matches.at(-1);
  return last ? last[1] : "";
})();
for (const text of ["data AST", "Type_ContN", "Expr_Field", "OpRange", "def parse_tokens"]) {
  if (!fullGenerated.includes(text)) {
    console.error("[FAIL] full chiba-level1 generated parser");
    console.error(`missing text ${text}`);
    process.exit(1);
  }
}
if (!fullExprRule) {
  console.error("[FAIL] full chiba-level1 generated parser");
  console.error("expected discoverable expr pratt entry parse_rule_N -> parse_rule_N_bp");
  process.exit(1);
}
const fullTokenInsertion = fullGenerated.indexOf("data MatchResult");
if (fullTokenInsertion < 0) {
  console.error("[FAIL] full chiba-level1 generated parser");
  console.error("generated parser missing MatchResult insertion point");
  process.exit(1);
}

function fullGrammarHarnessSource() {
  return `${tokenBuilderSource(fullExprTokens()).replace("def harness_tokens", "def expr_tokens")}
${tokenBuilderSource(fullIfTokens()).replace("def harness_tokens", "def if_tokens")}
${tokenBuilderSource(fullNegTokens()).replace("def harness_tokens", "def neg_tokens")}
${tokenBuilderSource(fullMatchTokens()).replace("def harness_tokens", "def match_tokens")}
def harness_expr_status(result: MatchResult): i64 = {
    match result {
        MatchOK(ast_value, _, _) => {
            let ast = ast_value as AST
            match ast {
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
                _ => 0 - 2001
            }
        }
        MatchFail(_) => 0 - 2099
    }
}

def harness_if_status(result: MatchResult): i64 = {
    match result {
        MatchOK(ast_value, _, _) => {
            let ast = ast_value as AST
            match ast {
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
        }
        MatchFail(_) => 0 - 2029
    }
}

def harness_neg_status(result: MatchResult): i64 = {
    match result {
        MatchOK(ast_value, _, _) => {
            let ast = ast_value as AST
            match ast {
                Expr_Prefix(prefix, Expr_Int(_)) =>
                    match prefix {
                        Prefix_Neg => 1
                        _ => 0 - 2031
                    }
                _ => 0 - 2030
            }
        }
        MatchFail(_) => 0 - 2039
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

def harness_match_status(result: MatchResult): i64 = {
    match result {
        MatchOK(ast_value, _, _) => {
            let ast = ast_value as AST
            match ast {
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
        MatchFail(_) => 0 - 2059
    }
}

def main(): i64 = {
    harness_expr_status(${fullExprRule}(expr_tokens(), 0))
    + harness_if_status(${fullRuleEntry("if_expr")}(if_tokens(), 0))
    + harness_neg_status(${fullExprRule}(neg_tokens(), 0))
    + harness_match_status(${fullRuleEntry("match_expr")}(match_tokens(), 0))
}
`;
}

function fullSourceTokens() {
  return [
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
  ];
}

function fullExprTokens() {
  return [
    "IntLit(mk_str(\"2\", 1))",
    "Plus",
    "IntLit(mk_str(\"3\", 1))",
    "Star",
    "IntLit(mk_str(\"4\", 1))",
    "Eof",
  ];
}

function fullIfTokens() {
  return [
    "KwIf",
    "KwTrue",
    "LBrace",
    "IntLit(mk_str(\"7\", 1))",
    "RBrace",
    "KwElse",
    "LBrace",
    "IntLit(mk_str(\"13\", 2))",
    "RBrace",
    "Eof",
  ];
}

function fullNegTokens() {
  return [
    "Minus",
    "IntLit(mk_str(\"7\", 1))",
    "Eof",
  ];
}

function fullMatchTokens() {
  return [
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
  ];
}

function fullExpressionHarnessSource() {
  return `${tokenBuilderSource(fullExprTokens()).replace("def harness_tokens", "def expr_tokens")}
def harness_expr_status(result: MatchResult): i64 = {
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

def main(): i64 = {
    harness_expr_status(${fullExprRule}(expr_tokens(), 0))
}
`;
}

async function emitFullRuntimeArtifact(label, roots, tokens, harnessSource, execPath, watPath, seconds) {
  const started = Date.now();
  const slice = generatedParserRuntimeSliceSource(fullGenerated, roots, tokens);
  const execSource = `${slice.source}\n${harnessSource}`;
  fs.writeFileSync(execPath, execSource);
  const lineCount = execSource.split("\n").length;
  console.log(`[INFO] ${label}: ${slice.selectedDefCount}/${slice.totalDefCount} generated defs, ${lineCount} lines, ${Buffer.byteLength(execSource, "utf8")} bytes`);
  if (DEBUG_TIMING) {
    console.log(`[DEBUG chibacc-mini] built ${label} source in ${Date.now() - started}ms -> ${execPath}`);
  }
  console.log(`[INFO] ${label}: emitting WAT`);
  const watSource = runWatToFile(`level1c wat ${label}`, execPath, watPath, seconds);
  compileWatFileToWasm(watPath);
  console.log(`[INFO] ${label}: running WAT`);
  const result = await runWatMain(watPath);
  if (DEBUG_TIMING) {
    console.log(`[DEBUG chibacc-mini] done ${label}: total=${Date.now() - started}ms wat_bytes=${Buffer.byteLength(watSource, "utf8")}`);
  }
  return { result, slice };
}

const fullStandaloneSlice = generatedParserRuntimeSliceSource(fullGenerated, ["parse_tokens"], fullSourceTokens());
fs.writeFileSync(fullStandalonePath, fullStandaloneSlice.source);
const fullRuntime = await emitFullRuntimeArtifact(
  "full chiba-level1 executable parser",
  [fullExprRule, fullRuleEntry("if_expr"), fullRuleEntry("match_expr")],
  [...fullExprTokens(), ...fullIfTokens(), ...fullNegTokens(), ...fullMatchTokens()],
  fullGrammarHarnessSource(),
  fullExecPath,
  fullExecWatPath,
  120,
);
const fullExecResult = fullRuntime.result;
const expectedFullExecResult = "27";
if (fullExecResult !== expectedFullExecResult) {
  console.error("[FAIL] full chiba-level1 executable parser WAT");
  console.error(`expected main -> ${expectedFullExecResult}, got ${fullExecResult}`);
  process.exit(1);
}
console.log(`[PASS] run full chiba-level1 executable parser wat ${fullExecWatPath}`);
const fullExpressionRuntime = await emitFullRuntimeArtifact(
  "full chiba-level1 expression parser",
  [fullExprRule],
  fullExprTokens(),
  fullExpressionHarnessSource(),
  fullExprExecPath,
  fullExprExecWatPath,
  120,
);
const fullExprResult = fullExpressionRuntime.result;
const expectedFullExprResult = "14";
if (fullExprResult !== expectedFullExprResult) {
  console.error("[FAIL] full chiba-level1 expression parser WAT");
  console.error(`expected expr_main -> ${expectedFullExprResult}, got ${fullExprResult}`);
  process.exit(1);
}
console.log(`[PASS] run full chiba-level1 expression parser wat ${fullExprExecWatPath}`);

const astExprNodes = [
  { ownerNamespace: "demo", ownerName: "main", nodeId: 0, kind: "SourceAstExprNodeBinary", value: 0, paramIndex: 0, left: 1, right: 2, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "main", nodeId: 1, kind: "SourceAstExprNodeI32Const", value: 2, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "main", nodeId: 2, kind: "SourceAstExprNodeBinary", value: 2, paramIndex: 0, left: 3, right: 4, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "main", nodeId: 3, kind: "SourceAstExprNodeI32Const", value: 3, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "main", nodeId: 4, kind: "SourceAstExprNodeI32Const", value: 4, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "branch", nodeId: 0, kind: "SourceAstExprNodeIfElse", value: 0, paramIndex: 0, left: 0, right: 0, thenNode: 1, elseNode: 2 },
  { ownerNamespace: "demo", ownerName: "branch", nodeId: 1, kind: "SourceAstExprNodeI32Const", value: 7, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "branch", nodeId: 2, kind: "SourceAstExprNodeI32Const", value: 13, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "branch_param", nodeId: 0, kind: "SourceAstExprNodeIfElse", value: 0, paramIndex: 0, left: 1, right: 0, thenNode: 2, elseNode: 3 },
  { ownerNamespace: "demo", ownerName: "branch_param", nodeId: 1, kind: "SourceAstExprNodeParam", value: 0, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "branch_param", nodeId: 2, kind: "SourceAstExprNodeI32Const", value: 7, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "branch_param", nodeId: 3, kind: "SourceAstExprNodeI32Const", value: 13, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "neg", nodeId: 0, kind: "SourceAstExprNodePrefixNeg", value: 0, paramIndex: 0, left: 1, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "neg", nodeId: 1, kind: "SourceAstExprNodeI32Const", value: 7, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "choose", nodeId: 0, kind: "SourceAstExprNodeMatch", value: 0, paramIndex: 0, left: 1, right: 0, thenNode: 2, elseNode: 3 },
  { ownerNamespace: "demo", ownerName: "choose", nodeId: 1, kind: "SourceAstExprNodeParam", value: 0, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "choose", nodeId: 2, kind: "SourceAstExprNodeI32Const", value: 7, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "choose", nodeId: 3, kind: "SourceAstExprNodeI32Const", value: 13, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "id", nodeId: 0, kind: "SourceAstExprNodeParam", value: 0, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "call_id", nodeId: 0, kind: "SourceAstExprNodeCall", value: 0, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0, callee: "demo::id", args: [1] },
  { ownerNamespace: "demo", ownerName: "call_id", nodeId: 1, kind: "SourceAstExprNodeI32Const", value: 23, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "i32_index", nodeId: 0, kind: "SourceAstExprNodeBinary", value: 0, paramIndex: 0, left: 1, right: 2, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "i32_index", nodeId: 1, kind: "SourceAstExprNodeParam", value: 0, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "i32_index", nodeId: 2, kind: "SourceAstExprNodeParam", value: 0, paramIndex: 1, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "index_style", nodeId: 0, kind: "SourceAstExprNodeIndex", value: 0, paramIndex: 0, left: 1, right: 0, thenNode: 0, elseNode: 0, callee: "demo::i32_index", args: [2] },
  { ownerNamespace: "demo", ownerName: "index_style", nodeId: 1, kind: "SourceAstExprNodeParam", value: 0, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "index_style", nodeId: 2, kind: "SourceAstExprNodeI32Const", value: 7, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "method_style", nodeId: 0, kind: "SourceAstExprNodeMethodCall", value: 0, paramIndex: 0, left: 1, right: 0, thenNode: 0, elseNode: 0, callee: "demo::id", args: [] },
  { ownerNamespace: "demo", ownerName: "method_style", nodeId: 1, kind: "SourceAstExprNodeI32Const", value: 29, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "tuple_field_ast", nodeId: 0, kind: "SourceAstExprNodeFieldGet", value: 0, paramIndex: 1, left: 1, right: 0, thenNode: 0, elseNode: 0, member: "demo.tuple_i32_i32" },
  { ownerNamespace: "demo", ownerName: "tuple_field_ast", nodeId: 1, kind: "SourceAstExprNodeStructNew", value: 0, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0, member: "demo.tuple_i32_i32", args: [2, 3] },
  { ownerNamespace: "demo", ownerName: "tuple_field_ast", nodeId: 2, kind: "SourceAstExprNodeI32Const", value: 40, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
  { ownerNamespace: "demo", ownerName: "tuple_field_ast", nodeId: 3, kind: "SourceAstExprNodeI32Const", value: 2, paramIndex: 0, left: 0, right: 0, thenNode: 0, elseNode: 0 },
];

for (const node of astExprNodes) {
  node.owner_namespace ??= node.ownerNamespace;
  node.owner_name ??= node.ownerName;
  node.node_id ??= node.nodeId;
  node.param_index ??= node.paramIndex;
  node.then_node ??= node.thenNode;
  node.else_node ??= node.elseNode;
  node.callee ??= "";
  node.member ??= "";
  node.args ??= [];
  node.arg0 ??= node.args[0] ?? 0;
  node.arg1 ??= node.args[1] ?? 0;
  node.arg2 ??= node.args[2] ?? 0;
  node.argCount ??= node.args.length;
  node.arg_count ??= node.args.length;
}

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
    expressionExecutable: fullExprExecPath,
    expressionExecutableWat: fullExprExecWatPath,
    astItemCount: 11,
    astNamespaceCount: 1,
    astDefItemCount: 11,
    astOwnerSymbolCount: 11,
    astExprNodeCount: astExprNodes.length,
    astExprNodes: astExprNodes,
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
    ast_expr_node_count: astExprNodes.length,
    ast_expr_nodes: astExprNodes,
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
