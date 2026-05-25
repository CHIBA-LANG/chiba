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
if (!source.includes("compile_request_to_wat") || !source.includes("CompileRequest")) {
  fail("level-1b smoke source", "seed main must call the level-1b nanopass compile entry");
}
pass("level-1b smoke source exists");
const pipeline = fs.readFileSync(path.join(PROJECT, "compiler", "driver", "nanopass_pipeline.chiba"), "utf8");
if (!pipeline.includes("def compile_request_to_wat") || !pipeline.includes("run_nanopass_wat")) {
  fail("level-1b compile entry", "nanopass pipeline must expose compile_request_to_wat over run_nanopass_wat");
}
pass("level-1b compile entry wired");
primaryPathBlocked("level-1b seed compile requires generated frontend/runtime execution");
primaryPathBlocked("level-1b WAT smoke requires level-1b WAT emission");
console.log(`[BLOCKED] level-1b smoke artifact ${WAT}`);
