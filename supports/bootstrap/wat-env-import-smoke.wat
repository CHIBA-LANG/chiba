(module
  (import "env" "js_log" (func $js_log (param i64) (result i64)))
  (func (export "main") (result i64)
    i64.const 41
    call $js_log
    drop
    i64.const 9)
)
