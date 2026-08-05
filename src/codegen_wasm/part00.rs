// ===================================================================
// openOODA WebAssembly (.wat) Code Generator
//
// Honest subset: Int / Bool / Float arithmetic, function calls, `let`,
// `if`/`while`, interned String *literals* (data segments + `println_str`),
// content `==`/`!=` via host `env.streq`, and **List[Int]** only
// (`list_new` / `list_push` / `list_get` / `list_len` + bump heap).
//
// Fail-closed (non-zero): Match, capability I/O, non-Int/String lists,
// struct, string *numeric* arithmetic (sub/mul/div). String `+` concatenates
// on the bump heap (gated). List[Int] and List[String] on bump heap; String
// list `==` uses `$list_str_eq` (streq content, not pointer). List methods
// `.push`/`.len` lower to free `list_*` (not general method dispatch).
//
// String model: distinct UTF-8 literals interned as NUL-terminated bytes;
// values are i32 offsets. List model: header {len,cap,data} + i64 elements
// on a bump heap (`$heap` global). List / str_slice / str_concat heap only when used (W↓).
//
// Locals: nested `let` inside while/if (e.g. for-list desugar) are collected
// into the function's local table so wasmtime type-checks.
//
// WAT validated via `wasm-tools validate` when available, else structurally.
// ===================================================================
use crate::ast::*;
use anyhow::{anyhow, bail, Result};
use std::collections::{BTreeMap, HashSet};
use std::process::Command;

thread_local! {
    /// Nested while break/continue label pairs for the current WASM emit.
    static WASM_LOOP_STACK: std::cell::RefCell<Vec<(String, String)>> =
        std::cell::RefCell::new(Vec::new());
    /// Intern pool for string literals in the current emit (order = data layout).
    static WASM_STRINGS: std::cell::RefCell<Vec<String>> =
        std::cell::RefCell::new(Vec::new());
    /// Struct definitions for WASM memory layout (name -> fields).
    static WASM_STRUCTS: std::cell::RefCell<std::collections::HashMap<String, Vec<String>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Intern `s` into the WASM string pool; return its i32 data-segment offset.
/// Duplicate content reuses the first offset (zero-cost after first insert).
fn intern_string(s: &str) -> u32 {
    WASM_STRINGS.with(|cell| {
        let mut strings = cell.borrow_mut();
        let mut offset: u32 = 0;
        for existing in strings.iter() {
            if existing == s {
                return offset;
            }
            offset = offset.saturating_add(existing.len() as u32).saturating_add(1);
        }
        strings.push(s.to_string());
        offset
    })
}

pub struct WasmCodeGen;
