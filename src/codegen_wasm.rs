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

impl WasmCodeGen {
    pub fn emit_wat(program: &Program) -> Result<String> {
        WASM_STRINGS.with(|s| s.borrow_mut().clear());
        let aliases = program.collect_type_aliases();
        
        WASM_STRUCTS.with(|s| {
            let mut structs = s.borrow_mut();
            structs.clear();
            for (name, ty) in &aliases {
                if let Type::Custom(def) = ty {
                    if let Some(fields_str) = def.strip_prefix("struct:") {
                        let inner = if let Some(idx) = fields_str.find('{') {
                            fields_str[idx+1..fields_str.len()-1].to_string()
                        } else {
                            fields_str.to_string()
                        };
                        let mut field_names = Vec::new();
                        if !inner.is_empty() {
                            for f in inner.split(',') {
                                let parts: Vec<&str> = f.split(':').collect();
                                field_names.push(parts[0].to_string());
                            }
                        }
                        structs.insert(name.clone(), field_names);
                    }
                } else if let Type::Struct { fields, .. } = ty {
                    structs.insert(name.clone(), fields.iter().map(|(n, _)| n.clone()).collect());
                }
            }
        });
        
        let needs_list_rt = Self::program_needs_list_runtime(program);
        // Emit function bodies first so we can gate host imports + heap on real use (E-M D↓/W↓):
        // pure Int programs must not pull streq/str_contains/println_str or $heap.
        let mut funcs_wat = String::new();
        for item in &program.items {
            if let Item::Function(func) = item {
                let f_wat = Self::emit_function(func, &aliases)?;
                funcs_wat.push_str(&f_wat);
                funcs_wat.push('\n');
            }
        }
        // Import surface from actual call sites in emitted WAT (not speculative AST).
        // Match `call $println` without also matching `call $println_str`.
        let needs_println = funcs_wat.lines().any(|l| {
            let t = l.trim();
            t == "call $println" || t.starts_with("call $println ")
        });
        let needs_println_str = funcs_wat.contains("call $println_str");
        // `$list_str_eq` RT calls `$streq` — import when either body or eq RT needs it.
        let needs_list_str_eq = funcs_wat.contains("call $list_str_eq");
        let needs_streq = funcs_wat.contains("call $streq") || needs_list_str_eq;
        let needs_str_contains = funcs_wat.contains("call $str_contains");
        // Heap only when bodies actually bump-allocate (list RT / str_slice / str_concat).
        let body_uses_heap = funcs_wat.contains("global.get $heap");

        let mut wat = String::new();
        wat.push_str(";; ===================================================================\n");
        wat.push_str(";; openOODA WebAssembly Text Format (.wat) Target Backend\n");
        wat.push_str(";; Generated by `ooda build --target wasm` (v0.24.0-alpha)\n");
        wat.push_str(";; ===================================================================\n\n");
        wat.push_str("(module\n");
        if needs_println {
            wat.push_str("  (import \"env\" \"println\" (func $println (param i64)))\n");
        }
        if needs_println_str {
            wat.push_str("  (import \"env\" \"println_str\" (func $println_str (param i32)))\n");
        }
        if needs_streq {
            wat.push_str("  (import \"env\" \"streq\" (func $streq (param i32 i32) (result i32)))\n");
        }
        if needs_str_contains {
            wat.push_str(
                "  (import \"env\" \"str_contains\" (func $str_contains (param i32 i32) (result i32)))\n",
            );
        }
        if needs_println || needs_println_str || needs_streq || needs_str_contains {
            wat.push('\n');
        }

        let string_count = WASM_STRINGS.with(|s| s.borrow().len());
        let needs_heap = needs_list_rt || body_uses_heap;
        let needs_memory = needs_list_rt
            || string_count > 0
            || needs_heap
            || funcs_wat.contains("i32.load8_u");
        if needs_memory {
            wat.push_str("  (memory 1)\n");
            wat.push_str("  (export \"memory\" (memory 0))\n");
            let mut offset = 0usize;
            WASM_STRINGS.with(|strings| {
                for s in strings.borrow().iter() {
                    let mut hex_str = String::new();
                    for b in s.as_bytes() {
                        hex_str.push_str(&format!("\\{:02x}", b));
                    }
                    hex_str.push_str("\\00");
                    wat.push_str(&format!("  (data (i32.const {}) \"{}\")\n", offset, hex_str));
                    offset += s.len() + 1;
                }
            });
            if needs_heap {
                let heap_start = (offset + 15) & !15;
                wat.push_str(&format!(
                    "  (global $heap (mut i32) (i32.const {}))\n",
                    heap_start
                ));
            }
            if needs_list_rt {
                // Base list RT always when lists used; eq helpers only if called (W↓).
                wat.push_str(Self::list_runtime_wat());
                if funcs_wat.contains("call $list_eq") {
                    wat.push_str(Self::list_eq_runtime_wat());
                }
                if needs_list_str_eq {
                    wat.push_str(Self::list_str_eq_runtime_wat());
                }
            }
        }

        wat.push_str(&funcs_wat);
        wat.push_str(")\n");
        Self::validate_wat(&wat)?;
        Ok(wat)
    }

    /// List[Int] bump-heap runtime (only emitted when the program uses lists).
    /// Deep equality (`$list_eq`) is separate — see `list_eq_runtime_wat`.
    fn list_runtime_wat() -> &'static str {
        r#"
  (func $list_new (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap))
    (global.set $heap (i32.add (local.get $ptr) (i32.const 16)))
    (i32.store (local.get $ptr) (i32.const 0))
    (i32.store offset=4 (local.get $ptr) (i32.const 0))
    (i32.store offset=8 (local.get $ptr) (i32.const 0))
    (local.get $ptr)
    return
  )
  (func $list_len (param $list i32) (result i64)
    (i64.extend_i32_u (i32.load (local.get $list)))
    return
  )
  (func $list_get (param $list i32) (param $index i64) (result i64)
    (local $data i32)
    (local.set $data (i32.load offset=8 (local.get $list)))
    (i64.load (i32.add (local.get $data) (i32.mul (i32.wrap_i64 (local.get $index)) (i32.const 8))))
    return
  )
  (func $list_push (param $list i32) (param $elem i64) (result i32)
    (local $len i32) (local $cap i32) (local $data i32) (local $new_cap i32) (local $new_data i32) (local $i i32)
    (local.set $len (i32.load (local.get $list)))
    (local.set $cap (i32.load offset=4 (local.get $list)))
    (local.set $data (i32.load offset=8 (local.get $list)))
    (if (i32.eq (local.get $len) (local.get $cap))
      (then
        (if (i32.eqz (local.get $cap))
          (then (local.set $new_cap (i32.const 4)))
          (else (local.set $new_cap (i32.mul (local.get $cap) (i32.const 2))))
        )
        (local.set $new_data (global.get $heap))
        (global.set $heap (i32.add (local.get $new_data) (i32.mul (local.get $new_cap) (i32.const 8))))
        (if (i32.gt_u (local.get $cap) (i32.const 0))
          (then
            (local.set $i (i32.const 0))
            (loop $copy_loop
              (i64.store
                (i32.add (local.get $new_data) (i32.mul (local.get $i) (i32.const 8)))
                (i64.load (i32.add (local.get $data) (i32.mul (local.get $i) (i32.const 8))))
              )
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br_if $copy_loop (i32.lt_u (local.get $i) (local.get $cap)))
            )
          )
        )
        (i32.store offset=4 (local.get $list) (local.get $new_cap))
        (i32.store offset=8 (local.get $list) (local.get $new_data))
        (local.set $data (local.get $new_data))
      )
    )
    (i64.store (i32.add (local.get $data) (i32.mul (local.get $len) (i32.const 8))) (local.get $elem))
    (i32.store (local.get $list) (i32.add (local.get $len) (i32.const 1)))
    (local.get $list)
    return
  )
"#
    }

    /// Deep List[Int] equality — only when `call $list_eq` appears (W↓).
    fn list_eq_runtime_wat() -> &'static str {
        r#"
  (func $list_eq (param $a i32) (param $b i32) (result i32)
    (local $len_a i32) (local $len_b i32) (local $data_a i32) (local $data_b i32) (local $i i32)
    (if (i32.eq (local.get $a) (local.get $b)) (then (return (i32.const 1))))
    (local.set $len_a (i32.load (local.get $a)))
    (local.set $len_b (i32.load (local.get $b)))
    (if (i32.ne (local.get $len_a) (local.get $len_b)) (then (return (i32.const 0))))
    (local.set $data_a (i32.load offset=8 (local.get $a)))
    (local.set $data_b (i32.load offset=8 (local.get $b)))
    (local.set $i (i32.const 0))
    (loop $cmp_loop
      (if (i32.eq (local.get $i) (local.get $len_a)) (then (return (i32.const 1))))
      (if (i64.ne
            (i64.load (i32.add (local.get $data_a) (i32.mul (local.get $i) (i32.const 8))))
            (i64.load (i32.add (local.get $data_b) (i32.mul (local.get $i) (i32.const 8))))
          )
          (then (return (i32.const 0)))
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $cmp_loop)
    )
    (i32.const 0)
    return
  )
"#
    }

    /// List[String] content equality via host `$streq` per element (not i64 pointer eq).
    /// Only injected when `call $list_str_eq` appears (W↓). Aligns with interpreter String PartialEq.
    fn list_str_eq_runtime_wat() -> &'static str {
        r#"
  (func $list_str_eq (param $a i32) (param $b i32) (result i32)
    (local $len_a i32) (local $len_b i32) (local $data_a i32) (local $data_b i32) (local $i i32) (local $pa i32) (local $pb i32)
    (if (i32.eq (local.get $a) (local.get $b)) (then (return (i32.const 1))))
    (local.set $len_a (i32.load (local.get $a)))
    (local.set $len_b (i32.load (local.get $b)))
    (if (i32.ne (local.get $len_a) (local.get $len_b)) (then (return (i32.const 0))))
    (local.set $data_a (i32.load offset=8 (local.get $a)))
    (local.set $data_b (i32.load offset=8 (local.get $b)))
    (local.set $i (i32.const 0))
    (loop $cmp_loop
      (if (i32.eq (local.get $i) (local.get $len_a)) (then (return (i32.const 1))))
      (local.set $pa (i32.wrap_i64 (i64.load (i32.add (local.get $data_a) (i32.mul (local.get $i) (i32.const 8))))))
      (local.set $pb (i32.wrap_i64 (i64.load (i32.add (local.get $data_b) (i32.mul (local.get $i) (i32.const 8))))))
      (if (i32.eqz (call $streq (local.get $pa) (local.get $pb))) (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $cmp_loop)
    )
    (i32.const 0)
    return
  )
"#
    }

    fn program_needs_list_runtime(program: &Program) -> bool {
        for item in &program.items {
            if let Item::Function(f) = item {
                if Self::type_is_list(&f.return_type) {
                    return true;
                }
                for p in &f.params {
                    if Self::type_is_list(&p.param_type) {
                        return true;
                    }
                }
                if Self::block_needs_list(&f.body) {
                    return true;
                }
            }
        }
        false
    }

    
    fn is_type_string(t: &Type) -> bool {
        matches!(t, Type::String) || matches!(t, Type::Custom(s) if s == "String")
    }

    fn type_is_list(t: &Type) -> bool {
        matches!(t, Type::List(_))
    }

    fn block_needs_list(block: &Block) -> bool {
        for stmt in &block.stmts {
            if Self::stmt_needs_list(stmt) {
                return true;
            }
        }
        if let Some(e) = &block.expr {
            return Self::expr_needs_list(e);
        }
        false
    }

    fn stmt_needs_list(stmt: &Statement) -> bool {
        match stmt {
            Statement::Let {
                type_annotation,
                init,
                ..
            } => {
                if type_annotation
                    .as_ref()
                    .map(Self::type_is_list)
                    .unwrap_or(false)
                {
                    return true;
                }
                Self::expr_needs_list(init)
            }
            Statement::Assign { value, .. } | Statement::Return(Some(value), _) => {
                Self::expr_needs_list(value)
            }
            Statement::Expr(e, _) => Self::expr_needs_list(e),
            Statement::While { cond, body, .. } => {
                Self::expr_needs_list(cond) || Self::block_needs_list(body)
            }
            Statement::FieldAssign { object, value, .. } => {
                Self::expr_needs_list(object) || Self::expr_needs_list(value)
            }
            Statement::Return(None, _) | Statement::Break(_) | Statement::Continue(_) => false,
        }
    }

    fn expr_needs_list(expr: &Expression) -> bool {
        match expr {
            Expression::Call { name, args, .. } => {
                match name.as_str() {
                    // Explicit list ops always need the bump-heap list runtime.
                    "list_new" | "list_push" | "list_get" | "list_len" | ".push" => return true,
                    ".len" => {
                        // E-M W↓: String `.len` is pure WAT (NUL scan), whether the
                        // receiver is a literal or a String-typed local (`i32`).
                        // List `.len` needs `$list_len` only when the receiver is
                        // list-shaped (list_new/list_push/.push or nested list expr).
                        // Do NOT inject list RT for every non-literal `.len` — that
                        // falsely bloated `let s = "hi"; s.len()` and string_ops.oo.
                        if let Some(recv) = args.first() {
                            if Self::expr_is_list_shaped(recv) {
                                return true;
                            }
                        }
                    }
                    ".char_at" | ".contains" | ".str_slice" => {} // string surface, not list RT
                    _ => {}
                }
                args.iter().any(Self::expr_needs_list)
            }
            Expression::Binary { left, right, .. } => {
                Self::expr_needs_list(left) || Self::expr_needs_list(right)
            }
            Expression::Unary { expr, .. } => Self::expr_needs_list(expr),
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::expr_needs_list(cond)
                    || Self::block_needs_list(then_branch)
                    || else_branch
                        .as_ref()
                        .map(|b| Self::block_needs_list(b))
                        .unwrap_or(false)
            }
            Expression::While { cond, body, .. } => {
                Self::expr_needs_list(cond) || Self::block_needs_list(body)
            }
            Expression::Match { expr, arms, .. } => {
                Self::expr_needs_list(expr) || arms.iter().any(|a| Self::expr_needs_list(&a.body))
            }
            Expression::StructLit { fields, .. } => {
                fields.iter().any(|(_, e)| Self::expr_needs_list(e))
            }
            Expression::Literal(_, _) | Expression::Variable(_, _) => false,
        }
    }

    /// True when `expr` is known to produce a List pointer (not String i32).
    /// Used to decide whether `.len` needs `$list_len` RT vs pure string WAT.
    /// List-typed parameters / annotations still force RT via `program_needs_list_runtime`
    /// and `stmt_needs_list`; this only classifies expression shape at a use site.
    fn expr_is_list_shaped(expr: &Expression) -> bool {
        match expr {
            Expression::Call { name, .. } => {
                matches!(name.as_str(), "list_new" | "list_push" | ".push")
            }
            Expression::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::block_tail_is_list_shaped(then_branch)
                    || else_branch
                        .as_ref()
                        .map(|b| Self::block_tail_is_list_shaped(b))
                        .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn block_tail_is_list_shaped(block: &Block) -> bool {
        block
            .expr
            .as_ref()
            .map(|e| Self::expr_is_list_shaped(e))
            .unwrap_or(false)
    }

    fn require_list_supported(inner: &Type, ctx: &str) -> Result<()> {
        match inner {
            Type::Int | Type::String => Ok(()),
            Type::Custom(s) if s == "Int" || s == "String" || s == "_" => Ok(()), // unrefined / pending
            other => bail!(
                "WASM backend only supports List[Int]/List[String] (not {:?}) in '{}'.",
                other,
                ctx
            ),
        }
    }

    /// Map semantic local tags (`list` vs string `i32`) to WAT storage types.
    fn wat_storage_ty(sem: &str) -> &'static str {
        match sem {
            "list" | "list_str" | "i32" => "i32",
            "f64" => "f64",
            _ => "i64",
        }
    }

    /// Concatenate two NUL-terminated strings onto the bump heap; leave i32 ptr on stack.
    /// Scratch locals: `__cat_a`, `__cat_b`, `__cat_dst`, `__cat_i` (declared by collect).
    fn emit_str_concat(
        left: &Expression,
        right: &Expression,
        locals: &BTreeMap<String, &'static str>,
    ) -> Result<String> {
        for k in ["__cat_a", "__cat_b", "__cat_dst", "__cat_i"] {
            if !locals.contains_key(k) {
                bail!("internal: string concat missing scratch local {}", k);
            }
        }
        static CAT_LAB: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let id = CAT_LAB.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut wat = String::new();
        // a, b pointers
        wat.push_str(&Self::emit_expr(left, locals)?);
        wat.push_str("    local.set $__cat_a\n");
        wat.push_str(&Self::emit_expr(right, locals)?);
        wat.push_str("    local.set $__cat_b\n");
        // dst = heap
        wat.push_str("    global.get $heap\n");
        wat.push_str("    local.set $__cat_dst\n");
        // copy a into dst (until NUL), leave __cat_i = len_a
        wat.push_str("    i32.const 0\n");
        wat.push_str("    local.set $__cat_i\n");
        wat.push_str(&format!("    block $cat_a_done_{}\n", id));
        wat.push_str(&format!("      loop $cat_a_loop_{}\n", id));
        wat.push_str("        local.get $__cat_a\n");
        wat.push_str("        local.get $__cat_i\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        i32.load8_u\n");
        wat.push_str("        i32.eqz\n");
        wat.push_str(&format!("        br_if $cat_a_done_{}\n", id));
        wat.push_str("        local.get $__cat_dst\n");
        wat.push_str("        local.get $__cat_i\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        local.get $__cat_a\n");
        wat.push_str("        local.get $__cat_i\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        i32.load8_u\n");
        wat.push_str("        i32.store8\n");
        wat.push_str("        local.get $__cat_i\n");
        wat.push_str("        i32.const 1\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        local.set $__cat_i\n");
        wat.push_str(&format!("        br $cat_a_loop_{}\n", id));
        wat.push_str("      end\n");
        wat.push_str("    end\n");
        // copy b after a (until NUL); __cat_i advances
        wat.push_str("    i32.const 0\n");
        // reuse __cat_a as b-index (stack-local reuse, no extra W)
        wat.push_str("    local.set $__cat_a\n");
        wat.push_str(&format!("    block $cat_b_done_{}\n", id));
        wat.push_str(&format!("      loop $cat_b_loop_{}\n", id));
        wat.push_str("        local.get $__cat_b\n");
        wat.push_str("        local.get $__cat_a\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        i32.load8_u\n");
        wat.push_str("        i32.eqz\n");
        wat.push_str(&format!("        br_if $cat_b_done_{}\n", id));
        wat.push_str("        local.get $__cat_dst\n");
        wat.push_str("        local.get $__cat_i\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        local.get $__cat_b\n");
        wat.push_str("        local.get $__cat_a\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        i32.load8_u\n");
        wat.push_str("        i32.store8\n");
        wat.push_str("        local.get $__cat_i\n");
        wat.push_str("        i32.const 1\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        local.set $__cat_i\n");
        wat.push_str("        local.get $__cat_a\n");
        wat.push_str("        i32.const 1\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        local.set $__cat_a\n");
        wat.push_str(&format!("        br $cat_b_loop_{}\n", id));
        wat.push_str("      end\n");
        wat.push_str("    end\n");
        // NUL terminate
        wat.push_str("    local.get $__cat_dst\n");
        wat.push_str("    local.get $__cat_i\n");
        wat.push_str("    i32.add\n");
        wat.push_str("    i32.const 0\n");
        wat.push_str("    i32.store8\n");
        // heap += total_len + 1
        wat.push_str("    local.get $__cat_dst\n");
        wat.push_str("    local.get $__cat_i\n");
        wat.push_str("    i32.add\n");
        wat.push_str("    i32.const 1\n");
        wat.push_str("    i32.add\n");
        wat.push_str("    global.set $heap\n");
        wat.push_str("    local.get $__cat_dst\n");
        Ok(wat)
    }

    /// Copy s[start..end) to bump heap; leave new string pointer (i32) on stack.
    fn emit_str_slice(
        recv: &Expression,
        start: &Expression,
        end: &Expression,
        locals: &BTreeMap<String, &'static str>,
    ) -> Result<String> {
        for k in ["__slice_src", "__slice_dst", "__slice_i", "__slice_n"] {
            if !locals.contains_key(k) {
                bail!("internal: .str_slice missing scratch local {}", k);
            }
        }
        static SLICE_LAB: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let id = SLICE_LAB.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut wat = String::new();
        // src ptr
        wat.push_str(&Self::emit_expr(recv, locals)?);
        wat.push_str("    local.set $__slice_src\n");
        // n = end - start (as i32)
        wat.push_str(&Self::emit_expr(end, locals)?);
        wat.push_str(&Self::emit_expr(start, locals)?);
        wat.push_str("    i64.sub\n");
        wat.push_str("    i32.wrap_i64\n");
        wat.push_str("    local.set $__slice_n\n");
        // dst = heap; heap += n+1
        wat.push_str("    global.get $heap\n");
        wat.push_str("    local.set $__slice_dst\n");
        wat.push_str("    global.get $heap\n");
        wat.push_str("    local.get $__slice_n\n");
        wat.push_str("    i32.const 1\n");
        wat.push_str("    i32.add\n");
        wat.push_str("    i32.add\n");
        wat.push_str("    global.set $heap\n");
        // i = 0
        wat.push_str("    i32.const 0\n");
        wat.push_str("    local.set $__slice_i\n");
        wat.push_str(&format!("    block $slice_done_{}\n", id));
        wat.push_str(&format!("      loop $slice_loop_{}\n", id));
        wat.push_str("        local.get $__slice_i\n");
        wat.push_str("        local.get $__slice_n\n");
        wat.push_str("        i32.ge_u\n");
        wat.push_str(&format!("        br_if $slice_done_{}\n", id));
        // dst[i] = src[start+i]
        wat.push_str("        local.get $__slice_dst\n");
        wat.push_str("        local.get $__slice_i\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        local.get $__slice_src\n");
        wat.push_str(&Self::emit_expr(start, locals)?);
        wat.push_str("        i32.wrap_i64\n");
        wat.push_str("        local.get $__slice_i\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        i32.load8_u\n");
        wat.push_str("        i32.store8\n");
        wat.push_str("        local.get $__slice_i\n");
        wat.push_str("        i32.const 1\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        local.set $__slice_i\n");
        wat.push_str(&format!("        br $slice_loop_{}\n", id));
        wat.push_str("      end\n");
        wat.push_str("    end\n");
        // NUL terminate
        wat.push_str("    local.get $__slice_dst\n");
        wat.push_str("    local.get $__slice_n\n");
        wat.push_str("    i32.add\n");
        wat.push_str("    i32.const 0\n");
        wat.push_str("    i32.store8\n");
        wat.push_str("    local.get $__slice_dst\n");
        Ok(wat)
    }

    /// Emit pure-WAT byte length of a NUL-terminated string (leave i64 on stack).
    /// Scratch locals `$__strlen_p` / `$__strlen_i` are declared by collect when needed.
    fn emit_string_len(
        recv: &Expression,
        locals: &BTreeMap<String, &'static str>,
    ) -> Result<String> {
        if !locals.contains_key("__strlen_p") || !locals.contains_key("__strlen_i") {
            bail!("internal: string .len missing scratch locals");
        }
        static STRLEN_LAB: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let id = STRLEN_LAB.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut wat = String::new();
        wat.push_str(&Self::emit_expr(recv, locals)?);
        wat.push_str("    local.set $__strlen_p\n");
        wat.push_str("    i64.const 0\n");
        wat.push_str("    local.set $__strlen_i\n");
        wat.push_str(&format!("    block $strlen_done_{}\n", id));
        wat.push_str(&format!("      loop $strlen_loop_{}\n", id));
        wat.push_str("        local.get $__strlen_p\n");
        wat.push_str("        local.get $__strlen_i\n");
        wat.push_str("        i32.wrap_i64\n");
        wat.push_str("        i32.add\n");
        wat.push_str("        i32.load8_u\n");
        wat.push_str("        i32.eqz\n");
        wat.push_str(&format!("        br_if $strlen_done_{}\n", id));
        wat.push_str("        local.get $__strlen_i\n");
        wat.push_str("        i64.const 1\n");
        wat.push_str("        i64.add\n");
        wat.push_str("        local.set $__strlen_i\n");
        wat.push_str(&format!("        br $strlen_loop_{}\n", id));
        wat.push_str("      end\n");
        wat.push_str("    end\n");
        wat.push_str("    local.get $__strlen_i\n");
        Ok(wat)
    }

    fn emit_function(func: &FunctionDecl, aliases: &std::collections::HashMap<String, Type>) -> Result<String> {
        let mut f_wat = String::new();
        let is_main = func.name == "main";

        // Reject capability parameters — the WASM subset has no IO model.
        for p in &func.params {
            let resolved = p.param_type.resolve_alias(aliases);
            match resolved {
                Type::NetCap | Type::FsCap | Type::SysCap | Type::EnvCap => {
                    bail!(
                        "WASM backend does not support capability parameters in '{}' (parameter '{}'). \
                         Use `ooda run` for programs that exercise capability IO.",
                        func.name,
                        p.name
                    );
                }
                Type::String => {}
                Type::Option(_) | Type::Result(_, _) => bail!(
                    "WASM backend does not yet support Option/Result in '{}'. Use `ooda run`.",
                    func.name
                ),
                Type::List(inner) => {
                    Self::require_list_supported(inner.as_ref(), &func.name)?;
                }
                Type::Struct { .. } => {}
                Type::Float | Type::Int | Type::Bool | Type::Void => {}
                Type::Custom(_) => {}
            }
        }
        if let Type::List(inner) = &func.return_type {
            Self::require_list_supported(inner.as_ref(), &func.name)?;
        }
        // String returns are allowed (i32 data offset — not a full string object).

        // Helper: OODA `Type` → wasm primitive type name.
        let wat_param_ty = |t: &Type| -> Result<&'static str> {
            let resolved = t.resolve_alias(aliases);
            Ok(match resolved {
                Type::Int | Type::Bool => "i64",
                Type::String => "i32",
                Type::Float => "f64",
                Type::Void => "i64",
                Type::List(inner) => {
                    Self::require_list_supported(inner.as_ref(), "param")?;
                    if Self::is_type_string(inner.as_ref()) {
                        "list_str"
                    } else {
                        "list" // semantic tag; WAT storage is i32 pointer
                    }
                }
                Type::Struct { .. } => "i32",
                Type::Custom(_) => "i32",
                _ => bail!("unsupported param type in WASM: {:?}", t),
            })
        };

        // Header
        f_wat.push_str(&format!("  (func ${}", func.name));
        if is_main {
            f_wat.push_str(" (export \"main\")");
        }
        for param in &func.params {
            let ty = wat_param_ty(&param.param_type)?;
            f_wat.push_str(&format!(
                " (param ${} {})",
                param.name,
                Self::wat_storage_ty(ty)
            ));
        }
        // The function's return type. `main` is special: the wasm
        // host entry-point returns i32, so we wrap i64 → i32 at the
        // end (and let f64 fall through unchanged).
        let ret_ty: &'static str = match &func.return_type {
            Type::Int | Type::Bool => "i64",
            Type::String => "i32",
            Type::Float => "f64",
            Type::Void => "i64",
            Type::List(_) => "list",
            Type::Custom(name) => Box::leak(name.clone().into_boxed_str()),
            Type::Struct { .. } => "i32",
            _ => "i64",
        };
        if is_main {
            f_wat.push_str(" (result i32)\n");
        } else {
            f_wat.push_str(&format!(" (result {})\n", Self::wat_storage_ty(ret_ty)));
        }

        // Collect locals including nested while/if (for-list desugar binds loop vars inside while).
        let mut locals: BTreeMap<String, &'static str> = BTreeMap::new();
        for p in &func.params {
            locals.insert(p.name.clone(), wat_param_ty(&p.param_type)?);
        }
        Self::collect_locals_in_block(&func.body, &mut locals);
        for (name, ty) in &locals {
            // Params already appear as (param …); re-declaring as local is invalid.
            if func.params.iter().any(|p| p.name == *name) {
                continue;
            }
            f_wat.push_str(&format!(
                "    (local ${} {})\n",
                name,
                Self::wat_storage_ty(ty)
            ));
        }

        // Walk statements, emitting body
        let mut emitted_return = false;
        for stmt in &func.body.stmts {
            match stmt {
                Statement::Let { name, init, .. } => {
                    let e_wat = Self::emit_expr(init, &locals)?;
                    f_wat.push_str(&e_wat);
                    f_wat.push_str(&format!("    local.set ${}\n", name));
                }
                Statement::FieldAssign { object, field, value, .. } => {
                    let recv_ty = Self::infer_expr_type(object, &locals);
                    let mut offset = None;
                    
                    WASM_STRUCTS.with(|s| {
                        if let Some(fields) = s.borrow().get(recv_ty) {
                            if let Some(idx) = fields.iter().position(|f| f == field) {
                                offset = Some(idx * 8);
                            }
                        }
                    });
                    
                    if let Some(off) = offset {
                        f_wat.push_str(&Self::emit_expr(object, &locals)?);
                        f_wat.push_str(&Self::emit_expr(value, &locals)?);
                        let ty = Self::infer_expr_type(value, &locals);
                        if ty != "i64" && ty != "f64" {
                            f_wat.push_str("    i64.extend_i32_u\n");
                        } else if ty == "f64" {
                            f_wat.push_str("    i64.reinterpret_f64\n");
                        }
                        f_wat.push_str(&format!("    i64.store offset={}\n", off));
                    } else {
                        bail!(
                            "WASM backend could not find field '{}' on type '{}'",
                            field, recv_ty
                        );
                    }
                }
                Statement::Assign { name, value, .. } => {
                    if !locals.contains_key(name) {
                        // Allow assign to params (also addressable as locals in WASM)
                    }
                    let e_wat = Self::emit_expr(value, &locals)?;
                    f_wat.push_str(&e_wat);
                    f_wat.push_str(&format!("    local.set ${}\n", name));
                }
                Statement::Return(Some(expr), _) => {
                    let e_wat = Self::emit_expr(expr, &locals)?;
                    f_wat.push_str(&e_wat);
                    if is_main {
                        f_wat.push_str("    i32.wrap_i64\n");
                    }
                    f_wat.push_str("    return\n");
                    emitted_return = true;
                }
                Statement::Return(None, _) => {
                    if is_main {
                        f_wat.push_str("    i32.const 0\n");
                        f_wat.push_str("    return\n");
                    } else {
                        f_wat.push_str("    return\n");
                    }
                    emitted_return = true;
                }
                other => {
                    // Let / assign / expr / while / break / continue
                    f_wat.push_str(&Self::emit_stmt_wat(other, &locals)?);
                }
            }
        }

        // Tail expression (if any)
        if let Some(body_expr) = &func.body.expr {
            let e_wat = Self::emit_expr(body_expr, &locals)?;
            f_wat.push_str(&e_wat);
            if is_main {
                f_wat.push_str("    i32.wrap_i64\n");
            }
            f_wat.push_str("    return\n");
            emitted_return = true;
        }

        if !emitted_return {
            if is_main {
                f_wat.push_str("    i32.const 0\n    return\n");
            } else {
                f_wat.push_str("    return\n");
            }
        }

        f_wat.push_str("  )\n");
        Ok(f_wat)
    }

    /// Emit WAT for an expression.
    ///
    /// Returns the WAT fragment that leaves the expression's value on
    /// the wasm stack. The caller does NOT need to know the result
    /// type — the existing WASM ops are polymorphic over i64 / f64
    /// in the relevant slots (e.g. `local.set` accepts both). Where
    /// the type DOES matter (Binary ops, Call to typed params), the
    /// fragment internally handles promotion via `f64.convert_i64_s`.
    fn emit_expr(expr: &Expression, locals: &BTreeMap<String, &'static str>) -> Result<String> {
        let mut wat = String::new();
        match expr {
            Expression::Literal(Literal::Int(n), _) => {
                wat.push_str(&format!("    i64.const {}\n", n));
            }
            Expression::Literal(Literal::Bool(b), _) => {
                wat.push_str(&format!("    i64.const {}\n", if *b { 1 } else { 0 }));
            }
            Expression::Literal(Literal::String(s), _) => {
                let offset = intern_string(s);
                wat.push_str(&format!("    i32.const {}\n", offset));
            }
            Expression::Literal(Literal::Float(f), _) => {
                wat.push_str(&format!("    f64.const {}\n", f));
            }
            Expression::Literal(Literal::Void, _) => {
                wat.push_str("    i64.const 0\n");
            }
            Expression::Variable(name, _) => {
                // local.get is type-polymorphic — works for both i64 and f64.
                wat.push_str(&format!("    local.get ${}\n", name));
            }
            Expression::Binary { op, left, right, .. } => {
                // Infer operand types. Prefer the declared type of a
                // local; fall back to the literal shape for fresh
                // expressions.
                let lhs_ty = Self::infer_expr_type(left, locals);
                let rhs_ty = Self::infer_expr_type(right, locals);
                let either_str = lhs_ty == "i32" || rhs_ty == "i32";
                let either_list = lhs_ty == "list" || rhs_ty == "list" || lhs_ty == "list_str" || rhs_ty == "list_str";
                // String/list pointers are not numbers: refuse arithmetic / ordering.
                // String Eq/Neq → $streq (content). List Eq/Neq → $list_eq (deep Int content).
                if either_str {
                    match op {
                        BinOp::Eq | BinOp::Neq => {
                            if lhs_ty != "i32" || rhs_ty != "i32" {
                                bail!(
                                    "WASM backend does not mix String pointers with numeric types in binary ops; use `ooda run`."
                                );
                            }
                        }
                        BinOp::Add => {
                            // String + String → bump-heap concat (pure WAT). No silent pointer math.
                            if lhs_ty != "i32" || rhs_ty != "i32" {
                                bail!(
                                    "WASM string concat requires String + String (got {} + {}); use `ooda run`.",
                                    lhs_ty,
                                    rhs_ty
                                );
                            }
                            wat.push_str(&Self::emit_str_concat(left, right, locals)?);
                            return Ok(wat);
                        }
                        BinOp::Sub | BinOp::Mul | BinOp::Div => {
                            bail!(
                                "WASM backend does not lower string arithmetic (`{:?}`); \
                                 use `ooda run` for numeric conversion (no silent pointer math).",
                                op
                            );
                        }
                        BinOp::Gt | BinOp::Lt | BinOp::Gte | BinOp::Lte => {
                            bail!(
                                "WASM backend does not lower ordered compare on String pointers; use `ooda run`."
                            );
                        }
                        BinOp::And | BinOp::Or => {
                            bail!("WASM backend does not lower &&/|| on String; use `ooda run`.");
                        }
                        BinOp::DotDot | BinOp::DotDotEq => {
                            bail!("WASM backend does not yet lower range operators (`..`, `..=`). Use `ooda run`.")
                        }
                    }
                }
                if either_list {
                    match op {
                        BinOp::Eq | BinOp::Neq => {
                            // Homogeneous only: Int lists use $list_eq (i64 content);
                            // String lists use $list_str_eq (streq per element — not pointer eq).
                            let both_int = lhs_ty == "list" && rhs_ty == "list";
                            let both_str = lhs_ty == "list_str" && rhs_ty == "list_str";
                            if !both_int && !both_str {
                                bail!(
                                    "WASM backend does not mix List kinds in ==/!= (got {} vs {}); use `ooda run`.",
                                    lhs_ty,
                                    rhs_ty
                                );
                            }
                        }
                        _ => bail!(
                            "WASM backend does not lower {:?} on List pointers; use list_get/list_len.",
                            op
                        ),
                    }
                }
                // Promote i64 → f64 if either operand is f64.
                let promote = |ty: &'static str, code: &mut String| {
                    if ty == "i64" && (lhs_ty == "f64" || rhs_ty == "f64") {
                        code.push_str("    f64.convert_i64_s\n");
                    }
                };
                let l_wat = Self::emit_expr(left, locals)?;
                let mut l_code = String::new();
                promote(lhs_ty, &mut l_code);
                wat.push_str(&l_wat);
                wat.push_str(&l_code);

                let r_wat = Self::emit_expr(right, locals)?;
                let mut r_code = String::new();
                promote(rhs_ty, &mut r_code);
                wat.push_str(&r_wat);
                wat.push_str(&r_code);

                // Both operands are now on the stack as the same type.
                let ty = if lhs_ty == "f64" || rhs_ty == "f64" {
                    "f64"
                } else if either_str || either_list {
                    "i32"
                } else {
                    "i64"
                };
                match op {
                    BinOp::Add => wat.push_str(&format!("    {}.add\n", ty)),
                    BinOp::Sub => wat.push_str(&format!("    {}.sub\n", ty)),
                    BinOp::Mul => wat.push_str(&format!("    {}.mul\n", ty)),
                    BinOp::Div if ty == "i64" => wat.push_str("    i64.div_s\n"),
                    BinOp::Div => wat.push_str("    f64.div\n"),
                    // Comparisons yield i32 in WebAssembly; extend to i64 Bool model.
                    // String: $streq content. List[Int]: $list_eq i64 elements.
                    // List[String]: $list_str_eq (streq each element — matches interpreter content eq).
                    BinOp::Eq => {
                        if either_str {
                            wat.push_str("    call $streq\n");
                        } else if either_list {
                            if lhs_ty == "list_str" {
                                wat.push_str("    call $list_str_eq\n");
                            } else {
                                wat.push_str("    call $list_eq\n");
                            }
                        } else {
                            wat.push_str(&format!("    {}.eq\n", ty));
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Neq => {
                        if either_str {
                            wat.push_str("    call $streq\n");
                            wat.push_str("    i32.eqz\n");
                        } else if either_list {
                            if lhs_ty == "list_str" {
                                wat.push_str("    call $list_str_eq\n");
                            } else {
                                wat.push_str("    call $list_eq\n");
                            }
                            wat.push_str("    i32.eqz\n");
                        } else {
                            wat.push_str(&format!("    {}.ne\n", ty));
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Gt => {
                        if ty == "i64" {
                            wat.push_str("    i64.gt_s\n")
                        } else {
                            wat.push_str("    f64.gt\n")
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Lt => {
                        if ty == "i64" {
                            wat.push_str("    i64.lt_s\n")
                        } else {
                            wat.push_str("    f64.lt\n")
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Gte => {
                        if ty == "i64" {
                            wat.push_str("    i64.ge_s\n")
                        } else {
                            wat.push_str("    f64.ge\n")
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Lte => {
                        if ty == "i64" {
                            wat.push_str("    i64.le_s\n")
                        } else {
                            wat.push_str("    f64.le\n")
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    // Boolean ops stay i64.
                    BinOp::And | BinOp::Or if ty == "i64" => match op {
                        BinOp::And => wat.push_str("    i64.and\n"),
                        BinOp::Or => wat.push_str("    i64.or\n"),
                        _ => unreachable!(),
                    },
                    BinOp::And | BinOp::Or => bail!(
                        "WASM backend does not yet lower {} on Float operands in this alpha.",
                        match op {
                            BinOp::And => "&&",
                            BinOp::Or => "||",
                            _ => "?"
                        }
                    ),
                    BinOp::DotDot | BinOp::DotDotEq => {
                        bail!("WASM backend does not yet lower range operators (`..`, `..=`). Use `ooda run`.")
                    }
                }
            }
            Expression::Call { name, args, .. } => {
                // List methods on List[Int] → list_*; String methods → WAT/host.
                if name == ".push"
                    || name == ".len"
                    || name == ".char_at"
                    || name == ".contains"
                    || name == ".str_slice"
                {
                    if args.is_empty() {
                        bail!("WASM method '{}' requires a receiver", name);
                    }
                    let recv_ty = Self::infer_expr_type(&args[0], locals);
                    if name == ".push" {
                        if recv_ty != "list" && recv_ty != "list_str" {
                            bail!("WASM .push requires List receiver");
                        }
                        if args.len() != 2 {
                            bail!("WASM .push expects receiver + one element");
                        }
                        let elem_ty = Self::infer_expr_type(&args[1], locals);
                        wat.push_str(&Self::emit_expr(&args[0], locals)?);
                        wat.push_str(&Self::emit_expr(&args[1], locals)?);
                        // list_str always stores i32 string ptrs as i64 slots; untyped `list`
                        // that typecheck refined to String also extends (dual-engine honesty).
                        if (recv_ty == "list_str" || recv_ty == "list") && elem_ty == "i32" {
                            wat.push_str("    i64.extend_i32_u\n");
                        } else if recv_ty == "list" && elem_ty == "i64" {
                            // List[Int]
                        } else {
                            bail!(
                                "WASM .push type mismatch (recv {}, elem {}); List[Int] needs Int, List[String] needs String.",
                                recv_ty,
                                elem_ty
                            );
                        }
                        wat.push_str("    call $list_push\n");
                    } else if name == ".len" {
                        if args.len() != 1 {
                            bail!("WASM .len expects only a receiver");
                        }
                        // List[Int] and List[String] share header layout — same $list_len (zero-cost).
                        if recv_ty == "list" || recv_ty == "list_str" {
                            wat.push_str(&Self::emit_expr(&args[0], locals)?);
                            wat.push_str("    call $list_len\n");
                        } else if recv_ty == "i32" {
                            wat.push_str(&Self::emit_string_len(&args[0], locals)?);
                        } else {
                            bail!(
                                "WASM .len requires List[Int], List[String], or String receiver (got {}); use `ooda run`.",
                                recv_ty
                            );
                        }
                    } else if name == ".char_at" {
                        // .char_at(index) on String → i64 byte value (ASCII subset)
                        if recv_ty != "i32" {
                            bail!(
                                "WASM .char_at requires String receiver (got {}); use `ooda run`.",
                                recv_ty
                            );
                        }
                        if args.len() != 2 {
                            bail!("WASM .char_at expects receiver + Int index");
                        }
                        let idx_ty = Self::infer_expr_type(&args[1], locals);
                        if idx_ty != "i64" {
                            bail!("WASM .char_at index must be Int (got {})", idx_ty);
                        }
                        wat.push_str(&Self::emit_expr(&args[0], locals)?);
                        wat.push_str(&Self::emit_expr(&args[1], locals)?);
                        wat.push_str("    i32.wrap_i64\n");
                        wat.push_str("    i32.add\n");
                        wat.push_str("    i32.load8_u\n");
                        wat.push_str("    i64.extend_i32_u\n");
                    } else if name == ".contains" {
                        // .contains(needle) on String → host str_contains → Bool i64
                        if recv_ty != "i32" {
                            bail!(
                                "WASM .contains requires String receiver (got {}); use `ooda run`.",
                                recv_ty
                            );
                        }
                        if args.len() != 2 {
                            bail!("WASM .contains expects receiver + String needle");
                        }
                        let needle_ty = Self::infer_expr_type(&args[1], locals);
                        if needle_ty != "i32" {
                            bail!("WASM .contains needle must be String (got {})", needle_ty);
                        }
                        wat.push_str(&Self::emit_expr(&args[0], locals)?);
                        wat.push_str(&Self::emit_expr(&args[1], locals)?);
                        wat.push_str("    call $str_contains\n");
                        wat.push_str("    i64.extend_i32_u\n");
                    } else {
                        // .str_slice(start, end) exclusive end → new NUL string on $heap
                        if recv_ty != "i32" {
                            bail!(
                                "WASM .str_slice requires String receiver (got {}); use `ooda run`.",
                                recv_ty
                            );
                        }
                        if args.len() != 3 {
                            bail!("WASM .str_slice expects receiver + start + end Ints");
                        }
                        if Self::infer_expr_type(&args[1], locals) != "i64"
                            || Self::infer_expr_type(&args[2], locals) != "i64"
                        {
                            bail!("WASM .str_slice start/end must be Int");
                        }
                        wat.push_str(&Self::emit_str_slice(
                            &args[0], &args[1], &args[2], locals,
                        )?);
                    }
                } else if name.starts_with('.') {
                    if args.len() != 1 {
                        bail!("Field access '{}' expects exactly one argument (the receiver)", name);
                    }
                    let field_name = &name[1..];
                    let recv_ty = Self::infer_expr_type(&args[0], locals);
                    let mut offset = None;
                    
                    WASM_STRUCTS.with(|s| {
                        if let Some(fields) = s.borrow().get(recv_ty) {
                            if let Some(idx) = fields.iter().position(|f| f == field_name) {
                                offset = Some(idx * 8);
                            }
                        }
                    });
                    
                    if let Some(off) = offset {
                        wat.push_str(&Self::emit_expr(&args[0], locals)?);
                        wat.push_str(&format!("    i64.load offset={}\n", off));
                    } else {
                        bail!(
                            "WASM backend could not find field '{}' on type '{}'",
                            field_name, recv_ty
                        );
                    }
                } else if name == "println" {
                    if args.is_empty() {
                        bail!("WASM println requires at least one Int or String argument");
                    }
                    for arg in args {
                        // Int → $println (i64); String (i32 offset) → $println_str; Float truncates.
                        let arg_ty = Self::infer_expr_type(arg, locals);
                        wat.push_str(&Self::emit_expr(arg, locals)?);
                        if arg_ty == "f64" {
                            wat.push_str("    i64.trunc_f64_s\n");
                            wat.push_str("    call $println\n");
                        } else if arg_ty == "i32" {
                            wat.push_str("    call $println_str\n");
                        } else if arg_ty == "list" {
                            bail!("WASM println cannot print List; use list_get/list_len");
                        } else {
                            wat.push_str("    call $println\n");
                        }
                    }
                } else {
                    let mut push_extend_str = false;
                    let mut get_wrap_str = false;
                    if name == "list_push" && args.len() >= 2 {
                        let recv_ty = Self::infer_expr_type(&args[0], locals);
                        let elem_ty = Self::infer_expr_type(&args[1], locals);
                        if (recv_ty == "list_str" || recv_ty == "list") && elem_ty == "i32" {
                            push_extend_str = true;
                        } else if recv_ty == "list" && elem_ty == "i64" {
                            // List[Int]
                        } else {
                            bail!(
                                "WASM list_push type mismatch (recv {}, elem {}); \
                                 List[Int] needs Int elements, List[String] needs String.",
                                recv_ty,
                                elem_ty
                            );
                        }
                    } else if name == "list_get" && !args.is_empty() {
                        let recv_ty = Self::infer_expr_type(&args[0], locals);
                        if recv_ty == "list_str" {
                            get_wrap_str = true;
                        }
                    }
                    for (i, arg) in args.iter().enumerate() {
                        wat.push_str(&Self::emit_expr(arg, locals)?);
                        if name == "list_push" && push_extend_str && i == 1 {
                            wat.push_str("    i64.extend_i32_u\n");
                        }
                    }
                    wat.push_str(&format!("    call ${}\n", name));
                    if name == "list_get" && get_wrap_str {
                        wat.push_str("    i32.wrap_i64\n");
                    }
                }
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                wat.push_str(&Self::emit_if(cond, then_branch, else_branch.as_ref(), locals)?);
            }
            Expression::Unary { op, expr, .. } => {
                let inner_ty = Self::infer_expr_type(expr, locals);
                wat.push_str(&Self::emit_expr(expr, locals)?);
                match op {
                    UnaryOp::Not => {
                        // `!` on Int → eqz (i32). On Float → f64.ne (i32).
                        // Bools are stored as i64 0/1; extend so `let x = !false` typechecks in WAT.
                        match inner_ty {
                            "i64" => {
                                wat.push_str("    i64.eqz\n");
                                wat.push_str("    i64.extend_i32_u\n");
                            }
                            "f64" => {
                                wat.push_str("    f64.const 0.0\n");
                                wat.push_str("    f64.ne\n");
                                wat.push_str("    i64.extend_i32_u\n");
                            }
                            _ => {
                                wat.push_str("    i64.eqz\n");
                                wat.push_str("    i64.extend_i32_u\n");
                            }
                        }
                    }
                    UnaryOp::Neg => match inner_ty {
                        "i64" => {
                            wat.push_str("    i64.const -1\n");
                            wat.push_str("    i64.mul\n");
                        }
                        "f64" => {
                            wat.push_str("    f64.const -1.0\n");
                            wat.push_str("    f64.mul\n");
                        }
                        _ => {
                            wat.push_str("    i64.const -1\n");
                            wat.push_str("    i64.mul\n");
                        }
                    },
                }
            }
            Expression::While { cond, body, .. } => {
                wat.push_str(&Self::emit_while(cond, body, locals)?);
            }
            Expression::Match { expr, arms, .. } => {
                let mut wat = String::new();
                let cond = Self::emit_expr(expr, locals)?;
                let cty = Self::infer_expr_type(expr, locals);
                
                // Find return type of the match by looking at the first arm body
                let ret_ty = arms.first().map(|a| Self::infer_expr_type(&a.body, locals)).unwrap_or("i64");
                let ret_storage = Self::wat_storage_ty(ret_ty);
                
                // Unique label from source span (no heap loop-stack for match).
                let lbl = format!("match_{}_{}", expr.span().line, expr.span().col);
                
                // We need a scratch local for the condition value. We can just push it and compare?
                // But wait, if we push it, `if` consumes it. If `if` fails, the value is gone!
                // So we MUST use a local! We don't have a unique local generator.
                // `tmp_match_cond` is fine if matches don't evaluate other matches in their condition.
                // Nested matches in arms are fine because the condition is evaluated BEFORE the arm.
                // But nested matches inside the condition itself are also evaluated before the outer condition is set!
                let tmp = "__match_tmp";
                
                wat.push_str(&cond);
                wat.push_str(&format!("    local.set ${}\n", tmp));
                wat.push_str(&format!("    block ${} (result {})\n", lbl, ret_storage));
                
                for arm in arms {
                    match &arm.pattern {
                        Pattern::Literal(lit) => {
                            wat.push_str(&format!("    local.get ${}\n", tmp));
                            let dummy_expr = Expression::Literal(lit.clone(), expr.span().clone());
                            wat.push_str(&Self::emit_expr(&dummy_expr, locals)?);
                            if cty == "f64" {
                                wat.push_str("    f64.eq\n");
                            } else if cty == "i32" || cty == "list" || cty == "list_str" {
                                wat.push_str("    call $list_eq\n");
                            } else {
                                wat.push_str("    i64.eq\n");
                            }
                            wat.push_str("    if\n");
                            wat.push_str(&Self::emit_expr(&arm.body, locals)?);
                            // Extend return type if needed? 
                            // We assume body returns exact correct storage type (since it's strongly typed).
                            wat.push_str(&format!("    br ${}\n", lbl));
                            wat.push_str("    end\n");
                        }
                        Pattern::Variant { name, arg } => {
                            if name == "Ok" || name == "Err" {
                                // Result match lowering: we don't have variants natively in WASM yet
                                // Fallback for now: we just evaluate body.
                                if let Some(arg_name) = arg {
                                    wat.push_str(&format!("    local.get ${}\n", tmp));
                                    wat.push_str(&format!("    local.set ${}\n", arg_name));
                                }
                                wat.push_str(&Self::emit_expr(&arm.body, locals)?);
                                wat.push_str(&format!("    br ${}\n", lbl));
                            } else {
                                // Custom variants unsupported in WASM
                                wat.push_str(&Self::emit_expr(&arm.body, locals)?);
                                wat.push_str(&format!("    br ${}\n", lbl));
                            }
                        }
                        Pattern::Wildcard => {
                            wat.push_str(&Self::emit_expr(&arm.body, locals)?);
                            wat.push_str(&format!("    br ${}\n", lbl));
                        }
                    }
                }
                
                // Fallback dummy return (typechecker should enforce exhaustive)
                wat.push_str(&format!("    {}.const 0\n", ret_storage));
                wat.push_str("    ;; dummy fallback\n");
                wat.push_str("    end\n");
            }
            Expression::StructLit { name, fields, .. } => {
                let struct_def = WASM_STRUCTS.with(|s| s.borrow().get(name).cloned());
                let struct_fields = struct_def.unwrap_or_default();
                let size = struct_fields.len() * 8;
                
                wat.push_str("    global.get $heap\n");
                
                for (field_name, e) in fields {
                    let idx = struct_fields.iter().position(|f| f == field_name).unwrap_or(0);
                    let offset = idx * 8;
                    wat.push_str("    global.get $heap\n");
                    wat.push_str(&Self::emit_expr(e, locals)?);
                    let ty = Self::infer_expr_type(e, locals);
                    if ty != "i64" && ty != "f64" {
                        wat.push_str("    i64.extend_i32_u\n");
                    } else if ty == "f64" {
                        wat.push_str("    i64.reinterpret_f64\n");
                    }
                    wat.push_str(&format!("    i64.store offset={}\n", offset));
                }
                
                wat.push_str("    global.get $heap\n");
                wat.push_str(&format!("    i32.const {}\n", if size == 0 { 8 } else { size }));
                wat.push_str("    i32.add\n");
                wat.push_str("    global.set $heap\n");
            }
        }
        Ok(wat)
    }

    /// Best-effort type inference for an expression: declared local
    /// type > literal shape > default i64.
    fn infer_expr_type(expr: &Expression, locals: &BTreeMap<String, &'static str>) -> &'static str {
        match expr {
            Expression::Literal(Literal::Float(_), _) => "f64",
            Expression::Literal(Literal::Bool(_), _) => "i64",
            Expression::Literal(Literal::Int(_), _) => "i64",
            Expression::Literal(Literal::String(_), _) => "i32",
            Expression::StructLit { name, .. } => Box::leak(name.clone().into_boxed_str()),
            Expression::Variable(name, _) => locals.get(name).copied().unwrap_or("i64"),
            Expression::Binary { op, left, right, .. } => {
                // Comparisons always yield Bool (i64 0/1). Arithmetic preserves operand class.
                match op {
                    BinOp::Eq
                    | BinOp::Neq
                    | BinOp::Gt
                    | BinOp::Lt
                    | BinOp::Gte
                    | BinOp::Lte
                    | BinOp::And
                    | BinOp::Or => "i64",
                    _ => {
                        let lt = Self::infer_expr_type(left, locals);
                        let rt = Self::infer_expr_type(right, locals);
                        if lt == "f64" || rt == "f64" {
                            "f64"
                        } else if lt == "i32" || rt == "i32" {
                            "i32"
                        } else {
                            "i64"
                        }
                    }
                }
            }
            Expression::Call { name, args, .. } => {
                if name == "list_new" || name == "list_push" || name == ".push" {
                    if let Some(arg0) = args.first() {
                        let ty = Self::infer_expr_type(arg0, locals);
                        if ty == "list_str" { return "list_str"; }
                    }
                    "list"
                } else if name == "list_get" {
                    if let Some(arg0) = args.first() {
                        let ty = Self::infer_expr_type(arg0, locals);
                        if ty == "list_str" {
                            return "i32";
                        }
                    }
                    "i64"
                } else if name == "list_len" || name == ".char_at" || name == ".contains" {
                    "i64"
                } else if name == ".len" {
                    // List .len → i64; String .len → i64 (same numeric width).
                    "i64"
                } else if name == ".str_slice" {
                    "i32" // new string pointer
                } else {
                    let _ = args;
                    "i64"
                }
            }
            // Default to i64 for everything else (if, match, etc.).
            _ => "i64",
        }
    }

    /// Collect `let` bindings in a block and nested while/if (stack-only map updates).
    fn collect_locals_in_block(
        block: &Block,
        locals: &mut BTreeMap<String, &'static str>,
    ) {
        for stmt in &block.stmts {
            Self::collect_locals_in_stmt(stmt, locals);
        }
        if let Some(e) = &block.expr {
            Self::collect_locals_in_expr(e, locals);
        }
    }

    fn collect_locals_in_stmt(stmt: &Statement, locals: &mut BTreeMap<String, &'static str>) {
        match stmt {
            Statement::Let {
                name,
                init,
                type_annotation,
                ..
            } => {
                let ty = if let Some(Type::List(inner)) = type_annotation {
                    if Self::is_type_string(inner.as_ref()) { "list_str" } else { "list" }
                } else {
                    Self::infer_expr_type(init, locals)
                };
                locals.insert(name.clone(), ty);
                Self::collect_locals_in_expr(init, locals);
            }
            Statement::Assign { name, value, .. } => {
                Self::collect_locals_in_expr(value, locals);
                // Refine untyped list → list_str when assigned from push of String (matches typecheck).
                if let Expression::Call {
                    name: cname, args, ..
                } = value
                {
                    if (cname == "list_push" || cname == ".push") && args.len() >= 2 {
                        let elem_ty = Self::infer_expr_type(&args[1], locals);
                        if elem_ty == "i32" {
                            locals.insert(name.clone(), "list_str");
                        }
                    }
                }
            }
            Statement::Return(Some(value), _) => {
                Self::collect_locals_in_expr(value, locals);
            }
            Statement::Expr(e, _) => Self::collect_locals_in_expr(e, locals),
            Statement::While { cond, body, .. } => {
                Self::collect_locals_in_expr(cond, locals);
                Self::collect_locals_in_block(body, locals);
            }
            Statement::FieldAssign { object, value, .. } => {
                Self::collect_locals_in_expr(object, locals);
                Self::collect_locals_in_expr(value, locals);
            }
            Statement::Return(None, _) | Statement::Break(_) | Statement::Continue(_) => {}
        }
    }

    fn collect_locals_in_expr(expr: &Expression, locals: &mut BTreeMap<String, &'static str>) {
        match expr {
            Expression::Binary { op, left, right, .. } => {
                Self::collect_locals_in_expr(left, locals);
                Self::collect_locals_in_expr(right, locals);
                // String + String concat needs fixed scratch locals (pure-WAT bump heap).
                if matches!(op, BinOp::Add) {
                    let lt = Self::infer_expr_type(left, locals);
                    let rt = Self::infer_expr_type(right, locals);
                    if lt == "i32" && rt == "i32" {
                        locals.insert("__cat_a".into(), "i32");
                        locals.insert("__cat_b".into(), "i32");
                        locals.insert("__cat_dst".into(), "i32");
                        locals.insert("__cat_i".into(), "i32");
                    }
                }
            }
            Expression::Unary { expr, .. } => Self::collect_locals_in_expr(expr, locals),
            Expression::Call { name, args, .. } => {
                for a in args {
                    Self::collect_locals_in_expr(a, locals);
                }
                // String .len needs fixed scratch locals for pure-WAT NUL scan.
                if name == ".len" {
                    if let Some(recv) = args.first() {
                        if Self::infer_expr_type(recv, locals) == "i32" {
                            locals.insert("__strlen_p".into(), "i32");
                            locals.insert("__strlen_i".into(), "i64");
                        }
                    }
                }
                if name == ".str_slice" {
                    locals.insert("__slice_src".into(), "i32");
                    locals.insert("__slice_dst".into(), "i32");
                    locals.insert("__slice_i".into(), "i32");
                    locals.insert("__slice_n".into(), "i32");
                }
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::collect_locals_in_expr(cond, locals);
                Self::collect_locals_in_block(then_branch, locals);
                if let Some(eb) = else_branch {
                    Self::collect_locals_in_block(eb, locals);
                }
            }
            Expression::While { cond, body, .. } => {
                Self::collect_locals_in_expr(cond, locals);
                Self::collect_locals_in_block(body, locals);
            }
            Expression::Match { expr, arms, .. } => {
                Self::collect_locals_in_expr(expr, locals);
                for arm in arms {
                    Self::collect_locals_in_expr(&arm.body, locals);
                }
            }
            Expression::StructLit { fields, .. } => {
                for (_, e) in fields {
                    Self::collect_locals_in_expr(e, locals);
                }
            }
            Expression::Literal(_, _) | Expression::Variable(_, _) => {}
        }
    }

    fn emit_while(
        cond: &Expression,
        body: &Block,
        locals: &BTreeMap<String, &'static str>,
    ) -> Result<String> {
        let mut wat = String::new();
        let cond_ty = Self::infer_expr_type(cond, locals);
        static LOOP_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let id = LOOP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let br_lab = format!("break_{}", id);
        let cont_lab = format!("continue_{}", id);
        WASM_LOOP_STACK.with(|s| s.borrow_mut().push((br_lab.clone(), cont_lab.clone())));
        // while cond { body } → break when cond is *false* (zero).
        wat.push_str(&format!("    block ${}\n", br_lab));
        wat.push_str(&format!("      loop ${}\n", cont_lab));
        wat.push_str(&Self::emit_expr(cond, locals)?);
        match cond_ty {
            "f64" => {
                wat.push_str("        f64.const 0.0\n");
                wat.push_str("        f64.eq\n"); // 1 when false → break
            }
            _ => {
                wat.push_str("        i64.eqz\n");
            }
        }
        wat.push_str(&format!("        br_if ${}\n", br_lab));
        for stmt in &body.stmts {
            wat.push_str(&Self::emit_stmt_wat(stmt, locals)?);
        }
        // Parser treats a final expression without `;` as `body.expr` (e.g. idiomatic
        // `if cond { break; }` as while tail). Dropping it silently miscompiled control
        // flow — dual-engine honesty: lower tail as a statement (drop residual value).
        if let Some(tail) = &body.expr {
            wat.push_str(&Self::emit_expr(tail, locals)?);
            match &**tail {
                Expression::Call { name, .. } if name == "println" => {}
                _ => {
                    wat.push_str("        drop\n");
                }
            }
        }
        wat.push_str(&format!("        br ${}\n", cont_lab));
        wat.push_str("      end\n");
        wat.push_str("    end\n");
        WASM_LOOP_STACK.with(|s| {
            s.borrow_mut().pop();
        });
        // while is statement-only: no stack residue (do not push dummy values).
        Ok(wat)
    }

/// Lower `if cond { then } else { els }` to WAT.
    ///
    /// The condition is normalised to i32 (0 = false, non-zero = true).
    /// Both branches must produce a single value of the unified
    /// type (i64 or f64) on the stack so the outer block has a
    /// uniform `(result T)` signature.
    fn emit_if(
        cond: &Expression,
        then_branch: &Block,
        else_branch: Option<&Block>,
        locals: &BTreeMap<String, &'static str>,
    ) -> Result<String> {
        // Reject `match` (not yet lowered) anywhere in the branches.
        for stmt in &then_branch.stmts {
            if matches!(stmt, Statement::Expr(expr, _) | Statement::Return(Some(expr), _)
                if matches!(expr, Expression::Match { .. }))
            {
                bail!("WASM backend does not yet lower `match` expressions; use `ooda run`.");
            }
        }
        if let Some(eb) = else_branch {
            for stmt in &eb.stmts {
                if matches!(stmt, Statement::Expr(expr, _) | Statement::Return(Some(expr), _)
                    if matches!(expr, Expression::Match { .. }))
                {
                    bail!("WASM backend does not yet lower `match` expressions; use `ooda run`.");
                }
            }
        }

        // Determine the unified result type of the if from the
        // tail expression of each branch (or default to i64).
        let branch_ty = |b: &Block| -> &'static str {
            if let Some(tail) = &b.expr {
                Self::expr_literal_type(tail, locals)
            } else {
                // Walk statements; pick the last Return's type, else Void.
                let mut ty = "i64";
                for s in &b.stmts {
                    if let Statement::Return(Some(e), _) = s {
                        ty = Self::expr_literal_type(e, locals);
                    }
                }
                ty
            }
        };
        let result_ty = if let Some(eb) = else_branch {
            let t_ty = branch_ty(then_branch);
            let e_ty = branch_ty(eb);
            if t_ty == "f64" || e_ty == "f64" { "f64" } else { "i64" }
        } else {
            branch_ty(then_branch)
        };

        let mut wat = String::new();

        // 1. Condition, normalised to i32
        wat.push_str(&Self::emit_expr(cond, locals)?);
        wat.push_str(&format!(
            "    {}.const 0\n    {}.ne\n",
            if Self::infer_expr_type(cond, locals) == "f64" { "f64" } else { "i64" },
            if Self::infer_expr_type(cond, locals) == "f64" { "f64" } else { "i64" },
        ));

        // 2. Structured if/then/else with the unified result type.
        // Each arm must leave exactly one `result_ty` value — side-effect-only
        // arms (e.g. println) push a zero of that type so wasmtime type-checks.
        wat.push_str(&format!("    (if (result {})\n", result_ty));
        wat.push_str("      (then\n");
        wat.push_str(&Self::emit_if_branch(then_branch, locals, result_ty)?);
        wat.push_str("      )\n");
        wat.push_str("      (else\n");
        if let Some(eb) = else_branch {
            wat.push_str(&Self::emit_if_branch(eb, locals, result_ty)?);
        } else {
            wat.push_str(&format!("        {}.const 0\n", result_ty));
        }
        wat.push_str("      )\n");
        wat.push_str("    )\n");

        Ok(wat)
    }

    /// Emit one if-arm: statements + optional tail expr, else default zero.
    fn emit_if_branch(
        branch: &Block,
        locals: &BTreeMap<String, &'static str>,
        result_ty: &'static str,
    ) -> Result<String> {
        let mut wat = String::new();
        for stmt in &branch.stmts {
            wat.push_str(&Self::emit_stmt_wat(stmt, locals)?);
        }
        if let Some(tail) = &branch.expr {
            wat.push_str(&Self::emit_expr(tail, locals)?);
        } else {
            // No value-producing tail: synthesize unit for `(result T)`.
            wat.push_str(&format!("        {}.const 0\n", result_ty));
        }
        Ok(wat)
    }

    /// Type of a literal-shaped expression (ignores locals/operations).
    fn expr_literal_type(expr: &Expression, locals: &BTreeMap<String, &'static str>) -> &'static str {
        Self::infer_expr_type(expr, locals)
    }

    /// Emit WAT for one statement (used by `emit_if` for each
    /// branch). Mirrors the body-emission logic in `emit_function`.
    fn emit_stmt_wat(stmt: &Statement, locals: &BTreeMap<String, &'static str>) -> Result<String> {
        let mut wat = String::new();
        match stmt {
            Statement::Let { name, init, .. } => {
                wat.push_str(&Self::emit_expr(init, locals)?);
                wat.push_str(&format!("        local.set ${}\n", name));
            }
                Statement::FieldAssign { .. } => {
                    bail!("WASM backend does not support field assignment. Use `ooda run`.");
                }
            Statement::Assign { name, value, .. } => {
                wat.push_str(&Self::emit_expr(value, locals)?);
                wat.push_str(&format!("        local.set ${}\n", name));
            }
            Statement::Return(Some(expr), _) => {
                wat.push_str(&Self::emit_expr(expr, locals)?);
                wat.push_str("        return\n");
            }
            Statement::Break(_) => {
                let br = WASM_LOOP_STACK.with(|s| {
                    s.borrow().last().map(|(b, _)| b.clone())
                }).ok_or_else(|| anyhow::anyhow!("WASM: break outside loop"))?;
                wat.push_str(&format!("        br ${}\n", br));
            }
            Statement::Continue(_) => {
                let cont = WASM_LOOP_STACK.with(|s| {
                    s.borrow().last().map(|(_, c)| c.clone())
                }).ok_or_else(|| anyhow::anyhow!("WASM: continue outside loop"))?;
                wat.push_str(&format!("        br ${}\n", cont));
            }
            Statement::Return(None, _) => {
                wat.push_str("        return\n");
            }
            Statement::Expr(expr, _) => {
                wat.push_str(&Self::emit_expr(expr, locals)?);
                // Statement-context: drop leftover stack values.
                // println consumes its args; if/while/other exprs leave a value.
                match expr {
                    Expression::Call { name, .. } if name == "println" => {}
                    _ => {
                        wat.push_str("        drop\n");
                    }
                }
            }
            Statement::While { cond, body, .. } => {
                wat.push_str(&Self::emit_while(cond, body, locals)?);
            }
        }
        Ok(wat)
    }

    /// Structural validation: scan the emitted WAT for undeclared locals and
    /// missing `return` instructions. Optionally round-trip with
    /// `wasm-tools validate` when available.
    fn validate_wat(wat: &str) -> Result<()> {
        if wat.is_empty() {
            bail!("WASM validation failed: empty WAT output");
        }
        // Track per-function locals.
        let mut current_locals: HashSet<String> = HashSet::new();
        let mut current_params: HashSet<String> = HashSet::new();
        let mut in_function = false;
        let mut saw_return = false;

        for line in wat.lines() {
            let t = line.trim();
            if t.starts_with("(func $") {
                if in_function && !saw_return {
                    bail!("WASM validation failed: previous function missing `return`");
                }
                in_function = true;
                saw_return = false;
                current_locals.clear();
                current_params.clear();
                // Parse `(func $name (param $p1 i64) ...`
                let after_func = t.trim_start_matches("(func $");
                if let Some(space_idx) = after_func.find(' ') {
                    let mut rest = &after_func[space_idx..];
                    // Consume export if present
                    if rest.trim_start().starts_with("(export") {
                        if let Some(close) = rest.find(')') {
                            rest = &rest[close + 1..];
                        }
                    }
                    while let Some(open) = rest.find("(param $") {
                        let after_param = &rest[open + "(param $".len()..];
                        if let Some(space_idx) = after_param.find(' ') {
                            let pname = &after_param[..space_idx];
                            current_params.insert(pname.to_string());
                            rest = &after_param[space_idx..];
                        } else {
                            break;
                        }
                    }
                }
            } else if t.starts_with("(local $") {
                let after_local = t.trim_start_matches("(local $");
                if let Some(space_idx) = after_local.find(' ') {
                    let lname = &after_local[..space_idx];
                    current_locals.insert(lname.to_string());
                }
            } else if t.starts_with("local.get $") || t.starts_with("local.set $") {
                let op = if t.starts_with("local.get") { "local.get" } else { "local.set" };
                let rest = &t[op.len() + 2..]; // skip "$" + name
                let name = rest.split_whitespace().next().unwrap_or("");
                if !current_params.contains(name) && !current_locals.contains(name) {
                    bail!(
                        "WASM validation failed: {} references undeclared local/param `${}` (must be in (param ...) or (local ...))",
                        op,
                        name
                    );
                }
            } else if t == "return" {
                saw_return = true;
            }
        }
        if in_function && !saw_return {
            bail!("WASM validation failed: last function missing `return`");
        }

        // Optional external validation
        if let Ok(status) = Self::run_wasm_tools_validate(wat) {
            if !status {
                bail!("WASM validation failed: `wasm-tools validate` rejected the WAT");
            }
        }
        Ok(())
    }

    fn run_wasm_tools_validate(wat: &str) -> Result<bool> {
        let candidates = ["wasm-tools", "wasm-tools-1", "wasm-tools-2"];
        let bin = candidates
            .into_iter()
            .find(|c| Command::new(c).arg("--version").output().is_ok());
        let Some(bin) = bin else {
            return Err(anyhow!("wasm-tools not installed"));
        };

        let dir = std::env::temp_dir().join(format!("ooda-wasm-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let wat_path = dir.join("check.wat");
        std::fs::write(&wat_path, wat)?;
        let out = Command::new(bin)
            .arg("validate")
            .arg(&wat_path)
            .output()?;
        let _ = std::fs::remove_dir_all(&dir);
        Ok(out.status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(src: &str) -> Program {
        let mut l = Lexer::new(src);
        let tokens = l.tokenize().expect("lex");
        let mut p = Parser::new(tokens);
        p.parse_program().expect("parse")
    }

    #[test]
    fn emits_valid_wat_for_straight_line_int() {
        let prog = parse(
            r#"
            pub fn add(a: Int, b: Int) -> Int { return a + b; }
            pub fn main() { let x = add(2, 3); }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        assert!(wat.contains("(local $x i64)"), "wat:\n{}", wat);
        assert!(wat.contains("local.set $x"), "wat:\n{}", wat);
        assert!(wat.contains("i64.const 2"), "wat:\n{}", wat);
        assert!(wat.contains("i64.const 3"), "wat:\n{}", wat);
        assert!(wat.contains("call $add"), "wat:\n{}", wat);
    }

    #[test]
    fn accepts_string_literals_with_data_segment() {
        let prog = parse(r#"pub fn main() { let s = "hi"; }"#);
        let res = WasmCodeGen::emit_wat(&prog).unwrap();
        assert!(res.contains("(memory 1)"));
        assert!(res.contains(r#"(data (i32.const 0) "\68\69\00")"#));
    }

    #[test]
    fn interns_duplicate_string_literals_one_data_segment() {
        let prog = parse(
            r#"
pub fn main() {
    let a = "hello";
    let b = "hello";
    if a == b { println(1); } else { println(0); }
}
"#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).unwrap();
        // One data segment for "hello", not two
        let data_count = wat.matches("(data (i32.const").count();
        assert_eq!(data_count, 1, "expected single interned data segment:\n{}", wat);
        assert!(wat.contains("i32.const 0"), "both should load offset 0:\n{}", wat);
        assert!(wat.contains("call $streq"), "string == uses streq host import:\n{}", wat);
    }

    #[test]
    fn println_string_literal_uses_println_str() {
        let prog = parse(r#"pub fn main() { println("hi"); }"#);
        let wat = WasmCodeGen::emit_wat(&prog).unwrap();
        assert!(wat.contains("call $println_str"), "wat:\n{}", wat);
        assert!(wat.contains(r#"(data (i32.const 0) "\68\69\00")"#), "wat:\n{}", wat);
    }

    #[test]
    fn lowers_string_concat_on_bump_heap() {
        let prog = parse(
            r#"
pub fn main() {
    let a = "a";
    let b = "b";
    let c = a + b;
    println(c);
}
"#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit string concat");
        assert!(
            wat.contains("global.get $heap") && wat.contains("global.set $heap"),
            "concat must bump heap:\n{}",
            wat
        );
        assert!(
            wat.contains("i32.store8") && wat.contains("i32.load8_u"),
            "concat must copy bytes:\n{}",
            wat
        );
        // No host concat import — pure WAT (zero host D for this path).
        assert!(
            !wat.contains("str_concat") && !wat.contains("strcat"),
            "must not invent host strcat:\n{}",
            wat
        );
        assert!(wat.contains("call $println_str"), "println result string:\n{}", wat);
    }

    #[test]
    fn while_tail_if_break_is_not_silently_dropped() {
        // Idiomatic OODA: last stmt without `;` becomes body.expr — must still lower.
        let prog = parse(
            r#"
pub fn main() {
    let mut i = 0;
    while i < 10 {
        i = i + 1;
        if i == 3 { break; }
    }
    println(i);
}
"#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit while tail break");
        assert!(
            wat.contains("br $break_") || wat.contains("br $break"),
            "break in while tail if must lower:\n{}",
            wat
        );
        assert!(
            wat.contains("(if (result"),
            "tail if must lower:\n{}",
            wat
        );
    }

    #[test]
    fn refuses_string_sub_no_pointer_math() {
        let prog = parse(
            r#"
pub fn main() {
    let a = "a";
    let b = "b";
    let c = a - b;
    println(c);
}
"#,
        );
        let err = WasmCodeGen::emit_wat(&prog).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("string arithmetic") || msg.contains("pointer"),
            "expected refuse string -, got: {}",
            msg
        );
    }

    #[test]
    fn lowers_if_then_else_to_valid_wat() {
        let prog = parse(
            r#"
            pub fn pick(x: Int) -> Int {
                if x > 0 { return x; } else { return 0 - x; }
            }
            pub fn main() {}
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        // Both branches must produce i64 and the if must be a
        // structured (if (result i64) (then …) (else …)) block.
        assert!(wat.contains("(if (result i64)"), "wat:\n{}", wat);
        assert!(wat.contains("(then"), "wat:\n{}", wat);
        assert!(wat.contains("(else"), "wat:\n{}", wat);
    }

    #[test]
    fn lowers_if_without_else_with_default_zero_branch() {
        let prog = parse(
            r#"
            pub fn sign(x: Int) -> Int {
                if x >= 0 { return x; }
                return 0 - x;
            }
            pub fn main() {}
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        assert!(wat.contains("(if (result i64)"), "wat:\n{}", wat);
        assert!(wat.contains("i64.const 0"), "wat:\n{}", wat);
    }

    #[test]
    fn compiles_match_expressions_now() {
        let prog = parse(
            r#"
            pub fn classify(x: Int) -> Int {
                match x {
                    0 => 0,
                    1 => 1,
                    _ => 2,
                }
            }
            "#,
        );
        let res = WasmCodeGen::emit_wat(&prog);
        assert!(res.is_ok());
    }

    #[test]
    fn rejects_capability_parameters_non_zero() {
        let prog = parse(
            r#"
            pub fn fetch(net: &NetCap, url: String) -> String { return fetch(url); }
            pub fn main() {}
            "#,
        );
        let res = WasmCodeGen::emit_wat(&prog);
        assert!(res.is_err());
        assert!(format!("{}", res.unwrap_err()).contains("capability"));
    }

    #[test]
    fn emits_local_decl_for_let() {
        let prog = parse(
            r#"
            pub fn main() {
                let a = 1;
                let b = 2;
                let c = a + b;
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        assert!(wat.contains("(local $a i64)"));
        assert!(wat.contains("(local $b i64)"));
        assert!(wat.contains("(local $c i64)"));
    }

    #[test]
    fn emits_println_int() {
        let prog = parse(
            r#"
            pub fn main() {
                let x = 42;
                println(x);
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        assert!(wat.contains("call $println"), "wat:\n{}", wat);
        assert!(wat.contains("local.get $x"), "wat:\n{}", wat);
    }

    #[test]
    fn emits_wasm_for_float_arithmetic() {
        let prog = parse(
            r#"
            pub fn main() {
                let x = 1.5 + 2.5;
                let y = x * 2.0;
                println(y);
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        // Float locals are f64
        assert!(wat.contains("(local $x f64)"), "wat:\n{}", wat);
        assert!(wat.contains("(local $y f64)"), "wat:\n{}", wat);
        // Float constants and ops
        assert!(wat.contains("f64.const 1.5"), "wat:\n{}", wat);
        assert!(wat.contains("f64.const 2.5"), "wat:\n{}", wat);
        assert!(wat.contains("f64.add"), "wat:\n{}", wat);
        assert!(wat.contains("f64.mul"), "wat:\n{}", wat);
        // println is the i64 host import, so the Float value is truncated
        assert!(wat.contains("i64.trunc_f64_s"), "wat:\n{}", wat);
    }

    #[test]
    fn while_breaks_when_condition_is_false_not_true() {
        // Polarity: br_if $break must fire when cond is *false* (i64.eqz),
        // not when true (old bug inverted loops so bodies never ran).
        let prog = parse(
            r#"
            pub fn main() {
                let mut i = 0;
                while i < 3 {
                    i = i + 1;
                }
                println(i);
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit while");
        assert!(
            wat.contains("i64.eqz"),
            "while must break on false via i64.eqz:\n{}",
            wat
        );
        // Must not use the inverted "ne 0 → break on true" pattern.
        assert!(
            !wat.contains("i64.ne\n        br_if $break_"),
            "inverted while polarity must not appear:\n{}",
            wat
        );
        assert!(
            wat.contains("br_if $break_") || wat.contains("br_if $break"),
            "wat:\n{}",
            wat
        );
        assert!(
            wat.contains("br $continue_") || wat.contains("br $continue"),
            "wat:\n{}",
            wat
        );
        // Comparisons produce i32 in WASM; we extend to i64 Bool model.
        assert!(
            wat.contains("i64.extend_i32_u"),
            "compare result must extend i32→i64:\n{}",
            wat
        );
    }

    #[test]
    fn nested_while_break_uses_unique_labels() {
        // Locals must be declared at function top — declare j outside.
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let mut i = 0;
                let mut j = 0;
                while i < 2 {
                    j = 0;
                    while j < 3 {
                        if j == 1 { break; }
                        j = j + 1;
                    }
                    i = i + 1;
                }
                return i;
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit nested while");
        assert!(
            wat.matches("block $break_").count() >= 2,
            "expected unique nested break labels, got:\n{}",
            wat
        );
        assert!(
            wat.contains("br $break_"),
            "inner break must target labeled break block:\n{}",
            wat
        );
    }

    #[test]
    fn compiles_match_expressions_in_wasm() {
        let prog = parse(
            r#"
            pub fn classify(x: Int) -> Int {
                match x {
                    0 => 0,
                    1 => 1,
                    _ => 2,
                }
            }
            "#,
        );
        let res = WasmCodeGen::emit_wat(&prog);
        assert!(res.is_ok());
    }
}
