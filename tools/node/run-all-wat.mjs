import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = ".";

function listWatFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === ".git" || entry.name === "node_modules") continue;
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...listWatFiles(file));
    } else if (entry.isFile() && entry.name.endsWith(".wat")) {
      out.push(file.replace(/^\.\//, ""));
    }
  }
  return out.sort();
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function expectedFor(file) {
  if (file.includes("wat-wasi-exit-status-smoke.wat")) return { status: 7, includes: [] };
  if (file.includes("wat-wasi-args-env-smoke.wat")) {
    return { status: 0, args: ["--arg", "alpha", "--arg", "beta", "--env", "CHIBA_WASI_SMOKE=ok"], includes: ["301"] };
  }
  if (file.includes("wat-wasi-import-smoke.wat")) return { status: 0, includes: ["B04 wasi smoke ok", "0"] };
  if (file.includes("wat-wasi-file-read-smoke.wat")) return { status: 0, includes: ["66"] };
  if (file.includes("wat-wasi-array-slice-io-smoke.wat")) return { status: 0, includes: ["B04 file read ok", "66"] };
  if (file.includes("wat-start-smoke.wat")) return { status: 0, includes: ["12"] };
  if (file.includes("wat-env-import-smoke.wat")) return { status: 0, includes: ["env.js_log 41", "9"] };
  if (file.includes("wat-extern-env-smoke.wat")) return { status: 0, includes: ["env.js_log 41", "9"] };
  if (file.includes("wat-extern-wasi-smoke.wat")) return { status: 0, includes: ["0"] };
  if (file.includes("wat-tuple-heap-smoke.wat") || file.endsWith("/tuple.wat")) return { status: 0, includes: ["41"] };
  if (file.includes("wat-assign-smoke.wat")) return { status: 0, includes: ["7"] };
  if (file.includes("wat-loop-smoke.wat")) return { status: 0, includes: ["7"] };
  if (file.includes("wat-tailcall-smoke.wat")) return { status: 0, includes: ["0"] };
  if (file.includes("wat-smoke-01.wat") || file.includes("level1c-01.wat") || file.includes("01-test.wat")) return { status: 0, includes: ["1"] };
  if (file.includes("level1c.wat")) return { status: 0, includes: ["0"] };
  if (file.includes("method_resolution.wat")) return { status: 0, includes: ["4"] };
  if (file.includes("namespace") && file.includes("part_a.wat")) return { status: 0, includes: ["20"] };
  if (file.includes("namespace") && file.includes("part_b.wat")) return { status: 0, includes: ["22"] };
  if (file.includes("namespace") && file.includes("use_both.wat")) return { status: 0, includes: ["42"] };
  return { status: 0, includes: [] };
}

function instantiateOnly(file, wat) {
  if (!wat.includes('(export "main"') && !wat.includes('(export "_start"')) return true;
  if (file.includes("continuation_scheme_multi.wat")) return true;
  if (file.includes("refs_atomic_valid.wat")) return true;
  if (file.includes("refs_atomic_invalid.wat")) return true;
  if (file.includes("checked_template_instantiation_invalid.wat")) return true;
  if (file.includes("checked_template_instantiation.wat")) return true;
  if (file.includes("type_inference_invalid.wat")) return true;
  if (file.includes("row_shape_unify")) return true;
  return false;
}

function runWat(file, mode) {
  const args = ["--no-warnings", "tools/node/run-wat.mjs", file];
  if (mode.instantiateOnly) args.push("--instantiate-only");
  for (const arg of mode.args || []) args.push(arg);
  return spawnSync(process.execPath, args, { encoding: "utf8" });
}

let failed = 0;
let executed = 0;
let instantiated = 0;

for (const file of listWatFiles(ROOT)) {
  const wat = read(file);
  const mode = expectedFor(file);
  mode.instantiateOnly = instantiateOnly(file, wat);
  const result = runWat(file, mode);
  const output = `${result.stdout}${result.stderr}`;
  const ok =
    result.status === mode.status &&
    mode.includes.every((line) => output.includes(line));

  if (ok) {
    if (mode.instantiateOnly) {
      instantiated += 1;
      console.log(`[PASS] instantiate ${file}`);
    } else {
      executed += 1;
      console.log(`[PASS] run ${file}`);
    }
  } else {
    failed += 1;
    console.error(`[FAIL] ${mode.instantiateOnly ? "instantiate" : "run"} ${file}`);
    console.error(output.split("\n").slice(0, 12).join("\n"));
  }
}

if (failed !== 0) process.exit(1);
console.log(`[PASS] all wat files executed=${executed} instantiated=${instantiated}`);
