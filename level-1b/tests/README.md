# level-1b tests

Tests in this directory cover the actual Second Bootstrap tree. Transitional
fixtures in `level-1b/supports` may remain while C00-C03 are built.

Required groups:

- CLI surface and diagnostics.
- Source loading, doc comments, and namespace surface.
- `compile_if` target/backend filtering.
- Direct wasmtime execution of `chibac.wasm`.
- Wasm-GC managed layout behavior for arrays, slices, strings, closures, and
  continuation packages.
