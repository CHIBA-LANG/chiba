import { spawnSync } from "node:child_process";
import process from "node:process";

function run(command, args, options = {}) {
  return spawnSync(command, args, {
    encoding: "utf8",
    ...options,
  });
}

function checkOutput(name, result, expect, status = 0) {
  const output = `${result.stdout}${result.stderr}`;
  const ok =
    result.status === status && expect.every((line) => output.includes(line));

  if (ok) {
    console.log(`[PASS] ${name}`);
    return 0;
  }

  console.error(`[FAIL] ${name}`);
  console.error(output.split("\n").slice(0, 18).join("\n"));
  return 1;
}

function runWatFile(test) {
  const result = run(process.execPath, [
    "--no-warnings",
    "tools/node/run-wat.mjs",
    test.file,
  ]);
  return checkOutput(test.name, result, test.expect);
}

function runLevel1c(test) {
  const result = run("./target/debug/level1c.o", test.args);
  return checkOutput(test.name, result, test.expect);
}

function runGeneratedWat(test) {
  const generated = run("./target/debug/level1c.o", ["wat", test.file]);
  if (generated.status !== 0) {
    return checkOutput(test.name, generated, test.expect);
  }

  const result = run(
    process.execPath,
    ["--no-warnings", "tools/node/run-wat.mjs", "-"],
    { input: generated.stdout },
  );
  return checkOutput(test.name, result, test.expect);
}

const WAT_CASES = [
  {
    name: "env import",
    file: "supports/bootstrap/wat-env-import-smoke.wat",
    expect: ["env.js_log 41", "9"],
  },
  {
    name: "wasi fd_write",
    file: "supports/bootstrap/wat-wasi-import-smoke.wat",
    expect: ["B04 wasi smoke ok", "0"],
  },
  {
    name: "default _start",
    file: "supports/bootstrap/wat-start-smoke.wat",
    expect: ["12"],
  },
];

const LEVEL1C_CASES = [
  {
    name: "level1c check continuation valid",
    args: ["check", "supports/bootstrap/continuation-valid.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c check continuation invalid",
    args: ["check", "supports/bootstrap/continuation-invalid.chiba"],
    expect: ["shift outside reset", "3"],
  },
];

const GENERATED_WAT_CASES = [
  {
    name: "generated wat grammar 01",
    file: "chiba-level1-grammar-spec/01-test.chiba",
    expect: ["1"],
  },
  {
    name: "generated wat loop",
    file: "supports/bootstrap/wat-loop-smoke.chiba",
    expect: ["7"],
  },
  {
    name: "generated wat tailcall",
    file: "supports/bootstrap/wat-tailcall-smoke.chiba",
    expect: ["0"],
  },
];

let failed = 0;

for (const test of WAT_CASES) {
  failed += runWatFile(test);
}

for (const test of LEVEL1C_CASES) {
  failed += runLevel1c(test);
}

for (const test of GENERATED_WAT_CASES) {
  failed += runGeneratedWat(test);
}

if (failed !== 0) {
  process.exit(1);
}
