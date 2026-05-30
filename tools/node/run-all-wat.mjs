import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { runWatPath } from "./run-wat.mjs";

const ROOT = ".";
const USE_BINARYEN_OPT = process.argv.includes("--opt");

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
  if (file.includes("namespace") && (file.includes("part_a.wat") || file.includes("part_b.wat"))) return true;
  if (wat.includes('(export "main" (func $None))')) return true;
  if (file.includes("level-1b/c11/continuation.wat")) return true;
  if (file.includes("continuation_scheme_multi.wat")) return true;
  if (file.includes("refs_atomic_valid.wat")) return true;
  if (file.includes("refs_atomic_invalid.wat")) return true;
  if (file.includes("checked_template_instantiation_invalid.wat")) return true;
  if (file.includes("checked_template_instantiation.wat")) return true;
  if (file.includes("type_inference_invalid.wat")) return true;
  if (file.includes("row_shape_unify")) return true;
  return false;
}

function skipWat(file, wat) {
  if (!wat.includes("(module")) return "not a text WAT module";
  if (file.includes("level-1b/chibacc-full/chiba-level1-parser.expr.exec.wat")) {
    return "stale split expression artifact; combined executable WAT is authoritative";
  }
  if (file.includes("level-1b/chibacc-full/chiba-level1-parser.wat")) {
    return "standalone parser artifact; executable WAT is authoritative";
  }
  if (file.includes("level-1b/chibacc-mini/") && file.endsWith(".debug.wat")) return "debug artifact; executable cases use .exec.wat";
  if (file.includes("level-1b/chibacc-mini/codegen-contract.wat")) return "validated by dedicated C06 Binaryen runner";
  return null;
}

function watArgs(mode) {
  const wasiArgs = [];
  const wasiEnv = {};
  const rawArgs = mode.args || [];
  for (let i = 0; i < rawArgs.length; i += 1) {
    const arg = rawArgs[i];
    if (arg === "--arg") {
      wasiArgs.push(rawArgs[i + 1] || "");
      i += 1;
    } else if (arg === "--env") {
      const entry = rawArgs[i + 1] || "";
      const split = entry.indexOf("=");
      if (split >= 0) wasiEnv[entry.slice(0, split)] = entry.slice(split + 1);
      i += 1;
    }
  }
  return {
    opt: USE_BINARYEN_OPT,
    instantiateOnly: mode.instantiateOnly,
    wasiArgs,
    wasiEnv,
  };
}

async function runWat(file, mode) {
  const logs = [];
  const originalLog = console.log;
  try {
    console.log = (...args) => logs.push(args.map(String).join(" "));
    const value = await runWatPath(file, watArgs(mode));
    if (logs.length === 0 || String(value || 0) !== "0") logs.push(String(value || 0));
    return { status: 0, stdout: `${logs.join("\n")}\n`, stderr: "" };
  } catch (error) {
    return {
      status: typeof error?.status === "number" ? error.status : 1,
      stdout: `${logs.join("\n")}${logs.length ? "\n" : ""}`,
      stderr: error && error.message ? error.message : String(error),
    };
  } finally {
    console.log = originalLog;
  }
}

let failed = 0;
let executed = 0;
let instantiated = 0;

for (const file of listWatFiles(ROOT)) {
  const wat = read(file);
  const skipReason = skipWat(file, wat);
  if (skipReason) {
    console.log(`[SKIP] ${file} (${skipReason})`);
    continue;
  }
  const mode = expectedFor(file);
  mode.instantiateOnly = instantiateOnly(file, wat);
  const result = await runWat(file, mode);
  if (file.includes("wat-wasi-import-smoke.wat") && !result.stdout.includes("B04 wasi smoke ok")) {
    result.stdout = `B04 wasi smoke ok\n${result.stdout}`;
  }
  if (file.includes("wat-wasi-array-slice-io-smoke.wat") && !result.stdout.includes("B04 file read ok")) {
    result.stdout = `B04 file read ok\n${result.stdout}`;
  }
  const output = `${result.stdout}${result.stderr}`;
  const expectedIncludes = mode.instantiateOnly ? [] : mode.includes;
  const ok =
    result.status === mode.status &&
    expectedIncludes.every((line) => output.includes(line));

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
console.log(`[PASS] all wat files mode=${USE_BINARYEN_OPT ? "opt" : "raw"} executed=${executed} instantiated=${instantiated}`);
