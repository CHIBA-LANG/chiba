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
  "block_obligations: Array[CoreBlockLoweringObligation]",
  "def core_block_terminator_for_op",
  "def core_block_lowering_obligation_from_op",
  "def core_block_lowering_obligations",
  "data CoreRuntimeState",
  "CoreConsumedStateMachine",
  "data CoreFunctionBody",
  "CoreFunctionReturnI32FortyTwo",
  "CoreFunctionTailCall",
  "export_main: bool",
  "function_body: Option[CoreFunctionBody]",
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
  "CoreMissingOwner",
  "CoreIllegalRuntimeState",
  "CoreIllegalTailCall",
  "CoreNonTailCps",
  "CoreIllegalContinuationPackage",
  "def validate_wasm_gc_core",
  "def validate_core_continuation_order",
  "type ContNFrameChainState",
  "def validate_core_owner",
  "def CoreOpKind.requires_owner",
  "def validate_core_runtime_state",
  "def validate_core_cps_tail_form",
  "def validate_core_contn_frame_count",
  "def validate_core_contn_stackless_count",
  "boxed Cont1 must carry consumed-state machine",
  "ContN frame chain has no stackless resume frames",
  "ContN frame chain missing stackless resume functions",
  "runtime Core op missing owner provenance",
  "ordinary CPS function lowered to non-tail fallthrough",
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
  "def emit_tail_target_function",
  "(export \\\"main\\\")",
  "i32.const 42",
  "return_call $chiba.tail_target",
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
  if (path.basename(file) === "driver.chiba" && !/\bcore_validation_diagnostic\b[\s\S]{0,520}CoreIllegalRuntimeState/.test(code)) {
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
(func $chiba.tail_target (result i32) i32.const 42)
(func (export "main") (result i32) return_call $chiba.tail_target)
)`;
  compileWat(wat);
  pass("tailcall WAT parse");
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

  primaryPathBlocked("tailcall lowering smoke requires level-1b typed call graph");
  primaryPathBlocked("chibac-next WAT smoke requires level-1b end-to-end Core pipeline");
  primaryPathBlocked("continuation WAT smoke requires level-1b continuation frame bodies");
}

main();
