;; Minimal test extension WASM module.
;; Exports handle_tool which returns a fixed JSON string: {"result":"hello"}
(module
    ;; 1 page = 64KB of memory
    (memory (export "memory") 1)

    ;; Store the result string at offset 0 in data segment
    ;; {"result":"hello"}
    (data (i32.const 0) "{\"result\":\"hello\"}")

    ;; handle_tool(name_ptr: i32, name_len: i32, params_ptr: i32, params_len: i32) -> i64
    ;; Returns packed (ptr << 32 | len)
    ;; Our string is at ptr=0, len=18
    (func (export "handle_tool") (param i32 i32 i32 i32) (result i64)
        ;; ptr=0, len=18 → (0 << 32) | 18 = 18
        (i64.const 18)
    )
)
