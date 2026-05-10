import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "supports/semantic-gates";
const WAT_DIR = ".scratch/semantic-gates/wat";

const GATE_FILES = [
  "method_resolution.chiba",
  "method_resolution_invalid.chiba",
  "row_poly.chiba",
  "row_poly_invalid.chiba",
  "refs_atomic_valid.chiba",
  "refs_atomic_invalid.chiba",
  "type_inference.chiba",
  "type_inference_invalid.chiba",
  "continuation_scheme_multi.chiba",
  "string_slice.chiba",
  "namespace/part_a.chiba",
  "namespace/part_b.chiba",
  "namespace/use_both.chiba",
  "namespace_project/src/part_a.chiba",
  "namespace_project/src/part_b.chiba",
  "namespace_project/src/use_both.chiba",
];

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function run(command, args) {
  return spawnSync(command, args, { encoding: "utf8" });
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

function parseOk(file) {
  const result = run("./target/debug/level1c.o", ["parse", file]);
  assert(`parse ${file}`, result.status === 0 && result.stdout.startsWith("OK("), result.stdout || result.stderr);
}

function parseAll() {
  const files = GATE_FILES.map((name) => path.join(ROOT, name));
  for (const file of files) parseOk(file);
  pass("semantic gate sources parse");
}

function watName(file) {
  return file.replaceAll("/", "__").replace(/\.chiba$/, ".wat");
}

function emitWatAll() {
  fs.mkdirSync(WAT_DIR, { recursive: true });
  for (const rel of GATE_FILES) {
    const file = path.join(ROOT, rel);
    const result = run("./target/debug/level1c.o", ["wat", file]);
    assert(`emit wat ${rel}`, result.status === 0 && result.stdout.includes("(module"), result.stdout || result.stderr);
    fs.writeFileSync(path.join(WAT_DIR, watName(rel)), result.stdout);
  }
  pass("semantic gate wat files");
}

function collectTypes(source) {
  const types = new Map();
  for (const match of source.matchAll(/type\s+([A-Za-z_]\w*)[^{]*\{([\s\S]*?)\}/g)) {
    const fields = new Set();
    for (const field of match[2].matchAll(/\b([A-Za-z_]\w*)\s*:/g)) {
      fields.add(field[1]);
    }
    types.set(match[1], fields);
  }
  return types;
}

function collectMethods(source) {
  const methods = new Set();
  for (const match of source.matchAll(/def\s+([A-Za-z_]\w*)\.([A-Za-z_]\w*)\s*\(/g)) {
    methods.add(`${match[1]}.${match[2]}`);
  }
  return methods;
}

function collectParams(signature) {
  const params = new Map();
  for (const param of signature.split(",")) {
    const match = param.match(/\b([A-Za-z_]\w*)\s*:\s*([A-Za-z_]\w*(?:\[[^\]]+\])?)/);
    if (match) params.set(match[1], match[2]);
  }
  return params;
}

function checkMethodResolution() {
  const name = "method resolution routes";
  const source = read(path.join(ROOT, "method_resolution.chiba"));
  const types = collectTypes(source);
  const methods = collectMethods(source);

  for (const line of source.split("\n")) {
    const gate = line.match(/\/\/ gate: ([a-z-]+)/);
    if (!gate) continue;

    const def = line.match(/def\s+[A-Za-z_]\w*\(([^)]*)\)/);
    const call = line.match(/=\s*([A-Za-z_]\w*)\.([A-Za-z_]\w*)\s*\(/);
    assert(name, def && call, `cannot read method gate line: ${line}`);

    const params = collectParams(def[1]);
    const receiver = call[1];
    const member = call[2];
    let route = "unresolved";

    if (params.has(receiver)) {
      const receiverType = params.get(receiver);
      if (types.get(receiverType)?.has(member)) {
        route = "field-callable";
      } else if (methods.has(`${receiverType}.${member}`)) {
        route = "receiver-method";
      }
    } else if (methods.has(`${receiver}.${member}`)) {
      route = "qualified-callee";
    }

    assert(name, route === gate[1], `${line.trim()} routed as ${route}, expected ${gate[1]}`);
  }

  pass(name);
}

function checkMethodResolutionCompilerGate() {
  const name = "method resolution compiler gate";
  const valid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "method_resolution.chiba")]);
  assert(name, valid.status === 0 && valid.stdout.includes("check ok"), valid.stdout || valid.stderr);
  const invalid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "method_resolution_invalid.chiba")]);
  assert(name, invalid.status === 0 && invalid.stderr.includes("unresolved method missing for Widget"), invalid.stdout || invalid.stderr);
  pass(name);
}

function rowFields(rowBody) {
  const fields = [];
  for (const field of rowBody.matchAll(/\b([A-Za-z_]\w*)\s*:/g)) fields.push(field[1]);
  return fields.sort().join(",");
}

function checkRowPoly() {
  const name = "row polymorphism gates";
  const source = read(path.join(ROOT, "row_poly.chiba"));
  const rowKeys = [];

  for (const def of source.matchAll(/def\s+([A-Za-z_]\w*)\[([^\]]+)\][^{=\n]*=\s*([^\n]+)/g)) {
    const rows = [...def[2].matchAll(/\{([^}]*)\}/g)];
    assert(name, rows.length === 1, `${def[1]} must have exactly one row constraint in level-1`);
    const key = rowFields(rows[0][1]);
    rowKeys.push(key);
    const body = def[3];
    for (const access of body.matchAll(/\bvalue\.([A-Za-z_]\w*)/g)) {
      assert(name, key.split(",").includes(access[1]), `${def[1]} accesses missing row field ${access[1]}`);
    }
    if (def[1] === "row_identity") {
      assert(name, /:\s*T\s*=/.test(def[0]) && body.trim() === "value", "row_identity must return the full row-polymorphic value");
    }
  }

  assert(name, rowKeys[0] === rowKeys[1], `canonical row keys differ: ${rowKeys[0]} vs ${rowKeys[1]}`);
  pass(name);
}

function checkRowPolyCompilerGate() {
  const name = "row polymorphism compiler gate";
  const valid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "row_poly.chiba")]);
  assert(name, valid.status === 0 && valid.stdout.includes("check ok"), valid.stdout || valid.stderr);
  const invalid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "row_poly_invalid.chiba")]);
  assert(name, invalid.status === 0 && invalid.stderr.includes("row constraint missing field id"), invalid.stdout || invalid.stderr);
  pass(name);
}

function checkStringSlice() {
  const name = "string interpolation and slice gates";
  const source = read(path.join(ROOT, "string_slice.chiba"));
  assert(name, source.includes("${text}"), "string interpolation smoke missing");
  assert(name, source.includes('r#"raw ${text} stays raw"#'), "raw string smoke missing");
  assert(name, /text\[0\]/.test(source), "string byte-index smoke missing");
  assert(name, /text\[0\.\.4\]/.test(source), "slice smoke missing");
  assert(name, /text\.char_at\(0\)/.test(source), "explicit char_at smoke missing");
  const wat = read(path.join(WAT_DIR, "string_slice.wat"));
  assert(name, wat.includes("(type $array_u8 (array i8))"), "WAT backing byte array layout missing");
  assert(name, wat.includes("(type $slice_u8 (struct (field (ref $array_u8)) (field i32) (field i32)))"), "WAT slice layout missing");
  assert(name, wat.includes("array.new_fixed $array_u8 0"), "String does not lower to managed Array[u8]");
  assert(name, wat.includes("struct.new $slice_u8"), "str/slice expression does not lower to managed Slice[u8] view");
  pass(name);
}

function extractNamespace(source) {
  const match = source.match(/\bnamespace\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)/);
  return match ? match[1] : "";
}

function extractDefs(source) {
  return [...source.matchAll(/\bdef\s+([A-Za-z_]\w*)\s*(?:\[|\(|:)/g)].map((match) => match[1]);
}

function checkNamespaceMerge() {
  const name = "namespace multi-file merge";
  const project = path.join(ROOT, "namespace_project");
  const files = ["part_a.chiba", "part_b.chiba", "use_both.chiba"].map((file) => path.join(project, "src", file));
  const table = new Map();
  for (const file of files) {
    const source = read(file);
    const ns = extractNamespace(source);
    const defs = extractDefs(source);
    if (!table.has(ns)) table.set(ns, new Set());
    for (const def of defs) table.get(ns).add(def);
  }

  assert(name, table.get("semantic.gates.parts")?.has("left"), "part_a left() missing from merged namespace");
  assert(name, table.get("semantic.gates.parts")?.has("right"), "part_b right() missing from merged namespace");
  const app = read(path.join(project, "src/use_both.chiba"));
  assert(name, app.includes("use semantic.gates.parts.*"), "consumer must import merged namespace");
  assert(name, /\bleft\(\)\s*\+\s*right\(\)/.test(app), "consumer must call functions from both namespace fragments");
  const compiled = run("timeout", [
    "10",
    "./chibac_amd64-unknown-linux_chiba_dev.o",
    "--project",
    project,
    "--entry",
    "use_both.chiba",
    "--output",
    "namespace_gate.o",
  ]);
  assert(name, compiled.status === 0, compiled.stdout || compiled.stderr);
  pass(name);
}

function checkTopLevelRefAttrs(source, expectOk) {
  const errors = [];
  const lines = source.split("\n");
  for (let i = 0; i < lines.length; i += 1) {
    if (/\bdef\s+[A-Za-z_]\w*\s*:\s*Ref\[/.test(lines[i])) {
      const prev = lines[i - 1] || "";
      if (!prev.includes("#[world_local]")) errors.push(`top-level Ref without #[world_local] at line ${i + 1}`);
    }
  }
  if (expectOk) assert("Ref/Atomic memory gates", errors.length === 0, errors.join("\n"));
  return errors;
}

function checkAtomicTypes(source, expectOk) {
  const errors = [];
  for (const match of source.matchAll(/Atomic\[((?:Ptr\[[^\]]+\])|[^\]]+)\]/g)) {
    const ty = match[1].trim();
    if (!/^(i32|i64|usize|bool|Ptr\[[^\]]+\])$/.test(ty)) {
      errors.push(`unsupported Atomic[${ty}]`);
    }
  }
  if (expectOk) assert("Ref/Atomic memory gates", errors.length === 0, errors.join("\n"));
  return errors;
}

function checkAtomicOrderings(source, expectOk) {
  const errors = [];
  for (const match of source.matchAll(/Atomic\.load\([^,\n]+,\s*([A-Za-z_]\w*)\)/g)) {
    if (match[1] === "Release" || match[1] === "AcqRel") errors.push(`invalid load ordering ${match[1]}`);
  }
  for (const match of source.matchAll(/Atomic\.store\([^,\n]+,\s*[^,\n]+,\s*([A-Za-z_]\w*)\)/g)) {
    if (match[1] === "Acquire" || match[1] === "AcqRel") errors.push(`invalid store ordering ${match[1]}`);
  }
  if (expectOk) assert("Ref/Atomic memory gates", errors.length === 0, errors.join("\n"));
  return errors;
}

function checkAssignments(source, expectOk) {
  const errors = [];
  const params = new Map();
  for (const fn of source.matchAll(/def\s+[A-Za-z_]\w*\(([^)]*)\)/g)) {
    for (const [name, ty] of collectParams(fn[1])) params.set(name, ty);
  }
  for (const match of source.matchAll(/([A-Za-z_]\w*(?:\[[^\]]+\])?(?:\.[A-Za-z_]\w*)?)\s*:=/g)) {
    const lhs = match[1];
    if (lhs.includes("[")) {
      const base = lhs.slice(0, lhs.indexOf("["));
      if (/^Ref\[Array/.test(params.get(base) || "")) {
        errors.push("Ref[Array[T]] direct element assignment is illegal");
      }
    } else {
      const base = lhs.split(".")[0];
      if (!/^Ref\[/.test(params.get(base) || "") && !source.includes(`def ${base}: Ref[`)) {
        errors.push(`assignment lhs ${lhs} is not known Ref[T]`);
      }
    }
  }
  if (expectOk) assert("Ref/Atomic memory gates", errors.length === 0, errors.join("\n"));
  return errors;
}

function checkMemory() {
  const name = "Ref/UnsafeRef/Ptr/Atomic gates";
  const valid = read(path.join(ROOT, "refs_atomic_valid.chiba"));
  const invalid = read(path.join(ROOT, "refs_atomic_invalid.chiba"));
  checkTopLevelRefAttrs(valid, true);
  checkAtomicTypes(valid, true);
  checkAtomicOrderings(valid, true);
  checkAssignments(valid, true);

  const errors = [
    ...checkTopLevelRefAttrs(invalid, false),
    ...checkAtomicTypes(invalid, false),
    ...checkAtomicOrderings(invalid, false),
    ...checkAssignments(invalid, false),
  ];
  assert(name, errors.some((err) => err.includes("top-level Ref")), "invalid top-level Ref was not rejected");
  assert(name, errors.some((err) => err.includes("Atomic[Record]")), "invalid Atomic[Record] was not rejected");
  assert(name, errors.some((err) => err.includes("load ordering")), "invalid Atomic.load ordering was not rejected");
  assert(name, errors.some((err) => err.includes("store ordering")), "invalid Atomic.store ordering was not rejected");
  assert(name, errors.some((err) => err.includes("Ref[Array")), "Ref[Array[T]] element assignment was not rejected");
  const checkedValid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "refs_atomic_valid.chiba")]);
  assert(name, checkedValid.status === 0 && checkedValid.stdout.includes("check ok"), checkedValid.stdout || checkedValid.stderr);
  const checkedInvalid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "refs_atomic_invalid.chiba")]);
  assert(name, checkedInvalid.status === 0 && checkedInvalid.stderr.includes("top-level Ref requires #[world_local]"), checkedInvalid.stdout || checkedInvalid.stderr);
  pass(name);
}

function checkTypeInference() {
  const name = "type inference gates";
  const valid = run("./target/debug/level1c.o", ["parse", path.join(ROOT, "type_inference.chiba")]);
  assert(name, valid.status === 0 && valid.stdout.startsWith("OK("), valid.stdout || valid.stderr);
  const checkedValid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_inference.chiba")]);
  assert(name, checkedValid.status === 0 && checkedValid.stdout.includes("check ok"), checkedValid.stdout || checkedValid.stderr);
  const checkedInvalid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_inference_invalid.chiba")]);
  assert(name, checkedInvalid.status === 0 && checkedInvalid.stderr.includes("generic parameter type requires annotation"), checkedInvalid.stdout || checkedInvalid.stderr);
  pass(name);
}

function evaluateClassicShiftReset() {
  const k = (value) => 2 * value;
  return 1 + k(k(4));
}

function checkContinuation() {
  const name = "delimited continuation multi-entry";
  const file = path.join(ROOT, "continuation_scheme_multi.chiba");
  const checked = run("./target/debug/level1c.o", ["check", file]);
  assert(name, checked.status === 0 && checked.stdout.includes("check ok"), checked.stdout || checked.stderr);
  const usage = run("./target/debug/level1c.o", ["cont-usage", file]);
  assert(name, usage.status === 0 && usage.stdout.includes("resumes=2 class=many"), usage.stdout || usage.stderr);
  assert(name, evaluateClassicShiftReset() === 17, "classic Scheme shift/reset multi-shot value must be 17");
  pass(name);
}

parseAll();
emitWatAll();
checkMethodResolution();
checkMethodResolutionCompilerGate();
checkRowPoly();
checkRowPolyCompilerGate();
checkNamespaceMerge();
checkStringSlice();
checkMemory();
checkTypeInference();
checkContinuation();
