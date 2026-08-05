    #[test]
    fn accepts_homogeneous_list_push_and_get() {
        let src = r#"
            pub fn main() {
                let xs = list_new();
                let ys = list_push(xs, 1);
                let zs = list_push(ys, 2);
                let n = list_get(zs, 0);
                println(n);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "homogeneous Int list: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn list_get_element_type_flows_to_use() {
        // After push Int, list_get yields Int — cannot use as String return.
        let src = r#"
            pub fn bad() -> String {
                let xs = list_new();
                let ys = list_push(xs, 1);
                return list_get(ys, 0);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("return type") || err.contains("String") || err.contains("Int"),
            "list_get Int must not soft-accept as String: {}",
            err
        );
    }
    #[test]
    fn rejects_const_char_at_out_of_bounds() {
        let src = r#"
            pub fn main() {
                let c = char_at("hi", 99);
                println(c);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("out of bounds") && err.contains("char_at"),
            "const char_at OOB must fail at typecheck: {}",
            err
        );
    }
    #[test]
    fn accepts_const_char_at_in_bounds() {
        let src = r#"
            pub fn main() {
                let c = char_at("hi", 0);
                println(c);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "in-bounds char_at: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn rejects_const_str_slice_out_of_bounds() {
        let src = r#"
            pub fn main() {
                let s = str_slice("hi", 0, 9);
                println(s);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("out of bounds") && err.contains("str_slice"),
            "const str_slice OOB: {}",
            err
        );
    }
    #[test]
    fn nested_let_does_not_pollute_outer_type_env() {
        // Was: `let x = "hi"` inside if retyped outer Int x → String (scope leak).
        let src = r#"
            pub fn main() {
                let x = 1;
                if true {
                    let x = "hi";
                    println(x);
                }
                let y = x + 1;
                println(y);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "outer x must stay Int after nested shadow: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn nested_while_let_does_not_pollute_outer_type_env() {
        let src = r#"
            pub fn main() {
                let x = 1;
                let mut i = 0;
                while i < 1 {
                    let x = "hi";
                    println(x);
                    i = i + 1;
                }
                let y = x + 1;
                println(y);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "while-body let must not leak: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn rejects_const_arg_out_of_param_refinement_bounds() {
        let src = r#"
            pub fn port(p: Int[1..65535]) -> Int {
                return p;
            }
            pub fn main() {
                println(port(0));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("0"),
            "const arg must enforce param Int[lo..hi]: {}",
            err
        );
    }
    #[test]
    fn accepts_const_arg_in_param_refinement_bounds() {
        let src = r#"
            pub fn port(p: Int[1..65535]) -> Int {
                return p;
            }
            pub fn main() {
                println(port(8080));
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn outer_let_mut_assign_inside_if_still_typechecks() {
        let src = r#"
            pub fn main() {
                let mut x = 1;
                if true {
                    x = 2;
                }
                println(x);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
    #[test]
    fn else_if_sibling_let_does_not_leak_across_branches() {
        // else if desugars to nested tail-if; sibling lets must not pollute.
        let src = r#"
            pub fn main() {
                if false {
                } else if false {
                    let x = 1;
                } else {
                    println(x);
                }
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("undefined variable") && err.contains("'x'"),
            "else-branch must not see else-if let x: {}",
            err
        );
    }

    #[test]
    fn type_alias_int_unifies_for_arith_and_return() {
        let src = r#"
            type Port = Int;
            pub fn bump(p: Port) -> Int {
                return p + 1;
            }
            pub fn main() {
                println(bump(3));
            }
        "#;
        assert!(
            check(src).is_ok(),
            "Port=Int must unify for + and return: {:?}",
            check(src).err()
        );
    }
    #[test]
    fn type_alias_refinement_param_const_oob_fails() {
        let src = r#"
            type Port = Int[1..65535];
            pub fn take(p: Port) -> Int {
                return 1;
            }
            pub fn main() {
                println(take(0));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation"),
            "alias Int[lo..hi] param must enforce const OOB: {}",
            err
        );
    }
    #[test]
    fn method_char_at_on_string_literal_in_bounds() {
        let src = r#"
            pub fn main() {
                let c = "hi".char_at(0);
                println(c);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }
