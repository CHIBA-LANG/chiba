import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const SOURCE_ROOT = "level-1b/compiler/source";
const DRIVER_ROOT = "level-1b/compiler/driver";
const FIXTURE = "level-1b/supports/pre-c07-smokes/doc_compile_if.chiba";
const REQUIRED_TEXT = [
  "type ProjectSurface",
  "type NamespaceSurface",
  "def load_project",
  "type DocCommentBlock",
  "def attach_namespace_doc",
  "data SourceGateErrorKind",
  "def check_source_semantic_gates",
  "SourceGateRefArrayDirectAssignment",
  "data CompileIfPredicate",
  "CompileIfAll",
  "CompileIfNot",
  "\"wasm32-unknown-wasi\"",
  "\"wasm-gc\"",
  "type DriverDiagnostic",
  "def run_source_driver",
  "stable_sort",
];

function fail(message) {
  console.error("[FAIL] level-1b C07 source driver");
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
  while (cursor >= 0 && lines[cursor].trimStart().startsWith("#[")) {
    cursor -= 1;
    while (cursor >= 0 && lines[cursor].trim() === "") cursor -= 1;
  }
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
  const code = stripLineComments(source);
  const lines = source.split(/\n/);
  const errors = [];
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|heap_alloc\s*\(|load(?:8|16|32|64)\s*\(/.test(code)) {
    errors.push(`${file}: source driver leaks Metal/raw memory`);
  }
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${file}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function main() {
  const files = [...listChiba(SOURCE_ROOT), ...listChiba(DRIVER_ROOT)];
  const joined = files.map(read).join("\n");
  const missing = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missing.length !== 0) fail(`missing source driver contract text:\n${missing.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("source driver contract");

  const fixture = read(FIXTURE);
  for (const needle of [
    "/// Documented C07 namespace.",
    "#[doc(path=\"docs/c07.md\")]",
    "namespace pre_c07.doc_compile_if",
    "backend=\"wasm-gc\"",
    "target=\"wasm32-unknown-wasi\"",
    "not(backend=\"wasm-gc\")",
  ]) {
    if (!fixture.includes(needle)) fail(`C07 fixture missing ${needle}`);
  }
  const parsed = spawnSync("./target/debug/level1c.o", ["parse", FIXTURE], { encoding: "utf8" });
  if (parsed.status !== 0 || !parsed.stdout.startsWith("OK(")) {
    fail(`C07 doc/compile_if fixture does not parse\n${parsed.stdout || parsed.stderr}`);
  }
  pass("doc compile_if fixture parse");

  const namespace = spawnSync("timeout", ["30", "vp", "run", "level1b:namespace"], { encoding: "utf8" });
  if (namespace.status !== 0) fail(namespace.stdout || namespace.stderr);
  pass("namespace project oracle");
}

main();
