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
```
