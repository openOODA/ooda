mod ast;
mod lexer;
mod parser;
mod eval;
mod diagnostics;
mod fmt;
mod outline;
mod capabilities;
mod typecheck;
mod codegen;
mod patch;
mod reflect;
mod bench;
mod pkg;
mod lsp;
mod context;
mod replay;
mod migrate;

use clap::{Parser as ClapParser, Subcommand};
use std::path::PathBuf;
use std::fs;
use anyhow::{Context, Result};

use lexer::Lexer;
use parser::Parser;
use eval::Interpreter;
use diagnostics::AiDiagnostic;
use capabilities::CapabilityChecker;
use typecheck::TypeChecker;
use codegen::LlvmCodeGen;

#[derive(ClapParser)]
#[command(name = "ooda")]
#[command(author = "openOODA Core Team")]
#[command(version = "0.11.0-alpha")]
#[command(about = "The OODA Programming Language Compiler & Toolchain", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Extract `line:col` from messages like `... at 12:3 ...` or `line 12`.
fn parse_loc(msg: &str) -> (usize, usize) {
    // Prefer "at LINE:COL"
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
    // "line N"
    if let Some(idx) = msg.find("line ") {
        let rest = &msg[idx + 5..];
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(l) = num.parse() {
            return (l, 1);
        }
    }
    (1, 1)
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
    },
    /// Run empirical benchmarks & claim verification suite on .oo file
    Bench {
        /// Path to the .oo file
        file: PathBuf,
    },
    /// Compile an OODA source file (.oo) into a native binary via LLVM
    Build {
        /// Path to the .oo file
        file: PathBuf,
        /// Produce optimized release build
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, json_errors } => {
            let code = match fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    if json_errors {
                        AiDiagnostic::new("FileNotFound", &file, 1, 1, format!("{}", e), "Ensure the file path exists and is readable.").print_json();
                    } else {
                        eprintln!("Error: Failed to read file '{}': {}", file.display(), e);
                    }
                    std::process::exit(1);
                }
            };

            let mut lexer = Lexer::new(&code);
            let tokens = match lexer.tokenize() {
                Ok(t) => t,
                Err(e) => {
                    let msg = format!("{}", e);
                    let (line, col) = parse_loc(&msg);
                    if json_errors {
                        AiDiagnostic::new("LexerError", &file, line, col, msg, "Syntax error encountered during tokenization.")
                            .with_fix("Fix syntax token", "Ensure brackets, quotes, and operators are balanced.")
                            .print_json();
                    } else {
                        eprintln!("Lexer Error: {}", e);
                    }
                    std::process::exit(1);
                }
            };

            let mut parser = Parser::new(tokens);
            let program = match parser.parse_program() {
                Ok(p) => p,
                Err(e) => {
                    let msg = format!("{}", e);
                    let (line, col) = parse_loc(&msg);
                    if json_errors {
                        AiDiagnostic::new("ParserError", &file, line, col, msg, "Structure error encountered during AST parsing.")
                            .with_fix("Fix AST structure", "Check function headers, contracts, and statement semicolons.")
                            .print_json();
                    } else {
                        eprintln!("Parser Error: {}", e);
                    }
                    std::process::exit(1);
                }
            };

            // Capability Security Verifier
            if let Err(e) = CapabilityChecker::check_program(&program) {
                let msg = format!("{}", e);
                let (line, col) = parse_loc(&msg);
                if json_errors {
                    AiDiagnostic::new("CapabilitySecurityViolation", &file, line, col, msg, "Function attempts I/O without receiving explicit capability token handle.")
                        .with_fix("Grant Capability Token", "Pass &NetCap or &FsCap in parameter list.")
                        .print_json();
                } else {
                    eprintln!("Security Error: {}", e);
                }
                std::process::exit(1);
            }

            // Static type checker
            if let Err(e) = TypeChecker::check_program(&program) {
                let msg = format!("{}", e);
                let (line, col) = parse_loc(&msg);
                if json_errors {
                    AiDiagnostic::new("TypeError", &file, line, col, msg, "Static type mismatch detected before execution.")
                        .with_fix("Fix types", "Ensure operands and annotations agree (Int/Float/String/Bool/caps).")
                        .print_json();
                } else {
                    eprintln!("Type Error: {}", e);
                }
                std::process::exit(1);
            }

            let mut interpreter = Interpreter::new(program);
            if let Err(e) = interpreter.execute_all() {
                let msg = format!("{}", e);
                let (line, col) = parse_loc(&msg);
                if json_errors {
                    AiDiagnostic::new("RuntimeContractError", &file, line, col, msg, "Execution failed or precondition/postcondition contract violated.")
                        .with_fix("Enforce contract preconditions", "Verify argument values passed into functions.")
                        .print_json();
                } else {
                    eprintln!("Runtime Error: {}", e);
                }
                std::process::exit(1);
            }
        }
        Commands::Bench { file } => {
            bench::run_empirical_verification_suite(&file)?;
        }
        Commands::Build { file, release, emit_llvm, target } => {
            let code = fs::read_to_string(&file)?;
            let mut lexer = Lexer::new(&code);
            let tokens = lexer.tokenize()?;
            let mut parser = Parser::new(tokens);
            let program = parser.parse_program()?;

            CapabilityChecker::check_program(&program)?;
            TypeChecker::check_program(&program)?;

            if target.to_lowercase() == "wasm" {
                anyhow::bail!(
                    "WASM target is not implemented in this alpha (no bytecode emission). \
                     Use `ooda build` for the integer-subset LLVM IR backend, or `ooda run` for the interpreter."
                );
            }

            let llvm_ir = LlvmCodeGen::emit_llvm_ir(&program).with_context(|| {
                format!(
                    "LLVM integer-subset codegen failed for '{}'. \
                     Supported: Int/Bool straight-line functions, println(Int), main. \
                     Use `ooda run` for String programs and full language surface.",
                    file.display()
                )
            })?;

            let out_ll = file.with_extension("ll");
            let out_bin = file.with_extension("");
            fs::write(&out_ll, &llvm_ir)?;

            println!(
                "🔨 [openOODA LLVM Compiler] Generated LLVM IR: {}",
                out_ll.display()
            );

            // Attempt native binary assembly via clang
            let clang_res = std::process::Command::new("clang")
                .arg(&out_ll)
                .arg("-o")
                .arg(&out_bin)
                .output();

            if let Ok(out) = clang_res {
                if out.status.success() {
                    println!("🚀 [openOODA Native Build] Successfully compiled native executable binary: {}", out_bin.display());
                } else {
                    println!("⚠️ [openOODA Native Build] Clang assembly warning/skipped (IR saved at {})", out_ll.display());
                }
            } else {
                println!("💡 [openOODA Native Build] Clang not found in PATH; LLVM IR emitted at {}", out_ll.display());
            }

            if emit_llvm {
                println!("\n--- Generated LLVM IR ---\n{}", llvm_ir);
            }
        }
        Commands::Test { file, fuzz } => {
            let code = fs::read_to_string(&file)
                .with_context(|| format!("Failed to read file '{}'", file.display()))?;

            let mut lexer = Lexer::new(&code);
            let tokens = lexer.tokenize()?;

            let mut parser = Parser::new(tokens);
            let program = parser.parse_program()?;

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
                println!("📦 openOODA Package Manager v0.1.5-alpha. Use --init or --install.");
            }
        }
        Commands::Lsp => {
            lsp::LspDaemon::start()?;
        }
        Commands::Context { file, symbol, tier } => {
            let ctx = context::ContextEngine::build_micro_context(&file.display().to_string(), &symbol, &tier)?;
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
