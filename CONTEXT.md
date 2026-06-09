# Chiba Level-1 Context

Date: 2026-06-09

This repository is on the `level-1r` Rust reference compiler route. The practical objective is still: make real source programs pass through visible nanopasses and execute as WAT, then use the proven structure for Chiba self-bootstrap.

## Current Source Of Truth

- `AGENTS.md`: highest-priority semantic and engineering rules.
- `level-1r.md`: Rust reference compiler scope and design constraints.
- `HANDOFF.md`: current working state for the next agent.
- Spec dirs:
  - `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec`
  - `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/type_system`

## Hard Rules

- Do not infer semantics from string shape outside lexer / chibalex / regex tokenization.
- Identifier legality must come from UTF-8/XID lexer policy or a shared XID table.
- Semantic, pipeline, backend, and lowering passes consume AST, symbol, typed, Core/CIR, ABI, or explicit facts.
- Keep Core/CIR target-neutral. WAT/Wasm-GC/Binaryen details belong below Core/CIR.
- Do not use gates, fake nodes, or narrow string-name shortcuts to hide missing implementation.
- Do not touch, stage, or revert user-owned `AGENTS.md` changes unless explicitly asked.
- Commit coherent tested slices, but do not commit the current slice until the focused failing test is green.

## Recent Good Commits

Latest committed good state before the current dirty slice:

```text
2884218 level-1r: unify dyn row backend member abi
966df94 level-1r: unify row callable member records
f8717ff level-1r: preserve dyn row method receiver identity
5b0e556 level-1r: lower zero-arg adt parameter patterns
51ede47 level-1r: lower tuple match patterns
```

At commit time, the recent row/member slices had full `cargo test --manifest-path level-1r/Cargo.toml` passing.

## Current Dirty Slice

The active uncommitted work is an ADT payload pattern dispatch slice:

- `CoreOp::Match.patterns` moved from `Vec<String>` to `Vec<CorePattern>`.
- Debug rendering was updated to render structured `CorePattern`.
- Typed/pattern code now propagates constructor payload binder types, including nested payloads like `Some(Some(x))`.
- Pattern coverage now handles tuple/product and constructor payload coverage recursively.
- Backend ABI now carries ADT tag payload lanes:
  - `BackendCallableArgExpansion::AdtTag { payloads }`
  - `BackendAdtTagAbi { payloads }`
  - `BackendAdtPayloadAbi { variant, index, kind, nested }`
- Pipeline builds nested ADT payload lanes with a depth cap of 4.
- Backend render registers ADT payload lanes and can render payload-pattern match dispatch for source pattern clause definitions.

Touched implementation files:

```text
level-1r/src/backend/bir/emit.rs
level-1r/src/backend/cir/core.rs
level-1r/src/semantic/pattern.rs
level-1r/src/semantic/typed.rs
level-1r/src/tooling/debug.rs
level-1r/src/tooling/pipeline.rs
```

Touched tests:

```text
level-1r/tests/frontend_pipeline_baseline.rs
level-1r/tests/program_pipeline_baseline.rs
```

Unrelated dirty state:

```text
AGENTS.md
wasm32_target_nogc.TODO.md
```

Leave those alone unless the user explicitly asks.

## Current Blocker

The active focused failure is:

```sh
cargo test --manifest-path level-1r/Cargo.toml \
  --test program_pipeline_baseline \
  program_contn_repeated_resume_replays_captured_adt_match_context \
  -- --nocapture
```

It fails because `bundle.backend_link.diagnostics` is not empty.

Visual repro:

```sh
cat >/tmp/chiba-contn-p0.chiba <<'EOF'
data Option[T] = { Some(T), None }
def main() = resetn { match shift retry { retry(Option.Some(7)) + retry(Option.None) } { Option.Some(value) => value, Option.None => 0 } }
EOF

cargo run --manifest-path level-1r/Cargo.toml -- --visual /tmp/chiba-contn-p0.chiba |
  rg -n "backend:|diagnostic|return-match|resume0|capture-continuation|wat-lines" -C 4
```

Current diagnostic:

```text
diagnostic unsupported i32 return value resume0
```

Relevant Core shape:

```text
capture-continuation retry kind=contn param=resume0
  ops=[return-match scrutinee=resume0 arms=[Option.Some(value) => value, Option.None() => 0]]
```

## Important Current Hypothesis

The failure is in backend preflight/render fact propagation for captured continuation parameters, not in parser or typed pattern elaboration.

Do not solve it by parsing `resume0`, lane names, constructor names, or any string shape. The correct fix must consume:

- `CoreValue::Adt`
- `CorePattern`
- `BackendAdtTagAbi`
- `BackendAdtPayloadAbi`
- `RenderEnv` bindings/locals/ADT ABI facts

The risky area is the interaction between:

- `RenderEnv::with_captured_continuation_arg_abi`
- `unsupported_captured_continuation_runtime_value`
- `constructor_runtime_match_env`
- `constructor_match_payload_values`
- `render_pattern_condition_i32`
- `core_value_is_renderable_i32`

There is likely still a mismatch between treating `resume0` as a runtime ADT parameter and resolving it to one concrete resume argument during `ContN` replay checks.

## Validation State

Recently observed passing focused tests before the current blocker work:

```text
source_pattern_clause_defs_dispatch_nested_payload_adt_parameters_to_executable_wat
source_pattern_clause_defs_dispatch_payload_adt_parameters_to_executable_wat
program_adt_payload_literal_pattern_checks_payload_before_arm_body
adt_baseline
pattern_baseline
```

Current full test is not green because the `program_contn_repeated_resume_replays_captured_adt_match_context` focused test is red.

Before committing this slice, run:

```sh
cargo check --manifest-path level-1r/Cargo.toml
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline program_contn_repeated_resume_replays_captured_adt_match_context -- --nocapture
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline source_pattern_clause_defs_dispatch_nested_payload_adt_parameters_to_executable_wat -- --nocapture
cargo test --manifest-path level-1r/Cargo.toml --test program_pipeline_baseline program_adt_payload_literal_pattern_checks_payload_before_arm_body -- --nocapture
cargo fmt --manifest-path level-1r/Cargo.toml --check
cargo test --manifest-path level-1r/Cargo.toml
```

Suggested commit message after green:

```text
level-1r: lower ADT payload pattern dispatch
```

## P0 Direction After This Slice

After the ADT/ContN blocker is fixed and committed, the next high-value P0 work is unified member/row behavior:

- field callable and receiver method both satisfy `[T:{r| f: F}]`;
- dependent `[F, T:{r| f: F}]` works;
- `dyn {f: ...}` can package fields and method adapters;
- field wins over same-named method;
- negative cases reject non-callable fields and mismatched method signatures;
- callable storage variants function/closure/cont1/contn are tested after storage then call, including captures.
