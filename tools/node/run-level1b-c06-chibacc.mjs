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
  "GrammarTypeCont1",
  "GrammarTypeContN",
  "type PrattTable",
  "data RecoveryAction",
  "def lower_chibacc",
  "def retry_alternative",
  "def parse_pratt_at",
  "def generate_parser",
  "reset",
  "shift retry",
];

function fail(message) {
  console.error("[FAIL] level-1b C06 chibacc");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function primaryPathBlocked(name) {
  console.log(`[BLOCKED] ${name}`);
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
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${rel}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function checkMiniSpecs() {
  const required = new Set(["simple.chibacc", "pratt.chibacc", "list.chibacc", "continuation-type.chibacc"]);
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

  primaryPathBlocked("chibacc continuation smoke parse requires level-1b parser execution");
  primaryPathBlocked("chibacc codegen contract requires level-1b WAT emission");

  checkMiniSpecs();

  const mini = run("vp", ["run", "level1b:chibacc-mini"]);
  if (mini.status !== 0) fail(mini.stdout || mini.stderr);
  pass("chibacc mini oracle");
}

main();
