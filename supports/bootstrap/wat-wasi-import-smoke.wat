(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 8) "B04 wasi smoke ok\n")
  (func (export "_initialize"))
  (func (export "main") (result i64)
    i32.const 0
    i32.const 8
    i32.store
    i32.const 4
    i32.const 18
    i32.store
    i32.const 1
    i32.const 0
    i32.const 1
    i32.const 20
    call $fd_write
    drop
    i64.const 0)
)
