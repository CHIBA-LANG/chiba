import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";

const PROJECT = "level-1b";
const ENTRY = "level1b_main.chiba";
const SOURCE = path.join(PROJECT, "src", ENTRY);
const ARTIFACT_DIR = ".scratch/level-1b";
const WAT = path.join(ARTIFACT_DIR, "level1b-main.seed.wat");
const WASM = path.join(ARTIFACT_DIR, "level1b-main.seed.wasm");

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

const referenceOnlyLegacyCompiler = "./target/debug/level1c.o";
const emit = spawnSync("timeout", ["60", referenceOnlyLegacyCompiler, "wat", SOURCE], {
  encoding: "utf8",
  maxBuffer: 64 * 1024 * 1024,
});
if (emit.status !== 0) {
  fail("level-1b seed WAT emit", `${emit.stdout}${emit.stderr}`.split("\n").slice(0, 80).join("\n"));
}
fs.writeFileSync(WAT, emit.stdout);
const wat = emit.stdout;
if (!wat.includes("(module") || !wat.includes('(export "main"') || !wat.includes("call $compile_request_to_wat")) {
  fail("level-1b seed WAT emit", "level1b-main WAT must be emitted from the real level-1b compile entry");
}
if (wat.includes("source-primary-backend")) {
  fail("level-1b seed WAT emit", "level1b smoke must not reuse C11 source-primary fixture WAT");
}
pass(`level-1b seed WAT ${WAT}`);

const compile = spawnSync("timeout", ["60", process.execPath, "tools/node/compile-wat.mjs", WAT, "--output", WASM], {
  encoding: "utf8",
  maxBuffer: 64 * 1024 * 1024,
});
if (compile.status !== 0) {
  fail("level-1b seed WAT assemble", `${compile.stdout}${compile.stderr}`.split("\n").slice(0, 80).join("\n"));
}
pass(`level-1b seed WASM ${WASM}`);
