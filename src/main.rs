// openOODA CLI binary — logic modules live in the `ooda` library.
use ooda::ast::Program;
use ooda::bench;
use ooda::capabilities::CapabilityChecker;
use ooda::codegen::LlvmCodeGen;
use ooda::codegen_c::{runtime_c_path, CCodeGen};
use ooda::codegen_wasm::WasmCodeGen;
use ooda::diagnostics::{parse_loc, AiDiagnostic};
use ooda::dump::{format_ast_dump, format_check_err, format_check_ok, format_token_dump};
use ooda::eval::Interpreter;
use ooda::fmt;
use ooda::lexer::Lexer;
use ooda::loader::load_program;
use ooda::lsp;
use ooda::migrate;
use ooda::outline;
use ooda::parser::Parser;
use ooda::patch;
use ooda::pkg;
use ooda::reflect;
use ooda::replay;
use ooda::typecheck::TypeChecker;
use ooda::context::ContextEngine;

use clap::{Parser as ClapParser, Subcommand};
use std::path::PathBuf;
use std::fs;
use std::time::Instant;
use anyhow::{Context, Result};

#[derive(ClapParser)]
#[command(name = "ooda")]
#[command(author = "openOODA Core Team")]
#[command(version = "0.166.0-alpha")]
#[command(about = "The OODA Programming Language Compiler & Toolchain", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run an OODA source file (.oo) instantly using the JIT interpreter
    Run {
        /// Path to the .oo file
        file: PathBuf,
        /// Output machine-readable JSON errors for AI auto-fixing
        #[arg(long)]
        json_errors: bool,
        /// Program arguments injected into `main(args: List[String], …)` (use `--` before them)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run empirical benchmarks & claim verification suite on .oo file
    Bench {
        /// Path to the .oo file
        file: PathBuf,
        /// Display Energy-Maneuverability (E-M) Specific Excess Power breakdown
        #[arg(long)]
        em: bool,
    },
    /// Measure real E-M analysis telemetry for a .oo file (parse/check µs, W, V — no fake scores)
    Em {
        /// Path to the .oo file
        file: PathBuf,
        /// Emit machine-readable JSON EmReport (measured fields only; no theater scores)
        #[arg(long)]
        json: bool,
    },
    /// Compile .oo to native (CHS→C+gcc preferred; LLVM integer subset fallback)
    Build {
        /// Path to the .oo file
        file: PathBuf,
        /// Pass -O3 -flto to gcc on CHS C native path (no effect on interpreter / LLVM text emit)
        #[arg(long)]
        release: bool,
        /// Output LLVM IR text file (.ll)
        #[arg(long)]
        emit_llvm: bool,
        /// Target triple architecture (native, wasm)
        #[arg(long, default_value = "native")]
        target: String,
    },
    /// Run inline verify test blocks and contracts
    Test {
        /// Path to the .oo file or directory
        file: PathBuf,
        /// Enable automated fuzzing
        #[arg(long)]
        fuzz: bool,
    },
    /// Typecheck + capability check without executing (AI / CI gate)
    Check {
        /// Path to the .oo file
        file: PathBuf,
        /// Output machine-readable JSON errors
        #[arg(long)]
        json_errors: bool,
    },
    /// Apply surgical AST JSON diff patch to source code
    Patch {
        /// Path to the .oo file
        file: PathBuf,
        /// JSON patch string
        #[arg(long)]
        diff: String,
        /// Emit machine-readable JSON result (file, target, ok) after apply
        #[arg(long)]
        json: bool,
    },
    /// Inspect symbol reflection metadata (types, contracts, capabilities)
    Reflect {
        /// Path to the .oo file
        file: PathBuf,
        /// Target symbol name to reflect
        symbol: String,
    },
    /// Format OODA source code files
    Fmt {
        /// Path to the .oo file
        file: PathBuf,
        /// Write formatted code directly back to file
        #[arg(long)]
        write: bool,
    },
    /// Emit token-minimized module outline (types & contracts only)
    Outline {
        /// Path to the .oo file
        file: PathBuf,
        /// Machine-readable JSON outline (functions, types, contracts) for AI agents
        #[arg(long)]
        json: bool,
    },
    /// Manage OODA package dependencies and lockfiles
    Pkg {
        /// Dependency name or GitHub repository to install
        #[arg(long)]
        install: Option<String>,
        /// Initialize a new package manifest
        #[arg(long)]
        init: Option<String>,
    },
    /// Start Language Server Protocol daemon for VSCode/Cursor IDEs
    Lsp,
    /// Build auto-scaling context payload for 8GB VRAM up to Frontier Cloud LLMs
    Context {
        /// Path to the .oo file
        file: PathBuf,
        /// Target symbol
        symbol: String,
        /// Hardware/LLM VRAM tier (8gb, 16gb, frontier)
        #[arg(long, default_value = "8gb")]
        tier: String,
    },
    /// Replay deterministic execution step-by-step for debugging
    Replay {
        /// Path to the .oo file
        file: PathBuf,
        /// Target function or test name
        target: String,
    },
    /// Migrate codebase syntax automatically to latest edition
    Migrate {
        /// Path to the .oo file
        file: PathBuf,
        /// Target edition (e.g. 2026)
        #[arg(long, default_value = "2026")]
        edition: String,
        /// Emit machine-readable MigrateReport JSON (counts only)
        #[arg(long)]
        json: bool,
    },
    /// Canonical dumps for CHS golden parity (tokens / ast / check)
    Dump {
        /// What to dump: tokens | ast | check
        #[arg(value_parser = ["tokens", "ast", "check"])]
        kind: String,
        /// Path to the .oo file
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            file,
            json_errors,
            args,
        } => {
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
        }
        Commands::Check { file, json_errors } => {
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
        }
        Commands::Bench { file, em } => {
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
        }
        Commands::Em { file, json } => {
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
        }
        Commands::Build { file, release, emit_llvm, target } => {

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
            // Dual-engine honesty: sealed I/O has no runtime cap tokens in C/LLVM/WASM yet.
            // Refuse to emit open native binaries that drop the interpreter's default-deny gate.
            if matches!(
                target_l.as_str(),
                "c" | "chs" | "native" | "wasm" | "llvm"
            ) {
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
        }
        Commands::Dump { kind, file } => {
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
        }
        Commands::Test { file, fuzz } => {
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
        }
        Commands::Patch { file, diff, json } => {
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
        }
        Commands::Reflect { file, symbol } => {
            let code = fs::read_to_string(&file)?;
            let mut lexer = Lexer::new(&code);
            let tokens = lexer.tokenize()?;
            let mut parser = Parser::new(tokens);
            let program = parser.parse_program()?;

            let metadata = reflect::reflect_symbol(&program, &symbol)?;
            println!("🔍 [openOODA Symbol Reflection] Symbol '{}':\n{}", symbol, metadata);
        }
        Commands::Fmt { file, write } => {
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
        }
        Commands::Outline { file, json } => {
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
        }
        Commands::Pkg { install, init } => {
            if let Some(name) = init {
                pkg::PackageManager::init(&name)?;
            } else if let Some(repo) = install {
                pkg::PackageManager::install(&repo)?;
            } else {
                println!("📦 openOODA Package Manager (product {}). Use --init or --install.", env!("CARGO_PKG_VERSION"));
            }
        }
        Commands::Lsp => {
            lsp::LspDaemon::start()?;
        }
        Commands::Context { file, symbol, tier } => {
            let ctx = ContextEngine::build_micro_context(&file.display().to_string(), &symbol, &tier)?;
            println!("🤖 [LLVM/VRAM Dynamic Auto-Scaling Context Payload (Tier: {})]:\n{}", tier, ctx);
        }
        Commands::Replay { file, target } => {
            replay::ReplayEngine::replay_execution(&file.display().to_string(), &target)?;
        }
        Commands::Migrate { file, edition, json } => {
            if json {
                let _ = migrate::migrate_path_json(&file, &edition)?;
            } else {
                migrate::MigrationEngine::migrate_codebase(
                    &file.display().to_string(),
                    &edition,
                )?;
            }
        }
    }

    Ok(())
}

/// Real parse + capability + type-check timings. Wired into
/// `--json-errors` so AI agents see measured µs, not "honest
/// theater" hardcoded numbers.
struct AnalyzeTimings {
    parse_us: u128,
    capability_us: u128,
    typecheck_us: u128,
}

impl AnalyzeTimings {
    fn attach(self, d: AiDiagnostic) -> AiDiagnostic {
        d.with_timings(
            self.parse_us,
            self.capability_us.saturating_add(self.typecheck_us),
        )
    }
}

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
            let (fix_desc, fix_diff, is_patch): (String, String, bool) = if msg.contains("must-use")
                || msg.contains("unused Result")
            {
                (
                    "Handle Result/Option with match".into(),
                    r#"{"target_function":"<fn>","new_body":"let r = <expr>;\nmatch r {\n  Ok(v) => { /* use v */ },\n  Err(e) => { /* handle e */ }\n}"}"#.into(),
                    true,
                )
            } else if msg.contains("non-exhaustive match") {
                (
                    "Cover all match variants".into(),
                    r#"{"target_function":"<fn>","new_body":"match r {\n  Ok(v) => …,\n  Err(e) => …\n  // or add `_ => process_exit(1)` then replace\n}"}"#.into(),
                    true,
                )
            } else if msg.contains("immutable") || msg.contains("let mut") {
                let vname = msg
                    .split('\'')
                    .nth(1)
                    .unwrap_or("x");
                (
                    "Use let mut for assigned binding".into(),
                    format!(
                        "{{\"codemod\":\"let_mut\",\"binding\":\"{}\",\"hint\":\"ooda migrate --edition 2026 rewrites assigned immutable let → let mut\"}}",
                        vname
                    ),
                    true,
                )
            } else if msg.contains("missing return") {
                // Message shape: "declares return type {T} but body has type Void (missing return value)"
                let ret_ty = msg
                    .split("declares return type ")
                    .nth(1)
                    .and_then(|s| s.split(" but body").next())
                    .unwrap_or("Int")
                    .trim();
                let stub = match ret_ty {
                    "Int" | "Float" => "return 0;",
                    "Bool" => "return false;",
                    "String" => "return \"\";",
                    "Void" => "return;",
                    t if t.starts_with("Option") => "return None;",
                    t if t.starts_with("Result") => {
                        "return Err(\"TODO: missing return\");"
                    }
                    _ => "return 0; /* TODO: match declared return type */",
                };
                (
                    format!("Add return value on every path (declared {})", ret_ty),
                    format!(
                        r#"{{"codemod":"missing_return","declared_return":"{}","target_line":{},"new_code":"{}"}}"#,
                        ret_ty.replace('"', "\\\""),
                        line,
                        stub.replace('"', "\\\"")
                    ),
                    true,
                )
            } else if msg.contains("unreachable code after return") {
                (
                    "Remove dead code after return".into(),
                    r#"{"hint":"delete statements after `return` — they never execute"}"#.into(),
                    true,
                )
            } else if msg.contains("division by zero") {
                (
                    "Fix zero divisor".into(),
                    r#"{"hint":"const divisor is 0 — change the literal or guard the division"}"#.into(),
                    true,
                )
            } else if msg.contains("undefined function") {
                let fname = msg
                    .split("undefined function '")
                    .nth(1)
                    .and_then(|s| s.split('\'').next())
                    .unwrap_or("name");
                (
                    "Define or import function".into(),
                    format!(
                        "{{\"target_function\":\"{}\",\"new_body\":\"// implement {}\\nreturn 0;\"}}",
                        fname, fname
                    ),
                    true,
                )
            } else if msg.contains("argument(s), found") {
                // Arity: function 'f' expects N argument(s), found M
                let fname = msg
                    .split("function '")
                    .nth(1)
                    .and_then(|s| s.split('\'').next())
                    .unwrap_or("f");
                let expected = msg
                    .split("expects ")
                    .nth(1)
                    .and_then(|s| s.split(' ').next())
                    .unwrap_or("?");
                let found = msg
                    .split("found ")
                    .nth(1)
                    .map(|s| {
                        s.chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect::<String>()
                    })
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "?".into());
                (
                    "Fix call argument count".into(),
                    format!(
                        "{{\"codemod\":\"arg_count\",\"function\":\"{}\",\"expected_arity\":{},\"found_arity\":{},\"hint\":\"supply exactly the declared parameters (or change the callee signature)\"}}",
                        fname, expected, found
                    ),
                    true,
                )
            } else if msg.contains("cannot concatenate") || msg.contains("convert with .to_string()")
            {
                (
                    "Convert non-String operand before concat".into(),
                    r#"{"codemod":"str_concat","hint":"use left + right.to_string() (or both String) for concatenation"}"#.into(),
                    true,
                )
            } else if msg.contains("assert_eq arguments must have matching types") {
                let found = msg
                    .split("found ")
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "?".into());
                (
                    "Fix assert_eq operand types".into(),
                    format!(
                        "{{\"codemod\":\"assert_eq_types\",\"found\":\"{}\",\"hint\":\"assert_eq requires identical static types on both sides\"}}",
                        found
                    ),
                    true,
                )
            } else if msg.contains("out of bounds")
                && (msg.contains("char_at") || msg.contains("str_slice"))
            {
                (
                    "Fix const string index / slice bounds".into(),
                    r#"{"codemod":"str_bounds","hint":"use an index in 0..chars_len(s) or a valid [start..end] slice"}"#.into(),
                    true,
                )
            } else if msg.contains("RefinementTypeViolation") {
                // Int[lo..hi] on let / return / call-site arg
                let val = msg
                    .split("value ")
                    .nth(1)
                    .and_then(|s| {
                        s.split(' ')
                            .next()
                            .map(|t| t.trim_end_matches(',').to_string())
                    })
                    .unwrap_or_else(|| "?".into());
                let bounds = msg
                    .split("bounds [")
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .unwrap_or("lo..hi");
                (
                    "Fix refinement bounds".into(),
                    format!(
                        "{{\"codemod\":\"refinement_bounds\",\"value\":\"{}\",\"bounds\":\"[{}]\",\"hint\":\"pass/return/assign a value inside Int[{}]\"}}",
                        val, bounds, bounds
                    ),
                    true,
                )
            } else if msg.contains("cannot assign") && msg.contains("to '") {
                // cannot assign String to 'x' of type Int
                let vname = msg
                    .split("to '")
                    .nth(1)
                    .and_then(|s| s.split('\'').next())
                    .unwrap_or("x");
                let found = msg
                    .split("cannot assign ")
                    .nth(1)
                    .and_then(|s| s.split(' ').next())
                    .unwrap_or("?");
                let expected = msg
                    .split("of type ")
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "?".into());
                (
                    "Fix assignment types".into(),
                    format!(
                        "{{\"codemod\":\"assign_type\",\"binding\":\"{}\",\"expected\":\"{}\",\"found\":\"{}\",\"hint\":\"assign a {} value or change the binding's type\"}}",
                        vname, expected, found, expected
                    ),
                    true,
                )
            } else if msg.contains("list element type mismatch") {
                let expected = msg
                    .split("List[")
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .unwrap_or("?");
                let found = msg
                    .split("cannot push ")
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "?".into());
                (
                    "Fix list element type".into(),
                    format!(
                        "{{\"codemod\":\"list_elem\",\"expected\":\"{}\",\"found\":\"{}\",\"hint\":\"push only {} values or start a new list\"}}",
                        expected, found, expected
                    ),
                    true,
                )
            } else if msg.contains("matching numeric types")
                || (msg.contains("arithmetic") && msg.contains("found"))
            {
                (
                    "Annotate list/element types before arithmetic".into(),
                    r#"{"codemod":"arith_types","hint":"use List[Int]/List[String] annotations or push homogeneous elements before `for` so element type is not `_`"}"#.into(),
                    true,
                )
            } else if msg.contains("return type") && msg.contains("does not match declared") {
                // return type String does not match declared Int — in 'f'
                let fname = msg
                    .split("in '")
                    .nth(1)
                    .and_then(|s| s.split('\'').next())
                    .unwrap_or("f");
                let found = msg
                    .split("return type ")
                    .nth(1)
                    .and_then(|s| s.split(' ').next())
                    .unwrap_or("?");
                let expected = msg
                    .split("declared ")
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "?".into());
                (
                    "Align return type and body".into(),
                    format!(
                        "{{\"codemod\":\"return_type\",\"target_function\":\"{}\",\"declared\":\"{}\",\"found\":\"{}\",\"hint\":\"change body to return {} or patch new_return_type\"}}",
                        fname, expected, found, expected
                    ),
                    true,
                )
            } else if msg.contains("argument ") && msg.contains("expects ") && msg.contains("found ") {
                // Arg type: function 'f' argument N expects T, found U
                let fname = msg
                    .split("function '")
                    .nth(1)
                    .and_then(|s| s.split('\'').next())
                    .unwrap_or("f");
                let arg_index = msg
                    .split("argument ")
                    .nth(1)
                    .and_then(|s| s.split(' ').next())
                    .unwrap_or("0");
                let expected = msg
                    .split("expects ")
                    .nth(1)
                    .and_then(|s| s.split(',').next())
                    .unwrap_or("?")
                    .trim()
                    .to_string();
                let found = msg
                    .split("found ")
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "?".into());
                (
                    "Fix call argument type".into(),
                    format!(
                        "{{\"codemod\":\"arg_type\",\"function\":\"{}\",\"arg_index\":{},\"expected\":\"{}\",\"found\":\"{}\",\"hint\":\"pass a value of the expected type or change the callee param\"}}",
                        fname, arg_index, expected, found
                    ),
                    true,
                )
            } else if msg.contains("unknown method") {
                let mname = msg
                    .split("unknown method '")
                    .nth(1)
                    .and_then(|s| s.split('\'').next())
                    .unwrap_or(".method");
                let on_ty = msg
                    .split(" on ")
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "Type".into());
                (
                    "Fix method / field access".into(),
                    format!(
                        "{{\"codemod\":\"unknown_method\",\"method\":\"{}\",\"receiver\":\"{}\",\"hint\":\"use a real method (.len/.char_at/.push/…) or a struct field; free-form builtins use name(recv, …)\"}}",
                        mname, on_ty
                    ),
                    true,
                )
            } else if msg.contains("undefined variable") {
                let vname = msg
                    .split("undefined variable '")
                    .nth(1)
                    .and_then(|s| s.split('\'').next())
                    .unwrap_or("x");
                (
                    "Define or bind variable".into(),
                    format!(
                        "{{\"codemod\":\"undefined_var\",\"name\":\"{}\",\"hint\":\"add `let {} = …` before use, or fix the name\"}}",
                        vname, vname
                    ),
                    true,
                )
            } else if msg.contains("`?` only allowed") || msg.contains("`?` requires Result") {
                (
                    "Fix try-operator usage".into(),
                    r#"{"codemod":"try_op","hint":"`?` only on Result values inside functions that return Result[T,E] with matching E"}"#.into(),
                    true,
                )
            } else {
                (
                    "Fix types".into(),
                    "Ensure operands and annotations agree (Int/Float/String/Bool/caps).".into(),
                    false,
                )
            };
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

enum NativeLinkResult {
    Ok,
    NoTool,
    ToolFailed { tool: String, detail: String },
}

/// Link LLVM IR (.ll) to a native binary.
///
/// Only **clang** (and versioned clang-*) can consume LLVM IR text as input.
/// Plain `gcc`/`cc` treat `.ll` as a linker script and fail noisily — never try them.
fn try_native_link(ll: &std::path::Path, out_bin: &std::path::Path) -> NativeLinkResult {
    let mut tools: Vec<String> = Vec::new();
    // Prefer explicit OODA_CLANG / CC only if the name looks like clang.
    for key in ["OODA_CLANG", "CC"] {
        if let Ok(cc) = std::env::var(key) {
            let base = cc.rsplit('/').next().unwrap_or(&cc);
            if base.contains("clang") && !tools.iter().any(|x| x == &cc) {
                tools.push(cc);
            }
        }
    }
    for t in ["clang", "clang-18", "clang-17", "clang-16", "clang-15", "clang-14"] {
        if !tools.iter().any(|x| x == t) {
            tools.push(t.to_string());
        }
    }

    let mut last_fail: Option<(String, String)> = None;
    let mut saw_clang = false;
    for tool in tools {
        let probe = std::process::Command::new(&tool).arg("--version").output();
        let Ok(probe_out) = probe else {
            continue;
        };
        let ver = String::from_utf8_lossy(&probe_out.stdout);
        if !ver.to_ascii_lowercase().contains("clang") {
            // Refuse non-clang drivers even if named oddly.
            continue;
        }
        saw_clang = true;
        // `-x ir` forces IR input language so the suffix is unambiguous.
        let out = std::process::Command::new(&tool)
            .arg("-x")
            .arg("ir")
            .arg(ll)
            .arg("-Wno-override-module")
            .arg("-o")
            .arg(out_bin)
            .output();
        match out {
            Ok(o) if o.status.success() => return NativeLinkResult::Ok,
            Ok(o) => {
                let detail = String::from_utf8_lossy(&o.stderr).trim().to_string();
                last_fail = Some((
                    tool,
                    if detail.is_empty() {
                        format!("exit {}", o.status)
                    } else {
                        detail.chars().take(240).collect()
                    },
                ));
            }
            Err(e) => last_fail = Some((tool, e.to_string())),
        }
    }
    if let Some((tool, detail)) = last_fail {
        NativeLinkResult::ToolFailed { tool, detail }
    } else if !saw_clang {
        NativeLinkResult::NoTool
    } else {
        NativeLinkResult::NoTool
    }
}

#[cfg(test)]
mod version_consistency_tests {
    /// Sentinel that prevents version drift across artifacts.
    ///
    /// Bug history: rounds 6, 7, and 8 each had to manually re-align
    /// `Cargo.toml`, `src/main.rs` (clap version), `scripts/release.sh`,
    /// `README.md`, `qa/README.md`, and `docs/index.html`. This test
    /// fails CI if any future bump forgets an artifact, locking in
    /// one canonical version per release.
    ///
    /// If you need to bump: change every string below to the new
    /// version, then commit.
    const CANONICAL_VERSION: &str = "v0.166.0-alpha";
    // For comparing against Cargo.toml which lacks the 'v'
    const CANONICAL_VERSION_NO_V: &str = "0.166.0-alpha";

    fn clap_version() -> &'static str {
        let src = include_str!("main.rs");
        for line in src.lines() {
            if let Some(rest) = line.strip_prefix("#[command(version = \"") {
                if let Some(v) = rest.strip_suffix("\")]") {
                    return v;
                }
            }
        }
        panic!("could not locate `#[command(version = ...)]` in src/main.rs");
    }

    #[test]
    fn clap_version_matches_canonical() {
        assert_eq!(clap_version(), CANONICAL_VERSION_NO_V);
    }

    #[test]
    fn cargo_pkg_version_matches_canonical() {
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            CANONICAL_VERSION_NO_V,
            "Cargo.toml package version must match CANONICAL_VERSION_NO_V"
        );
    }

    #[test]
    fn release_sh_version_derives_from_cargo() {
        // release.sh's default VERSION reads Cargo.toml's version at
        // build time (`v${CARGO_VER}`), so any bump to Cargo.toml
        // propagates automatically. This test asserts that the
        // release script still derives from Cargo rather than a
        // hardcoded string.
        let sh = include_str!("../scripts/release.sh");
        for line in sh.lines() {
            if line.contains("VERSION=") && line.contains("CARGO_VER") {
                return;
            }
        }
        panic!(
            "scripts/release.sh must derive its default VERSION from \
             Cargo.toml via CARGO_VER (no hardcoded version string)."
        );
    }

    #[test]
    fn readme_version_matches_canonical() {
        let readme = include_str!("../README.md");
        for line in readme.lines() {
            if !line.starts_with("**openOODA Project**") {
                continue;
            }
            // Find the substring after "Version `".
            if let Some(idx) = line.find("Version `") {
                let rest = &line[idx + "Version `".len()..];
                if let Some(v) = rest.split('`').next() {
                    assert_eq!(v, CANONICAL_VERSION,
                        "README.md version header does not match the canonical version");
                    return;
                }
            }
            panic!("README header lacks Version-anchor: {}", line);
        }
        panic!("could not locate README version header");
    }

    #[test]
    fn install_oo_default_pin_matches_canonical() {
        let install = include_str!("../install/install.oo");
        let needle = format!("\"{}\"", CANONICAL_VERSION);
        assert!(
            install.contains(&needle),
            "install/install.oo default OODA_VERSION pin must be {}",
            CANONICAL_VERSION
        );
    }

    #[test]
    fn bootstrap_pin_file_matches_canonical() {
        let pin = include_str!("../install/BOOTSTRAP_PIN").trim();
        assert_eq!(
            pin, CANONICAL_VERSION,
            "install/BOOTSTRAP_PIN must match Cargo-derived canonical version \
             (sync openooda-gh-pages install defaults from this file)"
        );
    }

    /// When the monorepo sibling website is present, install entrypoints must
    /// pin the same version (stops homepage CTA thrash to stale tags).
    #[test]
    fn monorepo_site_install_pins_match_canonical_if_present() {
        let candidates = [
            "../openOODA.github.io/install",
            "../openOODA.github.io/install.sh",
        ];
        let mut saw_any = false;
        for rel in candidates {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
            if !path.is_file() {
                continue;
            }
            saw_any = true;
            let body = std::fs::read_to_string(&path).expect("read site install");
            let needle = format!("OODA_VERSION:-{}", CANONICAL_VERSION);
            assert!(
                body.contains(&needle) || body.contains(&format!("\"{}\"", CANONICAL_VERSION)),
                "{} must pin {} (found neither OODA_VERSION:-{} nor quoted pin)",
                path.display(),
                CANONICAL_VERSION,
                CANONICAL_VERSION
            );
        }
        // In monorepo checkouts this must fire; bare ooda clone alone is ok to skip.
        let _ = saw_any;
    }

    /// Docs brand README (if monorepo sibling present) must not lag the pin.
    #[test]
    fn monorepo_docs_readme_pin_if_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/README.md");
        if !path.is_file() {
            return;
        }
        let body = std::fs::read_to_string(&path).expect("docs README");
        assert!(
            body.contains(CANONICAL_VERSION),
            "docs/README.md must mention {} when monorepo sibling present",
            CANONICAL_VERSION
        );
    }

}
