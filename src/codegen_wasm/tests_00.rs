
    fn parse(src: &str) -> Program {
        let mut l = Lexer::new(src);
        let tokens = l.tokenize().expect("lex");
        let mut p = Parser::new(tokens);
        p.parse_program().expect("parse")
    }


    #[test]
    fn emits_valid_wat_for_straight_line_int() {
        let prog = parse(
            r#"
            pub fn add(a: Int, b: Int) -> Int { return a + b; }
            pub fn main() { let x = add(2, 3); }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        assert!(wat.contains("(local $x i64)"), "wat:\n{}", wat);
        assert!(wat.contains("local.set $x"), "wat:\n{}", wat);
        assert!(wat.contains("i64.const 2"), "wat:\n{}", wat);
        assert!(wat.contains("i64.const 3"), "wat:\n{}", wat);
        assert!(wat.contains("call $add"), "wat:\n{}", wat);
    }


    #[test]
    fn accepts_string_literals_with_data_segment() {
        let prog = parse(r#"pub fn main() { let s = "hi"; }"#);
        let res = WasmCodeGen::emit_wat(&prog).unwrap();
        assert!(res.contains("(memory 1)"));
        assert!(res.contains(r#"(data (i32.const 0) "\68\69\00")"#));
    }


    #[test]
    fn interns_duplicate_string_literals_one_data_segment() {
        let prog = parse(
            r#"
pub fn main() {
    let a = "hello";
    let b = "hello";
    if a == b { println(1); } else { println(0); }
}
"#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).unwrap();
        // One data segment for "hello", not two
        let data_count = wat.matches("(data (i32.const").count();
        assert_eq!(data_count, 1, "expected single interned data segment:\n{}", wat);
        assert!(wat.contains("i32.const 0"), "both should load offset 0:\n{}", wat);
        assert!(wat.contains("call $streq"), "string == uses streq host import:\n{}", wat);
    }


    #[test]
    fn println_string_literal_uses_println_str() {
        let prog = parse(r#"pub fn main() { println("hi"); }"#);
        let wat = WasmCodeGen::emit_wat(&prog).unwrap();
        assert!(wat.contains("call $println_str"), "wat:\n{}", wat);
        assert!(wat.contains(r#"(data (i32.const 0) "\68\69\00")"#), "wat:\n{}", wat);
    }


    #[test]
    fn lowers_string_concat_on_bump_heap() {
        let prog = parse(
            r#"
pub fn main() {
    let a = "a";
    let b = "b";
    let c = a + b;
    println(c);
}
"#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit string concat");
        assert!(
            wat.contains("global.get $heap") && wat.contains("global.set $heap"),
            "concat must bump heap:\n{}",
            wat
        );
        assert!(
            wat.contains("i32.store8") && wat.contains("i32.load8_u"),
            "concat must copy bytes:\n{}",
            wat
        );
        // No host concat import — pure WAT (zero host D for this path).
        assert!(
            !wat.contains("str_concat") && !wat.contains("strcat"),
            "must not invent host strcat:\n{}",
            wat
        );
        assert!(wat.contains("call $println_str"), "println result string:\n{}", wat);
    }


    #[test]
    fn while_tail_if_break_is_not_silently_dropped() {
        // Idiomatic OODA: last stmt without `;` becomes body.expr — must still lower.
        let prog = parse(
            r#"
pub fn main() {
    let mut i = 0;
    while i < 10 {
        i = i + 1;
        if i == 3 { break; }
    }
    println(i);
}
"#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit while tail break");
        assert!(
            wat.contains("br $break_") || wat.contains("br $break"),
            "break in while tail if must lower:\n{}",
            wat
        );
        assert!(
            wat.contains("(if (result"),
            "tail if must lower:\n{}",
            wat
        );
    }


    #[test]
    fn refuses_string_sub_no_pointer_math() {
        let prog = parse(
            r#"
pub fn main() {
    let a = "a";
    let b = "b";
    let c = a - b;
    println(c);
}
"#,
        );
        let err = WasmCodeGen::emit_wat(&prog).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("string arithmetic") || msg.contains("pointer"),
            "expected refuse string -, got: {}",
            msg
        );
    }


    #[test]
    fn lowers_if_then_else_to_valid_wat() {
        let prog = parse(
            r#"
            pub fn pick(x: Int) -> Int {
                if x > 0 { return x; } else { return 0 - x; }
            }
            pub fn main() {}
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        // Both branches must produce i64 and the if must be a
        // structured (if (result i64) (then …) (else …)) block.
        assert!(wat.contains("(if (result i64)"), "wat:\n{}", wat);
        assert!(wat.contains("(then"), "wat:\n{}", wat);
        assert!(wat.contains("(else"), "wat:\n{}", wat);
    }


    #[test]
    fn lowers_if_without_else_with_default_zero_branch() {
        let prog = parse(
            r#"
            pub fn sign(x: Int) -> Int {
                if x >= 0 { return x; }
                return 0 - x;
            }
            pub fn main() {}
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        assert!(wat.contains("(if (result i64)"), "wat:\n{}", wat);
        assert!(wat.contains("i64.const 0"), "wat:\n{}", wat);
    }


    #[test]
    fn compiles_match_expressions_now() {
        let prog = parse(
            r#"
            pub fn classify(x: Int) -> Int {
                match x {
                    0 => 0,
                    1 => 1,
                    _ => 2,
                }
            }
            "#,
        );
        let res = WasmCodeGen::emit_wat(&prog);
        assert!(res.is_ok());
    }


    #[test]
    fn rejects_capability_parameters_non_zero() {
        let prog = parse(
            r#"
            pub fn fetch(net: &NetCap, url: String) -> String { return fetch(url); }
            pub fn main() {}
            "#,
        );
        let res = WasmCodeGen::emit_wat(&prog);
        assert!(res.is_err());
        assert!(format!("{}", res.unwrap_err()).contains("capability"));
    }


    #[test]
    fn emits_local_decl_for_let() {
        let prog = parse(
            r#"
            pub fn main() {
                let a = 1;
                let b = 2;
                let c = a + b;
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        assert!(wat.contains("(local $a i64)"));
        assert!(wat.contains("(local $b i64)"));
        assert!(wat.contains("(local $c i64)"));
    }


    #[test]
    fn emits_println_int() {
        let prog = parse(
            r#"
            pub fn main() {
                let x = 42;
                println(x);
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        assert!(wat.contains("call $println"), "wat:\n{}", wat);
        assert!(wat.contains("local.get $x"), "wat:\n{}", wat);
    }

