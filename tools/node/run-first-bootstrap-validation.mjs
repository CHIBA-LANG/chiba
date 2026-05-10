import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ARTIFACT_DIR = ".scratch/first-bootstrap";

function run(name, command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    ...options,
  });
  if (result.status !== 0) {
    console.error(`[FAIL] ${name}`);
    console.error(`${result.stdout}${result.stderr}`.split("\n").slice(0, 40).join("\n"));
    process.exit(result.status || 1);
  }
  console.log(`[PASS] ${name}`);
  return result;
}

function sha256(file) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(file));
  return hash.digest("hex");
}

function binaryenVersion() {
  const pkg = JSON.parse(fs.readFileSync("node_modules/binaryen/package.json", "utf8"));
  return pkg.version;
}

function compileArtifact(name, source) {
  const watPath = path.join(ARTIFACT_DIR, `${name}.wat`);
  const wasmPath = path.join(ARTIFACT_DIR, `${name}.wasm`);
  const generated = run(`emit ${name}.wat`, "./target/debug/level1c.o", ["wat", source]);
  fs.writeFileSync(watPath, generated.stdout);
  run(`compile ${name}.wasm`, process.execPath, [
    "tools/node/compile-wat.mjs",
    watPath,
    "--output",
    wasmPath,
    "--opt",
  ]);
  return { name, source, watPath, wasmPath, watHash: sha256(watPath), wasmHash: sha256(wasmPath) };
}

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

run("bootstrap smoke", process.execPath, ["tools/node/run-bootstrap-smokes.mjs"]);
run("bootstrap smoke opt", process.execPath, ["tools/node/run-bootstrap-smokes.mjs", "--opt"]);
run("lexer compare", process.execPath, ["tools/node/run-lexer-compare.mjs"]);
run("parser compare", process.execPath, ["tools/node/run-parser-compare.mjs"]);
run("parser error smoke", process.execPath, ["tools/node/run-parser-error-smoke.mjs"]);
run("semantic gates", process.execPath, ["tools/node/run-semantic-gates.mjs"]);
run("level-1b smoke", process.execPath, ["tools/node/run-level1b-smoke.mjs"]);
run("level-1b std surface", process.execPath, ["tools/node/run-level1b-std-surface.mjs"]);

const artifacts = [
  compileArtifact("level1c", "supports/bootstrap/level1c-bootstrap-main.chiba"),
  compileArtifact("wat-tuple-heap-smoke", "supports/bootstrap/wat-tuple-heap-smoke.chiba"),
  compileArtifact("wat-extern-env-smoke", "supports/bootstrap/wat-extern-env-smoke.chiba"),
];

run("level1c.wasm help", process.execPath, [
  "tools/node/run-level1c-wasm.mjs",
  "--wasm",
  artifacts[0].wasmPath,
  "--help",
]);
run("level1c.wasm parse", process.execPath, [
  "tools/node/run-level1c-wasm.mjs",
  "--wasm",
  artifacts[0].wasmPath,
  "parse",
  "chiba-level1-grammar-spec/01-test.chiba",
]);
run("level1c.wasm check", process.execPath, [
  "tools/node/run-level1c-wasm.mjs",
  "--wasm",
  artifacts[0].wasmPath,
  "check",
  "supports/bootstrap/continuation-valid.chiba",
]);
run("level1c.wasm typed", process.execPath, [
  "tools/node/run-level1c-wasm.mjs",
  "--wasm",
  artifacts[0].wasmPath,
  "typed",
  "chiba-level1-grammar-spec/01-test.chiba",
]);
run("level1c.wasm nanopass", process.execPath, [
  "tools/node/run-level1c-wasm.mjs",
  "--wasm",
  artifacts[0].wasmPath,
  "nanopass",
  "chiba-level1-grammar-spec/01-test.chiba",
]);
run("level1c.wasm cont-usage", process.execPath, [
  "tools/node/run-level1c-wasm.mjs",
  "--wasm",
  artifacts[0].wasmPath,
  "cont-usage",
  "supports/bootstrap/continuation-multi-resume.chiba",
]);
run("all wat files", process.execPath, ["tools/node/run-all-wat.mjs"]);

console.log("[INFO] first bootstrap hashes");
console.log(`seed ${sha256("chibac_amd64-unknown-linux_chiba_dev.o")}`);
console.log(`level1c.o ${sha256("target/debug/level1c.o")}`);
console.log(`parser_spec_runner.o ${sha256("target/debug/parser_spec_runner.o")}`);
console.log(`node ${process.version}`);
console.log(`binaryen ${binaryenVersion()}`);
for (const artifact of artifacts) {
  console.log(`${artifact.name}.wat ${artifact.watHash}`);
  console.log(`${artifact.name}.wasm ${artifact.wasmHash}`);
}
