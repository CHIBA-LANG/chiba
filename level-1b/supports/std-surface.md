# level-1b Standard Surface

This freezes the source-level standard surface allowed before Second Bootstrap.
Implementations may temporarily reuse level-0 stdlib code, but level-1b callers
must use this surface rather than raw pointer conventions.

## Ownership And Views

- `Array[T]`: owned immutable managed storage.
- `MutArray[T]`: owned mutable managed storage, restricted by mutability facts.
- `Slice[T]`: read-only view into `Array[T]` with `{backing, offset, len}`.
- `MutSlice[T]`: view into `MutArray[T]`, only available in checked mutable
  regions.
- `String == Array[u8]`: owned UTF-8/WTF-8 bytes.
- `str == Slice[u8]`: borrowed/read-only byte view.
- `cstr`: ABI-only NUL-terminated boundary view.

`String[index]` and `str[index]` use byte/slice semantics. Codepoint access must
be explicit through `.char_at(n)`.

## Required Modules

- `option`: `Option[T]`, `Some`, `None`, unwrap helpers only where diagnostics
  remain deterministic.
- `list`: persistent list basics for compiler IR and summaries.
- `array`: `Array[T]`, `MutArray[T]`, freeze/thaw, length, get, set for mutable.
- `slice`: range view, bounds check, length, byte/index access.
- `string`: literal, concat/interpolation builder, `.char_at`, compare/hash.
- `vec`: builder/growable storage implemented behind `#![Metal]` or checked
  mutable array wrappers.
- `map`: deterministic string/symbol map; iteration order must be stable.
- `file`: read file into `String`, write `String`/`str`.
- `process`: argv/env and process runner helpers.
- `print`: stdout/stderr helpers.
- `ref`: `Ref[T]`, `UnsafeRef[T]`, `Ptr[T]`, `Atomic[T]` capability surface.

## Metal Boundary

Only `#![Metal]` modules may expose or implement:

- raw pointer arithmetic
- raw `load*` / `store*`
- allocator internals
- C/WASI ABI layout shims
- `Ptr[T]` construction from integers

Non-Metal level-1b source may hold `Ref[T]`, `UnsafeRef[T]`, `Ptr[T]`, or
`Atomic[T]` only through typed APIs checked by the semantic pass.

## Pre-C01 Smoke Matrix

Executable fixtures live under `level-1b/supports/pre-c01-smokes`.

- string literal, interpolation, byte index, range slice, `.char_at`
- Array/Slice length and indexed access
- Vec builder append/read/freeze
- deterministic Map insert/get/iterate
- file read/write and stdout/stderr
- argv/env read
- Ref/UnsafeRef/Ptr/Atomic valid and invalid fixtures
