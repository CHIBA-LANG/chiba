import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const PROJECT = "level-1b";
const ENTRY = "level1b_main.chiba";
const SOURCE = path.join(PROJECT, "src", ENTRY);
const ARTIFACT_DIR = ".scratch/level-1b";
const WAT = path.join(ARTIFACT_DIR, "level1b-main.wat");

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function fail(name, message) {
  console.error(`[FAIL] ${name}`);
  console.error(message);
  process.exit(1);
}

function primaryPathBlocked(name) {
  console.log(`[BLOCKED] ${name}`);
}

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

if (!fs.existsSync(SOURCE)) fail("level-1b smoke source", `missing ${SOURCE}`);
const source = fs.readFileSync(SOURCE, "utf8");
if (/def\s+main\s*\(\s*\)\s*:\s*i64\s*=\s*42\b/.test(source)) {
  fail("level-1b smoke source", "seed main must not be a magic-number placeholder");
}
if (!source.includes("cli_ready_for_codegen") || !source.includes("cli_default_config")) {
  fail("level-1b smoke source", "seed main must be wired to the CLI readiness contract");
}
pass("level-1b smoke source exists");
primaryPathBlocked("level-1b seed compile requires level-1b primary compiler execution");
primaryPathBlocked("level-1b WAT smoke requires level-1b WAT emission");
console.log(`[BLOCKED] level-1b smoke artifact ${WAT}`);
