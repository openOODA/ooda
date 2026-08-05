impl LlvmCodeGen {

    /// Structural validation of emitted IR (always). Optional llvm-as if on PATH.
    pub fn validate_ir(ir: &str) -> Result<()> {
        if ir.is_empty() {
            bail!("LLVM validation failed: empty IR");
        }

        // Count function bodies and ensure every define has a ret before closing brace
        let mut in_func = false;
        let mut saw_ret = false;
        let mut define_count = 0;
        for line in ir.lines() {
            let t = line.trim();
            if t.starts_with("define ") {
                if in_func && !saw_ret {
                    bail!("LLVM validation failed: function missing ret before next define");
                }
                in_func = true;
                saw_ret = false;
                define_count += 1;
            } else if t.ends_with(':') && !t.contains(' ') {
                saw_ret = false;
            } else if t.starts_with("ret ") {
                if saw_ret {
                    bail!("LLVM validation failed: multiple ret in the same basic block path (duplicate ret)");
                }
                saw_ret = true;
            } else if t == "}" && in_func {
                if !saw_ret {
                    bail!("LLVM validation failed: function ended without ret");
                }
                in_func = false;
                saw_ret = false;
            }
            // Type-consistency: reject known-bad patterns from earlier buggy emitters
            if t.contains("load i64, i64* %var_") && ir.contains("alloca i8*") {
                // only flag if same function mixes — simple global heuristic skipped
            }
            if t.contains("load i64, i64* %var_") {
                // extract var name and ensure alloca is i64 if we can
            }
        }

        // Pair alloca/load types for %var_X
        let mut alloca_ty: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for line in ir.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("%var_") {
                if let Some((name, rhs)) = rest.split_once(" = alloca ") {
                    alloca_ty.insert(name.to_string(), rhs.to_string());
                }
            }
            if t.contains(" = load ") {
                // pattern: %rN = load TY, TY* %var_NAME
                if let Some(idx) = t.find("load ") {
                    let after = &t[idx + 5..];
                    let parts: Vec<&str> = after.split(',').collect();
                    if parts.len() >= 2 {
                        let load_ty = parts[0].trim();
                        let ptr = parts[1].trim(); // e.g. i64* %var_x
                        if let Some(var_pos) = ptr.find("%var_") {
                            let var = ptr[var_pos + 5..].trim();
                            if let Some(a_ty) = alloca_ty.get(var) {
                                if a_ty != load_ty {
                                    bail!(
                                        "LLVM validation failed: load type {} does not match alloca {} for %var_{}",
                                        load_ty,
                                        a_ty,
                                        var
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        if define_count == 0 {
            bail!("LLVM validation failed: no functions defined");
        }

        // Optional external validation with llvm-as when available
        if let Ok(status) = Self::run_llvm_as(ir) {
            if !status {
                bail!("LLVM validation failed: llvm-as rejected the generated IR");
            }
        }

        Ok(())
    }


    fn run_llvm_as(ir: &str) -> Result<bool> {
        let llvm_as = ["llvm-as", "llvm-as-18", "llvm-as-17", "llvm-as-16", "llvm-as-15"]
            .into_iter()
            .find(|c| Command::new(c).arg("-version").output().is_ok());

        let Some(bin) = llvm_as else {
            return Err(anyhow!("llvm-as not installed"));
        };

        let dir = std::env::temp_dir().join(format!("ooda-llvm-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let ll = dir.join("check.ll");
        let bc = dir.join("check.bc");
        std::fs::write(&ll, ir)?;
        let out = Command::new(bin)
            .arg(&ll)
            .arg("-o")
            .arg(&bc)
            .output()?;
        let _ = std::fs::remove_dir_all(&dir);
        Ok(out.status.success())
    }

}
