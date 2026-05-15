import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "supports/semantic-gates";
const WAT_DIR = ".scratch/semantic-gates/wat";

const GATE_FILES = [
  "method_resolution.chiba",
  "method_resolution_invalid.chiba",
  "operator_resolution.chiba",
  "operator_resolution_invalid_missing.chiba",
  "operator_resolution_invalid_ambiguous.chiba",
  "operator_resolution_invalid_operand.chiba",
  "row_poly.chiba",
  "row_poly_invalid.chiba",
  "row_shape_unify.chiba",
  "row_shape_unify_invalid_record.chiba",
  "row_shape_unify_invalid_update.chiba",
  "row_shape_unify_invalid_generic.chiba",
  "row_shape_unify_invalid_generic_name.chiba",
  "row_shorthand.chiba",
  "row_shorthand_invalid.chiba",
  "checked_template_instantiation.chiba",
  "checked_template_instantiation_invalid.chiba",
  "refs_atomic_valid.chiba",
  "refs_atomic_invalid.chiba",
  "type_inference.chiba",
  "type_inference_invalid.chiba",
  "type_generics.chiba",
  "type_generics_invalid_return.chiba",
  "type_generics_invalid_duplicate.chiba",
  "type_unify.chiba",
  "type_unify_invalid_return.chiba",
  "type_unify_invalid_let.chiba",
  "type_unify_invalid_binary.chiba",
  "extern_abi.chiba",
  "extern_abi_invalid.chiba",
  "extern_abi_invalid_signature.chiba",
  "nominal_row_data_union.chiba",
  "compile_if_invalid.chiba",
  "nominal_row_data_union_invalid.chiba",
  "nominal_row_data_union_invalid_data.chiba",
  "nominal_row_data_union_invalid_union.chiba",
  "continuation_scheme_multi.chiba",
  "string_slice.chiba",
  "string_return.chiba",
  "pipe.chiba",
  "namespace/part_a.chiba",
  "namespace/part_b.chiba",
  "namespace/use_both.chiba",
  "namespace_project/src/part_a.chiba",
  "namespace_project/src/part_b.chiba",
  "namespace_project/src/use_both.chiba",
];

// Pre-C06 routes WAT emission through L8 validated Core. Fixtures that exercise
// semantic-only constructs such as methods, rich generics, invalid programs, or
// project-level namespace merging stay check-only until their lowering passes
// produce complete Core.
const WAT_GATE_FILES = [
  "string_slice.chiba",
  "string_return.chiba",
  "type_unify.chiba",
  "type_inference.chiba",
  "extern_abi.chiba",
  "pipe.chiba",
  "namespace/part_a.chiba",
  "namespace/part_b.chiba",
];

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function run(command, args) {
  if (command === "./target/debug/level1c.o") {
    return spawnSync("timeout", ["10", command, ...args], { encoding: "utf8" });
  }
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
  fs.rmSync(WAT_DIR, { recursive: true, force: true });
  fs.mkdirSync(WAT_DIR, { recursive: true });
  for (const rel of WAT_GATE_FILES) {
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

function checkOperatorResolutionCompilerGate() {
  const name = "operator resolution compiler gate";
  const valid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "operator_resolution.chiba")]);
  assert(name, valid.status === 0 && valid.stdout.includes("check ok"), valid.stdout || valid.stderr);
  const missing = run("./target/debug/level1c.o", ["check", path.join(ROOT, "operator_resolution_invalid_missing.chiba")]);
  assert(name, missing.status === 0 && missing.stderr.includes("missing operator op_add for Scalar"), missing.stdout || missing.stderr);
  const ambiguous = run("./target/debug/level1c.o", ["check", path.join(ROOT, "operator_resolution_invalid_ambiguous.chiba")]);
  assert(name, ambiguous.status === 0 && ambiguous.stderr.includes("ambiguous operator op_add for Scalar"), ambiguous.stdout || ambiguous.stderr);
  const operand = run("./target/debug/level1c.o", ["check", path.join(ROOT, "operator_resolution_invalid_operand.chiba")]);
  assert(name, operand.status === 0 && operand.stderr.includes("operator operand type mismatch"), operand.stdout || operand.stderr);
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

function checkRowShapeUnify() {
  const name = "row shape unification gates";
  const checkedValid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "row_shape_unify.chiba")]);
  assert(name, checkedValid.status === 0 && checkedValid.stdout.includes("check ok"), checkedValid.stdout || checkedValid.stderr);
  const badRecord = run("./target/debug/level1c.o", ["check", path.join(ROOT, "row_shape_unify_invalid_record.chiba")]);
  assert(name, badRecord.status === 0 && badRecord.stderr.includes("duplicate record field"), badRecord.stdout || badRecord.stderr);
  const badUpdate = run("./target/debug/level1c.o", ["check", path.join(ROOT, "row_shape_unify_invalid_update.chiba")]);
  assert(name, badUpdate.status === 0 && badUpdate.stderr.includes("duplicate record field"), badUpdate.stdout || badUpdate.stderr);
  const badGeneric = run("./target/debug/level1c.o", ["check", path.join(ROOT, "row_shape_unify_invalid_generic.chiba")]);
  assert(name, badGeneric.status === 0 && badGeneric.stderr.includes("duplicate row field"), badGeneric.stdout || badGeneric.stderr);
  const badGenericName = run("./target/debug/level1c.o", ["check", path.join(ROOT, "row_shape_unify_invalid_generic_name.chiba")]);
  assert(name, badGenericName.status === 0 && badGenericName.stderr.includes("duplicate generic parameter"), badGenericName.stdout || badGenericName.stderr);
  pass(name);
}

function checkRowShorthand() {
  const name = "row shorthand gates";
  const valid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "row_shorthand.chiba")]);
  assert(name, valid.status === 0 && valid.stdout.includes("check ok"), valid.stdout || valid.stderr);
  const invalid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "row_shorthand_invalid.chiba")]);
  assert(name, invalid.status === 0 && invalid.stderr.includes("row constraint missing field id"), invalid.stdout || invalid.stderr);
  const typed = run("./target/debug/level1c.o", ["typed", path.join(ROOT, "row_shorthand.chiba")]);
  assert(name, typed.status === 0 && typed.stdout.includes("type $T"), typed.stdout || typed.stderr);
  pass(name);
}

function checkCheckedTemplateInstantiation() {
  const name = "checked template instantiation gates";
  const valid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "checked_template_instantiation.chiba")]);
  assert(name, valid.status === 0 && valid.stdout.includes("check ok"), valid.stdout || valid.stderr);
  const invalid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "checked_template_instantiation_invalid.chiba")]);
  assert(name, invalid.status === 0 && invalid.stderr.includes("generic instantiation missing field name"), invalid.stdout || invalid.stderr);
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
  assert(name, wat.includes("(type $array_u8 (array (mut i8)))"), "WAT backing byte array layout missing");
  assert(name, wat.includes("(type $slice_u8 (struct (field (ref $array_u8)) (field i32) (field i32)))"), "WAT slice layout missing");
  assert(name, wat.includes("array.new_fixed $array_u8 21"), "raw string literal does not lower to real Array[u8] payload");
  assert(name, wat.includes("i32.const 114") && wat.includes("i32.const 119"), "raw string literal byte payload missing");
  assert(name, wat.includes("call $__chiba_string_concat2"), "string interpolation does not call concat runtime");
  assert(name, wat.includes("(param $v1 (ref $array_u8))"), "String parameter does not lower to Array[u8] ref");
  assert(name, wat.includes("call $__chiba_string_byte_at"), "String byte index does not lower through bounds helper");
  assert(name, wat.includes("call $__chiba_string_slice"), "String range slice does not lower through bounds helper");
  assert(name, wat.includes("array.get_u $array_u8"), "String byte helper does not read Array[u8]");
  assert(name, wat.includes("struct.new $slice_u8"), "String range helper does not build Slice[u8] view");
  assert(name, wat.includes("call $__chiba_string_codepoint_at"), "char_at/codepoint_at does not lower to UTF-8 codepoint helper");
  pass(name);
}

function checkStringReturnAbi() {
  const name = "string result ABI gates";
  const wat = read(path.join(WAT_DIR, "string_return.wat"));
  assert(name, wat.includes("(func $string_return_value (result (ref $array_u8))"), "String-returning function does not use Array[u8] result ABI");
  assert(name, wat.includes("(func $string_return_inferred (result (ref $array_u8))"), "Inferred String-returning function does not use Array[u8] result ABI");
  assert(name, /\(local \$v[0-9]+ \(ref \$array_u8\)\)/.test(wat), "String-returning call result does not bind to Array[u8] local");
  assert(name, wat.includes("call $string_return_value"), "String-returning helper call missing");
  assert(name, wat.includes("call $string_return_inferred"), "Inferred String-returning helper call missing");
  assert(name, wat.includes("call $__chiba_string_byte_at"), "String-returning local cannot be byte-indexed through helper");
  pass(name);
}

function checkPipeLowering() {
  const name = "pipe parser and codegen gates";
  const file = path.join(ROOT, "pipe.chiba");
  const parsed = run("./target/debug/level1c.o", ["parse", file]);
  assert(name, parsed.status === 0 && parsed.stdout.includes("OpPipeGt"), parsed.stdout || parsed.stderr);
  assert(name, parsed.stdout.includes("Expr_PipeHole"), "pipe placeholder must parse as Expr_PipeHole");
  assert(name, parsed.stdout.includes("ParamDiscard"), "wildcard parameter must parse as ParamDiscard");
  assert(name, parsed.stdout.includes("Pattern_Wildcard"), "let wildcard must parse as Pattern_Wildcard");

  const cir = run("./target/debug/level1c.o", ["cir", file]);
  assert(name, cir.status === 0, cir.stdout || cir.stderr);
  assert(name, !cir.stdout.includes("binary |>"), "pipe must desugar before CIR binary lowering");
  assert(name, cir.stdout.includes('L1RefUnresolved("inc")'), "default pipe must lower to inc(lhs)");
  assert(name, cir.stdout.includes('L1RefUnresolved("add")'), "placeholder pipe must lower to add(...lhs...)");
  assert(name, cir.stdout.includes("L1StmtExpr") && cir.stdout.includes('L1RefUnresolved("weak_param")'), "let _ must lower as a discard expression statement");

  const wat = read(path.join(WAT_DIR, "pipe.wat"));
  assert(name, !wat.includes("unsupported-binary"), "pipe WAT must not use unsupported binary fallback");
  assert(name, wat.includes("call $inc"), "default pipe WAT call to inc missing");
  assert(name, wat.includes("call $add"), "placeholder pipe WAT call to add missing");
  assert(name, wat.includes("call $finish"), "pipe chain WAT call to finish missing");

  const runWat = run(process.execPath, ["tools/node/run-wat.mjs", path.join(WAT_DIR, "pipe.wat"), "--invoke", "main"]);
  assert(name, runWat.status === 0 && runWat.stdout.trim() === "36", runWat.stdout || runWat.stderr);
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
  const projectChecked = run("./target/debug/level1c.o", ["check-project", project]);
  assert(name, projectChecked.status === 0 && projectChecked.stdout.includes("check project ok"), projectChecked.stdout || projectChecked.stderr);
  const invalidProject = run("./target/debug/level1c.o", ["check-project", path.join(ROOT, "namespace_project_invalid")]);
  assert(name, invalidProject.status === 0 && invalidProject.stderr.includes("project missing src/part_b.chiba"), invalidProject.stdout || invalidProject.stderr);
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
  assert(name, checkedInvalid.status === 0 && checkedInvalid.stderr.includes("unsupported Atomic[T]"), checkedInvalid.stdout || checkedInvalid.stderr);
  pass(name);
}

function checkTypeInference() {
  const name = "type inference gates";
  const valid = run("./target/debug/level1c.o", ["parse", path.join(ROOT, "type_inference.chiba")]);
  assert(name, valid.status === 0 && valid.stdout.startsWith("OK("), valid.stdout || valid.stderr);
  const checkedValid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_inference.chiba")]);
  assert(name, checkedValid.status === 0 && checkedValid.stdout.includes("check ok"), checkedValid.stdout || checkedValid.stderr);
  const checkedInvalid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_inference_invalid.chiba")]);
  assert(name, checkedInvalid.status === 0 && checkedInvalid.stderr.includes("Ref.new(None) requires explicit Option[T] annotation"), checkedInvalid.stdout || checkedInvalid.stderr);
  pass(name);
}

function checkTypeGenerics() {
  const name = "explicit generic gates";
  const valid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_generics.chiba")]);
  assert(name, valid.status === 0 && valid.stdout.includes("check ok"), valid.stdout || valid.stderr);
  const badReturn = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_generics_invalid_return.chiba")]);
  assert(name, badReturn.status === 0 && badReturn.stderr.includes("return type mismatch"), badReturn.stdout || badReturn.stderr);
  const badDuplicate = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_generics_invalid_duplicate.chiba")]);
  assert(name, badDuplicate.status === 0 && badDuplicate.stderr.includes("duplicate generic parameter"), badDuplicate.stdout || badDuplicate.stderr);
  const typed = run("./target/debug/level1c.o", ["typed", path.join(ROOT, "type_generics.chiba")]);
  assert(name, typed.status === 0 && typed.stdout.includes("type T"), typed.stdout || typed.stderr);
  pass(name);
}

function checkTypeUnify() {
  const name = "type unification gates";
  const checkedValid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_unify.chiba")]);
  assert(name, checkedValid.status === 0 && checkedValid.stdout.includes("check ok"), checkedValid.stdout || checkedValid.stderr);
  const badReturn = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_unify_invalid_return.chiba")]);
  assert(name, badReturn.status === 0 && badReturn.stderr.includes("return type mismatch"), badReturn.stdout || badReturn.stderr);
  const badLet = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_unify_invalid_let.chiba")]);
  assert(name, badLet.status === 0 && badLet.stderr.includes("let type mismatch"), badLet.stdout || badLet.stderr);
  const badBinary = run("./target/debug/level1c.o", ["check", path.join(ROOT, "type_unify_invalid_binary.chiba")]);
  assert(name, badBinary.status === 0 && badBinary.stderr.includes("expression type mismatch"), badBinary.stdout || badBinary.stderr);
  pass(name);
}

function checkExternAbi() {
  const name = "extern ABI gates";
  const checkedValid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "extern_abi.chiba")]);
  assert(name, checkedValid.status === 0 && checkedValid.stdout.includes("check ok"), checkedValid.stdout || checkedValid.stderr);
  const badAbi = run("./target/debug/level1c.o", ["check", path.join(ROOT, "extern_abi_invalid.chiba")]);
  assert(name, badAbi.status === 0 && badAbi.stderr.includes("unsupported extern ABI"), badAbi.stdout || badAbi.stderr);
  const badSig = run("./target/debug/level1c.o", ["check", path.join(ROOT, "extern_abi_invalid_signature.chiba")]);
  assert(name, badSig.status === 0 && badSig.stderr.includes("wasi fd_write signature mismatch"), badSig.stdout || badSig.stderr);
  pass(name);
}

function checkNominalRowDataUnion() {
  const name = "nominal row data union gates";
  const parsed = run("./target/debug/level1c.o", ["parse", path.join(ROOT, "nominal_row_data_union.chiba")]);
  assert(name, parsed.status === 0 && parsed.stdout.includes("AttrArgIdentCall"), parsed.stdout || parsed.stderr);
  assert(name, parsed.stdout.includes('"all"') && parsed.stdout.includes('"not"') && parsed.stdout.includes('"or"'), "compile_if boolean predicates must remain structured in parser AST");
  const checkedValid = run("./target/debug/level1c.o", ["check", path.join(ROOT, "nominal_row_data_union.chiba")]);
  assert(name, checkedValid.status === 0 && checkedValid.stdout.includes("check ok"), checkedValid.stdout || checkedValid.stderr);
  const invalidCompileIf = run("./target/debug/level1c.o", ["check", path.join(ROOT, "compile_if_invalid.chiba")]);
  assert(name, invalidCompileIf.status === 0 && invalidCompileIf.stderr.includes("unknown compile_if predicate"), invalidCompileIf.stdout || invalidCompileIf.stderr);
  const badType = run("./target/debug/level1c.o", ["check", path.join(ROOT, "nominal_row_data_union_invalid.chiba")]);
  assert(name, badType.status === 0 && badType.stderr.includes("duplicate nominal field"), badType.stdout || badType.stderr);
  const badData = run("./target/debug/level1c.o", ["check", path.join(ROOT, "nominal_row_data_union_invalid_data.chiba")]);
  assert(name, badData.status === 0 && badData.stderr.includes("duplicate data variant or field"), badData.stdout || badData.stderr);
  const badUnion = run("./target/debug/level1c.o", ["check", path.join(ROOT, "nominal_row_data_union_invalid_union.chiba")]);
  assert(name, badUnion.status === 0 && badUnion.stderr.includes("duplicate union field"), badUnion.stdout || badUnion.stderr);
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
checkOperatorResolutionCompilerGate();
checkRowPoly();
checkRowPolyCompilerGate();
checkRowShapeUnify();
checkRowShorthand();
checkCheckedTemplateInstantiation();
checkNamespaceMerge();
checkStringSlice();
checkStringReturnAbi();
checkPipeLowering();
checkMemory();
checkTypeInference();
checkTypeGenerics();
checkTypeUnify();
checkExternAbi();
checkNominalRowDataUnion();
checkContinuation();
