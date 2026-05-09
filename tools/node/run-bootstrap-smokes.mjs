import { spawnSync } from "node:child_process";
import process from "node:process";

const CASES = [
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
];

let failed = 0;

for (const test of CASES) {
  const result = spawnSync(
    process.execPath,
    ["--no-warnings", "tools/node/run-wat.mjs", test.file],
    { encoding: "utf8" },
  );
  const output = `${result.stdout}${result.stderr}`;
  const ok =
    result.status === 0 && test.expect.every((line) => output.includes(line));

  if (ok) {
    console.log(`[PASS] ${test.name}`);
  } else {
    failed += 1;
    console.error(`[FAIL] ${test.name}`);
    console.error(output.split("\n").slice(0, 16).join("\n"));
  }
}

if (failed !== 0) {
  process.exit(1);
}
