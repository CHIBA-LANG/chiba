# B02 Level-1 Nanopass CIR Entry

Correction: level-0 is only the seed and reference implementation. Long-term
backend work lives in `src/`, not in `level0/`.

The level-1 compiler owns a nanopass-style CIR. CIR is a single large ADT with
stage-prefixed node families instead of a chain of unrelated IR containers.

Examples:

- `L0OpIf(...)`: surface/control-preserving node before alpha conversion.
- `L1OpIf(...)`: alpha-converted node with binder/name ids resolved.
- Future `L2*`, `L3*`, ... families can represent typed, effect-checked,
  answer-checked, CPS, closure-converted, and Wasm-GC Core stages.

## Direction

The first level-1 path is:

```text
generated parser AST
  -> L0 CIR surface core
  -> L1 CIR alpha core
  -> later typed/effect/answer/usage/CPS/closure/Core nanopasses
  -> Wasm-GC wat/object
```

The parser AST is an input format, not the compiler's semantic IR.

## B02 Acceptance

B02 is done only when `src/backend/cir` contains:

- A CIR ADT with stage-prefixed node families.
- A pass context for stable binder ids.
- An alpha-conversion pass skeleton.
- Explicit unsupported nodes for AST/CIR surface not yet lowered.

The first implementation may be partial, but unsupported nodes must be visible
and testable. They must not silently become opaque `i64` pointers.

## Verification

The current `src/backend/cir` skeleton has been checked with the level-0 seed:

```sh
timeout 120 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --entry chiba_level1_lexer_spec_main.chiba --output lexer_spec_runner.o
timeout 120 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --entry chiba_level1_parser_spec_main.chiba --output parser_spec_runner.o
```

Both commands complete. This verifies scanning, resolving, type-checking,
lowering, BIR validation, native assembly, and binary output for the current
level-1 source tree under the seed compiler.

## Level-0 Boundary

Do not add the long-term Wasm backend to `level0/` while level-0 can still move
the seed path forward. Level-0 code remains useful as a reference for frontend
quirks, one-pass CPS, usage, escape, closure conversion, and runtime behavior,
but level-1 backend architecture should grow under `src/`.
