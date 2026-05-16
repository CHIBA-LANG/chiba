import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/compiler/semantic";
const REQUIRED_FILES = [
  "abi_capability.chiba",
  "alpha.chiba",
  "driver.chiba",
  "method_operator.chiba",
  "pattern.chiba",
  "template.chiba",
  "types.chiba",
];
const REQUIRED_TEXT = [
  "def alpha_convert",
  "def elaborate_patterns",
  "def unify",
  "def infer_types",
  "data TemplateObligation",
  "def check_template",
  "type MethodKey",
  "type OperatorKey",
  "def build_method_operator_index",
  "data ExternAbi",
  "data CapabilityUse",
  "def check_extern_abi",
  "def check_capabilities",
  "def run_typed_semantics",
  "stable_sort",
];

function fail(message) {
  console.error("[FAIL] level-1b C08 semantic");
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
  if (/\beffect\b/i.test(code)) errors.push(`${file}: semantic C08 must not introduce effect naming`);
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|heap_alloc\s*\(|load(?:8|16|32|64)\s*\(/.test(code)) {
    errors.push(`${file}: semantic pass leaks Metal/raw memory implementation`);
  }
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${file}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function runGate(label, script, timeoutSeconds) {
  const result = spawnSync("timeout", [String(timeoutSeconds), "vp", "run", script], {
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
  });
  if (result.status !== 0) fail(`${label}\n${result.stdout || ""}${result.stderr || ""}`);
  pass(label);
}

function main() {
  const files = listChiba(ROOT);
  const seen = new Set(files.map((file) => path.basename(file)));
  const missing = REQUIRED_FILES.filter((file) => !seen.has(file));
  if (missing.length !== 0) fail(`missing semantic files:\n${missing.join("\n")}`);

  const joined = files.map(read).join("\n");
  const missingText = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missingText.length !== 0) fail(`missing semantic contract text:\n${missingText.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("semantic source contract");

  runGate("type-system oracle", "level1b:type-system", 60);
  runGate("semantic gates oracle", "semantic:gates", 60);
  runGate("capability oracle", "level1b:capability", 30);
}

main();
