impl WasmCodeGen {

    /// Emit WAT for one statement (used by `emit_if` for each
    /// branch). Mirrors the body-emission logic in `emit_function`.
    fn emit_stmt_wat(stmt: &Statement, locals: &BTreeMap<String, &'static str>) -> Result<String> {
        let mut wat = String::new();
        match stmt {
            Statement::Let { name, init, .. } => {
                wat.push_str(&Self::emit_expr(init, locals)?);
                wat.push_str(&format!("        local.set ${}\n", name));
            }
                Statement::FieldAssign { .. } => {
                    bail!("WASM backend does not support field assignment. Use `ooda run`.");
                }
            Statement::Assign { name, value, .. } => {
                wat.push_str(&Self::emit_expr(value, locals)?);
                wat.push_str(&format!("        local.set ${}\n", name));
            }
            Statement::Return(Some(expr), _) => {
                wat.push_str(&Self::emit_expr(expr, locals)?);
                wat.push_str("        return\n");
            }
            Statement::Break(_) => {
                let br = WASM_LOOP_STACK.with(|s| {
                    s.borrow().last().map(|(b, _)| b.clone())
                }).ok_or_else(|| anyhow::anyhow!("WASM: break outside loop"))?;
                wat.push_str(&format!("        br ${}\n", br));
            }
            Statement::Continue(_) => {
                let cont = WASM_LOOP_STACK.with(|s| {
                    s.borrow().last().map(|(_, c)| c.clone())
                }).ok_or_else(|| anyhow::anyhow!("WASM: continue outside loop"))?;
                wat.push_str(&format!("        br ${}\n", cont));
            }
            Statement::Return(None, _) => {
                wat.push_str("        return\n");
            }
            Statement::Expr(expr, _) => {
                wat.push_str(&Self::emit_expr(expr, locals)?);
                // Statement-context: drop leftover stack values.
                // println consumes its args; if/while/other exprs leave a value.
                match expr {
                    Expression::Call { name, .. } if name == "println" => {}
                    _ => {
                        wat.push_str("        drop\n");
                    }
                }
            }
            Statement::While { cond, body, .. } => {
                wat.push_str(&Self::emit_while(cond, body, locals)?);
            }
        }
        Ok(wat)
    }


    /// Structural validation: scan the emitted WAT for undeclared locals and
    /// missing `return` instructions. Optionally round-trip with
    /// `wasm-tools validate` when available.
    fn validate_wat(wat: &str) -> Result<()> {
        if wat.is_empty() {
            bail!("WASM validation failed: empty WAT output");
        }
        // Track per-function locals.
        let mut current_locals: HashSet<String> = HashSet::new();
        let mut current_params: HashSet<String> = HashSet::new();
        let mut in_function = false;
        let mut saw_return = false;

        for line in wat.lines() {
            let t = line.trim();
            if t.starts_with("(func $") {
                if in_function && !saw_return {
                    bail!("WASM validation failed: previous function missing `return`");
                }
                in_function = true;
                saw_return = false;
                current_locals.clear();
                current_params.clear();
                // Parse `(func $name (param $p1 i64) ...`
                let after_func = t.trim_start_matches("(func $");
                if let Some(space_idx) = after_func.find(' ') {
                    let mut rest = &after_func[space_idx..];
                    // Consume export if present
                    if rest.trim_start().starts_with("(export") {
                        if let Some(close) = rest.find(')') {
                            rest = &rest[close + 1..];
                        }
                    }
                    while let Some(open) = rest.find("(param $") {
                        let after_param = &rest[open + "(param $".len()..];
                        if let Some(space_idx) = after_param.find(' ') {
                            let pname = &after_param[..space_idx];
                            current_params.insert(pname.to_string());
                            rest = &after_param[space_idx..];
                        } else {
                            break;
                        }
                    }
                }
            } else if t.starts_with("(local $") {
                let after_local = t.trim_start_matches("(local $");
                if let Some(space_idx) = after_local.find(' ') {
                    let lname = &after_local[..space_idx];
                    current_locals.insert(lname.to_string());
                }
            } else if t.starts_with("local.get $") || t.starts_with("local.set $") {
                let op = if t.starts_with("local.get") { "local.get" } else { "local.set" };
                let rest = &t[op.len() + 2..]; // skip "$" + name
                let name = rest.split_whitespace().next().unwrap_or("");
                if !current_params.contains(name) && !current_locals.contains(name) {
                    bail!(
                        "WASM validation failed: {} references undeclared local/param `${}` (must be in (param ...) or (local ...))",
                        op,
                        name
                    );
                }
            } else if t == "return" {
                saw_return = true;
            }
        }
        if in_function && !saw_return {
            bail!("WASM validation failed: last function missing `return`");
        }

        // Optional external validation
        if let Ok(status) = Self::run_wasm_tools_validate(wat) {
            if !status {
                bail!("WASM validation failed: `wasm-tools validate` rejected the WAT");
            }
        }
        Ok(())
    }


    fn run_wasm_tools_validate(wat: &str) -> Result<bool> {
        let candidates = ["wasm-tools", "wasm-tools-1", "wasm-tools-2"];
        let bin = candidates
            .into_iter()
            .find(|c| Command::new(c).arg("--version").output().is_ok());
        let Some(bin) = bin else {
            return Err(anyhow!("wasm-tools not installed"));
        };

        let dir = std::env::temp_dir().join(format!("ooda-wasm-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let wat_path = dir.join("check.wat");
        std::fs::write(&wat_path, wat)?;
        let out = Command::new(bin)
            .arg("validate")
            .arg(&wat_path)
            .output()?;
        let _ = std::fs::remove_dir_all(&dir);
        Ok(out.status.success())
    }

}
