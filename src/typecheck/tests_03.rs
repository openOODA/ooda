    #[test]
    fn rejects_int_eq_float() {
        let src = r#"
            pub fn main() {
                let b = 1 == 1.0;
                println(b);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("cannot compare") || err.contains("equality"),
            "Int == Float must fail, got: {}",
            err
        );
    }
    #[test]
    fn rejects_if_expr_without_else_non_void() {
        let src = r#"
            pub fn main() {
                let x = if true { 1 };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("else") || err.contains("if expression"),
            "if-as-value without else must fail, got: {}",
            err
        );
    }
    #[test]
    fn accepts_if_stmt_without_else() {
        let src = r#"
            pub fn main() {
                if true {
                    println(1);
                }
            }
        "#;
        assert!(
            check(src).is_ok(),
            "statement if without else is fine: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn accepts_if_expr_with_else() {
        let src = r#"
            pub fn main() {
                let x = if true { 1 } else { 0 };
                println(x);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "if/else value: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn accepts_if_else_both_return() {
        let src = r#"
            pub fn f(x: Int) -> Int {
                if x > 0 {
                    return x;
                } else {
                    return 0;
                }
            }
            pub fn main() {
                println(f(1));
            }
        "#;
        assert!(
            check(src).is_ok(),
            "if/else both return must typecheck: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn rejects_partial_if_return_fallthrough() {
        let src = r#"
            pub fn f(x: Int) -> Int {
                if x > 0 {
                    return x;
                }
            }
            pub fn main() {
                println(f(0));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("missing return") || err.contains("Void"),
            "partial if return must fail: {}",
            err
        );
    }
    #[test]
    fn rejects_bind_void_from_while() {
        let src = r#"
            pub fn main() {
                let x = while false {
                    println(1);
                };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("Void") || err.contains("bind"),
            "let x = while must fail: {}",
            err
        );
    }
    #[test]
    fn rejects_const_int_division_by_zero() {
        let src = r#"
            pub fn main() {
                let x = 1 / 0;
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("division by zero"),
            "const /0 must fail closed: {}",
            err
        );
    }
    #[test]
    fn rejects_const_float_division_by_zero() {
        let src = r#"
            pub fn main() {
                let x = 1.0 / 0.0;
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("division by zero"),
            "const float /0 must fail closed: {}",
            err
        );
    }
    #[test]
    fn accepts_early_return_with_dead_code_after() {
        // Early return means the function always returns; trailing stmts are
        // unreachable (separate diagnostic) — first ensure always-returns works.
        let src = r#"
            pub fn f() -> Int {
                return 1;
            }
            pub fn main() {
                println(f());
            }
        "#;
        assert!(
            check(src).is_ok(),
            "plain early return: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn rejects_unreachable_after_return() {
        let src = r#"
            pub fn f() -> Int {
                return 1;
                let y = 2;
            }
            pub fn main() {
                println(f());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("unreachable"),
            "dead code after return must fail: {}",
            err
        );
    }
    #[test]
    fn rejects_unreachable_after_if_else_return() {
        let src = r#"
            pub fn f(x: Int) -> Int {
                if x > 0 {
                    return x;
                } else {
                    return 0;
                }
                let z = 1;
            }
            pub fn main() {
                println(f(1));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("unreachable"),
            "dead code after if/else return: {}",
            err
        );
    }

    #[test]
    fn rejects_list_push_element_type_mismatch() {
        let src = r#"
            pub fn main() {
                let xs = list_new();
                let ys = list_push(xs, 1);
                let zs = list_push(ys, "a");
                println(list_len(zs));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("list element type mismatch")
                || (err.contains("List[Int]") && err.contains("String")),
            "heterogeneous list push must fail: {}",
            err
        );
    }
    #[test]
    fn list_push_assign_refines_element_type_for_for_loop() {
        // Unannotated list_new + push must refine List[_] → List[Int] so
        // `for x in xs { s = s + x }` typechecks (list-for desugar uses list_get).
        let src = r#"
            pub fn main() -> Int {
                let mut xs = list_new();
                xs = list_push(xs, 10);
                xs = list_push(xs, 20);
                let mut s = 0;
                for x in xs {
                    s = s + x;
                }
                return s;
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
