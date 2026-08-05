fn cmd_run(file: PathBuf, json_errors: bool, args: Vec<String>) -> Result<()> {
    let (program, timings) = match load_and_analyze(&file, json_errors) {
        Ok(p) => p,
        Err(code) => std::process::exit(code),
    };

    let mut interpreter = Interpreter::new(program).with_argv(args);
    if let Err(e) = interpreter.execute_all() {
        let msg = format!("{}", e);
        let (line, col) = parse_loc(&msg);
        if json_errors {
            AiDiagnostic::new("RuntimeContractError", &file, line, col, msg.clone(), "Execution failed or precondition/postcondition contract violated.")
                .with_patch_fix(
                    "Satisfy requires / handle ensures",
                    r#"{"codemod":"contract","hint":"requires failed: fix call-site args; ensures failed: fix function body or postcondition; see call site line:col"}"#,
                )
                .with_timings(
                    timings.parse_us,
                    timings.capability_us.saturating_add(timings.typecheck_us),
                )
                .print_json();
        } else {
            eprintln!("Runtime Error: {}", e);
        }
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_check(file: PathBuf, json_errors: bool) -> Result<()> {
    match load_and_analyze(&file, json_errors) {
        Ok((_, t)) => {
            if json_errors {
                let mut d = AiDiagnostic::new(
                    "CheckOk",
                    &file,
                    0,
                    0,
                    "ok",
                    "parse + capabilities + types passed",
                );
                d = t.attach(d);
                d.print_json();
            } else {
                println!(
                    "✓ [openOODA check] {} — parse+cap+types OK \
                     (parse={}µs cap={}µs type={}µs)",
                    file.display(),
                    t.parse_us,
                    t.capability_us,
                    t.typecheck_us
                );
            }
        }
        Err(code) => std::process::exit(code),
    }
    Ok(())
}

fn cmd_bench(file: PathBuf, em: bool) -> Result<()> {
    if em {
        // Honest: run measured E-M clocks before the empirical suite.
        let source_bytes = fs::read_to_string(&file)
            .map(|s| s.len())
            .unwrap_or(0);
        let parse_start = Instant::now();
        let program = load_program(&file).ok();
        let parse_us = parse_start.elapsed().as_micros();
        let report = if let Some(ref prog) = program {
            let cap_start = Instant::now();
            let cap_ok = CapabilityChecker::check_program(prog).is_ok();
            let capability_us = cap_start.elapsed().as_micros();
            let ty_start = Instant::now();
            let ty_ok = TypeChecker::check_program(prog).is_ok();
            let typecheck_us = ty_start.elapsed().as_micros();
            ooda::em::EmReport::from_measured(
                file.display().to_string(),
                source_bytes,
                parse_us,
                capability_us,
                typecheck_us,
                !cap_ok,
                !ty_ok,
            )
        } else {
            ooda::em::EmReport::from_measured_load_failed(
                file.display().to_string(),
                source_bytes,
                parse_us,
            )
        };
        println!("{}", report.display_summary());
        println!();
    }
    bench::run_empirical_verification_suite(&file)?;
    Ok(())
}

fn cmd_em(file: PathBuf, json: bool) -> Result<()> {
    // Honest E-M: wall-clock parse + cap + typecheck only — never invent 82.4% scores.
    let source_bytes = fs::read_to_string(&file)
        .map(|s| s.len())
        .unwrap_or(0);
    let parse_start = Instant::now();
    let program = match load_program(&file) {
        Ok(p) => p,
        Err(e) => {
            let parse_us = parse_start.elapsed().as_micros();
            let report = ooda::em::EmReport::from_measured_load_failed(
                file.display().to_string(),
                source_bytes,
                parse_us,
            );
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", report.display_summary());
            }
            eprintln!("Load Error: {}", e);
            std::process::exit(1);
        }
    };
    let parse_us = parse_start.elapsed().as_micros();
    let cap_start = Instant::now();
    let cap_ok = CapabilityChecker::check_program(&program).is_ok();
    let capability_us = cap_start.elapsed().as_micros();
    let ty_start = Instant::now();
    let ty_ok = TypeChecker::check_program(&program).is_ok();
    let typecheck_us = ty_start.elapsed().as_micros();
    let report = ooda::em::EmReport::from_measured(
        file.display().to_string(),
        source_bytes,
        parse_us,
        capability_us,
        typecheck_us,
        !cap_ok,
        !ty_ok,
    );
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.display_summary());
    }
    if report.check_failed {
        std::process::exit(1);
    }
    Ok(())
}

