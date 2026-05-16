(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 128) "chibac level-1b\0ausage: chibac <file>... -I <dir> --target wasm32-unknown-wasi --backend wasm-gc -o <file>\0aoptions: -I --include -t --target -B --backend -o --out --emit -S -c -E -O0 -O1 -O2 -O3 -Os -Oz -g --error-format --color --diagnostic-width\0a")
  (func (export "_start")
    i32.const 0
    i32.const 128
    i32.store
    i32.const 4
    i32.const 247
    i32.store
    i32.const 1
    i32.const 0
    i32.const 1
    i32.const 64
    call $fd_write
    drop)
)
