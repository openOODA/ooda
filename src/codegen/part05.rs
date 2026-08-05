
#[cfg(test)]
mod tests {
    use super::*;

    fn emit(src: &str) -> Result<String> {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize()?;
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse_program()?;
        LlvmCodeGen::emit_llvm_ir(&program)
    }

    #[test]
    fn emits_valid_int_main() {
        let ir = emit(
            r#"
            pub fn add(a: Int, b: Int) -> Int {
                return a + b;
            }
            pub fn main() {
                let x = add(2, 3);
                println(x);
            }
        "#,
        )
        .expect("emit");
        assert!(ir.contains("define i64 @add(i64 %arg_a, i64 %arg_b)"));
        assert!(ir.contains("define i32 @main()"));
        assert!(ir.contains("add i64"));
        assert!(!ir.contains("load i64, i64* %var_name")); // no string-as-int bug
        LlvmCodeGen::validate_ir(&ir).expect("validate");
    }

    #[test]
    fn rejects_string_program() {
        let err = emit(
            r#"
            pub fn main() {
                let s = "hello";
                println(s);
            }
        "#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("integer-subset") || format!("{}", err).contains("String"));
    }

    #[test]
    fn no_duplicate_ret() {
        let ir = emit(
            r#"
            pub fn main() {
                return 0;
            }
        "#,
        )
        .unwrap();
        let main_body = ir.split("define i32 @main()").nth(1).unwrap();
        let ret_count = main_body.matches("ret ").count();
        assert_eq!(ret_count, 1);
    }

    #[test]
    fn refuses_char_at_string_surface() {
        let err = emit(
            r#"
            pub fn main() {
                let c = char_at("hi", 0);
                println(c);
            }
        "#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("char_at") || err.contains("string") || err.contains("String"),
            "LLVM must refuse string char_at: {}",
            err
        );
    }

    #[test]
    fn refuses_char_at_method() {
        let err = emit(
            r#"
            pub fn main() {
                let c = "hi".char_at(0);
                println(c);
            }
        "#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("method") || err.contains("char_at") || err.contains("String") || err.contains("string"),
            "LLVM must refuse .char_at: {}",
            err
        );
    }

    #[test]
    fn while_tail_if_break_lowers_to_br_end() {
        // Idiomatic last-if-without-`;` must not be silently dropped (dual-engine honesty).
        let ir = emit(
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
        )
        .expect("emit break");
        assert!(
            ir.contains("br label %while_end_"),
            "break must branch to while_end:\n{}",
            ir
        );
        assert!(
            ir.contains("then_") && ir.contains("else_"),
            "tail if must lower:\n{}",
            ir
        );
        LlvmCodeGen::validate_ir(&ir).expect("validate");
    }

    #[test]
    fn if_println_side_effect_not_silently_dropped() {
        let ir = emit(
            r#"
            pub fn main() {
                let i = 2;
                if i == 2 {
                    println(i);
                } else {
                    println(0);
                }
            }
        "#,
        )
        .expect("emit if println");
        let printf_count = ir.matches("@printf").count();
        // declare + two call sites (then and else)
        assert!(
            printf_count >= 3,
            "both branches must call printf; got {} @printf:\n{}",
            printf_count,
            ir
        );
        LlvmCodeGen::validate_ir(&ir).expect("validate");
    }
}

