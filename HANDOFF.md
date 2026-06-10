# Handoff — Level-1R Rust Baseline Compiler

Date: 2026-06-10

## Objective

Continue `level-1r` toward P0 completion. Current status: core language/runtime slices are broadly green, including continuation, row/member, dyn row, string/rune, Ref/UnsafeRef, ADT payload patterns, callable storage, and executable WAT coverage. P0 is not complete because performance, auditability, and bootstrap/parallel-incremental evidence are not closed.

## Git State

Branch: `level-1r`

Latest relevant commits:

```text
f15c75c level-1r: expose program pass timings
381518c level-1r: preserve runtime local kinds in captured continuations
2884218 level-1r: unify dyn row backend member abi
966df94 level-1r: unify row callable member records
f8717ff level-1r: preserve dyn row method receiver identity
```

At the time of this handoff, `cargo test --manifest-path level-1r/Cargo.toml` was observed passing after `381518c`. Focused timing-summary and continuation regression tests were observed passing after `f15c75c`.

## Closed Blockers

These are no longer active blockers:

- `program_contn_repeated_resume_replays_captured_adt_match_context`
- `program_cont1_replays_captured_ref_and_unsafe_ref_mutations_once`

The fix was to preserve runtime local kinds inside captured continuation preflight/render paths, so `runtime-let` values such as Ref/UnsafeRef cells do not degrade to default i32 assumptions.

## Current Active Slice

The current work should focus on P0 auditability and performance:

- `P0_AUDIT.md` maps every `AGENTS.md` checkpoint item to evidence and remaining gaps;
- `level-1r/tests/spec_alignment.rs` should keep that audit from drifting when AGENTS changes;
- next implementation target should be profiling/optimizing repeated WAT runtime setup in `program_pipeline_baseline`;
- do not mark P0 complete until the Missing/Partial rows are actually closed with evidence.

## Known Performance Problem

Hot measurements:

```text
program_pipeline_baseline full suite: real 33.46s
global_init_ subset: real 21.41s for 34 tests
program_ subset: real 31.26s for 132 tests
source_ subset: real 34.58s for 130 tests
single executable WAT tests: roughly 0.4s-0.7s
```

The largest current P0 risk is not that a single nanopass is known to be slow. The risk is that the executable test path repeatedly performs full pipeline + backend/link + WAT runtime process/setup. A compiler intended to feel Go-fast cannot treat this as acceptable.

## Next Commands

After any timing-summary edit:

```sh
cargo fmt --manifest-path level-1r/Cargo.toml --check
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline program_summary_contains_program_level_nanopass_events -- --nocapture
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline program_contn_repeated_resume_replays_captured_adt_match_context -- --nocapture
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline program_cont1_replays_captured_ref_and_unsafe_ref_mutations_once -- --nocapture
```

After any P0 audit edit:

```sh
cargo test --manifest-path level-1r/Cargo.toml --test spec_alignment p0_audit_covers_every_agents_checkpoint_item -- --nocapture
```

Run full suite before committing behavior changes:

```sh
cargo test --manifest-path level-1r/Cargo.toml
```

## Do Not Do

- Do not reintroduce semantic string-shape inference in backend/lowering.
- Do not treat green full tests as proof of P0 completion without closing the `P0_AUDIT.md` Missing/Partial rows.
- Do not preserve obsolete blocker notes in handoff/context files.
- Do not optimize by weakening executable WAT coverage. The goal is faster evidence, not less evidence.

Suggested message for the audit slice:

```text
level-1r: add p0 audit gate
```
