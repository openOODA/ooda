impl WasmCodeGen {

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

}
