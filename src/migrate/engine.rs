// ===================================================================
// openOODA edition migrator (edition 2026)
//
// Codemods:
// 1) v0.10 → v0.18: exhaustive Result/Option match
//    Insert `_ => process_exit(1)` on non-exhaustive matches.
// 2) v0.10 → v0.20: let-mut for assigned bindings
//    Immutable `let x` that is later assigned becomes `let mut x`.
// ===================================================================
use crate::ast::*;
use crate::lexer::Lexer;
use crate::parser::Parser;
use anyhow::{anyhow, bail, Result};
use std::collections::HashSet;
use std::fs;

/// Summary of a migrate run (for humans and `ooda migrate --json`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MigrateReport {
    pub file: String,
    pub edition: String,
    pub match_wildcard_arms: usize,
    pub let_mut_fixes: usize,
    pub changed: bool,
}

/// CLI-facing wrapper. `ooda migrate <file> --edition <year>` is
/// wired to this in main.rs.
pub struct MigrationEngine;

impl MigrationEngine {
    pub fn migrate_codebase(file_path: &str, target_edition: &str) -> Result<()> {
        let _ = migrate_path_inner(std::path::Path::new(file_path), target_edition, false)?;
        Ok(())
    }
}

/// Path-based entry point (also exported for tests).
pub fn migrate_path(path: &std::path::Path, target_edition: &str) -> Result<()> {
    let _ = migrate_path_inner(path, target_edition, false)?;
    Ok(())
}

/// Pure in-memory let→let mut rewrites for LSP WorkspaceEdit.
/// Each entry is `(byte_start, byte_end, replacement)` over UTF-8 source bytes
/// (half-open range; insert-only edits use `start == end`).
///
/// Stack-oriented: no heap document clone beyond the rewrite vector; callers
/// convert byte offsets to LSP positions without re-parsing.
pub fn suggest_let_mut_edits(source: &str) -> Result<Vec<(usize, usize, String)>> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| anyhow!("suggest_let_mut_edits: lexer: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| anyhow!("suggest_let_mut_edits: parse: {}", e))?;
    let mut rewrites: Vec<(usize, usize, String)> = Vec::new();
    for item in &program.items {
        if let Item::Function(f) = item {
            collect_let_mut_rewrites(&f.body, source, &mut rewrites);
        }
    }
    Ok(rewrites)
}

/// Migrate and print JSON MigrateReport on stdout.
pub fn migrate_path_json(path: &std::path::Path, target_edition: &str) -> Result<MigrateReport> {
    migrate_path_inner(path, target_edition, true)
}

fn migrate_path_inner(path: &std::path::Path, target_edition: &str, json: bool) -> Result<MigrateReport> {
    if target_edition != "2026" {
        bail!(
            "ooda migrate only supports target-edition 2026 in this alpha \
             (got '{}'). Unknown editions fail closed.",
            target_edition
        );
    }

    if !path.exists() {
        bail!("migrate: file not found: {}", path.display());
    }

    let code = fs::read_to_string(path)?;
    let mut lexer = Lexer::new(&code);
    let tokens = lexer
        .tokenize()
        .map_err(|e| anyhow!("migrate: lexer error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| anyhow!("migrate: parser error: {}", e))?;

    // Walk the AST collecting byte ranges that need rewrites.
    // Each rewrite is (pos, end, replacement): replace [pos, end).
    let mut rewrites: Vec<(usize, usize, String)> = Vec::new();
    let mut match_count = 0usize;
    let mut mut_count = 0usize;
    for item in &program.items {
        if let Item::Function(f) = item {
            let before = rewrites.len();
            collect_match_rewrites(&f.body, &code, &mut rewrites);
            match_count += rewrites.len() - before;
            let before = rewrites.len();
            collect_let_mut_rewrites(&f.body, &code, &mut rewrites);
            mut_count += rewrites.len() - before;
        }
    }

    let report = MigrateReport {
        file: path.display().to_string(),
        edition: target_edition.to_string(),
        match_wildcard_arms: match_count,
        let_mut_fixes: mut_count,
        changed: !rewrites.is_empty(),
    };

    if rewrites.is_empty() {
        if json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            println!(
                "✓ [openOODA migrate] {} is already on edition {} (no changes needed).",
                path.display(),
                target_edition
            );
        }
        return Ok(report);
    }

    // Apply in reverse byte order so earlier offsets stay valid.
    rewrites.sort_by(|a, b| b.0.cmp(&a.0));
    let mut new_code = code.clone();
    for (pos, end, text) in &rewrites {
        new_code.replace_range(*pos..*end, text);
    }
    fs::write(path, &new_code)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "🔧 [openOODA migrate] {} (edition {}): {} match wildcard arm(s), {} let→let mut fix(es). \
         Replace each `_ => process_exit(1)` with a real handler when present.",
            path.display(),
            target_edition,
            match_count,
            mut_count
        );
    }
    Ok(report)
}
