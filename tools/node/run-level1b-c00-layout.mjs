import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const ROOT = "level-1b";

const REQUIRED_DIRS = [
  "metalstd",
  "std",
  "prelude",
  "compiler",
  "compiler/cli",
  "compiler/source",
  "compiler/diagnostic",
  "compiler/driver",
  "tools",
  "tests",
];

const REQUIRED_READMES = [
  "README.md",
  ...REQUIRED_DIRS.map((dir) => `${dir}/README.md`),
];

function fail(message) {
  console.error(`[FAIL] ${message}`);
  process.exit(1);
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

for (const dir of REQUIRED_DIRS) {
  const full = path.join(ROOT, dir);
  if (!fs.statSync(full, { throwIfNoEntry: false })?.isDirectory()) {
    fail(`missing level-1b directory: ${full}`);
  }
}

for (const file of REQUIRED_READMES) {
  const full = path.join(ROOT, file);
  if (!fs.statSync(full, { throwIfNoEntry: false })?.isFile()) {
    fail(`missing level-1b contract file: ${full}`);
  }
}

const rootReadme = read(path.join(ROOT, "README.md"));
const todo = read("TODO.md");
const longterm = read("TODO.longterm.md");

const rootNeedles = [
  "wasmtime chibac.wasm --",
  "Node runners and Binaryen wrappers are developer and CI conveniences only",
  "Linear memory is not the ordinary Chiba heap",
  "`///` doc comments",
  "Block comments are not used",
];

for (const needle of rootNeedles) {
  if (!rootReadme.includes(needle)) {
    fail(`level-1b README missing contract text: ${needle}`);
  }
}

const todoNeedles = [
  "chibac.wasm",
  "wasmtime chibac.wasm --",
  "Wasm-GC managed object",
  "Node runner",
];

for (const needle of todoNeedles) {
  if (!todo.includes(needle)) {
    fail(`TODO.md missing C00 runtime contract text: ${needle}`);
  }
}

if (todo.includes("## Long-term Pipeline")) {
  fail("TODO.md still contains the long-term pipeline section");
}

if (!longterm.includes("## Long-term Pipeline")) {
  fail("TODO.longterm.md does not contain the long-term pipeline section");
}

console.log("[PASS] level-1b C00 layout contract");
