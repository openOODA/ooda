mod ast;
mod lexer;
mod parser;
mod eval;
mod diagnostics;
mod fmt;
mod outline;
mod capabilities;
mod codegen;
mod patch;
mod reflect;
mod bench;
mod pkg;

use clap::{Parser as ClapParser, Subcommand};
use std::path::PathBuf;
use std::fs;
use anyhow::{Context, Result};

use lexer::Lexer;
use parser::Parser;
use eval::Interpreter;
use diagnostics::AiDiagnostic;
use capabilities::CapabilityChecker;
use codegen::LlvmCodeGen;

#[derive(ClapParser)]
#[command(name = "ooda")]
#[command(author = "openOODA Core Team")]
#[command(version = "0.1.4-alpha")]
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
                    if json_errors {
                        AiDiagnostic::new("LexerError", &file, 1, 1, format!("{}", e), "Syntax error encountered during tokenization.")
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
                    if json_errors {
                        AiDiagnostic::new("ParserError", &file, 1, 1, format!("{}", e), "Structure error encountered during AST parsing.")
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
                if json_errors {
                    AiDiagnostic::new("CapabilitySecurityViolation", &file, 1, 1, format!("{}", e), "Function attempts I/O without receiving explicit capability token handle.")
                        .with_fix("Grant Capability Token", "Pass &NetCap or &FsCap in parameter list.")
                        .print_json();
                } else {
                    eprintln!("Security Error: {}", e);
                }
                std::process::exit(1);
            }

            let mut interpreter = Interpreter::new(program);
            if let Err(e) = interpreter.execute_all() {
                if json_errors {
                    AiDiagnostic::new("RuntimeContractError", &file, 1, 1, format!("{}", e), "Execution failed or precondition/postcondition contract violated.")
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
        Commands::Build { file, release, emit_llvm } => {
            let code = fs::read_to_string(&file)?;
            let mut lexer = Lexer::new(&code);
            let tokens = lexer.tokenize()?;
            let mut parser = Parser::new(tokens);
            let program = parser.parse_program()?;

            CapabilityChecker::check_program(&program)?;

            let llvm_ir = LlvmCodeGen::emit_llvm_ir(&program);

            let out_ll = file.with_extension("ll");
            fs::write(&out_ll, &llvm_ir)?;

            println!("🔨 [openOODA LLVM Compiler] Compiled {} -> {} (release={})", file.display(), out_ll.display(), release);
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
                println!("📦 openOODA Package Manager v0.1.4-alpha. Use --init or --install.");
            }
        }
    }

    Ok(())
}
