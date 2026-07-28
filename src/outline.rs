// ===================================================================
// openOODA Module Outline Generator
//
// Produces a token-minimized summary of a `.oo` program suitable for
// AI agent context. Emits clean source-like syntax — NOT the AST's
// `Debug` repr — so requires / ensures clauses are short and
// readable. The previous version printed
//   `Binary { op: Gte, left: Variable("a", Span { ... }), right: ... }`
// which was *longer than the source file itself*, defeating the
// 85–90% token-reduction promise.
//
// Format:
//   pub fn greet(name: String) -> String
//       requires name.len() > 0
//       ensures result.len() > 0
//   type Port = Int where 1..=65535
// ===================================================================
use crate::ast::*;

pub fn generate_outline(program: &Program) -> String {
    let mut out = String::new();

    for (i, item) in program.items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match item {
            Item::TypeAlias(name, target_type) => {
                out.push_str(&format!("type {} = {}\n", name, format_type(target_type)));
            }
            Item::Function(func) => {
                out.push_str(&format_function(func));
            }
        }
    }

    out
}

fn format_function(func: &FunctionDecl) -> String {
    let mut s = String::new();

    let pub_str = if func.is_pub { "pub " } else { "" };
    let params_str = func
        .params
        .iter()
        .map(format_param)
        .collect::<Vec<_>>()
        .join(", ");
    let ret_str = match &func.return_type {
        Type::Void => String::new(),
        t => format!(" -> {}", format_type(t)),
    };

    s.push_str(&format!(
        "{}fn {}({}){}\n",
        pub_str, func.name, params_str, ret_str
    ));

    for req in &func.requires {
        s.push_str(&format!("    requires {}\n", format_expr(req)));
    }
    for ens in &func.ensures {
        s.push_str(&format!("    ensures {}\n", format_expr(ens)));
    }

    s
}

fn format_param(p: &Parameter) -> String {
    let ref_str = if p.is_ref { "&" } else { "" };
    format!("{}: {}{}", p.name, ref_str, format_type(&p.param_type))
}

fn format_type(t: &Type) -> String {
    match t {
        Type::Int => "Int".into(),
        Type::Float => "Float".into(),
        Type::String => "String".into(),
        Type::Bool => "Bool".into(),
        Type::Void => "Void".into(),
        Type::Custom(s) => s.clone(),
        Type::NetCap => "&NetCap".into(),
        Type::FsCap => "&FsCap".into(),
        Type::EnvCap => "&EnvCap".into(),
        Type::SysCap => "&SysCap".into(),
        Type::Option(inner) => format!("Option[{}]", format_type(inner)),
        Type::Result(ok, err) => {
            format!("Result[{}, {}]", format_type(ok), format_type(err))
        }
    }
}

fn format_literal(lit: &Literal) -> String {
    match lit {
        Literal::Int(n) => n.to_string(),
        Literal::Float(n) => n.to_string(),
        Literal::Bool(b) => b.to_string(),
        Literal::String(s) => format!("\"{}\"", s),
        Literal::Void => "()".into(),
    }
}

/// Format a requires / ensures expression back into source-like syntax.
/// This handles the small subset that contracts actually use:
/// literals, variables, binary comparisons (`a >= 0`, `result * b == a`),
/// method calls (`x.len()`, `y.to_string()`), and logical connectives.
fn format_expr(expr: &Expression) -> String {
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
        Expression::If { .. } | Expression::Match { .. } => "<expr>".into(),
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
    fn outline_handles_type_aliases() {
        let src = "type Port = Int;";
        let outline = generate_outline(&parse(src));
        assert_eq!(outline.trim(), "type Port = Int");
    }
}