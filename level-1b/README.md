# level-1b Bootstrap Workspace

`level-1b` is the Second Bootstrap implementation workspace. It is a clean
level-1 source tree for the replacement `chibac` compiler, not a copy of the
current `src/` compiler stack. The current `src/`, level-0 compiler, native
chibalex, and native chibacc are oracle inputs while this tree is rewritten.

The final level-1b artifact is `chibac.wasm`. Users must be able to run that
compiler directly:

```sh
wasmtime chibac.wasm -- input.chiba -I std -I prelude --target wasm32-unknown-wasi --backend wasm-gc -o out.wasm
```

Node runners and Binaryen wrappers are developer and CI conveniences only. They
may compile WAT, run golden tests, or provide local harnesses, but level-1b
source and generated `chibac.wasm` must not depend on Node-only host imports.

## Source Layout

- `metalstd/`: `#![Metal]` modules for Wasm-GC intrinsics, WASI preview1 ABI,
  typed `Ptr[T]`, `UnsafeRef[T]`, `Atomic[T]`, traps, and linear-memory scratch
  only at host ABI boundaries.
- `std/`: ordinary Chiba standard library. `Array`, `Slice`, `Vec`, `String`,
  `str`, closures, continuation packages, and allocation use Wasm-GC managed
  object semantics.
- `prelude/`: default user imports. It depends on `std`, never directly on
  `metalstd`.
- `compiler/`: `chibac` CLI, source loading, diagnostics, namespace/project
  surface scan, semantic passes, CPS, Core, and backend.
- `tools/`: level-1b-authored build/test helpers that are intended to become
  compiler subcommands or internal utilities.
- `tests/`: level-1b fixtures and golden tests for CLI, source surface,
  wasmtime execution, and bootstrap validation.
- `supports/`: transitional fixtures driven by the repository's Node scripts.

## Coding Contract

- Public APIs, namespaces, pass entries, core ADTs, unsafe/Metal boundaries, and
  runtime ABI declarations use `///` doc comments. Block comments are not used.
- Non-Metal code does not introduce opaque pointer-shaped `i64` APIs or hidden
  internal mutability. Low-level state is exposed through typed capability
  surfaces and checked `unsafe` use.
- Chiba-flavored code is preferred: immutable values, pattern-first control
  flow, method calls, and pipe-friendly free functions. Optimization passes are
  responsible for directification, inlining, closure erasure, and allocation
  removal.
- Linear memory is not the ordinary Chiba heap. It is scratch for WASI preview1
  and host ABI exchange. Ordinary arrays, slices, strings, closure envs, and
  continuation packages are Wasm-GC managed objects.

## Current Smoke Contract

- `src/level1b_main.chiba` remains the fixed smoke entry while C00 is being
  established.
- `vp run level1b:smoke` compiles this project with the level-0 seed using
  `timeout 10`, emits WAT through the current level-1 path, and runs the result
  through the Node WAT harness.
- As C00 grows, the smoke target must be extended with a wasmtime-direct path
  for the same `chibac.wasm` CLI surface.
