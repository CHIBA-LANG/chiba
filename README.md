# usage

[![chiba-ci](https://github.com/CHIBA-LANG/chiba-level0/actions/workflows/ci.yml/badge.svg)](https://github.com/CHIBA-LANG/chiba-level0/actions/workflows/ci.yml)

1. has `gcc`
2. download main.o from releases https://github.com/CHIBA-LANG/chiba-level0/releases
3. ./main.o --project .

## Bootstrap smoke commands

```sh
vp run smoke:bootstrap
vp exec node tools/node/run-wat.mjs supports/bootstrap/wat-env-import-smoke.wat
vp exec node tools/node/run-wat.mjs supports/bootstrap/wat-wasi-import-smoke.wat
timeout 120 ./chibac_amd64-unknown-linux_chiba_dev.o --project . --entry chiba_level1c_main.chiba --output level1c.o
timeout 20 ./target/debug/level1c.o --help
timeout 20 ./target/debug/level1c.o parse chiba-level1-grammar-spec/01-test.chiba
timeout 20 ./target/debug/level1c.o check chiba-level1-grammar-spec/01-test.chiba
timeout 20 ./target/debug/level1c.o check supports/bootstrap/continuation-valid.chiba
timeout 20 ./target/debug/level1c.o check supports/bootstrap/continuation-invalid.chiba
timeout 20 ./target/debug/level1c.o wat chiba-level1-grammar-spec/01-test.chiba > .scratch/level1c-01.wat
vp exec node tools/node/run-wat.mjs .scratch/level1c-01.wat
```
