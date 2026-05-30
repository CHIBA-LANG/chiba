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
  "type AlphaAstOwnerOrigin",
  "ast_owner_origins: Array[AlphaAstOwnerOrigin]",
  "def alpha_ast_owner_origins_from_source_facts",
  "alpha_binders_from_source_items_and_ast_owners(project.facts.items, project.facts.ast_owner_symbols)",
  "alpha_ast_owner_origins_from_source_facts(project.facts.ast_owner_symbols, project.facts.items.len()",
  "has_i32_const_body: item.has_i32_const_body",
  "i32_const_body: item.i32_const_body",
  "uses: Array[SourceUseScanResult]",
  "owner_namespace: Option[NamespaceNameSlice]",
  "attributes: SourceItemAttributeFacts",
  "surface: SourceItemSurfaceFacts",
  "def alpha_origins_from_source_items",
  "def elaborate_patterns",
  "def pattern_ids_from_binders",
  "data PatternSourceKind",
  "PatternSourceIfLet",
  "PatternSourceMatchArm",
  "data PatternTestKind",
  "PatternTestTag",
  "PatternTestFieldShape",
  "type PatternDftStep",
  "type PatternExhaustivenessFact",
  "missing_case_is_error: bool",
  "type PatternEnvFact",
  "failure_branch_introduces_bindings: bool",
  "dft_steps: Array[PatternDftStep]",
  "exhaustiveness: Array[PatternExhaustivenessFact]",
  "envs: Array[PatternEnvFact]",
  "def origin_has_pattern_surface",
  "def pattern_source_kind_from_origin",
  "def pattern_test_kind_from_origin",
  "def pattern_surface_binds_value",
  "def pattern_dft_step_from_origin",
  "def pattern_dft_steps_from_origins",
  "def pattern_exhaustiveness_from_origin",
  "def pattern_exhaustiveness_from_origins",
  "def pattern_env_from_origin",
  "def pattern_envs_from_origins",
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
  "type TypedFunctionFact",
  "symbol: str",
  "local_symbol: str",
  "i32_param_count: usize",
  "data TypedExprKind",
  "data TypedI32ConstAtom",
  "data TypedBranchCondition",
  "TypedExprI32Const",
  "TypedExprParam(usize)",
  "TypedExprTailCall",
  "TypedExprTailCallArgs",
  "TypedExprIfElse",
  "TypedExprBranchJoinPending",
  "data TypedExprVisitKind",
  "type TypedExprVisit",
  "def typed_expr_traverse",
  "def typed_function_expr_visits_all",
  "def typed_no_arg_call_exact",
  "def typed_i32_const_atom_from_region",
  "def typed_i32_const_atom_exact_len",
  "def typed_i32_const_atom_exact_from_region",
  "def typed_branch_literal_condition_matches",
  "def typed_branch_condition_from_region",
  "def typed_branch_condition_from_item_region",
  "def typed_function_if_else_expr_body",
  "def typed_function_tail_call_target_slice",
  "def typed_function_tail_call_target_text",
  "def typed_function_first_param_name",
  "def typed_function_i32_param_count",
  "def typed_function_param_index_from_region",
  "def typed_function_param_expr_from_region",
  "def typed_call_arg_end_nested",
  "def typed_call_arg_expr_from_region",
  "def typed_index_expr_close",
  "def typed_index_expr_body",
  "def typed_index_slice_expr_body",
  "def typed_function_expr_from_region",
  "def typed_symbol_is_qualified",
  "def typed_namespace_symbol_text",
  "def typed_qualified_symbol_text",
  "def typed_function_symbol_from_name",
  "def typed_resolve_tail_call_target",
  "def typed_function_body_starts_with_atom",
  "def typed_source_region_only_spaces",
  "def typed_i32_const_call_arg_exact_len",
  "def typed_i32_const_call_arg_exact",
  "TypedI32ConstLiteral(usize)",
  "def typed_function_symbol_eq",
  "def typed_function_tail_call_target",
  "def typed_functions_have_symbol",
  "def typed_functions_have_later_symbol",
  "def typed_function_symbols_unique",
  "def typed_function_main_abi_valid",
  "invalid-surface: duplicate function symbol",
  "invalid-surface: exported main cannot take i32 params in current ABI slice",
  "def typed_function_tail_targets_resolved_for_body",
  "def typed_function_tail_targets_resolved",
  "missing-lowering: namespace-aware typed callee resolution absent",
  "functions: Array[TypedFunctionFact]",
  "adt_ctors: Array[LoweredAdtCtor]",
  "def typed_adt_next_ctor_offset",
  "def typed_adt_payload_arity",
  "def typed_adt_ctors_from_data_body",
  "def typed_adt_ctors_from_skeleton",
  "def typed_adt_ctors_from_skeletons",
  "def typed_function_fact_from_skeleton",
  "def typed_function_facts_from_skeletons",
  "def typed_item_skeletons_from_ast_owners",
  "def alpha_module_has_ast_primary_items",
  "def typed_item_skeletons_from_alpha_primary",
  "if alpha_module_has_ast_primary_items(module)",
  "def typed_function_fact_from_ast_owner",
  "def typed_function_facts_with_ast_owners",
  "def typed_expr_from_ast_node",
  "def typed_primitive_binary_op_from_ast_value",
  "Some(TypedExprPrimitiveBinary(op, left, right))",
  "if origin.from_chibacc { None }",
  "ast_expr_nodes: Array[SourceAstExprNodeFact]",
  "let skeleton_functions = typed_function_facts_from_skeletons",
  "let functions = typed_function_facts_with_ast_owners(skeleton_functions, ast_owner_symbols, module.project.facts.ast_expr_nodes)",
  "def typed_module_from_skeletons",
  "def type_inference_has_pattern_coverage",
  "def typed_item_skeletons_have_callable_storage_surface",
  "def typed_item_skeletons_have_method_receiver_surface",
  "type MethodSurfaceFact",
  "method_surface: Array[MethodSurfaceFact]",
  "type MethodUseSurfaceFact",
  "method_use_surface: Array[MethodUseSurfaceFact]",
  "type BranchingSurfaceFact",
  "branching_surface: Array[BranchingSurfaceFact]",
  "type ControlSurfaceFact",
  "control_surface: Array[ControlSurfaceFact]",
  "def control_surface_fact_from_skeleton",
  "def control_surface_facts_from_skeletons",
  "type GlobalInitSurfaceFact",
  "global_init_surface: Array[GlobalInitSurfaceFact]",
  "type GlobalInitObligation",
  "type GlobalInitPlanStep",
  "requires_module_load_init: bool",
  "requires_dependency_edge: bool",
  "global_init_obligations: Array[GlobalInitObligation]",
  "global_init_plan: Array[GlobalInitPlanStep]",
  "reads_prior_global: bool",
  "emits_module_start_store: bool",
  "cycle_free_prefix: bool",
  "def typed_item_has_global_init_surface",
  "def global_init_surface_fact_from_skeleton",
  "def global_init_surface_facts_from_skeletons",
  "def global_init_obligation_from_surface",
  "def global_init_obligations_from_surface",
  "def global_init_plan_step_from_obligation",
  "def global_init_plan_from_obligations",
  "def global_init_plan_cycle_free",
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
  "pattern_dft_steps: Array[PatternDftStep]",
  "pattern_exhaustiveness: Array[PatternExhaustivenessFact]",
  "pattern_envs: Array[PatternEnvFact]",
  "def typed_item_has_pattern_surface",
  "def pattern_surface_fact_from_skeleton",
  "def pattern_surface_facts_from_skeletons",
  "def pattern_lowering_obligation_from_surface",
  "def pattern_lowering_obligations_from_surface",
  "type IntrinsicSurfaceFact",
  "intrinsic_surface: Array[IntrinsicSurfaceFact]",
  "type IntrinsicLoweringObligation",
  "intrinsic_lowering_obligations: Array[IntrinsicLoweringObligation]",
  "intrinsic_namespace: String",
  "has_tuple_to_adt: bool",
  "has_adt_to_tuple: bool",
  "has_unsafe_cast: bool",
  "tuple_to_adt_identity: Option[AdtTupleIntrinsicIdentity]",
  "adt_to_tuple_identity: Option[AdtTupleIntrinsicIdentity]",
  "type IntrinsicBridgeRoundtripFact",
  "intrinsic_bridge_roundtrips: Array[IntrinsicBridgeRoundtripFact]",
  "def intrinsic_bridge_roundtrip_fact_from_obligation",
  "def intrinsic_bridge_roundtrip_facts",
  "preserves_identity: bool",
  "requires_method_first_bridge: bool",
  "unsafe_cast_is_compiler_only: bool",
  "def typed_item_has_intrinsic_surface",
  "def typed_source_has_utf8_scanner_fallback",
  "missing-facts: UTF-8 typed identifier facts require chibacc AST primary path",
  "def typed_item_region_has_intrinsic_text",
  "def intrinsic_surface_fact_from_skeleton",
  "def intrinsic_surface_facts_from_skeletons",
  "def intrinsic_tuple_to_adt_identity",
  "def intrinsic_adt_to_tuple_identity",
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
  "right_operand: Option[SourceNameSlice]",
  "index_operand: Option[SourceNameSlice]",
  "slice_start_operand: Option[SourceNameSlice]",
  "slice_len_operand: Option[SourceNameSlice]",
  "operator_kind: SourceOperatorUseKind",
  "def typed_item_has_operator_use_surface",
  "def operator_use_surface_fact_from_skeleton",
  "def operator_use_surface_facts_from_skeletons",
  "type ClosureSurfaceFact",
  "closure_surface: Array[ClosureSurfaceFact]",
  "syntactic_no_capture: bool",
  "def closure_surface_all_syntactic_no_capture",
  "def typed_item_has_closure_surface",
  "def closure_surface_fact_from_skeleton",
  "def closure_surface_facts_from_skeletons",
  "type ScopeShadowFact",
  "scope_shadow: Array[ScopeShadowFact]",
  "allows_local_shadowing: bool",
  "rejects_same_scope_duplicate: bool",
  "def scope_shadow_fact_from_skeleton",
  "def scope_shadow_facts_from_skeletons",
  "def scope_shadow_facts_policy_valid",
  "def branching_surface_fact_from_skeleton",
  "def branching_surface_facts_from_skeletons",
  "def method_surface_fact_from_skeleton",
  "def method_surface_facts_from_skeletons",
  "def method_use_surface_fact_from_skeleton",
  "def method_use_surface_facts_from_skeletons",
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
  "missing-lowering: executable local scope graph absent",
  "def typed_item_skeletons_have_namespace_ownership_surface",
  "type NamespaceOwnershipFact",
  "namespace_ownership: Array[NamespaceOwnershipFact]",
  "type TypedAstOwnerSymbolFact",
  "ast_owner_symbols: Array[TypedAstOwnerSymbolFact]",
  "def ast_owner_symbol_fact_from_alpha",
  "def ast_owner_symbol_facts_from_alpha",
  "ast_owner_symbol_facts_from_alpha(module.ast_owner_origins",
  "has_i32_const_body: origin.has_i32_const_body",
  "i32_const_body: origin.i32_const_body",
  "has_i32_add_mul_body: origin.has_i32_add_mul_body",
  "has_i32_if_else_body: origin.has_i32_if_else_body",
  "has_i32_prefix_neg_body: origin.has_i32_prefix_neg_body",
  "has_i32_match_param_body: origin.has_i32_match_param_body",
  "has_i32_add_mul_body: bool",
  "has_i32_if_else_body: bool",
  "has_i32_prefix_neg_body: bool",
  "has_i32_match_param_body: bool",
  "def typed_ast_owner_add_mul_body",
  "def typed_ast_owner_if_else_body",
  "def typed_ast_owner_prefix_neg_body",
  "def typed_ast_owner_match_param_body",
  "def typed_expr_from_ast_owner",
  "PrimitiveBinaryAdd",
  "PrimitiveBinarySub",
  "PrimitiveBinaryMul",
  "type NamespaceImportFact",
  "namespace_imports: Array[NamespaceImportFact]",
  "type NamespaceResolutionFact",
  "namespace_resolution: Array[NamespaceResolutionFact]",
  "current_namespace_precedes_imports: bool",
  "requires_private_visibility_check: bool",
  "rejects_ambiguous_imports: bool",
  "def namespace_ownership_fact_from_skeleton",
  "def namespace_ownership_facts_from_skeletons",
  "def namespace_import_fact_from_use",
  "def namespace_import_facts_from_uses",
  "def namespace_resolution_fact_from_skeleton",
  "def namespace_resolution_facts_from_skeletons",
  "def namespace_resolution_facts_policy_valid",
  "missing-facts: namespace ownership fact generation absent",
  "missing-facts: namespace import fact generation absent",
  "missing-lowering: executable import and name lookup absent",
  "def typed_item_skeletons_have_compiler_intrinsic_surface",
  "missing-facts: compiler intrinsic surface fact generation absent",
  "missing-lowering: compiler intrinsic surface lowering absent",
  "def typed_item_skeletons_have_global_init_surface",
  "missing-facts: global initialization surface fact generation absent",
  "missing-lowering: global initialization executable lowering absent",
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
  "def method_operator_file_text",
  "def method_operator_slice_text",
  "def method_receiver_last_dot",
  "def method_receiver_nominal_id",
  "def method_member_name",
  "type MethodSelfBindingFact",
  "self_bindings: Array[MethodSelfBindingFact]",
  "binds_self_to_receiver: bool",
  "receiver_has_generics: bool",
  "def method_self_bindings_none",
  "def method_self_binding_from_surface_entry",
  "def method_self_bindings_from_surface_entries",
  "def method_self_bindings_valid",
  "def method_key_from_surface_entry",
  "def operator_key_from_surface_entry",
  "def method_keys_from_surface_entries",
  "def operator_keys_from_surface_entries",
  "def method_operator_index_from_surface",
  "type MethodCandidateSet",
  "method_candidates: Array[MethodCandidateSet]",
  "visibility_filtered_count: usize",
  "private_visibility_ok: bool",
  "def method_key_visible_from_use",
  "def method_candidates_visible_from_use",
  "def method_candidate_set_from_use",
  "def method_candidate_sets",
  "def method_candidate_diagnostics",
  "invalid-surface: private method candidate not visible",
  "type OperatorResolutionObligation",
  "type OperatorCandidateSet",
  "receiver: Option[SourceNameSlice]",
  "receiver_name: String",
  "operator_name: String",
  "candidates: Array[OperatorKey]",
  "candidate_count: usize",
  "expected_operand_count: usize",
  "observed_operand_count: usize",
  "operand_arity_matches: bool",
  "operand_types_match: bool",
  "requires_operand_matching: bool",
  "requires_ambiguity_check: bool",
  "missing_candidate_is_error: bool",
  "data OperatorCandidateStatus",
  "OperatorCandidateMissing",
  "OperatorCandidateAmbiguityPending",
  "OperatorCandidateOperandMatchingPending",
  "OperatorCandidateOperandArityMismatch",
  "OperatorCandidateUnknownOperatorPending",
  "type OperatorCandidateDiagnosticFact",
  "operator_candidates: Array[OperatorCandidateSet]",
  "def operator_resolution_obligation_from_surface",
  "def operator_resolution_obligations_from_facts",
  "def build_operator_resolution_obligations",
  "def operator_member_name_for_use_kind",
  "def operator_receiver_base_name",
  "def operator_receiver_base_name_loop",
  "def operator_key_matches_use_kind",
  "def operator_obligation_receiver_name",
  "def operator_key_matches_receiver",
  "def operator_candidates_matching_receiver_and_kind",
  "def operator_candidates_matching_use_kind",
  "def operator_obligation_requires_operand_matching",
  "def operator_obligation_observed_operand_count",
  "def operator_expected_operand_count",
  "def operator_operand_types_match",
  "def operator_binary_operand_types_match",
  "def operator_index_operand_types_match",
  "def operator_index_slice_operand_types_match",
  "def operator_candidate_set_from_obligation",
  "def operator_candidate_sets",
  "def operator_candidate_status",
  "def operator_candidate_diagnostic_fact",
  "def operator_candidate_diagnostic_facts",
  "def operator_candidate_diagnostic",
  "def operator_candidate_diagnostics",
  "def method_operator_index_with_candidates",
  "def method_operator_index_empty",
  "def receiver_has_builtin_operator_intrinsic",
  "receiver == \"u8\"",
  "receiver == \"f64\"",
  "receiver == \"Vec\"",
  "receiver == \"Array\"",
  "receiver == \"String\"",
  "receiver == \"str\"",
  "def builtin_index_operator_keys_for",
  "builtin_index_operator_keys_for(\"Vec\"",
  "builtin_index_operator_keys_for(\"Array\"",
  "builtin_index_operator_keys_for(\"Slice\"",
  "builtin_index_operator_keys_for(\"String\"",
  "builtin_index_operator_keys_for(\"str\"",
  "def build_method_operator_index",
  "invalid-surface: method candidate missing",
  "invalid-surface: operator overload candidate missing",
  "invalid-surface: operator operand arity mismatch",
  "invalid-surface: operator operand type mismatch",
  "data ExternAbi",
  "data CapabilityUse",
  "def check_extern_abi",
  "def check_capabilities",
  "type AdtVariantTag",
  "type AdtTupleShape",
  "data AdtTupleFieldRole",
  "AdtTupleTagField",
  "AdtTuplePayloadField",
  "type AdtTupleField",
  "type TupleNominalIdentity",
  "tuple_key: str",
  "nominal: NominalId",
  "def semantic_type_key",
  "def tuple_key_from_types",
  "def tuple_nominal_identity_from_types",
  "type AdtCtorHelperBody",
  "helper_arity: usize",
  "tuple_fields: Array[AdtTupleField]",
  "tuple_nominal: TupleNominalIdentity",
  "tuple_record_shape: AggregateShape",
  "body: AdtCtorHelperBody",
  "requires_utf8_identifier_lowering: bool",
  "data ConstructorLoweringStrategy",
  "ConstructorMutateOnceUsedInput",
  "use_count: UseCount",
  "def adt_tuple_field_name",
  "def adt_ctor_byte_requires_utf8_identifier_lowering",
  "def adt_ctor_name_requires_utf8_identifier_lowering",
  "def adt_ctor_symbol_from_name",
  "def AdtVariantDecl.canonical_tag",
  "def adt_symbol_type",
  "def adt_tuple_fields_for_variant",
  "def adt_tuple_field_types",
  "def adt_tuple_nominal_identity",
  "compiler.tuple",
  "def adt_tuple_record_shape",
  "returns_canonical_tuple: true",
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

function referenceGate(name) {
  console.log(`[REF] ${name}`);
}

function referenceGateFailed(name) {
  console.log(`[REF-FAIL] ${name}`);
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
  if (file.endsWith("method_operator.chiba") && /\brequires_operand_matching:\s*operator_obligation_requires_operand_matching\s*\(\s*obligation\s*\)/.test(code)) {
    errors.push(`${file}: operator candidate sets must not leave receiver-typed operator uses pending operand matching`);
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

function checkTupleNominalIdentitySource(joined) {
  const errors = [];
  const typeRecord = read(path.join(ROOT, "type_record.chiba"));
  if (!/def\s+tuple_type_key_sequence\b[\s\S]*semantic_type_key\s*\(\s*types\.get\s*\(\s*index\s*\)\s*\)/.test(typeRecord)) {
    errors.push("type_record.chiba: tuple nominal key must be derived from each ordered semantic element type");
  }
  if (!/def\s+tuple_key_from_types\b[\s\S]*"tuple\("\.concat\s*\(\s*tuple_type_key_sequence\s*\(\s*types,\s*0,\s*""\s*\)\s*\)\.concat\s*\(\s*"\)"\s*\)/.test(typeRecord)) {
    errors.push("type_record.chiba: tuple nominal key must wrap the ordered type-key sequence in a canonical tuple(...) identity");
  }
  if (!/def\s+tuple_nominal_identity_from_types\b[\s\S]*namespace_name:\s*"compiler\.tuple"[\s\S]*name:\s*key/.test(typeRecord)) {
    errors.push("type_record.chiba: tuple nominal identity must use compiler.tuple::tuple(<ordered semantic type keys>)");
  }
  if (/def\s+tuple_nominal_identity_from_types\b[\s\S]{0,500}(source|span|binder|helper|occurrence|local)/.test(typeRecord)) {
    errors.push("type_record.chiba: tuple nominal identity must not depend on source/helper/local occurrence data");
  }
  if (!/def\s+adt_tuple_nominal_identity\b[\s\S]*tuple_nominal_identity_from_types\s*\(\s*adt_tuple_field_types/.test(joined)) {
    errors.push("adt_tuple_lowering.chiba: ADT tuple bridge must reuse ordinary tuple nominal identity from ordered field types");
  }
  return errors;
}

function runGate(label, script, timeoutSeconds) {
  const result = spawnSync("timeout", [String(timeoutSeconds), "vp", "run", script], {
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
  });
  if (result.status === 0) referenceGate(label);
  else referenceGateFailed(label);
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
  errors.push(...checkTupleNominalIdentitySource(joined));
  if (errors.length !== 0) fail(errors.join("\n"));
  pass("semantic source contract");

  runGate("type-system reference", "level1b:type-system", 60);
  runGate("semantic gates reference", "semantic:gates", 60);
  runGate("capability reference", "level1b:capability", 30);
}

main();
