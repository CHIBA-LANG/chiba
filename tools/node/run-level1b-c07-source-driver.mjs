import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import process from "node:process";
import { compileWat } from "./wat-compile.mjs";

const SOURCE_ROOT = "level-1b/compiler/source";
const DRIVER_ROOT = "level-1b/compiler/driver";
const FIXTURE = "level-1b/supports/pre-c07-smokes/doc_compile_if.chiba";
const UTF8_FIXTURE = "level-1b/supports/pre-c07-smokes/utf8_identifier_blocked.chiba";
const ARTIFACT_DIR = ".scratch/level-1b/c07-source-driver";
const LEGACY_REFERENCE_COMPILER = "./target/debug/level1c.o";
const REQUIRED_TEXT = [
  "type ProjectSurface",
  "type SourceProjectFacts",
  "type SourceNameSlice",
  "type SourceHeaderFacts",
  "type SourceDocFacts",
  "type SourceCompileIfFacts",
  "data SourceCompileIfShape",
  "SourceCompileIfUnknown",
  "def source_scan_compile_if_shape",
  "def source_scan_compile_if_shape_at",
  "has_target_predicate",
  "has_backend_predicate",
  "has_unknown_predicate",
  "type SourceItemScanResult",
  "type SourceItemAttributeFacts",
  "type SourceItemSurfaceFacts",
  "type SourceUtf8ScanWarning",
  "data SourceFrontendMode",
  "SourceFrontendScannerFallback",
  "SourceFrontendChibalexChibaccPrimary",
  "type SourceParsedModuleFacts",
  "type SourceParserFacts",
  "parser: SourceParserFacts",
  "ast_primary_path: bool",
  "token_primary_path_ready: bool",
  "ast_primary_path_blocked: bool",
  "data SourceItemKind",
  "SourceItemUnion",
  "SourceItemInterface",
  "data SourceOperatorUseKind",
  "SourceOperatorUseAdd",
  "SourceOperatorUseIndexSlice",
  "type NamespaceSurface",
  "type SourceNamespaceScanWarning",
  "type SourceUseScanResult",
  "type SourceOwnerSymbolFact",
  "def load_project",
  "type SourceScanCursor",
  "type NamespaceScanResult",
  "def source_starts_with_at",
  "def source_scan_header",
  "def source_scan_doc_facts",
  "def source_scan_compile_if_facts",
  "def source_scan_compile_if_facts_at",
  "def source_item_attributes_from_pending",
  "def source_scan_compile_if_expr",
  "def source_scan_compile_if_expr_end",
  "def source_line_has_text",
  "source_line_has_text(text, start, \"target=\")",
  "source_line_has_text(text, start, \"backend=\")",
  "def source_scan_item_surface",
  "has_callable_arrow",
  "has_explicit_continuation_type",
  "has_cont1_type",
  "has_contn_type",
  "has_send_callable_qualifier",
  "has_method_receiver",
  "has_operator_method",
  "method_receiver: Option[SourceNameSlice]",
  "method_member: Option[SourceNameSlice]",
  "has_self_type",
  "has_pattern_params",
  "has_explicit_type_params",
  "has_compiler_intrinsic_call",
  "has_tuple_type_surface",
  "has_global_value_initializer",
  "has_global_value_dependency",
  "has_pipe_expr",
  "has_pipe_placeholder",
  "has_pipe_chain",
  "has_if_expr",
  "has_else_expr",
  "has_else_if_expr",
  "has_match_expr",
  "has_if_let_expr",
  "has_deep_pattern_surface",
  "has_short_circuit_expr",
  "has_reset_expr",
  "has_shift_expr",
  "has_shiftn_expr",
  "has_lambda_expr",
  "has_trailing_closure_expr",
  "has_generic_self_method",
  "has_string_interpolation",
  "has_string_slice_surface",
  "has_char_at_surface",
  "has_index_expr",
  "has_index_slice_expr",
  "has_infix_operator_expr",
  "operator_use_kind: SourceOperatorUseKind",
  "has_operator_obligation_surface",
  "has_auto_generic_params",
  "has_explicit_instantiation_call",
  "has_let_binding",
  "has_wildcard_let",
  "has_scope_shadow_candidate",
  "def source_line_has_text_before_byte",
  "def source_line_has_word",
  "def source_scan_method_receiver",
  "def source_scan_method_member",
  "def source_line_has_uppercase_call_before_byte",
  "def source_line_has_byte_before_byte",
  "def source_scan_item_has_compiler_intrinsic_call",
  "def source_scan_item_has_cont1_type",
  "def source_scan_item_has_contn_type",
  "def source_scan_item_has_global_value_dependency",
  "def source_scan_item_has_pipe_expr",
  "def source_scan_item_has_if_expr",
  "def source_scan_item_has_else_expr",
  "def source_scan_item_has_else_if_expr",
  "def source_scan_item_has_deep_pattern_surface",
  "def source_scan_item_has_short_circuit_expr",
  "def source_scan_item_has_reset_expr",
  "def source_scan_item_has_shift_expr",
  "def source_scan_item_has_shiftn_expr",
  "def source_scan_item_has_lambda_expr",
  "def source_scan_item_has_trailing_closure_expr",
  "def source_scan_item_has_generic_self_method",
  "def source_scan_item_has_string_interpolation",
  "def source_scan_item_has_index_expr",
  "def source_scan_item_operator_use_kind",
  "def source_scan_item_has_infix_operator_expr",
  "def source_scan_item_has_auto_generic_params",
  "def source_scan_item_has_explicit_instantiation_call",
  "def source_scan_item_has_let_binding",
  "def source_scan_item_has_body",
  "def source_scan_item_at",
  "def source_scan_private_item_at",
  "def source_scan_item_name",
  "def source_scan_items",
  "def source_scan_items_line",
  "def source_parsed_module_facts_from_scanned_file",
  "def source_parsed_modules_all_ast_primary",
  "def scan_project_parsed_modules_from_files",
  "def scan_project_parser_facts_from_primary_tokens",
  "token_primary_path_ready: true",
  "ast_primary_path_blocked: source_parsed_modules_all_ast_primary(modules, 0) == false",
  "missing-lowering: chibacc AST primary path still scanner-derived",
  "def source_scan_file_owner_namespace",
  "owner_namespace: Option[NamespaceNameSlice]",
  "def source_scan_utf8_warnings",
  "def scan_project_utf8_warnings",
  "def source_scan_first_namespace",
  "def source_scan_namespace_warnings",
  "def source_scan_use_at",
  "def source_scan_uses",
  "def scan_project_uses",
  "def scan_project_namespace_warnings",
  "def scan_project_source_facts",
  "def scan_project_headers",
  "def scan_project_docs",
  "def scan_project_compile_if",
  "def scan_project_items",
  "def scan_project_owner_symbols",
  "def source_owner_symbols_from_items",
  "type DocCommentBlock",
  "def attach_namespace_doc",
  "data SourceGateErrorKind",
  "SourceGateMissingNamespace",
  "SourceGateParserPrimaryPathAbsent",
  "SourceGateUtf8ScannerUnsupported",
  "SourceGateUnknownCompileIfPredicate",
  "SourceGateCompileIfFilteringAbsent",
  "SourceGateInlineNamespaceUnsupported",
  "SourceGateImportResolutionAbsent",
  "SourceGateExternAbiSignatureAbsent",
  "SourceGateTopLevelRefNeedsWorldLocal",
  "SourceGateDefBodyAbsent",
  "SourceGateSelfOutsideMethod",
  "SourceGateDuplicateItem",
  "SourceGateItemScanAbsent",
  "SourceGateTypedItemLoweringAbsent",
  "def source_gate_errors",
  "def check_source_semantic_gates",
  "SourceGateRefArrayDirectAssignment",
  "source_project_parser_primary_blocked(project)",
  "project.facts.parser.diagnostic",
  "missing-facts: source item scan absent",
  "missing-facts: UTF-8 aware source scanner absent",
  "missing-facts: unknown compile_if predicate shape",
  "missing-lowering: item compile_if filtering absent",
  "missing-lowering: inline namespace block assembly absent",
  "missing-facts: extern ABI signature type annotations absent",
  "missing-attribute: top-level Ref requires world_local",
  "missing-facts: source def body or initializer absent",
  "invalid-surface: Self only valid in method receiver scope",
  "invalid-surface: duplicate item in namespace",
  "missing-lowering: source import/name resolution absent",
  "missing-facts: namespace scan found no namespace",
  "type SourceImportScopeInput",
  "type SourceImportResolution",
  "type SourceNameResolutionObligation",
  "obligations: Array[SourceNameResolutionObligation]",
  "requires_visibility_check: bool",
  "requires_conflict_check: bool",
  "def source_import_scope_for_header",
  "def source_name_resolution_obligation_from_scope",
  "def source_name_resolution_obligations",
  "def resolve_source_imports",
  "def source_project_needs_import_resolution",
  "def source_items_have_extern_without_type_annotation",
  "def source_items_have_compile_if",
  "def source_items_have_static_ref_without_world_local",
  "def source_items_have_def_without_body",
  "def source_items_have_self_outside_method",
  "def source_project_has_duplicate_item",
  "def source_name_slices_equal",
  "data CompileIfPredicate",
  "CompileIfAll",
  "CompileIfNot",
  "def SourceHeaderFacts.prelude_policy",
  "\"wasm32-unknown-wasi\"",
  "\"wasm-gc\"",
  "type DriverDiagnostic",
  "def source_compile_if_fact_enabled",
  "def source_compile_if_all_enabled",
  "def source_compile_if_any_enabled",
  "def source_compile_if_not_enabled",
  "def source_item_enabled_for_target",
  "def source_item_after_compile_if_filter",
  "def filter_source_items_for_target",
  "def source_facts_with_filtered_items",
  "def project_with_filtered_items",
  "def run_source_driver",
  "data PipelineStage",
  "def run_nanopass_wat",
  "stable_sort",
];

function fail(message) {
  console.error("[FAIL] level-1b C07 source driver");
  console.error(message);
  process.exit(1);
}

function pass(name) {
  console.log(`[PASS] ${name}`);
}

function referenceGate(name) {
  console.log(`[REF] ${name}`);
}

function referenceGateFailed(name) {
  console.log(`[REF-FAIL] ${name}`);
}

function assertIncludes(name, text, needles) {
  for (const needle of needles) {
    if (!text.includes(needle)) fail(`${name}: missing ${needle}`);
  }
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function invokeWatExport(wat, exportName) {
  const module = new WebAssembly.Module(compileWat(wat));
  const instance = new WebAssembly.Instance(module, {});
  const fn = instance.exports[exportName];
  if (typeof fn !== "function") fail(`WAT artifact does not export ${exportName}`);
  return fn();
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
  while (cursor >= 0 && lines[cursor].trimStart().startsWith("#[")) {
    cursor -= 1;
    while (cursor >= 0 && lines[cursor].trim() === "") cursor -= 1;
  }
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
  if (/\bmetalstd\b|Ptr\s*\[|UnsafeRef\s*\[|heap_alloc\s*\(|load(?:8|16|32|64)\s*\(/.test(code)) {
    errors.push(`${file}: source driver leaks Metal/raw memory`);
  }
  if (file.endsWith("compile_if.chiba") && /__compiler_builtin\s*\(\s*"std\.compile_if_eval"/.test(code)) {
    errors.push(`${file}: compile_if eval must be implemented in level-1b source`);
  }
  if (file.endsWith("semantic_gate.chiba") && /\bdef\s+check_source_semantic_gates\b[^\n]*=\s*SourceGateResult\s*\(\s*project\s*,\s*Vec\s*\[\s*SourceGateError\s*\]\s*\.\s*new\s*\(\s*\)\s*\.\s*freeze\s*\(\s*\)\s*\)/.test(code)) {
    errors.push(`${file}: source semantic gates must not pass through with empty errors`);
  }
  for (let i = 0; i < lines.length; i += 1) {
    if (isPublicItem(lines[i]) && previousDocBlock(lines, i).length === 0) {
      errors.push(`${file}:${i + 1}: public item is missing /// doc comment`);
    }
  }
  return errors;
}

function main() {
  const files = [...listChiba(SOURCE_ROOT), ...listChiba(DRIVER_ROOT)];
  const joined = files.map(read).join("\n");
  const missing = REQUIRED_TEXT.filter((needle) => !joined.includes(needle));
  if (missing.length !== 0) fail(`missing source driver contract text:\n${missing.join("\n")}`);

  const errors = files.flatMap((file) => checkSource(file, read(file)));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("source driver contract");

  const fixture = read(FIXTURE);
  for (const needle of [
    "/// Documented C07 namespace.",
    "#[doc(path=\"docs/c07.md\")]",
    "namespace pre_c07.doc_compile_if",
    "backend=\"wasm-gc\"",
    "target=\"wasm32-unknown-wasi\"",
    "not(backend=\"wasm-gc\")",
  ]) {
    if (!fixture.includes(needle)) fail(`C07 fixture missing ${needle}`);
  }
  for (const needle of [
    "all(",
    "wasm_only",
    "native_only",
  ]) {
    if (!fixture.includes(needle)) fail(`C07 fixture missing ${needle}`);
  }
  pass("doc compile_if fixture source");

  const utf8Fixture = read(UTF8_FIXTURE);
  for (const needle of [
    "namespace pre_c07.utf8_identifier_blocked",
    "Résultat",
    "Succès",
    "café",
  ]) {
    if (!utf8Fixture.includes(needle)) fail(`C07 UTF-8 fixture missing ${needle}`);
  }
  pass("UTF-8 identifier fixture source");

  const parserReferenceOnlyLegacyCompiler = LEGACY_REFERENCE_COMPILER;
  const parsed = spawnSync("timeout", ["10", parserReferenceOnlyLegacyCompiler, "parse", FIXTURE], { encoding: "utf8" });
  if (parsed.status !== 0 || !parsed.stdout.startsWith("OK(")) {
    fail(`level1c parser primary fixture failed:\n${parsed.stdout}${parsed.stderr}`);
  }
  assertIncludes("level1c parser primary fixture", parsed.stdout, [
    "SourceFile(",
    "Namespace(",
    "Path_Cons(",
    "\"pre_c07\"",
    "\"doc_compile_if\"",
    "Item_WithAttrs(",
    "Attr(",
    "\"compile_if\"",
    "\"wasm_only\"",
    "\"native_only\"",
    "DefFun(",
    "Expr_Int(",
  ]);
  pass("level1c parser primary fixture AST");

  ensureDir(ARTIFACT_DIR);
  const watReferenceOnlyLegacyCompiler = LEGACY_REFERENCE_COMPILER;
  const wat = spawnSync("timeout", ["30", watReferenceOnlyLegacyCompiler, "wat", FIXTURE], { encoding: "utf8" });
  if (wat.status !== 0 || !wat.stdout.includes("(module")) {
    fail(`level1c WAT primary fixture failed:\n${wat.stdout}${wat.stderr}`);
  }
  const watPath = path.join(ARTIFACT_DIR, "doc_compile_if.wat");
  fs.writeFileSync(watPath, wat.stdout);
  assertIncludes("level1c WAT primary fixture", wat.stdout, [
    "(func $native_only",
    "(export \"native_only\"",
  ]);
  const result = invokeWatExport(wat.stdout, "native_only");
  if (String(result) !== "0") {
    fail(`level1c WAT primary fixture did not run native_only -> 0, got ${String(result)}`);
  }
  pass(`level1c WAT primary fixture ${watPath}`);

  const namespace = spawnSync("timeout", ["30", "vp", "run", "level1b:namespace"], { encoding: "utf8" });
  if (namespace.status === 0) referenceGate("namespace project reference");
  else referenceGateFailed("namespace project reference");
}

main();
