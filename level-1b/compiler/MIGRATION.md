# level-1b compiler migration map

This file tracks the removal of `src/backend/cir` as a semantic dependency.
`src/backend/cir` may remain an oracle while a pass is rewritten, but every row
below must have a level-1b owner before Second Bootstrap validation.

| old `src/backend/cir` file | level-1b owner | status |
| --- | --- | --- |
| `compile_if.chiba` | `compiler/source/compile_if.chiba` | rewritten |
| `namespace_project_check.chiba` | `compiler/source/project.chiba` | contract only |
| `source_semantic_check.chiba` | `compiler/source/semantic_gate.chiba`, `compiler/driver/pass_driver.chiba` | partial rewrite: gate result and driver diagnostic flow owned; concrete source scans pending |
| `lower_ast.chiba` | `compiler/lower/ast_to_core.chiba` | contract only |
| `ir.chiba` | `compiler/ir/*.chiba` | contract only |
| `show.chiba` | `compiler/ir/show.chiba` | rewritten |
| `alpha.chiba` | `compiler/semantic/alpha.chiba` | partial rewrite: alpha module boundary owned; source binder traversal pending |
| `typed.chiba` | `compiler/semantic/typed_elaboration.chiba`, `compiler/semantic/types.chiba` | partial rewrite: typed elaboration/fact attachment owned; full typed AST expression elaboration pending |
| `typed_semantic_check.chiba` | `compiler/semantic/driver.chiba`, `compiler/semantic/typed_elaboration.chiba` | partial rewrite: typed diagnostics sort and elaboration boundary owned; full semantic driver still contract |
| `type_kind.chiba` | `compiler/semantic/type_kind.chiba` | rewritten |
| `type_l2_check.chiba` | `compiler/semantic/type_infer.chiba` | contract only |
| `type_unify.chiba` | `compiler/semantic/type_unify.chiba` | rewritten |
| `type_row.chiba` | `compiler/semantic/type_row.chiba` | rewritten |
| `type_record.chiba` | `compiler/semantic/type_record.chiba`, `compiler/semantic/type_row.chiba` | rewritten |
| `type_nominal.chiba` | `compiler/semantic/type_nominal.chiba` | rewritten |
| `type_method.chiba` | `compiler/semantic/method_operator.chiba` | partial rewrite: nominal method/operator index data flow owned; module item scan pending |
| `type_template.chiba` | `compiler/semantic/template.chiba` | partial rewrite: typed-fact obligation collection rewritten; full template body driver still contract |
| `type_generic_body.chiba` | `compiler/semantic/generic_body.chiba`, `compiler/semantic/template.chiba` | partial rewrite: generic add/field body and field instantiation checks rewritten; full template driver still contract |
| `type_generalize.chiba` | `compiler/semantic/type_generalize.chiba` | rewritten |
| `type_facts.chiba` | `compiler/semantic/type_facts.chiba` | partial rewrite: typed fact array boundary owned; expression fact extraction pending |
| `type_capability.chiba` | `compiler/semantic/capability_rules.chiba`, `compiler/semantic/abi_capability.chiba` | partial rewrite: scalar/capability helper rules and module pass boundary owned; typed item traversal pending |
| `answer_control.chiba` | `compiler/control/answer_control.chiba` | partial rewrite: answer/control module boundary and empty fact stream owned; expression-level reset/shift scan pending |
| `continuation_check.chiba` | `compiler/control/answer_type.chiba`, `compiler/control/answer_control.chiba` | partial rewrite: answer type unification helper owned; expression-level continuation collection pending |
| `continuation_boundary_check.chiba` | `compiler/control/continuation_boundary.chiba`, `compiler/control/replay_safety.chiba` | partial rewrite: boundary/replay module flow and facts owned; source capture scan pending |
| `continuation_usage.chiba` | `compiler/control/continuation_usage.chiba` | partial rewrite: usage module boundary owned; binder reference counting pending |
| `usage.chiba` | `compiler/control/usage_subject.chiba`, `compiler/control/continuation_usage.chiba`, `compiler/closure/usage_cps.chiba` | partial rewrite: usage count algebra and CPS fact threading owned; full subject collection pending |
| `cps.chiba` | `compiler/control/cps.chiba` | partial rewrite: CPS module boundary is level-1b-owned; full expression CPS lowering still pending |
| `closure.chiba` | `compiler/closure/*.chiba` | partial rewrite: CPS usage, continuation package decisions, layout threading, lambda lift stubs, and env simplification rewritten; full capture extraction still pending |
| `core.chiba` | `compiler/backend/core.chiba`, `compiler/backend/layout.chiba` | rewritten |
| `validate_core.chiba` | `compiler/backend/validate_core.chiba` | rewritten |
| `nanopass.chiba` | `compiler/driver/nanopass_pipeline.chiba`, `compiler/driver/pass_driver.chiba`, `compiler/backend/driver.chiba` | partial rewrite: pass graph and diagnostic channel conversion owned; underlying source/semantic full drivers still partial |

## Exit Criteria

- No level-1b C08-C12 gate may rely on `src/backend/cir` for the primary pass
  behavior once its owner is marked rewritten.
- Oracle use must be named as oracle and paired with a level-1b fixture/golden.
- C12 cannot start while any row is `missing rewrite` or `contract only`.
