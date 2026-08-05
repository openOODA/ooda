
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
        migrate_path_inner(&path, "2026", false).expect("migrate");

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
        migrate_path_inner(&path, "2026", false).expect("migrate");

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
        migrate_path_inner(&path, "2026", false).expect("migrate");
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, src, "should not change already-exhaustive match");
    }

    #[test]
    fn unknown_edition_fails_closed() {
        let path = temp_oo("mig_unknown.oo", "pub fn main() {}");
        let res = migrate_path_inner(&path, "1999", false);
        assert!(res.is_err());
        assert!(format!("{}", res.unwrap_err()).contains("only supports"));
    }

    /// Codemod #2: immutable `let x` later assigned → `let mut x`.
    #[test]
    fn migrates_let_to_let_mut_when_assigned() {
        let src = r#"
pub fn main() {
    let x = 1;
    x = 2;
    println(x);
}
"#;
        let path = temp_oo("mig_let_mut.oo", src);
        migrate_path_inner(&path, "2026", false).expect("migrate");

        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains("let mut x = 1"),
            "expected let→let mut rewrite, got:\n{}",
            after
        );
        assert!(
            !after.contains("let x = 1"),
            "immutable let should be gone:\n{}",
            after
        );

        // Already-mut bindings must not double-insert.
        let mut_src = r#"
pub fn main() {
    let mut y = 1;
    y = 2;
    println(y);
}
"#;
        let path2 = temp_oo("mig_let_mut_already.oo", mut_src);
        migrate_path_inner(&path2, "2026", false).expect("migrate");
        let after2 = std::fs::read_to_string(&path2).unwrap();
        assert_eq!(after2, mut_src, "already-mut should be unchanged");
        assert!(
            !after2.contains("let mut mut "),
            "must not double-insert mut:\n{}",
            after2
        );
    }

    #[test]
    fn immutable_let_without_assign_unchanged() {
        let src = r#"
pub fn main() {
    let z = 41;
    println(z + 1);
}
"#;
        let path = temp_oo("mig_let_no_assign.oo", src);
        migrate_path_inner(&path, "2026", false).expect("migrate");
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, src, "unassigned let must stay immutable");
    }

    #[test]
    fn suggest_let_mut_edits_pure_in_memory() {
        let src = "pub fn main() {\n    let x = 1;\n    x = 2;\n}\n";
        let edits = suggest_let_mut_edits(src).expect("suggest");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].2, "mut ");
        // Apply insert and confirm result
        let (pos, end, text) = &edits[0];
        let mut out = src.to_string();
        out.replace_range(*pos..*end, text);
        assert!(out.contains("let mut x = 1"));
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
