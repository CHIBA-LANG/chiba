import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/supports/chibacc-mini";
const OUT = ".scratch/level-1b/chibacc-mini";
const CHIBACC = fs.existsSync("/tmp/chibacc-project-mainline/target/debug/chibacc.new.o")
  ? "/tmp/chibacc-project-mainline/target/debug/chibacc.new.o"
  : "./chibacc.o";
const CASES = [
  {
    file: "simple.chibacc",
    namespace: "chibaccmini.simple",
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
        Attr(name, args) =>
            if streq(name, mk_str("attribute", 9)) != 0 { 0 } else { 3 }
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

function runParseOk(name, file) {
  const result = run(name, "timeout", ["10", "./target/debug/level1c.o", "parse", file]);
  if (!result.stdout.startsWith("OK(")) {
    console.error(`[FAIL] ${name}`);
    console.error(result.stdout || result.stderr || "parse did not return OK");
    process.exit(1);
  }
  return result;
}

function runCheckOk(name, file) {
  const result = run(name, "timeout", ["10", "./target/debug/level1c.o", "check", file]);
  if (!result.stdout.includes("check ok")) {
    console.error(`[FAIL] ${name}`);
    console.error(result.stdout || result.stderr || "check did not report ok");
    process.exit(1);
  }
  return result;
}

fs.mkdirSync(OUT, { recursive: true });

function tokenDataSource(tokens) {
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
  const variants = [...names].map((name) => {
    if (tokens.some((token) => token.startsWith(`${name}(`))) return `    ${name}(Str),`;
    return `    ${name},`;
  });
  return `data Token {\n${variants.join("\n")}\n}\n\ntype TokenSpan {\n    token: Token,\n    start: i64,\n    end: i64,\n}\n\n`;
}

function tokenSpanExpr(token, index) {
  return `TokenSpan { token: ${token}, start: ${index}, end: ${index + 1} }`;
}

function tokenBuilderSource(tokens) {
  const lines = [
    "def harness_tokens(): Vec = {",
    "    let tokens = vec_new()",
  ];
  (tokens || []).forEach((token, index) => {
    lines.push(`    let _ = vec_push(tokens, ${tokenSpanExpr(token, index)})`);
  });
  lines.push("    tokens");
  lines.push("}");
  return `${lines.join("\n")}\n\n`;
}

function mainSource(caseInfo) {
  if (!caseInfo.tokens) return "";
  return `${tokenBuilderSource(caseInfo.tokens)}def harness_status(result: LabeledAST): i64 = {\n    match result {\n        OK(_, _) => 0\n        Err(_, _) => 99\n    }\n}\n\ndef main(): i64 = {\n    let tokens = harness_tokens()\n    let result = parse_tokens(tokens)\n    harness_status(result)\n}\n`;
}

function executableGeneratedSource(caseInfo, generated) {
  let withoutShortCircuit = generated;
  withoutShortCircuit = blockTailMatchDefs(withoutShortCircuit);
  const astEnd = withoutShortCircuit.indexOf("data MatchResult");
  if (astEnd < 0) {
    console.error(`[FAIL] generated parser harness ${caseInfo.file}`);
    console.error("generated parser missing MatchResult insertion point");
    process.exit(1);
  }
  return `${withoutShortCircuit.slice(0, astEnd)}${tokenDataSource(caseInfo.tokens || [])}${withoutShortCircuit.slice(astEnd)}\n${mainSource(caseInfo)}`;
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

function runGeneratedParser(caseInfo, generated) {
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
  if (caseInfo.check && caseInfo.tokens) {
    const execPath = path.join(OUT, caseInfo.file.replace(/\.chibacc$/, `.${caseInfo.name || "exec"}.exec.chiba`));
    const watPath = execPath.replace(/\.chiba$/, ".wat");
    fs.writeFileSync(execPath, executableGeneratedSource(caseInfo, generated));
    const wat = run(`level1c wat generated parser ${label}`, "timeout", ["20", "./target/debug/level1c.o", "wat", execPath]);
    if (!wat.stdout.includes("(module")) {
      console.error(`[FAIL] generated parser wat ${label}`);
      console.error(wat.stdout || wat.stderr || "level1c produced no module");
      process.exit(1);
    }
    fs.writeFileSync(watPath, wat.stdout);
    const executed = run(`run generated parser wat ${label}`, "timeout", ["20", "node", "tools/node/run-wat.mjs", watPath, "--invoke", "main"]);
    const actual = executed.stdout.trim().split(/\s+/).pop();
    if (actual !== "0") {
      console.error(`[FAIL] generated parser AST ${label}`);
      console.error(`expected main -> 0, got ${actual}`);
      process.exit(1);
    }
  }
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
  runGeneratedParser(caseInfo, generated);
}
