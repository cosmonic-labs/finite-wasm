(module
  (func (export "finite_wasm_gas_exhausted"))
  (func (export "finite_wasm_stack_exhausted"))
)
(register "internal")

(module
  (import "internal" "finite_wasm_gas_exhausted" (func $finite_wasm_gas_exhausted))
  (import "internal" "finite_wasm_stack_exhausted" (func $finite_wasm_stack_exhausted))

  (global $gas (export "finite_wasm_remaining_gas") (mut i64) (i64.const 0))

  ;; intial value of this will be hard-coded, exported just for testing
  (global $stack (export "finite_wasm_remaining_stack") (mut i64) (i64.const 0))

  ;; this will be hard-coded, exported mutable just for testing
  (global $op_cost (export "finite_wasm_op_cost") (mut i32) (i32.const 0))

  ;; functions below will be inlined during instrumentation

  (func (export "finite_wasm_gas") (param $n i64)
    global.get $gas
    i64.const 0
    local.get $n
    i64.const 0
    ;; $gas | 0 | $n | 0

    i64.sub128
    i64.popcnt
    i32.wrap_i64
    if
        call $finite_wasm_gas_exhausted
        unreachable
    end
    ;; $gas - $n

    global.set $gas
  )

  (func (export "finite_wasm_stack") (param $operand_size i64) (param $frame_size i64)
    global.get $stack
    i64.const 0
    ;; $stack | 0

    local.get $operand_size
    i64.const 0
    ;; $stack | 0 | $operand_size | 0

    i64.sub128
    i64.popcnt
    i32.wrap_i64
    if
        call $finite_wasm_stack_exhausted
        unreachable
    end
    ;; $stack - $operand_size

    i64.const 0
    local.get $frame_size
    i64.const 0
    ;; $stack - $operand_size | 0 | $frame_size | 0

    i64.sub128
    i64.popcnt
    i32.wrap_i64
    if
        call $finite_wasm_stack_exhausted
        unreachable
    end
    ;; $stack - $operand_size - $frame_size

    global.set $stack


    global.get $gas
    i64.const 0
    local.get $frame_size
    global.get $op_cost
    i64.extend_i32_u
    ;; $gas | 0 | $frame_size | $op_cost

    i64.mul_wide_u
    i64.popcnt
    i32.wrap_i64
    if
        call $finite_wasm_gas_exhausted
        unreachable
    end
    ;; $gas | 0 | $frame_size * $op_cost

    i64.const 0
    ;; $gas | 0 | $frame_size * $op_cost | 0

    i64.sub128
    i64.popcnt
    i32.wrap_i64
    if
        call $finite_wasm_gas_exhausted
        unreachable
    end
    ;; $gas - $frame_size * $op_cost
    global.set $gas

    local.get $frame_size
    i64.const 8
    ;; $frame_size | 8

    i64.rem_u
    i32.wrap_i64
    ;; $frame_size % 8
    if
        global.get $gas
        i64.const 0
        local.get $frame_size
        i64.const 0
        ;; $gas | 0 | $frame_size | 0

        i64.sub128
        i64.popcnt
        i32.wrap_i64
        if
            call $finite_wasm_gas_exhausted
            unreachable
        end
        global.set $gas
    end
  )

  (func (export "finite_wasm_unstack") (param $operand_size i64) (param $frame_size i64)
    global.get $stack
    i64.const 0
    ;; $stack | 0

    local.get $operand_size
    i64.const 0
    ;; $stack | 0 | $operand_size | 0

    i64.add128
    i64.popcnt
    i32.wrap_i64
    if
        unreachable
    end
    ;; $stack + $operand_size

    i64.const 0
    local.get $frame_size
    i64.const 0
    ;; $stack + $operand_size | 0 | $frame_size | 0

    i64.add128
    i64.popcnt
    i32.wrap_i64
    if
        unreachable
    end
    ;; $stack + $operand_size + $frame_size

    global.set $stack
  )

  (func (export "linear_gas") (param $count i32) (param $linear i64) (param $constant i64)
    global.get $gas
    i64.const 0
    local.get $count
    i64.extend_i32_u 
    local.get $linear
    ;; $gas | 0 | $count | $linear

    i64.mul_wide_u
    i64.popcnt
    i32.wrap_i64
    if
        call $finite_wasm_gas_exhausted
        unreachable
    end
    ;; $gas | 0 | $count * $linear

    i64.const 0
    local.get $constant
    i64.const 0
    ;; $gas | 0 | $count * $linear | 0 | $constant | 0

    i64.add128
    i64.popcnt
    i32.wrap_i64
    if
        call $finite_wasm_gas_exhausted
        unreachable
    end
    ;; $gas | 0 | $count * $linear + $constant

    i64.const 0
    ;; $gas | 0 | $count * $linear + $constant | 0

    i64.sub128
    i64.popcnt
    i32.wrap_i64
    if
        call $finite_wasm_gas_exhausted
        unreachable
    end

    global.set $gas
  )
)

(register "generated")

(module
  (import "generated" "finite_wasm_remaining_gas" (global $finite_wasm_remaining_gas (mut i64)))
  (import "generated" "finite_wasm_remaining_stack" (global $finite_wasm_remaining_stack (mut i64)))
  (import "generated" "finite_wasm_op_cost" (global $finite_wasm_op_cost (mut i32)))

  (import "generated" "linear_gas" (func $linear_gas (param i32 i64 i64)))
  (import "generated" "finite_wasm_stack" (func $finite_wasm_stack (param i64 i64)))
  (import "generated" "finite_wasm_unstack" (func $finite_wasm_unstack (param i64 i64)))

  (func (export "get_gas") (result i64) (global.get $finite_wasm_remaining_gas))
  (func (export "set_gas") (param i64) (global.set $finite_wasm_remaining_gas (local.get 0)))

  (func (export "get_stack") (result i64) (global.get $finite_wasm_remaining_stack))
  (func (export "set_stack") (param i64) (global.set $finite_wasm_remaining_stack (local.get 0)))

  (func (export "get_op_cost") (result i32) (global.get $finite_wasm_op_cost))
  (func (export "set_op_cost") (param i32) (global.set $finite_wasm_op_cost (local.get 0)))

  (export "linear_gas" (func $linear_gas))
  (export "finite_wasm_stack" (func $finite_wasm_stack))
  (export "finite_wasm_unstack" (func $finite_wasm_unstack))
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


(invoke "set_gas" (i64.const 0xffff_ffff_ffff_ffff))
(invoke "finite_wasm_stack" (i64.const 0) (i64.const 0))
(assert_trap (invoke "finite_wasm_stack" (i64.const 1) (i64.const 0)) "unreachable")
(assert_trap (invoke "finite_wasm_stack" (i64.const 0) (i64.const 1)) "unreachable")
(assert_trap (invoke "finite_wasm_stack" (i64.const 1) (i64.const 1)) "unreachable")
(assert_return (invoke "get_gas") (i64.const 0xffff_ffff_ffff_ffff))

(invoke "set_gas" (i64.const 10))
(invoke "set_stack" (i64.const 10))
(invoke "finite_wasm_stack" (i64.const 1) (i64.const 1))
(assert_return (invoke "get_stack") (i64.const 8))
(assert_return (invoke "get_gas") (i64.const 9))

(invoke "set_gas" (i64.const 10))
(invoke "set_stack" (i64.const 10))
(invoke "finite_wasm_stack" (i64.const 2) (i64.const 1))
(assert_return (invoke "get_stack") (i64.const 7))
(assert_return (invoke "get_gas") (i64.const 9))

(invoke "set_gas" (i64.const 10))
(invoke "set_stack" (i64.const 10))
(invoke "finite_wasm_stack" (i64.const 1) (i64.const 2))
(assert_return (invoke "get_stack") (i64.const 7))
(assert_return (invoke "get_gas") (i64.const 8))

(invoke "set_gas" (i64.const 10))
(invoke "set_stack" (i64.const 10))
(invoke "set_op_cost" (i32.const 1))
(invoke "finite_wasm_stack" (i64.const 1) (i64.const 2))
(assert_return (invoke "get_stack") (i64.const 7))
(assert_return (invoke "get_gas") (i64.const 6))

(invoke "set_gas" (i64.const 10))
(invoke "set_stack" (i64.const 10))
(invoke "set_op_cost" (i32.const 2))
(invoke "finite_wasm_stack" (i64.const 1) (i64.const 2))
(assert_return (invoke "get_stack") (i64.const 7))
(assert_return (invoke "get_gas") (i64.const 4))

(invoke "set_gas" (i64.const 10))
(invoke "set_stack" (i64.const 10))
(invoke "set_op_cost" (i32.const 2))
(invoke "finite_wasm_stack" (i64.const 1) (i64.const 3))
(assert_return (invoke "get_stack") (i64.const 6))
(assert_return (invoke "get_gas") (i64.const 1))

(invoke "set_gas" (i64.const 40))
(invoke "set_stack" (i64.const 17))
(invoke "set_op_cost" (i32.const 2))
(invoke "finite_wasm_stack" (i64.const 1) (i64.const 16))
(assert_return (invoke "get_stack") (i64.const 0))
(assert_return (invoke "get_gas") (i64.const 8))
