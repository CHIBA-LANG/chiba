import fs from "node:fs";
import process from "node:process";

const SCAN_ROOTS = ["level-1b/compiler"];
const HARNESS_ROOTS = ["tools/node"];
const PRIORITY = new Map([
  ["legacy-dependency", 0],
  ["oracle-dependency", 1],
  ["missing-backend-layout", 2],
  ["missing-lowering", 3],
  ["missing-facts", 4],
  ["primary-path-blocked", 5],
]);

const CHECKS = [
  {
    id: "comment-only-backend-output",
    file: "level-1b/compiler/backend/wat_emit.chiba",
    blocker: "missing-backend-layout",
    semantic: "WAT emitter represents Core operations as comments instead of executable backend output",
    detect(source) {
      return /def\s+emit_core_op\b[\s\S]*?=\s*[\s\S]*"\s*;;\s*core-op/.test(source);
    },
  },
];

function listFiles(dir, suffix = ".chiba") {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = `${dir}/${entry.name}`;
    if (entry.isDirectory()) out.push(...listFiles(file, suffix));
    else if (entry.isFile() && entry.name.endsWith(suffix)) out.push(file);
  }
  return out.sort();
}

function fail(message) {
  console.error("[FAIL] level-1b truthfulness audit");
  console.error(message);
  process.exit(1);
}

function read(file) {
  try {
    return fs.readFileSync(file, "utf8");
  } catch (error) {
    fail(`missing-audit-input: ${file}: ${error.message}`);
  }
}

function lineOf(source, index) {
  return source.slice(0, index).split("\n").length;
}

const blockers = [];

for (const check of CHECKS) {
  const source = read(check.file);
  if (check.detect(source)) {
    blockers.push({
      ...check,
      message: `${check.blocker}: ${check.semantic}`,
    });
  }
}

for (const root of SCAN_ROOTS) {
  for (const file of listFiles(root)) {
    const source = read(file);
    const passThrough = /\bdef\s+[A-Za-z0-9_.*]+\s*\([^)]*\bmodule\s*:[^)]*\)[\s\S]*?=\s*Ok\s*\(\s*module\s*\)/g;
    for (const match of source.matchAll(passThrough)) {
      blockers.push({
        id: "ok-module-pass-through",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "missing-lowering",
        semantic: "pass returns input module unchanged on compiler path",
        message: "missing-lowering: pass returns input module unchanged on compiler path",
      });
    }

    const emptyFacts = /Vec\s*\[\s*([A-Za-z0-9_]*Fact[A-Za-z0-9_]*)\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)/g;
    for (const match of source.matchAll(emptyFacts)) {
      blockers.push({
        id: "empty-facts",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "missing-facts",
        semantic: `${match[1]} stream is created empty instead of derived from source semantics`,
        message: `missing-facts: ${match[1]} stream is created empty instead of derived from source semantics`,
      });
    }

    const cpsWrapper = /\bdef\s+one_pass_cps\b[\s\S]*?=\s*CpsModule\s*\(\s*module\s*\)/g;
    for (const match of source.matchAll(cpsWrapper)) {
      blockers.push({
        id: "one-pass-cps-wrapper",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "missing-lowering",
        semantic: "one-pass CPS with administrative beta-reduction returns a wrapper without transforming control",
        message: "missing-lowering: one-pass CPS with administrative beta-reduction returns a wrapper without transforming control",
      });
    }

    const sourceGatePassThrough = /\bdef\s+check_source_semantic_gates\b[\s\S]*?SourceGateResult\s*\(\s*project\s*,\s*Vec\s*\[\s*SourceGateError\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)\s*\)/g;
    for (const match of source.matchAll(sourceGatePassThrough)) {
      blockers.push({
        id: "source-gate-empty-pass-through",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "missing-facts",
        semantic: "source semantic gates pass through without parsed source facts",
        message: "missing-facts: source semantic gates pass through without parsed source facts",
      });
    }

    const typeInferPassThrough = /\bdef\s+infer_types\b[\s\S]*?Ok\s*\(\s*TypedModule\s*\(\s*module\s*\)\s*\)/g;
    for (const match of source.matchAll(typeInferPassThrough)) {
      blockers.push({
        id: "type-inference-pass-through",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "missing-facts",
        semantic: "type inference passes an alpha module through as a typed module",
        message: "missing-facts: type inference passes an alpha module through as a typed module",
      });
    }

    const emptySurfaceLowering = /\bdef\s+lower_ast_to_surface\b[\s\S]*?Ok\s*\(\s*SurfaceModule\s*\(\s*""\s*,\s*Vec\s*\[\s*SurfaceItem\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)\s*\)\s*\)/g;
    for (const match of source.matchAll(emptySurfaceLowering)) {
      blockers.push({
        id: "empty-surface-lowering",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "missing-lowering",
        semantic: "AST lowering returns an empty SurfaceModule instead of source items",
        message: "missing-lowering: AST lowering returns an empty SurfaceModule instead of source items",
      });
    }

    const emptyCapturePackage = /ContinuationPackaged\s*\(\s*binder\s*\)\s*=>\s*out\.push\s*\(\s*ClosureEnvLayout\s*\(\s*binder\s*,\s*empty_capture_fields\s*\(\s*\)\s*\)\s*\)/g;
    for (const match of source.matchAll(emptyCapturePackage)) {
      blockers.push({
        id: "packaged-continuation-empty-captures",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "missing-facts",
        semantic: "packaged continuation layout materializes with empty captures instead of extracted capture set",
        message: "missing-facts: packaged continuation layout materializes with empty captures instead of extracted capture set",
      });
    }

    const cpsUsageLosesSubject = /CpsUseFact\s*\(\s*UseSubjectBinder\s*\(\s*fact\.binder\s*\)\s*,\s*fact\.count\s*\)/g;
    for (const match of source.matchAll(cpsUsageLosesSubject)) {
      blockers.push({
        id: "cps-usage-loses-continuation-subject",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "missing-facts",
        semantic: "CPS usage conversion maps all usage facts to binders and loses continuation/lambda/closure subject kind",
        message: "missing-facts: CPS usage conversion maps all usage facts to binders and loses continuation/lambda/closure subject kind",
      });
    }

    const useManyPackages = /UseMany\s*=>\s*out\.push\s*\(\s*ContinuationPackaged\s*\(\s*binder\s*\)\s*\)/g;
    for (const match of source.matchAll(useManyPackages)) {
      blockers.push({
        id: "usemany-packages-without-cont-kind",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "missing-lowering",
        semantic: "continuation packaging is driven by UseMany alone without preserving Cont1/ContN storage semantics",
        message: "missing-lowering: continuation packaging is driven by UseMany alone without preserving Cont1/ContN storage semantics",
      });
    }
  }
}

for (const root of HARNESS_ROOTS) {
  for (const file of listFiles(root, ".mjs")) {
    if (!/run-level1b-/.test(file)) continue;
    const source = read(file);
    const oracleSuccess = /\b(?:pass|runGate)\s*\(\s*["']([^"']*oracle[^"']*)["']/gi;
    for (const match of source.matchAll(oracleSuccess)) {
      blockers.push({
        id: "oracle-success-path",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "oracle-dependency",
        semantic: `level-1b harness reports oracle-backed success path '${match[1]}'`,
        message: `oracle-dependency: level-1b harness reports oracle-backed success path '${match[1]}'`,
      });
    }

    const legacyExecution = /["'](\.\/target\/debug\/level1c\.o|\.\/chibac_amd64-unknown-linux_chiba_dev\.o)["']/g;
    for (const match of source.matchAll(legacyExecution)) {
      blockers.push({
        id: "legacy-compiler-execution",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "legacy-dependency",
        semantic: `level-1b harness executes legacy compiler '${match[1]}' in its success path`,
        message: `legacy-dependency: level-1b harness executes legacy compiler '${match[1]}' in its success path`,
      });
    }

    const primaryPathBlocker = /primaryPathBlocked\s*\(\s*["']([^"']+)["']\s*\)/g;
    for (const match of source.matchAll(primaryPathBlocker)) {
      blockers.push({
        id: "primary-path-blocked",
        file: `${file}:${lineOf(source, match.index)}`,
        blocker: "primary-path-blocked",
        semantic: `level-1b harness names an unavailable primary execution path '${match[1]}'`,
        message: `primary-path-blocked: ${match[1]}`,
      });
    }
  }
}

if (blockers.length !== 0) {
  console.error("[FAIL] level-1b truthfulness audit");
  const sorted = blockers.sort((a, b) => {
    const byPriority = (PRIORITY.get(a.blocker) ?? 99) - (PRIORITY.get(b.blocker) ?? 99);
    if (byPriority !== 0) return byPriority;
    return a.file.localeCompare(b.file);
  });
  for (const blocker of sorted) {
    console.error(`${blocker.file}: ${blocker.message}`);
    console.error(`  check=${blocker.id}`);
  }
  process.exit(1);
}

console.log("[PASS] level-1b truthfulness audit");
