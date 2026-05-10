import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const WAT_DIR = ".scratch/bootstrap-smokes/wat";

function run(command, args, options = {}) {
  return spawnSync(command, args, {
    encoding: "utf8",
    ...options,
  });
}

const USE_BINARYEN_OPT = process.argv.includes("--opt");

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
    ...(USE_BINARYEN_OPT ? ["--opt"] : []),
    ...(test.args || []),
  ]);
  return checkOutput(test.name, result, test.expect, test.status || 0);
}

function runLevel1c(test) {
  const result = run("./target/debug/level1c.o", test.args);
  return checkOutput(test.name, result, test.expect);
}

function watName(file) {
  return file.replaceAll("/", "__").replace(/\.chiba$/, ".wat");
}

function writeGeneratedWat(test, wat) {
  fs.mkdirSync(WAT_DIR, { recursive: true });
  fs.writeFileSync(path.join(WAT_DIR, watName(test.file)), wat);
}

function runGeneratedWat(test) {
  const generated = run("./target/debug/level1c.o", ["wat", test.file]);
  if (generated.status !== 0) {
    return checkOutput(test.name, generated, test.expect);
  }
  writeGeneratedWat(test, generated.stdout);

  const result = run(
    process.execPath,
    [
      "--no-warnings",
      "tools/node/run-wat.mjs",
      "-",
      ...(USE_BINARYEN_OPT ? ["--opt"] : []),
    ],
    { input: generated.stdout },
  );
  return checkOutput(test.name, result, test.expect);
}

function checkGeneratedWatText(test) {
  const generated = run("./target/debug/level1c.o", ["wat", test.file]);
  if (generated.status === 0) writeGeneratedWat(test, generated.stdout);
  return checkOutput(test.name, generated, test.expect);
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
    name: "wasi file read",
    file: "supports/bootstrap/wat-wasi-file-read-smoke.wat",
    expect: ["66"],
  },
  {
    name: "wasi args env",
    file: "supports/bootstrap/wat-wasi-args-env-smoke.wat",
    args: ["--arg", "alpha", "--arg", "beta", "--env", "CHIBA_WASI_SMOKE=ok"],
    expect: ["301"],
  },
  {
    name: "wasi exit status",
    file: "supports/bootstrap/wat-wasi-exit-status-smoke.wat",
    expect: [],
    status: 7,
  },
  {
    name: "default _start",
    file: "supports/bootstrap/wat-start-smoke.wat",
    expect: ["12"],
  },
];

const LEVEL1C_CASES = [
  {
    name: "level1c help",
    args: ["--help"],
    expect: ["Usage: level1c <command> <file>", "Commands: lex parse check cir typed nanopass cont-usage wat"],
  },
  {
    name: "level1c parse grammar 01",
    args: ["parse", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: ["OK(", "SourceFile("],
  },
  {
    name: "level1c typed grammar 01",
    args: ["typed", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: ["L2Module", "L2OpTyped", "type i64", "0"],
  },
  {
    name: "level1c nanopass grammar 01",
    args: ["nanopass", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: ["L8ValidatedCoreModule", "L8ValidatedCoreOp", "L7CoreOp", "L6OpClosureEnv", "L5OpCps", "L4OpUsage", "L3OpAnswerEffect", "validation ok", "0"],
  },
  {
    name: "level1c check continuation valid",
    args: ["check", "supports/bootstrap/continuation-valid.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c check continuation nested reset",
    args: ["check", "supports/bootstrap/continuation-nested.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c check continuation multi resume",
    args: ["check", "supports/bootstrap/continuation-multi-resume.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c continuation usage multi resume",
    args: ["cont-usage", "supports/bootstrap/continuation-multi-resume.chiba"],
    expect: ["continuation #1 \"k\" resumes=2 class=many", "0"],
  },
  {
    name: "level1c check continuation invalid",
    args: ["check", "supports/bootstrap/continuation-invalid.chiba"],
    expect: ["shift outside reset", "3"],
  },
  {
    name: "level1c check continuation cross world invalid",
    args: ["check", "supports/bootstrap/continuation-cross-world-invalid.chiba"],
    expect: ["continuation crosses world/thread boundary", "3"],
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
    name: "generated wat assign",
    file: "supports/bootstrap/wat-assign-smoke.chiba",
    expect: ["7"],
  },
  {
    name: "generated wat tailcall",
    file: "supports/bootstrap/wat-tailcall-smoke.chiba",
    expect: ["0"],
  },
  {
    name: "generated wat tuple heap",
    file: "supports/bootstrap/wat-tuple-heap-smoke.chiba",
    expect: ["41"],
  },
  {
    name: "generated wat extern env",
    file: "supports/bootstrap/wat-extern-env-smoke.chiba",
    expect: ["env.js_log 41", "9"],
  },
];

const GENERATED_WAT_TEXT_CASES = [
  {
    name: "generated wat extern wasi import",
    file: "supports/bootstrap/wat-extern-wasi-smoke.chiba",
    expect: ['(import "wasi_snapshot_preview1" "fd_write" (func $wasi_fd_write'],
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

for (const test of GENERATED_WAT_TEXT_CASES) {
  failed += checkGeneratedWatText(test);
}

if (failed !== 0) {
  process.exit(1);
}
