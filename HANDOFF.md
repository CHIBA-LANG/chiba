# Handoff — chiba-level1 bootstrap / level-1b truthfulness cleanup

Date: 2026-06-01

This document is for a fresh agent continuing the work. It intentionally does **not** duplicate the full roadmap/spec text. Use the referenced files as source of truth.

## Current checkpoint: P0 generated-parser WAT + seed-WAT smoke + C07-C11 narrow slices

Current status: **P0 is not complete**. Do not claim only chibacc remains, and
do not describe generated parser WAT success as level-1b self-bootstrap.

Latest checkpoint result: `level1b:smoke` now emits and runs its seed WAT
`main`, and `level1b:chibacc-mini` passes end-to-end with the default native
generator (`./chibacc.o`). This includes simple/pratt/list/continuation/
attribute/calculator generated parser WAT execution, full
`src/frontend/chiba-level1.chibacc` executable parser WAT execution, full
expression parser WAT execution, and AST evidence emission.

Latest successful command:

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

Important output:

```text
[PASS] run generated parser wat simple.chibacc
[PASS] run generated parser wat pratt.chibacc
[PASS] run generated parser wat list.chibacc
[PASS] run generated parser wat continuation-type.chibacc
[PASS] run generated parser wat attribute-args.chibacc
[PASS] run generated parser wat calculator.chibacc
[PASS] run full chiba-level1 executable parser wat .scratch/level-1b/chibacc-full/chiba-level1-parser.exec.wat
[PASS] run full chiba-level1 expression parser wat .scratch/level-1b/chibacc-full/chiba-level1-parser.expr.exec.wat
[PASS] chibacc mini AST evidence .scratch/level-1b/chibacc-mini/ast-primary-evidence.json
```

Additional latest runner state:

- `level1b:smoke`: passed; it now runs `.scratch/level-1b/level1b-main.seed.wat`
  `main` through the WAT runner. Current result is still `1` because the empty
  request path returns a diagnostic code, so this is runtime reachability, not
  a successful compile.
- `level1b:c07-source-driver`: passed.
- `level1b:c08-semantic`: passed; still prints known `[REF-FAIL]`
  reference/oracle comparison notes.
- `level1b:c09-control-cps`: passed.
- `level1b:c10-closure-package`: passed.
- `level1b:c11-backend`: passed and produced 27 WAT/wasm artifacts.
- Important C11 artifacts:
  `.scratch/level-1b/c11-backend/contn-multiframe-spine.wat`,
  `.scratch/level-1b/c11-backend/contn-param-resume.wat`,
  `.scratch/level-1b/c11-backend/ast-primary-typed-main.wat`.
- Observed C11 runtime results:
  `contn-multiframe-spine` has `head_resume -> 21`, `tail_resume -> 22`,
  `frame_count -> 2`; `contn-param-resume` has `resume_param(37) -> 37`;
  `ast-primary-typed-main` has `main -> 14` and `tuple_field_ast -> 2`.

What was fixed in this checkpoint:

- `level-1b/compiler/semantic/type_infer.chiba`: fixed invalid inline match arm
  separators that caused parsing to stop before `infer_types`, which previously
  produced WAT with `call $infer_types` but no `(func $infer_types ...)`.
- `level-1b/compiler/backend/core.chiba`,
  `level-1b/compiler/backend/wat_emit.chiba`, and
  `level-1b/compiler/backend/validate_core.chiba`: normalized Core call symbols
  to `str` and removed an incorrectly lowered helper signature from the WAT
  emitter path.
- `src/backend/wasm/wat.chiba`: taught layout field typing that
  `CoreExprKind__CoreExprCall__a3` field 0 is `(ref $array_u8)`, so pattern
  payload locals for Core call targets are declared as string refs instead of
  `eqref`.
- Parser-owned AST arithmetic now lowers to `TypedExprPrimitiveBinary`, and
  prefix negation lowers to primitive `0 - x`; do not reintroduce helper-tailcall
  fake nodes such as `ExprI32Add` / `TypedExprTailCallI32Const` /
  `CoreExprParam0`.
- Receiver-first method/index argument lowering was fixed to avoid duplicating
  the receiver.
- `typed_if_condition_from_ast_node` now honors condition node id `0`; it only
  falls back to literal true when there is no condition node.
- ContN multi-frame lowering now threads `frame_index` through Core and WAT
  naming. Stackless resume functions use names like
  `$chiba.contn.resume.<owner>.frame.<index>`, and frame-chain WAT materializes
  one spine node per `frame_count`.
- Chibacc AST evidence now includes direct AST tuple/field nodes
  (`SourceAstExprNodeStructNew` + `SourceAstExprNodeFieldGet`), and C11 runs the
  `tuple_field_ast -> 2` artifact.
- `level-1b/compiler/source/project.chiba`: empty source project loading now
  returns an internal empty surface instead of calling host
  `env::std.source_load_project`. Non-empty project loading still imports that
  host builtin.
- `level-1b/compiler/source/semantic_gate.chiba` and
  `level-1b/compiler/driver/pass_driver.chiba`: common field/method chains were
  replaced by helper accessors on the seed smoke entrance path to avoid active
  seed lowering hitting unsupported chained access.
- `tools/node/run-level1b-smoke.mjs`: the smoke now requires running emitted seed
  WAT `main`; it is no longer only a generate/assemble check.

Latest rebuild completed:

```sh
timeout 600 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --entry chiba_level1c_main.chiba --output level1c.o
```

It wrote:

```text
target/debug/level1c.o 18057840 2026-06-01 16:46:53 +0200
```

Recommended next validation:

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

Do not use `CHIBACC=/tmp/chibacc-project-clean/target/debug/chibacc.clean.o`
unless that file exists. In this checkout the runner defaulted to `./chibacc.o`
and passed.

Important smoke caveat:

- Direct `wasmtime --invoke main .scratch/level-1b/level1b-main.seed.wasm`
  still cannot instantiate without a host import provider because non-empty
  loading imports `env::std.source_load_project`.
- `tools/node/run-wat.mjs` provides proxy imports and currently reaches `main`.
- The generated seed WAT still contains many unreachable blocks for unsupported
  method/field lowering. The current runtime path avoids some of them through
  helper accessors; do not treat their mere presence as success or as a blocker
  unless they are on the active runtime path.

Next P0 focus is no longer the generated-parser WAT blocker. Continue by making
level-1b carry primary semantics instead of depending on the active `src` path:
full typed AST traversal, real CPS/reset/shift/shiftn body lowering, closure and
ContN capture/frame lowering, Core/block/WAT emission from real AST/CPS inputs,
and then the actual level-1b self-bootstrap path.

The user's current definition of P0 self-bootstrap is strict: level-1b must be
able to compile enough of itself through Wasm that `src` level-1c and level-0
can be removed from the success path. C07-C11 green, generated parser WAT, and
Node runner artifacts are intermediate evidence only.

Do not spend the next session widening gate vocabulary. The highest-value
implementation work is to replace the narrow source-slice/body-fact path with
real AST-owned terms that flow through C08 -> C09 -> C10 -> C11 and still produce
inspectable/runnable WAT.

Immediate concrete smoke slice:

1. Build a minimal internal parser-owned AST project for empty input, preferably
   near `level-1b/compiler/source/project.chiba`.
2. Use namespace `demo`, owner symbol `demo::main`, and AST nodes for
   `2 + 3 * 4`, matching the existing chibacc-mini evidence shape.
3. Populate parser/source facts with nonzero AST item, namespace, def item,
   owner symbol, and expression node facts.
4. Change the empty request path from `empty_project_surface()` to that internal
   demo project, then update `level1b:smoke` expected result from `1` to `0`
   only after `compile_request_to_wat` returns `Ok`.
5. If the seed WAT traps on unsupported field/method access, inspect the first
   runtime path and replace source-level chained access with helper accessors
   rather than faking success.

Current user-directed priority: start from AST type and level-1b WAT lowering.
Read the active `src` lowering path to understand the working flow, then port or
rewrite the algorithmic shape into level-1b. Do not leave `src` as the semantic
provider. Keep CIR target-independent; WAT/Wasm-GC/Binaryen details stay in the
backend layer.

Concrete reference files to inspect before changing the next P0 slice:

- `src/frontend/*`: how parser-owned AST gets represented before backend input.
- `src/backend/cir/typed.chiba`: current active typed/CIR facts and expression
  typing conventions.
- `src/backend/cir/lower.chiba`: active CIR construction flow and branch/call
  ownership.
- `src/backend/wasm/wat.chiba`: target-specific layout and WAT emission details
  that must stay below level-1b Core/CIR boundaries.
- `src/chiba_level1c_main.chiba`: active seed compiler entry and ownership of
  the current `target/debug/level1c.o` path.
- `level0/src/backend/cir/lower.chiba` and `level0/src/backend/bir/lower.chiba`:
  reference flow only; do not copy known level0 branching/scanner bugs.

## P0 work still left

These are implementation tasks, not gate-writing tasks:

1. **Typed AST primary path**
   - Replace scanner/source-slice facts with parser-owned AST item/body facts.
   - Cover namespace-qualified symbols, attributes, tuple/ADT construction and
     access, method/operator/index calls, branch nodes, and reset/shift/shiftn
     expressions.
   - `scan.chiba` may remain only as a transition scaffold; it must not stay the
     source of truth for day0 UTF-8 or namespace/item semantics.
   - Empty smoke input should become a parser-owned AST demo project, not an
     empty-project diagnostic path.

2. **One-pass CPS + beta**
   - Run CPS on real typed expressions, not body-shape facts.
   - Ordinary expressions/calls must become tail-form after CPS.
   - `if`/`else if`/`if let`/`match`/short-circuit joins must lower through
     explicit continuation joins.
   - `reset`/`shift`/`shiftn` must lower real bodies; `ContN` frame
     materialization is the explicit non-tail exception.

3. **Closure and continuation lowering**
   - Extract free-var captures and env fields from real CPS terms.
   - Rewrite closure call sites instead of only emitting no-capture or
     conservative env shells.
   - Lower boxed escaping `Cont1` through the consumed-state machine.
   - Lower `ContN` with stackless frame bodies, capture projection, frame chain,
     and repeatable package derived from real inputs.

4. **Core/block/WAT executable primary path**
   - Lower CPS/closure output into backend-neutral Core blocks first.
   - Keep CIR/Core free of WAT, Wasm-GC layout ids, `funcref`, `eqref`, and
     Binaryen details.
   - Emit runnable WAT for real compiler bodies, not only arithmetic/branch/
     tuple/Cont shell fixtures.
   - Preserve inspectable artifacts, but do not add fake gates to re-label
     narrow slices as progress.

First concrete coding slice recommended for the next agent:

1. Read `src/backend/cir/typed.chiba`, `src/backend/cir/lower.chiba`, and
   `src/backend/wasm/wat.chiba`.
2. In level-1b, extend the AST-owned typed expression representation/traversal
   so function bodies can carry nested branch/call/tuple/ADT/method/operator
   terms as data.
3. Thread those terms through C09 and C11 for one executable non-trivial body
   without introducing special-case fake nodes such as `ExprI32Add`,
   `TypedExprTailCallI32Const`, or `CoreExprParam0`.
4. Only after that, run C07-C11 and inspect the generated WAT.

## Where to look first

- Main repo: `/home/lemonhx/Desktop/LJVM/chiba-level1`
- Spec repo: `/home/lemonhx/Desktop/LJVM/chiba-org-web`
- Main roadmap: `TODO.md`
- Checkpoint history: `TODO.checkpoint.md`, `TODO.level1.md`, `TODO.level1-b.type.md`, git history
- Current continuation/type decisions:
	- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/type_system/continuations.md`
	- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/type_system/send.md`
	- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/chiba-level1-spec/02-control-flow/shift.md`
	- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/chiba-level1-spec/10-ir-and-lowering/cir-cps-ir.md`
	- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/chiba-level1-spec/10-ir-and-lowering/passes-and-placement.md`
	- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/chiba-level1-spec/10-ir-and-lowering/cir-to-bir-lowering.md`

## Current git state

Main repo status before the next checkpoint:

- Branch: `master`
- Remote: `origin https://github.com/chiba-lang/chiba-level0`
- `git fetch --all --prune` completed; local branch is ahead of `origin/master`.
- Working tree contains the level-1b truthfulness cleanup, continuation grammar surface, generated frontend refresh, and this handoff/context documentation.
- Current dirty focus is level-1b C07-C11 implementation plus runner checks:
  `supports/bootstrap/continuation-valid.chiba`,
  `level-1b/supports/pre-c10-smokes/multishot_package.chiba`,
  `level-1b/supports/pre-c11-smokes/continuation_core.chiba`,
  `level-1b/compiler/semantic/type_infer.chiba`,
  `level-1b/compiler/closure/closure_convert.chiba`,
  `level-1b/compiler/backend/core.chiba`,
  `level-1b/compiler/backend/wat_emit.chiba`,
  `tools/node/run-level1b-c07-source-driver.mjs`,
  `tools/node/run-level1b-c08-semantic.mjs`,
  `tools/node/run-level1b-c09-control-cps.mjs`,
  `tools/node/run-level1b-c10-closure-package.mjs`,
  `tools/node/run-level1b-c11-backend.mjs`, and
  `tools/node/run-level1b-chibacc-mini.mjs`.
- Recent local edits changed parser-owned arithmetic lowering toward
  `TypedExprPrimitiveBinary`, receiver-first args without duplicate receivers,
  and ContN frame-chain emission that respects Core `frame_count`. Validate
  these before building further ContN work.

The `TODO.md` and new grammar contract fixtures were committed in `6b6f84d`.

Spec repo current last commit is clean at:

- `758f576 Clarify continuation types and their semantics, including surface type sugar for Cont1 and ContN`

Note: the user or another process changed/committed spec repo content after the previous turn. Re-read spec files before further editing.

## How compilation / validation works right now

The project currently has two important compiler paths:

1. **Legacy active first-bootstrap path**
	 - Seed compiler: `./chibac_amd64-unknown-linux_chiba_dev.o`
	 - Current bootstrap compiler object: `./target/debug/level1c.o`
	 - Wasm smoke artifact: `./target/debug/level1c.wasm`
	 - This path is what made `validate:first-bootstrap` green.

2. **level-1b intended primary path**
	 - Source tree: `level-1b/`
	 - It has many C00-C11 contract gates and IR/backend skeletons.
	 - Current judgement: **level-1b is not usable as primary compiler behavior yet**. Many passes are contract-only/stub-only/comment-only. See `TODO.md`.

Main validation commands are package scripts in `package.json`:

```sh
pnpm -s run validate:first-bootstrap
pnpm -s run level1b:type-system
pnpm -s run checkpoint:gates
pnpm -s run semantic:gates
pnpm -s run smoke:parser-errors
pnpm -s run smoke:parser-compare
pnpm -s run smoke:lexer-compare
```

Earlier validated state in this conversation:

- `timeout 360 vp run level1b:chibacc-mini` passed after the latest rebuild.
- `pnpm -s run validate:first-bootstrap` passed.
- `timeout 5 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --entry chiba_level1c_main.chiba --output level1c.o` timed out with exit 124 and did not crash before timeout.
- `level1b:c04-regex`, `level1b:c07-source-driver`, `level1b:c08-semantic`, `level1b:c09-control-cps`, `level1b:c10-closure-package`, and `level1b:c11-backend` passed after the latest level-1b edits.
- `pnpm -s run level1b:type-system` passed.
- parser error smoke, semantic gates, checkpoint gates, all-wat were green.

Do **not** run expensive bootstrap loops casually. The user explicitly wants expensive bootstrap only after a clear staged result and after committing relevant changes.

If `wasmtime` is needed and unavailable in a non-login shell, run:

```sh
source ~/.zshrc
```

Binaryen is intentionally still part of the toolchain. The rule is: Binaryen may assemble/validate/optimize `.wat -> .wasm`, but must not hide Chiba semantic holes.

## What is what

- `src/backend/cir/*`
	- Legacy active path used by first-bootstrap.
	- Still carries real behavior for current green bootstrap.

- `level-1b/compiler/control/*`
	 - Intended answer/control/usage/CPS pipeline.
	 - Current state: typed DFS expression visits feed `CpsModule.expr_visits`; reset/shift/shiftn produce `CpsControlTermFact`; administrative beta and full AST expression CPS remain pending.

- `level-1b/std/regex/*`
	 - Current state: Unicode 15.1 XID ranges are level-1b source data; `Char.is_xid_start/continue` no longer use compiler builtins. Regex program lowering emits real `RegexSplit` / `RegexJump` labels for alternation/repeat, and matcher uses a source-level backtracking VM. Capture numbering/rollback still pending.

- `level-1b/compiler/source/*`
	 - Current state: source parser facts are data-driven. Chibacc-mini and full
	   parser executable evidence can prove parser-owned AST slices, including
	   owner symbol and some expression evidence. Full `SourceItemScanResult`
	   replacement is still not done, and `scan.chiba` still supplies many
	   fallback facts.
	 - Empty project helpers currently avoid host loading for smoke, but they
	   produce an empty/diagnostic surface. Replace this with a parser-owned demo
	   AST before claiming the smoke proves compiler success.

- `level-1b/compiler/closure/*`
	- Intended usage/CPS → continuation simplify → closure conversion → lambda lift → env simplify pipeline.
	- Currently mostly contract/skeleton; capture sets, env field rewrite, real call rewriting are not complete.

- `level-1b/compiler/backend/*`
	- Intended Wasm-GC Core/layout/validation/WAT pipeline.
	- Current `wat_emit` has real executable slices for const/param/tailcall,
	  branch, tuple/heap allocation, boxed Cont1, closure call_ref, erased
	  continuation package, ContN frame/package, and AST-owned typed-main smokes.
	  It is still not a full backend for arbitrary AST/CPS inputs.

- `chiba-level1-grammar-spec/`
	- Positive grammar source/golden fixtures.
- New file `32-test.chiba` covers `shift`/`shiftn` and `cont1`/`contN` grammar.
- Current `level1c.o parse chiba-level1-grammar-spec/32-test.chiba` returns `OK`.
- Lexer/parser compare currently passes 32 specs.

- `chiba-level1-grammar-error-spec/`
	- Negative grammar fixtures.
- New files `111-test.chiba`, `112-test.chiba`, `113-test.chiba` cover continuation grammar error shapes.
- `smoke:parser-errors` currently passes 113 specs.

- `tools/node/*`
	- Validation and smoke harnesses.
	- Several harnesses were recently stabilized to avoid stale runner artifacts and brittle internal IDs.

## Where we are now

The old optimistic roadmap was corrected. Current truth:

- First-bootstrap is green.
- Type-system smoke is green.
- level-1b gates can be green while still not proving real primary behavior.
- level-1b is currently considered **not yet usable** as primary compiler
  implementation because major semantic paths are still narrow slices or
  source-slice/body-fact driven.
- Next major work is not “run second bootstrap”; it is completing real
  AST-owned typed traversal, CPS, closure/ContN lowering, and Core/WAT emission
  until level-1b can compile itself without `src` carrying semantics.

The current `TODO.md` is the concise source of truth for priority order.

## Key design decisions reached in this session

Do not re-litigate unless the user explicitly asks; reference spec/TODO instead.

- `Ref[T]` is the safe mutation surface.
- Multi-shot continuation capturing `Ref[T]` uses shared-reference semantics: no snapshot/copy/rollback of captured cells.
- `shift k { ... }` captures one-shot continuation: `Cont1[A, B]` / `cont1 (A) -> B`.
- `shiftn k { ... }` captures multi-shot continuation: `ContN[A, B]` / `contN (A) -> B`.
- Tagged forms:
	- `shift :tag k { ... }`
	- `shiftn :tag k { ... }`
- `cont1 (A) -> B` is type sugar for `Cont1[A, B]`.
- `contN (A) -> B` is type sugar for `ContN[A, B]`.
- `Cont1` does **not** auto-upgrade to `ContN` when stored.
- Escaping/stored `Cont1` lowers to boxed one-shot consumed-state machine.
- `ContN` lowers to repeatable continuation package/frame spine.
- Parameter-position `(A) -> B` is checked-template callable obligation and can instantiate to function/closure/`Cont1`/`ContN` if obligations allow.
- Storage-position `(A) -> B` lowers to erased callable ADT with variants at least function / closure / boxed `Cont1` / `ContN`.
- `((A) -> B) send` excludes `Cont1`, boxed `Cont1`, `ContN`, and `!send` closures.
- chibalex must support UTF-8 source and identifiers. The current C07 `scan.chiba` is only an ASCII byte-level transition scanner; before it becomes authoritative for namespace/item facts, it must either become UTF-8 aware or fail-closed on non-ASCII facts instead of silently missing them.
- C05 chibalex now has explicit UTF-8/XID identifier policy and codepoint-based state advance via `next_char_offset`; `utf8-ident.chibalex` is the mini fixture.
- C06 chibacc alternative retry/recovery source uses `shiftn retry`; parser retry is multi-shot by contract, not one-shot `shift`.
- C05/C06 generated lexer/parser codegen artifacts no longer carry `ContractOnly` status or `missing-lowering` diagnostics. Lowering now builds typed IR from AST values; codegen emits entry text, but generated parser execution is still not the full C07 AST primary path.
- C07 `compile_if` facts now keep an expression slice, top-level predicate shape, and coarse target/backend/all/or/not/unknown classification. Predicate parsing/eval against target facts is still not wired into item filtering.
- Source semantic gate fail-closes on unknown `compile_if` predicate shape, so unsupported predicates do not pass as ordinary source facts.
- Source scanner detects inline/indented namespace forms and fail-closes until inline namespace block assembly exists. File-header namespace remains the current transition path.
- Source item scan preserves `private` on def/type/data/union/interface/extern; alpha origins and typed item skeletons carry it forward.
- Source scanner now derives `use` declaration facts with path slice, glob marker, and multi-import marker. Actual import resolution still absent.
- Source import policy now builds import scope inputs from owner namespace, explicit uses, and prelude policy. Source semantic gate fail-closes on explicit `use` or default prelude injection until import/name resolution exists.
- Source item scan now attaches line-start `#[compile_if(...)]` facts to the following item. This preserves item-level conditional compilation evidence; real compile_if eval/filtering is still absent.
- Source semantic gate now fail-closes when item-level `compile_if` facts are present but item filtering has not run, preventing disabled items from producing binders/exports/backend symbols.
- Source item scan now attaches the file-header namespace as `owner_namespace`; inline namespace ownership remains fail-closed until block assembly exists.
- Source item scan now preserves item header slices and coarse surface shape: params present, type annotation present, body/initializer present. It still does not parse parameter patterns, type expressions, or body AST.
- Source semantic gate now rejects `extern` items with no scanned type annotation, matching the spec rule that extern ABI boundaries require explicit types. Real ABI signature parsing remains absent.
- Source scanner/gate now recognises `#[world_local]` and `Ref[` in static def headers, then rejects top-level static `Ref` without world-local. Real type-expression parsing remains absent.
- Source semantic gate now rejects `def` items with no scanned body/initializer marker, so typed lowering cannot consume bodyless def skeletons.
- C08 typed item skeletons now classify scanned items as function/static value/extern function/nominal type/data/union/interface before real HM + row inference.
- C08 alpha origins retain source item kind/name/file/owner_namespace/line/column/private/attributes/surface. Type inference builds `TypedItemSkeleton` and checks placeholder pattern coverage, then fail-closes on missing source item type-expression/body inference rather than claiming a typed module.
- C09 replay safety must not mark every usage fact safe. It now fail-closes until replay capture classification exists.
- C09 replay capture taxonomy distinguishes pure value, shared `Ref` cell, and non-replay state. Shared `Ref` is legal for ContN as shared-reference semantics, not snapshot/rollback.
- C10 CPS usage now fail-closes on missing closure/lambda/continuation subject extraction; previous continuation-only use reconstruction was not enough to prove closure directification/capture semantics.
- C10 closure conversion no longer emits backend lowering facts for deleted continuations, preventing deleted continuations from leaking as default direct Cont1 lowering.
- C11 Core ops retain runtime state machine facts. Boxed `Cont1` must carry `CoreConsumedStateMachine`; non-Cont1 ops must not.
- C11 Core ops retain `frame_count`; validator rejects zero-frame ContN frame chains so repeatable packages cannot be backed by empty stackless resume frames.
- C05 chibalex mini generator advances identifier scans by UTF-8 lead-byte length instead of byte-by-byte inside non-ASCII identifiers. Real XID tables and invalid-sequence validation are still primary lexer work.
- C06 chibacc gate rejects `shift retry` in parser retry; retry must remain multi-shot `shiftn retry`.
- C09 control IR now includes callable storage facts/variants for erased callable ADT obligations and a sendable callable invariant that excludes continuation variants.
- C08 capability rules include `check_sendable_callable_storage`; sendable callable storage containing continuation variants returns a capability error.
- C08 ADT/constructor lowering now has `ConstructorMutationObligation` and `ConstructorMutateOnceUsedInput` strategy for exactly-once input -> mutation lowering; real usage/backend integration is still absent.
- No-capture closure must optimize to direct function/funref/inline; no env allocation.
- Non-escaping exactly-once `Cont1` must optimize to direct resume/inline/tail jump; no continuation package.
- CIR must stay target-independent. It must not contain Wasm-GC layout ids, `funcref`/`eqref`, WAT opcodes, Binaryen feature flags, or target ABI details. Those belong in BIR/LIR/backend layout.

## Immediate next session focus

Likely next work, in order:

1. Inspect the active lowering path in `src`, especially AST/type/CIR/WAT
	 flow. Use it as a reference for control flow and data ownership, not as a
	 permanent dependency.
2. Replace level-1b typed body facts with real typed AST traversal:
	 expression nodes, branch nodes, call/operator/method nodes, tuple/ADT nodes,
	 reset/shift/shiftn nodes, and source-owned namespace symbols.
3. Feed typed AST terms into C09 one-pass CPS + beta:
	 ordinary expressions must become tail-form after CPS; branch/match arms join
	 through explicit continuations; multi-shot `ContN` is the explicit materialized
	 frame exception.
	 Use `level0/src/backend/cir/lower.chiba` as the algorithmic reference, not
	 `src/backend/cir/cps.chiba`: level0's `lower_expr(expr, k_meta, ...)` uses a
	 compiler-time meta-continuation and beta-reduces administrative redex during
	 lowering. `level-1r/src/cps.rs` mirrors this shape in Rust as a reference
	 skeleton. level-1b currently has C09 facts and narrow typed-term threading,
	 but not the complete atom/call/lambda/branch/reset/shift/shiftn one-pass CPS
	 transform.
4. Feed CPS into C10 closure/continuation lowering:
	 free-var capture extraction, no-capture direct closure path, boxed `Cont1`
	 consumed-state lowering, `ContN` stackless frame bodies, frame chain, and
	 repeatable package.
5. Feed C10 output into C11 Core/WAT:
	 backend-neutral Core blocks first, then executable Wasm-GC/WAT. Do not encode
	 Wasm layout details into CIR to make a case pass.

Immediate slice to finish before claiming progress:

1. Broaden typed AST traversal from current arithmetic/call/branch slices to
   real source-owned nodes, especially match/if-let, method/operator calls,
   tuple/ADT construction/access, and reset/shift/shiftn bodies.
2. Replace scanner/source-slice body facts with parser-owned AST item/body facts
   through C07 -> C08 -> C09 -> C10 -> C11.
3. Turn reset/shift/shiftn and ContN frame bodies from body-shape facts into
   real typed/CPS/closure/Core inputs, including capture projection.
4. Implement closure capture/env extraction and call-site rewrite, not just
   no-capture or conservative env-shell facts.
5. Lower pattern/match/if-let into executable decision trees with field
   extraction and exhaustiveness handling.
6. Keep each slice executable in WAT, but do not mark P0 complete until the
   source AST -> typed AST -> CPS -> closure/ContN -> Core/WAT path is primary
   enough to start real level-1b chibacc/self-bootstrap work.

## Important workflow constraints

- Keep `TODO.md` truthful. If a gate is only contract smoke, say so.
- Do not broaden grammar to accept invalid specs silently.
- Do not fabricate golden files for syntax the parser does not support.
- Do not couple CIR to Wasm. Keep Wasm-GC details below CIR.
- Prefer small, staged changes with tests.
- Do not run expensive bootstrap unless there is a clear staged result and relevant changes are committed.

## Useful commands

Quick status:

```sh
git --no-pager status --short
git -C /home/lemonhx/Desktop/LJVM/chiba-org-web --no-pager status --short
```

Light validation for docs/whitespace:

```sh
git --no-pager diff --check
git -C /home/lemonhx/Desktop/LJVM/chiba-org-web --no-pager diff --check
```

Main validation, when appropriate:

```sh
pnpm -s run validate:first-bootstrap
pnpm -s run level1b:type-system
```

Targeted gates:

```sh
pnpm -s run smoke:parser-errors
pnpm -s run semantic:gates
pnpm -s run checkpoint:gates
```
