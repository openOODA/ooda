    #[test]
    fn method_char_at_const_oob_fails() {
        let src = r#"
            pub fn main() {
                let c = "hi".char_at(99);
                println(c);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("out of bounds") && err.contains("char_at"),
            "{}",
            err
        );
    }
    #[test]
    fn match_if_outer_mut_assign_typechecks() {
        let src = r#"
            pub fn main() -> Int {
                let mut x = 0;
                let r = Ok(5);
                match r {
                    Ok(v) => if true {
                        x = v;
                        v
                    } else {
                        0
                    },
                    Err(e) => 0,
                };
                return x;
            }
        "#;
        assert!(
            check(src).is_ok(),
            "match+if assign to outer let mut: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn alias_let_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn main() {
                let p: Port = 99;
                println(p);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "alias let ann must enforce bounds: {}",
            err
        );
    }
    #[test]
    fn alias_return_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f() -> Port {
                return 99;
            }
            pub fn main() {
                println(f());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "alias return must enforce bounds: {}",
            err
        );
    }
    #[test]
    fn alias_let_refinement_in_bounds_ok() {
        let src = r#"
            type Port = Int[1..10];
            pub fn main() {
                let p: Port = 5;
                println(p);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn nested_return_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f(b: Bool) -> Port {
                if b {
                    return 99;
                }
                return 1;
            }
            pub fn main() { println(f(true)); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "nested if return must enforce bounds: {}",
            err
        );
    }
    #[test]
    fn while_return_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f() -> Port {
                let mut i = 0;
                while i < 1 {
                    return 99;
                }
                return 1;
            }
            pub fn main() { println(f()); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "while return must enforce bounds: {}",
            err
        );
    }
    #[test]
    fn tail_expr_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f() -> Port { 99 }
            pub fn main() { println(f()); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "tail expr return must enforce bounds: {}",
            err
        );
    }
    #[test]
    fn match_arm_const_return_refinement_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f() -> Port {
                match Ok(1) {
                    Ok(v) => 99,
                    Err(e) => 1,
                }
            }
            pub fn main() { println(f()); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "match arm const return: {}",
            err
        );
    }
    #[test]
    fn list_get_const_oob_fails() {
        let src = r#"
            pub fn main() {
                let xs = list_new();
                let ys = list_push(xs, 1);
                let z = list_get(ys, 5);
                println(z);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("out of bounds") || err.contains("list_get"),
            "list_get const OOB: {}",
            err
        );
    }

    #[test]
    fn list_get_negative_const_fails() {
        let src = r#"
            pub fn main() {
                let xs = list_new();
                let ys = list_push(xs, 1);
                let z = list_get(ys, 0 - 1);
                println(z);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(err.contains("negative") || err.contains("out of bounds"), "{}", err);
    }
    #[test]
    fn nested_field_assign_typechecks() {
        let src = r#"
            type Inner = struct { n: Int };
            type Outer = struct { inner: Inner };
            pub fn main() -> Int {
                let mut o = Outer { inner: Inner { n: 1 } };
                o.inner.n = 3;
                return o.inner.n;
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn path_exists_method_requires_fscap_receiver_type() {
        let src = r#"
            pub fn main(net: &NetCap) {
                let b = net.path_exists("/tmp");
                println(b);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("FsCap") || err.contains(".path_exists"),
            "must require FsCap receiver: {}",
            err
        );
    }
    #[test]
    fn field_assign_typechecks_and_rejects_immutable() {
        let ok = r#"
            type Pt = struct { x: Int, y: Int };
            pub fn main() {
                let mut p = Pt { x: 1, y: 2 };
                p.x = 3;
                println(p.x);
            }
        "#;
        assert!(check(ok).is_ok(), "{:?}", check(ok).err());
        let bad = r#"
            type Pt = struct { x: Int, y: Int };
            pub fn main() {
                let p = Pt { x: 1, y: 2 };
                p.x = 3;
            }
        "#;
        let err = check(bad).unwrap_err().to_string();
        assert!(err.contains("immutable") || err.contains("let mut"), "{}", err);
    }
