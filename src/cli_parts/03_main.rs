fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run { file, json_errors, args } => cmd_run(file, json_errors, args),
        Commands::Check { file, json_errors } => cmd_check(file, json_errors),
        Commands::Bench { file, em } => cmd_bench(file, em),
        Commands::Em { file, json } => cmd_em(file, json),
        Commands::Build { file, release, emit_llvm, target } => cmd_build(file, release, emit_llvm, target),
        Commands::Dump { kind, file } => cmd_dump(kind, file),
        Commands::Test { file, fuzz } => cmd_test(file, fuzz),
        Commands::Patch { file, diff, json } => cmd_patch(file, diff, json),
        Commands::Reflect { file, symbol } => cmd_reflect(file, symbol),
        Commands::Fmt { file, write } => cmd_fmt(file, write),
        Commands::Outline { file, json } => cmd_outline(file, json),
        Commands::Pkg { install, init } => cmd_pkg(install, init),
        Commands::Lsp => cmd_lsp(),
        Commands::Context { file, symbol, tier } => cmd_context(file, symbol, tier),
        Commands::Replay { file, target } => cmd_replay(file, target),
        Commands::Migrate { file, edition, json } => cmd_migrate(file, edition, json),
    }
}
