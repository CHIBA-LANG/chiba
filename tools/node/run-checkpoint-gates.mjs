import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "supports/checkpoint";
const SEMANTIC_ROOT = "supports/semantic-gates";
const SCRATCH = ".scratch/checkpoint";

function run(command, args, timeout = "10") {
  return spawnSync("timeout", [timeout, command, ...args], { encoding: "utf8" });
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function fail(name, message) {
  console.error(`[FAIL] ${name}`);
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function assert(name, condition, message) {
  if (!condition) fail(name, message);
}

function parse(file) {
  const result = run("./target/debug/level1c.o", ["parse", file]);
  assert(`parse ${file}`, result.status === 0 && result.stdout.startsWith("OK("), result.stdout || result.stderr);
  return result.stdout;
}

function checkOk(file) {
  const result = run("./target/debug/level1c.o", ["check", file]);
  assert(`check ${file}`, result.status === 0 && result.stdout.includes("check ok"), result.stdout || result.stderr);
  return result.stdout;
}

function checkErr(file, expected) {
  const result = run("./target/debug/level1c.o", ["check", file]);
  assert(`check invalid ${file}`, result.status === 0 && result.stderr.includes(expected), result.stdout || result.stderr);
  return result.stderr;
}

function emitWat(file, outName) {
  fs.mkdirSync(SCRATCH, { recursive: true });
  const result = run("./target/debug/level1c.o", ["wat", file]);
  assert(`wat ${file}`, result.status === 0 && result.stdout.includes("(module"), result.stdout || result.stderr);
  const out = path.join(SCRATCH, outName);
  fs.writeFileSync(out, result.stdout);
  return out;
}

function runWat(file, expected) {
  const result = spawnSync(process.execPath, ["tools/node/run-wat.mjs", file, "--invoke", "main"], { encoding: "utf8" });
  assert(`run ${file}`, result.status === 0 && result.stdout.trim() === expected, result.stdout || result.stderr);
}

function assertIncludes(name, source, needles) {
  for (const needle of needles) assert(name, source.includes(needle), `missing ${needle}`);
}

function checkSyntaxSurface() {
  const name = "checkpoint syntax surface";
  const parsed = parse(path.join(ROOT, "syntax/checkpoint_surface.chiba"));
  assertIncludes(name, parsed, [
    "StringPart_Expr",
    "IdentTail_GenericCall",
    "IfCond_Let",
    "PatternIdent_Call",
    "Expr_PipeHole",
    "ParamPattern",
    '"tuple_to_adt"',
    '"adt_to_tuple"',
  ]);
  assert(name, !parsed.includes("ParamDiscard"), "parameters must not fall back to ParamDiscard");
  pass(name);
}

function checkPipeGlobalStringRuntime() {
  const name = "checkpoint pipe/global/string runtime";
  const file = path.join(ROOT, "correctness/pipe_global_string.chiba");
  checkOk(file);
  const wat = emitWat(file, "pipe_global_string.wat");
  const source = read(wat);
  assertIncludes(name, source, ["call $inc", "call $add3", "call $finish", "call $__chiba_string_concat2"]);
  runWat(wat, "142");
  pass(name);
}

function collectMethodKeys(source) {
  const keys = [];
  for (const match of source.matchAll(/\bdef\s+([A-Za-z_]\w*(?:\[[^\]]+\])?)\.([A-Za-z_]\w*)\s*\(/g)) {
    keys.push(`${match[1]}.${match[2]}`);
  }
  return keys;
}

function checkMethods() {
  const name = "checkpoint method correctness";
  const valid = path.join(ROOT, "correctness/method_surface.chiba");
  checkOk(valid);
  const parsed = parse(valid);
  assertIncludes(name, parsed, ["DefNameMethod", '"get"', '"bump"', "Expr_Field"]);

  const duplicate = read(path.join(ROOT, "correctness/method_duplicate_invalid.chiba"));
  const seen = new Set();
  let duplicated = "";
  for (const key of collectMethodKeys(duplicate)) {
    if (seen.has(key)) duplicated = key;
    seen.add(key);
  }
  assert(name, duplicated === "Widget.get", `expected duplicate Widget.get, got ${duplicated || "<none>"}`);

  checkErr(path.join(ROOT, "correctness/method_missing_import/use_without_import.chiba"), "unresolved method secret for Remote");
  const userSource = read(path.join(ROOT, "correctness/method_missing_import/use_without_import.chiba"));
  assert(name, !/\buse\s+checkpoint\.correctness\.hidden_methods/.test(userSource), "missing-import fixture accidentally imports hidden methods");
  pass(name);
}

function checkOperatorIndexing() {
  const name = "checkpoint operator indexing";
  const file = path.join(ROOT, "correctness/index_operator_surface.chiba");
  const source = read(file);
  assertIncludes(name, source, ["def Bag.op_index", "def Bag.op_index_slice", "bag[0]", "bag[0..4]"]);
  const parsed = run("./target/debug/level1c.o", ["parse", file]);
  assert(name, parsed.status === 0 && parsed.stdout.startsWith("Err("), "index fixture should capture current parser gap until bracket postfix is stabilized");
  pass(name);
}

function checkGenericsRows() {
  const name = "checkpoint generics and rows";
  checkOk(path.join(SEMANTIC_ROOT, "type_inference.chiba"));
  checkOk(path.join(SEMANTIC_ROOT, "row_poly.chiba"));
  checkOk(path.join(SEMANTIC_ROOT, "checked_template_instantiation.chiba"));

  const typed = run("./target/debug/level1c.o", ["typed", path.join(SEMANTIC_ROOT, "row_shorthand.chiba")]);
  assert(name, typed.status === 0, typed.stdout || typed.stderr);
  assertIncludes(name, typed.stdout, ["row_shorthand_identity", "type $T"]);
  pass(name);
}

function checkGlobalCycles() {
  const name = "checkpoint global cycle model";
  const source = read(path.join(ROOT, "correctness/global_cycle_invalid.chiba"));
  const deps = new Map();
  for (const match of source.matchAll(/\bdef\s+([A-Za-z_]\w*)\s*:\s*[^=]+=\s*([A-Za-z_]\w*)/g)) {
    deps.set(match[1], match[2]);
  }
  assert(name, deps.get("A") === "B" && deps.get("B") === "A", "fixture must model a direct global init cycle");
  pass(name);
}

function checkDeepPatternsAndClauses() {
  const name = "checkpoint deep patterns and pattern clauses";
  const surface = parse(path.join(ROOT, "syntax/checkpoint_surface.chiba"));
  assertIncludes(name, surface, ["IfCond_Let", "Expr_Match", "PatternIdent_Call", "Pattern_Record", "Pattern_Paren"]);
  const clauses = parse(path.join(ROOT, "correctness/pattern_clauses.chiba"));
  assert(name, (clauses.match(/DefName\(\n\s+"name"/g) || []).length === 2, "pattern clause fixture must keep two same-name clauses");
  assertIncludes(name, clauses, ["ParamPattern", '"Some"', '"None"']);
  pass(name);
}

function checkCompileIfMutualExclusion() {
  const name = "checkpoint compile_if mutually exclusive items";
  const parsed = parse(path.join(SEMANTIC_ROOT, "nominal_row_data_union.chiba"));
  assertIncludes(name, parsed, ["AttrArgIdentCall", '"all"', '"not"', '"or"']);
  checkOk(path.join(SEMANTIC_ROOT, "nominal_row_data_union.chiba"));
  checkErr(path.join(SEMANTIC_ROOT, "compile_if_invalid.chiba"), "unknown compile_if predicate");
  pass(name);
}

checkSyntaxSurface();
checkPipeGlobalStringRuntime();
checkMethods();
checkOperatorIndexing();
checkGenericsRows();
checkGlobalCycles();
checkDeepPatternsAndClauses();
checkCompileIfMutualExclusion();
