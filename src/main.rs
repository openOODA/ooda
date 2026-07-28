use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
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
    /// Run an OODA source file instantly using the JIT interpreter
    Run {
        /// Path to the .oo file
        file: PathBuf,
        /// Output machine-readable JSON errors for AI auto-fixing
        #[arg(long)]
        json_errors: bool,
    },
    /// Compile an OODA source file into a native binary via LLVM
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
        file: Option<PathBuf>,
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, json_errors } => {
            println!("⚡ [openOODA JIT] Running {} (json_errors={})", file.display(), json_errors);
        }
        Commands::Build { file, release } => {
            println!("🔨 [openOODA Compiler] Building {} (release={})", file.display(), release);
        }
        Commands::Test { file, fuzz } => {
            let target = file.map_or_else(|| "all tests".into(), |p| p.display().to_string());
            println!("🧪 [openOODA Test Runner] Running {} (fuzz={})", target, fuzz);
        }
        Commands::Fmt { file } => {
            println!("✨ [openOODA Formatter] Formatting {}", file.display());
        }
        Commands::Outline { file } => {
            println!("📋 [openOODA Outline] Extracting API outline for {}", file.display());
        }
    }

    Ok(())
}
