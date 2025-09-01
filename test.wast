(module
  (func (export "finite_wasm_gas_exhausted"))
)
(register "internal")

(module
  (import "internal" "finite_wasm_gas_exhausted" (func $finite_wasm_gas_exhausted))

  (global $gas (export "finite_wasm_gas") (mut i64) (i64.const 0))

  (func (export "linear_gas") (param $count i32) (param $linear i64) (param $constant i64)
    global.get $gas
    i64.const 0

    local.get $count
    i64.extend_i32_u 

    local.get $linear

    i64.mul_wide_u

    i64.popcnt
    i32.wrap_i64
    (if
      (then
        call $finite_wasm_gas_exhausted
        unreachable
      )
    )

    i64.const 0

    local.get $constant

    i64.const 0

    i64.add128

    i64.popcnt
    i32.wrap_i64
    (if
      (then
        call $finite_wasm_gas_exhausted
        unreachable
      )
    )

    i64.const 0

    i64.sub128

    i64.popcnt
    i32.wrap_i64
    (if
      (then
        call $finite_wasm_gas_exhausted
        unreachable
      )
    )

    global.set $gas
  )
)

(register "generated")

(module
  (import "generated" "finite_wasm_gas" (global $finite_wasm_gas (mut i64)))
  (import "generated" "linear_gas" (func $linear_gas (param i32 i64 i64)))

  (func (export "get_gas") (result i64) (global.get $finite_wasm_gas))
  (func (export "set_gas") (param i64) (global.set $finite_wasm_gas (local.get 0)))

  (export "linear_gas" (func $linear_gas))
)

(assert_trap (invoke "linear_gas" (i32.const 1) (i64.const 1) (i64.const 0)) "unreachable")
(assert_trap (invoke "linear_gas" (i32.const 1) (i64.const 0) (i64.const 1)) "unreachable")
(assert_trap (invoke "linear_gas" (i32.const 0) (i64.const 1) (i64.const 1)) "unreachable")
(assert_trap (invoke "linear_gas" (i32.const 0) (i64.const 0) (i64.const 1)) "unreachable")

(invoke "set_gas" (i64.const 1))

(assert_trap (invoke "linear_gas" (i32.const 0) (i64.const 0) (i64.const 2)) "unreachable")
(assert_trap (invoke "linear_gas" (i32.const 2) (i64.const 1) (i64.const 0)) "unreachable")
(assert_trap (invoke "linear_gas" (i32.const 1) (i64.const 2) (i64.const 0)) "unreachable")

(assert_return (invoke "get_gas") (i64.const 1))

(invoke "set_gas" (i64.const 10))
(invoke "linear_gas" (i32.const 1) (i64.const 1) (i64.const 0))
(assert_return (invoke "get_gas") (i64.const 9))

(invoke "set_gas" (i64.const 10))
(invoke "linear_gas" (i32.const 1) (i64.const 0) (i64.const 1))
(assert_return (invoke "get_gas") (i64.const 9))

(invoke "set_gas" (i64.const 10))
(invoke "linear_gas" (i32.const 0) (i64.const 1) (i64.const 1))
(assert_return (invoke "get_gas") (i64.const 9))

(invoke "set_gas" (i64.const 10))
(invoke "linear_gas" (i32.const 0) (i64.const 0) (i64.const 1))
(assert_return (invoke "get_gas") (i64.const 9))

(invoke "set_gas" (i64.const 10))
(invoke "linear_gas" (i32.const 1) (i64.const 1) (i64.const 1))
(assert_return (invoke "get_gas") (i64.const 8))

(invoke "set_gas" (i64.const 10))
(invoke "linear_gas" (i32.const 2) (i64.const 1) (i64.const 1))
(assert_return (invoke "get_gas") (i64.const 7))

(invoke "set_gas" (i64.const 10))
(invoke "linear_gas" (i32.const 1) (i64.const 2) (i64.const 1))
(assert_return (invoke "get_gas") (i64.const 7))

(invoke "set_gas" (i64.const 10))
(assert_trap (invoke "linear_gas" (i32.const 10) (i64.const 1) (i64.const 1)) "unreachable")
(assert_trap (invoke "linear_gas" (i32.const 1) (i64.const 10) (i64.const 1)) "unreachable")
(assert_trap (invoke "linear_gas" (i32.const 1) (i64.const 1) (i64.const 10)) "unreachable")

(invoke "set_gas" (i64.const 0xffff_ffff_ffff_ffff))
(invoke "linear_gas" (i32.const 0) (i64.const 0) (i64.const 0xffff_ffff_ffff_ffff))
(assert_return (invoke "get_gas") (i64.const 0))

(invoke "set_gas" (i64.const 0xffff_ffff_ffff_ffff))
(invoke "linear_gas" (i32.const 1) (i64.const 0xffff_ffff_ffff_ffff) (i64.const 0))
(assert_return (invoke "get_gas") (i64.const 0))

(invoke "set_gas" (i64.const 0xffff_ffff_ffff_ffff))
(assert_trap (invoke "linear_gas" (i32.const 2) (i64.const 0xffff_ffff_ffff_ffff) (i64.const 0)) "unreachable")
(assert_trap (invoke "linear_gas" (i32.const 0xffff_ffff) (i64.const 0xffff_ffff_ffff) (i64.const 0)) "unreachable")
(assert_trap (invoke "linear_gas" (i32.const 1) (i64.const 1) (i64.const 0xffff_ffff_ffff_ffff)) "unreachable")
