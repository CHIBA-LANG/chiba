# level-1b metalstd

`metalstd` is the only low-level library layer. Every Chiba source file in this
directory must be `#![Metal]`.

This layer owns Wasm-GC allocation intrinsics, WASI preview1 ABI shims, typed
pointer capabilities, atomic capabilities, traps, and linear-memory scratch used
only for host ABI boundaries. It must not provide high-level collections,
strings, parser helpers, regex helpers, or ordinary user IO facades.

Public APIs need `///` doc comments, and every unsafe boundary needs a
`/// Safety` section describing the caller obligations.
