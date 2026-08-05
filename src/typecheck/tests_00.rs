
    fn check(src: &str) -> Result<()> {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize()?;
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse_program()?;
        TypeChecker::check_program(&program)
    }
    #[test]
    fn rejects_bool_plus_int() {
        let src = r#"
            pub fn main() {
                let x = true + 1;
            }
        "#;
        assert!(check(src).is_err());
    }
    #[test]
    fn accepts_int_arith() {
        let src = r#"
            pub fn add(a: Int, b: Int) -> Int {
                return a + b;
            }
            pub fn main() {
                let x = add(1, 2);
            }
        "#;
        assert!(check(src).is_ok());
    }
    #[test]
    fn rejects_undefined_variable() {
        let src = r#"
            pub fn main() {
                let x = missing_var + 1;
            }
        "#;
        assert!(check(src).is_err());
    }
    #[test]
    fn old_must_reference_a_parameter() {
        // `old(undefined_var)` is undefined — should fail with a
        // specific old() error.
        let src = r#"
            pub fn bad(x: Int) -> Int
                ensures result == old(undefined_var) + 1
            {
                return x + 1;
            }
            pub fn main() { println(bad(1)); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("`old(undefined_var)`"),
            "expected specific old() error, got: {}",
            err
        );
        assert!(
            err.contains("references no parameter"),
            "expected 'no parameter' hint, got: {}",
            err
        );
    }
    #[test]
    fn old_with_real_parameter_typechecks() {
        let src = r#"
            pub fn increment(x: Int) -> Int
                ensures result == old(x) + 1
            {
                return x + 1;
            }
            pub fn main() { println(increment(1)); }
        "#;
        assert!(check(src).is_ok(), "expected ok, got: {:?}", check(src).err());
    }
    #[test]
    fn type_error_includes_real_source_span() {
        // `missing_var` is on line 4, col 26 (after 12 spaces of indent).
        let src = "pub fn main() {\n    let x = 1;\n    let y = 2;\n    let z = missing_var;\n}\n";
        let err = check(src).unwrap_err();
        let msg = format!("{}", err);
        // The error message must carry the actual line:col of the
        // offending identifier so --json-errors can surface it.
        assert!(
            msg.contains("at 4:"),
            "expected error to carry span line 4, got: {}",
            msg
        );
        assert!(
            msg.contains("missing_var"),
            "expected error to name the variable, got: {}",
            msg
        );
    }
    #[test]
    fn rejects_unused_result_must_use() {
        let src = r#"
            pub fn get() -> Result[Int, String] {
                return Ok(1);
            }
            pub fn main() {
                get();
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("must-use") || err.contains("unused"),
            "got: {}",
            err
        );
    }
    #[test]
    fn rejects_let_underscore_result_must_use() {
        let src = r#"
            pub fn get() -> Result[Int, String] {
                return Ok(1);
            }
            pub fn main() {
                let _ = get();
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("must-use") || err.contains("unused"),
            "got: {}",
            err
        );
    }
    #[test]
    fn rejects_nonexhaustive_result_match() {
        let src = r#"
            pub fn main() {
                let r = Ok(1);
                let x = match r {
                    Ok(v) => v,
                };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("non-exhaustive") || err.contains("Err"),
            "got: {}",
            err
        );
    }
    #[test]
    fn accepts_exhaustive_result_match() {
        let src = r#"
            pub fn main() {
                let r = Ok(1);
                let x = match r {
                    Ok(v) => v,
                    Err(e) => 0,
                };
                println(x);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn rejects_assign_to_immutable_let() {
        let src = r#"
            pub fn main() {
                let x = 1;
                x = 2;
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("immutable") || err.contains("let mut"),
            "got: {}",
            err
        );
    }
    #[test]
    fn accepts_assign_to_let_mut() {
        let src = r#"
            pub fn main() {
                let mut x = 1;
                x = 2;
                println(x);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn rejects_out_of_bounds_refinement_initializer() {
        let src = r#"
            pub fn main() {
                let port: Int[1..65535] = 99999;
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99999"),
            "expected static refinement error, got: {}",
            err
        );
    }
    #[test]
    fn accepts_option_some_none_match() {
        let src = r#"
            pub fn main() {
                let o = Some(1);
                let x = match o {
                    Some(v) => v,
                    None => 0,
                };
                println(x);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn rejects_nonexhaustive_option_match() {
        let src = r#"
            pub fn main() {
                let o = Some(1);
                let x = match o {
                    Some(v) => v,
                };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(err.contains("non-exhaustive") || err.contains("None"), "{}", err);
    }

