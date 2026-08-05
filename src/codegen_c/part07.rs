impl Gen {

    fn emit_call_methods_1(
        &mut self,
        method_name: &str,
        name: &str,
        args: &[Expression],
        env: &mut HashMap<String, String>,
        mut code: String,
        cargs: Vec<String>,
        arg_tys: Vec<String>,
        t: String,
    ) -> Result<(String, String, String)> {
        match method_name {
            "host_ast_dump" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_host_ast_dump({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "host_check" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_host_check({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "host_token_dump" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_host_token_dump({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "chs_build" => {
                code.push_str(&format!(
                    "  OoResS {} = oo_chs_build({}, {});\n",
                    t, cargs[0], cargs[1]
                ));
                Ok((code, t, "OoResS".into()))
            }
            "process_exit" => {
                code.push_str(&format!("  exit((int)({}));\n", cargs[0]));
                Ok((code, "0".into(), "int".into()))
            }
            "sys_exec" | "system_exec" => {
                let cmd = cargs.last().unwrap();
                code.push_str(&format!(
                    "  int sysret_{} = system({}.data ? {}.data : \"\");\n", t, cmd, cmd
                ));
                code.push_str(&format!(
                    "  OoResS {} = {{ .ok = (sysret_{} == 0), .val = oo_str_lit(sysret_{} == 0 ? \"OK\" : \"FAIL\") }};\n",
                    t, t, t
                ));
                Ok((code, t, "OoResS".into()))
            }
            "Ok" => {
                // Result ok — payload is String or generic; use OoResS
                let v = cargs.get(0).cloned().unwrap_or_else(|| "oo_str_lit(\"\")".into());
                let ty = arg_tys.get(0).map(|s| s.as_str()).unwrap_or("OoStr");
                if ty == "OoStr" {
                    code.push_str(&format!(
                        "  OoResS {} = {{ .ok = 1, .val = {} }};\n",
                        t, v
                    ));
                } else {
                    // box int as string for simplicity
                    let buf = self.fresh("okb");
                    code.push_str(&format!("  char {}[32];\n", buf));
                    code.push_str(&format!(
                        "  snprintf({}, sizeof({}), \"%lld\", (long long)({}));\n",
                        buf, buf, v
                    ));
                    code.push_str(&format!(
                        "  OoResS {} = {{ .ok = 1, .val = oo_str_lit({}) }};\n",
                        t, buf
                    ));
                }
                Ok((code, t, "OoResS".into()))
            }
            "Err" => {
                let v = cargs.get(0).cloned().unwrap_or_else(|| "oo_str_lit(\"err\")".into());
                let ty = arg_tys.get(0).map(|s| s.as_str()).unwrap_or("OoStr");
                if ty == "OoStr" {
                    code.push_str(&format!(
                        "  OoResS {} = {{ .ok = 0, .val = {} }};\n",
                        t, v
                    ));
                } else {
                    let buf = self.fresh("erb");
                    code.push_str(&format!("  char {}[64];\n", buf));
                    code.push_str(&format!(
                        "  snprintf({}, sizeof({}), \"%lld\", (long long)({}));\n",
                        buf, buf, v
                    ));
                    code.push_str(&format!(
                        "  OoResS {} = {{ .ok = 0, .val = oo_str_lit({}) }};\n",
                        t, buf
                    ));
                }
                Ok((code, t, "OoResS".into()))
            }
            "println" => {
                // handled at stmt level usually
                for (i, a) in cargs.iter().enumerate() {
                    let ty = &arg_tys[i];
                    if ty == "OoStr" {
                        code.push_str(&format!("  oo_print_str({});\n", a));
                    } else if ty == "int" {
                        code.push_str(&format!("  oo_print_bool({});\n", a));
                    } else {
                        code.push_str(&format!("  oo_print_int({});\n", a));
                    }
                }
                code.push_str("  oo_println();\n");
                Ok((code, "0".into(), "int".into()))
            }
            ".contains" => {
                if cargs.len() != 2 {
                    bail!("C backend: .contains expects receiver + needle");
                }
                code.push_str(&format!(
                    "  int {} = oo_str_contains({}, {});\n",
                    t, cargs[0], cargs[1]
                ));
                Ok((code, t, "int".into()))
            }
            // Method-style string ops: same runtime as free functions (dual-engine parity).
            ".char_at" => {
                if cargs.len() != 2 {
                    bail!("C backend: .char_at expects receiver + index");
                }
                code.push_str(&format!(
                    "  OoStr {} = oo_char_at({}, {});\n",
                    t, cargs[0], cargs[1]
                ));
                Ok((code, t, "OoStr".into()))
            }
            ".str_slice" => {
                if cargs.len() != 3 {
                    bail!("C backend: .str_slice expects receiver + start + end");
                }
                code.push_str(&format!(
                    "  OoStr {} = oo_str_slice({}, {}, {});\n",
                    t, cargs[0], cargs[1], cargs[2]
                ));
                Ok((code, t, "OoStr".into()))
            }
            other if other.starts_with('.') => {
                bail!("C backend: unsupported method {}", other)
            }
            other => {
                let rty = self
                    .fn_ret
                    .get(other)
                    .cloned()
                    .unwrap_or_else(|| "long long".into());
                if rty == "void" {
                    code.push_str(&format!(
                        "  oo_{}({});\n",
                        other,
                        cargs.join(", ")
                    ));
                    Ok((code, "0".into(), "int".into()))
                } else {
                    code.push_str(&format!(
                        "  {} {} = oo_{}({});\n",
                        rty,
                        t,
                        other,
                        cargs.join(", ")
                    ));
                    Ok((code, t, rty))
                }
            }
            _ => unreachable!("emit_call_methods_1"),
        }
    }

}
