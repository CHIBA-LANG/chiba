# Chiba Conformance Fixture Index

This directory is a source-level conformance seed for future bootstrap compiler evaluation.

Each `.chiba` file is intentionally self-describing:

- `// expect: accept` means the source should parse and pass the relevant semantic checks.
- `// expect: reject ...` means the compiler should reject it with a stable diagnostic category.
- `// expect: warning ...` means the compiler should accept it while reporting a stable warning category.
- `// expect: runtime ...` means an executable backend should produce that observable result.
- `// expect: fact ...` means typed/lowered/visual facts should contain that language-level fact.
- `// expect: mixed-outcomes` means the file intentionally contains independent accept/reject/warning snippets.
- `// backend-matrix: wasm-gc, wasm32-nogc` means the current compiler test runner
  must compile this whole fixture for each listed backend target. Fixtures
  without this line remain lifecycle conformance material and are not forced
  through the current implementation yet.

No separate oracle files are provided yet. The expected behavior is written next to the source so the first runner can decide how to materialize parser, typed, runtime, and lowering checks.

The fixtures are grouped by language feature area, not by current implementation pass.
