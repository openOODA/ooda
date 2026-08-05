
    #[test]
    fn real_read_write_file_roundtrip() {
        let base_dir = std::path::PathBuf::from("/home/jeryd/openooda/target/tmp");
        let dir = base_dir.join(format!(
            "ooda_m0_fs_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.txt");
        let path_s = path.to_string_lossy().to_string();

        let prog = parse(
            r#"
            pub fn main(fs: &FsCap) {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("main".into());

        let w = interp
            .call_function(
                "write_file",
                vec![
                    Value::Capability("FsCap".into()),
                    Value::String(path_s.clone()),
                    Value::String("hello-m0".into()),
                ],
                &mut HashMap::new(),
            )
            .expect("write");
        assert!(matches!(w, Value::Ok(_)), "write ok: {:?}", w);

        let r = interp
            .call_function(
                "read_file",
                vec![
                    Value::Capability("FsCap".into()),
                    Value::String(path_s),
                ],
                &mut HashMap::new(),
            )
            .expect("read");
        match r {
            Value::Ok(inner) => assert_eq!(*inner, Value::String("hello-m0".into())),
            other => panic!("expected Ok content, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }


    #[test]
    fn read_file_without_fscap_is_denied() {
        let prog = parse(
            r#"
            pub fn rogue() {
                let _ = read_file("/etc/passwd");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("rogue".into());
        let res = interp.call_function(
            "read_file",
            vec![Value::String("/etc/passwd".into())],
            &mut HashMap::new(),
        );
        assert!(res.is_err());
        assert!(format!("{}", res.unwrap_err()).contains("&FsCap"));
    }


    #[test]
    fn fetch_without_netcap_runtime_denies() {
        let prog = parse(
            r#"
            pub fn rogue() {
                let r = fetch("https://example.invalid");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("rogue".into());
        let res = interp.call_function(
            "fetch",
            vec![Value::String("https://example.invalid".into())],
            &mut HashMap::new(),
        );
        assert!(res.is_err(), "expected runtime deny, got {:?}", res);
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("Runtime Security Capability Violation"), "got: {}", msg);
        assert!(msg.contains("&NetCap"), "got: {}", msg);
    }


    #[test]
    fn write_file_with_wrong_kind_handle_runtime_denies() {
        // Live NetCap is not a valid handle for Fs sealed write_file.
        let prog = parse(
            r#"
            pub fn mix(net: &NetCap, fs: &FsCap) {
                let r = write_file(net, "/tmp/x", "y");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("mix".into());
        let res = interp.call_function(
            "write_file",
            vec![
                Value::Capability("NetCap".into()),
                Value::String("/tmp/x".into()),
                Value::String("y".into()),
            ],
            &mut HashMap::new(),
        );
        assert!(res.is_err(), "wrong-kind handle must deny: {:?}", res);
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("object-capability") || msg.contains("live") || msg.contains("FsCap"),
            "got: {}",
            msg
        );
    }


    #[test]
    fn fetch_ambient_only_without_handle_arg_runtime_denies() {
        // Function declares &NetCap but call omits the live handle — object-cap deny.
        let prog = parse(
            r#"
            pub fn ambient(net: &NetCap) {
                let r = fetch("https://example.invalid");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("ambient".into());
        let res = interp.call_function(
            "fetch",
            vec![Value::String("https://example.invalid".into())],
            &mut HashMap::new(),
        );
        assert!(res.is_err(), "ambient-only fetch must deny: {:?}", res);
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("object-capability") || msg.contains("live"),
            "expected object-cap message, got: {}",
            msg
        );
    }


    #[test]
    fn fetch_with_netcap_returns_result_not_fake_ok() {
        // With NetCap granted AND live handle arg, fetch is allowed. A refused
        // loopback URL must yield Err (or Ok if something answers) — never "200 OK".
        let prog = parse(
            r#"
            pub fn ok(net: &NetCap) -> Result[String, String] {
                return fetch(net, "https://127.0.0.1:1/");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("ok".into());
        let res = interp
            .call_function(
                "fetch",
                vec![
                    Value::Capability("NetCap".into()),
                    Value::String("https://127.0.0.1:1/".into()),
                ],
                &mut HashMap::new(),
            )
            .expect("fetch with live NetCap handle must be allowed");
        match res {
            Value::Err(e) => {
                let s = format!("{}", e);
                assert!(!s.is_empty(), "err message must be non-empty");
                assert!(!s.contains("200 OK"), "must not fake success: {}", s);
            }
            Value::Ok(body) => {
                // Unexpected but honest if something listened on :1
                assert!(matches!(*body, Value::String(_)));
            }
            other => panic!("fetch must return Result, got {:?}", other),
        }
    }


    #[test]
    fn where_type_alias_parses_successfully() {
        let src = r#"type Port = Int where 1..=65535; pub fn main() {}"#;
        let tokens = crate::lexer::Lexer::new(src).tokenize().expect("lex");
        let prog = crate::parser::Parser::new(tokens)
            .parse_program()
            .expect("where must parse successfully");
        if let crate::ast::Item::TypeAlias(name, target) = &prog.items[0] {
            assert_eq!(name, "Port");
            assert!(matches!(target, crate::ast::Type::Custom(ref s) if s == "Int[1..65535]"));
        } else {
            panic!("Expected TypeAlias");
        }
    }


    #[test]
    fn where_type_alias_rejects_non_const_range() {
        let src = r#"type Port = Int where x..y; pub fn main() {}"#;
        let tokens = crate::lexer::Lexer::new(src).tokenize().expect("lex");
        let err = crate::parser::Parser::new(tokens)
            .parse_program()
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("where") && (err.contains("const") || err.contains("range")),
            "got: {}",
            err
        );
    }

