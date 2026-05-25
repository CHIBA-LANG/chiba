import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const ROOT = "level-1b/compiler/closure";
const CONTRACT_FILES = [
  "level-1b/compiler/ir/closure.chiba",
];
const NO_CAPTURE_FIXTURE = "level-1b/supports/pre-c10-smokes/no_capture_direct.chiba";
const CAPTURE_FIXTURE = "level-1b/supports/pre-c10-smokes/capture_env.chiba";
const MULTISHOT_FIXTURE = "level-1b/supports/pre-c10-smokes/multishot_package.chiba";
const UNSAFE_FIXTURE = "level-1b/supports/pre-c10-smokes/unsafe_capture_invalid.chiba";
const REQUIRED_FILES = [
  "continuation_simplify.chiba",
  "closure_convert.chiba",
  "driver.chiba",
  "env_simplify.chiba",
  "lambda_lift.chiba",
  "usage_cps.chiba",
];
const REQUIRED_TEXT = [
  "def analyze_cps_usage",
  "def cps_usage_facts_none",
  "def push_closure_surface_usage_fact",
  "def cps_usage_facts_from_closure_surface",
  "def cps_usage_facts_from_runtime_surface",
  "UseSubjectLambda",
  "UseSubjectClosure",
  "syntactic_no_capture: bool",
  "def simplify_continuations",
  "def convert_closures",
  "def lift_lambdas",
  "def simplify_closure_envs",
  "def run_closure_package",
  "ContinuationPackaged",
  "ClosureEnvLayout",
  "type ContinuationFrameChainObligation",
  "requires_stackless_resume_function: bool",
  "requires_frame_chain: bool",
  "requires_repeatable_package: bool",
  "type ContinuationOneShotObligation",
  "requires_direct_resume: bool",
  "requires_boxed_state_machine: bool",
  "requires_consumed_state: bool",
  "type ContinuationCapturePlan",
  "continuation_capture_plans: Array[ContinuationCapturePlan]",
  "requires_capture_scan: bool",
  "requires_frame_body_extraction: bool",
  "ref_capture_uses_shared_reference_semantics: bool",
  "def continuation_capture_plan",
  "def continuation_capture_plans",
  "ClosureDirectFunction",
  "ClosureEnvErased",
  "ClosureDirectified",
  "ClosureNoCaptureDirect",
  "ClosureCapturingEnv",
  "closures: Array[ClosureLoweringFact]",
  "def cps_use_fact_requires_closure_capture",
  "def closure_usage_capture_extraction_blocker",
  "def closure_usage_capture_extraction_blockers",
  "def closure_usage_requires_capture_extraction",
  "def closure_usage_env_layout",
  "def closure_env_layouts_from_usage",
  "def closure_env_layouts_for_module",
  "missing-facts: lambda capture extraction absent",
  "missing-facts: closure env field extraction absent",
  "def closure_capture_class",
  "def closure_env_field_from_capture",
  "def closure_env_from_layout",
  "def closure_lowering_fact",
  "def closure_lowering_facts",
  "env: None",
  "env: Some",
  "type StacklessResumeFunction",
  "data StacklessResumeBodyKind",
  "StacklessResumeBodyShiftN",
  "def stackless_resume_body_from_control",
  "def stackless_resume_body_for_owner",
  "type ContinuationFrame",
  "ContinuationLowerBoxedCont1",
  "ContinuationLowerRepeatableContN",
  "def continuation_lowering_fact",
  "def continuation_lowering_facts_from_module",
  "def stackless_resume_function_for_owner",
  "def continuation_frame_for_owner",
  "def continuation_lowering_frames",
  "def continuation_lowering_kind",
  "def continuation_decision_emits_lowering_fact",
  "def ContinuationSimplification.capability",
  "def continuation_capture_extraction_blocker",
  "def continuation_capture_extraction_blockers",
  "def continuation_frame_chain_obligation",
  "def continuation_frame_chain_obligations",
  "def continuation_one_shot_obligation",
  "def continuation_one_shot_obligations",
  "consumed_state",
  "CaptureSharedRefCell",
  "CaptureWorldLocalRejected",
  "CaptureThreadLocalRejected",
  "CaptureUnsafeRejected",
  "missing-facts: boxed Cont1 capture extraction absent",
  "missing-facts: ContN frame-chain capture extraction absent",
];

function fail(message) {
  console.error("[FAIL] level-1b C10 closure package");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
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
  if (/\beffect\b/i.test(code)) errors.push(`${file}: closure pipeline must not introduce effect naming`);
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|heap_alloc\s*\(|load(?:8|16|32|64)\s*\(/.test(code)) {
    errors.push(`${file}: closure pass leaks Metal/raw memory implementation`);
  }
  if (/\bL[0-9]+Op|\bL[0-9]+Item|\bbackend\.cir\b/.test(code)) {
    errors.push(`${file}: level-1b closure pass must not copy old CIR level tags`);
  }
  if (/\bCpsUseFact\s*\(\s*UseSubjectBinder\s*\(\s*fact\.binder\s*\)\s*,\s*fact\.count\s*\)/.test(code)) {
    errors.push(`${file}: CPS usage must preserve continuation/lambda/closure subject kind`);
  }
  if (/\bContinuationPackaged\s*\(\s*binder\s*\)\s*=>\s*out\.push\s*\(\s*ClosureEnvLayout\s*\(\s*binder\s*,\s*empty_capture_fields\s*\(\s*\)\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: packaged continuations must not materialize empty capture layouts`);
  }
  if (/\bUseMany\s*=>\s*out\.push\s*\(\s*ContinuationPackaged\s*\(\s*binder\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: continuation packaging must not be driven by UseMany alone`);
  }
  if (/\bContinuationDeleted\s*\(\s*BinderId\s*\)/.test(code) || /\bContinuationInlined\s*\(\s*BinderId\s*\)/.test(code)) {
    errors.push(`${file}: continuation simplification must preserve capability on deleted/inlined continuations`);
  }
  if (/\bContinuationBoxedOneShot\s*\([^)]+\)\s*=>\s*ContinuationLowerRepeatableContN/.test(code)) {
    errors.push(`${file}: boxed Cont1 must not lower as repeatable ContN package`);
  }
  if (/\bCaptureSharedRefCell\s*=>\s*false/.test(code)) {
    errors.push(`${file}: captured Ref[T] cells must keep shared-reference package legality, not snapshot rejection`);
  }
  if (/\bContinuationPackaged\s*\([^)]+\)\s*=>\s*Some\s*\(\s*SemanticDiagnostic[\s\S]{0,160}boxed Cont1/.test(code)) {
    errors.push(`${file}: ContN frame-chain blockers must not reuse boxed Cont1 diagnostics`);
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
  if (missing.length !== 0) fail(`missing C10 files:\n${missing.join("\n")}`);

  const joined = files.concat(CONTRACT_FILES).map(read).join("\n");
  const missingText = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missingText.length !== 0) fail(`missing C10 contract text:\n${missingText.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("closure source contract");

  const noCapture = read(NO_CAPTURE_FIXTURE);
  if (!noCapture.includes("{|value: i64| value + 1 }")) {
    fail("no-capture closure fixture must keep immediately-called lambda shape");
  }
  if (!joined.includes("function.no_capture()") || !joined.includes("ClosureNoCaptureDirect")) {
    fail("closure simplification must keep no-capture directification path");
  }
  pass("no-capture directification fixture");

  for (const [file, needle] of [
    [CAPTURE_FIXTURE, "capture_env"],
    [MULTISHOT_FIXTURE, "multi"],
    [UNSAFE_FIXTURE, "unsafe"],
  ]) {
    if (!read(file).includes(needle)) fail(`C10 fixture missing ${needle}: ${file}`);
  }

  if (!joined.includes("ContinuationCapturePlan") || !joined.includes("requires_frame_body_extraction: bool")) {
    fail("C10 source must keep continuation capture/frame extraction obligations");
  }
  if (!joined.includes("ClosureCapturingEnv") || !joined.includes("closure_env_field_from_capture")) {
    fail("C10 source must keep capturing closure env extraction path");
  }
  pass("closure/continuation capture obligations source-locked");
}

main();
