#[derive(ClapParser)]
#[command(name = "ooda")]
#[command(author = "openOODA Core Team")]
#[command(version = "0.180.0-alpha")]
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

