impl WasmCodeGen {

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
            Expression::Match { expr: scrut, arms, .. } => {
                Self::collect_locals_in_expr(scrut, locals);
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
            if matches!(
                stmt,
                Statement::Expr(expr, _) | Statement::Return(Some(expr), _)
                    if matches!(expr, Expression::Match { .. })
            ) {
                bail!("WASM backend does not yet lower `match` expressions; use `ooda run`.");
            }
        }
        if let Some(eb) = else_branch {
            for stmt in &eb.stmts {
                if matches!(
                stmt,
                Statement::Expr(expr, _) | Statement::Return(Some(expr), _)
                    if matches!(expr, Expression::Match { .. })
            ) {
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

}
