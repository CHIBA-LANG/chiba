# Handoff — chiba-level1 bootstrap / level-1b truthfulness cleanup

Date: 2026-05-24

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
	- Currently many pass-through stubs (`Ok(module)`, empty facts, etc.).

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
- C05/C06 generated lexer/parser codegen artifacts carry `ContractOnly` status and `missing-lowering` diagnostics until executable codegen exists.
- C07 `compile_if` facts now keep an expression slice, top-level predicate shape, and coarse target/backend/all/or/not/unknown classification. Predicate parsing/eval against target facts is still not wired into item filtering.
- Source semantic gate fail-closes on unknown `compile_if` predicate shape, so unsupported predicates do not pass as ordinary source facts.
- Source scanner detects inline/indented namespace forms and fail-closes until inline namespace block assembly exists. File-header namespace remains the current transition path.
- Source item scan preserves `private` on def/type/data/union/interface/extern; alpha origins and typed item skeletons carry it forward.
- Source scanner now derives `use` declaration facts with path slice, glob marker, and multi-import marker. Actual import resolution still absent.
- Source import policy now builds import scope inputs from owner namespace, explicit uses, and prelude policy. Source semantic gate fail-closes on explicit `use` or default prelude injection until import/name resolution exists.
- Source item scan now attaches line-start `#[compile_if(...)]` facts to the following item. This preserves item-level conditional compilation evidence; real compile_if eval/filtering is still absent.
- Source item scan now attaches the file-header namespace as `owner_namespace`; inline namespace ownership remains fail-closed until block assembly exists.
- Source item scan now preserves item header slices and coarse surface shape: params present, type annotation present, body/initializer present. It still does not parse parameter patterns, type expressions, or body AST.
- C08 alpha origins retain source item kind/name/file/owner_namespace/line/column/private/attributes/surface. Type inference builds `TypedItemSkeleton` and checks placeholder pattern coverage, then fail-closes on missing source item type-expression/body inference rather than claiming a typed module.
- C09 replay safety must not mark every usage fact safe. It now fail-closes until replay capture classification exists.
- C10 CPS usage now fail-closes on missing closure/lambda/continuation subject extraction; previous continuation-only use reconstruction was not enough to prove closure directification/capture semantics.
- C11 Core ops retain runtime state machine facts. Boxed `Cont1` must carry `CoreConsumedStateMachine`; non-Cont1 ops must not.
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
