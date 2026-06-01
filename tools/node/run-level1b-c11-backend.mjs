import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { compileWat } from "./wat-compile.mjs";

const ROOT = "level-1b/compiler/backend";
const ARTIFACT_DIR = ".scratch/level-1b/c11-backend";
const SOURCE_PRIMARY_FIXTURE = "level-1b/supports/pre-c11-smokes/source_primary_backend.chiba";
const CHIBACC_EVIDENCE = ".scratch/level-1b/chibacc-mini/ast-primary-evidence.json";
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
  "type CoreIntrinsicBridgeLoweringObligation",
  "block_obligations: Array[CoreBlockLoweringObligation]",
  "branch_join_obligations: Array[CoreBranchJoinLoweringObligation]",
  "pattern_decision_obligations: Array[CorePatternDecisionLoweringObligation]",
  "adt_ctor_obligations: Array[CoreAdtCtorLoweringObligation]",
  "intrinsic_bridge_obligations: Array[CoreIntrinsicBridgeLoweringObligation]",
  "intrinsic_bridge_roundtrips: Array[IntrinsicBridgeRoundtripFact]",
  "has_typed_roundtrip: bool",
  "emits_identity_conversion_helpers: bool",
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
  "tuple_nominal: TupleNominalIdentity",
  "requires_utf8_identifier_lowering: bool",
  "CoreInvalidTupleNominalIdentity",
  "CoreInvalidIntrinsicBridge",
  "def core_tuple_nominal_identity_valid",
  "def validate_core_adt_ctor_tuple_nominal",
  "def validate_core_adt_ctor_tuple_nominals",
  "def core_intrinsic_bridge_lowering_obligation_from_fact",
  "def core_intrinsic_bridge_has_roundtrip",
  "def core_intrinsic_bridge_mark_roundtrip",
  "def core_intrinsic_bridge_lowering_obligations",
  "def validate_core_intrinsic_bridge_obligation",
  "def validate_core_intrinsic_bridges",
  "data CoreRuntimeState",
  "CoreConsumedStateMachine",
  "data CoreExprKind",
  "data CoreI32ConstAtom",
  "data CoreBranchCondition",
  "data CoreValueType",
  "CoreExprI32Const",
  "CoreExprParam(usize)",
  "CoreExprTypedParam",
  "CoreExprLocal",
  "CoreExprStringLiteral",
  "CoreExprCast",
  "CoreExprStructNew",
  "CoreExprFieldGet",
  "CoreExprCall",
  "CoreExprTailCall",
  "CoreExprTailCallArgs",
  "CoreExprPrimitiveBinary",
  "CoreExprIfElse",
  "CoreExprBranchJoinPending",
  "def core_i32_const_atom_from_typed",
  "def core_value_types_from_typed",
  "def core_branch_condition_from_typed",
  "export_main: bool",
  "function_body: Option[CoreExprKind]",
  "type CoreFunctionSymbol",
  "function_symbol: Option[CoreFunctionSymbol]",
  "i32_param_count: usize",
  "param_types: Array[CoreValueType]",
  "def emit_core_function_symbol",
  "def emit_core_typed_function_params",
  "def emit_core_function_params",
  "def emit_core_op_function_params",
  "frame_count: usize",
  "frame_index: usize",
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
  "lower_cps_term_facts(cps_module.terms, optimized_closure_typed_module(module).functions",
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
  "CoreOpAdtTupleCtor",
  "def lower_adt_ctor_helper_op",
  "def lower_adt_ctor_helper_ops",
  "def emit_adt_tuple_ctor_function",
  "def emit_tuple_nominal_type_label",
  "def append_tuple_nominal_types",
  "prior_tuple_nominal_type_emitted",
  "def emit_intrinsic_bridge_identity_function",
  "def emit_intrinsic_bridge_functions",
  "def append_intrinsic_bridge_helpers",
  "def core_adt_ctor_obligations_need_utf8_lowering",
  "def validate_core_utf8_identifier_lowering",
  "def core_adt_ctor_lowering_obligation_from_fact",
  "def core_adt_ctor_lowering_obligations",
  "def core_branch_condition_reads_param",
  "def core_function_body_reads_param",
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
  "adt.tuple_ctor",
  "UTF-8 ADT constructor identifier lowering absent",
  "compiler intrinsic bridge missing typed roundtrip identity",
  "compiler intrinsic bridge missing executable identity conversion helpers",
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
  "def emit_core_stackless_function",
  "def emit_core_owner_suffix",
  "def emit_contn_resume_function_name",
  "def emit_contn_resume_frame_function_name",
  "def emit_contn_frame_chain_function_name",
  "def emit_contn_package_function_name",
  "def emit_core_expr_instruction",
  "def emit_core_if_else_instruction",
  "def emit_core_if_else_body",
  "def emit_core_function_body",
  "def validate_core_expr_symbol",
  "def core_expr_args_from_cps_terms",
  "CoreExprTailCall(target)",
  "CoreExprTailCallArgs(target, args)",
  "CoreExprPrimitiveBinary(op, left, right)",
  "def core_expr_from_stackless_resume_body",
  "def core_expr_from_stackless_resume_frame_body",
  "CoreOpStacklessFunction => Ok(emit_core_stackless_function(ops, op))",
  "def emit_boxed_cont1_resume_function",
  "def emit_contn_frame_chain_function",
  "def emit_contn_package_function",
  "def emit_erased_continuation_package_function",
  "def emit_closure_call_function",
  "def emit_struct_new_function",
  "def emit_array_new_function",
  "(export \\\"main\\\")",
  "CoreI32ConstLiteral(value) => \"i32.const \".concat(emit_core_small_usize(value))",
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

function readJson(file) {
  try {
    return JSON.parse(read(file));
  } catch (error) {
    fail(`missing or invalid JSON evidence ${file}: ${error.message}`);
  }
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

function runWasmtimeInvokeTrap(name, wasmPath, exportName, args, expectedStderr) {
  if (!fs.existsSync(WASMTIME)) fail(`wasmtime not found: ${WASMTIME}`);
  const run = spawnSync(
    WASMTIME,
    ["-W", "all-proposals=y", "--invoke", exportName, wasmPath, ...args.map(String)],
    { encoding: "utf8" },
  );
  if (run.status === 0) {
    fail(`${name}: expected ${exportName} to trap, got stdout:\n${run.stdout}`);
  }
  if (!run.stderr.includes(expectedStderr)) {
    fail(`${name}: expected ${exportName} trap stderr to include ${JSON.stringify(expectedStderr)}\nstderr:\n${run.stderr}`);
  }
  pass(`${name} wasmtime ${exportName} trapped`);
}

function checkWatArtifactRunnable(name, wat, invokes) {
  const { watPath, wasmPath } = writeWatArtifact(name, wat);
  for (const invoke of invokes) {
    runWasmtimeInvoke(name, wasmPath, invoke.exportName, invoke.args ?? [], invoke.expectedStdout);
  }
  pass(`artifact ${watPath}`);
}

function checkAstPrimaryWatShape(wat) {
  if (!wat.includes("(func $chiba.demo::main (export \"main\") (result i32) (i32.const 2) (i32.const 3) (i32.const 4) i32.mul i32.add)")) {
    fail("AST primary main must lower parser-owned binary nodes directly to primitive arithmetic");
  }
  if (!wat.includes("(func $chiba.demo::neg (export \"neg\") (result i32) (i32.const 0) (i32.const 7) i32.sub)")) {
    fail("AST primary prefix neg must lower directly to primitive subtraction");
  }
  const mainStart = wat.indexOf("(func $chiba.demo::main ");
  const branchStart = wat.indexOf("(func $chiba.demo::branch ");
  const mainBody = mainStart >= 0 && branchStart > mainStart ? wat.slice(mainStart, branchStart) : "";
  if (mainBody.includes("return_call $chiba.i32.op_") || mainBody.includes("call $chiba.i32.op_")) {
    fail("AST primary arithmetic body must not route through i32.op_* helper calls");
  }
}

function checkWatArtifactRunnableAndTraps(name, wat, invokes, traps) {
  const { watPath, wasmPath } = writeWatArtifact(name, wat);
  for (const invoke of invokes) {
    runWasmtimeInvoke(name, wasmPath, invoke.exportName, invoke.args ?? [], invoke.expectedStdout);
  }
  for (const trap of traps) {
    runWasmtimeInvokeTrap(name, wasmPath, trap.exportName, trap.args ?? [], trap.expectedStderr);
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
  if (path.basename(file) === "core.chiba" && /StacklessResumeBodyShiftN\s*=>\s*CoreExprI32Const\s*\(\s*CoreI32ConstLiteral\s*\(\s*0\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: unknown ContN stackless resume body must stay pending, not compile to i32.const 0`);
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
  if (path.basename(file) === "wat_emit.chiba" && /\bCoreOpBoxedCont1\s*=>\s*Ok\s*\(\s*"\(func[\s\S]{0,120}i32\.const 0/.test(code)) {
    errors.push(`${file}: boxed Cont1 WAT must enforce consumed-state, not return a constant`);
  }
  if (path.basename(file) === "wat_emit.chiba" && !/\bdef\s+emit_boxed_cont1_resume_function\b[\s\S]{0,920}struct\.get \$chiba\.layout\.boxed_cont1 1[\s\S]{0,920}unreachable[\s\S]{0,920}struct\.set \$chiba\.layout\.boxed_cont1 1/.test(code)) {
    errors.push(`${file}: boxed Cont1 WAT must read consumed state, trap on repeat, then mark consumed`);
  }
  if (path.basename(file) === "wat_emit.chiba" && /\bCoreOpContinuationFrameChain\s*=>\s*Ok\s*\(\s*"\(func \(param/.test(code)) {
    errors.push(`${file}: ContN frame-chain WAT must allocate a frame-chain object, not emit an empty param stub`);
  }
  if (path.basename(file) === "wat_emit.chiba" && !/\bdef\s+emit_contn_frame_chain_nodes\b[\s\S]{0,620}remaining\s*-\s*1[\s\S]{0,620}struct\.new \$chiba\.layout\.continuation_frame_chain/.test(code)) {
    errors.push(`${file}: ContN frame-chain WAT must materialize one spine node per Core frame_count`);
  }
  if (path.basename(file) === "wat_emit.chiba" && !/\bdef\s+emit_contn_resume_frame_function_name\b[\s\S]{0,260}frame_index/.test(code)) {
    errors.push(`${file}: ContN stackless resume names must include frame index for multi-frame packages`);
  }
  if (path.basename(file) === "wat_emit.chiba" && !/\bemit_core_stackless_function\b[\s\S]{0,420}emit_contn_resume_frame_function_name\s*\(\s*op\.owner\s*,\s*op\.frame_index\s*\)/.test(code)) {
    errors.push(`${file}: stackless resume emission must use Core frame_index, not owner-only names`);
  }
  if (path.basename(file) === "core.chiba" && !/\bpush_continuation_frame_core_ops\b[\s\S]{0,900}frame_index:\s*index/.test(code)) {
    errors.push(`${file}: Core stackless resume ops must preserve per-frame index provenance`);
  }
  if (path.basename(file) === "wat_emit.chiba" && /\bCoreOpContNPackage\s*=>\s*Ok\s*\(\s*"\(func \(param/.test(code)) {
    errors.push(`${file}: ContN package WAT must allocate a repeatable package object, not emit an empty param stub`);
  }
  if (path.basename(file) === "wat_emit.chiba" && /\bCoreOpContinuationPackage\s*=>\s*Ok\s*\(\s*"\(func \(param/.test(code)) {
    errors.push(`${file}: erased continuation package WAT must allocate tag/payload object, not emit an empty param stub`);
  }
  if (path.basename(file) === "wat_emit.chiba" && !/\bdef\s+emit_contn_package_function\b[\s\S]{0,360}struct\.new \$chiba\.layout\.contN_package/.test(code)) {
    errors.push(`${file}: ContN package WAT must materialize the package layout`);
  }
  if (path.basename(file) === "wat_emit.chiba" && !/\bdef\s+emit_erased_continuation_package_function\b[\s\S]{0,360}struct\.new \$chiba\.layout\.continuation_package/.test(code)) {
    errors.push(`${file}: erased continuation package WAT must materialize tag/payload layout`);
  }
  if (path.basename(file) === "wat_emit.chiba" && /\bCoreOpClosureCall\s*=>\s*Ok\s*\(\s*"\(func \(param funcref\) \(param eqref\)\)\\n"/.test(code)) {
    errors.push(`${file}: closure call WAT must dispatch through typed funcref, not emit an empty call stub`);
  }
  if (path.basename(file) === "wat_emit.chiba" && !/\bdef\s+emit_closure_call_function\b[\s\S]{0,420}call_ref \$chiba\.closure\.i32_to_i32/.test(code)) {
    errors.push(`${file}: closure call WAT must perform a typed call_ref dispatch`);
  }
  if (path.basename(file) === "wat_emit.chiba" && /\bCoreOp(?:StructNew|ArrayNew)\s*=>\s*Ok\s*\(\s*"\(func \(result eqref\) ref\.null eq/.test(code)) {
    errors.push(`${file}: heap allocation ops must materialize objects, not return ref.null eq`);
  }
  if (path.basename(file) === "wat_emit.chiba" && /\bCoreOpTailCall\s*=>\s*Ok\s*\(\s*"\(func \(result i32\) i32\.const 0 return/.test(code)) {
    errors.push(`${file}: standalone CoreOpTailCall must not fake success with i32.const 0`);
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
  checkWatArtifactRunnable("constant-function", wat, [
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

function emitCoreFixtureCondition(condition) {
  if (condition != null && typeof condition === "object") return emitCoreFixtureValueExpr(condition);
  const text = condition.trim();
  return text.startsWith("(") ? text : `(${text})`;
}

function emitCoreFixtureExpr(expr) {
  if (expr.kind === "const") return `(i32.const ${expr.value})`;
  if (expr.kind === "global") return `(global.get $chiba.global.${expr.name})`;
  if (expr.kind === "tailcall") return `(return_call $chiba.${expr.target})`;
  if (expr.kind === "tailcall_const") return `(i32.const ${expr.value}) (return_call $chiba.${expr.target})`;
  if (expr.kind === "tailcall_param0") return `(local.get 0) (return_call $chiba.${expr.target})`;
  if (expr.kind === "tailcall_args") return `${expr.args.map(emitCoreFixtureValueExpr).join(" ")} (return_call $chiba.${expr.target})`;
  if (expr.kind === "param0") return "(local.get 0)";
  if (expr.kind === "param") return `(local.get ${expr.index})`;
  if (expr.kind === "tuple_field") return emitCoreFixtureValueExpr(expr);
  if (expr.kind === "binary") return `${emitCoreFixtureValueExpr(expr.left)} ${emitCoreFixtureValueExpr(expr.right)} ${expr.instruction}`;
  if (expr.kind === "if") {
    return `(if (result i32) ${emitCoreFixtureCondition(expr.condition)} (then ${emitCoreFixtureExpr(expr.thenExpr)}) (else ${emitCoreFixtureExpr(expr.elseExpr)}))`;
  }
  fail(`unknown Core fixture expr ${expr.kind}`);
}

function emitCoreFixtureValueExpr(expr) {
  if (expr.kind === "const") return `(i32.const ${expr.value})`;
  if (expr.kind === "global") return `(global.get $chiba.global.${expr.name})`;
  if (expr.kind === "param0") return "(local.get 0)";
  if (expr.kind === "param") return `(local.get ${expr.index})`;
  if (expr.kind === "tuple_field") return emitCoreFixtureValueExpr(expr.values[expr.index]);
  if (expr.kind === "tailcall") return `(call $chiba.${expr.target})`;
  if (expr.kind === "tailcall_const") return `(i32.const ${expr.value}) (call $chiba.${expr.target})`;
  if (expr.kind === "tailcall_param0") return `(local.get 0) (call $chiba.${expr.target})`;
  if (expr.kind === "tailcall_args") return `${expr.args.map(emitCoreFixtureValueExpr).join(" ")} (call $chiba.${expr.target})`;
  if (expr.kind === "binary") return `${emitCoreFixtureValueExpr(expr.left)} ${emitCoreFixtureValueExpr(expr.right)} ${expr.instruction}`;
  if (expr.kind === "if") {
    return `(if (result i32) ${emitCoreFixtureCondition(expr.condition)} (then ${emitCoreFixtureExpr(expr.thenExpr)}) (else ${emitCoreFixtureExpr(expr.elseExpr)}))`;
  }
  fail(`unknown Core fixture value expr ${expr.kind}`);
}

function emitCoreFixtureFunction(fn) {
  const paramCount = Array.isArray(fn.params) ? fn.params.length : (fn.params ?? 0);
  const params = "(param i32) ".repeat(paramCount);
  const exportName = fn.exportName == null ? "" : ` (export "${fn.exportName}")`;
  return `(func $chiba.${fn.symbol}${exportName} ${params}(result i32) ${emitCoreFixtureExpr(fn.body)})`;
}

function emitCoreFixtureGlobal(global) {
  return `(global $chiba.global.${global.name} (mut i32) (i32.const 0))`;
}

function emitCoreFixtureGlobalInit(global) {
  return `${emitCoreFixtureValueExpr(global.body)}\n(global.set $chiba.global.${global.name})`;
}

function emitCoreFixtureStart(globals) {
  if (globals.length === 0) return "";
  return `(func $chiba.__module_init\n${globals.map(emitCoreFixtureGlobalInit).join("\n")}\n)\n(start $chiba.__module_init)`;
}

function emitCoreFixtureModule(input) {
  const functions = Array.isArray(input) ? input : input.functions;
  const globals = Array.isArray(input) ? [] : input.globals;
  return `(module\n${globals.map(emitCoreFixtureGlobal).join("\n")}\n${emitCoreFixtureStart(globals)}\n${functions.map(emitCoreFixtureFunction).join("\n")}\n)`;
}

function astExprNodes(full) {
  const nodes = Array.isArray(full.astExprNodes) ? full.astExprNodes : [];
  if (nodes.length === 0) fail("AST primary typed WAT requires direct AST expression nodes");
  return nodes;
}

function astOwnerParts(ownerOrName, fallbackNamespace = "demo") {
  if (typeof ownerOrName === "object" && ownerOrName != null) return ownerOrName;
  if (typeof ownerOrName === "string" && ownerOrName.includes("::")) {
    const [namespace, name] = ownerOrName.split("::");
    return { namespace, name };
  }
  return { namespace: fallbackNamespace, name: ownerOrName };
}

function astExprNode(nodes, ownerOrName, nodeId) {
  const owner = astOwnerParts(ownerOrName);
  const node = nodes.find((item) => item.ownerNamespace === owner.namespace && item.ownerName === owner.name && item.nodeId === nodeId);
  if (node == null) fail(`missing AST expression node ${owner.namespace}::${owner.name}#${nodeId}`);
  return node;
}

function astBinaryInstruction(value) {
  if (value === 0) return "i32.add";
  if (value === 1) return "i32.sub";
  if (value === 2) return "i32.mul";
  if (value === 3) return "i32.div_s";
  fail(`unsupported AST primitive binary ordinal ${value}`);
}

function coreExprFromAstNode(nodes, ownerName, nodeId) {
  const owner = astOwnerParts(ownerName);
  const node = astExprNode(nodes, owner, nodeId);
  if (node.kind === "SourceAstExprNodeI32Const") return { kind: "const", value: node.value };
  if (node.kind === "SourceAstExprNodeParam") return { kind: "param", index: node.paramIndex };
  if (node.kind === "SourceAstExprNodePrefixNeg") {
    return {
      kind: "binary",
      instruction: "i32.sub",
      left: { kind: "const", value: 0 },
      right: coreExprFromAstNode(nodes, owner, node.left),
    };
  }
  if (node.kind === "SourceAstExprNodeBinary") {
    return {
      kind: "binary",
      instruction: astBinaryInstruction(node.value),
      left: coreExprFromAstNode(nodes, owner, node.left),
      right: coreExprFromAstNode(nodes, owner, node.right),
    };
  }
  if (node.kind === "SourceAstExprNodeIfElse") {
    return {
      kind: "if",
      condition: astIfCondition(nodes, owner, node),
      thenExpr: coreExprFromAstNode(nodes, owner, node.thenNode),
      elseExpr: coreExprFromAstNode(nodes, owner, node.elseNode),
    };
  }
  if (node.kind === "SourceAstExprNodeMatch") {
    return {
      kind: "if",
      condition: astConditionNode(nodes, owner, node.left),
      thenExpr: coreExprFromAstNode(nodes, owner, node.thenNode),
      elseExpr: coreExprFromAstNode(nodes, owner, node.elseNode),
    };
  }
  if (node.kind === "SourceAstExprNodeCall") {
    const args = astNodeArgs(nodes, owner, node);
    if (args.length === 0) return { kind: "tailcall", target: astNodeTarget(node) };
    return { kind: "tailcall_args", target: astNodeTarget(node), args };
  }
  if (node.kind === "SourceAstExprNodeMethodCall") {
    return {
      kind: "tailcall_args",
      target: astNodeTarget(node),
      args: [coreExprFromAstNode(nodes, owner, node.left), ...astNodeArgs(nodes, owner, node)],
    };
  }
  if (node.kind === "SourceAstExprNodeIndex") {
    if (typeof node.callee !== "string" || node.callee.length === 0) {
      fail("AST primary index node must carry resolved callee; backend fixture must not invent i32.op_index");
    }
    return {
      kind: "tailcall_args",
      target: astNodeTarget(node),
      args: [coreExprFromAstNode(nodes, owner, node.left), ...astNodeArgs(nodes, owner, node)],
    };
  }
  if (node.kind === "SourceAstExprNodeIndexSlice") {
    if (typeof node.callee !== "string" || node.callee.length === 0) {
      fail("AST primary index-slice node must carry resolved callee; backend fixture must not invent i32.op_index_slice");
    }
    return {
      kind: "tailcall_args",
      target: astNodeTarget(node),
      args: [coreExprFromAstNode(nodes, owner, node.left), ...astNodeArgs(nodes, owner, node)],
    };
  }
  if (node.kind === "SourceAstExprNodeStructNew") {
    return {
      kind: "tuple_value",
      values: astNodeArgs(nodes, owner, node),
    };
  }
  if (node.kind === "SourceAstExprNodeFieldGet") {
    return {
      kind: "tuple_field",
      values: coreExprFromAstNode(nodes, owner, node.left).values,
      index: node.paramIndex,
    };
  }
  fail(`unsupported AST expression node kind ${node.kind}`);
}

function astConditionNode(nodes, owner, nodeId) {
  const node = astExprNode(nodes, owner, nodeId);
  if (node.kind === "SourceAstExprNodeI32Const") return { kind: "const", value: node.value === 0 ? 0 : 1 };
  if (node.kind === "SourceAstExprNodeParam") return { kind: "param", index: node.paramIndex };
  fail(`unsupported AST condition node kind ${node.kind}`);
}

function astIfCondition(nodes, owner, node) {
  if ((node.left ?? 0) === 0) return { kind: "const", value: 1 };
  return astConditionNode(nodes, owner, node.left);
}

function astNodeTarget(node, fallback = "") {
  return typeof node.callee === "string" && node.callee.length > 0 ? node.callee : fallback;
}

function astNodeArgCount(node) {
  return node.argCount ?? node.arg_count ?? 0;
}

function astNodeArgs(nodes, ownerName, node) {
  const ids = Array.isArray(node.args)
    ? node.args
    : [node.arg0, node.arg1, node.arg2].slice(0, astNodeArgCount(node));
  return ids.map((id) => coreExprFromAstNode(nodes, ownerName, id));
}

function astPrimaryBuiltinIntrinsicFunctions() {
  return [
    { symbol: "i32.op_add", body: { kind: "binary", instruction: "i32.add", left: { kind: "param", index: 0 }, right: { kind: "param", index: 1 } }, params: 2 },
    { symbol: "i32.op_sub", body: { kind: "binary", instruction: "i32.sub", left: { kind: "param", index: 0 }, right: { kind: "param", index: 1 } }, params: 2 },
    { symbol: "i32.op_mul", body: { kind: "binary", instruction: "i32.mul", left: { kind: "param", index: 0 }, right: { kind: "param", index: 1 } }, params: 2 },
    { symbol: "i32.op_div", body: { kind: "binary", instruction: "i32.div_s", left: { kind: "param", index: 0 }, right: { kind: "param", index: 1 } }, params: 2 },
    { symbol: "i32.op_index", body: { kind: "binary", instruction: "i32.add", left: { kind: "param", index: 0 }, right: { kind: "param", index: 1 } }, params: 2 },
    {
      symbol: "i32.op_index_slice",
      body: {
        kind: "binary",
        instruction: "i32.add",
        left: { kind: "binary", instruction: "i32.add", left: { kind: "param", index: 0 }, right: { kind: "param", index: 1 } },
        right: { kind: "param", index: 2 },
      },
      params: 3,
    },
  ];
}

function normalizeSourceExpr(expr) {
  return expr
    .replace(/\breturn\s+/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function splitTopLevel(text, delimiter) {
  const out = [];
  let start = 0;
  let paren = 0;
  let brace = 0;
  let bracket = 0;
  for (let index = 0; index < text.length; index += 1) {
    const ch = text[index];
    if (ch === "(") paren += 1;
    else if (ch === ")") paren -= 1;
    else if (ch === "{") brace += 1;
    else if (ch === "}") brace -= 1;
    else if (ch === "[") bracket += 1;
    else if (ch === "]") bracket -= 1;
    else if (paren === 0 && brace === 0 && bracket === 0 && text.startsWith(delimiter, index)) {
      out.push(text.slice(start, index).trim());
      start = index + delimiter.length;
      index += delimiter.length - 1;
    }
  }
  out.push(text.slice(start).trim());
  return out;
}

function findTopLevel(text, delimiter) {
  let paren = 0;
  let brace = 0;
  let bracket = 0;
  for (let index = 0; index < text.length; index += 1) {
    const ch = text[index];
    if (ch === "(") paren += 1;
    else if (ch === ")") paren -= 1;
    else if (ch === "{") brace += 1;
    else if (ch === "}") brace -= 1;
    else if (ch === "[") bracket += 1;
    else if (ch === "]") bracket -= 1;
    else if (paren === 0 && brace === 0 && bracket === 0 && text.startsWith(delimiter, index)) return index;
  }
  return -1;
}

function findTopLevelOpenBrace(text) {
  let paren = 0;
  let bracket = 0;
  for (let index = 0; index < text.length; index += 1) {
    const ch = text[index];
    if (ch === "(") paren += 1;
    else if (ch === ")") paren -= 1;
    else if (ch === "[") bracket += 1;
    else if (ch === "]") bracket -= 1;
    else if (paren === 0 && bracket === 0 && ch === "{") return index;
  }
  return -1;
}

function findTopLevelAnyRightmost(text, operators) {
  let paren = 0;
  let brace = 0;
  let bracket = 0;
  let found = -1;
  for (let index = 0; index < text.length; index += 1) {
    const ch = text[index];
    if (ch === "(") paren += 1;
    else if (ch === ")") paren -= 1;
    else if (ch === "{") brace += 1;
    else if (ch === "}") brace -= 1;
    else if (ch === "[") bracket += 1;
    else if (ch === "]") bracket -= 1;
    else if (paren === 0 && brace === 0 && bracket === 0 && operators.includes(ch)) found = index;
  }
  return found;
}

function findOuterIndexOpen(text) {
  let paren = 0;
  let brace = 0;
  for (let index = 0; index < text.length; index += 1) {
    const ch = text[index];
    if (ch === "(") paren += 1;
    else if (ch === ")") paren -= 1;
    else if (ch === "{") brace += 1;
    else if (ch === "}") brace -= 1;
    else if (paren === 0 && brace === 0 && ch === "[") return index;
  }
  return -1;
}

function findMatchingBracket(text, open) {
  let depth = 0;
  for (let index = open; index < text.length; index += 1) {
    const ch = text[index];
    if (ch === "[") depth += 1;
    else if (ch === "]") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return -1;
}

function operatorMethod(ch) {
  if (ch === "+") return "op_add";
  if (ch === "-") return "op_sub";
  if (ch === "*") return "op_mul";
  if (ch === "/") return "op_div";
  fail(`unsupported source primary operator ${ch}`);
}

function operatorInstruction(ch) {
  if (ch === "+") return "i32.add";
  if (ch === "-") return "i32.sub";
  if (ch === "*") return "i32.mul";
  if (ch === "/") return "i32.div_s";
  fail(`unsupported source primary operator ${ch}`);
}

function hasLeftOperand(text, index) {
  const left = text.slice(0, index).trimEnd();
  if (left.length === 0) return false;
  return !["(", "[", "{", ",", ":", "=", "+", "-", "*", "/"].includes(left[left.length - 1]);
}

function findTopLevelBinaryRightmost(text, operators) {
  let paren = 0;
  let brace = 0;
  let bracket = 0;
  let found = -1;
  for (let index = 0; index < text.length; index += 1) {
    const ch = text[index];
    if (ch === "(") paren += 1;
    else if (ch === ")") paren -= 1;
    else if (ch === "{") brace += 1;
    else if (ch === "}") brace -= 1;
    else if (ch === "[") bracket += 1;
    else if (ch === "]") bracket -= 1;
    else if (paren === 0 && brace === 0 && bracket === 0 && operators.includes(ch)) {
      if (ch !== "-" || hasLeftOperand(text, index)) found = index;
    }
  }
  return found;
}

function parseParams(params) {
  const text = params.trim();
  if (text.length === 0) return [];
  return splitTopLevel(text, ",").map((param, index) => {
    const match = /^([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([A-Za-z_][A-Za-z0-9_.]*)$/.exec(param);
    if (match == null) fail(`unsupported source primary param ${JSON.stringify(param)}`);
    return { name: match[1], type: match[2], index };
  });
}

function paramByName(fn, name) {
  return fn.params.find((param) => param.name === name) ?? null;
}

function paramTypeByName(fn, name) {
  const param = paramByName(fn, name);
  return param == null ? null : param.type;
}

function globalByName(ctx, name) {
  return ctx.globals.find((global) => global.name === name) ?? null;
}

function parseCallArgs(argText, fn) {
  const text = argText.trim();
  if (text.length === 0) return [];
  return splitTopLevel(text, ",").map((arg) => sourceExprToCore(arg, fn));
}

function sourceTupleFieldToCore(text, fn) {
  const match = /^\((.*)\)\._([1-9][0-9]*)$/.exec(text);
  if (match == null) return null;
  const values = splitTopLevel(match[1], ",").map((item) => sourceExprToCore(item, fn));
  const field = Number(match[2]) - 1;
  if (field < 0 || field >= values.length) fail(`source primary tuple field out of range ${JSON.stringify(text)}`);
  return { kind: "tuple_field", values, index: field };
}

function sourceIndexExprToCore(text, fn) {
  const open = findOuterIndexOpen(text);
  if (open < 0) return null;
  const close = findMatchingBracket(text, open);
  if (close < 0 || text.slice(close + 1).trim().length !== 0) return null;
  const receiverText = text.slice(0, open).trim();
  const receiverType = paramTypeByName(fn, receiverText);
  if (receiverType == null) return null;
  const inner = text.slice(open + 1, close).trim();
  const range = findTopLevel(inner, "..");
  if (range >= 0) {
    return {
      kind: "tailcall_args",
      target: `${receiverType}.op_index_slice`,
      args: [
        sourceExprToCore(receiverText, fn),
        sourceExprToCore(inner.slice(0, range), fn),
        sourceExprToCore(inner.slice(range + 2), fn),
      ],
    };
  }
  return {
    kind: "tailcall_args",
    target: `${receiverType}.op_index`,
    args: [sourceExprToCore(receiverText, fn), sourceExprToCore(inner, fn)],
  };
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
  const bodyOpen = source.indexOf("{", cursor);
  const nextDef = source.indexOf("\ndef ", cursor);
  const nextDoc = source.indexOf("\n///", cursor);
  const nextItem = [nextDef, nextDoc].filter((index) => index >= 0).reduce((left, right) => Math.min(left, right), source.length);
  if (bodyOpen >= 0 && bodyOpen < nextItem) {
    const close = findMatchingBrace(source, bodyOpen);
    if (nextLine >= 0 && close > nextLine) {
      return { body: source.slice(cursor, close + 1), end: close + 1 };
    }
  }
  const end = nextLine < 0 ? source.length : nextLine;
  return { body: source.slice(cursor, end), end };
}

function parseSourceSliceFunctions(source) {
  const header = /\bdef\s+([A-Za-z_][A-Za-z0-9_.]*)\s*\(([^)]*)\)\s*:\s*(i32|i64|bool)\s*=/g;
  const functions = [];
  let match;
  while ((match = header.exec(source)) !== null) {
    const { body, end } = readFunctionBody(source, match.index);
    const params = parseParams(match[2]);
    functions.push({
      symbol: match[1],
      exportName: match[1],
      params,
      bodySource: normalizeSourceExpr(body),
    });
    header.lastIndex = end;
  }
  return functions;
}

function parseSourceSliceGlobals(source) {
  const header = /\bdef\s+([A-Z_][A-Z0-9_]*)\s*:\s*i32\s*=/g;
  const globals = [];
  let match;
  while ((match = header.exec(source)) !== null) {
    const { body, end } = readFunctionBody(source, match.index);
    globals.push({
      name: match[1],
      bodySource: normalizeSourceExpr(body),
    });
    header.lastIndex = end;
  }
  return globals;
}

function sourceConditionToCore(condition, fn) {
  const text = condition.trim();
  const andParts = splitTopLevel(text, "&&");
  if (andParts.length > 1) {
    return `(if (result i32) (${sourceConditionToCore(andParts[0], fn)}) (then ${sourceConditionToCore(andParts.slice(1).join("&&"), fn)}) (else (i32.const 0)))`;
  }
  const orParts = splitTopLevel(text, "||");
  if (orParts.length > 1) {
    return `(if (result i32) (${sourceConditionToCore(orParts[0], fn)}) (then (i32.const 1)) (else ${sourceConditionToCore(orParts.slice(1).join("||"), fn)}))`;
  }
  if (text === "true") return "i32.const 1";
  if (text === "false") return "i32.const 0";
  const param = paramByName(fn, text);
  if (param != null) return `local.get ${param.index}`;
  fail(`unsupported source primary branch condition ${JSON.stringify(text)}`);
}

function sourceEqualsConditionToCore(left, right, fn) {
  return `(i32.eq ${emitCoreFixtureValueExpr(sourceExprToCore(left, fn))} ${emitCoreFixtureValueExpr(sourceExprToCore(right, fn))})`;
}

function parseIfExpr(text) {
  if (!text.startsWith("if ")) return null;
  const thenOpen = findTopLevelOpenBrace(text);
  if (thenOpen < 0) return null;
  const thenClose = findMatchingBrace(text, thenOpen);
  if (thenClose < 0) return null;
  const condition = text.slice(2, thenOpen).trim();
  const rest = text.slice(thenClose + 1).trim();
  if (!rest.startsWith("else")) return null;
  const elseBody = rest.slice("else".length).trim();
  if (elseBody.startsWith("if ")) {
    return {
      condition,
      thenExpr: text.slice(thenOpen + 1, thenClose),
      elseExpr: elseBody,
    };
  }
  if (!elseBody.startsWith("{")) return null;
  const elseClose = findMatchingBrace(elseBody, 0);
  if (elseClose < 0 || elseBody.slice(elseClose + 1).trim().length !== 0) return null;
  return {
    condition,
    thenExpr: text.slice(thenOpen + 1, thenClose),
    elseExpr: elseBody.slice(1, elseClose),
  };
}

function sourceExprToCore(expr, fn) {
  const text = normalizeSourceExpr(expr);
  if (text.length === 0) fail(`unsupported source primary empty expression in ${fn.symbol}`);
  if (/^[0-9]+$/.test(text)) return { kind: "const", value: Number(text) };
  const param = paramByName(fn, text);
  if (param != null) return param.index === 0 ? { kind: "param0" } : { kind: "param", index: param.index };
  const global = globalByName(fn, text);
  if (global != null) return { kind: "global", name: global.name };
  const tupleField = sourceTupleFieldToCore(text, fn);
  if (tupleField != null) return tupleField;
  const indexExpr = sourceIndexExprToCore(text, fn);
  if (indexExpr != null) return indexExpr;
  const pipeParts = splitTopLevel(text, "|>");
  if (pipeParts.length > 1) {
    return pipeParts.slice(1).reduce((lhs, rhs) => sourcePipeStepToCore(lhs, rhs, fn), sourceExprToCore(pipeParts[0], fn));
  }
  if (text.startsWith("-") && text.slice(1).trim().length > 0) {
    return {
      kind: "binary",
      instruction: "i32.sub",
      left: { kind: "const", value: 0 },
      right: sourceExprToCore(text.slice(1), fn),
    };
  }
  const lowOpAt = findTopLevelBinaryRightmost(text, ["+", "-"]);
  const highOpAt = lowOpAt >= 0 ? -1 : findTopLevelBinaryRightmost(text, ["*", "/"]);
  const opAt = lowOpAt >= 0 ? lowOpAt : highOpAt;
  if (opAt >= 0) {
    const operator = text[opAt];
    const method = operatorMethod(operator);
    const leftText = text.slice(0, opAt).trim();
    const receiverType = paramTypeByName(fn, leftText);
    if (receiverType != null && !fn.symbol.endsWith(`.${method}`)) {
      return {
        kind: "tailcall_args",
        target: `${receiverType}.${method}`,
        args: [sourceExprToCore(leftText, fn), sourceExprToCore(text.slice(opAt + 1), fn)],
      };
    }
    return {
      kind: "binary",
      instruction: operatorInstruction(operator),
      left: sourceExprToCore(leftText, fn),
      right: sourceExprToCore(text.slice(opAt + 1), fn),
    };
  }
  const ifExpr = parseIfExpr(text);
  if (ifExpr != null) {
    return {
      kind: "if",
      condition: sourceConditionToCore(ifExpr.condition, fn),
      thenExpr: sourceExprToCore(ifExpr.thenExpr, fn),
      elseExpr: sourceExprToCore(ifExpr.elseExpr, fn),
    };
  }
  const matchBool = /^match\s+(.+?)\s*\{\s*true\s*=>\s*([\s\S]+?)\s*(?:false|_)\s*=>\s*([\s\S]+?)\s*\}$/.exec(text);
  if (matchBool != null) {
    return {
      kind: "if",
      condition: sourceConditionToCore(matchBool[1], fn),
      thenExpr: sourceExprToCore(matchBool[2], fn),
      elseExpr: sourceExprToCore(matchBool[3], fn),
    };
  }
  const matchI32 = /^match\s+(.+?)\s*\{\s*([0-9]+)\s*=>\s*([\s\S]+?)\s*([0-9]+)\s*=>\s*([\s\S]+?)\s*_\s*=>\s*([\s\S]+?)\s*\}$/.exec(text);
  if (matchI32 != null) {
    return {
      kind: "if",
      condition: sourceEqualsConditionToCore(matchI32[1], matchI32[2], fn),
      thenExpr: sourceExprToCore(matchI32[3], fn),
      elseExpr: {
        kind: "if",
        condition: sourceEqualsConditionToCore(matchI32[1], matchI32[4], fn),
        thenExpr: sourceExprToCore(matchI32[5], fn),
        elseExpr: sourceExprToCore(matchI32[6], fn),
      },
    };
  }
  const methodMatch = /^(.+)\.([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)$/.exec(text);
  if (methodMatch != null && paramByName(fn, methodMatch[1].trim()) != null) {
    const receiverText = methodMatch[1].trim();
    const receiverExpr = sourceExprToCore(receiverText, fn);
    const receiverType = paramTypeByName(fn, receiverText);
    const target = receiverType == null ? methodMatch[2] : `${receiverType}.${methodMatch[2]}`;
    return {
      kind: "tailcall_args",
      target,
      args: [receiverExpr, ...parseCallArgs(methodMatch[3], fn)],
    };
  }
  const callMatch = /^([A-Za-z_][A-Za-z0-9_.]*)\s*\((.*)\)$/.exec(text);
  if (callMatch != null) {
    const args = parseCallArgs(callMatch[2], fn);
    if (args.length === 0) return { kind: "tailcall", target: callMatch[1] };
    if (args.length === 1 && args[0].kind === "const") return { kind: "tailcall_const", target: callMatch[1], value: args[0].value };
    if (args.length === 1 && args[0].kind === "param0") return { kind: "tailcall_param0", target: callMatch[1] };
    return { kind: "tailcall_args", target: callMatch[1], args };
  }
  fail(`unsupported source primary expression ${JSON.stringify(text)}`);
}

function sourcePipeStepToCore(lhs, rhs, fn) {
  const text = normalizeSourceExpr(rhs);
  const callMatch = /^([A-Za-z_][A-Za-z0-9_.]*)\s*(?:\((.*)\))?$/.exec(text);
  if (callMatch == null) fail(`unsupported source primary pipe step ${JSON.stringify(text)}`);
  const rawArgs = callMatch[2] == null ? [] : splitTopLevel(callMatch[2], ",");
  const hasPlaceholder = rawArgs.some((arg) => arg.trim() === "_");
  const args = rawArgs.map((arg) => (arg.trim() === "_" ? lhs : sourceExprToCore(arg, fn)));
  return {
    kind: "tailcall_args",
    target: callMatch[1],
    args: hasPlaceholder ? args : [lhs, ...args],
  };
}

function sourceSliceFunctionToCore(fn) {
  return {
    symbol: fn.symbol,
    exportName: fn.exportName,
    params: fn.params,
    body: sourceExprToCore(fn.bodySource, fn),
  };
}

function sourceSliceGlobalToCore(global, ctx) {
  return {
    name: global.name,
    body: sourceExprToCore(global.bodySource, ctx),
  };
}

function compileSourceSliceWat(source) {
  const parsedGlobals = parseSourceSliceGlobals(source);
  const globalCtx = { symbol: "<global>", params: [], globals: parsedGlobals };
  const globals = parsedGlobals.map((global) => sourceSliceGlobalToCore(global, globalCtx));
  const functions = parseSourceSliceFunctions(source)
    .map((fn) => ({ ...fn, globals }))
    .map(sourceSliceFunctionToCore);
  if (functions.length === 0) fail("source primary fixture produced no functions");
  return emitCoreFixtureModule({ globals, functions });
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
(type $chiba.layout.continuation_frame (struct (field (ref null func)) (field eqref)))
(type $chiba.layout.continuation_frame_chain (struct (field (ref $chiba.layout.continuation_frame)) (field (ref null $chiba.layout.continuation_frame_chain))))
(type $chiba.layout.contN_package (struct (field (ref $chiba.layout.continuation_frame_chain)) (field i32)))
(func $chiba.resume0 (result i32) i32.const 21)
(func $chiba.resume1 (param (ref $chiba.layout.contN_package)) (result i32)
  (struct.get $chiba.layout.contN_package 1 (local.get 0))
  (i32.const 21)
  i32.add)
(func $chiba.make_frame (result (ref $chiba.layout.continuation_frame))
  (struct.new $chiba.layout.continuation_frame (ref.func $chiba.resume0) (ref.null any)))
(func $chiba.make_chain (result (ref $chiba.layout.continuation_frame_chain))
  (struct.new $chiba.layout.continuation_frame_chain (call $chiba.make_frame) (ref.null $chiba.layout.continuation_frame_chain)))
(func $chiba.make_package (result (ref $chiba.layout.contN_package))
  (struct.new $chiba.layout.contN_package (call $chiba.make_chain) (i32.const 21)))
(func $chiba.resume_package (export "resume_package") (result i32)
  (call $chiba.resume1 (call $chiba.make_package)))
(func $chiba.resume_twice (export "resume_twice") (result i32)
  (local $pkg (ref $chiba.layout.contN_package))
  (local.set $pkg (call $chiba.make_package))
  (call $chiba.resume1 (local.get $pkg))
  (call $chiba.resume1 (local.get $pkg))
  i32.add)
)`;
  checkWatArtifactRunnable("contn-package", wat, [
    { exportName: "resume_package", expectedStdout: "42" },
    { exportName: "resume_twice", expectedStdout: "84" },
  ]);
}

function checkContNMultiFrameSpineWatSmoke() {
  const wat = `(module
(type $chiba.layout.continuation_frame (struct (field (ref null func)) (field eqref)))
(type $chiba.layout.continuation_frame_chain (struct (field (ref $chiba.layout.continuation_frame)) (field (ref null $chiba.layout.continuation_frame_chain))))
(type $chiba.layout.contN_package (struct (field (ref $chiba.layout.continuation_frame_chain)) (field i32)))
(func $chiba.contn.resume.7.frame.0 (result i32) i32.const 21)
(func $chiba.contn.resume.7.frame.1 (result i32) i32.const 22)
(func $chiba.contn.frame_chain.7 (result (ref $chiba.layout.continuation_frame_chain))
  ref.func $chiba.contn.resume.7.frame.0
  ref.null any
  struct.new $chiba.layout.continuation_frame
  ref.func $chiba.contn.resume.7.frame.1
  ref.null any
  struct.new $chiba.layout.continuation_frame
  ref.null $chiba.layout.continuation_frame_chain
  struct.new $chiba.layout.continuation_frame_chain
  struct.new $chiba.layout.continuation_frame_chain)
(func $chiba.contn.package.7 (result (ref $chiba.layout.contN_package))
  call $chiba.contn.frame_chain.7
  i32.const 1
  struct.new $chiba.layout.contN_package)
(func (export "head_resume") (result i32)
  call $chiba.contn.resume.7.frame.0)
(func (export "tail_resume") (result i32)
  call $chiba.contn.resume.7.frame.1)
(func (export "frame_count") (result i32)
  (local $chain (ref $chiba.layout.continuation_frame_chain))
  (local $next (ref null $chiba.layout.continuation_frame_chain))
  (local.set $chain (struct.get $chiba.layout.contN_package 0 (call $chiba.contn.package.7)))
  (local.set $next (struct.get $chiba.layout.continuation_frame_chain 1 (local.get $chain)))
  (if (result i32)
    (ref.is_null (local.get $next))
    (then i32.const 1)
    (else i32.const 2)))
)`;
  checkWatArtifactRunnable("contn-multiframe-spine", wat, [
    { exportName: "head_resume", expectedStdout: "21" },
    { exportName: "tail_resume", expectedStdout: "22" },
    { exportName: "frame_count", expectedStdout: "2" },
  ]);
}

function checkContNParamResumeWatSmoke() {
  const wat = `(module
(type $chiba.layout.continuation_frame (struct (field (ref null func)) (field eqref)))
(type $chiba.layout.continuation_frame_chain (struct (field (ref $chiba.layout.continuation_frame)) (field (ref null $chiba.layout.continuation_frame_chain))))
(type $chiba.layout.contN_package (struct (field (ref $chiba.layout.continuation_frame_chain)) (field i32)))
(func $chiba.contn.resume.8.frame.0 (param i32) (result i32) local.get 0)
(func $chiba.contn.frame_chain.8 (result (ref $chiba.layout.continuation_frame_chain))
  ref.func $chiba.contn.resume.8.frame.0
  ref.null any
  struct.new $chiba.layout.continuation_frame
  ref.null $chiba.layout.continuation_frame_chain
  struct.new $chiba.layout.continuation_frame_chain)
(func $chiba.contn.package.8 (result (ref $chiba.layout.contN_package))
  call $chiba.contn.frame_chain.8
  i32.const 1
  struct.new $chiba.layout.contN_package)
(func (export "resume_param") (param i32) (result i32)
  call $chiba.contn.package.8
  drop
  local.get 0
  call $chiba.contn.resume.8.frame.0)
)`;
  checkWatArtifactRunnable("contn-param-resume", wat, [
    { exportName: "resume_param", args: [37], expectedStdout: "37" },
  ]);
}

function checkAdtTupleCtorWatSmoke() {
  const wat = `(module
(type $chiba.adt_tuple.tuple_40_nominal_40_std_58__58_symbol_41__44_var_40_0_41__41_ (struct (field i32) (field i32)))
(func $chiba.__ADT_CTOR_Ok (param i32) (result (ref $chiba.adt_tuple.tuple_40_nominal_40_std_58__58_symbol_41__44_var_40_0_41__41_))
  i32.const 0
  local.get 0
  struct.new $chiba.adt_tuple.tuple_40_nominal_40_std_58__58_symbol_41__44_var_40_0_41__41_)
(func (export "tag") (param i32) (result i32)
  (struct.get $chiba.adt_tuple.tuple_40_nominal_40_std_58__58_symbol_41__44_var_40_0_41__41_ 0 (call $chiba.__ADT_CTOR_Ok (local.get 0))))
(func (export "payload") (param i32) (result i32)
  (struct.get $chiba.adt_tuple.tuple_40_nominal_40_std_58__58_symbol_41__44_var_40_0_41__41_ 1 (call $chiba.__ADT_CTOR_Ok (local.get 0))))
)`;
  checkWatArtifactRunnable("adt-tuple-ctor", wat, [
    { exportName: "tag", args: [41], expectedStdout: "0" },
    { exportName: "payload", args: [41], expectedStdout: "41" },
  ]);
}

function checkAdtTupleCtorSharedNominalWatSmoke() {
  const tupleType = "$chiba.adt_tuple.tuple_40_nominal_40_std_58__58_symbol_41__44_var_40_0_41__41_";
  const wat = `(module
(type ${tupleType} (struct (field i32) (field i32)))
(func $chiba.__ADT_CTOR_Ok (param i32) (result (ref ${tupleType}))
  i32.const 0
  local.get 0
  struct.new ${tupleType})
(func $chiba.__ADT_CTOR_Err (param i32) (result (ref ${tupleType}))
  i32.const 1
  local.get 0
  struct.new ${tupleType})
(func (export "ok_payload") (param i32) (result i32)
  (struct.get ${tupleType} 1 (call $chiba.__ADT_CTOR_Ok (local.get 0))))
(func (export "err_tag") (param i32) (result i32)
  (struct.get ${tupleType} 0 (call $chiba.__ADT_CTOR_Err (local.get 0))))
)`;
  const typeDeclCount = (wat.match(/\(type \$chiba\.adt_tuple\./g) ?? []).length;
  if (typeDeclCount !== 1) fail(`adt tuple nominal smoke expected one shared tuple type, saw ${typeDeclCount}`);
  checkWatArtifactRunnable("adt-tuple-ctor-shared-nominal", wat, [
    { exportName: "ok_payload", args: [41], expectedStdout: "41" },
    { exportName: "err_tag", args: [41], expectedStdout: "1" },
  ]);
}

function checkTupleNominalOrderDistinctWatSmoke() {
  const tupleAB = "$chiba.adt_tuple.tuple_40_nominal_40_std_58__58_symbol_41__44_var_40_0_41__41_";
  const tupleBA = "$chiba.adt_tuple.tuple_40_var_40_0_41__44_nominal_40_std_58__58_symbol_41__41_";
  const wat = `(module
(type ${tupleAB} (struct (field i32) (field i32)))
(type ${tupleBA} (struct (field i32) (field i32)))
(func $chiba.make_ab (param i32) (result (ref ${tupleAB}))
  i32.const 7
  local.get 0
  struct.new ${tupleAB})
(func $chiba.make_ba (param i32) (result (ref ${tupleBA}))
  local.get 0
  i32.const 7
  struct.new ${tupleBA})
(func (export "ab_second") (param i32) (result i32)
  (struct.get ${tupleAB} 1 (call $chiba.make_ab (local.get 0))))
(func (export "ba_first") (param i32) (result i32)
  (struct.get ${tupleBA} 0 (call $chiba.make_ba (local.get 0))))
)`;
  const typeDeclCount = (wat.match(/\(type \$chiba\.adt_tuple\./g) ?? []).length;
  if (typeDeclCount !== 2) fail(`tuple nominal order smoke expected two order-distinct tuple types, saw ${typeDeclCount}`);
  if (tupleAB === tupleBA) fail("tuple nominal order smoke expected order-distinct tuple labels");
  checkWatArtifactRunnable("adt-tuple-ctor-order-distinct-nominal", wat, [
    { exportName: "ab_second", args: [41], expectedStdout: "41" },
    { exportName: "ba_first", args: [41], expectedStdout: "41" },
  ]);
}

function checkTupleHeapFieldAccessWatSmoke() {
  const tuple2 = "$chiba.tuple.nominal_40_std_58__58_i32_41__44_nominal_40_std_58__58_i32_41";
  const tuple3 = "$chiba.tuple.nominal_40_std_58__58_i32_41__44_nominal_40_std_58__58_i32_41__44_nominal_40_std_58__58_i32_41";
  const wat = `(module
(type ${tuple2} (struct (field i32) (field i32)))
(type ${tuple3} (struct (field i32) (field i32) (field i32)))
(func $chiba.make_pair (param i32) (param i32) (result (ref ${tuple2}))
  local.get 0
  local.get 1
  struct.new ${tuple2})
(func $chiba.make_triple (param i32) (param i32) (param i32) (result (ref ${tuple3}))
  local.get 0
  local.get 1
  local.get 2
  struct.new ${tuple3})
(func (export "tuple_first") (result i32)
  (struct.get ${tuple2} 0 (call $chiba.make_pair (i32.const 41) (i32.const 1))))
(func (export "tuple_second") (result i32)
  (struct.get ${tuple2} 1 (call $chiba.make_pair (i32.const 40) (i32.const 2))))
(func (export "tuple_third") (result i32)
  (struct.get ${tuple3} 2 (call $chiba.make_triple (i32.const 38) (i32.const 2) (i32.const 1))))
)`;
  checkWatArtifactRunnable("tuple-heap-field-access", wat, [
    { exportName: "tuple_first", expectedStdout: "41" },
    { exportName: "tuple_second", expectedStdout: "2" },
    { exportName: "tuple_third", expectedStdout: "1" },
  ]);
}

function checkIntrinsicBridgeIdentityWatSmoke() {
  const tupleType = "$chiba.adt_tuple.tuple_40_nominal_40_std_58__58_symbol_41__44_var_40_0_41__41_";
  const wat = `(module
(type ${tupleType} (struct (field i32) (field i32)))
(func $chiba.compiler.intrinsic.tuple_to_adt (param eqref) (result eqref) local.get 0)
(func $chiba.compiler.intrinsic.adt_to_tuple (param eqref) (result eqref) local.get 0)
(func $chiba.make_tuple (param i32) (result (ref ${tupleType}))
  i32.const 0
  local.get 0
  struct.new ${tupleType})
(func $chiba.roundtrip (param i32) (result (ref ${tupleType}))
  (ref.cast (ref ${tupleType})
    (call $chiba.compiler.intrinsic.adt_to_tuple
      (call $chiba.compiler.intrinsic.tuple_to_adt
        (call $chiba.make_tuple (local.get 0))))))
(func (export "roundtrip_payload") (param i32) (result i32)
  (struct.get ${tupleType} 1 (call $chiba.roundtrip (local.get 0))))
)`;
  checkWatArtifactRunnable("adt-tuple-intrinsic-roundtrip", wat, [
    { exportName: "roundtrip_payload", args: [41], expectedStdout: "41" },
  ]);
}

function checkBoxedCont1StateMachineWatSmoke() {
  const wat = `(module
(type $chiba.layout.continuation_frame (struct (field (ref null func)) (field eqref)))
(type $chiba.layout.boxed_cont1 (struct (field (ref null $chiba.layout.continuation_frame)) (field (mut i32))))
(func $chiba.cont1.resume (param (ref null $chiba.layout.boxed_cont1)) (result i32)
  (local $box (ref $chiba.layout.boxed_cont1))
  local.get 0
  ref.cast (ref $chiba.layout.boxed_cont1)
  local.tee $box
  struct.get $chiba.layout.boxed_cont1 1
  if
    unreachable
  end
  local.get $box
  i32.const 1
  struct.set $chiba.layout.boxed_cont1 1
  i32.const 41)
(func $chiba.make_box (result (ref $chiba.layout.boxed_cont1))
  (struct.new $chiba.layout.boxed_cont1 (ref.null $chiba.layout.continuation_frame) (i32.const 0)))
(func (export "resume_once") (result i32)
  (call $chiba.cont1.resume (call $chiba.make_box)))
(func (export "resume_twice_traps") (result i32)
  (local $box (ref $chiba.layout.boxed_cont1))
  (local.set $box (call $chiba.make_box))
  (drop (call $chiba.cont1.resume (local.get $box)))
  (call $chiba.cont1.resume (local.get $box)))
)`;
  checkWatArtifactRunnableAndTraps("boxed-cont1-state-machine", wat, [
    { exportName: "resume_once", expectedStdout: "41" },
  ], [
    { exportName: "resume_twice_traps", expectedStderr: "wasm trap: wasm `unreachable` instruction executed" },
  ]);
}

function checkClosureCallRefWatSmoke() {
  const wat = `(module
(type $chiba.closure.i32_to_i32 (func (param i32) (result i32)))
(func $chiba.add_one (param i32) (result i32)
  local.get 0
  i32.const 1
  i32.add)
(func $chiba.closure.call (param funcref) (param eqref) (param i32) (result i32)
  local.get 2
  local.get 0
  ref.cast (ref $chiba.closure.i32_to_i32)
  call_ref $chiba.closure.i32_to_i32)
(func (export "closure_call") (param i32) (result i32)
  (call $chiba.closure.call (ref.func $chiba.add_one) (ref.null any) (local.get 0)))
)`;
  checkWatArtifactRunnable("closure-call-ref", wat, [
    { exportName: "closure_call", args: [41], expectedStdout: "42" },
  ]);
}

function checkHeapAllocationWatSmoke() {
  const wat = `(module
(type $chiba.layout.struct (struct))
(type $chiba.layout.array (array (mut i32)))
(func $chiba.struct_new (result eqref)
  struct.new $chiba.layout.struct)
(func $chiba.array_new (result eqref)
  i32.const 0
  i32.const 3
  array.new $chiba.layout.array)
(func (export "struct_is_non_null") (result i32)
  (ref.is_null (call $chiba.struct_new))
  i32.eqz)
(func (export "array_len") (result i32)
  (array.len (ref.cast (ref $chiba.layout.array) (call $chiba.array_new))))
)`;
  checkWatArtifactRunnable("heap-allocation", wat, [
    { exportName: "struct_is_non_null", expectedStdout: "1" },
    { exportName: "array_len", expectedStdout: "3" },
  ]);
}

function checkErasedContinuationPackageWatSmoke() {
  const wat = `(module
(type $chiba.layout.continuation_package (struct (field i32) (field eqref)))
(func $chiba.make_erased_callable (result (ref $chiba.layout.continuation_package))
  i32.const 2
  ref.null any
  struct.new $chiba.layout.continuation_package)
(func (export "erased_tag") (result i32)
  (struct.get $chiba.layout.continuation_package 0 (call $chiba.make_erased_callable)))
(func (export "erased_payload_is_null") (result i32)
  (ref.is_null (struct.get $chiba.layout.continuation_package 1 (call $chiba.make_erased_callable))))
)`;
  checkWatArtifactRunnable("erased-continuation-package", wat, [
    { exportName: "erased_tag", expectedStdout: "2" },
    { exportName: "erased_payload_is_null", expectedStdout: "1" },
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
(func (export "closure_env_layout_smoke") (result i32) i32.const 0)
(func $chiba.cont1_resume (param (ref null $chiba.layout.boxed_cont1)) (result i32)
  (local $box (ref $chiba.layout.boxed_cont1))
  local.get 0
  ref.cast (ref $chiba.layout.boxed_cont1)
  local.tee $box
  struct.get $chiba.layout.boxed_cont1 1
  if
    unreachable
  end
  local.get $box
  i32.const 1
  struct.set $chiba.layout.boxed_cont1 1
  i32.const 41)
(func $chiba.make_boxed_cont1 (result (ref $chiba.layout.boxed_cont1))
  (struct.new $chiba.layout.boxed_cont1 (ref.null $chiba.layout.continuation_frame) (i32.const 0)))
(func (export "boxed_cont1_state_smoke") (result i32)
  (call $chiba.cont1_resume (call $chiba.make_boxed_cont1)))
(func $chiba.contn_resume_once (result i32) i32.const 21)
(func (export "contn_resume_twice") (result i32)
  (call $chiba.contn_resume_once)
  (call $chiba.contn_resume_once)
  i32.add)
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
    { exportName: "closure_env_layout_smoke", expectedStdout: "0" },
    { exportName: "boxed_cont1_state_smoke", expectedStdout: "41" },
    { exportName: "contn_resume_twice", expectedStdout: "42" },
  ]);
}

function checkSourcePrimaryBackendWat() {
  const source = read(SOURCE_PRIMARY_FIXTURE);
  const wat = compileSourceSliceWat(source);
  if (!wat.includes("i32.const 7")) fail("source primary backend fixture must preserve small const 7");
  if (!wat.includes("i32.const 13")) fail("source primary backend fixture must preserve distinct else-arm const 13");
  if (!wat.includes("return_call $chiba.id")) fail("source primary backend fixture must lower tail calls as return_call");
  if (!wat.includes("return_call $chiba.callee")) fail("source primary backend fixture must lower source no-arg tail calls as return_call");
  if (!wat.includes("return_call $chiba.i32.op_add")) fail("source primary backend fixture must lower + through i32.op_add target");
  if (!wat.includes("return_call $chiba.i32.op_sub")) fail("source primary backend fixture must lower - through i32.op_sub target");
  if (!wat.includes("return_call $chiba.i32.op_mul")) fail("source primary backend fixture must lower * through i32.op_mul target");
  if (!wat.includes("return_call $chiba.i32.op_div")) fail("source primary backend fixture must lower / through i32.op_div target");
  if (!wat.includes("return_call $chiba.i32.op_index")) fail("source primary backend fixture must lower [] through i32.op_index target");
  if (!wat.includes("return_call $chiba.i32.op_index_slice")) fail("source primary backend fixture must lower [..] through i32.op_index_slice target");
  if (!wat.includes("return_call $chiba.i32.i32_id")) fail("source primary backend fixture must lower method call through receiver type target");
  if (!wat.includes("(export \"tuple_first\")") || !wat.includes("(export \"tuple_second\")") || !wat.includes("(export \"tuple_third\")")) {
    fail("source primary backend fixture must include executable tuple field access lowering");
  }
  if (!wat.includes("(global $chiba.global.ONE") || !wat.includes("(global $chiba.global.TWO")) {
    fail("source primary backend fixture must emit static global storage");
  }
  if (!wat.includes("(start $chiba.__module_init)")) fail("source primary backend fixture must emit module-load init start");
  if (!wat.includes("global.get $chiba.global.ONE") || !wat.includes("global.set $chiba.global.TWO")) {
    fail("source primary backend fixture must lower global read/dependency initialization");
  }
  if (!wat.includes("(export \"choose_match\")")) fail("source primary backend fixture must include executable match lowering");
  if (!wat.includes("(export \"choose_match_fallback\")")) fail("source primary backend fixture must include executable match fallback lowering");
  if (!wat.includes("(export \"choose_match_i32\")") || !wat.includes("i32.eq")) {
    fail("source primary backend fixture must include executable i32 literal match lowering");
  }
  if (!wat.includes("(export \"choose_else_if\")")) fail("source primary backend fixture must include executable else-if lowering");
  if (!wat.includes("(export \"choose_nested\")")) fail("source primary backend fixture must include executable nested branch lowering");
  if (!wat.includes("(export \"short_and\")") || !wat.includes("(export \"short_or\")")) {
    fail("source primary backend fixture must include executable short-circuit lowering");
  }
  if (!wat.includes("i32.add") || !wat.includes("i32.sub") || !wat.includes("i32.mul") || !wat.includes("i32.div_s")) {
    fail("source primary backend fixture must include executable builtin operator primitive lowering");
  }
  if (!wat.includes("return_call $chiba.add3")) fail("source primary backend fixture must include executable multi-arg tailcall lowering");
  if (!wat.includes("(export \"pipe_placeholder\")")) fail("source primary backend fixture must include executable pipe placeholder lowering");
  if (!wat.includes("(export \"pipe_chain\")")) fail("source primary backend fixture must include executable pipe chain lowering");
  if (!wat.includes("(export \"pipe_method_path\")")) fail("source primary backend fixture must include executable qualified pipe method lowering");
  if (!wat.includes("(export \"method_style\")")) fail("source primary backend fixture must include executable method-style lowering");
  checkWatArtifactRunnable("source-primary-backend", wat, [
    { exportName: "id", args: [19], expectedStdout: "19" },
    { exportName: "seven", expectedStdout: "7" },
    { exportName: "callee", expectedStdout: "42" },
    { exportName: "main", expectedStdout: "7" },
    { exportName: "main_noarg_tail", expectedStdout: "42" },
    { exportName: "literal_branch", expectedStdout: "7" },
    { exportName: "choose", args: [1], expectedStdout: "7" },
    { exportName: "choose", args: [0], expectedStdout: "13" },
    { exportName: "choose_else_if", args: [1, 0], expectedStdout: "7" },
    { exportName: "choose_else_if", args: [0, 1], expectedStdout: "13" },
    { exportName: "choose_else_if", args: [0, 0], expectedStdout: "42" },
    { exportName: "choose_nested", args: [1, 1], expectedStdout: "7" },
    { exportName: "choose_nested", args: [1, 0], expectedStdout: "13" },
    { exportName: "choose_nested", args: [0, 1], expectedStdout: "42" },
    { exportName: "choose_match", args: [1], expectedStdout: "7" },
    { exportName: "choose_match", args: [0], expectedStdout: "13" },
    { exportName: "choose_match_fallback", args: [1], expectedStdout: "7" },
    { exportName: "choose_match_fallback", args: [0], expectedStdout: "13" },
    { exportName: "choose_match_i32", args: [0], expectedStdout: "7" },
    { exportName: "choose_match_i32", args: [1], expectedStdout: "13" },
    { exportName: "choose_match_i32", args: [2], expectedStdout: "42" },
    { exportName: "short_and", args: [1, 1], expectedStdout: "7" },
    { exportName: "short_and", args: [1, 0], expectedStdout: "13" },
    { exportName: "short_and", args: [0, 1], expectedStdout: "13" },
    { exportName: "short_or", args: [1, 0], expectedStdout: "7" },
    { exportName: "short_or", args: [0, 1], expectedStdout: "7" },
    { exportName: "short_or", args: [0, 0], expectedStdout: "13" },
    { exportName: "i32.op_add", args: [2, 5], expectedStdout: "7" },
    { exportName: "i32.op_sub", args: [9, 4], expectedStdout: "5" },
    { exportName: "i32.op_mul", args: [6, 7], expectedStdout: "42" },
    { exportName: "i32.op_div", args: [21, 3], expectedStdout: "7" },
    { exportName: "add2", args: [7, 13], expectedStdout: "20" },
    { exportName: "sub2", args: [20, 8], expectedStdout: "12" },
    { exportName: "neg_param", args: [7], expectedStdout: "-7" },
    { exportName: "neg_literal", expectedStdout: "-7" },
    { exportName: "add_negative", args: [20, 8], expectedStdout: "12" },
    { exportName: "mul2", args: [6, 7], expectedStdout: "42" },
    { exportName: "div2", args: [21, 3], expectedStdout: "7" },
    { exportName: "op_precedence", args: [2, 3, 4], expectedStdout: "14" },
    { exportName: "add3", args: [2, 3, 4], expectedStdout: "9" },
    { exportName: "call_add3", args: [6, 7], expectedStdout: "18" },
    { exportName: "pipe_default", expectedStdout: "7" },
    { exportName: "pipe_placeholder", expectedStdout: "14" },
    { exportName: "pipe_chain", expectedStdout: "7" },
    { exportName: "pipe_method_path", expectedStdout: "7" },
    { exportName: "i32.i32_id", args: [23], expectedStdout: "23" },
    { exportName: "method_style", args: [29], expectedStdout: "29" },
    { exportName: "qualified_call", expectedStdout: "31" },
    { exportName: "global_const", expectedStdout: "7" },
    { exportName: "global_dependency", expectedStdout: "42" },
    { exportName: "global_expr_use", expectedStdout: "49" },
    { exportName: "i32.op_index", args: [20, 7], expectedStdout: "27" },
    { exportName: "index_style", args: [20], expectedStdout: "27" },
    { exportName: "i32.op_index_slice", args: [20, 7, 13], expectedStdout: "40" },
    { exportName: "index_slice_style", args: [20], expectedStdout: "40" },
    { exportName: "tuple_first", expectedStdout: "41" },
    { exportName: "tuple_second", expectedStdout: "2" },
    { exportName: "tuple_third", expectedStdout: "1" },
  ]);
}

function checkAstPrimaryTypedMainWat() {
  const evidence = readJson(CHIBACC_EVIDENCE);
  const full = evidence.fullGrammar || {};
  if (full.astOwnerNamespace !== "demo" || full.astDefItemName !== "main") {
    fail("AST primary typed main evidence must expose demo::main");
  }
  if (full.astDefHasI32ConstBody !== false || full.astDefI32ConstBody !== 0 || full.astDefHasI32AddMulBody !== true || full.astDefHasI32IfElseBody !== false || full.astDefHasI32PrefixNegBody !== false) {
    fail("AST primary typed main evidence must expose add/mul body");
  }
  if (full.astBranchOwnerNamespace !== "demo" || full.astBranchDefItemName !== "branch" || full.astBranchBodyShape !== "SourceAstExprI32IfElse" || full.astBranchHasIfElseBody !== true) {
    fail("AST primary typed main evidence must expose demo::branch if/else body");
  }
  if (full.astNegOwnerNamespace !== "demo" || full.astNegDefItemName !== "neg" || full.astNegBodyShape !== "SourceAstExprI32PrefixNeg" || full.astNegHasPrefixNegBody !== true) {
    fail("AST primary typed main evidence must expose demo::neg prefix neg body");
  }
  if (full.astMatchOwnerNamespace !== "demo" || full.astMatchDefItemName !== "choose" || full.astMatchBodyShape !== "SourceAstExprI32MatchParam" || full.astMatchHasParamBody !== true) {
    fail("AST primary typed main evidence must expose demo::choose match-param body");
  }
  if (full.astExpressionOwnerNamespace !== "demo" || full.astExpressionDefItemName !== "main" || full.astExpressionHasAddMulBody !== true || full.astExpressionHasIfElseBody !== false || full.astExpressionHasPrefixNegBody !== false) {
    fail("AST primary expression evidence must expose demo::main add/mul body");
  }
  const nodes = astExprNodes(full);
  const wat = emitCoreFixtureModule([
    ...astPrimaryBuiltinIntrinsicFunctions(),
    {
      symbol: `${full.astOwnerNamespace}::${full.astDefItemName}`,
      exportName: "main",
      body: coreExprFromAstNode(nodes, full.astDefItemName, 0),
    },
    {
      symbol: `${full.astBranchOwnerNamespace}::${full.astBranchDefItemName}`,
      exportName: "branch",
      body: coreExprFromAstNode(nodes, full.astBranchDefItemName, 0),
    },
    {
      symbol: "demo::branch_param",
      exportName: "branch_param",
      params: 1,
      body: coreExprFromAstNode(nodes, "branch_param", 0),
    },
    {
      symbol: `${full.astNegOwnerNamespace}::${full.astNegDefItemName}`,
      exportName: "neg",
      body: coreExprFromAstNode(nodes, full.astNegDefItemName, 0),
    },
    {
      symbol: `${full.astMatchOwnerNamespace}::${full.astMatchDefItemName}`,
      exportName: "choose",
      params: 1,
      body: coreExprFromAstNode(nodes, full.astMatchDefItemName, 0),
    },
    {
      symbol: "demo::id",
      exportName: "id",
      params: 1,
      body: coreExprFromAstNode(nodes, "id", 0),
    },
    {
      symbol: "demo::call_id",
      exportName: "call_id",
      body: coreExprFromAstNode(nodes, "call_id", 0),
    },
    {
      symbol: "demo::i32_index",
      exportName: "i32_index",
      params: 2,
      body: coreExprFromAstNode(nodes, "i32_index", 0),
    },
    {
      symbol: "demo::index_style",
      exportName: "index_style",
      params: 1,
      body: coreExprFromAstNode(nodes, "index_style", 0),
    },
    {
      symbol: "demo::method_style",
      exportName: "method_style",
      body: coreExprFromAstNode(nodes, "method_style", 0),
    },
    {
      symbol: "demo::tuple_field_ast",
      exportName: "tuple_field_ast",
      body: coreExprFromAstNode(nodes, "tuple_field_ast", 0),
    },
  ]);
  checkAstPrimaryWatShape(wat);
  checkWatArtifactRunnable("ast-primary-typed-main", wat, [
    { exportName: "main", expectedStdout: "14" },
    { exportName: "branch", expectedStdout: "7" },
    { exportName: "branch_param", args: [1], expectedStdout: "7" },
    { exportName: "branch_param", args: [0], expectedStdout: "13" },
    { exportName: "neg", expectedStdout: "-7" },
    { exportName: "choose", args: [1], expectedStdout: "7" },
    { exportName: "choose", args: [0], expectedStdout: "13" },
    { exportName: "id", args: [23], expectedStdout: "23" },
    { exportName: "call_id", expectedStdout: "23" },
    { exportName: "i32_index", args: [20, 7], expectedStdout: "27" },
    { exportName: "index_style", args: [20], expectedStdout: "27" },
    { exportName: "method_style", expectedStdout: "29" },
    { exportName: "tuple_field_ast", expectedStdout: "2" },
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

function checkAstPrimaryLevel1bLoweringChain() {
  const semantic = read("level-1b/compiler/semantic/type_infer.chiba");
  const cps = read("level-1b/compiler/control/cps.chiba");
  const core = read("level-1b/compiler/backend/core.chiba");
  const required = [
    [semantic, "SourceAstExprNodeStructNew => typed_expr_from_ast_struct_new_node"],
    [semantic, "SourceAstExprNodeFieldGet => typed_expr_from_ast_field_get_node"],
    [semantic, "SourceAstExprNodeMethodCall => typed_expr_from_ast_method_node"],
    [semantic, "SourceAstExprNodeIndex =>"],
    [semantic, "typed_ast_arg_ids_from_node(node)"],
    [cps, "TypedExprStructNew(layout, fields, args)"],
    [cps, "TypedExprFieldGet(layout, fields, value, index)"],
    [cps, "TypedExprTailCallArgs(target, args)"],
    [cps, "CpsTermStructNew(layout as str, fields"],
    [cps, "CpsTermFieldGet(layout as str, fields"],
    [cps, "CpsTermTailCallExprs(target"],
    [core, "CpsTermStructNew(layout, fields, args)"],
    [core, "CpsTermFieldGet(layout, fields, value, index)"],
    [core, "CoreExprStructNew(core_wasm_gc_layout_from_typed(layout, fields)"],
    [core, "CoreExprFieldGet(core_wasm_gc_layout_from_typed(layout, fields)"],
    [core, "CoreExprTailCallArgs(target, core_expr_args_from_cps_terms"],
  ];
  const missing = required.filter(([source, needle]) => !source.includes(needle)).map(([, needle]) => needle);
  if (missing.length !== 0) fail(`AST primary executable WAT must be backed by level-1b typed/CPS/Core lowering, missing:\n${missing.join("\n")}`);
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
  checkAstPrimaryLevel1bLoweringChain();
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
  checkContNMultiFrameSpineWatSmoke();
  checkContNParamResumeWatSmoke();
  checkAdtTupleCtorWatSmoke();
  checkAdtTupleCtorSharedNominalWatSmoke();
  checkTupleNominalOrderDistinctWatSmoke();
  checkTupleHeapFieldAccessWatSmoke();
  checkIntrinsicBridgeIdentityWatSmoke();
  checkBoxedCont1StateMachineWatSmoke();
  checkClosureCallRefWatSmoke();
  checkHeapAllocationWatSmoke();
  checkErasedContinuationPackageWatSmoke();
  checkFeatureMatrixWat();
  checkSourcePrimaryBackendWat();
  checkAstPrimaryTypedMainWat();

  const chibacNext = emitCoreFixtureModule([
    { symbol: "main", exportName: "main", body: { kind: "const", value: 42 } },
  ]);
  checkWatArtifactRunnable("chibac-next-core-constant", chibacNext, [
    { exportName: "main", expectedStdout: "42" },
  ]);

  const continuationFrameBody = `(module
(type $chiba.layout.continuation_frame (struct (field (ref null func)) (field eqref)))
(type $chiba.layout.continuation_frame_chain (struct (field (ref $chiba.layout.continuation_frame)) (field (ref null $chiba.layout.continuation_frame_chain))))
(func $chiba.contN.resume0 (result i32) i32.const 21)
(func $chiba.contN.frame_body (param (ref $chiba.layout.continuation_frame_chain)) (result i32)
  (drop (local.get 0))
  (return_call $chiba.contN.resume0))
(func $chiba.contN.make_chain (result (ref $chiba.layout.continuation_frame_chain))
  (struct.new $chiba.layout.continuation_frame_chain
    (struct.new $chiba.layout.continuation_frame (ref.func $chiba.contN.resume0) (ref.null any))
    (ref.null $chiba.layout.continuation_frame_chain)))
(func $chiba.contN.resume_twice (export "contn_resume_twice") (result i32)
  (local $chain (ref $chiba.layout.continuation_frame_chain))
  (local.set $chain (call $chiba.contN.make_chain))
  (call $chiba.contN.frame_body (local.get $chain))
  (call $chiba.contN.frame_body (local.get $chain))
  i32.add)
)`;
  checkWatArtifactRunnable("continuation-frame-body", continuationFrameBody, [
    { exportName: "contn_resume_twice", expectedStdout: "42" },
  ]);
  checkArtifactsExist();
}

main();
