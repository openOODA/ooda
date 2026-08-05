impl LlvmCodeGen {

    fn emit_expr(
        expr: &Expression,
        mut reg: usize,
        locals: &std::collections::HashMap<String, &'static str>,
    ) -> Result<(String, String, usize, &'static str)> {
        let mut code = String::new();
        match expr {
            Expression::Literal(Literal::Int(n), _) => Ok((format!("{}", n), code, reg, "i64")),
            Expression::Literal(Literal::Bool(b), _) => {
                Ok((format!("{}", if *b { 1 } else { 0 }), code, reg, "i1"))
            }
            Expression::Literal(Literal::Float(f), _) => {
                Ok((format!("{}", f), code, reg, "double"))
            }
            Expression::Literal(Literal::Void, _) => Ok(("0".into(), code, reg, "i64")),
            Expression::Literal(Literal::String(_), _) => {
                bail!("internal: string literal reached LLVM emit")
            }
            Expression::Variable(name, _) => {
                let vty = locals.get(name).copied().unwrap_or("i64");
                let r = format!("%r{}", reg);
                reg += 1;
                code.push_str(&format!("  {} = load {}, {}* %var_{}\n", r, vty, vty, name));
                Ok((r, code, reg, vty))
            }
            Expression::Binary { op, left, right, .. } => {
                let (l, lc, r1, lty) = Self::emit_expr(left, reg, locals)?;
                let (r, rc, r2, rty) = Self::emit_expr(right, r1, locals)?;
                code.push_str(&lc);
                code.push_str(&rc);
                let res = format!("%r{}", r2);
                reg = r2 + 1;

                let use_float = lty == "double" || rty == "double";
                if use_float {
                    let (op_str, out_ty): (&str, &str) = match op {
                        BinOp::Add => ("fadd double", "double"),
                        BinOp::Sub => ("fsub double", "double"),
                        BinOp::Mul => ("fmul double", "double"),
                        BinOp::Div => ("fdiv double", "double"),
                        BinOp::Eq => ("fcmp oeq double", "i1"),
                        BinOp::Neq => ("fcmp one double", "i1"),
                        BinOp::Lt => ("fcmp olt double", "i1"),
                        BinOp::Lte => ("fcmp ole double", "i1"),
                        BinOp::Gt => ("fcmp ogt double", "i1"),
                        BinOp::Gte => ("fcmp oge double", "i1"),
                        _ => bail!("LLVM backend: unsupported float operator {:?}", op),
                    };
                    code.push_str(&format!("  {} = {} {}, {}\n", res, op_str, l, r));
                    return Ok((res, code, reg, out_ty));
                }

                // Promote i1 loads to i64 for arithmetic when needed
                let (l_i64, r_i64, prep) = if lty == "i1" {
                    let a = format!("%r{}", reg);
                    reg += 1;
                    let b = format!("%r{}", reg);
                    reg += 1;
                    let mut p = String::new();
                    p.push_str(&format!("  {} = zext i1 {} to i64\n", a, l));
                    p.push_str(&format!("  {} = zext i1 {} to i64\n", b, r));
                    (a, b, p)
                } else {
                    (l.clone(), r.clone(), String::new())
                };
                code.push_str(&prep);

                let (op_str, out_ty): (&str, &str) = match op {
                    BinOp::Add => ("add i64", "i64"),
                    BinOp::Sub => ("sub i64", "i64"),
                    BinOp::Mul => ("mul i64", "i64"),
                    BinOp::Div => ("sdiv i64", "i64"),
                    BinOp::Eq => ("icmp eq i64", "i1"),
                    BinOp::Neq => ("icmp ne i64", "i1"),
                    BinOp::Lt => ("icmp slt i64", "i1"),
                    BinOp::Lte => ("icmp sle i64", "i1"),
                    BinOp::Gt => ("icmp sgt i64", "i1"),
                    BinOp::Gte => ("icmp sge i64", "i1"),
                    BinOp::And => ("and i64", "i64"),
                    BinOp::Or => ("or i64", "i64"),
                    _ => ("add i64", "i64"),
                };

                code.push_str(&format!("  {} = {} {}, {}\n", res, op_str, l_i64, r_i64));
                Ok((res, code, reg, out_ty))
            }
            Expression::Call { name, args, .. } => {
                if name == "println" {
                    let mut fmt_args = String::new();
                    for arg in args {
                        let (val, ac, rnext, vty) = Self::emit_expr(arg, reg, locals)?;
                        reg = rnext;
                        code.push_str(&ac);
                        let as_i64 = if vty == "i1" {
                            let z = format!("%r{}", reg);
                            reg += 1;
                            code.push_str(&format!("  {} = zext i1 {} to i64\n", z, val));
                            z
                        } else if vty == "double" {
                            let z = format!("%r{}", reg);
                            reg += 1;
                            code.push_str(&format!("  {} = fptosi double {} to i64\n", z, val));
                            z
                        } else {
                            val
                        };
                        fmt_args.push_str(&format!(", i64 {}", as_i64));
                    }
                    let res = format!("%r{}", reg);
                    reg += 1;
                    code.push_str(&format!(
                        "  {} = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str.fmt_int, i64 0, i64 0){})\n",
                        res, fmt_args
                    ));
                    Ok((res, code, reg, "i32"))
                } else {
                    let mut arg_str = String::new();
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            arg_str.push_str(", ");
                        }
                        let (val, ac, rnext, vty) = Self::emit_expr(arg, reg, locals)?;
                        reg = rnext;
                        code.push_str(&ac);
                        let ty = if vty == "i1" { "i1" } else { "i64" };
                        arg_str.push_str(&format!("{} {}", ty, val));
                    }
                    let res = format!("%r{}", reg);
                    reg += 1;
                    code.push_str(&format!("  {} = call i64 @{}({})\n", res, name, arg_str));
                    Ok((res, code, reg, "i64"))
                }
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                // Full branch lowering (println / assign / break / continue / return).
                // Prior alpha only lowered Return and silently dropped side effects (D↑ honesty bug).
                let (c_val, c_code, mut r_curr, cty) = Self::emit_expr(cond, reg, locals)?;
                code.push_str(&c_code);

                let then_label = format!("then_{}", r_curr);
                let else_label = format!("else_{}", r_curr);
                let merge_label = format!("merge_{}", r_curr);
                r_curr += 1;

                let c_i1 = if cty == "i1" {
                    c_val
                } else {
                    let t = format!("%r{}", r_curr);
                    r_curr += 1;
                    code.push_str(&format!("  {} = icmp ne i64 {}, 0\n", t, c_val));
                    t
                };
                code.push_str(&format!(
                    "  br i1 {}, label %{}, label %{}\n",
                    c_i1, then_label, else_label
                ));

                code.push_str(&format!("\n{}:\n", then_label));
                let (then_code, r1, then_term) =
                    Self::emit_block_stmts(then_branch, r_curr, locals)?;
                r_curr = r1;
                code.push_str(&then_code);
                if !then_term {
                    code.push_str(&format!("  br label %{}\n", merge_label));
                }

                code.push_str(&format!("\n{}:\n", else_label));
                if let Some(eb) = else_branch {
                    let (else_code, r2, else_term) =
                        Self::emit_block_stmts(eb, r_curr, locals)?;
                    r_curr = r2;
                    code.push_str(&else_code);
                    if !else_term {
                        code.push_str(&format!("  br label %{}\n", merge_label));
                    }
                } else {
                    code.push_str(&format!("  br label %{}\n", merge_label));
                }

                code.push_str(&format!("\n{}:\n", merge_label));
                // Statement-context if leaves a dummy 0 (caller may drop).
                Ok(("0".to_string(), code, r_curr, "i64"))
            }
            Expression::Unary { op, expr, .. } => {
                let (v, vc, r1, vty) = Self::emit_expr(expr, reg, locals)?;
                code.push_str(&vc);
                reg = r1;
                let res = format!("%r{}", reg);
                reg += 1;
                match op {
                    UnaryOp::Not => {
                        let as_i1 = if vty == "i1" {
                            v
                        } else {
                            let t = format!("%r{}", reg);
                            reg += 1;
                            code.push_str(&format!("  {} = icmp ne i64 {}, 0\n", t, v));
                            t
                        };
                        code.push_str(&format!("  {} = xor i1 {}, true\n", res, as_i1));
                        Ok((res, code, reg, "i1"))
                    }
                    UnaryOp::Neg => {
                        if vty == "double" {
                            code.push_str(&format!("  {} = fneg double {}\n", res, v));
                            Ok((res, code, reg, "double"))
                        } else {
                            code.push_str(&format!("  {} = sub i64 0, {}\n", res, v));
                            Ok((res, code, reg, "i64"))
                        }
                    }
                }
            }
            Expression::While { cond, body, .. } => {
                let (wcode, r) = Self::emit_while(cond, body, reg, locals)?;
                code.push_str(&wcode);
                Ok(("0".into(), code, r, "i64"))
            }
            Expression::Match { .. } => {
                bail!(
                    "LLVM integer-subset backend does not lower match expressions. Use `ooda run`."
                )
            }
            Expression::StructLit { .. } => {
                bail!(
                    "LLVM CHS emit does not yet lower struct literals (host-only until M4). Use `ooda run`."
                )
            }
        }
    }

}
