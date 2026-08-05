impl LlvmCodeGen {

    fn emit_function(func: &FunctionDecl) -> Result<String> {
        let mut f_ir = String::new();
        let is_main = func.name == "main";

        // main always returns i32 for C ABI compatibility
        let ret_ty = if is_main {
            "i32"
        } else {
            Self::llvm_ty(&func.return_type)
        };

        f_ir.push_str(&format!("define {} @{}(", ret_ty, func.name));
        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                f_ir.push_str(", ");
            }
            let p_ty = Self::llvm_ty(&param.param_type);
            f_ir.push_str(&format!("{} %arg_{}", p_ty, param.name));
        }
        f_ir.push_str(") #0 {\nentry:\n");

        let mut reg = 1usize;
        let mut locals: std::collections::HashMap<String, &'static str> =
            std::collections::HashMap::new();

        for param in &func.params {
            let p_ty = Self::llvm_ty(&param.param_type);
            f_ir.push_str(&format!("  %var_{} = alloca {}\n", param.name, p_ty));
            f_ir.push_str(&format!(
                "  store {} %arg_{}, {}* %var_{}\n",
                p_ty, param.name, p_ty, param.name
            ));
            locals.insert(param.name.clone(), p_ty);
        }

        let mut returned = false;
        for stmt in &func.body.stmts {
            match stmt {
                Statement::Let { name, init, .. } => {
                    let (val, code, r, vty) = Self::emit_expr(init, reg, &locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                    f_ir.push_str(&format!("  %var_{} = alloca {}\n", name, vty));
                    f_ir.push_str(&format!("  store {} {}, {}* %var_{}\n", vty, val, vty, name));
                    locals.insert(name.clone(), vty);
                }
                Statement::FieldAssign { .. } => {
                    bail!("LLVM integer-subset backend does not support field assignment. Use `ooda run` or `ooda build --target c`.");
                }
                Statement::Assign { name, value, .. } => {
                    let (val, code, r, vty) = Self::emit_expr(value, reg, &locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                    let pty = locals.get(name).copied().unwrap_or(vty);
                    f_ir.push_str(&format!("  store {} {}, {}* %var_{}\n", pty, val, pty, name));
                }
                Statement::Return(Some(expr), _) => {
                    let (val, code, r, vty) = Self::emit_expr(expr, reg, &locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                    if is_main {
                        // truncate/extend to i32
                        if vty == "i64" {
                            f_ir.push_str(&format!("  %retcast{} = trunc i64 {} to i32\n", reg, val));
                            f_ir.push_str(&format!("  ret i32 %retcast{}\n", reg));
                            reg += 1;
                        } else if vty == "i32" {
                            f_ir.push_str(&format!("  ret i32 {}\n", val));
                        } else {
                            f_ir.push_str("  ret i32 0\n");
                        }
                    } else if ret_ty == "void" {
                        f_ir.push_str("  ret void\n");
                    } else {
                        f_ir.push_str(&format!("  ret {} {}\n", ret_ty, val));
                    }
                    returned = true;
                }
                Statement::Break(_) => {
                    let end = LLVM_LOOP_STACK.with(|s| {
                        s.borrow()
                            .last()
                            .map(|(b, _)| b.clone())
                    })
                    .ok_or_else(|| anyhow!("LLVM: break outside loop"))?;
                    f_ir.push_str(&format!("  br label %{}\n", end));
                    returned = true; // path ends (do not fall through)
                }
                Statement::Continue(_) => {
                    let head = LLVM_LOOP_STACK.with(|s| {
                        s.borrow()
                            .last()
                            .map(|(_, c)| c.clone())
                    })
                    .ok_or_else(|| anyhow!("LLVM: continue outside loop"))?;
                    f_ir.push_str(&format!("  br label %{}\n", head));
                    returned = true;
                }
                Statement::Return(None, _) => {
                    if is_main {
                        f_ir.push_str("  ret i32 0\n");
                    } else if ret_ty == "void" {
                        f_ir.push_str("  ret void\n");
                    } else {
                        f_ir.push_str(&format!("  ret {} 0\n", ret_ty));
                    }
                    returned = true;
                }
                Statement::Expr(expr, _) => {
                    let (_val, code, r, _vty) = Self::emit_expr(expr, reg, &locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                    if matches!(expr, Expression::If { .. }) {
                        returned = true;
                    }
                }
                Statement::While { cond, body, .. } => {
                    let (code, r) = Self::emit_while(cond, body, reg, &mut locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                }
            }
        }

        if let Some(body_expr) = &func.body.expr {
            let (val, code, _r, _vty) = Self::emit_expr(body_expr, reg, &locals)?;
            f_ir.push_str(&code);
            if !is_main && ret_ty != "void" && !f_ir.ends_with("ret ") {
                f_ir.push_str(&format!("  ret {} {}\n", ret_ty, val));
                returned = true;
            }
        }

        if !returned {
            if is_main {
                f_ir.push_str("  ret i32 0\n");
            } else if ret_ty == "void" {
                f_ir.push_str("  ret void\n");
            } else {
                f_ir.push_str(&format!("  ret {} 0\n", ret_ty));
            }
        }

        f_ir.push_str("}\n\n");
        Ok(f_ir)
    }

}
