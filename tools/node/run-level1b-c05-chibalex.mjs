import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/std/chibalex";
const CONT = "level-1b/supports/chibalex-continuation/backtracking.chiba";
const MINI_ROOT = "level-1b/supports/chibalex-mini";
const REQUIRED_FILES = ["ast.chiba", "codegen.chiba", "engine.chiba", "ir.chiba", "parser.chiba"];
const REQUIRED_TEXT = [
  "type ChibalexSpec",
  "type LexMode",
  "data LexBuiltinKeyword",
  "type LexRule",
  "LexKeywordCont1",
  "LexKeywordContN",
  "LexKeywordShiftn",
  "data LexUtf8Policy",
  "type LexIdentifierPolicy",
  "LexUtf8Required",
  "LexUtf8RejectInvalid",
  "type LoweredLexer",
  "def lower_chibalex",
  "def chibalex_parse_namespace",
  "def parse_regex_def",
  "def parse_rule",
  "def find_best_rule",
  "def LexState.advance_char",
  "next_char_offset",
  "data LexerCodegenStatus",
  "LexerCodegenComplete",
  "def generated_lexer_complete",
  "def generate_lexer",
  "reset",
  "shift retry",
];

function fail(message) {
  console.error("[FAIL] level-1b C05 chibalex");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function referenceGate(name) {
  console.log(`[REF] ${name}`);
}

function referenceGateFailed(name) {
  console.log(`[REF-FAIL] ${name}`);
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

function stripLineComments(source) {
  return source
    .split(/\n/)
    .filter((line) => !line.trimStart().startsWith("///") && !line.trimStart().startsWith("//"))
    .join("\n");
}

function checkSource(file, source) {
  const rel = path.relative(ROOT, file);
  const code = stripLineComments(source);
  const lines = source.split(/\n/);
  const errors = [];
  if (source.includes("#![Metal]")) errors.push(`${rel}: chibalex must be ordinary Chiba`);
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|load(?:8|16|32|64)\s*\(|store(?:8|16|32|64)\s*\(|heap_alloc\s*\(/.test(code)) {
    errors.push(`${rel}: chibalex leaks old Metal/raw-memory style`);
  }
  if (/\b(ptr|pointer|addr|raw)\w*\s*:\s*i64\b/i.test(code)) errors.push(`${rel}: chibalex uses opaque i64 pointer field`);
  if (/missing-lowering: executable chibalex lexer codegen absent|LexerCodegenContractOnly/.test(code)) {
    errors.push(`${rel}: chibalex codegen must not report contract-only success`);
  }
  if (/__compiler_builtin\(\"std\.chibalex_parse/.test(code)) {
    errors.push(`${rel}: chibalex parser must not call std.chibalex_parse compiler builtins`);
  }
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${rel}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function checkMiniSpecs() {
  const specs = fs.readdirSync(MINI_ROOT).filter((name) => name.endsWith(".chibalex")).sort();
  const required = new Set(["basic.chibalex", "longest.chibalex", "string-mode.chibalex"]);
  required.add("continuation-surface.chibalex");
  required.add("utf8-ident.chibalex");
  required.add("attribute-tokens.chibalex");
  for (const spec of specs) {
    required.delete(spec);
    const source = read(path.join(MINI_ROOT, spec));
    if (!source.includes("tokens")) fail(`${spec}: missing tokens section`);
    if (spec === "longest.chibalex" && !source.includes("\"==\"") && !source.includes("\"=\"")) {
      fail("longest fixture must contain overlapping tokens");
    }
    if (spec === "string-mode.chibalex" && !source.includes("mode STRING")) {
      fail("string-mode fixture must contain STRING mode");
    }
    if (spec === "continuation-surface.chibalex") {
      for (const needle of ["\"cont1\"", "\"contN\"", "\"shiftn\"", "\"->\"", "KwCont1", "KwContN", "KwShiftn", "ThinArrow"]) {
        if (!source.includes(needle)) fail(`continuation-surface fixture missing ${needle}`);
      }
    }
    if (spec === "utf8-ident.chibalex") {
      for (const needle of ["$XID_START", "$XID_CONTINUE", "Ident(Str)"]) {
        if (!source.includes(needle)) fail(`utf8-ident fixture missing ${needle}`);
      }
    }
    if (spec === "attribute-tokens.chibalex") {
      for (const needle of ["\"#\"", "\"[\"", "\"]\"", "\"(\"", "\")\"", "\"{\"", "\"}\"", "Ident(Str)", "StringLit(Str)", "IntLit(Str)", "KwTrue", "KwFalse"]) {
        if (!source.includes(needle)) fail(`attribute-tokens fixture missing ${needle}`);
      }
      if (/Attribute(Token|Lit|Start)/.test(source)) fail("attribute-tokens fixture must not collapse attributes into a legacy token");
    }
  }
  if (required.size !== 0) fail(`missing chibalex mini specs: ${[...required].join(", ")}`);
  pass("chibalex mini spec shape");
}

function run(command, args) {
  return spawnSync(command, args, { encoding: "utf8" });
}

function main() {
  for (const file of listChiba(ROOT)) {
    if (/\bcli\b/i.test(file) || /\bmain\.chiba$/i.test(file)) {
      fail(`std.chibalex must not contain CLI entry source: ${file}`);
    }
  }

  const files = listChiba(ROOT);
  const seen = new Set(files.map((file) => path.basename(file)));
  const missing = REQUIRED_FILES.filter((file) => !seen.has(file));
  if (missing.length !== 0) fail(`missing chibalex source files:\n${missing.join("\n")}`);

  const joined = `${files.map(read).join("\n")}\n${read(CONT)}`;
  const missingText = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missingText.length !== 0) fail(`missing chibalex source text:\n${missingText.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("chibalex source contract");

  const contSource = read(CONT);
  for (const needle of ["reset", "shift retry", "retry(candidate)"]) {
    if (!contSource.includes(needle)) fail(`chibalex continuation smoke missing ${needle}`);
  }
  pass("chibalex continuation smoke source");

  checkMiniSpecs();

  const mini = run("timeout", ["30", "vp", "run", "level1b:chibalex-mini"]);
  if (mini.status === 0) referenceGate("chibalex mini reference");
  else referenceGateFailed("chibalex mini reference");
}

main();
