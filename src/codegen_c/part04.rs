impl Gen {

    fn emit_expr(
        &mut self,
        expr: &Expression,
        env: &mut HashMap<String, String>,
    ) -> Result<(String, String, String)> {
        match expr {
            Expression::Literal(Literal::Int(n), _) => {
                Ok((String::new(), format!("{}LL", n), "long long".into()))
            }
            Expression::Literal(Literal::Bool(b), _) => {
                Ok((String::new(), if *b { "1" } else { "0" }.into(), "int".into()))
            }
            Expression::Literal(Literal::String(s), _) => {
                let lit = c_escape_string(s);
                let t = self.fresh("s");
                Ok((
                    format!("  OoStr {} = oo_str_lit(\"{}\");\n", t, lit),
                    t,
                    "OoStr".into(),
                ))
            }
            Expression::Literal(Literal::Float(f), _) => {
                Ok((String::new(), format!("{}LL", *f as i64), "long long".into()))
            }
            Expression::Literal(Literal::Void, _) => {
                Ok((String::new(), "0".into(), "int".into()))
            }
            Expression::Variable(name, _) => {
                let ty = env.get(name).cloned().unwrap_or_else(|| "long long".into());
                Ok((String::new(), name.clone(), ty))
            }
            Expression::Binary { op, left, right, .. } => {
                let (lc, lv, lty) = self.emit_expr(left, env)?;
                let (rc, rv, rty) = self.emit_expr(right, env)?;
                let mut code = lc;
                code.push_str(&rc);
                if matches!(op, BinOp::Add) && (lty == "OoStr" || rty == "OoStr") {
                    let t = self.fresh("cat");
                    code.push_str(&format!(
                        "  OoStr {} = oo_str_concat({}, {});\n",
                        t, lv, rv
                    ));
                    return Ok((code, t, "OoStr".into()));
                }
                if matches!(op, BinOp::Eq | BinOp::Neq) && lty == "OoStr" {
                    let t = self.fresh("eq");
                    if matches!(op, BinOp::Eq) {
                        code.push_str(&format!("  int {} = oo_str_eq({}, {});\n", t, lv, rv));
                    } else {
                        code.push_str(&format!("  int {} = !oo_str_eq({}, {});\n", t, lv, rv));
                    }
                    return Ok((code, t, "int".into()));
                }
                let cop = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Eq => "==",
                    BinOp::Neq => "!=",
                    BinOp::Lt => "<",
                    BinOp::Lte => "<=",
                    BinOp::Gt => ">",
                    BinOp::Gte => ">=",
                    BinOp::And => "&&",
                    BinOp::Or => "||",
                    _ => bail!("C backend: unsupported binop {:?}", op),
                };
                let t = self.fresh("b");
                let rty = if matches!(
                    op,
                    BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte | BinOp::And | BinOp::Or
                ) {
                    "int"
                } else {
                    "long long"
                };
                code.push_str(&format!(
                    "  {} {} = ({}) {} ({});\n",
                    rty, t, lv, cop, rv
                ));
                Ok((code, t, rty.into()))
            }
            Expression::Unary { op, expr, .. } => {
                let (c, v, _) = self.emit_expr(expr, env)?;
                let t = self.fresh("u");
                let mut code = c;
                match op {
                    UnaryOp::Not => {
                        code.push_str(&format!("  int {} = !({});\n", t, v));
                        Ok((code, t, "int".into()))
                    }
                    UnaryOp::Neg => {
                        code.push_str(&format!("  long long {} = -({});\n", t, v));
                        Ok((code, t, "long long".into()))
                    }
                }
            }
            Expression::Call { name, args, .. } => self.emit_call(name, args, env),
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                // Statement-style if (no value): emit control flow only.
                // Value-style if is limited to int results from tails.
                let (cc, cv, _) = self.emit_expr(cond, env)?;
                let mut code = cc;
                // Prefer statement-if when branches have returns/stmts without shared value.
                let t = self.fresh("ifv");
                code.push_str(&format!("  long long {} = 0;\n", t));
                code.push_str(&format!("  if ({}) {{\n", cv));
                // Use Unknown ret so Return(Some) still emits the value; Void would force `return 0`.
                for s in &then_branch.stmts {
                    code.push_str(&self.emit_stmt(s, env, &Type::Custom("_ret".into()))?);
                }
                if let Some(e) = &then_branch.expr {
                    let (tc, tv, tty) = self.emit_expr(e, env)?;
                    code.push_str(&tc);
                    if tty == "long long" || tty == "int" {
                        code.push_str(&format!("    {} = {};\n", t, tv));
                    }
                }
                code.push_str("  }");
                if let Some(eb) = else_branch {
                    code.push_str(" else {\n");
                    for s in &eb.stmts {
                        code.push_str(&self.emit_stmt(s, env, &Type::Custom("_ret".into()))?);
                    }
                    if let Some(e) = &eb.expr {
                        let (ec, ev, ety) = self.emit_expr(e, env)?;
                        code.push_str(&ec);
                        if ety == "long long" || ety == "int" {
                            code.push_str(&format!("    {} = {};\n", t, ev));
                        }
                    }
                    code.push_str("  }\n");
                } else {
                    code.push_str("\n");
                }
                Ok((code, t, "long long".into()))
            }
            Expression::While { .. } => {
                bail!("C backend: while as expression not supported; use statement while")
            }
            Expression::Match { expr, arms, .. } => {
                // Lower Result match: Ok/Err only, int/string payload loosely
                let (ec, ev, ety) = self.emit_expr(expr, env)?;
                let t = self.fresh("mv");
                let mut code = ec;
                if ety == "OoResS" {
                    code.push_str(&format!("  OoStr {} = oo_str_lit(\"\");\n", t));
                    code.push_str(&format!("  if (({}).ok) {{\n", ev));
                    for arm in arms {
                        if let Pattern::Variant { name, arg } = &arm.pattern {
                            if name == "Ok" {
                                if let Some(bind) = arg {
                                    code.push_str(&format!(
                                        "    OoStr {} = ({}).val;\n",
                                        bind, ev
                                    ));
                                    env.insert(bind.clone(), "OoStr".into());
                                }
                                let (bc, bv, bty) = self.emit_expr(&arm.body, env)?;
                                code.push_str(&bc);
                                if bty == "OoStr" {
                                    code.push_str(&format!("    {} = {};\n", t, bv));
                                }
                            }
                        }
                    }
                    code.push_str("  } else {\n");
                    for arm in arms {
                        if let Pattern::Variant { name, arg } = &arm.pattern {
                            if name == "Err" {
                                if let Some(bind) = arg {
                                    code.push_str(&format!(
                                        "    OoStr {} = ({}).val;\n",
                                        bind, ev
                                    ));
                                    env.insert(bind.clone(), "OoStr".into());
                                }
                                let (bc, bv, bty) = self.emit_expr(&arm.body, env)?;
                                code.push_str(&bc);
                                if bty == "OoStr" {
                                    code.push_str(&format!("    {} = {};\n", t, bv));
                                }
                            }
                        }
                    }
                    code.push_str("  }\n");
                    Ok((code, t, "OoStr".into()))
                } else {
                    // int match on scrutinee
                    code.push_str(&format!("  long long {} = 0;\n", t));
                    for arm in arms {
                        match &arm.pattern {
                            Pattern::Literal(Literal::Int(n)) => {
                                code.push_str(&format!("  if (({}) == {}LL) {{\n", ev, n));
                                let (bc, bv, _) = self.emit_expr(&arm.body, env)?;
                                code.push_str(&bc);
                                code.push_str(&format!("    {} = {};\n", t, bv));
                                code.push_str("  } else ");
                            }
                            Pattern::Wildcard => {
                                code.push_str("  {\n");
                                let (bc, bv, _) = self.emit_expr(&arm.body, env)?;
                                code.push_str(&bc);
                                code.push_str(&format!("    {} = {};\n", t, bv));
                                code.push_str("  }\n");
                            }
                            _ => {}
                        }
                    }
                    Ok((code, t, "long long".into()))
                }
            }
            Expression::StructLit { name, fields, .. } => {
                let t = self.fresh("st");
                let mut code = format!("  struct {} {} ;\n", name, t);
                for (fnm, fex) in fields {
                    let (fc, fv, _) = self.emit_expr(fex, env)?;
                    code.push_str(&fc);
                    code.push_str(&format!("  {} .{} = {};\n", t, fnm, fv));
                }
                Ok((code, t, format!("struct {}", name)))
            }
        }
    }

}
