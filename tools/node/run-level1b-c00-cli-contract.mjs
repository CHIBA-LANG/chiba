import fs from "node:fs";
import process from "node:process";

const CONTRACT = "level-1b/compiler/cli/contract.chiba";
const README = "level-1b/compiler/cli/README.md";
const WAT = "level-1b/tests/wasmtime/chibac-help-smoke.wat";

function fail(message) {
  console.error(`[FAIL] ${message}`);
  process.exit(1);
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function requireIncludes(name, source, needles) {
  for (const needle of needles) {
    if (!source.includes(needle)) fail(`${name} missing ${needle}`);
  }
}

const contract = read(CONTRACT);
const readme = read(README);
const wat = read(WAT);

requireIncludes("CLI contract", contract, [
  "data ChibacTarget",
  "ChibacTargetWasm32UnknownWasi",
  "data ChibacTargetArg",
  "ChibacTargetMissing",
  "data ChibacBackend",
  "ChibacBackendWasmGc",
  "data ChibacBackendArg",
  "ChibacBackendMissing",
  "ChibacEmitParse",
  "ChibacEmitCheck",
  "ChibacEmitPreprocess",
  "ChibacEmitAssembly",
  "ChibacEmitObject",
  "ChibacEmitWat",
  "ChibacEmitWasm",
  "data ChibacOptLevel",
  "ChibacOpt0",
  "ChibacOpt1",
  "ChibacOpt2",
  "ChibacOpt3",
  "ChibacOptSize",
  "ChibacOptTiny",
  "data ChibacErrorFormat",
  "data ChibacColorMode",
  "cli_ready_for_codegen",
  "cli_short_flag_supported",
  "cli_long_flag_supported",
]);

requireIncludes("CLI contract short flags", contract, [
  '"-I"',
  '"-t"',
  '"-B"',
  '"-o"',
  '"-S"',
  '"-c"',
  '"-E"',
  '"-g"',
  '"-O0"',
  '"-O1"',
  '"-O2"',
  '"-O3"',
  '"-Os"',
  '"-Oz"',
]);

requireIncludes("CLI contract long flags", contract, [
  '"--include"',
  '"--target"',
  '"--backend"',
  '"--out"',
  '"--emit"',
  '"--error-format"',
  '"--color"',
  '"--diagnostic-width"',
]);

requireIncludes("CLI README", readme, [
  "chibac <file>...",
  "-I/--include",
  "--target/-t",
  "--backend/-B",
  "-o/--out",
  "-O0",
  "-Oz",
]);

requireIncludes("wasmtime help smoke", wat, [
  "chibac level-1b",
  "--target wasm32-unknown-wasi",
  "--backend wasm-gc",
  "--emit",
  "-S",
  "-c",
  "-E",
  "-O0",
  "-Oz",
  "--error-format",
  "--color",
  "--diagnostic-width",
]);

if (contract.includes("target: ChibacTarget,") || contract.includes("backend: ChibacBackend,")) {
  fail("CLI config must record explicit missing target/backend state");
}

if (contract.includes("opt_level: i64")) {
  fail("CLI config must use ChibacOptLevel instead of raw i64 optimization level");
}

console.log("[PASS] level-1b C00 CLI contract");
