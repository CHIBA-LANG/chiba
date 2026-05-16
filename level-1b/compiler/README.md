# level-1b compiler

This directory contains the level-1b rewrite of `chibac`.

The compiler is organized as small passes. Each pass owns one semantic change:
source loading, CLI parsing, doc comment capture, namespace surface scan, alpha
conversion, pattern elaboration, type checking, answer/control checking, usage,
CPS, closure conversion, Wasm-GC Core lowering, validation, and backend emit are
separate modules.

`chibac.wasm` must be runnable directly with wasmtime. Development harnesses may
wrap the same artifact, but they are not part of the compiler runtime contract.
