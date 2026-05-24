import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const ROOTS = [
  "level-1b/compiler/ir",
  "level-1b/compiler/control",
  "level-1b/compiler/closure",
];
const TODO = "TODO.md";

const FORBIDDEN = [
  /\bWasm\b/,
  /\bWasmGC\b/,
  /\bWasmGc\b/,
  /\bWAT\b/,
  /\bwat\b/,
  /\bBinaryen\b/,
  /\bfuncref\b/,
  /\beqref\b/,
  /\bstruct\.new\b/,
  /\barray\.new\b/,
  /\breturn_call\b/,
  /\bexternref\b/,
];

const ALLOWED_FILES = new Set([
  "level-1b/compiler/ir/core.chiba",
]);

function fail(message) {
  console.error("[FAIL] level-1b CIR boundary");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
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

function stripComments(source) {
  return source
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("///") && !line.trimStart().startsWith("//"))
    .join("\n");
}

const files = ROOTS.flatMap(listChiba).filter((file) => !ALLOWED_FILES.has(file));
const errors = [];
for (const file of files) {
  const code = stripComments(fs.readFileSync(file, "utf8"));
  for (const pattern of FORBIDDEN) {
    if (pattern.test(code)) errors.push(`${file}: CIR/control/closure layer mentions backend detail ${pattern}`);
  }
}

const todo = fs.readFileSync(TODO, "utf8");
if (!todo.includes("CIR/backend boundary gate：compiler/ir/control/closure 不允许出现 Wasm/WAT/Binaryen/funcref/eqref/backend opcode")) {
  errors.push("TODO.md must record completed CIR/backend boundary gate slice");
}

if (errors.length !== 0) fail(errors.join("\n"));
pass("CIR/control/closure stay backend-neutral");
