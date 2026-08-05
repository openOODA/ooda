
/// Format a requires / ensures expression back into source-like syntax.
/// This handles the small subset that contracts actually use:
/// literals, variables, binary comparisons (`a >= 0`, `result * b == a`),
/// method calls (`x.len()`, `y.to_string()`), and logical connectives.
pub fn format_expr(expr: &Expression) -> String {
    match expr {
        Expression::Literal(lit, _) => format_literal(lit),
        Expression::Variable(name, _) => name.clone(),
        Expression::Binary { op, left, right, .. } => {
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Eq => "==",
                BinOp::Neq => "!=",
                BinOp::Lt => "<",
                BinOp::Lte => "<=",
                BinOp::Gt => ">",
                BinOp::Gte => ">=",
                BinOp::And => "&&",
                BinOp::Or => "||",
                BinOp::DotDot => "..",
                BinOp::DotDotEq => "..=",
            };
            format!("{} {} {}", format_expr(left), op_str, format_expr(right))
        }
        Expression::Call { name, args, .. } => {
            if let Some(method) = name.strip_prefix('.') {
                // Method call: receiver is args[0], the rest are explicit args.
                if let Some((recv, rest)) = args.split_first() {
                    let tail: Vec<String> = rest.iter().map(format_expr).collect();
                    let rest_str = if tail.is_empty() {
                        String::new()
                    } else {
                        format!(", {}", tail.join(", "))
                    };
                    format!("{}.{}{}", format_expr(recv), method, parenthesize_args(&rest_str))
                } else {
                    format!(".{}{}", method, parenthesize_args(""))
                }
            } else {
                let args_str = args.iter().map(format_expr).collect::<Vec<_>>().join(", ");
                format!("{}{}", name, parenthesize_args(&args_str))
            }
        }
        Expression::If { .. }
        | Expression::Match { .. }
        | Expression::While { .. }
        | Expression::StructLit { .. } => "<expr>".into(),
        Expression::Unary { op, expr, .. } => {
            let o = match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            };
            format!("{}{}", o, format_expr(expr))
        }
    }
}

fn parenthesize_args(args_str: &str) -> String {
    if args_str.is_empty() {
        "()".to_string()
    } else {
        format!("({})", args_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(src: &str) -> Program {
        let mut l = Lexer::new(src);
        let tokens = l.tokenize().expect("lex");
        let mut p = Parser::new(tokens);
        p.parse_program().expect("parse")
    }

    #[test]
    fn outline_is_shorter_than_source_for_simple_int_program() {
        let src = r#"
            pub fn add(a: Int, b: Int) -> Int
                requires a >= 0
                requires b >= 0
                ensures result >= 0
            {
                return a + b;
            }

            verify add {
                assert_eq!(add(2, 3), 5);
            }

            pub fn main() {
                let x = add(20, 22);
                println(x);
            }
        "#;
        let outline = generate_outline(&parse(src));
        // The new outline emits one clean line per signature and
        // 3 lines for requires/ensures. Total ≈ 12 lines × ~30 chars.
        // Source was 18 lines × ≈30 chars ≈ 540 chars.
        // Without the Debug-repr fix, the outline was ~1500 chars.
        assert!(
            outline.len() < 400,
            "outline was {} chars; expected <400\noutline:\n{}",
            outline.len(),
            outline
        );
        // Spot-check that the source-level syntax shows up.
        assert!(outline.contains("fn add(a: Int, b: Int) -> Int"));
        assert!(outline.contains("requires a >= 0"));
        assert!(!outline.contains("Span {"), "outline must not leak AST spans");
        assert!(!outline.contains("Binary { op:"), "outline must not leak AST Debug repr");
    }

    #[test]
    fn outline_handles_method_calls_and_strings() {
        let src = r#"
            pub fn greet(name: String) -> String
                requires name.len() > 0
                ensures result.len() > 0
            {
                return "Hello, " + name + "!";
            }
        "#;
        let outline = generate_outline(&parse(src));
        assert!(outline.contains("fn greet(name: String) -> String"));
        assert!(outline.contains("requires name.len() > 0"));
        assert!(outline.contains("ensures result.len() > 0"));
    }

    #[test]
    fn outline_formats_capability_params_with_single_ampersand() {
        // Regression: `format_type` used to return `"&NetCap"` for the
        // NetCap type, and `format_param` also prepended `&` based on
        // `is_ref`, producing `&&NetCap` in the outline. Capability
        // types should not carry the leading `&` so that the
        // `is_ref` flag is the single source of truth.
        let src = r#"
            pub fn fetch_user_profile(net: &NetCap, user_id: Int) -> Result[String, String] {
                return Ok("");
            }
            pub fn log_event(fs: &FsCap, message: String) -> Result[Void, String] {
                return Ok(());
            }
            pub fn run_shell(sys: &SysCap, cmd: String) -> Result[Int, String] {
                return Ok(0);
            }
            pub fn read_env(env: &EnvCap, key: String) -> String {
                return "";
            }
        "#;
        let outline = generate_outline(&parse(src));
        assert!(
            !outline.contains("&&"),
            "outline contains a double ampersand:\n{}",
            outline
        );
        assert!(outline.contains("net: &NetCap"));
        assert!(outline.contains("fs: &FsCap"));
        assert!(outline.contains("sys: &SysCap"));
        assert!(outline.contains("env: &EnvCap"));
    }

    #[test]
    fn outline_handles_type_aliases() {
        let src = "type Port = Int;";
        let outline = generate_outline(&parse(src));
        assert_eq!(outline.trim(), "type Port = Int");
    }

    #[test]
    fn outline_json_is_structured_not_debug_ast() {
        let src = r#"
            pub fn greet(name: String) -> String
                requires name.len() > 0
            {
                return "hi";
            }
            type Port = Int;
        "#;
        let js = generate_outline_json(&parse(src), "t.oo").expect("json");
        let v: serde_json::Value = serde_json::from_str(&js).expect(&js);
        assert_eq!(v["file"], "t.oo");
        assert_eq!(v["functions"][0]["name"], "greet");
        assert_eq!(v["functions"][0]["return_type"], "String");
        assert!(
            v["functions"][0]["requires"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false),
            "requires: {}",
            js
        );
        assert_eq!(v["types"][0]["name"], "Port");
        assert!(!js.contains("Binary {"), "no Debug AST: {}", js);
        assert!(!js.contains("Span {"), "no spans: {}", js);
    }
}