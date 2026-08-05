
    fn parse(src: &str) -> Program {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = crate::parser::Parser::new(tokens);
        parser.parse_program().expect("parse")
    }


    #[test]
    fn nested_let_does_not_pollute_outer_runtime_env() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let x = 1;
                if true {
                    let x = 99;
                }
                return x;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(1), "outer x must remain 1 after nested let shadow");
    }


    #[test]
    fn outer_mut_assign_inside_if_persists() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let mut x = 1;
                if true {
                    x = 2;
                }
                return x;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(2), "assign to outer let mut must persist");
    }


    #[test]
    fn nested_while_let_does_not_pollute_outer() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let x = 1;
                let mut i = 0;
                while i < 1 {
                    let x = 42;
                    i = i + 1;
                }
                return x;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(1), "while-body let must not leak");
    }


    #[test]
    fn match_if_outer_mut_assign_persists() {
        let prog = parse(
            r#"
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
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(5), "outer mut x must be 5 after match arm assign");
    }


    #[test]
    fn method_char_at_runtime() {
        let prog = parse(
            r#"
            pub fn main() -> String {
                return "hi".char_at(1);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::String("i".into()));
    }


    #[test]
    fn runtime_rejects_refinement_param_oob() {
        let prog = parse(
            r#"
            pub fn port(p: Int[1..65535]) -> Int {
                return p;
            }
            pub fn main() -> Int {
                let bad = 0;
                return port(bad);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let res = interp.call_function("main", vec![], &mut HashMap::new());
        assert!(res.is_err(), "non-const OOB refinement arg must fail at runtime");
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("RefinementTypeViolation"),
            "got: {}",
            msg
        );
    }


    #[test]
    fn runtime_rejects_alias_refinement_param_oob() {
        let prog = parse(
            r#"
            type Port = Int[1..65535];
            pub fn take(p: Port) -> Int {
                return p;
            }
            pub fn main() -> Int {
                let bad = 0;
                return take(bad);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let res = interp.call_function("main", vec![], &mut HashMap::new());
        assert!(res.is_err(), "alias Port Int[lo..hi] non-const OOB must fail");
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("RefinementTypeViolation"), "got: {}", msg);
    }


    #[test]
    fn runtime_capability_blocks_syscall_without_cap() {
        // The static checker would also catch this, but the runtime check
        // is the last line of defense: it must fire even if static checks
        // are bypassed.
        let prog = parse(
            r#"
            pub fn rogue() {
                let h = async_spawn_internal("x");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("rogue".into());
        let res = interp.call_function("async_spawn_internal", vec![Value::String("x".into())], &mut HashMap::new());
        assert!(res.is_err());
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("Runtime Security Capability Violation"), "got: {}", msg);
        assert!(msg.contains("&SysCap"), "got: {}", msg);
    }


    #[test]
    fn runtime_capability_allows_with_correct_cap() {
        let prog = parse(
            r#"
            pub fn ok(sys: &SysCap) -> String {
                return async_spawn_internal(sys, "y");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("ok".into());
        let res = interp.call_function(
            "async_spawn_internal",
            vec![
                Value::Capability("SysCap".into()),
                Value::String("y".into()),
            ],
            &mut HashMap::new(),
        );
        assert!(res.is_ok(), "expected ok, got: {:?}", res);
    }


    #[test]
    fn runtime_capability_wrong_kind_still_blocks() {
        let prog = parse(
            r#"
            pub fn wrong(net: &NetCap) {
                let h = async_spawn_internal("z");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("wrong".into());
        let res = interp.call_function(
            "async_spawn_internal",
            vec![Value::String("z".into())],
            &mut HashMap::new(),
        );
        assert!(res.is_err());
        assert!(format!("{}", res.unwrap_err()).contains("&SysCap"));
    }

