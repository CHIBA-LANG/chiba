# Chiba Level-1 Context

Date: 2026-06-10

This repository is on the `level-1r` Rust reference compiler route. The practical objective is still to make real source programs pass through visible nanopasses and execute as WAT, then use the proven structure for Chiba self-bootstrap.

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
- Commit coherent tested slices after verification.

## Recent Good Commits

```text
f15c75c level-1r: expose program pass timings
381518c level-1r: preserve runtime local kinds in captured continuations
2884218 level-1r: unify dyn row backend member abi
966df94 level-1r: unify row callable member records
f8717ff level-1r: preserve dyn row method receiver identity
5b0e556 level-1r: lower zero-arg adt parameter patterns
51ede47 level-1r: lower tuple match patterns
```

The `381518c` slice fixed captured continuation runtime local kind propagation. It keeps `ContN` ADT match replay and `Cont1` captured Ref/UnsafeRef mutation replay executable.

The `f15c75c` slice made program-level pass timings visible in summaries and updated handoff/context to remove stale continuation blockers.

## Verified State

Last verified full command:

```sh
cargo test --manifest-path level-1r/Cargo.toml
```

Result observed on 2026-06-10:

```text
all tests passed
```

Important focused tests now pass:

```text
program_contn_repeated_resume_replays_captured_adt_match_context
program_cont1_replays_captured_ref_and_unsafe_ref_mutations_once
source_pattern_clause_defs_dispatch_payload_adt_parameters_to_executable_wat
source_pattern_clause_defs_dispatch_nested_payload_adt_parameters_to_executable_wat
```

## Current P0 Gaps

The old `ContN` ADT match blocker is closed. Remaining P0 work is not just more feature fixtures:

- Performance fix: program WAT tests use a persistent Node batch runner, and integration tests are grouped into one `tests/all.rs` harness instead of 32 separate Cargo test binaries.
- P0 remaining work now lives directly in `AGENTS.md` as Step 1 non-performance landing work and Step 2 compile acceleration / parallel / incremental work. `P0_AUDIT.md` was removed.
- Parallel/incremental story: Pass 00-22 describe namespace/body/specialization parallelism and cache behavior; the Rust reference has pieces, but production-grade scheduling/cache evidence is not complete.
- Self-bootstrap boundary: `level-1r` is a Rust reference compiler. If P0 includes Chiba self-hosting, that is not done.

## Performance Facts

Measured hot test timings:

```text
Before WAT batching:
  program_pipeline_baseline full suite: 327 passed, real 33.46s

After WAT batching:
  program_pipeline_baseline full suite: 327 passed, real 1.07s
  global_init_ subset: 34 passed, real 0.64s

After grouping integration tests into `--test all`:
  cargo test --manifest-path level-1r/Cargo.toml -- --list: real 4.79s
  cargo test --manifest-path level-1r/Cargo.toml --quiet: real 10.48s
  tests/all.rs internal run: 676 tests, finished in 1.81s
```

Interpretation: current hot full-suite cost is dominated by Cargo/libtest harness startup and doctest/bin/lib fixed overhead. The Chiba program pipeline itself is no longer the observed slow path.

## Next High-Value Work

1. Keep `AGENTS.md` Step 1 synchronized with actual remaining non-performance P0 work.
2. Finish Step 1 semantic / middle-end / backend evidence before treating compile acceleration as the main work.
3. Step 2 starts with stable pass/backend/runtime timing thresholds, then parallel/incremental cache work.
4. Continue real executable gaps only where the AGENTS Step 1 list shows missing evidence.
