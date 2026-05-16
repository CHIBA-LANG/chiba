(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 128) "level1b regex c04 ok\0a")
  (func $_start (export "_start")
    (i32.store (i32.const 8) (i32.const 128))
    (i32.store (i32.const 12) (i32.const 22))
    (drop (call $fd_write
      (i32.const 1)
      (i32.const 8)
      (i32.const 1)
      (i32.const 32)))))
