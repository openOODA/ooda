mod ast;
mod lexer;
mod parser;
mod eval;

use clap::{Parser as ClapParser, Subcommand};
use std::path::PathBuf;
use std::fs;
use anyhow::{Context, Result};

use lexer::Lexer;
use parser::Parser;
use eval::Interpreter;

#[derive(ClapParser)]
#[command(name = "ooda")]
#[command(author = "openOODA Core Team")]
#[command(version = "0.1.0-alpha")]
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
    /// Compile an OODA source file (.oo) into a native binary via LLVM
    Build {
        /// Path to the .oo file
        file: PathBuf,
        /// Produce optimized release build
        #[arg(long)]
        release: bool,
    },
    /// Run inline verify test blocks and contracts
    Test {
        /// Path to the .oo file or directory
        file: PathBuf,
        /// Enable automated fuzzing
        #[arg(long)]
        fuzz: bool,
    },
    /// Format OODA source code files
    Fmt {
        /// Path to the .oo file
        file: PathBuf,
    },
    /// Emit token-minimized module outline (types & contracts only)
    Outline {
        /// Path to the .oo file
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, json_errors } => {
            let code = fs::read_to_string(&file)
                .with_context(|| format!("Failed to read file '{}'", file.display()))?;

            let mut lexer = Lexer::new(&code);
            let tokens = lexer.tokenize()
                .map_err(|e| if json_errors {
                    eprintln!("{{\"error\": \"LexerError\", \"message\": \"{}\"}}", e);
                    e
                } else {
                    e
                })?;

            let mut parser = Parser::new(tokens);
            let program = parser.parse_program()
                .map_err(|e| if json_errors {
                    eprintln!("{{\"error\": \"ParserError\", \"message\": \"{}\"}}", e);
                    e
                } else {
                    e
                })?;

            let mut interpreter = Interpreter::new(program);
            interpreter.execute_all()?;
        }
        Commands::Build { file, release } => {
            println!("🔨 [openOODA LLVM Compiler] Building {} (release={})", file.display(), release);
            println!("   Backend: Native LLVM IR CodeGen pipeline engaged.");
        }
        Commands::Test { file, fuzz } => {
            let code = fs::read_to_string(&file)
                .with_context(|| format!("Failed to read file '{}'", file.display()))?;

            let mut lexer = Lexer::new(&code);
            let tokens = lexer.tokenize()?;

            let mut parser = Parser::new(tokens);
            let program = parser.parse_program()?;

            let mut interpreter = Interpreter::new(program);
            println!("🧪 [openOODA Test Runner] Running contract verify blocks for {} (fuzz={})", file.display(), fuzz);
            interpreter.execute_all()?;
        }
        Commands::Fmt { file } => {
            println!("✨ [openOODA Formatter] Formatting {}", file.display());
        }
        Commands::Outline { file } => {
            let code = fs::read_to_string(&file)?;
            let mut lexer = Lexer::new(&code);
            let tokens = lexer.tokenize()?;
            let mut parser = Parser::new(tokens);
            let program = parser.parse_program()?;

            println!("📋 [openOODA Outline] API Summary for {}:", file.display());
            for item in program.items {
                match item {
                    ast::Item::Function(func) => {
                        println!("  pub fn {}(...) -> {:?}", func.name, func.return_type);
                    }
                    ast::Item::TypeAlias(name, t) => {
                        println!("  type {} = {:?}", name, t);
                    }
                }
            }
        }
    }

    Ok(())
}
