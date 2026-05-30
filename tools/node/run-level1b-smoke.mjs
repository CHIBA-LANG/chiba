import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { runWatPath } from "./run-wat.mjs";

const PROJECT = "level-1b";
const ENTRY = "level1b_main.chiba";
const SOURCE = path.join(PROJECT, "src", ENTRY);
const ARTIFACT_DIR = ".scratch/level-1b";
const WAT = path.join(ARTIFACT_DIR, "level1b-main.wat");
const SOURCE_PRIMARY_WAT = ".scratch/level-1b/c11-backend/source-primary-backend.wat";

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function fail(name, message) {
  console.error(`[FAIL] ${name}`);
  console.error(message);
  process.exit(1);
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

if (!fs.existsSync(SOURCE_PRIMARY_WAT)) {
  const result = spawnSync("timeout", ["60", "vp", "run", "level1b:c11-backend"], { encoding: "utf8" });
  if (result.status !== 0) {
    fail("level-1b backend smoke prerequisite", result.stdout || result.stderr || "level1b:c11-backend failed");
  }
}
if (!fs.existsSync(SOURCE_PRIMARY_WAT)) {
  fail("level-1b backend smoke prerequisite", `missing ${SOURCE_PRIMARY_WAT}`);
}
const wat = fs.readFileSync(SOURCE_PRIMARY_WAT, "utf8");
if (wat.includes('(func (export "main") (result i32) i32.const 0)')) {
  fail("level-1b smoke artifact", "level1b-main must not be a hand-written constant-zero shell");
}
if (!wat.includes('(export "main"') || !wat.includes("return_call") || !wat.includes("i32.add")) {
  fail("level-1b smoke artifact", "level1b-main must reuse executable C11 source-primary backend WAT");
}
fs.writeFileSync(WAT, wat);
const main = await runWatPath(WAT, "main");
if (main !== "7") {
  fail("level-1b smoke artifact", `expected main -> 7, got ${main}`);
}
pass(`level-1b smoke artifact ${WAT}`);
