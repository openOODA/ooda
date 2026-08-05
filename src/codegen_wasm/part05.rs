impl WasmCodeGen {

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
            Expression::Binary { .. } => {
                wat.push_str(&Self::emit_expr_binary(expr, locals)?);
            }
            Expression::Call { .. } => {
                wat.push_str(&Self::emit_expr_call(expr, locals)?);
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

}
