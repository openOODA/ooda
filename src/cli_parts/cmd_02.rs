fn cmd_test(file: PathBuf, fuzz: bool) -> Result<()> {
    let program = load_program(&file)
        .with_context(|| format!("Failed to load '{}'", file.display()))?;
    CapabilityChecker::check_program(&program)?;
    TypeChecker::check_program(&program)?;

    let mut interpreter = Interpreter::new(program);
    println!("🧪 [openOODA Test Runner] Running contract verify blocks for {} (fuzz={})", file.display(), fuzz);
    interpreter.execute_all()?;

    if fuzz {
        interpreter.fuzz_all()?;
    }
    Ok(())
}

fn cmd_patch(file: PathBuf, diff: String, json: bool) -> Result<()> {
    let target = serde_json::from_str::<serde_json::Value>(&diff)
        .ok()
        .and_then(|v| {
            v.get("target_function")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "".into());
    match patch::apply_patch(&file, &diff) {
        Ok(()) => {
            if json {
                let report = serde_json::json!({
                    "file": file.display().to_string(),
                    "target_function": target,
                    "ok": true,
                    "changed": true,
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "✂️  [openOODA Surgical Patcher] Successfully patched function '{}' in {}",
                    if target.is_empty() { "<fn>" } else { &target },
                    file.display()
                );
            }
        }
        Err(e) => {
            if json {
                let report = serde_json::json!({
                    "file": file.display().to_string(),
                    "target_function": target,
                    "ok": false,
                    "changed": false,
                    "error": format!("{}", e),
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            return Err(e);
        }
    }
    Ok(())
}

fn cmd_reflect(file: PathBuf, symbol: String) -> Result<()> {
    let code = fs::read_to_string(&file)?;
    let mut lexer = Lexer::new(&code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;

    let metadata = reflect::reflect_symbol(&program, &symbol)?;
    println!("🔍 [openOODA Symbol Reflection] Symbol '{}':\n{}", symbol, metadata);
    Ok(())
}

fn cmd_fmt(file: PathBuf, write: bool) -> Result<()> {
    let code = fs::read_to_string(&file)?;
    let mut lexer = Lexer::new(&code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;

    let formatted = fmt::format_program(&program);
    if write {
        fs::write(&file, &formatted)?;
        println!("✨ Formatted and saved {}", file.display());
    } else {
        print!("{}", formatted);
    }
    Ok(())
}

fn cmd_outline(file: PathBuf, json: bool) -> Result<()> {
    let code = fs::read_to_string(&file)?;
    let mut lexer = Lexer::new(&code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;

    if json {
        let js = outline::generate_outline_json(&program, &file.display().to_string())?;
        println!("{}", js);
    } else {
        let summary = outline::generate_outline(&program);
        println!(
            "📋 [openOODA Outline] API Summary for {}:\n{}",
            file.display(),
            summary
        );
    }
    Ok(())
}

fn cmd_pkg(install: Option<String>, init: Option<String>) -> Result<()> {
    if let Some(name) = init {
        pkg::PackageManager::init(&name)?;
    } else if let Some(repo) = install {
        pkg::PackageManager::install(&repo)?;
    } else {
        println!("📦 openOODA Package Manager (product {}). Use --init or --install.", env!("CARGO_PKG_VERSION"));
    }
    Ok(())
}

fn cmd_lsp() -> Result<()> {
    lsp::LspDaemon::start()?;
    Ok(())
}

fn cmd_context(file: PathBuf, symbol: String, tier: String) -> Result<()> {
    let ctx = ContextEngine::build_micro_context(&file.display().to_string(), &symbol, &tier)?;
    println!("🤖 [LLVM/VRAM Dynamic Auto-Scaling Context Payload (Tier: {})]:\n{}", tier, ctx);
    Ok(())
}

fn cmd_replay(file: PathBuf, target: String) -> Result<()> {
    replay::ReplayEngine::replay_execution(&file.display().to_string(), &target)?;
    Ok(())
}

fn cmd_migrate(file: PathBuf, edition: String, json: bool) -> Result<()> {
    if json {
        let _ = migrate::migrate_path_json(&file, &edition)?;
    } else {
        migrate::MigrationEngine::migrate_codebase(
            &file.display().to_string(),
            &edition,
        )?;
    }
    Ok(())
}

