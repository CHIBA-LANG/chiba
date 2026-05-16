# level-1b compiler driver

The driver connects CLI arguments to the compiler pass graph. C00 keeps the
driver single-threaded and deterministic. Long-term parallel pass boundaries are
recorded in `TODO.longterm.md`.

The driver owns target/backend selection for `wasm32-unknown-wasi` and
`wasm-gc`. Node, Binaryen, and local JavaScript harnesses are not target
backends.
