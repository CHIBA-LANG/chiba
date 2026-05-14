import { spawnSync } from "node:child_process";
import crypto from "node:crypto";

const LEVEL1C = "./target/debug/level1c.o";

function run(args) {
  return spawnSync(LEVEL1C, args, {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
}

function output(result) {
  return `${result.stdout}${result.stderr}`;
}

function hasSequence(text, parts) {
  let at = 0;
  for (const part of parts) {
    const next = text.indexOf(part, at);
    if (next < 0) return false;
    at = next + part.length;
  }
  return true;
}

function sha256(text) {
  return crypto.createHash("sha256").update(text).digest("hex");
}

function checkCase(test) {
  const result = run(test.args);
  const text = output(result);
  const digest = sha256(text);
  const ok =
    result.status === (test.status ?? 0) &&
    (test.expect || []).every((part) => text.includes(part)) &&
    (test.sequence || []).every((parts) => hasSequence(text, parts)) &&
    (!test.sha256 || digest === test.sha256);

  if (ok) {
    console.log(`[PASS] ${test.group}: ${test.name}`);
    return 0;
  }

  console.error(`[FAIL] ${test.group}: ${test.name}`);
  console.error(`args: ${test.args.join(" ")}`);
  console.error(`status: ${result.status}`);
  console.error(`sha256: ${digest}`);
  console.error(text.split("\n").slice(0, 20).join("\n"));
  return 1;
}

const SPEC = "chiba-level1-grammar-spec/01-test.chiba";

const CASES = [
  {
    group: "unifier",
    name: "unit smoke + golden dump",
    args: ["type-unify-smoke", SPEC],
    sha256: "1783c879e7d3997998d713d70d233250aef25414a558f3159eccca3ee6d5b74f",
    expect: ["unify-var-concrete", "unify-fn", "unify-tuple", "unify-occurs-error", "unify-nominal-namespace-error"],
  },
  {
    group: "unifier",
    name: "source valid",
    args: ["check", "supports/semantic-gates/type_unify.chiba"],
    expect: ["check ok"],
  },
  {
    group: "unifier",
    name: "source invalid return",
    args: ["check", "supports/semantic-gates/type_unify_invalid_return.chiba"],
    expect: ["return type mismatch"],
  },
  {
    group: "unifier",
    name: "source invalid binary",
    args: ["check", "supports/semantic-gates/type_unify_invalid_binary.chiba"],
    expect: ["expression type mismatch"],
  },
  {
    group: "unifier",
    name: "source invalid let",
    args: ["check", "supports/semantic-gates/type_unify_invalid_let.chiba"],
    expect: ["let type mismatch"],
  },
  {
    group: "l2-check",
    name: "primary L2 checker golden dump",
    args: ["type-l2-check-smoke", SPEC],
    sha256: "7a0a1970ed058365af82ec097f7a7403f34d7d03c253e3e794f065ae24fefdbd",
    expect: ["L2TypeCheckSmoke", "binary-i64-ok ok", "binary-mismatch err expression type mismatch", "atomic-string err unsupported Atomic[T]", "extern-fd-write err wasi fd_write signature mismatch"],
  },
  {
    group: "l2-check",
    name: "typed facts golden dump",
    args: ["type-facts-smoke", SPEC],
    sha256: "1d5cf786c3736fb5f89d7cec6a8243ac8ddb5c6438152c3a140b342eaa012812",
    expect: ["L2TypeFactsSmoke", "typed-ast-result", "constraint-set", "obligation-ir"],
  },
  {
    group: "inference",
    name: "typed source golden dump",
    args: ["typed", "supports/semantic-gates/type_inference.chiba"],
    sha256: "29be03b756fae05d722ff6e47fcd8f8aaa57b0487f34044c0f7ec9649721b9fe",
    expect: ["L2Module", "infer_return", "infer_params", "type $T2", "explicit_and_implicit", "type $T9", "annotated_generic", "type T"],
  },
  {
    group: "inference",
    name: "source invalid ambiguous none",
    args: ["check", "supports/semantic-gates/type_inference_invalid.chiba"],
    expect: ["Ref.new(None) requires explicit Option[T] annotation"],
  },
  {
    group: "inference",
    name: "let generalization golden dump",
    args: ["type-generalize-smoke", SPEC],
    sha256: "03788d71431241d96f28824ebdeedcc8268695103d0296c17387504e6d311779",
    expect: ["L2TypeGeneralizeSmoke", "type-scheme", "scheme-vars-end"],
  },
  {
    group: "generics",
    name: "explicit generic typed golden dump",
    args: ["typed", "supports/semantic-gates/type_generics.chiba"],
    sha256: "951f5445dd653b6a73ea5ad14e9db2a780ff0d8c02e1ab1adce88fe36a471eb9",
    expect: ["L2Module", "id", "type T", "left", "type T"],
  },
  {
    group: "generics",
    name: "rigid return mismatch",
    args: ["check", "supports/semantic-gates/type_generics_invalid_return.chiba"],
    expect: ["return type mismatch"],
  },
  {
    group: "generics",
    name: "duplicate generic parameter",
    args: ["check", "supports/semantic-gates/type_generics_invalid_duplicate.chiba"],
    expect: ["duplicate generic parameter"],
  },
  {
    group: "row",
    name: "row unification golden dump",
    args: ["type-row-smoke", SPEC],
    sha256: "b09f29a647ced9b7c72b173d44f577f83888c17dc2716dacc0b36f82a77ffe83",
    expect: ["same-fields-order-independent 1", "closed-row-extra-error", "open-row-extra-ok"],
  },
  {
    group: "row",
    name: "record typing golden dump",
    args: ["type-record-smoke", SPEC],
    sha256: "a9874a7a85189137c7035a28a20cae80723fba0154c3627b4308868f49af39ac",
    expect: ["record-literal-closed-row", "record-update-row", "record-update-conflict"],
  },
  {
    group: "row",
    name: "row source gates",
    args: ["check", "supports/semantic-gates/row_shape_unify.chiba"],
    expect: ["check ok"],
  },
  {
    group: "row",
    name: "row duplicate record",
    args: ["check", "supports/semantic-gates/row_shape_unify_invalid_record.chiba"],
    expect: ["duplicate record field"],
  },
  {
    group: "row",
    name: "row shorthand typed golden dump",
    args: ["typed", "supports/semantic-gates/row_shorthand.chiba"],
    sha256: "b6dbeaef115f9a098484aeabe33eb0d17cd99d92469dd2dc5cc1b4a78cee9cc5",
    expect: ["L2Module", "get_name", "type $T1", "get_age", "type $T3", "row_shorthand_identity", "type $T5"],
  },
  {
    group: "row",
    name: "row shorthand invalid",
    args: ["check", "supports/semantic-gates/row_shorthand_invalid.chiba"],
    expect: ["row constraint missing field id"],
  },
  {
    group: "checked-template",
    name: "template obligation golden dump",
    args: ["type-template-smoke", SPEC],
    sha256: "46ebb747d8e5afb74dbe293b666f5fd11b576f35a8982bb3f028817c54660be7",
    expect: ["row-bound-shorthand synthetic generic", "obligation field $T0.name: $T1", "obligation operator op_add"],
  },
  {
    group: "checked-template",
    name: "generic body golden dump",
    args: ["type-generic-body-smoke", SPEC],
    sha256: "9e2e5e5e6949da468e72a37ba7250de742380c0b6aed0037d372ce2e6b392eb8",
    expect: ["generic-body concrete-error", "generic-body field-obligation", "generic-instantiation-field-missing", "specialization-key semantic.gates.type_generics::id[T]"],
  },
  {
    group: "checked-template",
    name: "instantiation valid",
    args: ["check", "supports/semantic-gates/checked_template_instantiation.chiba"],
    expect: ["check ok"],
  },
  {
    group: "checked-template",
    name: "instantiation missing field",
    args: ["check", "supports/semantic-gates/checked_template_instantiation_invalid.chiba"],
    expect: ["generic instantiation missing field name"],
  },
  {
    group: "method-operator",
    name: "method routes golden dump",
    args: ["type-method-smoke", SPEC],
    sha256: "563de0ff97edba61accb3f6d190e997a3118832c7a9bf57ca3ccfb0ab32f35de",
    expect: ["method-route field-callable", "method-route nominal-receiver", "method-route qualified-callee"],
  },
  {
    group: "method-operator",
    name: "source valid",
    args: ["check", "supports/semantic-gates/method_resolution.chiba"],
    expect: ["check ok"],
  },
  {
    group: "method-operator",
    name: "source invalid",
    args: ["check", "supports/semantic-gates/method_resolution_invalid.chiba"],
    expect: ["unresolved method missing for Widget"],
  },
  {
    group: "capability-abi",
    name: "capability golden dump",
    args: ["type-capability-smoke", SPEC],
    sha256: "61c58ff7b95646caad9d2e2cd1c1e5304473e09a4d07c8cfb4f32ac11173ab20",
    expect: ["ref-assign-ok ok", "ptr-safe-error err", "atomic-bad err", "abi-bad err"],
  },
  {
    group: "capability-abi",
    name: "extern ABI valid",
    args: ["check", "supports/semantic-gates/extern_abi.chiba"],
    expect: ["check ok"],
  },
  {
    group: "capability-abi",
    name: "extern ABI invalid",
    args: ["check", "supports/semantic-gates/extern_abi_invalid.chiba"],
    expect: ["unsupported extern ABI"],
  },
  {
    group: "capability-abi",
    name: "extern ABI invalid signature",
    args: ["check", "supports/semantic-gates/extern_abi_invalid_signature.chiba"],
    expect: ["wasi fd_write signature mismatch"],
  },
  {
    group: "golden",
    name: "full type smoke golden dump",
    args: ["type-smoke", SPEC],
    sha256: "28a18efd4a5d87d8e816031caef24bc58f060edc91f1f097e4d3a975e9728ed4",
    expect: ["L2TypeSmoke", "constraint eq", "obligation method", "Continuation[i64, i64, multi]"],
  },
];

let failed = 0;
for (const test of CASES) {
  failed += checkCase(test);
}

if (failed !== 0) process.exit(1);
