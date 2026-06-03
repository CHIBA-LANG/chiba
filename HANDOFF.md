# Handoff — Level-1R Rust Baseline Compiler

Date: 2026-06-03

This handoff is intentionally short. The old 2026-06-01 level-1b checkpoint log was removed; use git history if needed.

## Current Objective

Build the `level-1r` Rust reference compiler with dense tests, visible nanopass/debug reports, and executable WAT slices. The Rust baseline should become the implementation reference before porting back to Chiba self-bootstrap.

## Current Branch / Git State

Branch: `level-1r`

Latest committed work:

```text
2f3b506 level-1r: run repeated pipe placeholder through operator wat
77ecedb level-1r: run source pipe through executable wat
b734cc5 level-1r: expose non exhaustive source match diagnostics
3864797 level-1r: show continuation storage in cli visual
fc3e8a6 level-1r: expose callable storage in visual reports
```

Working tree currently has uncommitted implementation work in:

```text
level-1r/src/backend/bir/emit.rs
level-1r/src/backend/cir/core.rs
level-1r/tests/backend_emit_baseline.rs
level-1r/tests/program_pipeline_baseline.rs
```

Do not discard those changes. They are the active arithmetic/nested-call executable WAT slice.

## Active Uncommitted Slice

Purpose:

- Support nested call results without backend guessing `w0` / `w1` string shapes.
- Carry call-result binding as explicit Core fact:

```rust
CoreOp::TailCallResult { binder }
```

- Let backend preflight and WAT emission consume that Core fact.
- Emit sequential WAT locals for multi-step call-result chains.
- Prove source-level `-`, `*`, `/` can compile through nested calls and run in WAT.

Important behavior:

- Single tailcall rendering remains on the existing path, preserving old direct-call tests and `;; tailcall ...` comments.
- Multi-call chains use `;; tailcall-result-chain` and local bindings.
- This follows `AGENTS.md`: no backend semantic inference from string shape.

## Validation Already Run

These passed after the latest implementation changes:

```sh
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline -- --nocapture
cargo test --manifest-path level-1r/Cargo.toml --test backend_emit_baseline -- --nocapture
git diff --check
```

Observed results:

```text
program_pipeline_baseline: 71 passed
backend_emit_baseline: 24 passed
```

Also scanned touched files for obvious forbidden shortcuts:

```sh
rg -n "starts_with\\(|is_ascii|TODO|gate|stub|fake" \
  level-1r/src/backend/bir/emit.rs \
  level-1r/src/backend/cir/core.rs \
  level-1r/tests/backend_emit_baseline.rs \
  level-1r/tests/program_pipeline_baseline.rs
```

Only test names / harmless WAT-prefix assertions were reported. No backend `operator::` string-shape semantic inference remains in this slice.

## Before Commit

Run once more if any file changed:

```sh
cargo fmt --manifest-path level-1r/Cargo.toml
cargo test --manifest-path level-1r/Cargo.toml --test backend_emit_baseline -- --nocapture
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline -- --nocapture
git diff --check
```

Suggested commit message:

```text
level-1r: run nested arithmetic calls through wat
```

## Next Work

Continue with real vertical slices, not more gates.

Good next slices:

- make `TailCallResult` / call-result chain lowering cleaner in Core and visual reports,
- add executable WAT tests for branch or match arms that call functions,
- expand operator intrinsic facts beyond the current i64 arithmetic baseline,
- add source-level closure/callable storage runtime tests,
- add continuation runtime tests only when they execute real Rust reference lowering, not a placeholder.

## Do Not Do

- Do not reintroduce `ExprI32Add`, `TypedExprTailCallI32Const`, `CoreExprParam0`, or similar narrow fake nodes.
- Do not make backend infer semantic meaning from string prefixes/suffixes.
- Do not add checkpoint gates to hide missing implementation.
- Do not treat old level-1b generated-parser WAT success as self-bootstrap completion.
