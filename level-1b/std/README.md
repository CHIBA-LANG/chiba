# level-1b std

`std` is the ordinary user-visible standard library. It may depend on
`metalstd`, but it must not leak Metal details through public APIs.

Managed storage follows the level-1b Wasm-GC contract:

- `Array[T]` is an owned immutable Wasm-GC array.
- `Slice[T]` is a managed view `{backing: Array[T], offset, len}`.
- `Vec[T]` is a builder over Wasm-GC managed backing storage and freezes to
  `Array[T]`.
- `String == Array[u8]`.
- `str == Slice[u8]`.

String indexing and slicing are byte/slice operations. Character inspection is
explicit through `.char_at(index)`.
