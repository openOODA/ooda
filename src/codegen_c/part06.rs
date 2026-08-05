impl Gen {
    fn emit_call_methods_0(
        &mut self,
        method_name: &str,
        mut code: String,
        cargs: Vec<String>,
        arg_tys: Vec<String>,
        t: String,
    ) -> Result<(String, String, String)> {
        match method_name {
            "list_new" => {
                // Bare expression form (not through pending let): default int list.
                code.push_str(&format!("  OoIList {} = oo_ilist_new();\n", t));
                Ok((code, t, "OoIList".into()))
            }
            "push" | "list_push" => {
                let list = &cargs[0];
                let item = &cargs[1];
                let lty = arg_tys.first().map(|s| s.as_str()).unwrap_or("");
                let item_ty = arg_tys.get(1).map(|s| s.as_str()).unwrap_or("");
                // Kind from list type, or first element when list is still pending.
                let as_str = lty == "OoSList"
                    || item_ty == "OoStr"
                    || (lty == "OoListPending" && item_ty == "OoStr");
                if as_str {
                    if lty == "OoListPending" {
                        let empty = self.fresh("sl0");
                        code.push_str(&format!("  OoSList {} = oo_slist_new();\n", empty));
                        code.push_str(&format!(
                            "  OoSList {} = oo_slist_push({}, {});\n",
                            t, empty, item
                        ));
                    } else {
                        code.push_str(&format!(
                            "  OoSList {} = oo_slist_push({}, {});\n",
                            t, list, item
                        ));
                    }
                    Ok((code, t, "OoSList".into()))
                } else if lty == "OoListPending" {
                    let empty = self.fresh("il0");
                    code.push_str(&format!("  OoIList {} = oo_ilist_new();\n", empty));
                    code.push_str(&format!(
                        "  OoIList {} = oo_ilist_push({}, {});\n",
                        t, empty, item
                    ));
                    Ok((code, t, "OoIList".into()))
                } else {
                    code.push_str(&format!(
                        "  OoIList {} = oo_ilist_push({}, {});\n",
                        t, list, item
                    ));
                    Ok((code, t, "OoIList".into()))
                }
            }
            "list_get" => {
                let lty = arg_tys.first().map(|s| s.as_str()).unwrap_or("");
                if lty == "OoSList" {
                    code.push_str(&format!(
                        "  OoStr {} = oo_slist_get({}, {});\n",
                        t, cargs[0], cargs[1]
                    ));
                    Ok((code, t, "OoStr".into()))
                } else if lty == "OoListPending" {
                    // Empty pending — should not be read; emit typed zero for compile.
                    code.push_str(&format!("  long long {} = 0; /* empty pending list_get */\n", t));
                    Ok((code, t, "long long".into()))
                } else {
                    code.push_str(&format!(
                        "  long long {} = oo_ilist_get({}, {});\n",
                        t, cargs[0], cargs[1]
                    ));
                    Ok((code, t, "long long".into()))
                }
            }
            "list_len" => {
                let lty = arg_tys.first().map(|s| s.as_str()).unwrap_or("");
                if lty == "OoSList" {
                    code.push_str(&format!(
                        "  long long {} = oo_slist_len({});\n",
                        t, cargs[0]
                    ));
                } else if lty == "OoListPending" {
                    code.push_str(&format!("  long long {} = 0; /* empty pending list */\n", t));
                } else {
                    code.push_str(&format!(
                        "  long long {} = oo_ilist_len({});\n",
                        t, cargs[0]
                    ));
                }
                Ok((code, t, "long long".into()))
            }
            "chars_len" => {
                code.push_str(&format!(
                    "  long long {} = oo_chars_len({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "long long".into()))
            }
            "char_at" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_char_at({}, {});\n",
                    t, cargs[0], cargs[1]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "str_slice" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_str_slice({}, {}, {});\n",
                    t, cargs[0], cargs[1], cargs[2]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "contains" | "str_contains" => {
                code.push_str(&format!(
                    "  int {} = (strstr({}.data ? {}.data : \"\", {}.data ? {}.data : \"\") != NULL);\n",
                    t, cargs[0], cargs[0], cargs[1], cargs[1]
                ));
                Ok((code, t, "int".into()))
            }
            "char_is_digit" => {
                code.push_str(&format!(
                    "  int {} = oo_char_is_digit({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "int".into()))
            }
            "char_is_alpha" => {
                code.push_str(&format!(
                    "  int {} = oo_char_is_alpha({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "int".into()))
            }
            "char_is_space" => {
                code.push_str(&format!(
                    "  int {} = oo_char_is_space({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "int".into()))
            }
            "read_file" | "fs_read" | ".read_file" => {
                let path = cargs.last().unwrap();
                code.push_str(&format!("  OoResS {} = oo_read_file({});\n", t, path));
                Ok((code, t, "OoResS".into()))
            }
            "write_file" | "fs_write" | ".write_file" => {
                // path, content — last two stringish
                let path = if cargs.len() >= 2 {
                    &cargs[cargs.len() - 2]
                } else {
                    &cargs[0]
                };
                let content = cargs.last().unwrap();
                code.push_str(&format!(
                    "  OoResV {} = oo_write_file({}, {});\n",
                    t, path, content
                ));
                // map to OoResS-like for is_ok: use ok field
                Ok((code, t, "OoResV".into()))
            }
            "path_exists" | "fs_exists" => {
                let path = cargs.last().unwrap();
                code.push_str(&format!("  int {} = oo_path_exists({});\n", t, path));
                Ok((code, t, "int".into()))
            }
            "file_size" => {
                let path = cargs.last().unwrap();
                code.push_str(&format!("  long long {} = oo_file_size({});\n", t, path));
                Ok((code, t, "long long".into()))
            }
            // Option and Result both lower to OoResS { int ok; OoStr val; }.
            "is_some" | "is_ok" => {
                if cargs.is_empty() {
                    bail!("C backend: .{} needs a receiver", method_name);
                }
                code.push_str(&format!("  int {} = ({}.ok);\n", t, cargs[0]));
                Ok((code, t, "int".into()))
            }
            "is_none" | "is_err" => {
                if cargs.is_empty() {
                    bail!("C backend: .{} needs a receiver", method_name);
                }
                code.push_str(&format!("  int {} = !({}.ok);\n", t, cargs[0]));
                Ok((code, t, "int".into()))
            }
            "env_get" => {
                // Dead for sealed programs: dual-engine refuse in main.rs. Kept for
                // host/smoke paths that emit C without the sealed gate.
                if cargs.is_empty() {
                    bail!("C backend: env_get needs a key argument");
                }
                let key = cargs.last().unwrap();
                code.push_str(&format!("  OoResS {} = oo_env_get({});\n", t, key));
                Ok((code, t, "OoResS".into()))
            }
            "to_string" => {
                code.push_str(&format!("  OoStr {} = oo_int_to_str({});\n", t, cargs[0]));
                Ok((code, t, "OoStr".into()))
            }
            "trim" => {
                code.push_str(&format!("  OoStr {} = oo_str_trim({});\n", t, cargs[0]));
                Ok((code, t, "OoStr".into()))
            }
            "to_lowercase" => {
                code.push_str(&format!("  OoStr {} = oo_str_to_lowercase({});\n", t, cargs[0]));
                Ok((code, t, "OoStr".into()))
            }
            _ => unreachable!("emit_call_methods_0"),
        }
    }

}
