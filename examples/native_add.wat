;; ===================================================================
;; openOODA WebAssembly Text Format (.wat) Target Backend
;; ===================================================================

(module
  (import "env" "println" (func $println (param i64)))
  (func $add_numbers (param $a i64) (param $b i64) (result i64)
    local.get $a
    local.get $b
    i64.add
    return
  )
  (func $main (export "main") (result i32)
    local.get $res
    call $println
    i32.const 0
  )
)
