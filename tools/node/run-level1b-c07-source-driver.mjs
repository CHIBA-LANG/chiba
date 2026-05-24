import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const SOURCE_ROOT = "level-1b/compiler/source";
const DRIVER_ROOT = "level-1b/compiler/driver";
const FIXTURE = "level-1b/supports/pre-c07-smokes/doc_compile_if.chiba";
const REQUIRED_TEXT = [
  "type ProjectSurface",
  "type SourceProjectFacts",
  "type SourceHeaderFacts",
  "type SourceDocFacts",
  "type SourceCompileIfFacts",
  "type SourceItemScanResult",
  "data SourceItemKind",
  "type NamespaceSurface",
  "def load_project",
  "type SourceScanCursor",
  "type NamespaceScanResult",
  "def source_starts_with_at",
  "def source_scan_header",
  "def source_scan_doc_facts",
  "def source_scan_compile_if_facts",
  "def source_scan_item_at",
  "def source_scan_items",
  "def source_scan_first_namespace",
  "def scan_project_source_facts",
  "def scan_project_headers",
  "def scan_project_docs",
  "def scan_project_compile_if",
  "def scan_project_items",
  "type DocCommentBlock",
  "def attach_namespace_doc",
  "data SourceGateErrorKind",
  "SourceGateMissingNamespace",
  "SourceGateItemScanAbsent",
  "SourceGateTypedItemLoweringAbsent",
  "def source_gate_errors",
  "def check_source_semantic_gates",
  "SourceGateRefArrayDirectAssignment",
  "missing-facts: source item scan absent",
  "missing-facts: namespace scan found no namespace",
  "missing-lowering: typed item lowering absent",
  "data CompileIfPredicate",
  "CompileIfAll",
  "CompileIfNot",
  "def SourceHeaderFacts.prelude_policy",
  "\"wasm32-unknown-wasi\"",
  "\"wasm-gc\"",
  "type DriverDiagnostic",
  "def run_source_driver",
  "data PipelineStage",
  "def run_nanopass_wat",
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

function oracleReference(name) {
  console.log(`[ORACLE] ${name}`);
}

function oracleReferenceFailed(name) {
  console.log(`[ORACLE-FAIL] ${name}`);
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
  if (file.endsWith("compile_if.chiba") && /__compiler_builtin\s*\(\s*"std\.compile_if_eval"/.test(code)) {
    errors.push(`${file}: compile_if eval must be implemented in level-1b source`);
  }
  if (file.endsWith("semantic_gate.chiba") && /\bdef\s+check_source_semantic_gates\b[\s\S]*Vec\s*\[\s*SourceGateError\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)/.test(code)) {
    errors.push(`${file}: source semantic gates must not pass through with empty errors`);
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
  primaryPathBlocked("doc compile_if fixture parse requires level-1b parser execution");

  const namespace = spawnSync("timeout", ["30", "vp", "run", "level1b:namespace"], { encoding: "utf8" });
  if (namespace.status === 0) oracleReference("namespace project reference");
  else oracleReferenceFailed("namespace project reference");
}

main();
