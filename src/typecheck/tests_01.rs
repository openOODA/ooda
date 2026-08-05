    #[test]
    fn accepts_while_and_else_if_and_not() {
        let src = r#"
            pub fn main() {
                let mut i = 0;
                while i < 2 {
                    i = i + 1;
                }
                let y = if i > 5 { 9 } else if i > 0 { i } else { 0 };
                let z = if !false { y } else { 0 };
                println(z);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn rejects_out_of_bounds_refinement_return_value() {
        let src = r#"
            pub fn get_port() -> Int[1..65535] {
                return 70000;
            }
            pub fn main() {
                println(get_port());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("70000"),
            "expected refinement return error, got: {}",
            err
        );
    }
    #[test]
    fn rejects_if_else_branch_type_mismatch() {
        let src = r#"
            pub fn main() {
                let x = if true { 1 } else { "nope" };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("incompatible types") || err.contains("if/else"),
            "got: {}",
            err
        );
    }
    #[test]
    fn rejects_unknown_method_on_int() {
        let src = r#"
            pub fn main() {
                let x = 1;
                let y = x.totally_fake();
                println(y);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("unknown method") && err.contains("totally_fake"),
            "got: {}",
            err
        );
    }
    #[test]
    fn rejects_return_type_mismatch() {
        let src = r#"
            pub fn bad() -> Int {
                return "hi";
            }
            pub fn main() {
                println(bad());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("return type") || err.contains("does not match"),
            "got: {}",
            err
        );
    }
    #[test]
    fn rejects_undefined_function_fail_closed() {
        let src = r#"
            pub fn main() {
                let x = totally_missing_builtin(1);
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("undefined function") && err.contains("totally_missing_builtin"),
            "expected undefined function error, got: {}",
            err
        );
    }
    #[test]
    fn fetch_is_typed_as_result() {
        let src = r#"
            pub fn ok(net: &NetCap) {
                let r = fetch(net, "https://example.invalid");
                assert_eq!(r.is_err(), true);
            }
            pub fn main(net: &NetCap) {
                ok(net);
            }
        "#;
        // Must typecheck: fetch returns Result so .is_err is valid
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn rejects_out_of_bounds_refinement_assignment() {
        let src = r#"
            pub fn main() {
                let mut port: Int[1..65535] = 8080;
                port = 70000;
                println(port);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("70000"),
            "expected assignment refinement error, got: {}",
            err
        );
    }
    #[test]
    fn rejects_out_of_bounds_refinement_assignment_in_nested_if() {
        let src = r#"
            pub fn main() {
                let mut port: Int[1..65535] = 8080;
                if true {
                    port = 70000;
                }
                println(port);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("70000"),
            "nested if must still enforce refinement bounds, got: {}",
            err
        );
    }
    #[test]
    fn rejects_out_of_bounds_refinement_assignment_in_while() {
        let src = r#"
            pub fn main() {
                let mut port: Int[1..65535] = 8080;
                while false {
                    port = 0;
                }
                println(port);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("0"),
            "while body must still enforce refinement bounds, got: {}",
            err
        );
    }
    #[test]
    fn rejects_const_expr_out_of_refinement_bounds() {
        let src = r#"
            pub fn main() {
                let port: Int[1..10] = 5 + 6;
                println(port);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("11"),
            "const-folded init must enforce refinement, got: {}",
            err
        );
    }
    #[test]
    fn rejects_int_as_string_return_fail_closed() {
        let src = r#"
            pub fn bad(x: Int) -> String {
                return x;
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("return type") || err.contains("does not match"),
            "Int must not soft-accept as String, got: {}",
            err
        );
    }
    #[test]
    fn rejects_string_eq_int() {
        let src = r#"
            pub fn main() {
                let b = "a" == 1;
                println(b);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("cannot compare") || err.contains("equality"),
            "String == Int must fail, got: {}",
            err
        );
    }
    #[test]
    fn rejects_call_arity_too_few() {
        let src = r#"
            pub fn add(a: Int, b: Int) -> Int {
                return a + b;
            }
            pub fn main() {
                let x = add(1);
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("expects 2 argument") || err.contains("found 1"),
            "arity too few must fail closed, got: {}",
            err
        );
    }

