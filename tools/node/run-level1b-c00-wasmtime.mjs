import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";
import { compileWat } from "./wat-compile.mjs";

const WAT = "level-1b/tests/wasmtime/chibac-help-smoke.wat";
const OUT_DIR = ".scratch/level-1b";
const WASM = path.join(OUT_DIR, "chibac-help-smoke.wasm");

function run(name, command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    ...options,
  });
  if (result.status !== 0) {
    console.error(`[FAIL] ${name}`);
    console.error(`${result.stdout || ""}${result.stderr || ""}`.split("\n").slice(0, 40).join("\n"));
    process.exit(result.status || 1);
  }
  console.log(`[PASS] ${name}`);
  return result;
}

function hasCommand(command) {
  return spawnSync("sh", ["-lc", `command -v ${command}`], { encoding: "utf8" }).status === 0;
}

const wat = await fs.readFile(WAT, "utf8");
await fs.mkdir(OUT_DIR, { recursive: true });
await fs.writeFile(WASM, compileWat(wat));
console.log(`[PASS] compile C00 wasmtime smoke ${WASM}`);

const nodeRun = run("C00 node WASI smoke", process.execPath, [
  "--no-warnings",
  "tools/node/run-wat.mjs",
  WAT,
]);

if (!nodeRun.stdout.includes("chibac level-1b") || !nodeRun.stdout.includes("--backend wasm-gc") || !nodeRun.stdout.includes("-O3") || !nodeRun.stdout.includes("--diagnostic-width")) {
  console.error("[FAIL] C00 node WASI smoke output");
  console.error(nodeRun.stdout);
  process.exit(1);
}

if (!hasCommand("wasmtime")) {
  console.log("[SKIP] C00 wasmtime direct smoke: wasmtime not found in PATH");
  process.exit(0);
}

const wasmtime = run("C00 wasmtime direct smoke", "timeout", [
  "10",
  "wasmtime",
  WASM,
  "--",
  "--help",
]);

if (!wasmtime.stdout.includes("chibac level-1b") || !wasmtime.stdout.includes("--target wasm32-unknown-wasi") || !wasmtime.stdout.includes("-S") || !wasmtime.stdout.includes("-Oz")) {
  console.error("[FAIL] C00 wasmtime direct smoke output");
  console.error(wasmtime.stdout);
  process.exit(1);
}
