# Wasm-GC Bootstrap Layout Contract

This document fixes the first-hop layout vocabulary. It is a lowering contract
for the level-0-to-level-1 bootstrap emitter, not the final optimized level-1
runtime model.

## Layout Ids

Every managed layout must receive a stable layout id and be visible in Core/IR
dumps. Allocation and field access in the bootstrap emitter must refer to layout
ids, not hidden `i64` pointer conventions.

Layout id construction is deterministic:

- Field ids are canonical and sorted by stable field id.
- Nominal ids are allocated from the whole-project stable symbol table.
- Tuple field ids are compiler-generated as `_1`, `_2`, ...
- Variant tags follow source/canonical variant order.

## Scalar Values

The first bootstrap may represent these as Wasm immediates:

- `i64`, `i32`, supported unsigned widths, `bool`, atom, tag, and unit sentinel.

Managed records, tuples, data values, strings, slices, closures, and
continuation packages are Wasm-GC heap references.

## Row-Backed `type`

`type T { ... }` lowers to a nominal row object layout:

- The row shape is canonical.
- Field source order does not affect the row key.
- Nominal identity remains part of the layout identity.
- The first bootstrap uses heap structs only. It does not unbox records.

## Tuple

Tuple values lower as anonymous positional rows:

- `(A, B)` uses fields `_1: A`, `_2: B`.
- `(A,)` is distinct from grouped expression `(A)`.
- `()` is unit and has no tuple layout.
- Tuple literals, destructuring, and tuple patterns must share the same
  anonymous layout key derived from element type sequence.

## Data

`data` lowers to a heap object with:

- Field 0: stable variant tag.
- Field 1: payload carrier.

The carrier is a row-like union payload layout built from all variant payload
layouts. No-payload variants use a unit/null sentinel payload. This contract is
only a bootstrap representation; it does not define `data` as a language-level
two-tuple.

## Union

Bootstrap `union` is a heap row object used as a payload/row carrier. It is not
a C overlapping-memory union. Field access/update shares row-field lowering with
record/type access.

Long-term surface `union` remains restricted to `#![Metal]` low-level layout
work; this bootstrap carrier is an internal lowering device.

## Array, String, `str`, and `cstr`

`Array[T]` is owned managed storage. `String` is the UTF-8/WTF-8 byte-specialized
owned array:

```text
String == Array[u8]
```

`str` is the corresponding borrowed/read-only view:

```text
str == Slice[u8]
```

`Slice[T]` keeps the backing array alive and carries a stable range into it:

- `arrayref`: Wasm-GC backing array reference
- `offset`: `i32` element offset
- `len`: `i32` element count

Bootstrap byte-string layouts therefore use an owned array plus a view object:

```wat
(type $array_u8 (array (mut i8)))
(type $slice_u8
  (struct
    (field (ref $array_u8))
    (field i32)
    (field i32)))
```

String literals produce `$array_u8` values, and interpolation concatenates
existing `$array_u8` parts into a new `$array_u8`. The bootstrap array type is
mutable so builders and concat helpers can fill fresh arrays; ordinary `String`
surface operations still expose byte-array semantics, not arbitrary mutation.
Index and range operations produce or consume `$slice_u8` view values through
bootstrap helper functions that explicitly trap on negative indexes, out of
range byte access, reversed ranges, and ranges past the backing array length.
`String[index]` follows `Slice[u8]` byte-index semantics. Character/codepoint
access is not implicit indexing; use an explicit method such as `.char_at(n)`;
the bootstrap codepoint helper also traps before reading outside the backing
array. `cstr` is an ABI boundary view and is not the ordinary managed string
representation.

File read, stdout/stderr write, lexer input, parser source span, and WASI
boundary helpers must use this same string/slice contract.

## Closure, Function, and Continuation

Top-level functions lower to direct function symbols. Closures lower to:

```text
{ funref, env }
```

Administrative continuations produced by one-pass CPS should be beta-reduced
when possible and should not allocate runtime objects. User-captured
continuations materialize only when usage analysis requires it.

Materialized continuation packages are split by kind:

- Linear package: exactly one legal resume.
- Multi-resume package: reusable only after answer type, replay-safety, and
  usage checks.

Continuation packages must carry enough frame/world facts for the Core
validator to reject cross-world or cross-thread capture/resume before emit.
