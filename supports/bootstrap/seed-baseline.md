# Level-0 Seed Baseline

This file records the seed used for the first level-1 bootstrap work.

## Seed

- Compiler: `./chibac_amd64-unknown-linux_chiba_dev.o`
- Equivalent checked copy: `./level0/target/debug/main.o`
- Hash: `7a7744ab9ace3d8e13ede45f2e5978e56cc07f597884085a9c4886753f3e268d`
- Tool command for seed tests: `timeout 10 ./chibac_amd64-unknown-linux_chiba_dev.o`
- Result on 2026-05-09: `508 tests: 508 passed, 0 failed`

Generator tools present next to the seed:

- `./chibacc.o`
  - Hash: `168610ace8c552ec69d85c9fcc3d26736c73ed550b5d96c7fea1ab96c90a4479`
  - Usage command: `timeout 10 ./chibacc.o --help`
- `./chibalex.o`
  - Hash: `9f2de5e4ea7e724153505e07af367fc0210730c579ec940701a222d8d86c0526`
  - Usage command: `timeout 10 ./chibalex.o --help`

## Current Project Compile Baseline

Command:

```sh
timeout 10 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --output baseline_main.o
```

Current result:

```text
[resolve error] duplicate function `main.argv_cstr`
[resolve error] duplicate function `main.argv_str`
[resolve error] duplicate function `main.print_usage`
[resolve error] duplicate function `main.main`
```

Cause: both `src/chiba_level1_lexer_spec_main.chiba` and
`src/chiba_level1_parser_spec_main.chiba` declare `namespace main` and define
the same CLI entry helper names. Bootstrap validation should compile isolated
runner projects or split these entrypoints before using whole-project compile as
a green baseline.

Isolated entry commands:

```sh
timeout 120 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --entry chiba_level1_lexer_spec_main.chiba --output lexer_spec_runner.o
timeout 120 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --entry chiba_level1_parser_spec_main.chiba --output parser_spec_runner.o
```

Current result: both commands complete after phase1 when given a relaxed
post-phase1 timeout and write:

- `target/debug/lexer_spec_runner.o`
- `target/debug/parser_spec_runner.o`

Runner smoke commands:

```sh
timeout 10 ./target/debug/lexer_spec_runner.o chiba-level1-grammar-spec/01-test.chiba
timeout 10 ./target/debug/parser_spec_runner.o chiba-level1-grammar-spec/01-test.chiba
```

Current result: both runners execute and return status `0`.

Golden status: not yet clean. `01-test.lexer.spec` currently contains AST-shaped
output while `lexer_spec_runner.o` prints token spans. `01-test.parser.spec`
differs from the generated parser output around `DefItem2`, where the runner
prints the newer `DefName(...)` wrapper. This is a parser/grammar spec drift to
resolve before B06 grammar validation.

## Invocation Rule

Level-0 compiler, chibacc, and chibalex invocations should first use `timeout
10` to catch phase1 hangs. Once scanning/resolving/type-checking has passed and
the command is known to be in later lowering/codegen, use a relaxed explicit
timeout such as `timeout 120` and require the command to finish.

## First Bootstrap Validation Snapshot

Command:

```sh
vp run validate:first-bootstrap
```

Current result on 2026-05-10:

- bootstrap WAT smoke: pass
- bootstrap WAT smoke with Binaryen `--opt`: pass
- lexer compare: 30 specs pass
- parser compare: 30 specs pass
- parser error smoke: 108 specs pass
- `level1c.wasm --help`: pass
- `level1c.wasm parse chiba-level1-grammar-spec/01-test.chiba`: pass
- `level1c.wasm check supports/bootstrap/continuation-valid.chiba`: pass
- `level1c.wasm cont-usage supports/bootstrap/continuation-multi-resume.chiba`: pass
- Node.js: `v24.15.0`
- Binaryen: `129.0.0`

Current hashes:

- seed `chibac_amd64-unknown-linux_chiba_dev.o`: `7a7744ab9ace3d8e13ede45f2e5978e56cc07f597884085a9c4886753f3e268d`
- native bootstrap runner `target/debug/level1c.o`: `1b3f3e4a9d91cb804e7230c7acd50c088172cb51778da8e0a5c32981bab13c8d`
- native parser runner `target/debug/parser_spec_runner.o`: `11cac5f09b3d2f2f0c8a0da26b14fc24c0f8f1926056d0f21baff489e0db1b59`
- generated `level1c.wat`: `f48cbc54a8d920143318a710666f54656312ffe978c58584edbda8b2ef3db1b8`
- generated `level1c.wasm`: `8592ef1fc4683263292e712e102962fbe7ed1fb8701d5fddd9c342d12a23b66a`
- generated `wat-tuple-heap-smoke.wat`: `a6cdf4d249822e454a9a9b4f695f5109d01703b9999237b9d8f068390fed95b8`
- generated `wat-tuple-heap-smoke.wasm`: `3f005da259eab137cf66975cae8bfc89c8c33b3aaf6da837dfd3b54f3e26abc1`
- generated `wat-extern-env-smoke.wat`: `795a1c76d5c6a60c6be42205fb18c1ce5ca98700159c831378431f17bd078f68`
- generated `wat-extern-env-smoke.wasm`: `6f51626725cc1038b9b429088bedf4497f75c410f12886a0924f0b862c1b5186`

The first `level1c.wasm` is a host-assisted bootstrap skeleton: the wasm module
is emitted from level-1 Chiba source by the level-0 seed and dispatches through
explicit `extern "C" ...` imports. Parser/check execution still delegates to the
native level-1 runner through the Node bootstrap launcher; replacing those host
imports with in-wasm lexer/parser/runtime code is the handoff into the second
bootstrap work.
