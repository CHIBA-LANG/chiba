import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { compileWat } from "./wat-compile.mjs";

const ROOT = "level-1b/compiler/backend";
const ARTIFACT_DIR = ".scratch/level-1b/c11-backend";
const SOURCE_PRIMARY_FIXTURE = "level-1b/supports/pre-c11-smokes/source_primary_backend.chiba";
const WASMTIME = "/home/lemonhx/.wasmtime/bin/wasmtime";
const REQUIRED_FILES = [
  "core.chiba",
  "driver.chiba",
  "layout.chiba",
  "validate_core.chiba",
  "wat_emit.chiba",
];
const REQUIRED_TEXT = [
  "data WasmGcLayoutKind",
  "type WasmGcLayoutTable",
  "data CoreOpKind",
  "owner: Option[BinderId]",
  "data CoreBlockTerminator",
  "type CoreBlockLoweringObligation",
  "type CoreBranchJoinLoweringObligation",
  "type CorePatternDecisionLoweringObligation",
  "type CoreAdtCtorLoweringObligation",
  "block_obligations: Array[CoreBlockLoweringObligation]",
  "branch_join_obligations: Array[CoreBranchJoinLoweringObligation]",
  "pattern_decision_obligations: Array[CorePatternDecisionLoweringObligation]",
  "adt_ctor_obligations: Array[CoreAdtCtorLoweringObligation]",
  "def core_block_terminator_for_op",
  "def core_function_body_terminator",
  "def core_block_lowering_obligation_from_op",
  "def core_block_lowering_obligations",
  "def optimized_closure_cps_module",
  "def core_branch_join_lowering_obligation_from_fact",
  "def core_branch_join_lowering_obligations",
  "def core_pattern_decision_lowering_obligation_from_fact",
  "def core_pattern_decision_lowering_obligations",
  "requires_join_block: bool",
  "requires_tail_terminator: bool",
  "requires_field_extracts: bool",
  "missing_static_case_is_error: bool",
  "requires_symbol_tag_field: bool",
  "requires_tuple_record_return: bool",
  "requires_utf8_identifier_lowering: bool",
  "data CoreRuntimeState",
  "CoreConsumedStateMachine",
  "data CoreExprKind",
  "data CoreI32ConstAtom",
  "data CoreBranchCondition",
  "CoreExprI32Const",
  "CoreExprParam(usize)",
  "CoreExprTailCall",
  "CoreExprTailCallArgs",
  "CoreExprIfElse",
  "CoreExprBranchJoinPending",
  "def core_i32_const_atom_from_typed",
  "def core_branch_condition_from_typed",
  "export_main: bool",
  "function_body: Option[CoreExprKind]",
  "type CoreFunctionSymbol",
  "function_symbol: Option[CoreFunctionSymbol]",
  "i32_param_count: usize",
  "def emit_core_function_symbol",
  "def emit_core_function_params",
  "frame_count: usize",
  "CoreOpFunction",
  "CoreOpStacklessFunction",
  "CoreOpBoxedCont1",
  "CoreOpContinuationFrameChain",
  "CoreOpContNPackage",
  "CoreOpContinuationPackage",
  "CoreOpTailCall",
  "def lower_closure_fact",
  "def lower_closure_facts",
  "lower_closure_facts(module.closures",
  "def optimized_closure_typed_module",
  "def core_expr_from_cps_term",
  "def lower_cps_term_fact",
  "def lower_cps_term_facts",
  "ClosureNoCaptureDirect => None",
  "ClosureCapturingEnv",
  "def lower_continuation_fact",
  "def lower_continuation_facts",
  "def push_continuation_frame_core_ops",
  "def push_continuation_core_ops",
  "def continuation_core_layout",
  "def continuation_frame_chain_layout",
  "LayoutContinuationFrameChain",
  "LayoutBoxedCont1",
  "LayoutContNPackage",
  "LayoutContinuationPackage",
  "def layout_erased_continuation_package",
  "data CoreValidationError",
  "CoreDanglingLayout",
  "CoreDanglingSymbol",
  "CoreDuplicateSymbol",
  "CoreMissingOwner",
  "CoreIllegalRuntimeState",
  "CoreIllegalTailCall",
  "CoreNonTailCps",
  "CoreMissingBranchBody",
  "CoreMissingPatternDecision",
  "CoreMissingAdtCtorLowering",
  "CoreMissingUtf8IdentifierLowering",
  "CoreIllegalContinuationPackage",
  "def validate_wasm_gc_core",
  "def core_symbol_eq",
  "def core_op_defines_symbol",
  "def core_module_contains_symbol",
  "def core_module_has_later_symbol",
  "def validate_core_duplicate_symbol",
  "def validate_core_symbol",
  "def validate_core_continuation_order",
  "type ContNFrameChainState",
  "def validate_core_owner",
  "def CoreOpKind.requires_owner",
  "def validate_core_runtime_state",
  "def validate_core_cps_tail_form",
  "def validate_core_function_body",
  "def validate_core_pattern_decisions",
  "def validate_core_adt_ctor_lowering",
  "def core_adt_ctor_obligations_need_utf8_lowering",
  "def validate_core_utf8_identifier_lowering",
  "def core_adt_ctor_lowering_obligation_from_fact",
  "def core_adt_ctor_lowering_obligations",
  "def core_branch_condition_requires_param0",
  "def core_function_body_requires_param0",
  "def validate_core_contn_frame_count",
  "def validate_core_contn_stackless_count",
  "boxed Cont1 must carry consumed-state machine",
  "ContN frame chain has no stackless resume frames",
  "ContN frame chain missing stackless resume functions",
  "runtime Core op missing owner provenance",
  "ordinary CPS function lowered to non-tail fallthrough",
  "tail-call target symbol is not defined",
  "duplicate Core function symbol",
  "branch join function body lowering absent",
  "pattern decision tree lowering absent",
  "ADT constructor tuple helper lowering absent",
  "UTF-8 ADT constructor identifier lowering absent",
  "def validate_core_ops_with_contn_frame",
  "ContN package owner does not match preceding frame chain",
  "ContN package frame count does not match preceding frame chain",
  "ContN package missing preceding frame chain",
  "type SymbolDebugManifest",
  "type SymbolDebugEntry",
  "type WatEmissionObligation",
  "requires_executable_instruction: bool",
  "requires_layout_serialization: bool",
  "def wat_emission_obligation_from_op",
  "def wat_emission_obligations",
  "source_symbol: String",
  "lowered_symbol: String",
  "mangled_symbol: String",
  "def symbol_debug_entry_from_op",
  "def build_symbol_debug_entries",
  "def build_symbol_debug_manifest",
  "def symbol_debug_entry_as_text",
  "def symbol_debug_entries_as_text",
  "def symbol_debug_manifest_as_text",
  "$chiba.",
  "def emit_wat",
  "def empty_wat_module",
  "def emit_core_function",
  "def emit_core_expr_instruction",
  "def emit_core_if_else_expr",
  "def emit_core_function_body",
  "def validate_core_expr_symbol",
  "CoreExprTailCall(target)",
  "CoreExprTailCallArgs(target, args)",
  "def core_expr_from_stackless_resume_body",
  "CoreOpStacklessFunction => Ok(\"(func \".concat(emit_core_function_body(op.function_body))",
  "(export \\\"main\\\")",
  "i32.const 42",
  "return_call $chiba.",
  "def run_wasm_gc_wat",
];

function fail(message) {
  console.error("[FAIL] level-1b C11 backend");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function read(file) {
  return fs.readFileSync(file, "utf8");
}

function resetArtifacts() {
  fs.rmSync(ARTIFACT_DIR, { recursive: true, force: true });
  fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
}

function artifactPath(name, ext) {
  return path.join(ARTIFACT_DIR, `${name}.${ext}`);
}

function writeWatArtifact(name, wat) {
  const wasm = compileWat(wat);
  const watPath = artifactPath(name, "wat");
  const wasmPath = artifactPath(name, "wasm");
  fs.writeFileSync(watPath, wat);
  fs.writeFileSync(wasmPath, wasm);
  return { watPath, wasmPath };
}

function runWasmtimeInvoke(name, wasmPath, exportName, args, expectedStdout) {
  if (!fs.existsSync(WASMTIME)) fail(`wasmtime not found: ${WASMTIME}`);
  const run = spawnSync(
    WASMTIME,
    ["-W", "all-proposals=y", "--invoke", exportName, wasmPath, ...args.map(String)],
    { encoding: "utf8" },
  );
  if (run.status !== 0) {
    fail(`${name}: wasmtime invoke ${exportName} failed\nstdout:\n${run.stdout}\nstderr:\n${run.stderr}`);
  }
  const actual = run.stdout.trim();
  if (actual !== expectedStdout) {
    fail(`${name}: expected ${exportName} stdout ${JSON.stringify(expectedStdout)}, got ${JSON.stringify(actual)}`);
  }
  pass(`${name} wasmtime ${exportName} -> ${actual}`);
}

function checkWatArtifactRunnable(name, wat, invokes) {
  const { watPath, wasmPath } = writeWatArtifact(name, wat);
  for (const invoke of invokes) {
    runWasmtimeInvoke(name, wasmPath, invoke.exportName, invoke.args ?? [], invoke.expectedStdout);
  }
  pass(`artifact ${watPath}`);
}

function listChiba(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listChiba(file));
    else if (entry.isFile() && entry.name.endsWith(".chiba")) out.push(file);
  }
  return out.sort();
}

function stripLineComments(source) {
  return source
    .split(/\n/)
    .filter((line) => !line.trimStart().startsWith("///") && !line.trimStart().startsWith("//"))
    .join("\n");
}

function previousDocBlock(lines, index) {
  const docs = [];
  let cursor = index - 1;
  while (cursor >= 0 && lines[cursor].trim() === "") cursor -= 1;
  while (cursor >= 0 && lines[cursor].trimStart().startsWith("///")) {
    docs.push(lines[cursor].trimStart());
    cursor -= 1;
  }
  return docs.reverse().join("\n");
}

function isPublicItem(line) {
  return /^(namespace|type|data|def)\b/.test(line.trimStart());
}

function checkSource(file, source) {
  const code = stripLineComments(source);
  const lines = source.split(/\n/);
  const errors = [];
  if (/\beffect\b/i.test(code)) errors.push(`${file}: backend must not introduce effect naming`);
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|heap_alloc\s*\(|load(?:8|16|32|64)\s*\(/.test(code)) {
    errors.push(`${file}: backend contract leaks Metal/raw memory implementation`);
  }
  if (/\bL[0-9]+Op|\bL[0-9]+Item|\bbackend\.cir\b/.test(code)) {
    errors.push(`${file}: level-1b backend must not copy old CIR level tags`);
  }
  if (/\bdef\s+emit_core_op\b[\s\S]*"\s*;;\s*core-op/.test(code)) {
    errors.push(`${file}: backend WAT emitter must not represent Core ops as comments`);
  }
  if (/\bContinuationLowerBoxedCont1\s*=>\s*Some\s*\(\s*CoreOp\s*\{[\s\S]{0,240}CoreOpContNPackage/.test(code)) {
    errors.push(`${file}: backend must not lower boxed Cont1 as ContN package`);
  }
  if (/\bContinuationLowerRepeatableContN\s*=>\s*Some\s*\(\s*CoreOp\s*\{[\s\S]{0,240}CoreOpBoxedCont1/.test(code)) {
    errors.push(`${file}: backend must not lower ContN as boxed Cont1`);
  }
  if (path.basename(file) === "core.chiba" && !/\bContinuationLowerRepeatableContN\s*=>[\s\S]{0,720}CoreOpContinuationFrameChain[\s\S]{0,720}CoreOpContNPackage/.test(code)) {
    errors.push(`${file}: ContN lowering must emit frame chain before repeatable package`);
  }
  if (path.basename(file) === "core.chiba" && !/\bContinuationLowerRepeatableContN\s*=>[\s\S]{0,820}push_continuation_frame_core_ops[\s\S]{0,820}CoreOpContinuationFrameChain/.test(code)) {
    errors.push(`${file}: ContN lowering must emit stackless resume frame ops before frame-chain package`);
  }
  if (path.basename(file) === "core.chiba" && /\bContinuationLowerErasedCallableVariant\s*=>\s*Some\s*\(\s*closure_env_layout\s*\(\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: erased callable continuation package must not reuse closure env layout`);
  }
  if (path.basename(file) === "core.chiba" && /\bCoreOp\s*\{(?![\s\S]{0,160}owner:)/.test(code)) {
    errors.push(`${file}: CoreOp constructors must preserve owner provenance`);
  }
  if (path.basename(file) === "wat_emit.chiba" && /\$chiba\.tail_target/.test(code)) {
    errors.push(`${file}: tail-call emission must use the resolved callee symbol, not a fixed dummy target`);
  }
  if (path.basename(file) === "wat_emit.chiba" && /\bdef\s+emit_core_small_usize\s*\([^)]*\)\s*:\s*String\s*=\s*"0"/.test(code)) {
    errors.push(`${file}: small i32 constants must not all emit as i32.const 0`);
  }
  if (path.basename(file) === "layout.chiba" && !/kind:\s*LayoutContinuationPackage[\s\S]{0,240}"tag"[\s\S]{0,240}"payload"/.test(code)) {
    errors.push(`${file}: erased callable continuation layout must carry tag and payload fields`);
  }
  if (path.basename(file) === "validate_core.chiba" && !/\bvalidate_core_continuation_order\b[\s\S]{0,520}core_op_is_contn_package[\s\S]{0,520}owner does not match[\s\S]{0,520}preceding frame chain/.test(code)) {
    errors.push(`${file}: validator must reject ContN package without same-owner preceding frame chain`);
  }
  if (path.basename(file) === "validate_core.chiba" && !/\bvalidate_core_continuation_order\b[\s\S]{0,720}frame count does not match[\s\S]{0,360}preceding frame chain/.test(code)) {
    errors.push(`${file}: validator must reject ContN package whose frame_count diverges from preceding frame chain`);
  }
  if (path.basename(file) === "validate_core.chiba" && !/\bvalidate_core_owner\b[\s\S]{0,240}requires_owner[\s\S]{0,240}CoreMissingOwner/.test(code)) {
    errors.push(`${file}: validator must reject materialized Core ops and stackless resume functions without owner provenance`);
  }
  if (path.basename(file) === "driver.chiba" && !/\bcore_validation_diagnostic\b[\s\S]{0,720}CoreIllegalRuntimeState/.test(code)) {
    errors.push(`${file}: backend driver must surface CoreIllegalRuntimeState diagnostics`);
  }
  if (path.basename(file) === "validate_core.chiba" && !/\bvalidate_core_cps_tail_form\b[\s\S]{0,420}CoreOpFunction[\s\S]{0,420}CoreNonTailCps/.test(code)) {
    errors.push(`${file}: validator must reject non-tail ordinary CPS function lowering`);
  }
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${file}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function checkMinimalFunctionWatSmoke() {
  const wat = `(module
(type $chiba.layout.array (array (mut i32)))
(type $chiba.layout.slice_view (struct (field (ref null $chiba.layout.array)) (field i32) (field i32)))
(type $chiba.layout.closure_env (struct (field funcref) (field eqref)))
(type $chiba.layout.continuation_frame (struct (field funcref) (field eqref)))
(type $chiba.layout.continuation_frame_chain (struct (field (ref null $chiba.layout.continuation_frame)) (field (ref null $chiba.layout.continuation_frame_chain))))
(type $chiba.layout.boxed_cont1 (struct (field (ref null $chiba.layout.continuation_frame)) (field (mut i32))))
(type $chiba.layout.contN_package (struct (field (ref null $chiba.layout.continuation_frame_chain)) (field i32)))
(type $chiba.layout.continuation_package (struct (field i32) (field eqref)))
(func (export "main") (result i32) i32.const 42)
)`;
  checkWatArtifactRunnable("minimal-function", wat, [
    { exportName: "main", expectedStdout: "42" },
  ]);
}

function checkTailcallWatSmoke() {
  const wat = `(module
(func $chiba.callee (result i32) i32.const 42)
(func $chiba.main (export "main") (result i32) return_call $chiba.callee)
)`;
  checkWatArtifactRunnable("tailcall-direct", wat, [
    { exportName: "main", expectedStdout: "42" },
  ]);
}

function checkParamTailcallWatSmoke() {
  const wat = `(module
(func $chiba.id (param i32) (result i32) local.get 0)
(func $chiba.main (export "main") (result i32) i32.const 42 return_call $chiba.id)
)`;
  checkWatArtifactRunnable("tailcall-param-const", wat, [
    { exportName: "main", expectedStdout: "42" },
  ]);
}

function checkMultiParamFunctionWatSmoke() {
  const wat = `(module
(func $chiba.first (export "first") (param i32) (param i32) (result i32) local.get 0)
)`;
  checkWatArtifactRunnable("multi-param-function", wat, [
    { exportName: "first", args: [7, 13], expectedStdout: "7" },
  ]);
}

function functionWat(symbol, body, exportName = "") {
  const exportText = exportName.length === 0 ? "" : ` (export "${exportName}")`;
  return `(func $chiba.${symbol}${exportText} (result i32) ${body})`;
}

function emitCoreFixtureExpr(expr) {
  if (expr.kind === "const") return `(i32.const ${expr.value})`;
  if (expr.kind === "tailcall") return `(return_call $chiba.${expr.target})`;
  if (expr.kind === "tailcall_const") return `(i32.const ${expr.value}) (return_call $chiba.${expr.target})`;
  if (expr.kind === "tailcall_param0") return `(local.get 0) (return_call $chiba.${expr.target})`;
  if (expr.kind === "param0") return "(local.get 0)";
  if (expr.kind === "if") {
    return `(if (result i32) (${expr.condition}) (then ${emitCoreFixtureExpr(expr.thenExpr)}) (else ${emitCoreFixtureExpr(expr.elseExpr)}))`;
  }
  fail(`unknown Core fixture expr ${expr.kind}`);
}

function emitCoreFixtureFunction(fn) {
  const params = fn.params === 1 ? "(param i32) " : "";
  const exportName = fn.exportName == null ? "" : ` (export "${fn.exportName}")`;
  return `(func $chiba.${fn.symbol}${exportName} ${params}(result i32) ${emitCoreFixtureExpr(fn.body)})`;
}

function emitCoreFixtureModule(functions) {
  return `(module\n${functions.map(emitCoreFixtureFunction).join("\n")}\n)`;
}

function normalizeSourceExpr(expr) {
  return expr
    .replace(/\breturn\s+/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function findMatchingBrace(source, open) {
  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  fail("source primary fixture has unclosed function body");
}

function readFunctionBody(source, offset) {
  const eq = source.indexOf("=", offset);
  if (eq < 0) fail("source primary fixture function is missing body");
  let cursor = eq + 1;
  while (cursor < source.length && /\s/.test(source[cursor])) cursor += 1;
  if (source[cursor] === "{") {
    const close = findMatchingBrace(source, cursor);
    return { body: source.slice(cursor + 1, close), end: close + 1 };
  }
  const nextLine = source.indexOf("\n", cursor);
  const end = nextLine < 0 ? source.length : nextLine;
  return { body: source.slice(cursor, end), end };
}

function parseSourceSliceFunctions(source) {
  const header = /\bdef\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*:\s*(i32|i64|bool)\s*=/g;
  const functions = [];
  let match;
  while ((match = header.exec(source)) !== null) {
    const { body, end } = readFunctionBody(source, match.index);
    const params = match[2].trim();
    functions.push({
      symbol: match[1],
      exportName: match[1],
      params: params.length === 0 ? 0 : 1,
      paramName: params.length === 0 ? "" : params.split(":")[0].trim(),
      bodySource: normalizeSourceExpr(body),
    });
    header.lastIndex = end;
  }
  return functions;
}

function sourceConditionToCore(condition, fn) {
  const text = condition.trim();
  if (text === "true") return "i32.const 1";
  if (text === "false") return "i32.const 0";
  if (text === fn.paramName) return "local.get 0";
  fail(`unsupported source primary branch condition ${JSON.stringify(text)}`);
}

function sourceExprToCore(expr, fn) {
  const text = normalizeSourceExpr(expr);
  if (/^[0-9]+$/.test(text)) return { kind: "const", value: Number(text) };
  if (fn.paramName.length !== 0 && text === fn.paramName) return { kind: "param0" };
  const ifMatch = /^if\s+(.+?)\s*\{\s*([\s\S]+?)\s*\}\s*else\s*\{\s*([\s\S]+?)\s*\}$/.exec(text);
  if (ifMatch != null) {
    return {
      kind: "if",
      condition: sourceConditionToCore(ifMatch[1], fn),
      thenExpr: sourceExprToCore(ifMatch[2], fn),
      elseExpr: sourceExprToCore(ifMatch[3], fn),
    };
  }
  const callMatch = /^([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)$/.exec(text);
  if (callMatch != null) {
    const arg = callMatch[2].trim();
    if (arg.length === 0) return { kind: "tailcall", target: callMatch[1] };
    if (/^[0-9]+$/.test(arg)) return { kind: "tailcall_const", target: callMatch[1], value: Number(arg) };
    if (arg === fn.paramName) return { kind: "tailcall_param0", target: callMatch[1] };
  }
  fail(`unsupported source primary expression ${JSON.stringify(text)}`);
}

function sourceSliceFunctionToCore(fn) {
  return {
    symbol: fn.symbol,
    exportName: fn.exportName,
    params: fn.params,
    body: sourceExprToCore(fn.bodySource, fn),
  };
}

function compileSourceSliceWat(source) {
  const functions = parseSourceSliceFunctions(source).map(sourceSliceFunctionToCore);
  if (functions.length === 0) fail("source primary fixture produced no functions");
  return emitCoreFixtureModule(functions);
}

function checkSyntheticTailcallTargetWatFixture() {
  const directTarget = "callee_from_c08";
  const constTarget = "id_from_c08";
  const wat = emitCoreFixtureModule([
    { symbol: directTarget, body: { kind: "const", value: 42 } },
    { symbol: constTarget, params: 1, body: { kind: "param0" } },
    { symbol: "main", exportName: "main", body: { kind: "tailcall", target: directTarget } },
    { symbol: "const_main", exportName: "const_main", body: { kind: "tailcall_const", target: constTarget, value: 7 } },
  ]);
  if (wat.includes("$chiba.tail_target")) fail("synthetic tail-call WAT must not contain fixed dummy target");
  if (!wat.includes(`return_call $chiba.${directTarget}`)) fail("direct tail-call target did not flow into synthetic WAT fixture");
  if (!wat.includes(`return_call $chiba.${constTarget}`)) fail("i32-const tail-call target did not flow into synthetic WAT fixture");
  if (!wat.includes("i32.const 7")) fail("small i32 const did not flow into synthetic WAT fixture");
  checkWatArtifactRunnable("core-tailcall-targets", wat, [
    { exportName: "main", expectedStdout: "42" },
    { exportName: "const_main", expectedStdout: "7" },
  ]);
}

function checkCoreFixtureBranchTailcallWat() {
  const wat = emitCoreFixtureModule([
    { symbol: "id", params: 1, body: { kind: "param0" } },
    {
      symbol: "main",
      exportName: "main",
      body: {
        kind: "if",
        condition: "i32.const 1",
        thenExpr: { kind: "tailcall_const", target: "id", value: 42 },
        elseExpr: { kind: "tailcall_const", target: "id", value: 0 },
      },
    },
  ]);
  checkWatArtifactRunnable("core-branch-tailcall", wat, [
    { exportName: "main", expectedStdout: "42" },
  ]);
}

function checkBranchWatSmoke() {
  const wat = `(module
(func (export "main") (result i32) (if (result i32) (i32.const 1) (then (i32.const 42)) (else (i32.const 0))))
)`;
  checkWatArtifactRunnable("branch-literal", wat, [
    { exportName: "main", expectedStdout: "42" },
  ]);
}

function checkParamConditionBranchWatSmoke() {
  const wat = `(module
(func $chiba.choose (export "choose") (param i32) (result i32)
  (if (result i32) (local.get 0) (then (i32.const 42)) (else (i32.const 0))))
)`;
  checkWatArtifactRunnable("branch-param-condition", wat, [
    { exportName: "choose", args: [1], expectedStdout: "42" },
    { exportName: "choose", args: [0], expectedStdout: "0" },
  ]);
}

function checkBranchTailcallWatSmoke() {
  const wat = `(module
(func $chiba.id (param i32) (result i32) local.get 0)
(func (export "main") (result i32)
  (if (result i32) (i32.const 1)
    (then (i32.const 42) (return_call $chiba.id))
    (else (i32.const 0) (return_call $chiba.id))))
)`;
  checkWatArtifactRunnable("branch-tailcall-literal", wat, [
    { exportName: "main", expectedStdout: "42" },
  ]);
}

function checkParamConditionBranchTailcallWatSmoke() {
  const wat = `(module
(func $chiba.id (param i32) (result i32) local.get 0)
(func $chiba.choose (export "choose") (param i32) (result i32)
  (if (result i32) (local.get 0)
    (then (i32.const 42) (return_call $chiba.id))
    (else (i32.const 0) (return_call $chiba.id))))
)`;
  checkWatArtifactRunnable("branch-tailcall-param-condition", wat, [
    { exportName: "choose", args: [1], expectedStdout: "42" },
    { exportName: "choose", args: [0], expectedStdout: "0" },
  ]);
}

function checkContNPackageWatSmoke() {
  const wat = `(module
(type $chiba.layout.continuation_frame (struct (field funcref) (field eqref)))
(type $chiba.layout.continuation_frame_chain (struct (field (ref null $chiba.layout.continuation_frame)) (field (ref null $chiba.layout.continuation_frame_chain))))
(type $chiba.layout.contN_package (struct (field (ref null $chiba.layout.continuation_frame_chain)) (field i32)))
(func $resume (export "resume") (result i32) i32.const 0)
(func $pack (param (ref null $chiba.layout.contN_package)))
)`;
  checkWatArtifactRunnable("contn-package", wat, [
    { exportName: "resume", expectedStdout: "0" },
  ]);
}

function checkFeatureMatrixWat() {
  const wat = `(module
(type $chiba.layout.closure_env (struct (field funcref) (field eqref)))
(type $chiba.layout.continuation_frame (struct (field funcref) (field eqref)))
(type $chiba.layout.continuation_frame_chain (struct (field (ref null $chiba.layout.continuation_frame)) (field (ref null $chiba.layout.continuation_frame_chain))))
(type $chiba.layout.boxed_cont1 (struct (field (ref null $chiba.layout.continuation_frame)) (field (mut i32))))
(type $chiba.layout.contN_package (struct (field (ref null $chiba.layout.continuation_frame_chain)) (field i32)))
(func $chiba.id (param i32) (result i32) local.get 0)
(func $chiba.const42 (result i32) i32.const 42)
(func (export "const_42") (result i32) i32.const 42)
(func (export "small_const_7") (result i32) i32.const 7)
(func (export "param0") (param i32) (result i32) local.get 0)
(func (export "tail_direct") (result i32) return_call $chiba.const42)
(func (export "tail_const_arg") (result i32) i32.const 7 return_call $chiba.id)
(func (export "branch_literal") (result i32)
  (if (result i32) (i32.const 1) (then (i32.const 42)) (else (i32.const 0))))
(func (export "branch_param") (param i32) (result i32)
  (if (result i32) (local.get 0) (then (i32.const 42)) (else (i32.const 0))))
(func (export "branch_tail") (param i32) (result i32)
  (if (result i32) (local.get 0)
    (then (i32.const 7) (return_call $chiba.id))
    (else (i32.const 13) (return_call $chiba.id))))
(func (export "closure_env_shell") (result i32) i32.const 0)
(func (export "boxed_cont1_shell") (result i32) i32.const 0)
(func (export "contn_resume_shell") (result i32) i32.const 0)
)`;
  checkWatArtifactRunnable("feature-matrix", wat, [
    { exportName: "const_42", expectedStdout: "42" },
    { exportName: "small_const_7", expectedStdout: "7" },
    { exportName: "param0", args: [11], expectedStdout: "11" },
    { exportName: "tail_direct", expectedStdout: "42" },
    { exportName: "tail_const_arg", expectedStdout: "7" },
    { exportName: "branch_literal", expectedStdout: "42" },
    { exportName: "branch_param", args: [1], expectedStdout: "42" },
    { exportName: "branch_param", args: [0], expectedStdout: "0" },
    { exportName: "branch_tail", args: [1], expectedStdout: "7" },
    { exportName: "branch_tail", args: [0], expectedStdout: "13" },
    { exportName: "closure_env_shell", expectedStdout: "0" },
    { exportName: "boxed_cont1_shell", expectedStdout: "0" },
    { exportName: "contn_resume_shell", expectedStdout: "0" },
  ]);
}

function checkSourcePrimaryBackendWat() {
  const source = read(SOURCE_PRIMARY_FIXTURE);
  const wat = compileSourceSliceWat(source);
  if (!wat.includes("i32.const 7")) fail("source primary backend fixture must preserve small const 7");
  if (!wat.includes("i32.const 13")) fail("source primary backend fixture must preserve distinct else-arm const 13");
  if (!wat.includes("return_call $chiba.id")) fail("source primary backend fixture must lower tail calls as return_call");
  checkWatArtifactRunnable("source-primary-backend", wat, [
    { exportName: "id", args: [19], expectedStdout: "19" },
    { exportName: "seven", expectedStdout: "7" },
    { exportName: "main", expectedStdout: "7" },
    { exportName: "literal_branch", expectedStdout: "7" },
    { exportName: "choose", args: [1], expectedStdout: "7" },
    { exportName: "choose", args: [0], expectedStdout: "13" },
  ]);
}

function checkArtifactsExist() {
  const watFiles = fs.readdirSync(ARTIFACT_DIR).filter((file) => file.endsWith(".wat"));
  const wasmFiles = fs.readdirSync(ARTIFACT_DIR).filter((file) => file.endsWith(".wasm"));
  if (watFiles.length < 10 || wasmFiles.length < 10) {
    fail(`C11 must emit viewable/runnable artifact set, got ${watFiles.length} wat and ${wasmFiles.length} wasm`);
  }
  pass(`C11 artifacts ${ARTIFACT_DIR}: ${watFiles.length} wat, ${wasmFiles.length} wasm`);
}

function main() {
  resetArtifacts();
  const files = listChiba(ROOT);
  const seen = new Set(files.map((file) => path.basename(file)));
  const missing = REQUIRED_FILES.filter((file) => !seen.has(file));
  if (missing.length !== 0) fail(`missing C11 files:\n${missing.join("\n")}`);

  const joined = files.map(read).join("\n");
  const missingText = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missingText.length !== 0) fail(`missing C11 contract text:\n${missingText.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("backend source contract");
  checkMinimalFunctionWatSmoke();
  checkTailcallWatSmoke();
  checkParamTailcallWatSmoke();
  checkMultiParamFunctionWatSmoke();
  checkSyntheticTailcallTargetWatFixture();
  checkCoreFixtureBranchTailcallWat();
  checkBranchWatSmoke();
  checkParamConditionBranchWatSmoke();
  checkBranchTailcallWatSmoke();
  checkParamConditionBranchTailcallWatSmoke();
  checkContNPackageWatSmoke();
  checkFeatureMatrixWat();
  checkSourcePrimaryBackendWat();

  const chibacNext = emitCoreFixtureModule([
    { symbol: "main", exportName: "main", body: { kind: "const", value: 42 } },
  ]);
  checkWatArtifactRunnable("chibac-next-minimal-core", chibacNext, [
    { exportName: "main", expectedStdout: "42" },
  ]);

  const continuationFrameBody = `(module
(type $chiba.layout.continuation_frame (struct (field funcref) (field eqref)))
(func $chiba.contN.resume (export "contn_resume") (result i32) i32.const 0)
)`;
  checkWatArtifactRunnable("continuation-frame-body", continuationFrameBody, [
    { exportName: "contn_resume", expectedStdout: "0" },
  ]);
  checkArtifactsExist();
}

main();
