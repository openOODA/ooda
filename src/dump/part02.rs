
fn dump_expr(out: &mut String, expr: &Expression, depth: usize) {
    let pad = indent(depth);
    match expr {
        Expression::Literal(lit, span) => {
            out.push_str(&format!(
                "{}EXPR LIT {} @{}:{}\n",
                pad,
                format_lit(lit),
                span.line,
                span.col
            ));
        }
        Expression::Variable(name, span) => {
            out.push_str(&format!(
                "{}EXPR VAR {} @{}:{}\n",
                pad, name, span.line, span.col
            ));
        }
        Expression::Binary {
            op,
            left,
            right,
            span,
        } => {
            out.push_str(&format!(
                "{}EXPR BIN op={:?} @{}:{}\n",
                pad, op, span.line, span.col
            ));
            dump_expr(out, left, depth + 1);
            dump_expr(out, right, depth + 1);
        }
        Expression::Call {
            name,
            args,
            propagate_err,
            span,
        } => {
            out.push_str(&format!(
                "{}EXPR CALL name={} argc={} prop={} @{}:{}\n",
                pad,
                name,
                args.len(),
                propagate_err,
                span.line,
                span.col
            ));
            for a in args {
                dump_expr(out, a, depth + 1);
            }
        }
        Expression::If {
            cond,
            then_branch,
            else_branch,
            span,
        } => {
            out.push_str(&format!(
                "{}EXPR IF @{}:{}\n",
                pad, span.line, span.col
            ));
            dump_expr(out, cond, depth + 1);
            dump_block(out, then_branch, depth + 1);
            if let Some(e) = else_branch {
                out.push_str(&format!("{}ELSE\n", pad));
                dump_block(out, e, depth + 1);
            }
        }
        Expression::Match { expr, arms, span } => {
            out.push_str(&format!(
                "{}EXPR MATCH arms={} @{}:{}\n",
                pad,
                arms.len(),
                span.line,
                span.col
            ));
            dump_expr(out, expr, depth + 1);
            for (i, arm) in arms.iter().enumerate() {
                out.push_str(&format!(
                    "{}ARM[{}] pat={}\n",
                    pad,
                    i,
                    format_pat(&arm.pattern)
                ));
                dump_expr(out, &arm.body, depth + 1);
            }
        }
        Expression::Unary { op, expr, span } => {
            out.push_str(&format!(
                "{}EXPR UNARY op={:?} @{}:{}\n",
                pad, op, span.line, span.col
            ));
            dump_expr(out, expr, depth + 1);
        }
        Expression::While { cond, body, span } => {
            out.push_str(&format!(
                "{}EXPR WHILE @{}:{}\n",
                pad, span.line, span.col
            ));
            dump_expr(out, cond, depth + 1);
            dump_block(out, body, depth + 1);
        }
        Expression::StructLit { name, fields, span } => {
            out.push_str(&format!(
                "{}EXPR STRUCTLIT name={} fields={} @{}:{}\n",
                pad,
                name,
                fields.len(),
                span.line,
                span.col
            ));
            for (n, e) in fields {
                out.push_str(&format!("{}FIELD {}\n", pad, n));
                dump_expr(out, e, depth + 1);
            }
        }
    }
}

fn format_lit(lit: &Literal) -> String {
    match lit {
        Literal::Int(n) => format!("int:{}", n),
        Literal::Float(f) => format!("float:{}", f),
        Literal::String(s) => format!("str:{}", escape_field(s)),
        Literal::Bool(b) => format!("bool:{}", b),
        Literal::Void => "void".into(),
    }
}

fn format_pat(p: &Pattern) -> String {
    match p {
        Pattern::Literal(l) => format!("lit:{}", format_lit(l)),
        Pattern::Variant { name, arg } => match arg {
            Some(a) => format!("var:{}({})", name, a),
            None => format!("var:{}", name),
        },
        Pattern::Wildcard => "_".into(),
    }
}

/// Check status line for parity: OK or ERR\tmessage
pub fn format_check_ok() -> String {
    "OK\n".into()
}

pub fn format_check_err(kind: &str, msg: &str) -> String {
    format!("ERR\t{}\t{}\n", kind, escape_field(msg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[test]
    fn token_dump_is_stable_for_hello() {
        let src = "pub fn main() { println(1); }\n";
        let mut lx = Lexer::new(src);
        let toks = lx.tokenize().unwrap();
        let dump = format_token_dump(&toks);
        assert!(dump.contains("KW_PUB\t"), "{}", dump);
        assert!(dump.contains("KW_FN\t"), "{}", dump);
        assert!(dump.contains("IDENT\t"), "{}", dump);
        assert!(dump.lines().last().unwrap().starts_with("EOF\t"));
    }

    #[test]
    fn ast_dump_includes_fn_main() {
        let src = "pub fn main() { let x = 1; }\n";
        let mut lx = Lexer::new(src);
        let toks = lx.tokenize().unwrap();
        let mut p = Parser::new(toks);
        let prog = p.parse_program().unwrap();
        let dump = format_ast_dump(&prog);
        assert!(dump.contains("FN name=main"), "{}", dump);
        assert!(dump.contains("LET mut=false name=x"), "{}", dump);
    }
}
