import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/std/chibacc";
const CONT = "level-1b/supports/chibacc-continuation/alternative_recovery.chiba";
const MINI_ROOT = "level-1b/supports/chibacc-mini";
const REQUIRED_FILES = ["ast.chiba", "codegen.chiba", "engine.chiba", "ir.chiba", "parser.chiba"];
const REQUIRED_TEXT = [
  "type ChibaccSpec",
  "data GrammarRuleBody",
  "data GrammarTypeSurface",
  "data AttrArg",
  "type AttrField",
  "type Attr",
  "AttrNamed",
  "AttrCall",
  "AttrList",
  "AttrObject",
  "GrammarTypeCont1",
  "GrammarTypeContN",
  "type PrattTable",
  "data RecoveryAction",
  "def chibacc_parse_namespace",
  "def chibacc_parse_start_name",
  "def chibacc_parse_rule_name",
  "def lower_chibacc",
  "def retry_alternative",
  "def parse_pratt_at",
  "data ParserCodegenStatus",
  "ParserCodegenComplete",
  "def GeneratedParser.complete",
  "def generate_parser",
  "reset",
  "shiftn retry",
];

function fail(message) {
  console.error("[FAIL] level-1b C06 chibacc");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function oracleReference(name) {
  console.log(`[ORACLE] ${name}`);
}

function oracleReferenceFailed(name) {
  console.log(`[ORACLE-FAIL] ${name}`);
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function listChiba(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listChiba(file));
    else if (entry.isFile() && entry.name.endsWith(".chiba")) out.push(file);
  }
  return out.sort();
}

function stripLineComments(source) {
  return source
    .split(/\n/)
    .filter((line) => !line.trimStart().startsWith("///") && !line.trimStart().startsWith("//"))
    .join("\n");
}

function previousDocBlock(lines, index) {
  const docs = [];
  let cursor = index - 1;
  while (cursor >= 0 && lines[cursor].trim() === "") cursor -= 1;
  while (cursor >= 0 && lines[cursor].trimStart().startsWith("///")) {
    docs.push(lines[cursor].trimStart());
    cursor -= 1;
  }
  return docs.reverse().join("\n");
}

function isPublicItem(line) {
  return /^(namespace|type|data|def)\b/.test(line.trimStart());
}

function checkSource(file, source) {
  const rel = path.relative(ROOT, file);
  const code = stripLineComments(source);
  const lines = source.split(/\n/);
  const errors = [];
  if (source.includes("#![Metal]")) errors.push(`${rel}: chibacc must be ordinary Chiba`);
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|load(?:8|16|32|64)\s*\(|store(?:8|16|32|64)\s*\(|heap_alloc\s*\(/.test(code)) {
    errors.push(`${rel}: chibacc leaks old Metal/raw-memory style`);
  }
  if (/\b(ptr|pointer|addr|raw)\w*\s*:\s*i64\b/i.test(code)) errors.push(`${rel}: chibacc uses opaque i64 pointer field`);
  if (/missing-lowering: executable chibacc parser codegen absent|ParserCodegenContractOnly/.test(code)) {
    errors.push(`${rel}: chibacc codegen must not report contract-only success`);
  }
  if (/__compiler_builtin\(\"std\.chibacc_parse/.test(code)) {
    errors.push(`${rel}: chibacc parser must not call std.chibacc_parse compiler builtins`);
  }
  if (rel === "engine.chiba" && /\bshift\s+retry\b/.test(code)) {
    errors.push(`${rel}: parser retry must use multi-shot shiftn retry`);
  }
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${rel}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function checkMiniSpecs() {
  const required = new Set(["simple.chibacc", "pratt.chibacc", "list.chibacc", "continuation-type.chibacc", "attribute-args.chibacc"]);
  for (const name of fs.readdirSync(MINI_ROOT).filter((file) => file.endsWith(".chibacc"))) {
    required.delete(name);
    const source = read(path.join(MINI_ROOT, name));
    if (!source.includes("start ")) fail(`${name}: missing start rule`);
    if (name === "pratt.chibacc" && !source.includes("pratt")) fail("pratt fixture must contain pratt rule");
    if (name === "list.chibacc" && !source.includes("names_tail")) fail("list fixture must contain recursive list rule");
    if (name === "continuation-type.chibacc") {
      for (const needle of ["KwCont1", "KwContN", "ThinArrow", "Type_Cont1", "Type_ContN", "Type_Callable"]) {
        if (!source.includes(needle)) fail(`continuation-type fixture missing ${needle}`);
      }
    }
    if (name === "attribute-args.chibacc") {
      for (const needle of ["rule attr_arg", "AttrNamed", "AttrCall", "AttrList", "AttrObject", "AttrBare"]) {
        if (!source.includes(needle)) fail(`attribute-args fixture missing ${needle}`);
      }
    }
  }
  if (required.size !== 0) fail(`missing chibacc mini specs: ${[...required].join(", ")}`);
  pass("chibacc mini spec shape");
}

function run(command, args) {
  return spawnSync(command, args, { encoding: "utf8" });
}

function main() {
  for (const file of listChiba(ROOT)) {
    if (/\bcli\b/i.test(file) || /\bmain\.chiba$/i.test(file)) {
      fail(`std.chibacc must not contain CLI entry source: ${file}`);
    }
  }

  const files = listChiba(ROOT);
  const seen = new Set(files.map((file) => path.basename(file)));
  const missing = REQUIRED_FILES.filter((file) => !seen.has(file));
  if (missing.length !== 0) fail(`missing chibacc source files:\n${missing.join("\n")}`);

  const joined = `${files.map(read).join("\n")}\n${read(CONT)}`;
  const missingText = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missingText.length !== 0) fail(`missing chibacc source text:\n${missingText.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("chibacc source contract");

  const contSource = read(CONT);
  for (const needle of ["reset", "shiftn retry", "retry(candidate)"]) {
    if (!contSource.includes(needle)) fail(`chibacc continuation smoke missing ${needle}`);
  }
  pass("chibacc continuation smoke source");
  if (!read("level-1b/supports/chibacc-mini/codegen_contract.chiba").includes("generate_parser(mk_parser()).text()")) {
    fail("chibacc codegen contract fixture must keep executable main");
  }
  pass("chibacc codegen contract fixture");

  checkMiniSpecs();

  const mini = run("vp", ["run", "level1b:chibacc-mini"]);
  if (mini.status === 0) oracleReference("chibacc mini reference");
  else oracleReferenceFailed("chibacc mini reference");
}

main();
