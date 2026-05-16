# level-1b compiler migration map

This file tracks the removal of `src/backend/cir` as a semantic dependency.
`src/backend/cir` may remain an oracle while a pass is rewritten, but every row
below must have a level-1b owner before Second Bootstrap validation.

| old `src/backend/cir` file | level-1b owner | status |
| --- | --- | --- |
| `compile_if.chiba` | `compiler/source/compile_if.chiba` | contract only |
| `namespace_project_check.chiba` | `compiler/source/project.chiba` | contract only |
| `source_semantic_check.chiba` | `compiler/source/project.chiba`, `compiler/driver/pass_driver.chiba` | missing rewrite |
| `lower_ast.chiba` | `compiler/lower/ast_to_core.chiba` | missing |
| `ir.chiba` | `compiler/ir/*.chiba` | missing |
| `show.chiba` | `compiler/ir/show.chiba` | missing |
| `alpha.chiba` | `compiler/semantic/alpha.chiba` | contract only |
| `typed.chiba` | `compiler/semantic/types.chiba` | missing rewrite |
| `typed_semantic_check.chiba` | `compiler/semantic/driver.chiba` | missing rewrite |
| `type_kind.chiba` | `compiler/semantic/types.chiba` | contract only |
| `type_l2_check.chiba` | `compiler/semantic/types.chiba` | missing rewrite |
| `type_unify.chiba` | `compiler/semantic/types.chiba` | missing rewrite |
| `type_row.chiba` | `compiler/semantic/types.chiba` | missing rewrite |
| `type_record.chiba` | `compiler/semantic/types.chiba` | missing rewrite |
| `type_nominal.chiba` | `compiler/semantic/types.chiba` | missing rewrite |
| `type_method.chiba` | `compiler/semantic/method_operator.chiba` | contract only |
| `type_template.chiba` | `compiler/semantic/template.chiba` | contract only |
| `type_generic_body.chiba` | `compiler/semantic/template.chiba` | missing rewrite |
| `type_generalize.chiba` | `compiler/semantic/types.chiba` | missing rewrite |
| `type_facts.chiba` | `compiler/semantic/types.chiba`, `compiler/semantic/abi_capability.chiba` | missing rewrite |
| `type_capability.chiba` | `compiler/semantic/abi_capability.chiba` | contract only |
| `answer_control.chiba` | `compiler/control/answer_control.chiba` | contract only |
| `continuation_check.chiba` | `compiler/control/answer_control.chiba` | missing rewrite |
| `continuation_boundary_check.chiba` | `compiler/control/replay_safety.chiba` | missing rewrite |
| `continuation_usage.chiba` | `compiler/control/continuation_usage.chiba` | contract only |
| `usage.chiba` | `compiler/control/continuation_usage.chiba`, `compiler/closure/usage_cps.chiba` | missing rewrite |
| `cps.chiba` | `compiler/control/cps.chiba` | contract only |
| `closure.chiba` | `compiler/closure/*.chiba` | contract only |
| `core.chiba` | `compiler/backend/core.chiba`, `compiler/backend/layout.chiba` | contract only |
| `validate_core.chiba` | `compiler/backend/validate_core.chiba` | contract only |
| `nanopass.chiba` | `compiler/driver/pass_driver.chiba`, `compiler/backend/driver.chiba` | missing rewrite |

## Exit Criteria

- No level-1b C08-C12 gate may rely on `src/backend/cir` for the primary pass
  behavior once its owner is marked rewritten.
- Oracle use must be named as oracle and paired with a level-1b fixture/golden.
- C12 cannot start while any row is `missing rewrite` or `contract only`.
