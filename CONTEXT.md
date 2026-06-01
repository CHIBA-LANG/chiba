# Chiba Level-1 Compiler Bootstrap

This context names the compiler bootstrap concepts used to decide whether level-1b is truly becoming the primary implementation path.

## Language

**Primary compiler behavior**:
The compiler behavior that carries the real success path without relying on the legacy `src/backend/cir` path to supply key semantics. Legacy code may remain as an oracle or diff reference, but it must not be required for level-1b to claim successful compiler behavior.
_Avoid_: primary-ish, mapped, gate-green

**Contract smoke**:
A check that proves an interface, fixture, pass entry point, IR shape, mapping, harness, or expected error path exists and can be invoked. It prevents structural regressions but does not prove user-visible compiler semantics.
_Avoid_: semantic proof, implemented behavior, done

**Semantic validation**:
A check that proves the compiler makes the correct user-visible semantic judgment or generates behavior that preserves the language contract. Shape-only, mapping-only, and comment-only outputs are not semantic validation.
_Avoid_: smoke, mapping, fixture coverage

**Fail-closed stub**:
A known-incomplete compiler component that refuses to act as primary compiler behavior. It may participate in contract smoke, but on a real compiler path it must produce a precise blocker instead of passing through data unchanged.
_Avoid_: silent pass-through, fake success, empty facts

**Blocker taxonomy**:
The fixed classes used when a fail-closed stub blocks real compiler behavior: `missing-facts`, `missing-lowering`, `missing-runtime-contract`, `missing-backend-layout`, `legacy-dependency`, and `oracle-dependency`. Each blocker must name one class and one concrete missing semantic.
_Avoid_: not implemented, TODO, unsupported

**Legacy dependency**:
A level-1b success path that requires the legacy `src/backend/cir` implementation to supply real lowering, backend output, facts, or fallback behavior. Static comparison, documentation references, and non-authoritative oracle diffs do not count as legacy dependency.
_Avoid_: fallback, borrowed success, legacy-backed gate

**Legacy reference**:
Legacy level-0 or `src/backend/cir` code used as a source of implementation patterns, test ideas, or behavioral comparison after spec alignment. A legacy reference is acceptable only when level-1b owns the resulting behavior and does not require legacy execution for success.
_Avoid_: copy-through, fallback path, unreviewed port

**Oracle dependency**:
A compiler success path that requires a builtin or oracle implementation to supply key language capability instead of the self-hosted compiler path. An oracle may be a ruler for comparison or fixture generation, but it must not be a leg the compiler stands on.
_Avoid_: oracle-backed success, builtin-backed success, hidden host behavior

**ChibaCC**:
The Chiba ecosystem parser generator that consumes lexer-produced `TokenItem` streams and produces parser AST results. ChibaCC grammars do not parse characters and do not use string terminals; character handling belongs to ChibaLex.
_Avoid_: character parser, scanner parser, string-terminal grammar

**LabeledAST**:
The top-level ChibaCC parse result wrapper that distinguishes complete AST success from recoverable or skipped error regions. It belongs to the generated start driver output, not to every internal grammar rule.
_Avoid_: rule result wrapper, internal AST type, parser error node

**ChibaCC LL(*) parser**:
The ChibaCC parsing model for ordinary grammar rules: generated recursive-descent parsing with arbitrary lookahead where needed, plus explicit Pratt islands for expression precedence. Alternative retry and recovery are modeled with multi-shot continuation semantics.
_Avoid_: PEG parser, LR parser, GLR parser, character parser

**Parser retry continuation**:
The multi-shot continuation captured while trying ChibaCC alternatives or recovery paths. It is semantically and operationally `shiftn`; generated parsers must not replace it with a separate retry stack fallback.
_Avoid_: one-shot parser retry, shift retry, exception-only recovery, retry stack fallback

**ChibaCC source generation complete**:
The state where ChibaCC can generate parser source that represents the requested grammar and uses real parser retry continuations. This is distinct from ChibaCC AST primary path completion, which also requires the generated parser to execute through the compiler backend.
_Avoid_: AST primary complete, generated parser executed, parser backend complete

**ChibaCC action expression**:
The arbitrary Chiba expression after `=>` in a grammar alternative or Pratt arm. ChibaCC owns binding labels and splicing this expression into generated parser source; the main Chiba compiler owns parsing, typing, CPS, closure, and backend semantics for the expression.
_Avoid_: callback syntax, action DSL, restricted action subset, parser-owned typechecking

**ChibaCC action scope**:
The names available inside a ChibaCC action expression. A labeled binding provides both value and span names such as `name` and `name_span`; positional bindings provide `$0` and `$0_span`; Pratt actions provide `$lhs` and `$lhs_span`; every alternative provides `$span`, while `_` bindings are ignored as values but still contribute to `$span`.
_Avoid_: i64 cast scope, spanless action, ignored token loss

**ChibaCC single AST model**:
The ChibaCC rule result model where grammar rules return variants of one user-defined `AST` type rather than each rule declaring its own result type. This favors bootstrap migration and keeps generated parser plumbing uniform.
_Avoid_: typed rule result model, per-rule AST type, parser-owned type hierarchy

**ChibaCC token binding**:
The default label binding for a token atom exposes the token payload as the action value and exposes the token span through the paired span name. Full token access must be explicit rather than replacing the common payload binding path.
_Avoid_: full token by default, manual payload unwrap, spanless payload binding

**Level-1b bootstrap complete**:
The state where C12 two-round bootstrap comparison succeeds through `level1c -> level1c-next -> level1c-next2` without legacy dependency or oracle dependency. The result must run the required specs and gates outside a Node-only path and record hashes, toolchain versions, and any accepted manifest differences.
_Avoid_: half-bootstrap, gate-green bootstrap, Node-only success

**Comment-only backend output**:
Backend output where comments stand in for executable Core, WAT, continuation frame, closure environment, or layout behavior. Comments may explain debug dumps or contract smoke fixtures, but they must not count as backend success.
_Avoid_: comment emitter, explanatory artifact, fake WAT

**Spec alignment audit**:
A pass over the external Chiba level-1 spec before language implementation work. It identifies hard semantic rules, compares them to level-1b behavior, and classifies gaps before cleanup or implementation changes begin.
_Avoid_: code-first cleanup, stub tidying, implementation-only audit

**Truthfulness audit**:
A fast feedback loop that detects level-1b paths whose names or gates imply semantic progress while the implementation still depends on stubs, empty facts, comment-only output, legacy execution, or oracle-backed success. It reports blockers; it does not implement the missing semantics.
_Avoid_: semantic gate, implementation pass, bootstrap validation

## Example Dialogue

Dev: "The migration gate is green. Is level-1b primary now?"

Domain expert: "Not unless level-1b itself carries the semantics. A green mapping or contract smoke does not establish primary compiler behavior."

Dev: "The backend gate sees the pass and a WAT file exists. Is that semantic validation?"

Domain expert: "No. That is contract smoke unless the emitted artifact carries the required behavior and the gate can reject semantic holes."

Dev: "A pass is not implemented yet. Can it return the module unchanged?"

Domain expert: "Only in contract smoke. On a primary path, an incomplete pass must fail closed with a precise blocker."

Dev: "What should the blocker say?"

Domain expert: "Use a blocker class plus the missing semantic, such as `missing-facts: continuation replay-safety facts absent`."

Dev: "Can level-1b compare itself against legacy output?"

Domain expert: "Yes, if legacy is only an oracle or diff reference. No, if legacy output is required for the level-1b success path."

Dev: "Can we borrow an algorithm from level-0?"

Domain expert: "Yes, as a legacy reference. First check it against the spec, then re-own the behavior in level-1b."

Dev: "Can an oracle stay after level-1b bootstrap completes?"

Domain expert: "Only as a ruler. Any oracle dependency that supplies key compiler capability must be removed."

Dev: "Is level-1b bootstrapped after it emits one next compiler?"

Domain expert: "No. Bootstrap complete means the two-round C12 comparison succeeds without legacy or oracle dependency."

Dev: "The WAT file has comments for every core operation. Is the backend working?"

Domain expert: "No. Comments may explain behavior, but executable backend output must carry it."

Dev: "Can we clean language stubs before reading the spec?"

Domain expert: "No. Run a spec alignment audit first, then decide which stubs become blockers, gates, or real implementations."

Dev: "The truthfulness audit fails. Did we break the build?"

Domain expert: "No. It is a feedback loop for known false-green paths; failure means it found blockers that must not be hidden."

## Current Working Context — 2026-06-01

**Active P0 checkpoint**:
Current work has cleared the generated ChibaCC parser WAT execution checkpoint
and the level-1b smoke now runs its emitted seed WAT `main`. This is still not
P0 complete and not level-1b self-bootstrap. The active task now moves back to
making level-1b carry primary semantics instead of depending on the active
`src` path.

**User-defined bootstrap target**:
P0 bootstrap means level-1b can compile enough of itself through Wasm to replace
the `src` level-1c seed path and level-0 bootstrap path. Narrow C07-C11 runnable
fixtures, generated parser WAT, or Node-only runner success are useful evidence,
but they are not that target.

**Latest successful command**:

```sh
timeout 120 vp run level1b:smoke
timeout 360 vp run level1b:chibacc-mini
timeout 120 vp run level1b:c07-source-driver
timeout 120 vp run level1b:c08-semantic
timeout 120 vp run level1b:c09-control-cps
timeout 120 vp run level1b:c10-closure-package
timeout 120 vp run level1b:c11-backend
git diff --check
```

**Checkpoint result**:

- simple/pratt/list/continuation/attribute/calculator generated parser source
  parses, checks, emits WAT, and runs under the WAT runner.
- Full `src/frontend/chiba-level1.chibacc` executable parser WAT emits and runs:
  `.scratch/level-1b/chibacc-full/chiba-level1-parser.exec.wat`.
- Full expression parser WAT emits and runs:
  `.scratch/level-1b/chibacc-full/chiba-level1-parser.expr.exec.wat`.
- AST evidence was emitted:
  `.scratch/level-1b/chibacc-mini/ast-primary-evidence.json`.
- `level1b:smoke` now emits and runs `.scratch/level-1b/level1b-main.seed.wat`
  through the WAT runner; the current empty-request result is still `1`, so this
  proves invocation and runtime reachability, not successful compilation.
- C07 source facts, C08 typed facts, C09 CPS facts, C10 closure/continuation
  package facts, and C11 backend runner currently pass for the narrow slices in
  this checkout.
- C11 writes inspectable/runnable WAT artifacts, including:
  `.scratch/level-1b/c11-backend/contn-multiframe-spine.wat`,
  `.scratch/level-1b/c11-backend/contn-param-resume.wat`, and
  `.scratch/level-1b/c11-backend/ast-primary-typed-main.wat`.

**Latest fixes that matter**:

- `level-1b/compiler/semantic/type_infer.chiba` no longer stops parsing before
  `infer_types`; this fixed the previous `call $infer_types` without function
  definition WAT failure.
- Core call symbols in level-1b backend were normalized toward `str`, and an
  incorrectly lowered helper signature was removed from `wat_emit`.
- Active `src/backend/wasm/wat.chiba` now knows
  `CoreExprKind__CoreExprCall__a3` field 0 is `(ref $array_u8)`, fixing the
  pattern payload local type used by generated level-1b backend WAT.
- Parser-owned AST arithmetic lowers through `TypedExprPrimitiveBinary`, not
  helper calls like `i32.op_add`; prefix `-x` lowers as primitive `0 - x`.
- ContN Core ops carry `frame_index` / `frame_count`; stackless resume WAT names
  include the frame index, and frame-chain WAT materializes one spine node per
  frame.
- Continuation parameter resume is covered by the C11 `contn-param-resume`
  artifact, currently validating `resume_param(37) -> 37`.
- Empty source project loading no longer calls the host import path; it returns
  an internal empty project surface. Non-empty project loading still imports
  `env::std.source_load_project`, so direct `wasmtime` instantiation without the
  runner's proxy import still fails on that import.
- The source/pass-driver entrance path now uses helper accessors for common
  `project.facts.*` and source gate fields to avoid active seed lowering hitting
  unsupported chained field/method access on the smoke path.

**Key files for continuation**:

- `tools/node/run-level1b-chibacc-mini.mjs`: mini ChibaCC generated parser runner and executable WAT harness.
- `tools/node/run-level1b-smoke.mjs`: seed level-1b smoke; now requires the
  emitted seed WAT `main` to run, not only compile.
- `src/backend/wasm/wat.chiba`: active seed WAT emitter/runtime used by
  `level1c.o`; treat as a reference and bootstrap dependency, not the future
  level-1b semantic provider.
- `src/backend/cir/typed.chiba`: active seed CIR typing pass; use for algorithm
  shape when porting typed traversal into level-1b.
- `level0/src/chibacc/codegen.chiba`: native generator source used by the current temp chibacc build.
- `level-1b/std/chibacc/codegen.chiba`: level-1b chibacc generator model, not yet the full primary path.

**Current local invariant**:
For generated parser execution, `MatchResult` carries `AST`, not `i64`, and
`LabeledAST` carries `OK(AST, Vec)` / `Err(Option[AST], Vec)`. Do not convert
these back to `i64` just to satisfy a narrow WAT validator issue.

**Immediate implementation direction**:

- The first active problem is `AST -> typed AST` and `typed AST/CPS -> level-1b
  Core/WAT` lowering. Before writing more chibacc surface, inspect how the
  active `src` path lowers AST/type/CIR/WAT, then port the algorithmic shape into
  level-1b with cleaner nanopass boundaries.
- The immediate smoke-path problem is to stop treating the empty request as the
  runnable seed. Empty input should construct a small parser-owned internal demo
  project, for example `demo::main = 2 + 3 * 4`, so
  `compile_request_to_wat -> run_nanopass_wat -> source -> semantic -> CPS ->
  closure -> backend -> WAT` returns `Ok` and `level-1b/src/level1b_main.chiba`
  can return `0`.
- Concretely, read these seed-path files before the next implementation slice:
  `src/backend/cir/typed.chiba`, `src/backend/cir/lower.chiba`,
  `src/backend/wasm/wat.chiba`, and `src/chiba_level1c_main.chiba`. They show
  the working ownership flow today; level-1b must re-own the behavior, not call
  back into it.
- Treat the current `src` path as a working reference for algorithm shape only:
  parser AST ownership, typed expression traversal, CIR block construction, and
  WAT layout emission should be understood there before changing level-1b. The
  final level-1b path must not call back into `src` for semantic success.
- Full typed AST traversal must feed real expression bodies, not source-slice or
  narrow body facts.
- CPS/reset/shift/shiftn must lower real bodies and preserve tail form after CPS.
- Closure and ContN lowering must extract captures, envs, frame bodies, and
  stackless frame-chain functions for real inputs.
- Pattern/match lowering must become an executable decision tree over tests,
  field extraction, and `if`/`else` joins, including deep patterns and
  exhaustiveness. Current obligations/smokes do not yet equal full lowering.
- Core/block/WAT must consume real AST/CPS-derived inputs, not only narrow
  fixtures.
- Branching is part of the primary path, not a follow-up: `if`, `else if`,
  `if let`, `match`, fallback/default arms, short-circuiting, and nested branch
  joins must all survive typed traversal, CPS join planning, and Core/WAT
  lowering.
- Keep `src` as a reference and active seed path only. Copying behavior is
  acceptable only after re-owning it in level-1b; `src` must not remain the
  semantic provider for a claimed level-1b success.
- Preserve the CIR/backend split while porting: CIR is backend-neutral; WAT,
  Wasm-GC layout ids, `funcref`/`eqref`, Binaryen details, and target ABI details
  belong below the backend boundary.
- Do not edit generated `.scratch` artifacts.

**Current remaining P0 blocks**:

- **Typed AST primary path**: C07/C08 still rely on parser evidence plus many
  source-slice/scanner facts. Replace `scan.chiba` facts with chibalex token
  stream + chibacc AST item/body facts, including namespace-qualified symbols,
  attributes, tuple/ADT nodes, method/operator/index calls, branch nodes, and
  reset/shift/shiftn expressions.
- **Seed smoke primary input**: empty request currently avoids host loading but
  still returns a diagnostic result. Replace the empty surface with a minimal
  parser-owned AST project so the smoke path proves real source/semantic/backend
  flow and can expect `main -> 0`.
- **CPS/control path**: one-pass CPS + beta must run on real typed expressions.
  Ordinary calls should be tail-form after CPS; `if`/`match`/short-circuit joins
  must become explicit continuations; `ContN` materialized frames are the
  explicit non-tail exception.
- **Closure/continuation path**: closure capture/env extraction, call-site
  rewrite, boxed `Cont1` consumed-state lowering, and `ContN` stackless frame
  body/capture projection must be derived from real CPS inputs, not shell facts.
- **Core/WAT executable path**: C11 must lower real AST/CPS-derived Core blocks
  into runnable WAT. Existing WAT artifacts prove slices such as arithmetic,
  branch, tuple-field, closure call_ref, boxed Cont1, and ContN shells; they do
  not yet prove arbitrary compiler bodies or self-bootstrap.

**Current collaboration invariant**:
Documentation must not re-label narrow runnable slices as P0 completion. When a
slice is executable but still fixture-shaped, say so directly. The next useful
engineering move is not more gate surface; it is replacing source-slice facts
with real AST-owned typed/CPS/Core inputs and keeping the generated WAT runnable.

**Validation ladder for this checkpoint**:

```sh
timeout 600 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --entry chiba_level1c_main.chiba --output level1c.o
timeout 120 vp run level1b:smoke
timeout 360 vp run level1b:chibacc-mini
timeout 120 vp run level1b:c07-source-driver
timeout 120 vp run level1b:c08-semantic
timeout 120 vp run level1b:c09-control-cps
timeout 120 vp run level1b:c10-closure-package
timeout 120 vp run level1b:c11-backend
git diff --check
```

Latest successful seed rebuild:

```text
target/debug/level1c.o 18057840 2026-06-01 16:46:53 +0200
```

Do not set `CHIBACC=/tmp/chibacc-project-clean/target/debug/chibacc.clean.o`
unless that binary exists. The successful run used the runner default
`./chibacc.o`.

**Collaboration note**:
If another engineer starts here, read `HANDOFF.md` first. It contains the exact
temporary checkpoint state, known failing errors, and the do-not-do list.
