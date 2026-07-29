// openOODA CLI binary — logic modules live in the `ooda` library.
use ooda::ast::Program;
use ooda::bench;
use ooda::capabilities::CapabilityChecker;
use ooda::codegen::LlvmCodeGen;
use ooda::codegen_c::{runtime_c_path, CCodeGen};
use ooda::codegen_wasm::WasmCodeGen;
use ooda::diagnostics::{AiDiagnostic, DiagnosticTimings};
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
#[command(version = "0.30.0-alpha")]
#[command(about = "The OODA Programming Language Compiler & Toolchain", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Extract `line:col` from messages emitted by the parser, typechecker,
/// and capability checker. Recognises these formats in priority order:
///
/// 1. `at LINE:COL`            — parser, typechecker (`Type error at 4:26: …`)
/// 2. `at line LINE, col COL`  — capability checker (`… at line 2, col 52.`)
/// 3. `line N`                 — fallback, column defaults to 1
fn parse_loc(msg: &str) -> (usize, usize) {
    // Format 1: ` at LINE:COL `
    if let Some(idx) = msg.find(" at ") {
        let rest = &msg[idx + 4..];
        let coords: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == ':')
            .collect();
        let parts: Vec<&str> = coords.split(':').collect();
        if parts.len() >= 2 {
            if let (Ok(l), Ok(c)) = (parts[0].parse(), parts[1].parse()) {
                return (l, c);
            }
        }
    }
    // Format 2: ` at line LINE, col COL `
    if let Some(idx) = msg.find(" at line ") {
        let after_line = &msg[idx + " at line ".len()..];
        let line_str: String = after_line
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(l) = line_str.parse::<usize>() {
            if let Some(comma_idx) = after_line.find(" col ") {
                let after_col = &after_line[comma_idx + " col ".len()..];
                let col_str: String = after_col
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(c) = col_str.parse::<usize>() {
                    return (l, c);
                }
            }
            // `at line LINE` with no column → column 1.
            return (l, 1);
        }
    }
    // Format 3: `line N`
    if let Some(idx) = msg.find("line ") {
        let rest = &msg[idx + 5..];
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(l) = num.parse() {
            return (l, 1);
        }
    }
    (1, 1)
}

#[cfg(test)]
mod parse_loc_tests {
    use super::parse_loc;

    #[test]
    fn extracts_at_line_col_format() {
        let (l, c) = parse_loc("Type error at 4:26: undefined variable 'foo'");
        assert_eq!((l, c), (4, 26));
    }

    #[test]
    fn extracts_capability_at_line_comma_col_format() {
        let msg = "Security Capability Violation: Function 'rogue_fetch' calls sealed effectful builtin 'fetch' which requires a &NetCap parameter, but none was declared at line 2, col 52. Default-deny: grant the capability token explicitly.";
        let (l, c) = parse_loc(msg);
        assert_eq!((l, c), (2, 52));
    }

    #[test]
    fn extracts_fallback_line_format() {
        let (l, c) = parse_loc("Expected token at line 7");
        assert_eq!((l, c), (7, 1));
    }

    #[test]
    fn defaults_to_one_one_when_no_match() {
        let (l, c) = parse_loc("totally unstructured error message");
        assert_eq!((l, c), (1, 1));
    }
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
    /// Calculate and display Energy-Maneuverability (E-M) Specific Excess Power (Ps) breakdown
    Em {
        /// Path to the .oo file
        file: PathBuf,
    },
    /// Compile .oo to native (CHS→C+gcc preferred; LLVM integer subset fallback)
    Build {
        /// Path to the .oo file
        file: PathBuf,
        /// Reserved: optimized release build (ignored in this alpha — no-op)
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
            let (program, _timings) = match load_and_analyze(&file, json_errors) {
                Ok(p) => p,
                Err(code) => std::process::exit(code),
            };

            let mut interpreter = Interpreter::new(program).with_argv(args);
            if let Err(e) = interpreter.execute_all() {
                let msg = format!("{}", e);
                let (line, col) = parse_loc(&msg);
                if json_errors {
                    AiDiagnostic::new("RuntimeContractError", &file, line, col, msg.clone(), "Execution failed or precondition/postcondition contract violated.")
                        .with_fix(
                            "Satisfy requires / handle ensures",
                            "Check call-site arguments against `requires`; error includes call site line:col when available.",
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
                        d = d.with_timings(t.parse_us, t.capability_us + t.typecheck_us);
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
        Commands::Bench { file, em: _ } => {
            bench::run_empirical_verification_suite(&file)?;
        }
        Commands::Em { file } => {
            let code_len = fs::metadata(&file).map(|m| m.len() as usize).unwrap_or(2048);
            let em_savings = ooda::em::EmSavings::calculate(500, 300, code_len, None);
            println!("{}", em_savings.display_summary());
        }
        Commands::Build { file, release: _, emit_llvm, target } => {
            let program = load_program(&file)
                .with_context(|| format!("Failed to load '{}'", file.display()))?;
            CapabilityChecker::check_program(&program)?;
            TypeChecker::check_program(&program)?;

            let target_l = target.to_lowercase();
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
                match CCodeGen::build_native(&program, &out_bin, &rt) {
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

            match try_native_link(&out_ll, &out_bin) {
                NativeLinkResult::Ok => {
                    println!(
                        "🚀 [openOODA Native Build] Native executable: {}",
                        out_bin.display()
                    );
                }
                NativeLinkResult::ToolFailed { tool, detail } => {
                    eprintln!(
                        "⚠️  [openOODA Native Build] {} link failed (IR kept at {}): {}",
                        tool,
                        out_ll.display(),
                        detail
                    );
                }
                NativeLinkResult::NoTool => {
                    println!(
                        "💡 [openOODA Native Build] No clang in PATH; IR only at {}. Use --target c with gcc, or install clang.",
                        out_ll.display()
                    );
                }
            }

            if emit_llvm {
                println!("\n--- Generated LLVM IR ---\n{}", llvm_ir);
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
        Commands::Patch { file, diff } => {
            patch::apply_patch(&file, &diff)?;
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
        Commands::Outline { file } => {
            let code = fs::read_to_string(&file)?;
            let mut lexer = Lexer::new(&code);
            let tokens = lexer.tokenize()?;
            let mut parser = Parser::new(tokens);
            let program = parser.parse_program()?;

            let summary = outline::generate_outline(&program);
            println!("📋 [openOODA Outline] API Summary for {}:\n{}", file.display(), summary);
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
        Commands::Migrate { file, edition } => {
            migrate::MigrationEngine::migrate_codebase(&file.display().to_string(), &edition)?;
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
    fn total_us(&self) -> u128 {
        self.parse_us
            .saturating_add(self.capability_us)
            .saturating_add(self.typecheck_us)
    }
    fn to_diag_timings(&self) -> DiagnosticTimings {
        // Only attach parse + typecheck; the capability pass is folded
        // into typecheck conceptually (both run after parse).
        DiagnosticTimings {
            parse_us: self.parse_us,
            check_us: self.capability_us.saturating_add(self.typecheck_us),
        }
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
            AiDiagnostic::new(
                "CapabilitySecurityViolation",
                file,
                line,
                col,
                msg.clone(),
                "Function attempts I/O without receiving explicit capability token handle.",
            )
            .with_fix(
                "Grant Capability Token",
                "fn f(net: &NetCap, ...) { ... fetch(url); }  // pass net from main()",
            )
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
            let fix = if msg.contains("must-use") || msg.contains("unused Result") {
                (
                    "Handle Result/Option",
                    "let r = f(); match r { Ok(v) => ..., Err(e) => ... }",
                )
            } else if msg.contains("non-exhaustive match") {
                (
                    "Cover all variants",
                    "match r { Ok(v) => ..., Err(e) => ... }  // or `_ => ...`",
                )
            } else if msg.contains("immutable") {
                ("Use let mut", "let mut x = ...; x = new_value;")
            } else {
                (
                    "Fix types",
                    "Ensure operands and annotations agree (Int/Float/String/Bool/caps).",
                )
            };
            AiDiagnostic::new(
                "TypeError",
                file,
                line,
                col,
                msg,
                "Static type mismatch detected before execution.",
            )
            .with_fix(fix.0, fix.1)
            .with_timings(parse_us, capability_us.saturating_add(typecheck_us))
            .print_json();
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
    const CANONICAL_VERSION: &str = "v0.29.0-alpha";
    /// clap's `#[command(version = ...)]` carries no `v` prefix
    /// (Cargo's `version = "..."` also doesn't). Strip it before
    /// comparing to the canonical form so the test fails loudly if
    /// either side is renamed.
    const CANONICAL_VERSION_NO_V: &str = "0.29.0-alpha";

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

}
