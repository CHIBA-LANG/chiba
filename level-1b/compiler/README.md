# level-1b compiler

This directory contains the level-1b rewrite of `chibac`.

The compiler is organized by the same reading order as the eventual compiler
pipeline, not by temporary bootstrap history:

- `cli/` owns command-line arguments and target/backend selection.
- `source/` owns source loading, include roots, doc comments, namespace scan,
  and `compile_if` filtering.
- `ir/` owns pass boundary ADTs only. It must not contain pass logic.
- `lower/` owns AST-to-surface lowering.
- `semantic/` owns alpha conversion, pattern elaboration, HM + row typing,
  checked templates, nominal method/operator resolution, and capability checks.
- `control/` owns answer/control checking, continuation usage, replay safety,
  and one-pass CPS.
- `closure/` owns continuation simplification, closure conversion, lambda
  lifting, and environment shrinking.
- `backend/` owns Wasm-GC layout/Core validation and WAT serialization.
- `driver/` owns pass orchestration only.
- `diagnostic/` owns user-visible diagnostic shape and stable ordering.

Passes stay small, but files should read in pipeline order. When a pass grows
large, split it under its owning pipeline stage instead of adding another
top-level peer directory.

`chibac.wasm` must be runnable directly with wasmtime. Development harnesses may
wrap the same artifact, but they are not part of the compiler runtime contract.
