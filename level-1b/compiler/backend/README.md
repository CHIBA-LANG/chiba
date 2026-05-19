# level-1b Wasm-GC backend

The backend starts from `OptimizedClosureModule`. It does not own source
semantics, type checking, continuation legality, or closure decisions.

The pass split is deliberately small:

- `layout.chiba` builds a stable Wasm-GC layout table.
- `core.chiba` lowers optimized closure Core to backend-neutral Wasm-GC Core.
- `validate_core.chiba` rejects dangling layouts/symbols, illegal tailcalls,
  and illegal continuation packages before emit.
- `wat_emit.chiba` serializes validated Core as WAT without semantic choices.

The backend stops at WAT. Binaryen v129 is the downstream build toolchain:
`wasm-as` assembles WAT to WASM, `wasm-opt` validates and optimizes,
`wasm-dis` roundtrips for debugging, and tools such as `wasm-merge`,
`wasm-reduce`, `wasm-shell`, and `wasm-ctor-eval` may support tests and
debugging. The Chiba backend owns the WAT contract; binary emission and
optimization are downstream toolchain work. During tests, Node plus the
`binaryen.js` binding may exercise the same WAT path.
