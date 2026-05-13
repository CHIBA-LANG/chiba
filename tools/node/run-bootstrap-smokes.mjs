import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const WAT_DIR = ".scratch/bootstrap-smokes/wat";

function run(command, args, options = {}) {
  return spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    ...options,
  });
}

const USE_BINARYEN_OPT = process.argv.includes("--opt");

function outputHasSequence(output, sequence) {
  let at = 0;
  for (const part of sequence) {
    const next = output.indexOf(part, at);
    if (next < 0) return false;
    at = next + part.length;
  }
  return true;
}

function checkOutput(name, result, expect, status = 0, expectSequence = [], reject = []) {
  const output = `${result.stdout}${result.stderr}`;
  const ok =
    result.status === status &&
    expect.every((line) => output.includes(line)) &&
    expectSequence.every((sequence) => outputHasSequence(output, sequence)) &&
    reject.every((line) => !output.includes(line));

  if (ok) {
    console.log(`[PASS] ${name}`);
    return 0;
  }

  console.error(`[FAIL] ${name}`);
  console.error(output.split("\n").slice(0, 18).join("\n"));
  return 1;
}

function runWatFile(test) {
  const result = run(process.execPath, [
    "--no-warnings",
    "tools/node/run-wat.mjs",
    test.file,
    ...(USE_BINARYEN_OPT ? ["--opt"] : []),
    ...(test.args || []),
  ]);
  return checkOutput(test.name, result, test.expect, test.status || 0);
}

function runLevel1c(test) {
  const result = run("./target/debug/level1c.o", test.args);
  return checkOutput(test.name, result, test.expect, 0, test.expectSequence || [], test.reject || []);
}

function watName(file) {
  return file.replaceAll("/", "__").replace(/\.chiba$/, ".wat");
}

function writeGeneratedWat(test, wat) {
  fs.mkdirSync(WAT_DIR, { recursive: true });
  fs.writeFileSync(path.join(WAT_DIR, watName(test.file)), wat);
}

function runGeneratedWat(test) {
  const generated = run("./target/debug/level1c.o", ["wat", test.file]);
  if (generated.status !== 0) {
    return checkOutput(test.name, generated, test.expect);
  }
  writeGeneratedWat(test, generated.stdout);

  const result = run(
    process.execPath,
    [
      "--no-warnings",
      "tools/node/run-wat.mjs",
      "-",
      ...(USE_BINARYEN_OPT ? ["--opt"] : []),
    ],
    { input: generated.stdout },
  );
  return checkOutput(test.name, result, test.expect);
}

function checkGeneratedWatText(test) {
  const generated = run("./target/debug/level1c.o", ["wat", test.file]);
  if (generated.status === 0) writeGeneratedWat(test, generated.stdout);
  return checkOutput(test.name, generated, test.expect);
}

const WAT_CASES = [
  {
    name: "env import",
    file: "supports/bootstrap/wat-env-import-smoke.wat",
    expect: ["env.js_log 41", "9"],
  },
  {
    name: "wasi fd_write",
    file: "supports/bootstrap/wat-wasi-import-smoke.wat",
    expect: ["B04 wasi smoke ok", "0"],
  },
  {
    name: "wasi file read",
    file: "supports/bootstrap/wat-wasi-file-read-smoke.wat",
    expect: ["66"],
  },
  {
    name: "wasi args env",
    file: "supports/bootstrap/wat-wasi-args-env-smoke.wat",
    args: ["--arg", "alpha", "--arg", "beta", "--env", "CHIBA_WASI_SMOKE=ok"],
    expect: ["301"],
  },
  {
    name: "wasi exit status",
    file: "supports/bootstrap/wat-wasi-exit-status-smoke.wat",
    expect: [],
    status: 7,
  },
  {
    name: "default _start",
    file: "supports/bootstrap/wat-start-smoke.wat",
    expect: ["12"],
  },
];

const LEVEL1C_CASES = [
  {
    name: "level1c help",
    args: ["--help"],
    expect: ["Usage: level1c <command> <file>", "Commands: lex parse check check-project cir typed type-smoke type-capability-smoke type-generalize-smoke type-generic-body-smoke type-kind-smoke type-l2-check-smoke type-method-smoke type-nominal-smoke type-record-smoke type-row-smoke type-template-smoke type-unify-smoke cps nanopass core-invalid-smoke cont-usage wat"],
  },
  {
    name: "level1c parse grammar 01",
    args: ["parse", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: ["OK(", "SourceFile("],
  },
  {
    name: "level1c typed grammar 01",
    args: ["typed", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: ["L2Module", "L2OpTyped", "type i64", "0"],
    expectSequence: [["L2StmtReturn", "type i64", "L1RefLocal(#1 \"value\")"]],
  },
  {
    name: "level1c type system smoke",
    args: ["type-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeSmoke",
      "tyvar-meta $T0 kind=value level=0 scope=1 visibility=user rigidity=rigid origin=user-generic T",
      "tyvar-meta $T1 kind=value level=1 scope=1 visibility=synthetic rigidity=flexible origin=implicit-param value",
      "row-meta row#7 closed=0 tail=$T2",
      "row-field name: str",
      "type ((i64, bool), row#7) => nominal#3 semantic.gates::User[$T0]",
      "type Ptr[Atomic[usize]]",
      "type Continuation[i64, bool, linear]",
      "constraint eq $T1 == Ref[String]",
      "constraint field-type row#7.name: str",
      "constraint abi i32 wasi",
      "obligation field row#7.name: str",
      "obligation operator op_add self=$T0 args=($T0) => $T0 source=default-visible",
      "obligation method nominal#3 semantic.gates::User[].len() => usize source=qualified semantic.gates.User.len",
      "obligation continuation-capability Continuation[i64, i64, multi] multi",
      "subst $T1 := Ref[String]",
      "0",
    ],
  },
  {
    name: "level1c type unifier smoke",
    args: ["type-unify-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "unify-var-concrete",
      "subst $T0 := i64",
      "unify-fn",
      "subst $T0 := bool",
      "unify-tuple",
      "subst $T1 := String",
      "unify-occurs-error",
      "occurs check failed",
      "unify-nominal-namespace-error",
      "nominal mismatch",
      "0",
    ],
  },
  {
    name: "level1c type kind smoke",
    args: ["type-kind-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeKindSmoke",
      "kind-i64 ok value",
      "kind-row ok row",
      "kind-ref ok capability",
      "kind-continuation ok continuation",
      "kind-abi-ptr ok abi-scalar",
      "kind-row-as-value err type kind mismatch",
      "kind-string-as-abi err not an ABI scalar type",
      "0",
    ],
  },
  {
    name: "level1c type generalize smoke",
    args: ["type-generalize-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeGeneralizeSmoke",
      "tyvar-meta $T0 kind=value level=1 scope=1 visibility=synthetic rigidity=flexible origin=let id",
      "type ($T0) => $T0",
      "type Ref[$T1]",
      "0",
    ],
    expectSequence: [["type ($T0) => $T0", "type-scheme", "scheme-vars-end", "type Ref[$T1]"]],
  },
  {
    name: "level1c type capability smoke",
    args: ["type-capability-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeCapabilitySmoke",
      "ref-assign-ok ok",
      "ref-assign-bad err Ref assignment type mismatch",
      "ptr-safe-error err Ptr requires unsafe block",
      "ptr-unsafe-ok ok",
      "atomic-ok ok",
      "atomic-bad err unsupported Atomic[T]",
      "abi-ok ok",
      "abi-bad err not an ABI scalar type",
      "0",
    ],
  },
  {
    name: "level1c type L2 check smoke",
    args: ["type-l2-check-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeCheckSmoke",
      "binary-i64-ok ok",
      "binary-mismatch err expression type mismatch",
      "0",
    ],
  },
  {
    name: "level1c type generic body smoke",
    args: ["type-generic-body-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeGenericBodySmoke",
      "generic-body concrete-error err definition-time type mismatch",
      "generic-body field-obligation checked $T1",
      "obligation field $T0.name: $T1",
      "generic-body operator-obligation checked $T2",
      "obligation operator op_add self=$T2 args=($T2) => $T2 source=default-visible",
      "specialization-key semantic.gates.type_generics::id[T]",
      "generic-instantiation-field-ok ok",
      "generic-instantiation-field-missing err missing generic field obligation",
      "0",
    ],
  },
  {
    name: "level1c type row smoke",
    args: ["type-row-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeRowSmoke",
      "same-fields-order-independent 1",
      "closed-row-unify",
      "subst $T0 := str",
      "closed-row-extra-error err closed row missing field",
      "open-row-extra-ok",
      "0",
    ],
  },
  {
    name: "level1c type nominal smoke",
    args: ["type-nominal-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeNominalSmoke",
      "type nominal#1 a.ns::User[]",
      "type nominal#2 b.ns::User[]",
      "same-row-shape 1",
      "nominal-unify-error nominal mismatch",
      "0",
    ],
  },
  {
    name: "level1c type record smoke",
    args: ["type-record-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeRecordSmoke",
      "record-key-order-independent 1",
      "record-literal-closed-row",
      "row-meta row#42 closed=1 tail=Unit",
      "record-literal-duplicate err duplicate record field",
      "record-update-row",
      "row-meta row#45 closed=1 tail=Unit",
      "row-field z: String",
      "record-update-duplicate err duplicate record update field",
      "record-update-conflict err record update field type conflict",
      "record-update-non-record err record update requires record-like value",
      "0",
    ],
  },
  {
    name: "level1c type method smoke",
    args: ["type-method-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeMethodSmoke",
      "method-route field-callable",
      "method-route nominal-receiver",
      "method-route qualified-callee",
      "method-entry nominal#30 semantic.gates.method::Widget.size (nominal#30 semantic.gates.method::Widget[]) => i64",
      "method-duplicate-error duplicate method definition",
      "0",
    ],
  },
  {
    name: "level1c type template smoke",
    args: ["type-template-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: [
      "L2TypeTemplateSmoke",
      "row-bound-shorthand synthetic generic",
      "tyvar-meta $T0 kind=value level=0 scope=1 visibility=synthetic rigidity=flexible origin=implicit-param value",
      "row-meta row#20 closed=0 tail=$T2",
      "obligation field $T0.name: $T1",
      "obligation method $T0.len() => usize source=default-visible",
      "obligation operator op_add self=$T0 args=($T0) => $T0 source=default-visible",
      "obligation shape-dispatch dispatch self=$T0 args=() => $T1 source=via local",
      "0",
    ],
  },
  {
    name: "level1c nanopass grammar 01",
    args: ["nanopass", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: ["L8ValidatedCoreModule", "L8ValidatedCoreOp", "L7CoreOp", "L6OpClosureEnv", "L5OpCps", "L4OpUsage", "L3OpAnswerControl", "validation ok", "0"],
  },
  {
    name: "level1c nanopass string slice core",
    args: ["nanopass", "supports/semantic-gates/string_slice.chiba"],
    expect: ["L8ValidatedCoreModule", "L7CoreOp", "core-op string-slice", "type Slice[u8]", "validation ok", "0"],
  },
  {
    name: "level1c nanopass continuation core",
    args: ["nanopass", "supports/bootstrap/continuation-multi-resume.chiba"],
    expect: ["L8ValidatedCoreModule", "L7CoreOp", "core-op continuation-package", "L5OpContinuationPackage", "validation ok", "0"],
    expectSequence: [[
      "L6OpClosureEnv",
      "usage many",
      "L5OpContinuationPackage",
      "usage many",
    ], [
      "L4OpUsage",
      "usage many",
      "L3OpAnswerControl",
      "control-boundary delimited",
    ], [
      "L3OpAnswerControl",
      "control-boundary delimited",
      "L1OpReset",
      "L3OpAnswerControl",
      "control-boundary shift",
      "L1OpShift(#1 \"k\")",
    ]],
    reject: ["usage unknown"],
  },
  {
    name: "level1c core validator rejects invalid core",
    args: ["core-invalid-smoke", "chiba-level1-grammar-spec/01-test.chiba"],
    expect: ["L8ValidatedCoreModule", "L8ValidatedCoreOp", "validation err(\"missing L7CoreOp\")"],
  },
  {
    name: "level1c cps continuation multi resume",
    args: ["cps", "supports/bootstrap/continuation-multi-resume.chiba"],
    expect: ["L5Module", "L5OpContinuationPackage", "L5OpCps", "L4OpUsage", "control-boundary", "0"],
    expectSequence: [["L5OpContinuationPackage", "usage many", "L5OpCps", "usage many", "L4OpUsage", "usage many"]],
    reject: ["usage unknown"],
  },
  {
    name: "level1c check continuation valid",
    args: ["check", "supports/bootstrap/continuation-valid.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c check continuation nested reset",
    args: ["check", "supports/bootstrap/continuation-nested.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c check continuation multi resume",
    args: ["check", "supports/bootstrap/continuation-multi-resume.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c continuation usage multi resume",
    args: ["cont-usage", "supports/bootstrap/continuation-multi-resume.chiba"],
    expect: ["continuation #1 \"k\" resumes=2 class=many", "0"],
  },
  {
    name: "level1c check continuation invalid",
    args: ["check", "supports/bootstrap/continuation-invalid.chiba"],
    expect: ["shift outside reset", "3"],
  },
  {
    name: "level1c check continuation cross world invalid",
    args: ["check", "supports/bootstrap/continuation-cross-world-invalid.chiba"],
    expect: ["continuation crosses world/thread boundary", "3"],
  },
  {
    name: "level1c check Ref Atomic valid",
    args: ["check", "supports/semantic-gates/refs_atomic_valid.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c check Ref Atomic invalid",
    args: ["check", "supports/semantic-gates/refs_atomic_invalid.chiba"],
    expect: ["top-level Ref requires #[world_local]", "3"],
  },
  {
    name: "level1c check method routes valid",
    args: ["check", "supports/semantic-gates/method_resolution.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c check method invalid",
    args: ["check", "supports/semantic-gates/method_resolution_invalid.chiba"],
    expect: ["unresolved method missing for Widget", "3"],
  },
  {
    name: "level1c check row poly valid",
    args: ["check", "supports/semantic-gates/row_poly.chiba"],
    expect: ["check ok", "0"],
  },
  {
    name: "level1c check row poly invalid",
    args: ["check", "supports/semantic-gates/row_poly_invalid.chiba"],
    expect: ["row constraint missing field id", "3"],
  },
];

const GENERATED_WAT_CASES = [
  {
    name: "generated wat grammar 01",
    file: "chiba-level1-grammar-spec/01-test.chiba",
    expect: ["1"],
  },
  {
    name: "generated wat loop",
    file: "supports/bootstrap/wat-loop-smoke.chiba",
    expect: ["7"],
  },
  {
    name: "generated wat assign",
    file: "supports/bootstrap/wat-assign-smoke.chiba",
    expect: ["7"],
  },
  {
    name: "generated wat tailcall",
    file: "supports/bootstrap/wat-tailcall-smoke.chiba",
    expect: ["0"],
  },
  {
    name: "generated wat tuple heap",
    file: "supports/bootstrap/wat-tuple-heap-smoke.chiba",
    expect: ["41"],
  },
  {
    name: "generated wat extern env",
    file: "supports/bootstrap/wat-extern-env-smoke.chiba",
    expect: ["env.js_log 41", "9"],
  },
];

const GENERATED_WAT_TEXT_CASES = [
  {
    name: "generated wat extern wasi import",
    file: "supports/bootstrap/wat-extern-wasi-smoke.chiba",
    expect: ['(import "wasi_snapshot_preview1" "fd_write" (func $wasi_fd_write'],
  },
];

let failed = 0;

for (const test of WAT_CASES) {
  failed += runWatFile(test);
}

for (const test of LEVEL1C_CASES) {
  failed += runLevel1c(test);
}

for (const test of GENERATED_WAT_CASES) {
  failed += runGeneratedWat(test);
}

for (const test of GENERATED_WAT_TEXT_CASES) {
  failed += checkGeneratedWatText(test);
}

if (failed !== 0) {
  process.exit(1);
}
