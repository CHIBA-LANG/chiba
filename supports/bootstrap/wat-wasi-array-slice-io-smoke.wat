(module
  (type $array_u8 (array (mut i8)))
  (type $slice_u8
    (struct
      (field (ref $array_u8))
      (field i32)
      (field i32)))

  (import "wasi_snapshot_preview1" "path_open"
    (func $path_open
      (param i32 i32 i32 i32 i32 i64 i64 i32 i32)
      (result i32)))
  (import "wasi_snapshot_preview1" "fd_read"
    (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_close"
    (func $fd_close (param i32) (result i32)))

  (memory (export "memory") 1)
  (data (i32.const 128) "supports/bootstrap/wasi-read-input.txt")

  (func (export "_initialize"))

  (func $array_u8_from_memory (param $ptr i32) (param $len i32) (result (ref $array_u8))
    (local $out (ref $array_u8))
    (local $i i32)
    local.get $len
    array.new_default $array_u8
    local.set $out
    block $done
      loop $copy
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done

        local.get $out
        local.get $i
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        array.set $array_u8

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy
      end
    end
    local.get $out)

  (func $slice_byte_at (param $s (ref $slice_u8)) (param $idx i32) (result i64)
    local.get $idx
    i32.const 0
    i32.lt_s
    if
      unreachable
    end
    local.get $idx
    local.get $s
    struct.get $slice_u8 2
    i32.ge_u
    if
      unreachable
    end
    local.get $s
    struct.get $slice_u8 0
    local.get $s
    struct.get $slice_u8 1
    local.get $idx
    i32.add
    array.get_u $array_u8
    i64.extend_i32_u)

  (func $slice_to_memory (param $s (ref $slice_u8)) (param $ptr i32)
    (local $i i32)
    block $done
      loop $copy
        local.get $i
        local.get $s
        struct.get $slice_u8 2
        i32.ge_u
        br_if $done

        local.get $ptr
        local.get $i
        i32.add
        local.get $s
        struct.get $slice_u8 0
        local.get $s
        struct.get $slice_u8 1
        local.get $i
        i32.add
        array.get_u $array_u8
        i32.store8

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy
      end
    end)

  (func $write_slice_stdout (param $s (ref $slice_u8)) (result i32)
    local.get $s
    i32.const 512
    call $slice_to_memory

    i32.const 32
    i32.const 512
    i32.store
    i32.const 36
    local.get $s
    struct.get $slice_u8 2
    i32.store

    i32.const 1
    i32.const 32
    i32.const 1
    i32.const 48
    call $fd_write)

  ;; This stands in for lexer/parser input: it only receives the Slice[u8] view.
  (func $lexer_first_byte (param $input (ref $slice_u8)) (result i64)
    local.get $input
    i32.const 0
    call $slice_byte_at)

  (func (export "main") (result i64)
    (local $errno i32)
    (local $fd i32)
    (local $nread i32)
    (local $arr (ref $array_u8))
    (local $input (ref $slice_u8))

    i32.const 3
    i32.const 0
    i32.const 128
    i32.const 38
    i32.const 0
    i64.const 2
    i64.const 0
    i32.const 0
    i32.const 0
    call $path_open
    local.set $errno

    local.get $errno
    if
      local.get $errno
      i64.extend_i32_u
      i64.const 1000
      i64.add
      return
    end

    i32.const 0
    i32.load
    local.set $fd

    i32.const 8
    i32.const 64
    i32.store
    i32.const 12
    i32.const 128
    i32.store

    local.get $fd
    i32.const 8
    i32.const 1
    i32.const 24
    call $fd_read
    local.set $errno

    local.get $fd
    call $fd_close
    drop

    local.get $errno
    if
      local.get $errno
      i64.extend_i32_u
      i64.const 2000
      i64.add
      return
    end

    i32.const 24
    i32.load
    local.set $nread

    i32.const 64
    local.get $nread
    call $array_u8_from_memory
    local.set $arr

    local.get $arr
    i32.const 0
    local.get $nread
    struct.new $slice_u8
    local.set $input

    local.get $input
    call $write_slice_stdout
    drop

    local.get $input
    call $lexer_first_byte)
)
