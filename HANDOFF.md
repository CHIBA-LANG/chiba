# Handoff — chiba-level1 bootstrap / level-1b truthfulness cleanup

Date: 2026-05-25

This document is for a fresh agent continuing the work. It intentionally does **not** duplicate the full roadmap/spec text. Use the referenced files as source of truth.

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
	 - Current state: source parser facts are data-driven. `ast_primary_path_blocked` is derived from per-module `ast_primary_path` facts, but all modules still have `has_ast=false`; chibacc AST primary execution is not done.

- `level-1b/compiler/closure/*`
	- Intended usage/CPS → continuation simplify → closure conversion → lambda lift → env simplify pipeline.
	- Currently mostly contract/skeleton; capture sets, env field rewrite, real call rewriting are not complete.

- `level-1b/compiler/backend/*`
	- Intended Wasm-GC Core/layout/validation/WAT pipeline.
	- Current `wat_emit` is comment-like and does not represent real backend behavior yet.

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
- level-1b is currently considered **unusable** as primary compiler implementation until contract-only stubs are cleaned up or replaced.
- Next major work is not “run second bootstrap”; it is **level-1b truthfulness cleanup** and real continuation/closure/callable lowering design implementation.

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

1. Commit/checkpoint the current truthfulness + continuation surface work.
2. Continue level-1b truthfulness cleanup:
	 - find pass-through stubs in `level-1b/compiler/control/*`, `level-1b/compiler/closure/*`, `level-1b/compiler/backend/*`;
	 - mark explicit blockers or replace fake pass-throughs with real facts/gates;
	 - ensure gates distinguish contract smoke from real semantic validation.
3. Move from grammar surface to semantic plumbing:
	 - update CIR/type IR for `Cont1` / `ContN` and callable storage obligations;
	 - keep `shift :tag` / `shiftn :tag` tag names alive through lowering;
	 - ensure explicit `cont1` / `contN` storage does not get erased into ordinary callable storage.
4. Then implement real lowering contracts:
	 - direct `Cont1`
	 - boxed `Cont1`
	 - `ContN` package
	 - erased callable ADT
	 - no-capture closure direct path

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

---

# Temporary handoff checkpoint — P0 mainline WAT execution

Date: 2026-05-27

This checkpoint exists so another engineer can continue from the exact current
state. It is **not** a P0 completion checkpoint.

## Current branch and dirty scope

- Branch: `master`
- Remote tracking: `master...origin/master`
- Current worktree has 19 modified files before this handoff update.
- The large generated file `src/frontend/chiba_level1_parser.chiba` is modified because the frontend grammar was regenerated earlier. Do not hand-edit generated parser output; change `src/frontend/chiba-level1.chibacc` / generator helpers and regenerate.
- The most recent manual edit before this handoff was in `tools/node/run-level1b-chibacc-mini.mjs`: generated harness token pushes no longer cast `TokenSpan` to `i64`.

Modified areas currently in the worktree:

- `TODO.md`: small truthful notes about global constants and missing match cases.
- `level-1b/compiler/*`: semantic/CPS/backend fact threading work.
- `level-1b/std/chibacc/codegen.chiba`: recovery-codegen plumbing.
- `level0/src/chibacc/*`: native chibacc generator/runtime changes used as the current generator source.
- `src/backend/cir/*`: typed/alpha support for the active level1c path.
- `src/backend/wasm/wat.chiba`: WAT runtime/type/emission support for generated parser execution.
- `src/frontend/*`: chibacc grammar/helper/parser refresh.
- `tools/node/run-level1b-*.mjs`: runner and mini generated-parser WAT execution harness changes.

## Active objective

User objective remains:

- finish all P0,
- do not add extra gates/TODO as substitute progress,
- include unit tests,
- compile real, inspectable, runnable WAT/wasm,
- only call back for chibacc proper once P0 blockers are actually cleared.

Do not mark P0 done from this checkpoint. The immediate local task is narrower:
make `level1b:chibacc-mini` generated parser WAT validate and run through
`tools/node/run-wat.mjs`, then expand coverage to C08-C11 and the remaining P0
semantic slices.

## Last reproduced failure

Command:

```sh
timeout 20 node tools/node/run-wat.mjs .scratch/level-1b/chibacc-mini/simple.exec.exec.wat --invoke main
```

Observed failure class:

- Binaryen/Wasm validator rejects generated parser WAT.
- The source parser file parses/checks and WAT is emitted, but WAT local/param
  types are inconsistent.

Representative errors:

- `match_token_Ident_span`: `MatchOK` expects first arg `AST`/`eqref`, but local payload was emitted with the wrong WAT type.
- `match_token`: `streq` expects `(ref $array_u8)` args, but local `want` was emitted as the wrong WAT type.
- `parse_rule_0_alt_0_step_*`: `__v*` AST step params and locals are still inconsistently typed.
- `harness_tokens`: `tokens` local and `vec_push` args were emitted as `i64` instead of ref/`eqref`.

Current generated source shape from `.scratch/level-1b/chibacc-mini/simple.exec.exec.chiba` is already closer to the intended model:

```chiba
data MatchResult {
    MatchOK(AST, i64, i64),
    MatchFail(i64)
}

data LabeledAST {
    OK(AST, Vec),
    Err(Option[AST], Vec)
}

def parse_rule_0_alt_0_step_3(..., __v0: AST, __v1: AST, __v2: AST): MatchResult = {
    let name: Str = __v0 as Str
    let value: Str = __v2 as Str
    MatchOK(Assign(name, value), pos, recovered)
}
```

So the next problem is mostly active `level1c.o` WAT emission/type propagation,
not a generated-source parse problem.

## Immediate next fix

Continue in `src/backend/wasm/wat.chiba` and, if needed,
`src/backend/cir/typed.chiba`.

Expected fixes:

- local declarations for `L1StmtLet` should derive from the value expression
  type instead of defaulting to `i64`;
- local declarations for `L2StmtLet` already have explicit `CirType`, but verify
  casts like `__v0 as Str` become `(ref $array_u8)` locals;
- match-pattern payload binders must declare locals using the matched variant
  field type, not always `i64`;
- function params should continue using `wat_param_valtype(...)` against source
  AST declarations, because generated parser functions now declare `AST`, `Str`,
  `Vec`, and `TokenSpan`;
- casts between `AST`/nominal/`Str` need real WAT casts or layout-aware no-op,
  not scalar `i64` behavior.

After any source fix, rebuild and rerun:

```sh
timeout 300 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --entry chiba_level1c_main.chiba --output level1c.o
timeout 120 vp run level1b:chibacc-mini
```

If only inspecting the existing emitted WAT error, use:

```sh
timeout 20 node tools/node/run-wat.mjs .scratch/level-1b/chibacc-mini/simple.exec.exec.wat --invoke main
```

## Important generator note

`tools/node/run-level1b-chibacc-mini.mjs` currently prefers:

```js
/tmp/chibacc-project-mainline/target/debug/chibacc.new.o
```

if it exists, otherwise `./chibacc.o`.

This means the generated parser used by the mini runner may come from the
temporary `/tmp/chibacc-project-mainline` generator, not only from files in this
repo. If continuing from a fresh environment, either rebuild that temp generator
or point the runner at the intended native chibacc binary.

Known rebuild command for that temp generator:

```sh
timeout 120 ./chibac_amd64-unknown-linux_chiba_dev.o --project /tmp/chibacc-project-mainline --entry main.chibacc.chiba --output chibacc.new.o
```

The `--output` must be a relative filename in that command.

## Current P0 truth

P0 is still incomplete. The current closest blocker is generated parser WAT
execution, but the broader P0 still includes:

- chibacc-mini generated parser WAT execution;
- C08-C11 real AST/CPS/Core/WAT input coverage beyond narrow slices;
- namespace/method/operator resolution;
- typed AST traversal and real expression elaboration;
- pattern/pipe/ADT tuple/intrinsic lowering;
- one-pass CPS + beta over real expressions and branches;
- closure/Cont1/ContN capture extraction and lowering, especially ContN frame bodies;
- UTF-8/XID shared frontend source;
- regex VM/bootstrap subset;
- chibalex/chibacc self-host primary path.

Do not claim “only chibacc remains”.

## Do not do

- Do not write new gate TODOs instead of implementing the blocker.
- Do not edit `.scratch` generated files as the fix.
- Do not hand-edit `src/frontend/chiba_level1_parser.chiba`; regenerate it.
- Do not use `chibac_amd64-unknown-linux_chiba_dev.o` as proof for level-1b
  generated parser execution. It is the level0 seed used to rebuild
  `target/debug/level1c.o`.
- Do not introduce fake-total node types such as `ExprI32Add`,
  `TypedExprTailCallI32Const`, or `CoreExprParam0` to pass a narrow case.
- Do not couple CIR to Wasm-GC layout or WAT opcodes.
