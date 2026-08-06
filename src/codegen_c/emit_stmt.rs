impl Gen {

    fn emit_main(&mut self, f: &FunctionDecl) -> Result<()> {
        self.c_main = true;
        self.fn_void = false;
        self.fn_ret_ty = Type::Int;
        let mut env = HashMap::new();
        let mut code = String::from("int main(int argc, char **argv) {\n");
        // inject caps as dummy ints
        for p in &f.params {
            match &p.param_type {
                Type::FsCap | Type::EnvCap | Type::SysCap | Type::NetCap => {
                    // Compile-only placeholder. Runtime object-cap is interpreter-only;
                    // dual-engine refuses sealed I/O before this path for sealed programs.
                    code.push_str(&format!(
                        "  int {} = 1; /* cap token erased on C (no runtime gate) */\n",
                        p.name
                    ));
                    env.insert(p.name.clone(), "/*cap*/".into());
                }
                Type::List(inner) if matches!(**inner, Type::String) || p.name == "args" || p.name == "argv" => {
                    code.push_str("  OoSList args = oo_slist_new();\n");
                    code.push_str("  for (int i = 1; i < argc; i++) {\n");
                    code.push_str("    args = oo_slist_push(args, oo_str_lit(argv[i]));\n");
                    code.push_str("  }\n");
                    // also bind param name if not args
                    if p.name != "args" {
                        code.push_str(&format!("  OoSList {} = args;\n", p.name));
                    }
                    env.insert(p.name.clone(), "OoSList".into());
                }
                other => {
                    code.push_str(&format!(
                        "  {} {} = {{0}}; /* default main param */\n",
                        self.c_ty(other),
                        p.name
                    ));
                    env.insert(p.name.clone(), self.c_ty(other));
                }
            }
        }
        // Use Int return type so `return;` in OODA main becomes `return 0;` in C.
        code.push_str(&self.emit_block(&f.body, &mut env, &Type::Int, true)?);
        code.push_str("  return 0;\n}\n");
        self.body.push_str(&code);
        Ok(())
    }


    fn emit_block(
        &mut self,
        block: &Block,
        env: &mut HashMap<String, String>,
        ret_ty: &Type,
        tail_is_fn_return: bool,
    ) -> Result<String> {
        let mut code = String::new();
        for stmt in &block.stmts {
            code.push_str(&self.emit_stmt(stmt, env, ret_ty)?);
        }
        if let Some(e) = &block.expr {
            let (c, v, ty) = self.emit_expr(e, env)?;
            code.push_str(&c);
            // Only function bodies should turn a trailing expression into `return`.
            // while/if bodies often end with a trailing `if` expression.
            if tail_is_fn_return && !matches!(ret_ty, Type::Void) {
                code.push_str(&format!("  return {};\n", v));
            } else {
                let _ = (ty, v);
            }
        }
        Ok(code)
    }


    fn emit_stmt(
        &mut self,
        stmt: &Statement,
        env: &mut HashMap<String, String>,
        ret_ty: &Type,
    ) -> Result<String> {
        match stmt {
            Statement::Let {
                name,
                type_annotation,
                init,
                ..
            } => {
                // Prefer annotation for empty list_new() so List[String] vs List[Int] is correct.
                let ann_ty = type_annotation.as_ref().map(|t| self.c_ty(t));
                // Unannotated bare list_new: defer C type until first list_push (E-M: no
                // dual-representation union; zero drag until first element).
                if matches!(
                    init,
                    Expression::Call { name: n, args, .. } if n == "list_new" && args.is_empty()
                ) && ann_ty.is_none()
                {
                    env.insert(name.clone(), "OoListPending".into());
                    return Ok(format!(
                        "  /* pending list {} — kind fixed on first push */\n",
                        name
                    ));
                }
                let (mut c, mut v, ty) = self.emit_expr(init, env)?;
                let cty = ann_ty.clone().unwrap_or(ty.clone());
                // Relower bare list_new to matching empty list type when annotated.
                if matches!(init, Expression::Call { name: n, args, .. } if n == "list_new" && args.is_empty())
                {
                    if cty == "OoSList" {
                        let t = self.fresh("sl");
                        c = format!("  OoSList {} = oo_slist_new();\n", t);
                        v = t;
                    } else if cty == "OoIList" {
                        let t = self.fresh("il");
                        c = format!("  OoIList {} = oo_ilist_new();\n", t);
                        v = t;
                    }
                }
                env.insert(name.clone(), cty.clone());
                Ok(format!("{}  {} {} = {};\n", c, cty, name, v))
            }
            Statement::FieldAssign { object, field, value, .. } => {
                // CHS structs are C structs: p.x = v or nested p.inner.n = v
                let lval = Self::c_field_lvalue(object, field)?;
                let (vcode, vtmp, vty) = self.emit_expr(value, env)?;
                let mut code = vcode;
                code.push_str(&format!("  {} = {};\n", lval, vtmp));
                let _ = vty;
                Ok(code)
            }
            Statement::Assign { name, value, .. } => {
                let (c, v, ty) = self.emit_expr(value, env)?;
                // First write into a pending list: declare with concrete OoIList/OoSList.
                if env.get(name).map(|s| s.as_str()) == Some("OoListPending") {
                    env.insert(name.clone(), ty.clone());
                    return Ok(format!("{}  {} {} = {};\n", c, ty, name, v));
                }
                // Refine env if push produced a more specific list kind.
                if (ty == "OoSList" || ty == "OoIList")
                    && env.get(name).map(|s| s.as_str()) != Some(ty.as_str())
                {
                    env.insert(name.clone(), ty.clone());
                }
                Ok(format!("{}  {} = {};\n", c, name, v))
            }
            Statement::Return(Some(e), _) => {
                // Bare `return list_new()` must honor function List[String] vs List[Int]
                // (default emit_expr list_new is OoIList — breaks pack_skip-style helpers).
                // Inside if-as-expr, ret_ty is Custom("_ret"); use current fn return type.
                if matches!(
                    e,
                    Expression::Call { name: n, args, .. } if n == "list_new" && args.is_empty()
                ) {
                    let effective = match ret_ty {
                        Type::Custom(s) if s == "_ret" => &self.fn_ret_ty,
                        other => other,
                    };
                    let cty = self.c_ty(effective);
                    if cty == "OoSList" {
                        let t = self.fresh("slr");
                        return Ok(format!(
                            "  OoSList {} = oo_slist_new();\n  return {};\n",
                            t, t
                        ));
                    }
                    if cty == "OoIList" {
                        let t = self.fresh("ilr");
                        return Ok(format!(
                            "  OoIList {} = oo_ilist_new();\n  return {};\n",
                            t, t
                        ));
                    }
                }
                let (c, v, _) = self.emit_expr(e, env)?;
                match ret_ty {
                    Type::Void => Ok(format!("{}  return;\n", c)),
                    Type::Custom(s) if s == "_ret" => {
                        // Nested in if/while: emit real return value (function returns non-void).
                        Ok(format!("{}  return {};\n", c, v))
                    }
                    _ => Ok(format!("{}  return {};\n", c, v)),
                }
            }
            Statement::Return(None, _) => {
                if self.c_main {
                    Ok("  return 0;\n".into())
                } else {
                    Ok("  return;\n".into())
                }
            }
            Statement::Expr(e, _) => {
                // println and side-effecting calls
                if let Expression::Call { name, args, .. } = e {
                    if name == "println" {
                        return self.emit_println(args, env);
                    }
                }
                let (c, _v, _) = self.emit_expr(e, env)?;
                Ok(c)
            }
            Statement::While { cond, body, .. } => {
                let mut code = String::from("  while (1) {\n");
                let (cc2, cv2, _) = self.emit_expr(cond, env)?;
                code.push_str(&cc2);
                code.push_str(&format!("    if (!({})) break;\n", cv2));
                code.push_str(&self.emit_block(body, env, ret_ty, false)?);
                code.push_str("  }\n");
                Ok(code)
            }
            Statement::Break(_) => Ok("  break;\n".into()),
            Statement::Continue(_) => Ok("  continue;\n".into()),
        }
    }


    fn emit_println(&mut self, args: &[Expression], env: &mut HashMap<String, String>) -> Result<String> {
        let mut code = String::new();
        for a in args {
            let (c, v, ty) = self.emit_expr(a, env)?;
            code.push_str(&c);
            if ty == "OoStr" {
                code.push_str(&format!("  oo_print_str({});\n", v));
            } else if ty == "int" {
                code.push_str(&format!("  oo_print_bool({});\n", v));
            } else {
                code.push_str(&format!("  oo_print_int({});\n", v));
            }
        }
        code.push_str("  oo_println();\n");
        Ok(code)
    }

}
