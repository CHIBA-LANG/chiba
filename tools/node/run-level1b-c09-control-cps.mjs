import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const ROOT = "level-1b/compiler/control";
const CONTRACT_FILES = [
  "level-1b/compiler/ir/type_ir.chiba",
  "level-1b/compiler/ir/control.chiba",
];
const REQUIRED_FILES = [
  "answer_control.chiba",
  "answer_type.chiba",
  "continuation_boundary.chiba",
  "continuation_usage.chiba",
  "cps.chiba",
  "driver.chiba",
  "replay_safety.chiba",
  "usage_subject.chiba",
];
const REQUIRED_TEXT = [
  "data ControlBoundary",
  "data AnswerTypeCheck",
  "def check_answer_type",
  "def check_answer_control",
  "control_surface.len() > 0",
  "type AnswerControlObligation",
  "requires_reset_answer: bool",
  "requires_shift_answer: bool",
  "requires_multishot_answer: bool",
  "def answer_control_obligation_from_surface",
  "def answer_control_obligations",
  "def typed_items_have_control_surface",
  "def answer_facts_none",
  "data UsageCount",
  "data UsageSubject",
  "def UsageCount.merge",
  "def usage_facts_none",
  "data ContinuationKind",
  "ContinuationCont1",
  "ContinuationContN",
  "data ContinuationStorageKind",
  "ContinuationBoxedOneShot",
  "ContinuationRepeatableFrameChain",
  "type ContinuationFact",
  "data CallableArrowPosition",
  "data CallableStorageVariant",
  "type CallableStorageFact",
  "def CallableStorageFact.excludes_continuations_for_send",
  "def analyze_usage",
  "data ContinuationBoundary",
  "type ContinuationBoundaryObligation",
  "requires_world_boundary_check: bool",
  "requires_thread_boundary_check: bool",
  "requires_send_boundary_check: bool",
  "def continuation_boundary_obligation_from_usage",
  "def continuation_boundary_obligations",
  "def check_continuation_boundary",
  "data ControlBoundaryError",
  "ControlCrossWorld",
  "ControlCrossThread",
  "ControlNonReplayState",
  "def check_replay_safety",
  "def replay_safety_facts_none",
  "type ContNReplaySafetyObligation",
  "requires_capture_classification: bool",
  "ref_capture_uses_shared_reference_semantics: bool",
  "def usage_count_requires_replay_check",
  "def contn_replay_safety_obligation_from_usage",
  "def contn_replay_safety_obligations",
  "data ReplayCaptureKind",
  "ReplayCaptureSharedRefCell",
  "def ReplayCaptureKind.safe_for_contn_replay",
  "def ReplayCaptureKind.uses_shared_reference_semantics",
  "missing-lowering: ContN replay-safety capture classification absent",
  "missing-lowering: continuation world/thread/send boundary scan absent",
  "def one_pass_cps",
  "data CpsAtom",
  "data CpsTermKind",
  "type CpsTermFact",
  "CpsTermReturn",
  "CpsTermTailCall",
  "CpsTermBranch",
  "CpsTermApplyContinuation",
  "CpsTermPendingJoin",
  "terms: Array[CpsTermFact]",
  "tail_position: bool",
  "def cps_term_from_typed_expr",
  "def cps_term_fact_from_function",
  "def cps_term_facts_from_functions",
  "type CpsTailFormFact",
  "data CpsTailFormKind",
  "CpsTailReturn",
  "CpsTailCall",
  "CpsTailBranchCall",
  "CpsMaterializedMultiShotFrame",
  "all_non_multishot_calls_are_tail: bool",
  "tail_forms: Array[CpsTailFormFact]",
  "branch_joins: Array[CpsBranchJoinFact]",
  "pattern_decisions: Array[CpsPatternDecisionFact]",
  "adt_ctors: Array[CpsAdtCtorLoweringFact]",
  "type CpsAdtCtorLoweringFact",
  "requires_tail_tuple_return: bool",
  "requires_utf8_identifier_lowering: bool",
  "def cps_adt_ctor_lowering_fact_from_ctor",
  "def cps_adt_ctor_lowering_facts",
  "def cps_tail_form_kind_from_function",
  "def cps_tail_form_kind_from_expr",
  "def cps_tail_form_kind_from_branch",
  "def cps_tail_form_fact_from_function",
  "def cps_tail_form_facts_from_functions",
  "def cps_tail_form_fact_from_continuation",
  "def cps_tail_form_facts_from_continuations",
  "def cps_tail_form_facts_copy",
  "data BranchJoinKind",
  "BranchJoinIfElse",
  "BranchJoinElseIf",
  "BranchJoinIfLet",
  "BranchJoinMatch",
  "BranchJoinShortCircuit",
  "type BranchJoinPlan",
  "type CpsBranchJoinFact",
  "type CpsPatternDecisionFact",
  "requires_test_chain: bool",
  "lowers_to_if_else_and_field_extract: bool",
  "failure_env_has_no_bindings: bool",
  "missing_static_case_is_error: bool",
  "def pattern_exhaustiveness_requires_error",
  "def pattern_failure_env_has_no_bindings",
  "def cps_pattern_decision_fact_from_step",
  "def cps_pattern_decision_facts_from_steps",
  "def cps_pattern_decision_facts",
  "def branch_join_plan_from_surface",
  "def branch_join_plans_from_surface",
  "def branch_join_plans",
  "def branch_join_kind_from_plan",
  "def cps_branch_join_fact_from_plan",
  "def cps_branch_join_facts_from_plans",
  "all_arms_resume_to_same_tail: bool",
  "def cps_continuation_facts_none",
  "def run_control_cps",
];

const VALID = [
  "supports/bootstrap/continuation-valid.chiba",
  "supports/bootstrap/continuation-nested.chiba",
  "supports/bootstrap/continuation-multi-resume.chiba",
  "supports/bootstrap/continuation-lexer-backtracking.chiba",
  "supports/bootstrap/continuation-parser-recovery.chiba",
  "supports/semantic-gates/continuation_scheme_multi.chiba",
];
const INVALID = [
  ["supports/bootstrap/continuation-answer-mismatch-invalid.chiba", "answer type mismatch"],
  ["supports/bootstrap/continuation-cross-world-invalid.chiba", "continuation crosses world/thread boundary"],
  ["supports/bootstrap/continuation-non-replay-invalid.chiba", "multi-resume captures non-replay state"],
];

function fail(message) {
  console.error("[FAIL] level-1b C09 control/cps");
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
  if (/\beffect\b/i.test(code)) errors.push(`${file}: C09 must not introduce effect naming`);
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|heap_alloc\s*\(|load(?:8|16|32|64)\s*\(/.test(code)) {
    errors.push(`${file}: control pass leaks Metal/raw memory implementation`);
  }
  if (/\bdef\s+check_answer_control\b[\s\S]*Vec\s*\[\s*AnswerFact\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)/.test(code)) {
    errors.push(`${file}: answer/control pass must not emit empty AnswerFact stream`);
  }
  if (/\bdef\s+analyze_usage\b[\s\S]*Vec\s*\[\s*UsageFact\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)/.test(code)) {
    errors.push(`${file}: continuation usage pass must not emit empty UsageFact stream`);
  }
  if (/\bdef\s+check_continuation_boundary\b[\s\S]*=\s*Ok\s*\(\s*module\s*\)/.test(code)) {
    errors.push(`${file}: continuation boundary pass must not return module unchanged`);
  }
  if (/\bdef\s+one_pass_cps\b[\s\S]*=\s*CpsModule\s*\(\s*module\s*\)/.test(code)) {
    errors.push(`${file}: one-pass CPS must not be a wrapper-only implementation`);
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
  if (missing.length !== 0) fail(`missing C09 files:\n${missing.join("\n")}`);

  const joined = files.concat(CONTRACT_FILES).map(read).join("\n");
  const missingText = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missingText.length !== 0) fail(`missing C09 contract text:\n${missingText.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("control/cps source contract");

  primaryPathBlocked("valid continuation gates require level-1b semantic checker execution");
  primaryPathBlocked("invalid continuation gates require level-1b semantic checker diagnostics");
}

main();
