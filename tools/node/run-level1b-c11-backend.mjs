import fs from "node:fs";
import path from "node:path";
import process from "node:process";

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
  "data CoreRuntimeState",
  "CoreConsumedStateMachine",
  "frame_count: usize",
  "CoreOpStacklessFunction",
  "CoreOpBoxedCont1",
  "CoreOpContinuationFrameChain",
  "CoreOpContNPackage",
  "CoreOpContinuationPackage",
  "CoreOpTailCall",
  "def lower_continuation_fact",
  "def lower_continuation_facts",
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
  "CoreIllegalContinuationPackage",
  "def validate_wasm_gc_core",
  "def validate_core_continuation_order",
  "type ContNFrameChainState",
  "def validate_core_owner",
  "def validate_core_runtime_state",
  "def validate_core_contn_frame_count",
  "boxed Cont1 must carry consumed-state machine",
  "ContN frame chain has no stackless resume frames",
  "runtime Core op missing owner provenance",
  "def validate_core_ops_with_contn_frame",
  "ContN package owner does not match preceding frame chain",
  "ContN package frame count does not match preceding frame chain",
  "ContN package missing preceding frame chain",
  "def emit_wat",
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
  if (path.basename(file) === "core.chiba" && !/\bContinuationLowerRepeatableContN\s*=>[\s\S]{0,420}CoreOpContinuationFrameChain[\s\S]{0,420}CoreOpContNPackage/.test(code)) {
    errors.push(`${file}: ContN lowering must emit frame chain before repeatable package`);
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
  if (path.basename(file) === "validate_core.chiba" && !/\bvalidate_core_owner\b[\s\S]{0,240}requires_layout[\s\S]{0,240}CoreMissingOwner/.test(code)) {
    errors.push(`${file}: validator must reject materialized Core ops without owner provenance`);
  }
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${file}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
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

  primaryPathBlocked("tailcall WAT smoke requires level-1b WAT emission");
  primaryPathBlocked("chibac-next WAT smoke requires level-1b WAT emission");
  primaryPathBlocked("continuation WAT smoke requires level-1b continuation backend emission");
}

main();
