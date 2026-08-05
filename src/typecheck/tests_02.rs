    #[test]
    fn rejects_call_arity_too_many() {
        let src = r#"
            pub fn add(a: Int, b: Int) -> Int {
                return a + b;
            }
            pub fn main() {
                let x = add(1, 2, 3);
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("expects 2 argument") || err.contains("found 3"),
            "arity too many must fail closed, got: {}",
            err
        );
    }
    #[test]
    fn rejects_zero_param_call_with_args() {
        let src = r#"
            pub fn conf() -> Int {
                return 1;
            }
            pub fn main() {
                let x = conf(99);
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("expects 0 argument") || err.contains("found 1"),
            "zero-param fn with args must fail, got: {}",
            err
        );
    }
    #[test]
    fn println_varargs_still_typechecks() {
        let src = r#"
            pub fn main() {
                println("a", 1);
                println(1);
                println();
            }
        "#;
        assert!(
            check(src).is_ok(),
            "println is varargs: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn rejects_int_plus_float_mixed_arith() {
        let src = r#"
            pub fn main() {
                let x = 1 + 2.0;
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("matching numeric")
                || err.contains("Float")
                || err.contains("Int"),
            "Int+Float must fail at typecheck (was runtime trap), got: {}",
            err
        );
    }
    #[test]
    fn rejects_match_arms_int_vs_string() {
        let src = r#"
            pub fn main() {
                let r: Result[Int, String] = Ok(1);
                let x = match r {
                    Ok(v) => v,
                    Err(e) => e,
                };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("incompatible types") || err.contains("match arms"),
            "Ok(Int) vs Err(String) arms must fail, got: {}",
            err
        );
    }
    #[test]
    fn rejects_assert_eq_mismatched_types() {
        let src = r#"
            pub fn main() {
                assert_eq(1, "x");
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("assert_eq")
                && (err.contains("matching") || err.contains("String") || err.contains("Int")),
            "assert_eq(Int, String) must fail, got: {}",
            err
        );
    }
    #[test]
    fn fetch_with_cap_arg_still_typechecks() {
        let src = r#"
            pub fn ok(net: &NetCap) {
                let r = fetch(net, "https://example.invalid");
                assert_eq(r.is_err(), true);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "fetch(net, url) must typecheck: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn ok_constructor_carries_payload_type_into_match() {
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
        assert!(
            check(src).is_ok(),
            "Ok(1) payload Int must type match arm: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn write_file_wrong_arity_fails_closed() {
        let src = r#"
            pub fn bad(fs: &FsCap) {
                let r = write_file(fs, "/tmp/x");
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("expects 3 argument") || err.contains("found 2"),
            "write_file object-cap arity: {}",
            err
        );
    }
    #[test]
    fn path_exists_object_cap_arity_ok() {
        let src = r#"
            pub fn main(fs: &FsCap) {
                let b = path_exists(fs, "/tmp");
                println(b);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "path_exists(cap, path): {:?}",
            check(src).err()
        );
    }
    #[test]
    fn sys_exec_varargs_typechecks() {
        let src = r#"
            pub fn main(sys: &SysCap) {
                let v = sys_exec(sys, "true");
                let w = sys_exec(sys, "echo", "hi");
                println(1);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "sys_exec varargs: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn method_write_file_wrong_arity_fails() {
        let src = r#"
            pub fn bad(fs: &FsCap) {
                let r = fs.write_file("/tmp/x");
                match r { Ok(_) => 0, Err(_) => 1 };
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains(".write_file") && (err.contains("expects 3") || err.contains("found 2")),
            "method write_file arity: {}",
            err
        );
    }
    #[test]
    fn method_write_file_full_arity_ok() {
        let src = r#"
            pub fn ok(fs: &FsCap) {
                let r = fs.write_file("app.log", "hi");
                match r { Ok(_) => 0, Err(_) => 1 };
            }
        "#;
        assert!(
            check(src).is_ok(),
            "method write_file(path, content): {:?}",
            check(src).err()
        );
    }
    #[test]
    fn rejects_missing_return_on_non_void_fn() {
        let src = r#"
            pub fn f() -> Int {
            }
            pub fn main() {
                println(f());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("missing return") || err.contains("Void"),
            "empty body -> Int must fail, got: {}",
            err
        );
    }

    #[test]
    fn rejects_statement_body_without_return_for_int_fn() {
        let src = r#"
            pub fn f(x: Int) -> Int {
                let y = x + 1;
            }
            pub fn main() {
                println(f(1));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("missing return") || err.contains("Void"),
            "no return in Int fn must fail, got: {}",
            err
        );
    }
