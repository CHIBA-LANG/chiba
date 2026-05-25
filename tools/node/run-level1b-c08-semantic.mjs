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
  "uses: Array[SourceUseScanResult]",
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
  "items: Array[TypedItemSkeleton]",
  "type TypedFunctionBodyFact",
  "data TypedFunctionBodyKind",
  "TypedFunctionReturnI32FortyTwo",
  "functions: Array[TypedFunctionBodyFact]",
  "def typed_function_body_fact_from_skeleton",
  "def typed_function_body_facts_from_skeletons",
  "def typed_module_from_skeletons",
  "def type_inference_has_pattern_coverage",
  "def typed_item_skeletons_have_callable_storage_surface",
  "def typed_item_skeletons_have_method_receiver_surface",
  "type MethodSurfaceFact",
  "method_surface: Array[MethodSurfaceFact]",
  "type BranchingSurfaceFact",
  "branching_surface: Array[BranchingSurfaceFact]",
  "type ControlSurfaceFact",
  "control_surface: Array[ControlSurfaceFact]",
  "def control_surface_fact_from_skeleton",
  "def control_surface_facts_from_skeletons",
  "type GlobalInitSurfaceFact",
  "global_init_surface: Array[GlobalInitSurfaceFact]",
  "type GlobalInitObligation",
  "requires_module_load_init: bool",
  "requires_dependency_edge: bool",
  "def typed_item_has_global_init_surface",
  "def global_init_surface_fact_from_skeleton",
  "def global_init_surface_facts_from_skeletons",
  "def global_init_obligation_from_surface",
  "def global_init_obligations_from_surface",
  "type PipeSurfaceFact",
  "pipe_surface: Array[PipeSurfaceFact]",
  "type PipeLoweringObligation",
  "repeated_placeholder_lhs_is_single_eval: bool",
  "def typed_item_has_pipe_surface",
  "def pipe_surface_fact_from_skeleton",
  "def pipe_surface_facts_from_skeletons",
  "def pipe_lowering_obligation_from_surface",
  "def pipe_lowering_obligations_from_surface",
  "type PatternSurfaceFact",
  "pattern_surface: Array[PatternSurfaceFact]",
  "type PatternLoweringObligation",
  "requires_match_exhaustiveness: bool",
  "requires_if_let_dual_env: bool",
  "def typed_item_has_pattern_surface",
  "def pattern_surface_fact_from_skeleton",
  "def pattern_surface_facts_from_skeletons",
  "def pattern_lowering_obligation_from_surface",
  "def pattern_lowering_obligations_from_surface",
  "type IntrinsicSurfaceFact",
  "intrinsic_surface: Array[IntrinsicSurfaceFact]",
  "type IntrinsicLoweringObligation",
  "intrinsic_namespace: String",
  "def typed_item_has_intrinsic_surface",
  "def intrinsic_surface_fact_from_skeleton",
  "def intrinsic_surface_facts_from_skeletons",
  "def intrinsic_lowering_obligation_from_surface",
  "def intrinsic_lowering_obligations_from_surface",
  "type GenericSurfaceFact",
  "generic_surface: Array[GenericSurfaceFact]",
  "type GenericTemplateObligation",
  "requires_auto_generic_inference: bool",
  "requires_instantiation_discharge: bool",
  "def typed_item_has_generic_surface",
  "def generic_surface_fact_from_skeleton",
  "def generic_surface_facts_from_skeletons",
  "def generic_template_obligation_from_surface",
  "def generic_template_obligations_from_surface",
  "type OperatorUseSurfaceFact",
  "operator_use_surface: Array[OperatorUseSurfaceFact]",
  "def typed_item_has_operator_use_surface",
  "def operator_use_surface_fact_from_skeleton",
  "def operator_use_surface_facts_from_skeletons",
  "type ClosureSurfaceFact",
  "closure_surface: Array[ClosureSurfaceFact]",
  "def typed_item_has_closure_surface",
  "def closure_surface_fact_from_skeleton",
  "def closure_surface_facts_from_skeletons",
  "def branching_surface_fact_from_skeleton",
  "def branching_surface_facts_from_skeletons",
  "def method_surface_fact_from_skeleton",
  "def method_surface_facts_from_skeletons",
  "missing-facts: method/operator surface fact generation absent",
  "missing-lowering: method receiver Self binding absent",
  "def typed_item_skeletons_have_generic_self_method_surface",
  "missing-facts: generic surface fact generation absent",
  "missing-lowering: generic method Self receiver binding absent",
  "def typed_item_skeletons_have_pattern_param_surface",
  "missing-facts: pattern surface fact generation absent",
  "missing-lowering: function pattern parameter desugaring absent",
  "def typed_item_skeletons_have_explicit_type_params",
  "missing-facts: generic surface fact generation absent",
  "missing-lowering: explicit generic parameter binding absent",
  "def typed_item_skeletons_have_generic_instantiation_surface",
  "missing-facts: generic surface fact generation absent",
  "missing-lowering: auto-generic and explicit instantiation checking absent",
  "def typed_item_skeletons_have_scope_shadow_surface",
  "missing-lowering: local scope graph and shadowing analysis absent",
  "def typed_item_skeletons_have_namespace_ownership_surface",
  "type NamespaceOwnershipFact",
  "namespace_ownership: Array[NamespaceOwnershipFact]",
  "type NamespaceImportFact",
  "namespace_imports: Array[NamespaceImportFact]",
  "def namespace_ownership_fact_from_skeleton",
  "def namespace_ownership_facts_from_skeletons",
  "def namespace_import_fact_from_use",
  "def namespace_import_facts_from_uses",
  "missing-facts: namespace ownership fact generation absent",
  "missing-facts: namespace import fact generation absent",
  "missing-lowering: import and name resolution absent",
  "def typed_item_skeletons_have_compiler_intrinsic_surface",
  "missing-facts: compiler intrinsic surface fact generation absent",
  "missing-lowering: compiler intrinsic surface lowering absent",
  "def typed_item_skeletons_have_global_init_surface",
  "missing-facts: global initialization surface fact generation absent",
  "missing-lowering: global initialization dependency and cycle analysis absent",
  "def typed_item_skeletons_have_pipe_surface",
  "missing-facts: pipe surface fact generation absent",
  "missing-lowering: pipe placeholder and receiver-first desugaring absent",
  "def typed_item_skeletons_have_deep_pattern_surface",
  "missing-facts: pattern surface fact generation absent",
  "missing-lowering: deep pattern match and if-let lowering absent",
  "def typed_item_skeletons_have_string_slice_surface",
  "missing-lowering: string interpolation and slice lowering absent",
  "def typed_item_skeletons_have_index_operator_surface",
  "missing-facts: operator use surface fact generation absent",
  "missing-lowering: index operator dispatch lowering absent",
  "def typed_item_skeletons_have_operator_obligation_surface",
  "missing-facts: operator use surface fact generation absent",
  "missing-lowering: operator obligation resolution absent",
  "missing-facts: callable storage fact generation absent",
  "has_send_callable_qualifier",
  "has_cont1_type",
  "has_contn_type",
  "def callable_storage_position_from_surface",
  "def callable_storage_variants_from_surface",
  "def callable_storage_fact_from_skeleton",
  "def callable_storage_facts_from_skeletons",
  "CallableSendableStoragePosition",
  "CallableExplicitContStorage",
  "CallableVariantBoxedCont1",
  "CallableVariantContN",
  "missing-lowering: callable storage type expression and body integration absent",
  "missing-lowering: source item type expression and body inference absent",
  "def check_l2_types",
  "data AggregateKind",
  "def build_aggregate_shape",
  "type TypedElaboration",
  "type TypedElaborationObligation",
  "requires_type_expression_lowering: bool",
  "requires_body_expression_lowering: bool",
  "def typed_elaboration_obligation_from_item",
  "def typed_elaboration_obligations",
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
  "missing-lowering: checked-template body analysis absent",
  "data GenericBodyCheck",
  "def check_generic_add_body",
  "def instantiate_field_obligation",
  "type MethodKey",
  "type OperatorKey",
  "type MethodOperatorSurfaceEntry",
  "surface_entries: Array[MethodOperatorSurfaceEntry]",
  "def method_operator_surface_entry_from_fact",
  "def method_operator_surface_entries_from_facts",
  "def method_operator_index_from_surface",
  "type OperatorResolutionObligation",
  "def operator_resolution_obligation_from_surface",
  "def operator_resolution_obligations_from_facts",
  "def build_operator_resolution_obligations",
  "def method_operator_index_empty",
  "def build_method_operator_index",
  "missing-lowering: method/operator index construction absent",
  "data ExternAbi",
  "data CapabilityUse",
  "def check_extern_abi",
  "def check_capabilities",
  "type AdtVariantTag",
  "type AdtTupleShape",
  "data ConstructorLoweringStrategy",
  "ConstructorMutateOnceUsedInput",
  "use_count: UseCount",
  "def constructor_strategy_for_use_count",
  "def constructor_mutation_obligation_from_use",
  "def constructor_mutation_obligation",
  "data AdtTupleConversionKind",
  "type AdtTupleIntrinsicIdentity",
  "intrinsic_namespace: str",
  "def adt_tuple_intrinsic_identity",
  "compiler.intrinsic",
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
  if (file.endsWith("method_operator.chiba") && /\bdef\s+build_method_operator_index\b[\s\S]*Ok\s*\(\s*MethodOperatorIndex\s*\(\s*Vec\s*\[\s*MethodKey\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)\s*,\s*Vec\s*\[\s*OperatorKey\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: method/operator index must not return an empty success`);
  }
  if (file.endsWith("template.chiba") && /\bdef\s+check_template\b[\s\S]*Ok\s*\(\s*Vec\s*\[\s*CheckedTemplate\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: checked-template pass must not return an empty success`);
  }
  if (file.endsWith("driver.chiba") && !/\bbuild_typed_facts\b[\s\S]{0,520}\bcheck_template\b[\s\S]{0,520}\bbuild_method_operator_index\b[\s\S]{0,520}\bcheck_extern_abi\b/.test(code)) {
    errors.push(`${file}: semantic driver must thread typed facts, template, and method/operator gates before ABI checks`);
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
