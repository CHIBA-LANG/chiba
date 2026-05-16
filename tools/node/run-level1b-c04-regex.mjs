import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";
import { compileWat } from "./wat-compile.mjs";

const REGEX_ROOT = "level-1b/compiler/regex";
const GOLDEN = "level-1b/supports/regex/regex-golden.json";
const WAT = "level-1b/tests/wasmtime/regex-c04-smoke.wat";
const WASM = ".scratch/level-1b/regex-c04-smoke.wasm";
const REQUIRED_FILES = ["ast.chiba", "matcher.chiba", "parser.chiba", "program.chiba", "utf8.chiba"];
const REQUIRED_TEXT = [
  "data RegexAst",
  "data RegexClassAtom",
  "data RegexAssertion",
  "data RegexInstruction",
  "type RegexProgram",
  "type RegexMatch",
  "def RegexProgram.longest_at",
  "def str.next_char_offset",
];

function fail(message) {
  console.error("[FAIL] level-1b C04 regex");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
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
  const rel = path.relative(REGEX_ROOT, file);
  const code = stripLineComments(source);
  const lines = source.split(/\n/);
  const errors = [];
  if (source.includes("#![Metal]")) errors.push(`${rel}: regex must be ordinary Chiba, not Metal`);
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|load(?:8|16|32|64)\s*\(|store(?:8|16|32|64)\s*\(/.test(code)) {
    errors.push(`${rel}: regex leaks old Metal/raw-memory style`);
  }
  if (/\bOP_[A-Z0-9_]+\b/.test(code)) errors.push(`${rel}: regex uses numeric opcode constants`);
  if (/\b(ptr|pointer|addr|raw)\w*\s*:\s*i64\b/i.test(code)) errors.push(`${rel}: regex uses opaque i64 pointer field`);
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${rel}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function checkGolden() {
  const cases = JSON.parse(read(GOLDEN));
  const required = new Set(["literal", "class", "repeat", "capture", "lookahead", "lookbehind", "lookaround-negative", "utf8-boundary", "longest"]);
  for (const test of cases) {
    required.delete(test.name);
    const re = new RegExp(test.pattern, "u");
    const match = test.input.match(re);
    const actual = match ? match[0] : "";
    if (actual !== test.expected) {
      fail(`regex golden failed: ${test.name}: expected ${test.expected}, got ${actual}`);
    }
  }
  if (required.size !== 0) fail(`missing regex golden cases: ${[...required].join(", ")}`);
  pass("regex golden oracle");
}

function runWasmtimeSmoke() {
  fs.mkdirSync(path.dirname(WASM), { recursive: true });
  const wasm = compileWat(read(WAT));
  fs.writeFileSync(WASM, wasm);
  const run = spawnSync("wasmtime", [WASM], { encoding: "utf8" });
  if (run.status !== 0) fail(run.stdout || run.stderr);
  if (!run.stdout.includes("level1b regex c04 ok")) fail(`unexpected wasmtime output: ${run.stdout}`);
  pass("regex wasmtime smoke");
}

function main() {
  const files = listChiba(REGEX_ROOT);
  const seen = new Set(files.map((file) => path.basename(file)));
  const missing = REQUIRED_FILES.filter((file) => !seen.has(file));
  if (missing.length !== 0) fail(`missing regex source files:\n${missing.join("\n")}`);

  const joined = files.map(read).join("\n");
  const missingText = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missingText.length !== 0) fail(`missing regex source text:\n${missingText.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("regex source contract");

  checkGolden();
  runWasmtimeSmoke();
}

main();
