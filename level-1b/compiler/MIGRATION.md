# level-1b compiler migration map

This file tracks the removal of `src/backend/cir` as a semantic dependency.
`src/backend/cir` may remain an oracle while a pass is rewritten, but every row
below must have a level-1b owner before Second Bootstrap validation.

| old `src/backend/cir` file | level-1b owner | status |
| --- | --- | --- |
| `compile_if.chiba` | `compiler/source/compile_if.chiba` | contract only |
| `namespace_project_check.chiba` | `compiler/source/project.chiba` | contract only |
| `source_semantic_check.chiba` | `compiler/source/semantic_gate.chiba`, `compiler/driver/pass_driver.chiba` | contract only |
| `lower_ast.chiba` | `compiler/lower/ast_to_core.chiba` | contract only |
| `ir.chiba` | `compiler/ir/*.chiba` | contract only |
| `show.chiba` | `compiler/ir/show.chiba` | contract only |
| `alpha.chiba` | `compiler/semantic/alpha.chiba` | contract only |
| `typed.chiba` | `compiler/semantic/typed_elaboration.chiba`, `compiler/semantic/types.chiba` | contract only |
| `typed_semantic_check.chiba` | `compiler/semantic/driver.chiba`, `compiler/semantic/typed_elaboration.chiba` | contract only |
| `type_kind.chiba` | `compiler/semantic/type_kind.chiba` | contract only |
| `type_l2_check.chiba` | `compiler/semantic/type_infer.chiba` | contract only |
| `type_unify.chiba` | `compiler/semantic/type_unify.chiba` | contract only |
| `type_row.chiba` | `compiler/semantic/type_row.chiba` | contract only |
| `type_record.chiba` | `compiler/semantic/type_record.chiba`, `compiler/semantic/type_row.chiba` | contract only |
| `type_nominal.chiba` | `compiler/semantic/type_nominal.chiba` | contract only |
| `type_method.chiba` | `compiler/semantic/method_operator.chiba` | contract only |
| `type_template.chiba` | `compiler/semantic/template.chiba` | contract only |
| `type_generic_body.chiba` | `compiler/semantic/generic_body.chiba`, `compiler/semantic/template.chiba` | contract only |
| `type_generalize.chiba` | `compiler/semantic/type_generalize.chiba` | contract only |
| `type_facts.chiba` | `compiler/semantic/type_facts.chiba` | contract only |
| `type_capability.chiba` | `compiler/semantic/capability_rules.chiba`, `compiler/semantic/abi_capability.chiba` | contract only |
| `answer_control.chiba` | `compiler/control/answer_control.chiba` | contract only |
| `continuation_check.chiba` | `compiler/control/answer_type.chiba`, `compiler/control/answer_control.chiba` | contract only |
| `continuation_boundary_check.chiba` | `compiler/control/continuation_boundary.chiba`, `compiler/control/replay_safety.chiba` | contract only |
| `continuation_usage.chiba` | `compiler/control/continuation_usage.chiba` | contract only |
| `usage.chiba` | `compiler/control/usage_subject.chiba`, `compiler/control/continuation_usage.chiba`, `compiler/closure/usage_cps.chiba` | contract only |
| `cps.chiba` | `compiler/control/cps.chiba` | contract only |
| `closure.chiba` | `compiler/closure/*.chiba` | contract only |
| `core.chiba` | `compiler/backend/core.chiba`, `compiler/backend/layout.chiba` | contract only |
| `validate_core.chiba` | `compiler/backend/validate_core.chiba` | contract only |
| `nanopass.chiba` | `compiler/driver/nanopass_pipeline.chiba`, `compiler/driver/pass_driver.chiba`, `compiler/backend/driver.chiba` | contract only |

## Exit Criteria

- No level-1b C08-C12 gate may rely on `src/backend/cir` for the primary pass
  behavior once its owner is marked rewritten.
- Oracle use must be named as oracle and paired with a level-1b fixture/golden.
- C12 cannot start while any row is `missing rewrite` or `contract only`.
