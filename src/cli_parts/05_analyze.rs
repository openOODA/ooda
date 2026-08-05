
/// Load .oo (with imports), then capability + type check. Returns
/// the timings so the caller can attach them to `--json-errors`.
/// On failure prints diagnostics and returns process exit code.
fn load_and_analyze(
    file: &std::path::Path,
    json_errors: bool,
) -> Result<(Program, AnalyzeTimings), i32> {
    let total_start = Instant::now();
    let parse_start = Instant::now();
    let program = match load_program(file) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("{}", e);
            let (line, col) = parse_loc(&msg);
            let parse_us = parse_start.elapsed().as_micros();
            if json_errors {
                AiDiagnostic::new(
                    "LoadError",
                    file,
                    line,
                    col,
                    msg,
                    "Failed to load .oo source or resolve import.",
                )
                .with_fix(
                    "Fix import path",
                    "import \"module.oo\";  // relative, OODA_PATH, or OODA_STD",
                )
                .with_timings(parse_us, 0)
                .print_json();
            } else {
                eprintln!("Load Error: {}", e);
            }
            return Err(1);
        }
    };
    let parse_us = parse_start.elapsed().as_micros();

    let cap_start = Instant::now();
    if let Err(e) = CapabilityChecker::check_program(&program) {
        let msg = format!("{}", e);
        let (line, col) = parse_loc(&msg);
        let capability_us = cap_start.elapsed().as_micros();
        if json_errors {
            // Surgical-ish fix: name the offending function when present in the message.
            let fn_name = msg
                .split("Function '")
                .nth(1)
                .and_then(|s| s.split('\'').next())
                .unwrap_or("f");
            let (cap_ty, effect_call) = if msg.contains("FsCap") || msg.contains("read_file") || msg.contains("write_file") {
                ("&FsCap", "write_file(cap, path, content)")
            } else if msg.contains("SysCap") {
                ("&SysCap", "sys_exec(cap, cmd)")
            } else if msg.contains("EnvCap") {
                ("&EnvCap", "env_get(cap, key)")
            } else {
                ("&NetCap", "fetch(cap, url)")
            };
            // Machine-applicable ooda patch JSON: add cap param (agents apply via `ooda patch`).
            let patch_json = format!(
                "{{\"target_function\":\"{}\",\"new_params\":\"cap: {}, ...existing\",\"new_body\":\"// object-cap: pass live handle into sealed calls\\n// e.g. {}\"}}",
                fn_name, cap_ty, effect_call
            );
            AiDiagnostic::new(
                "CapabilitySecurityViolation",
                file,
                line,
                col,
                msg.clone(),
                "Function attempts I/O without a live capability handle argument (object-capability).",
            )
            .with_patch_fix("Grant + thread capability handle", patch_json)
            .with_timings(parse_us, capability_us)
            .print_json();
        } else {
            eprintln!("Security Error: {}", e);
        }
        return Err(1);
    }
    let capability_us = cap_start.elapsed().as_micros();

    let typecheck_start = Instant::now();
    if let Err(e) = TypeChecker::check_program(&program) {
        let msg = format!("{}", e);
        let (line, col) = parse_loc(&msg);
        let typecheck_us = typecheck_start.elapsed().as_micros();
        if json_errors {
            // Prefer machine-applicable ooda-patch JSON for high-frequency AI auto-fix cases.
            let (fix_desc, fix_diff, is_patch) =
                typecheck_fix_suggestion(&msg, line);
            let d = AiDiagnostic::new(
                "TypeError",
                file,
                line,
                col,
                msg,
                "Static type mismatch detected before execution.",
            )
            .with_timings(parse_us, capability_us.saturating_add(typecheck_us));
            if is_patch {
                d.with_patch_fix(fix_desc, fix_diff).print_json();
            } else {
                d.with_fix(fix_desc, fix_diff).print_json();
            }
        } else {
            eprintln!("Type Error: {}", e);
        }
        return Err(1);
    }
    let typecheck_us = typecheck_start.elapsed().as_micros();
    let _total_us = total_start.elapsed().as_micros();

    Ok((
        program,
        AnalyzeTimings {
            parse_us,
            capability_us,
            typecheck_us,
        },
    ))
}
