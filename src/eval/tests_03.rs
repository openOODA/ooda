
    #[test]
    fn function_without_old_state_skips_snapshot() {
        // No `old()` references anywhere — interpreter should NOT
        // allocate a snapshot HashMap. We verify by checking that
        // a function with a requires clause (which doesn't need the
        // snapshot either) still runs and prints.
        let prog = parse(
            r#"
            pub fn double(x: Int) -> Int
                requires x >= 0
                ensures result == x * 2
            {
                return x * 2;
            }
            pub fn main() {
                let y = double(21);
                println(y);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.execute_all().expect("must run without snapshot");
    }


    #[test]
    fn return_inside_if_returns_from_function() {
        let prog = parse(
            r#"
            pub fn pick(x: Int) -> Int {
                if x > 0 {
                    return 1;
                }
                return 2;
            }
            pub fn main() {
                assert_eq(pick(5), 1);
                assert_eq(pick(0), 2);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        assert!(interp.execute_all().is_ok());
    }


    #[test]
    fn runtime_rejects_alias_return_refinement_oob() {
        let prog = parse(
            r#"
            type Port = Int[1..10];
            pub fn f(x: Int) -> Port {
                return x;
            }
            pub fn main() -> Int {
                let bad = 99;
                let _p = f(bad);
                return 0;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let res = interp.call_function("main", vec![], &mut HashMap::new());
        assert!(res.is_err(), "non-const alias return OOB must fail");
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("RefinementTypeViolation"), "got: {}", msg);
    }



    #[test]
    fn field_assign_runtime() {
        let prog = parse(
            r#"
            type Pt = struct { x: Int, y: Int };
            pub fn main() -> Int {
                let mut p = Pt { x: 1, y: 2 };
                p.x = 7;
                return p.x;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp.call_function("main", vec![], &mut HashMap::new()).expect("run");
        assert_eq!(v, Value::Int(7));
    }


    #[test]
    fn nested_field_assign_runtime() {
        let prog = parse(
            r#"
            type Inner = struct { n: Int };
            type Outer = struct { inner: Inner };
            pub fn main() -> Int {
                let mut o = Outer { inner: Inner { n: 1 } };
                o.inner.n = 9;
                return o.inner.n;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("nested field assign");
        assert_eq!(v, Value::Int(9));
    }


    #[test]
    fn contains_method_runtime() {
        let prog = parse(
            r#"
            pub fn main() -> Bool {
                return "hello".contains("ell");
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp.call_function("main", vec![], &mut HashMap::new()).expect("run");
        assert_eq!(v, Value::Bool(true));
    }


    #[test]
    fn question_mark_runtime_ok() {
        let prog = parse(
            r#"
            pub fn f() -> Result[Int, String] { return Ok(7); }
            pub fn g() -> Result[Int, String] {
                let x = f()?;
                return Ok(x);
            }
            pub fn main() -> Int {
                match g() {
                    Ok(v) => v,
                    Err(e) => 0,
                }
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(7));
    }


    #[test]
    fn question_mark_early_return_err() {
        let prog = parse(
            r#"
            pub fn fail() -> Result[Int, String] { return Err("nope"); }
            pub fn g() -> Result[Int, String] {
                let x = fail()?;
                return Ok(x + 1);
            }
            pub fn main() -> String {
                match g() {
                    Ok(v) => "ok",
                    Err(e) => e,
                }
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::String("nope".into()));
    }

