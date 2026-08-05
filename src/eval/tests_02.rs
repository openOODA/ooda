
    #[test]
    fn break_continue_while_runtime() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let mut i = 0;
                let mut s = 0;
                while i < 10 {
                    i = i + 1;
                    if i == 3 {
                        continue;
                    }
                    if i == 5 {
                        break;
                    }
                    s = s + i;
                }
                return s;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("break/continue");
        // i=1,2,4 summed (3 continued, 5 broke) => 7
        assert_eq!(v, Value::Int(7));
    }


    #[test]
    fn for_range_loop_runtime() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let mut s = 0;
                for i in 1..=3 {
                    s = s + i;
                }
                return s;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("for loop");
        assert_eq!(v, Value::Int(6)); // 1+2+3
    }


    #[test]
    fn python_embed_returns_honest_err() {
        let prog = parse(
            r#"
            pub fn main(sys: &SysCap) -> Result[String, String] {
                return python_embed_internal(sys, "torch");
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function(
                "main",
                vec![Value::Capability("SysCap".into())],
                &mut HashMap::new(),
            )
            .expect("call");
        match v {
            Value::Err(e) => {
                let s = format!("{:?}", e);
                assert!(
                    s.contains("not implemented") || s.contains("python_embed"),
                    "got: {}",
                    s
                );
            }
            other => panic!("expected Err, got {:?}", other),
        }
    }


    #[test]
    fn list_push_get_len() {
        let prog = parse(r#"pub fn main() {}"#);
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("main".into());
        let empty = interp
            .call_function("list_new", vec![], &mut HashMap::new())
            .unwrap();
        let one = interp
            .call_function(
                "list_push",
                vec![empty, Value::Int(7)],
                &mut HashMap::new(),
            )
            .unwrap();
        let two = interp
            .call_function(
                "list_push",
                vec![one, Value::Int(9)],
                &mut HashMap::new(),
            )
            .unwrap();
        let len = interp
            .call_function("list_len", vec![two.clone()], &mut HashMap::new())
            .unwrap();
        assert_eq!(len, Value::Int(2));
        let g0 = interp
            .call_function(
                "list_get",
                vec![two.clone(), Value::Int(0)],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(g0, Value::Int(7));
        let g1 = interp
            .call_function(
                "list_get",
                vec![two, Value::Int(1)],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(g1, Value::Int(9));
    }


    #[test]
    fn string_char_walk() {
        let prog = parse(r#"pub fn main() {}"#);
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("main".into());
        let s = Value::String("ab".into());
        let n = interp
            .call_function("chars_len", vec![s.clone()], &mut HashMap::new())
            .unwrap();
        assert_eq!(n, Value::Int(2));
        let c0 = interp
            .call_function(
                "char_at",
                vec![s.clone(), Value::Int(0)],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(c0, Value::String("a".into()));
        let slice = interp
            .call_function(
                "str_slice",
                vec![s, Value::Int(0), Value::Int(1)],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(slice, Value::String("a".into()));
        let dig = interp
            .call_function(
                "char_is_digit",
                vec![Value::String("9".into())],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(dig, Value::Bool(true));
    }


    #[test]
    fn struct_literal_and_field_access() {
        let prog = parse(
            r#"
            type Token = struct {
                kind: Int,
                text: String
            };
            pub fn main() {
                let t = Token { kind: 1, text: "fn" };
                println(t.kind);
                println(t.text);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        assert!(interp.execute_all().is_ok());
    }


    #[test]
    fn argv_injected_into_main() {
        let prog = parse(
            r#"
            pub fn main(args: List[String]) {
                println(list_len(args));
            }
            "#,
        );
        let mut interp = Interpreter::new(prog).with_argv(vec!["a".into(), "b".into()]);
        assert!(interp.execute_all().is_ok());
    }


    #[test]
    fn fuzz_fails_closed_on_unexpected_errors() {
        // Division by zero / postcondition trap must not soft-pass as green fuzz.
        let prog = parse(
            r#"
            pub fn bad(x: Int) -> Int
                requires x >= 0
                ensures result >= 0
            {
                return x - 100;
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let res = interp.fuzz_all();
        // Fuzz may ok if all combos either pass or pre-fail; with ensures result >= 0
        // and body x-100, x=0 yields -100 and postcondition fails → other_err.
        assert!(
            res.is_err(),
            "fuzz must fail closed when postconditions break: {:?}",
            res
        );
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("unexpected error") || msg.contains("Fuzz"),
            "got: {}",
            msg
        );
    }


    #[test]
    fn postcondition_old_state_snapshot_verification() {
        let prog = parse(
            r#"
            pub fn increment(x: Int) -> Int
                ensures result == old(x) + 1
            {
                return x + 1;
            }
            pub fn main() {
                let y = increment(5);
                println(y);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        assert!(interp.execute_all().is_ok());
    }

