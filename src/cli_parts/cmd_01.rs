fn cmd_build(file: PathBuf, release: bool, emit_llvm: bool, target: String) -> Result<()> {

    let program = load_program(&file)
        .with_context(|| format!("Failed to load '{}'", file.display()))?;
    CapabilityChecker::check_program(&program)?;
    TypeChecker::check_program(&program)?;

    let target_l = target.to_lowercase();
    // Fail-closed: non-interpreter backends do not lower requires/ensures yet.
    // Interpreter (`ooda run` / `ooda test`) still evaluates contracts.
    {
        let mut contract_fns = Vec::new();
        for item in &program.items {
            if let ooda::ast::Item::Function(f) = item {
                if !f.requires.is_empty() || !f.ensures.is_empty() {
                    contract_fns.push(f.name.clone());
                }
            }
        }
        if !contract_fns.is_empty()
            && matches!(
                target_l.as_str(),
                "c" | "chs" | "native" | "wasm" | "llvm"
            )
        {
            anyhow::bail!(
                "build --target {}: contracts (requires/ensures) are not lowered outside the interpreter yet \
                 (found on: {}). Use `ooda run` / `ooda test` for contract enforcement, or remove contracts \
                 from functions that must be compiled.",
                target_l,
                contract_fns.join(", ")
            );
        }
    }
    // Dual-engine honesty: `?` early-return is interpreter-only today.
    if matches!(
        target_l.as_str(),
        "c" | "chs" | "native" | "wasm" | "llvm"
    ) && ooda::typecheck::program_uses_try_operator(&program)
    {
        anyhow::bail!(
            "build --target {}: try-operator `?` is not lowered outside the interpreter yet.                      Use `ooda run` for Result propagation, or expand `?` into explicit match.",
            target_l
        );
    }
    // Dual-engine honesty for sealed I/O:
    // - C/native/chs: only effects lowered by CHS C + chs_rt (compile-time caps;
    //   tokens erased in C main). Aligns CLI with host chs_build (oodac bootstrap).
    // - wasm/llvm: still refuse all sealed effects (no runtime cap tokens / no lower).
    if matches!(target_l.as_str(), "c" | "chs" | "native") {
        let unsupported =
            ooda::codegen_c::sealed_effects_not_lowered_on_c(&program);
        if !unsupported.is_empty() {
            anyhow::bail!(
                "build --target {}: sealed effectful builtins not lowered on CHS C \
                 (found: {}). Supported on C: read_file/write_file/path_exists/file_size/\
                 env_get/sys_exec (+ method forms). Use `ooda run` for other sealed I/O, \
                 or remove those calls from compiled code.",
                target_l,
                unsupported.join(", ")
            );
        }
    } else if matches!(target_l.as_str(), "wasm" | "llvm") {
        let sealed = ooda::capabilities::collect_sealed_effect_names(&program);
        if !sealed.is_empty() {
            anyhow::bail!(
                "build --target {}: sealed effectful builtins are not lowered with runtime \
                 capability tokens outside the interpreter yet (found: {}). \
                 Use `ooda run` for cap-gated I/O, or remove sealed calls from compiled code.",
                target_l,
                sealed.join(", ")
            );
        }
    }

    if target_l == "wasm" {
        let wat = WasmCodeGen::emit_wat(&program)?;
        let out_wat = file.with_extension("wat");
        fs::write(&out_wat, &wat)?;
        println!(
            "⚡ [openOODA WebAssembly Compiler] Successfully compiled WebAssembly module: {}",
            out_wat.display()
        );
        if emit_llvm {
            println!("\n--- Generated WebAssembly Text (.wat) ---\n{}", wat);
        }
        return Ok(());
    }

    let out_bin = file.with_extension("");

    // Prefer CHS C backend + gcc for native (stage-1 path; no clang required).
    if target_l == "c" || target_l == "native" || target_l == "chs" {
        let rt = runtime_c_path();
        match CCodeGen::build_native(&program, &out_bin, &rt, release) {
            Ok(()) => {
                println!(
                    "🚀 [openOODA CHS C Backend] Native executable: {} (runtime {})",
                    out_bin.display(),
                    rt.display()
                );
                if emit_llvm {
                    let c = CCodeGen::emit_c(&program)?;
                    println!("\n--- Generated C ---\n{}", c);
                }
                return Ok(());
            }
            Err(e) if target_l == "c" || target_l == "chs" => {
                return Err(e);
            }
            Err(e) => {
                eprintln!(
                    "⚠️  [openOODA CHS C Backend] failed ({}); trying LLVM integer subset…",
                    e
                );
            }
        }
    }

    let llvm_ir = LlvmCodeGen::emit_llvm_ir(&program).with_context(|| {
        format!(
            "Codegen failed for '{}'. CHS C backend and LLVM integer-subset both unavailable/failed. \
             Use `ooda run` for interpreter, or `ooda build --target c` with gcc.",
            file.display()
        )
    })?;

    let out_ll = file.with_extension("ll");
    fs::write(&out_ll, &llvm_ir)?;

    println!(
        "🔨 [openOODA LLVM Compiler] Generated LLVM IR: {}",
        out_ll.display()
    );

    let linked = match try_native_link(&out_ll, &out_bin) {
        NativeLinkResult::Ok => {
            println!(
                "🚀 [openOODA Native Build] Native executable: {}",
                out_bin.display()
            );
            true
        }
        NativeLinkResult::ToolFailed { tool, detail } => {
            eprintln!(
                "⚠️  [openOODA Native Build] {} link failed (IR kept at {}): {}",
                tool,
                out_ll.display(),
                detail
            );
            false
        }
        NativeLinkResult::NoTool => {
            eprintln!(
                "💡 [openOODA Native Build] No clang in PATH; IR only at {}.",
                out_ll.display()
            );
            false
        }
    };

    if emit_llvm {
        println!("\n--- Generated LLVM IR ---\n{}", llvm_ir);
    }

    // Fail-closed: IR-only is not a successful native build (exit non-zero).
    // Use --emit-llvm to keep the IR artifact visible; still fails without a binary
    // so CI cannot green-pass on "wrote .ll".
    if !linked {
        anyhow::bail!(
            "native build did not produce an executable (IR at {}). \
             Install clang, or use `ooda build --target c` with gcc, or `ooda run`.",
            out_ll.display()
        );
    }
    Ok(())
}

fn cmd_dump(kind: String, file: PathBuf) -> Result<()> {
    let src = fs::read_to_string(&file)
        .with_context(|| format!("read {}", file.display()))?;
    match kind.as_str() {
        "tokens" => {
            let mut lexer = Lexer::new(&src);
            match lexer.tokenize() {
                Ok(tokens) => {
                    print!("{}", format_token_dump(&tokens));
                }
                Err(e) => {
                    eprint!("{}", format_check_err("lex", &format!("{}", e)));
                    std::process::exit(1);
                }
            }
        }
        "ast" => {
            let mut lexer = Lexer::new(&src);
            let tokens = match lexer.tokenize() {
                Ok(t) => t,
                Err(e) => {
                    eprint!("{}", format_check_err("lex", &format!("{}", e)));
                    std::process::exit(1);
                }
            };
            let mut parser = Parser::new(tokens);
            match parser.parse_program() {
                Ok(prog) => print!("{}", format_ast_dump(&prog)),
                Err(e) => {
                    eprint!("{}", format_check_err("parse", &format!("{}", e)));
                    std::process::exit(1);
                }
            }
        }
        "check" => {
            match load_and_analyze(&file, false) {
                Ok((_, _t)) => print!("{}", format_check_ok()),
                Err(_code) => {
                    // load_and_analyze already printed; emit stable ERR for harness
                    eprint!("{}", format_check_err("check", "failed"));
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("unknown dump kind '{}'; use tokens|ast|check", other);
            std::process::exit(2);
        }
    }
    Ok(())
}

