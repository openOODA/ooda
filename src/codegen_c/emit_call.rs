impl Gen {

    fn emit_call(
        &mut self,
        name: &str,
        args: &[Expression],
        env: &mut HashMap<String, String>,
    ) -> Result<(String, String, String)> {
        // Field access .foo with one arg
        if let Some(field) = name.strip_prefix('.') {
            if args.len() == 1 {
                if field == "to_string" {
                    let (c, v, ty) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("ts");
                    let mut code = c;
                    if ty == "OoStr" {
                        code.push_str(&format!("  OoStr {} = {};\n", t, v));
                    } else if ty == "int" {
                        // bool
                        code.push_str(&format!(
                            "  OoStr {} = oo_str_lit(({}) ? \"true\" : \"false\");\n",
                            t, v
                        ));
                    } else {
                        // int → decimal via snprintf
                        let buf = self.fresh("buf");
                        code.push_str(&format!("  char {}[32];\n", buf));
                        code.push_str(&format!(
                            "  snprintf({}, sizeof({}), \"%lld\", (long long)({}));\n",
                            buf, buf, v
                        ));
                        code.push_str(&format!("  OoStr {} = oo_str_lit({});\n", t, buf));
                    }
                    return Ok((code, t, "OoStr".into()));
                }
                if field == "len" {
                    let (c, v, ty) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("ln");
                    let mut code = c;
                    if ty == "OoStr" {
                        code.push_str(&format!(
                            "  long long {} = oo_str_byte_len({});\n",
                            t, v
                        ));
                    } else if ty == "OoIList" {
                        code.push_str(&format!(
                            "  long long {} = oo_ilist_len({});\n",
                            t, v
                        ));
                    } else if ty == "OoSList" {
                        code.push_str(&format!(
                            "  long long {} = oo_slist_len({});\n",
                            t, v
                        ));
                    } else {
                        bail!(".len on unsupported type {}", ty);
                    }
                    return Ok((code, t, "long long".into()));
                }
                // Option/Result both use OoResS.ok (not a distinct is_some field).
                if field == "is_ok" || field == "is_err" || field == "is_some" || field == "is_none"
                {
                    let (c, v, _) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("io");
                    let mut code = c;
                    if field == "is_ok" || field == "is_some" {
                        code.push_str(&format!("  int {} = ({}).ok;\n", t, v));
                    } else {
                        code.push_str(&format!("  int {} = !({}).ok;\n", t, v));
                    }
                    return Ok((code, t, "int".into()));
                }
                if field == "trim" {
                    let (c, v, _) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("tr");
                    let mut code = c;
                    code.push_str(&format!("  OoStr {} = oo_str_trim({});\n", t, v));
                    return Ok((code, t, "OoStr".into()));
                }
                if field == "to_lowercase" {
                    let (c, v, _) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("lc");
                    let mut code = c;
                    code.push_str(&format!("  OoStr {} = oo_str_to_lowercase({});\n", t, v));
                    return Ok((code, t, "OoStr".into()));
                }
                // struct field
                let (c, v, ty) = self.emit_expr(&args[0], env)?;
                let t = self.fresh("fld");
                let mut code = c;
                // Guess field type from known structs
                let fty = if let Some(sname) = ty.strip_prefix("struct ") {
                    self.structs
                        .get(sname)
                        .and_then(|fs| fs.iter().find(|(n, _)| n == field))
                        .map(|(_, t)| self.c_ty(t))
                        .unwrap_or_else(|| "long long".into())
                } else {
                    "long long".into()
                };
                code.push_str(&format!("  {} {} = {}.{};\n", fty, t, v, field));
                return Ok((code, t, fty));
            }
        }

        let mut code = String::new();
        let mut cargs = Vec::new();
        let mut arg_tys = Vec::new();
        let method_name_early = name.strip_prefix('.').unwrap_or(name);
        let skip_cap_args = matches!(
            method_name_early,
            "read_file"
                | "write_file"
                | "fs_read"
                | "fs_write"
                | "env_get"
                | "path_exists"
                | "fs_exists"
                | "file_size"
                | "sys_exec"
        );
        for a in args {
            let (c, v, ty) = self.emit_expr(a, env)?;
            code.push_str(&c);
            // Skip erased capability tokens by type (not by parameter name).
            if skip_cap_args && (ty == "/*cap*/" || ty == "int") {
                if let Expression::Variable(n, _) = a {
                    if env.get(n).map(|t| t.as_str()) == Some("/*cap*/") {
                        continue;
                    }
                }
            }
            cargs.push(v);
            arg_tys.push(ty);
        }

        let t = self.fresh("r");
        let method_name = name.strip_prefix('.').unwrap_or(name);
        if matches!(method_name, "list_new" | "push" | "list_push" | "list_get" | "list_len" | "chars_len" | "char_at" | "str_slice" | "contains" | "str_contains" | "char_is_digit" | "char_is_alpha" | "char_is_space" | "read_file" | "fs_read" | ".read_file" | "write_file" | "fs_write" | ".write_file" | "path_exists" | "fs_exists" | "file_size" | "is_some" | "is_ok" | "is_none" | "is_err" | "env_get" | "to_string" | "trim" | "to_lowercase") {
            return self.emit_call_methods_0(method_name, code, cargs, arg_tys, t);
        }
        if matches!(method_name, "host_ast_dump" | "host_check" | "host_token_dump" | "chs_build" | "process_exit" | "sys_exec" | "system_exec" | "Ok" | "Err" | "println" | ".contains" | ".char_at" | ".str_slice") {
            return self.emit_call_methods_1(method_name, code, cargs, arg_tys, t);
        }
        if method_name.starts_with('.') {
            bail!("C backend: unsupported method {}", method_name);
        }
        let rty = self
            .fn_ret
            .get(method_name)
            .cloned()
            .unwrap_or_else(|| "long long".into());
        if rty == "void" {
            code.push_str(&format!(
                "  oo_{}({});\n",
                method_name,
                cargs.join(", ")
            ));
            Ok((code, "0".into(), "int".into()))
        } else {
            code.push_str(&format!(
                "  {} {} = oo_{}({});\n",
                rty,
                t,
                method_name,
                cargs.join(", ")
            ));
            Ok((code, t, rty))
        }
    }

}
