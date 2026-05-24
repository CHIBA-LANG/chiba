import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";

const ROOT = "level-1b/compiler/semantic";
const REQUIRED_FILES = [
  "abi_capability.chiba",
  "adt_tuple_lowering.chiba",
  "alpha.chiba",
  "capability_rules.chiba",
  "driver.chiba",
  "generic_body.chiba",
  "method_operator.chiba",
  "pattern.chiba",
  "template.chiba",
  "type_generalize.chiba",
  "type_facts.chiba",
  "type_infer.chiba",
  "type_kind.chiba",
  "type_nominal.chiba",
  "type_record.chiba",
  "type_row.chiba",
  "type_unify.chiba",
  "typed_elaboration.chiba",
  "types.chiba",
];
const REQUIRED_TEXT = [
  "def alpha_convert",
  "def alpha_binders_from_source_items",
  "type AlphaBinderOrigin",
  "owner_namespace: Option[NamespaceNameSlice]",
  "attributes: SourceItemAttributeFacts",
  "surface: SourceItemSurfaceFacts",
  "def alpha_origins_from_source_items",
  "def elaborate_patterns",
  "def pattern_ids_from_binders",
  "def unify",
  "def unify_rows",
  "def check_type_kind",
  "def canonicalize_row",
  "def generalize_type",
  "def infer_types",
  "type TypedItemSkeleton",
  "data TypedItemSkeletonKind",
  "TypedItemStaticValue",
  "def classify_typed_item_skeleton_kind",
  "def typed_item_skeletons_from_alpha",
  "def type_inference_has_pattern_coverage",
  "def typed_item_skeletons_have_callable_storage_surface",
  "missing-facts: callable storage fact generation absent",
  "missing-lowering: source item type expression and body inference absent",
  "def check_l2_types",
  "data AggregateKind",
  "def build_aggregate_shape",
  "type TypedElaboration",
  "def elaborate_typed_module",
  "data SemanticConstraint",
  "data SemanticObligation",
  "def build_typed_facts",
  "def check_ref_assignment",
  "def check_atomic_payload",
  "def check_unsafe_type",
  "def check_sendable_callable_storage",
  "def check_sendable_callable_storages",
  "def capability_check_diagnostic",
  "callable_storage: Array[CallableStorageFact]",
  "sendable callable storage excludes continuations",
  "data TemplateObligation",
  "def check_template",
  "data GenericBodyCheck",
  "def check_generic_add_body",
  "def instantiate_field_obligation",
  "type MethodKey",
  "type OperatorKey",
  "def build_method_operator_index",
  "data ExternAbi",
  "data CapabilityUse",
  "def check_extern_abi",
  "def check_capabilities",
  "type AdtVariantTag",
  "type AdtTupleShape",
  "data ConstructorLoweringStrategy",
  "ConstructorMutateOnceUsedInput",
  "def constructor_mutation_obligation",
  "data AdtTupleConversionKind",
  "def lower_adt_ctor",
  "def lower_adt_ctors",
  "def run_typed_semantics",
  "stable_sort",
];

function fail(message) {
  console.error("[FAIL] level-1b C08 semantic");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function oracleReference(name) {
  console.log(`[ORACLE] ${name}`);
}

function oracleReferenceFailed(name) {
  console.log(`[ORACLE-FAIL] ${name}`);
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
  if (/\beffect\b/i.test(code)) errors.push(`${file}: semantic C08 must not introduce effect naming`);
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|heap_alloc\s*\(|load(?:8|16|32|64)\s*\(/.test(code)) {
    errors.push(`${file}: semantic pass leaks Metal/raw memory implementation`);
  }
  if (/\bdef\s+infer_types\b[\s\S]*Ok\s*\(\s*TypedModule\s*\(\s*module\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: type inference must not pass through an untyped alpha module`);
  }
  if (file.endsWith("alpha.chiba") && /\bdef\s+alpha_convert\b[\s\S]*AlphaModule\s*\(\s*Vec\s*\[\s*BinderId\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: alpha conversion must derive binders from source item facts`);
  }
  if (file.endsWith("pattern.chiba") && /\bdef\s+elaborate_patterns\b[\s\S]*PatternFacts\s*\(\s*Vec\s*\[\s*PatternId\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: pattern elaboration must derive facts from alpha binders`);
  }
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${file}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function runGate(label, script, timeoutSeconds) {
  const result = spawnSync("timeout", [String(timeoutSeconds), "vp", "run", script], {
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
  });
  if (result.status === 0) oracleReference(label);
  else oracleReferenceFailed(label);
}

function main() {
  const files = listChiba(ROOT);
  const seen = new Set(files.map((file) => path.basename(file)));
  const missing = REQUIRED_FILES.filter((file) => !seen.has(file));
  if (missing.length !== 0) fail(`missing semantic files:\n${missing.join("\n")}`);

  const joined = files.map(read).join("\n");
  const missingText = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missingText.length !== 0) fail(`missing semantic contract text:\n${missingText.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("semantic source contract");

  runGate("type-system reference", "level1b:type-system", 60);
  runGate("semantic gates reference", "semantic:gates", 60);
  runGate("capability reference", "level1b:capability", 30);
}

main();
