import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const SPEC_DIR = "chiba-level1-grammar-spec";

function listSpecs() {
  return fs
    .readdirSync(SPEC_DIR)
    .filter((name) => name.endsWith("-test.chiba"))
    .sort()
    .map((name) => path.join(SPEC_DIR, name));
}

function run(command, args) {
  return spawnSync(command, args, { encoding: "utf8" });
}

let failed = 0;
let checked = 0;

for (const file of listSpecs()) {
  checked += 1;
  const native = run("./target/debug/lexer_spec_runner.o", [file]);
  const level1c = run("./target/debug/level1c.o", ["lex", file]);
  if (native.status !== 0 || level1c.status !== 0 || native.stdout !== level1c.stdout) {
    failed += 1;
    console.error(`[DIFF] ${file}`);
    if (native.status !== 0) {
      console.error(`  lexer_spec_runner exit=${native.status}`);
    }
    if (level1c.status !== 0) {
      console.error(`  level1c lex exit=${level1c.status}`);
    }
  }
}

if (failed === 0) {
  console.log(`[PASS] lexer compare ${checked} specs`);
} else {
  console.error(`[FAIL] lexer compare ${failed}/${checked} specs`);
  process.exit(1);
}
