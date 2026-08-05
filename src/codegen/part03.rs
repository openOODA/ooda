impl LlvmCodeGen {

    /// Emit statements in a block (+ optional tail expr). Returns (ir, next_reg, terminated).
    /// `terminated` means every path left via ret/break/continue (no fallthrough).
    fn emit_block_stmts(
        block: &Block,
        mut reg: usize,
        locals: &std::collections::HashMap<String, &'static str>,
    ) -> Result<(String, usize, bool)> {
        let mut code = String::new();
        let mut terminated = false;
        for stmt in &block.stmts {
            if terminated {
                break;
            }
            let (sc, r, term) = Self::emit_one_stmt(stmt, reg, locals)?;
            reg = r;
            code.push_str(&sc);
            terminated = term;
        }
        if !terminated {
            if let Some(tail) = &block.expr {
                let (sc, r, term) = Self::emit_one_stmt(
                    &Statement::Expr((**tail).clone(), Span { line: 0, col: 0 }),
                    reg,
                    locals,
                )?;
                reg = r;
                code.push_str(&sc);
                terminated = term;
            }
        }
        Ok((code, reg, terminated))
    }


    /// Lower one statement. `terminated` = control does not fall through.
    fn emit_one_stmt(
        stmt: &Statement,
        mut reg: usize,
        locals: &std::collections::HashMap<String, &'static str>,
    ) -> Result<(String, usize, bool)> {
        let mut code = String::new();
        match stmt {
            Statement::Assign { name, value, .. } => {
                let (val, vcode, r2, vty) = Self::emit_expr(value, reg, locals)?;
                reg = r2;
                code.push_str(&vcode);
                let pty = locals.get(name).copied().unwrap_or(vty);
                code.push_str(&format!("  store {} {}, {}* %var_{}\n", pty, val, pty, name));
                Ok((code, reg, false))
            }
            Statement::Let { name, init, .. } => {
                let (val, vcode, r2, vty) = Self::emit_expr(init, reg, locals)?;
                reg = r2;
                code.push_str(&vcode);
                if locals.contains_key(name) {
                    code.push_str(&format!("  store {} {}, {}* %var_{}\n", vty, val, vty, name));
                } else {
                    // Nested let in while/if: stack alloca (W↓ vs heap).
                    code.push_str(&format!("  %var_{} = alloca {}\n", name, vty));
                    code.push_str(&format!("  store {} {}, {}* %var_{}\n", vty, val, vty, name));
                    // Note: cannot insert into immutable locals map here; pre-collected names preferred.
                }
                Ok((code, reg, false))
            }
            Statement::Expr(expr, _) => {
                let (_v, ecode, r2, _) = Self::emit_expr(expr, reg, locals)?;
                reg = r2;
                code.push_str(&ecode);
                Ok((code, reg, false))
            }
            Statement::Return(Some(ex), _) => {
                let (val, scode, rnext, vty) = Self::emit_expr(ex, reg, locals)?;
                reg = rnext;
                code.push_str(&scode);
                if vty == "i64" {
                    code.push_str(&format!("  ret i64 {}\n", val));
                } else if vty == "i1" {
                    let z = format!("%r{}", reg);
                    reg += 1;
                    code.push_str(&format!("  {} = zext i1 {} to i64\n", z, val));
                    code.push_str(&format!("  ret i64 {}\n", z));
                } else {
                    code.push_str(&format!("  ret i64 0\n"));
                }
                Ok((code, reg, true))
            }
            Statement::Return(None, _) => {
                code.push_str("  ret i64 0\n");
                Ok((code, reg, true))
            }
            Statement::Break(_) => {
                let end = LLVM_LOOP_STACK.with(|s| s.borrow().last().map(|(b, _)| b.clone()))
                    .ok_or_else(|| anyhow!("LLVM: break outside loop"))?;
                code.push_str(&format!("  br label %{}\n", end));
                Ok((code, reg, true))
            }
            Statement::Continue(_) => {
                let head = LLVM_LOOP_STACK.with(|s| s.borrow().last().map(|(_, c)| c.clone()))
                    .ok_or_else(|| anyhow!("LLVM: continue outside loop"))?;
                code.push_str(&format!("  br label %{}\n", head));
                Ok((code, reg, true))
            }
            Statement::While { cond, body, .. } => {
                let (wcode, r) = Self::emit_while(cond, body, reg, locals)?;
                Ok((wcode, r, false))
            }
            Statement::FieldAssign { .. } => {
                bail!(
                    "LLVM integer-subset backend does not support field assignment. Use `ooda run` or `ooda build --target c`."
                )
            }
        }
    }


    fn emit_while(
        cond: &Expression,
        body: &Block,
        mut reg: usize,
        locals: &std::collections::HashMap<String, &'static str>,
    ) -> Result<(String, usize)> {
        let mut code = String::new();
        let id = reg;
        reg += 1;
        let head = format!("while_head_{}", id);
        let body_l = format!("while_body_{}", id);
        let end = format!("while_end_{}", id);

        // break → end, continue → head (stack labels; zero heap W).
        LLVM_LOOP_STACK.with(|s| s.borrow_mut().push((end.clone(), head.clone())));

        code.push_str(&format!("  br label %{}\n", head));
        code.push_str(&format!("\n{}:\n", head));
        let (cval, ccode, r1, cty) = Self::emit_expr(cond, reg, locals)?;
        reg = r1;
        code.push_str(&ccode);
        let c_i1 = if cty == "i1" {
            cval
        } else {
            let t = format!("%r{}", reg);
            reg += 1;
            code.push_str(&format!("  {} = icmp ne i64 {}, 0\n", t, cval));
            t
        };
        code.push_str(&format!("  br i1 {}, label %{}, label %{}\n", c_i1, body_l, end));
        code.push_str(&format!("\n{}:\n", body_l));
        // stmts + body.expr tail (idiomatic if/break without trailing `;`).
        let (body_code, r2, body_term) = Self::emit_block_stmts(body, reg, locals)?;
        reg = r2;
        code.push_str(&body_code);
        if !body_term {
            code.push_str(&format!("  br label %{}\n", head));
        }
        code.push_str(&format!("\n{}:\n", end));
        LLVM_LOOP_STACK.with(|s| {
            s.borrow_mut().pop();
        });
        Ok((code, reg))
    }

}
