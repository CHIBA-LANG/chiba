# level-1b compiler cli

The `chibac` CLI follows the toolchain spec:

```text
chibac <file>... -I <dir>* --target wasm32-unknown-wasi --backend wasm-gc -o <file>
```

Supported options include `-I/--include`, `--target/-t`, `--backend/-B`,
`-o/--out`, `--emit`, `-S`, `-c`, `-E`, `-O0`, `-O1`, `-O2`, `-O3`, `-Os`,
`-Oz`, `-g`, and diagnostic controls.

The default behavior must be explicit and diagnostic-friendly. Unknown flags,
unsupported targets, unsupported backends, missing input files, and invalid
option combinations report stable errors.
