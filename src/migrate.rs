// ===================================================================
// openOODA edition migrator (v0.29.0-alpha, first real implementation).
//
// Implements a single high-leverage codemod:
//
//   v0.10 → v0.18: exhaustive Result/Option match
//   -----------------------------------------------------------------
//   The v0.18 release made match on Result/Option require BOTH variant
//   arms (or a wildcard). Code written against the v0.10 "loose match"
//   rules (e.g. `match rs { Ok(v) => ... }` with no Err arm) no longer
//   typechecks. `ooda migrate --edition 2026` finds every such match
//   expression and inserts a wildcard arm `_ => process_exit(1)` that
//   makes the typecheck pass while leaving a loud runtime signal at
//   the previously-unhandled variant. The user can then replace the
//   wildcard with a proper arm.
//
//   The wildcard is a STOPGAP. It fails loudly. That's the design
//   intent — we want users to notice the migration and write a real
//   Err handler.
// ===================================================================
use crate::ast::*;
use crate::lexer::Lexer;
use crate::parser::Parser;
use anyhow::{anyhow, bail, Result};
use std::fs;
use std::path::Path;

/// CLI-facing wrapper. `ooda migrate <file> --edition <year>` is
/// wired to this in main.rs.
pub struct MigrationEngine;

impl MigrationEngine {
    pub fn migrate_codebase(file_path: &str, target_edition: &str) -> Result<()> {
        migrate_path_inner(std::path::Path::new(file_path), target_edition)
    }
}

/// Path-based entry point (also exported for tests).
pub fn migrate_path(path: &std::path::Path, target_edition: &str) -> Result<()> {
    migrate_path_inner(path, target_edition)
}

fn migrate_path_inner(path: &std::path::Path, target_edition: &str) -> Result<()> {
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
    let mut program = parser
        .parse_program()
        .map_err(|e| anyhow!("migrate: parser error: {}", e))?;

    // Walk the AST collecting byte ranges that need rewrites.
    // Each rewrite is (insert_pos, replacement_text). We use a single
    // ordered pass and apply in reverse to keep offsets stable.
    let mut rewrites: Vec<(usize, usize, String)> = Vec::new();
    for item in &program.items {
        if let Item::Function(f) = item {
            collect_match_rewrites(&f.body, &code, &mut rewrites);
        }
    }

    if rewrites.is_empty() {
        println!(
            "✓ [openOODA migrate] {} is already on edition {} (no changes needed).",
            path.display(),
            target_edition
        );
        return Ok(());
    }

    // Apply in reverse byte order so earlier byte offsets stay
    // valid. Each rewrite is (pos, end, replacement): we replace
    // bytes [pos, end) with `replacement`. This lets us both
    // overwrite an existing `,` and insert new text cleanly.
    rewrites.sort_by(|a, b| b.0.cmp(&a.0));
    let mut new_code = code.clone();
    for (pos, end, text) in &rewrites {
        new_code.replace_range(*pos..*end, text);
    }
    fs::write(path, &new_code)?;

    println!(
        "🔧 [openOODA migrate] Inserted {} wildcard arm(s) in {} (target: edition {}). \
         Replace each `_ => process_exit(1)` with a real handler.",
        rewrites.len(),
        path.display(),
        target_edition
    );
    Ok(())
}

/// Walk a block collecting rewrites for non-exhaustive match
/// expressions on Result/Option. Each rewrite is (pos, end,
/// replacement) — a half-open byte range to overwrite.
fn collect_match_rewrites(
    block: &Block,
    source: &str,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    for stmt in &block.stmts {
        match stmt {
            Statement::Let { init, .. } => {
                collect_in_expr(init, source, rewrites);
            }
            Statement::Assign { value, .. } => {
                collect_in_expr(value, source, rewrites);
            }
            Statement::Return(Some(expr), _) => {
                collect_in_expr(expr, source, rewrites);
            }
            Statement::Return(None, _) => {}
            Statement::Expr(expr, _) => {
                collect_in_expr(expr, source, rewrites);
            }
            Statement::While { cond, body, .. } => {
                collect_in_expr(cond, source, rewrites);
                collect_match_rewrites(body, source, rewrites);
            }
        }
    }
    if let Some(expr) = &block.expr {
        collect_in_expr(expr, source, rewrites);
    }
}

fn collect_in_expr(
    expr: &Expression,
    source: &str,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    match expr {
        Expression::Literal(_, _) | Expression::Variable(_, _) => {}
        Expression::Binary { left, right, .. } => {
            collect_in_expr(left, source, rewrites);
            collect_in_expr(right, source, rewrites);
        }
        Expression::Call { args, .. } => {
            for a in args {
                collect_in_expr(a, source, rewrites);
            }
        }
        Expression::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_in_expr(cond, source, rewrites);
            collect_match_rewrites(then_branch, source, rewrites);
            if let Some(eb) = else_branch {
                collect_match_rewrites(eb, source, rewrites);
            }
        }
        Expression::Unary { expr, .. } => {
            collect_in_expr(expr, source, rewrites);
        }
        Expression::While { cond, body, .. } => {
            collect_in_expr(cond, source, rewrites);
            collect_match_rewrites(body, source, rewrites);
        }
        Expression::Match { expr, arms, span } => {
            // Recurse into the scrutinee.
            collect_in_expr(expr, source, rewrites);

            // Detect non-exhaustive Result/Option match.
            let mut has_ok = false;
            let mut has_err = false;
            let mut has_some = false;
            let mut has_none = false;
            let mut has_wildcard = false;
            for arm in arms {
                match &arm.pattern {
                    Pattern::Wildcard => has_wildcard = true,
                    Pattern::Variant { name, .. } => match name.as_str() {
                        "Ok" => has_ok = true,
                        "Err" => has_err = true,
                        "Some" => has_some = true,
                        "None" => has_none = true,
                        _ => {}
                    },
                    Pattern::Literal(_) => {}
                }
            }
            let needs_fix = (has_ok && !has_err)
                || (has_err && !has_ok)
                || (has_some && !has_none)
                || (has_none && !has_some);
            if !needs_fix || has_wildcard {
                return;
            }

            // Locate the byte range of the match-block's closing `}`.
            // We start at span.line/span.col (which may point at the
            // closing brace itself) and walk to the matching `}`.
            if let Some(close_pos) = find_matching_rbrace(source, span.line, span.col) {
                // Look backward from the close-brace, skipping
                // whitespace (spaces, tabs, newlines), for a `,`
                // separator. If we find one, replace it with our
                // wildcard arm so we don't end up with two commas.
                // Otherwise insert a comma + the arm before the
                // close-brace.
                let bytes = source.as_bytes();
                let mut j = close_pos;
                while j > 0
                    && (bytes[j - 1] == b' '
                        || bytes[j - 1] == b'\t'
                        || bytes[j - 1] == b'\n'
                        || bytes[j - 1] == b'\r')
                {
                    j -= 1;
                }
                if j > 0 && bytes[j - 1] == b',' {
                    // The previous arm already has a trailing `,`
                    // (the parser consumed it as a separator). We
                    // INSERT our arm after that comma; the original
                    // comma is kept as the separator.
                    rewrites.push((
                        j,
                        j,
                        " _ => process_exit(1),".to_string(),
                    ));
                } else {
                    // No trailing comma — insert our arm followed
                    // by a comma, before the close-brace.
                    rewrites.push((
                        close_pos,
                        close_pos,
                        " _ => process_exit(1),".to_string(),
                    ));
                }
            } else {
                // Could not locate a close-brace — skip this match.
                // The user will still see the typecheck error pointing
                // at the offending span, which is the next-best signal.
            }
        }
        Expression::StructLit { .. } => {}
    }
}

/// Find the byte offset of the `}` that closes the *match block*
/// whose AST span is at (1-indexed) line / col. The hint position
/// may point anywhere in the match expression (often the closing
/// brace or one of the arms). We treat the hint as *outside* the
/// block (depth = 0) and scan forward: the next time depth returns
/// to 0 we are at the matching `}`. Skips strings and comments.
fn find_matching_rbrace(source: &str, line: usize, col: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut idx = 0usize;
    for _ in 0..line.saturating_sub(1) {
        while idx < bytes.len() && bytes[idx] != b'\n' {
            idx += 1;
        }
        if idx < bytes.len() {
            idx += 1;
        }
    }
    idx += col.saturating_sub(1);
    if idx >= bytes.len() {
        return None;
    }

        // The hint may sit on the open-brace, the close-brace, or
    // anywhere between. If it sits on the close-brace itself,
    // that's already the answer. Otherwise treat it as outside the
    // block (depth 0) and scan forward for the first time depth
    // returns to 0.
    if bytes.get(idx) == Some(&b'}') {
        return Some(idx);
    }
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut i = idx;
    while i < bytes.len() {
        let b = bytes[i];
        if in_line_comment {
            if b == b'\n' {
                in_line_comment = false;
            }
        } else if in_block_comment {
            if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                in_block_comment = false;
                i += 1;
            }
        } else if in_string {
            if b == b'"' {
                in_string = false;
            } else if b == b'\\' && i + 1 < bytes.len() {
                i += 1;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                    in_line_comment = true;
                    i += 1;
                }
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                    in_block_comment = true;
                    i += 1;
                }
                b'{' => {
                    depth += 1;
                }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    eprintln!("[debug] no rbrace found, returning None");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rbrace_finds_matching_close() {
        let src = "match r { Ok(v) => v, Err(e) => 0 }";
        // match starts at line 1, col 1.
        let pos = find_matching_rbrace(src, 1, 1).expect("should find rbrace");
        // The matching close-brace is the very last `}`.
        assert_eq!(pos, src.len() - 1);
    }

    #[test]
    fn rbrace_handles_nested_braces() {
        let src = "match r { Ok(Some(v)) => v, Err(_) => { let x = 1; x } }";
        let pos = find_matching_rbrace(src, 1, 1).expect("should find rbrace");
        assert_eq!(pos, src.len() - 1);
    }

    #[test]
    fn rbrace_skips_strings_and_comments() {
        let src = "match r { Ok(_) => \"}\", // } comment\nErr(_) => 0 }";
        let pos = find_matching_rbrace(src, 1, 1).expect("should find rbrace");
        assert_eq!(pos, src.len() - 1);
    }
}
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::io::Write;

    fn temp_oo(name: &str, src: &str) -> std::path::PathBuf {
        let base = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let dir = base.join(".cache").join(format!("ooda-mig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(src.as_bytes()).unwrap();
        path
    }

    #[test]
    fn migrates_v0_10_non_exhaustive_result_match() {
        let src = r#"
pub fn main() {
    let r: Result[Int, String] = Ok(1);
    match r {
        Ok(v) => println(v),
    }
}
"#;
        let path = temp_oo("mig_result.oo", src);
        migrate_path_inner(&path, "2026").expect("migrate");

        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains(", _ => process_exit(1)"),
            "expected inserted wildcard arm, got:\n{}",
            after
        );

        // The migrated file now parses AND typechecks.
        let mut l = crate::lexer::Lexer::new(&after);
        let toks = l.tokenize().expect("lex");
        let mut p = crate::parser::Parser::new(toks);
        let prog = p.parse_program().expect("parse");
        crate::typecheck::TypeChecker::check_program(&prog)
            .expect("typecheck after migrate should pass");
    }

    #[test]
    fn migrates_v0_10_non_exhaustive_option_match() {
        let src = r#"
pub fn main() {
    let o: Option[Int] = Some(1);
    match o {
        Some(v) => println(v),
    }
}
"#;
        let path = temp_oo("mig_option.oo", src);
        migrate_path_inner(&path, "2026").expect("migrate");

        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains(", _ => process_exit(1)"),
            "expected inserted wildcard arm, got:\n{}",
            after
        );
    }

    #[test]
    fn already_exhaustive_match_is_unchanged() {
        let src = r#"
pub fn main() {
    let r: Result[Int, String] = Ok(1);
    match r {
        Ok(v) => println(v),
        Err(e) => println(e),
    }
}
"#;
        let path = temp_oo("mig_already.oo", src);
        migrate_path_inner(&path, "2026").expect("migrate");
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, src, "should not change already-exhaustive match");
    }

    #[test]
    fn unknown_edition_fails_closed() {
        let path = temp_oo("mig_unknown.oo", "pub fn main() {}");
        let res = migrate_path_inner(&path, "1999");
        assert!(res.is_err());
        assert!(format!("{}", res.unwrap_err()).contains("only supports"));
    }
}

#[cfg(test)]
mod debug_test {
    #[test]
    fn debug_rbrace() {
        let src = "match r { Ok(v) => v, Err(e) => 0 }";
        let pos = super::find_matching_rbrace(src, 1, 1);
        eprintln!("src={:?}", src);
        eprintln!("src.len()={}", src.len());
        eprintln!("returned pos={:?}", pos);
        eprintln!("expected={}", src.len() - 1);
        for (i, b) in src.as_bytes().iter().enumerate() {
            eprintln!("  pos {}: {:?}", i, *b as char);
        }
    }
}

#[cfg(test)]
mod _rbrace_debug_disabled {}
