# Chiba Level-1 Context

Date: 2026-06-03

This repository is currently moving through the **level-1r Rust reference compiler** route. The immediate goal is to build a tested, observable Rust baseline compiler first, then port the proven structure back to Chiba for self-bootstrap.

Old level-1b checkpoint logs were removed from this file. Use git history for older C07-C11 / chibacc-mini details.

## Current Source Of Truth

- `AGENTS.md`: highest-priority engineering and semantic rules.
- `level-1r.md`: Rust reference compiler scope and design constraints.
- `TODO.level1-b.md`: broader bootstrap roadmap and remaining language/compiler work.
- `HANDOFF.md`: current working state for the next agent.

## Hard Rules

- Do not infer semantics from string shape outside lexer / chibalex / regex tokenization.
- Identifier legality must come from UTF-8/XID lexer policy or a shared XID table.
- Semantic, pipeline, backend, and lowering passes consume AST, symbol, typed, Core/CIR, or explicit facts.
- Do not add gates as a substitute for implementation.
- Keep Core/CIR target-neutral. WAT/Wasm-GC/Binaryen details belong below Core/CIR.
- Commit frequently after coherent tested slices.

## Level-1R Definition

Level-1R is a Rust reference compiler path. It must cover:

- nanopass pipeline with visible reports,
- source parsing and typed AST,
- name/method/operator resolution,
- one-pass CPS with administrative beta reduction,
- closure and continuation lowering,
- callable storage lowering,
- target-neutral CIR/Core,
- executable WAT backend slices,
- regex / chibalex / chibacc reference behavior,
- dense Rust tests that behave like executable specs.

## Current Progress

Recent committed level-1r slices:

- `2f3b506 level-1r: run repeated pipe placeholder through operator wat`
- `77ecedb level-1r: run source pipe through executable wat`
- `b734cc5 level-1r: expose non exhaustive source match diagnostics`
- `3864797 level-1r: show continuation storage in cli visual`
- `fc3e8a6 level-1r: expose callable storage in visual reports`

Current uncommitted slice:

- Adds Core `TailCallResult { binder }` so nested call results are explicit Core facts instead of backend string guesses.
- Extends backend RenderEnv with locals and emits a sequential tailcall-result WAT chain for multi-call expressions.
- Adds source-level executable WAT coverage for `-`, `*`, `/` through nested calls.
- Keeps single tailcall output shape intact for existing direct-call tests.

Validated during the current slice:

```sh
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline -- --nocapture
cargo test --manifest-path level-1r/Cargo.toml --test backend_emit_baseline -- --nocapture
git diff --check
```

`program_pipeline_baseline` passed 71/71. `backend_emit_baseline` passed 24/24 after the latest chain-renderer comment fix. Re-run before committing if the working tree changed.

## Remaining Direction

The next useful work is not more checkpoint vocabulary. Continue with Rust baseline compiler slices that make real source programs execute through:

```text
source -> typed AST -> CPS -> Core -> backend WAT -> runtime
```

High-value next slices:

- make nested call/result lowering more explicit and less ad hoc,
- expand arithmetic/operator runtime coverage beyond i64 baseline,
- deepen branch/match + call chains through executable WAT,
- add callable/closure/continuation runtime slices,
- continue replacing narrow render-only behavior with explicit Core facts.
