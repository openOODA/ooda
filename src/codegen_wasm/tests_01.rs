
    #[test]
    fn emits_wasm_for_float_arithmetic() {
        let prog = parse(
            r#"
            pub fn main() {
                let x = 1.5 + 2.5;
                let y = x * 2.0;
                println(y);
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit");
        // Float locals are f64
        assert!(wat.contains("(local $x f64)"), "wat:\n{}", wat);
        assert!(wat.contains("(local $y f64)"), "wat:\n{}", wat);
        // Float constants and ops
        assert!(wat.contains("f64.const 1.5"), "wat:\n{}", wat);
        assert!(wat.contains("f64.const 2.5"), "wat:\n{}", wat);
        assert!(wat.contains("f64.add"), "wat:\n{}", wat);
        assert!(wat.contains("f64.mul"), "wat:\n{}", wat);
        // println is the i64 host import, so the Float value is truncated
        assert!(wat.contains("i64.trunc_f64_s"), "wat:\n{}", wat);
    }


    #[test]
    fn while_breaks_when_condition_is_false_not_true() {
        // Polarity: br_if $break must fire when cond is *false* (i64.eqz),
        // not when true (old bug inverted loops so bodies never ran).
        let prog = parse(
            r#"
            pub fn main() {
                let mut i = 0;
                while i < 3 {
                    i = i + 1;
                }
                println(i);
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit while");
        assert!(
            wat.contains("i64.eqz"),
            "while must break on false via i64.eqz:\n{}",
            wat
        );
        // Must not use the inverted "ne 0 → break on true" pattern.
        assert!(
            !wat.contains("i64.ne\n        br_if $break_"),
            "inverted while polarity must not appear:\n{}",
            wat
        );
        assert!(
            wat.contains("br_if $break_") || wat.contains("br_if $break"),
            "wat:\n{}",
            wat
        );
        assert!(
            wat.contains("br $continue_") || wat.contains("br $continue"),
            "wat:\n{}",
            wat
        );
        // Comparisons produce i32 in WASM; we extend to i64 Bool model.
        assert!(
            wat.contains("i64.extend_i32_u"),
            "compare result must extend i32→i64:\n{}",
            wat
        );
    }


    #[test]
    fn nested_while_break_uses_unique_labels() {
        // Locals must be declared at function top — declare j outside.
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let mut i = 0;
                let mut j = 0;
                while i < 2 {
                    j = 0;
                    while j < 3 {
                        if j == 1 { break; }
                        j = j + 1;
                    }
                    i = i + 1;
                }
                return i;
            }
            "#,
        );
        let wat = WasmCodeGen::emit_wat(&prog).expect("emit nested while");
        assert!(
            wat.matches("block $break_").count() >= 2,
            "expected unique nested break labels, got:\n{}",
            wat
        );
        assert!(
            wat.contains("br $break_"),
            "inner break must target labeled break block:\n{}",
            wat
        );
    }


    #[test]
    fn compiles_match_expressions_in_wasm() {
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

