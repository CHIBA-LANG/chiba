# level-1b prelude

`prelude` is the default import layer for non-Metal source files. It depends on
`std` and never directly on `metalstd`.

It re-exports common types and functions such as `Option`, `Result`, `Array`,
`Slice`, `String`, `str`, `Vec`, `Map`, `Range`, `Some`, `None`, `Ok`, `Err`,
`print`, `println`, and `panic`.

Sequence operations are methods on their receiver types, for example
`xs.map(f).filter(p).fold(init, step)`. Pipe-friendly free functions may live in
their owning std modules, such as `list_map`, but prelude does not add naked
`map` / `filter` / `fold` names.

`#![no_prelude_import]` disables this default import. `#![Metal]` files do not
implicitly import the prelude.
