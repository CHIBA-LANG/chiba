# Semantic Gates Before Second Bootstrap

These fixtures pin down level-1 semantic obligations that must not regress while
the full HM/row/answer-type pipeline is still being rewritten in level-1.

`vp run semantic:gates` checks:

- method call routing order: field callable, receiver method, qualified callee
- row polymorphism surface: one row bound, canonical field order, field access
- row identity: returning the full row-polymorphic value
- namespace merge: two files contributing to one namespace and a third consumer,
  including a real level-0 project compile smoke
- memory capabilities: `Ref[T]`, `UnsafeRef[T]`, `Ptr[T]`, `Atomic[T]`, `:=`
- delimited continuation multi-entry: classic multi-shot shift/reset shape
- string, interpolation, byte-index, explicit `char_at`, and slice WAT lowering holes

The gates first require every fixture to parse with the level-1 parser. They then
apply spec-level checks that are stricter than the current bootstrap backend, so
Second Bootstrap work has executable targets before the complete typed pipeline
exists.

Pre-C06 WAT emission is limited to fixtures that already lower to L8 validated
Core without semantic fallback or env holes. Those WAT files are written under
`.scratch/semantic-gates/wat/`. String and slice fixtures assert that the
bootstrap Wasm-GC backend emits managed layouts: `String` as `$array_u8` and
`str`/slice as a `$slice_u8` view over that backing array, rather than treating
them as opaque integers. `String[index]` follows `Slice[u8]` indexing semantics;
character access must be explicit through `.char_at(n)`.
