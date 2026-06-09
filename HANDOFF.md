# Handoff — Level-1R Rust Baseline Compiler

Date: 2026-06-09

## Objective

Continue the `level-1r` Rust reference compiler toward P0: real source programs should pass through parser, typed/pattern/control/CPS/Core, backend WAT, and runtime execution with visible diagnostics. The current handoff is for an unfinished ADT payload pattern dispatch slice that is blocked by one `ContN` backend case.

## Git State

Branch: `level-1r`

Latest good commits:

```text
2884218 level-1r: unify dyn row backend member abi
966df94 level-1r: unify row callable member records
f8717ff level-1r: preserve dyn row method receiver identity
5b0e556 level-1r: lower zero-arg adt parameter patterns
51ede47 level-1r: lower tuple match patterns
```

Current dirty files:

```text
M AGENTS.md                         # user-owned; do not touch/stage/revert
M level-1r/src/backend/bir/emit.rs
M level-1r/src/backend/cir/core.rs
M level-1r/src/semantic/pattern.rs
M level-1r/src/semantic/typed.rs
M level-1r/src/tooling/debug.rs
M level-1r/src/tooling/pipeline.rs
M level-1r/tests/frontend_pipeline_baseline.rs
M level-1r/tests/program_pipeline_baseline.rs
?? wasm32_target_nogc.TODO.md        # unrelated; ignore unless asked
```

Do not commit yet. A focused test is still failing.

## Active Slice

Purpose: support function parameter pattern clauses and ADT payload/nested payload backend execution.

New/changed tests in `level-1r/tests/program_pipeline_baseline.rs` include:

```text
source_pattern_clause_defs_dispatch_payload_adt_parameters_to_executable_wat
source_pattern_clause_defs_dispatch_nested_payload_adt_parameters_to_executable_wat
program_contn_repeated_resume_replays_captured_adt_match_context  # currently failing
```

Important implementation changes already in the dirty tree:

- `CoreOp::Match.patterns` now stores `Vec<CorePattern>`, not debug strings.
- Frontend/debug tests were updated for structured match patterns.
- Constructor payload typing now handles nested payload binders.
- Pattern coverage handles tuple/product and constructor payloads recursively.
- Backend ADT ABI now carries payload lanes and nested payload ABI.
- Pipeline builds nested ADT payload lanes with a depth cap of 4.
- Backend rendering can register ADT payload lanes and render source pattern-clause dispatch for payload ADTs.
- Captured continuation support was partially extended with `RenderEnv::with_captured_continuation_arg_abi`.

## Current Failure

Command:

```sh
cargo test --manifest-path level-1r/Cargo.toml \
  --test program_pipeline_baseline \
  program_contn_repeated_resume_replays_captured_adt_match_context \
  -- --nocapture
```

Failure:

```text
assertion failed: bundle.backend_link.diagnostics.is_empty()
```

Visual repro:

```sh
cat >/tmp/chiba-contn-p0.chiba <<'EOF'
data Option[T] = { Some(T), None }
def main() = resetn { match shift retry { retry(Option.Some(7)) + retry(Option.None) } { Option.Some(value) => value, Option.None => 0 } }
EOF

cargo run --manifest-path level-1r/Cargo.toml -- --visual /tmp/chiba-contn-p0.chiba |
  rg -n "backend:|diagnostic|return-match|resume0|capture-continuation|wat-lines" -C 4
```

Current backend diagnostic:

```text
diagnostic unsupported i32 return value resume0
```

Core shape:

```text
op 1 capture-continuation retry kind=contn param=resume0
  ops=[return-match scrutinee=resume0 arms=[Option.Some(value) => value, Option.None() => 0]]
```

## What Was Tried

In `level-1r/src/backend/bir/emit.rs`:

- Added/adjusted `RenderEnv::with_captured_continuation_arg_abi`.
- It now inserts the captured param binding and registers ADT payload lanes from the resume argument.
- Added helpers around ADT tag/payload facts:
  - `adt_tag_abi_from_value`
  - `backend_value_kind_for_core_value`
  - `bind_adt_payload_lanes_from_value`
  - `adt_pattern_tag_is_renderable`
  - `render_adt_pattern_tag_i32_indented`
- Removed the dead `continuation_kind_for_binder`.
- Changed `constructor_runtime_match_env` to avoid immediately resolving the runtime scrutinee to a single concrete resume argument.

These changes compile, but the focused `ContN` test still fails with the same diagnostic. The next agent should inspect the preflight path first, especially:

```text
unsupported_runtime_return_value
unsupported_captured_continuation_runtime_value
unsupported_i32_match
unsupported_i32_match_arms
constructor_runtime_match_env
constructor_match_payload_values
core_value_is_renderable_i32
render_pattern_condition_i32
```

Likely issue: the backend preflight still treats `resume0` as an ordinary i32 renderable value somewhere instead of recognizing the structured ADT tag parameter plus payload lanes. Another possible issue is that `resolve_core_value_binding` collapses the captured parameter to the first concrete resume argument too early for a multi-shot continuation.

## Constraints For The Fix

Do not use string-shape inference. In particular, do not special-case:

```text
resume0
Option
Some
None
_payload_
lane name prefixes/suffixes
```

The fix must consume structured facts:

```text
CoreValue::Adt
CorePattern
BackendAdtTagAbi
BackendAdtPayloadAbi
RenderEnv locals/bindings/adt_tag_params
```

For `ContN`, do not let one concrete resume argument permanently specialize the captured continuation body. The captured body must remain valid for all resume calls represented by the continuation ABI.

## Verification Before Commit

Run:

```sh
cargo check --manifest-path level-1r/Cargo.toml
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline program_contn_repeated_resume_replays_captured_adt_match_context -- --nocapture
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline source_pattern_clause_defs_dispatch_nested_payload_adt_parameters_to_executable_wat -- --nocapture
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline program_adt_payload_literal_pattern_checks_payload_before_arm_body -- --nocapture
cargo fmt --manifest-path level-1r/Cargo.toml --check
cargo test --manifest-path level-1r/Cargo.toml
```

Only after green, stage the relevant `level-1r` files. Do not stage `AGENTS.md` or `wasm32_target_nogc.TODO.md`.

Suggested commit:

```text
level-1r: lower ADT payload pattern dispatch
```

## Next P0 Work After Commit

After this slice is committed, continue with unified member and row behavior:

- `[T:{r| f: (x) -> y}]` accepts ordinary type `X` with method `f` but no field `f`;
- `dyn {f: (x) -> y}` works through field and method adapters;
- ordinary type `Y` with field `f` works when `f` is function/closure/cont1/contn;
- dependent `[F, T:{r| f: F}]` works;
- negative tests reject non-callable field `f` and method `f` with mismatched signature;
- field access wins over same-named receiver method;
- callable/closure/continuation values stored then called are tested, including captures;
- `Cont1` multiple-use should be compiler error where statically known, or boxed one-shot runtime trap only for escaped storage.

P0 is not done until these are executable or have explicit compiler diagnostics, not just visual gates.
