    #[test]
    fn question_mark_unwraps_result() {
        let src = r#"
            pub fn f() -> Result[Int, String] { return Ok(1); }
            pub fn g() -> Result[Int, String] {
                let x = f()?;
                return Ok(x);
            }
            pub fn main() {
                match g() {
                    Ok(v) => println(v),
                    Err(e) => println(e),
                }
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn question_mark_on_non_result_fails() {
        let src = r#"
            pub fn main() {
                let x = 1?;
                println(x);
            }
        "#;
        // may fail parse or type
        let r = check(src);
        assert!(r.is_err(), "1? must fail");
    }
    #[test]
    fn bool_match_true_false_exhaustive() {
        let src = r#"
            pub fn f(b: Bool) -> Int {
                match b {
                    true => 1,
                    false => 0,
                }
            }
            pub fn main() { println(f(true)); }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn bool_match_nonexhaustive_fails() {
        let src = r#"
            pub fn f(b: Bool) -> Int {
                match b {
                    true => 1,
                }
            }
            pub fn main() { println(f(true)); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(err.contains("non-exhaustive") || err.contains("Bool"), "{}", err);
    }
    #[test]
    fn contains_method_typechecks() {
        let src = r#"
            pub fn main() {
                let ok = "hello".contains("ell");
                println(ok);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn question_mark_in_void_fn_fails() {
        let src = r#"
            pub fn f() -> Result[Int, String] { return Ok(1); }
            pub fn main() {
                let x = f()?;
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("`?` only allowed") || err.contains("Result"),
            "void main cannot use ?: {}",
            err
        );
    }
    #[test]
    fn question_mark_err_type_must_match() {
        let src = r#"
            pub fn f() -> Result[Int, String] { return Err("e"); }
            pub fn g() -> Result[Int, Int] {
                let x = f()?;
                return Ok(x);
            }
            pub fn main() {}
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("error type") || err.contains("`?`"),
            "Err types must match: {}",
            err
        );
    }

