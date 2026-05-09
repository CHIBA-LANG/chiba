# Level-1 First Bootstrap Subset

This document freezes the source subset that the level-0 seed must accept while
building the first `level1c.wasm`. It is intentionally smaller than the long
term level-1 language, but it keeps delimited continuations as a day-0 feature.

## Scope

The first bootstrap compiler is a single-threaded, single-module program. It can
read source files, run the generated level-1 lexer/parser, perform stubbed or
minimal semantic checks, run continuation smoke tests, and emit a minimal
Wasm-GC/WASI target for the skeleton compiler.

The subset is allowed to use:

- `namespace`, `use`, `private`, and item attributes needed by existing source.
- Top-level `def`, local `let`, block expressions, `return`, `if`, `else`,
  `match`, `for`, `break`, and `continue`.
- `i64`, `i32`, unsigned integer widths when already represented by the
  frontend, `bool`, `()`, atoms/tags, `Str`, `String`, `cstr`, slices, tuples,
  row-backed `type`, `data`, and bootstrap-only row carrier `union`.
- Ordinary closures, no-capture lambdas, direct calls, unknown closure calls,
  and tail calls.
- `reset` / `shift` or equivalent primitives, including single-resume and
  multi-resume continuations.
- `extern "wasi" "symbol"` declarations with checked signatures.
- Bootstrap `#![Metal]` modules only for runtime/ABI glue that genuinely needs
  low-level representation.

The subset does not require level-0 to implement:

- Parallel project scheduling, worker pools, concurrent caches, or concurrent
  specialization registries.
- Namespace object output or `wasm-ld` linking.
- Full checked monomorphization infrastructure.
- Dynamic packaging, complex operator overload resolution, global method
  search, answer type polymorphism, or incremental cache invalidation.
- Long-term arena/RC optimization. The first Wasm lowering may use simple
  Wasm-GC heap objects for all managed aggregates.

## Continuation Rules

The bootstrap subset must preserve the language-level continuation contract:

- Every function and closure body has an implicit reset boundary.
- Explicit `reset` introduces a local answer type.
- `shift` captures only up to the nearest compatible reset.
- Captured continuations are not ordinary closures.
- Cross-world and cross-thread capture or resume is illegal.
- Multi-resume continuation use must be classified before lowering. The first
  bootstrap may use a conservative replay-safety checker, but it must reject
  obviously non-replay-safe captures such as FFI resources, world-local values,
  `UnsafeRef`, and Atomic state.

Smoke tests required before B06:

- Simple `reset` / `shift`.
- Nested `reset`.
- Single-resume continuation.
- Multi-resume parser/backtracking continuation.
- Answer type mismatch.
- Cross-world or cross-thread continuation error.

## Checked Generic Boundary

The first bootstrap compiler skeleton should avoid generic-heavy implementation
code. This does not change level-1 generics semantics. Generic definitions are
checked structural templates in the long-term compiler: definition-time checking
produces obligations, and instantiation only discharges concrete shape, method,
operator, dispatch, and continuation facts.

For the first bootstrap, source that would require specialization keys,
InstantiationRegistry, generic SCC scheduling, or cross-namespace
deduplication is outside the seed subset.

## Wasm Target Boundary

The first emitter may be dumb. It serializes already-accepted bootstrap IR to a
single `.wat` / `.wasm` module with:

- Functions, locals, integer operations, comparisons, blocks, loops, branches,
  calls, returns, and tail calls.
- Typed WASI imports.
- Simple heap allocation for managed records, tuples, data payloads, strings,
  slices, closures, and materialized continuation packages.
- Stable dump names for layout ids and continuation frame/package ids.

Any unsupported node must fail with an explicit bootstrap target error rather
than falling back to opaque `i64` pointer behavior.
