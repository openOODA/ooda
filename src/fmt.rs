// ===================================================================
// openOODA formatter — source-like syntax (not Debug AST dumps)
// ===================================================================
use crate::ast::*;
use crate::outline::{format_expr, format_type};

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();

    for (i, item) in program.items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match item {
            Item::TypeAlias(name, target_type) => {
                out.push_str(&format!("type {} = {};\n", name, format_type(target_type)));
            }
            Item::Import { path, .. } => {
                out.push_str(&format!("import \"{}\";\n", path));
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
        .map(|p| {
            let ref_str = if p.is_ref { "&" } else { "" };
            format!("{}: {}{}", p.name, ref_str, format_type(&p.param_type))
        })
        .collect::<Vec<_>>()
        .join(", ");

    let ret_str = if func.return_type != Type::Void {
        format!(" -> {}", format_type(&func.return_type))
    } else {
        String::new()
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

    s.push_str(&format_block(&func.body, 0));
    s.push('\n');

    if let Some(verify) = &func.verify_block {
        s.push_str(&format!("\nverify {} ", func.name));
        s.push_str(&format_block(verify, 0));
        s.push('\n');
    }

    s
}

fn format_block(block: &Block, indent: usize) -> String {
    let pad = "    ".repeat(indent);
    let pad1 = "    ".repeat(indent + 1);
    let mut s = String::new();
    s.push_str("{\n");
    for stmt in &block.stmts {
        s.push_str(&pad1);
        s.push_str(&format_stmt(stmt, indent + 1));
        s.push('\n');
    }
    if let Some(expr) = &block.expr {
        s.push_str(&pad1);
        s.push_str(&format_expr(expr));
        s.push('\n');
    }
    s.push_str(&pad);
    s.push('}');
    s
}

fn format_stmt(stmt: &Statement, indent: usize) -> String {
    match stmt {
        Statement::Let {
            name,
            mutable,
            type_annotation,
            init,
            ..
        } => {
            let mut_kw = if *mutable { "mut " } else { "" };
            let ty = type_annotation
                .as_ref()
                .map(|t| format!(": {}", format_type(t)))
                .unwrap_or_default();
            format!(
                "let {}{}{} = {};",
                mut_kw,
                name,
                ty,
                format_expr(init)
            )
        }
        Statement::Assign { name, value, .. } => {
            format!("{} = {};", name, format_expr(value))
        }
        Statement::Return(Some(e), _) => format!("return {};", format_expr(e)),
        Statement::Return(None, _) => "return;".into(),
        Statement::Expr(e, _) => format!("{};", format_expr(e)),
        Statement::While { cond, body, .. } => {
            format!(
                "while {} {}",
                format_expr(cond),
                format_block(body, indent)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(src: &str) -> Program {
        let mut l = Lexer::new(src);
        let toks = l.tokenize().expect("lex");
        let mut p = Parser::new(toks);
        p.parse_program().expect("parse")
    }

    #[test]
    fn fmt_uses_source_like_contracts_not_debug() {
        let src = r#"
pub fn greet(name: String) -> String
    requires name.len() > 0
    ensures result.len() > 0
{
    return "Hello, " + name + "!";
}
"#;
        let prog = parse(src);
        let out = format_program(&prog);
        assert!(out.contains("requires name.len() > 0"), "out={}", out);
        assert!(out.contains("ensures result.len() > 0"), "out={}", out);
        assert!(!out.contains("Binary {"), "no Debug AST: {}", out);
        assert!(!out.contains("Span {"), "no spans: {}", out);
        assert!(out.contains("fn greet(name: String) -> String"), "sig={}", out);
    }

    #[test]
    fn fmt_let_mut_and_assign() {
        let src = r#"
pub fn main() {
    let mut x = 1;
    x = 2;
    println(x);
}
"#;
        let out = format_program(&parse(src));
        assert!(out.contains("let mut x = 1;"), "out={}", out);
        assert!(out.contains("x = 2;"), "out={}", out);
        assert!(!out.contains("Statement::"), "out={}", out);
    }
}
