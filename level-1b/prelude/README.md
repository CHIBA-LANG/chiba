# level-1b prelude

`prelude` is the default import layer for non-Metal source files. It depends on
`std` and never directly on `metalstd`.

It re-exports common types and functions such as `Option`, `Result`, `Array`,
`Slice`, `String`, `str`, `Vec`, `Map`, `Range`, `Some`, `None`, `Ok`, `Err`,
`print`, `println`, `panic`, `assert`, and pipe-friendly sequence helpers.

`#![no_prelude_import]` disables this default import. `#![Metal]` files do not
implicitly import the prelude.
