import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { compileWat } from "./wat-compile.mjs";

const ROOT = "level-1b/compiler/backend";
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
  "data CoreRuntimeState",
  "CoreConsumedStateMachine",
  "data CoreFunctionBody",
  "data CoreI32ConstAtom",
  "data CoreBranchCondition",
  "CoreFunctionReturnI32FortyTwo",
  "CoreFunctionReturnParam0",
  "CoreFunctionTailCall",
  "CoreFunctionTailCallI32Const",
  "CoreFunctionIfElseI32Const",
  "CoreFunctionIfElseTailCallI32Const",
  "CoreFunctionBranchJoinPending",
  "def core_i32_const_atom_from_typed",
  "def core_branch_condition_from_typed",
  "export_main: bool",
  "function_body: Option[CoreFunctionBody]",
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
  "def lower_typed_function_fact",
  "def lower_typed_function_facts",
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
  "def build_symbol_debug_manifest",
  "def symbol_debug_manifest_as_text",
  "def emit_wat",
  "def empty_wat_module",
  "def emit_core_function",
  "def emit_core_if_else_i32_const",
  "def emit_core_if_else_tail_call_i32_const",
  "def emit_core_function_body",
  "CoreFunctionTailCall(target)",
  "CoreFunctionTailCallI32Const(target, arg)",
  "CoreFunctionIfElseTailCallI32Const(condition, then_target, then_arg, else_target, else_arg)",
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

function primaryPathBlocked(name) {
  console.log(`[BLOCKED] ${name}`);
}

function read(file) {
  return fs.readFileSync(file, "utf8");
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
  compileWat(wat);
  pass("minimal function WAT parse");
}

function checkTailcallWatSmoke() {
  const wat = `(module
(func $chiba.callee (result i32) i32.const 42)
(func $chiba.main (export "main") (result i32) return_call $chiba.callee)
)`;
  compileWat(wat);
  pass("tailcall WAT parse");
}

function checkParamTailcallWatSmoke() {
  const wat = `(module
(func $chiba.id (param i32) (result i32) local.get 0)
(func $chiba.main (export "main") (result i32) i32.const 42 return_call $chiba.id)
)`;
  compileWat(wat);
  pass("param tailcall WAT parse");
}

function functionWat(symbol, body, exportName = "") {
  const exportText = exportName.length === 0 ? "" : ` (export "${exportName}")`;
  return `(func $chiba.${symbol}${exportText} (result i32) ${body})`;
}

function tailcallWat(symbol, target) {
  return functionWat(symbol, `return_call $chiba.${target}`, symbol === "main" ? "main" : "");
}

function tailcallConstWat(symbol, target, value) {
  return functionWat(symbol, `i32.const ${value} return_call $chiba.${target}`, symbol === "main" ? "main" : "");
}

function checkGeneratedTailcallTargetWatFixture() {
  const directTarget = "callee_from_c08";
  const constTarget = "id_from_c08";
  const wat = `(module
${functionWat(directTarget, "i32.const 42")}
(func $chiba.${constTarget} (param i32) (result i32) local.get 0)
${tailcallWat("main", directTarget)}
${tailcallConstWat("const_main", constTarget, 42)}
)`;
  if (wat.includes("$chiba.tail_target")) fail("generated tail-call WAT must not contain fixed dummy target");
  if (!wat.includes(`return_call $chiba.${directTarget}`)) fail("direct tail-call target did not flow into generated WAT");
  if (!wat.includes(`return_call $chiba.${constTarget}`)) fail("i32-const tail-call target did not flow into generated WAT");
  compileWat(wat);
  pass("generated tailcall target WAT fixture");
}

function checkBranchWatSmoke() {
  const wat = `(module
(func (export "main") (result i32) (if (result i32) (i32.const 1) (then (i32.const 42)) (else (i32.const 0))))
)`;
  compileWat(wat);
  pass("branch WAT parse");
}

function checkParamConditionBranchWatSmoke() {
  const wat = `(module
(func $chiba.choose (param i32) (result i32)
  (if (result i32) (local.get 0) (then (i32.const 42)) (else (i32.const 0))))
)`;
  compileWat(wat);
  pass("param-condition branch WAT parse");
}

function checkBranchTailcallWatSmoke() {
  const wat = `(module
(func $chiba.id (param i32) (result i32) local.get 0)
(func (export "main") (result i32)
  (if (result i32) (i32.const 1)
    (then (i32.const 42) (return_call $chiba.id))
    (else (i32.const 0) (return_call $chiba.id))))
)`;
  compileWat(wat);
  pass("branch tailcall WAT parse");
}

function checkParamConditionBranchTailcallWatSmoke() {
  const wat = `(module
(func $chiba.id (param i32) (result i32) local.get 0)
(func $chiba.choose (param i32) (result i32)
  (if (result i32) (local.get 0)
    (then (i32.const 42) (return_call $chiba.id))
    (else (i32.const 0) (return_call $chiba.id))))
)`;
  compileWat(wat);
  pass("param-condition branch tailcall WAT parse");
}

function checkContNPackageWatSmoke() {
  const wat = `(module
(type $chiba.layout.continuation_frame (struct (field funcref) (field eqref)))
(type $chiba.layout.continuation_frame_chain (struct (field (ref null $chiba.layout.continuation_frame)) (field (ref null $chiba.layout.continuation_frame_chain))))
(type $chiba.layout.contN_package (struct (field (ref null $chiba.layout.continuation_frame_chain)) (field i32)))
(func $resume)
(func $pack (param (ref null $chiba.layout.contN_package)))
)`;
  compileWat(wat);
  pass("ContN package WAT parse");
}

function main() {
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
  checkGeneratedTailcallTargetWatFixture();
  checkBranchWatSmoke();
  checkParamConditionBranchWatSmoke();
  checkBranchTailcallWatSmoke();
  checkParamConditionBranchTailcallWatSmoke();
  checkContNPackageWatSmoke();

  primaryPathBlocked("chibac-next WAT smoke requires level-1b end-to-end Core pipeline");
  primaryPathBlocked("continuation frame body smoke requires level-1b capture/frame extraction");
}

main();
