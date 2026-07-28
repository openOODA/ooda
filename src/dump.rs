// ===================================================================
// Canonical CHS dumps for oodac golden parity (M1–M3).
// Stable text formats — not Debug pretty-print.
// ===================================================================
use crate::ast::*;
use crate::lexer::{SpannedToken, Token};

/// Stable token kind name used by goldens and oodac.
pub fn token_kind_name(tok: &Token) -> String {
    match tok {
        Token::Fn => "KW_FN".into(),
        Token::Pub => "KW_PUB".into(),
        Token::Let => "KW_LET".into(),
        Token::Mut => "KW_MUT".into(),
        Token::Import => "KW_IMPORT".into(),
        Token::Requires => "KW_REQUIRES".into(),
        Token::Ensures => "KW_ENSURES".into(),
        Token::Verify => "KW_VERIFY".into(),
        Token::If => "KW_IF".into(),
        Token::Else => "KW_ELSE".into(),
        Token::Match => "KW_MATCH".into(),
        Token::While => "KW_WHILE".into(),
        Token::Return => "KW_RETURN".into(),
        Token::Type => "KW_TYPE".into(),
        Token::Where => "KW_WHERE".into(),
        Token::True => "KW_TRUE".into(),
        Token::False => "KW_FALSE".into(),
        Token::Ident(_) => "IDENT".into(),
        Token::IntLit(_) => "INT".into(),
        Token::FloatLit(_) => "FLOAT".into(),
        Token::StringLit(_) => "STRING".into(),
        Token::Plus => "PLUS".into(),
        Token::Minus => "MINUS".into(),
        Token::Star => "STAR".into(),
        Token::Slash => "SLASH".into(),
        Token::EqEq => "EQEQ".into(),
        Token::Neq => "NEQ".into(),
        Token::Lt => "LT".into(),
        Token::Lte => "LTE".into(),
        Token::Gt => "GT".into(),
        Token::Gte => "GTE".into(),
        Token::AndAnd => "ANDAND".into(),
        Token::OrOr => "OROR".into(),
        Token::Eq => "EQ".into(),
        Token::Arrow => "ARROW".into(),
        Token::FatArrow => "FATARROW".into(),
        Token::Question => "QUESTION".into(),
        Token::Exclamation => "BANG".into(),
        Token::Colon => "COLON".into(),
        Token::Semi => "SEMI".into(),
        Token::Comma => "COMMA".into(),
        Token::Dot => "DOT".into(),
        Token::DotDot => "DOTDOT".into(),
        Token::DotDotEq => "DOTDOTEQ".into(),
        Token::Ampersand => "AMP".into(),
        Token::Pipe => "PIPE".into(),
        Token::LParen => "LPAREN".into(),
        Token::RParen => "RPAREN".into(),
        Token::LBrace => "LBRACE".into(),
        Token::RBrace => "RBRACE".into(),
        Token::LBracket => "LBRACKET".into(),
        Token::RBracket => "RBRACKET".into(),
        Token::Eof => "EOF".into(),
    }
}

pub fn token_text(tok: &Token) -> String {
    match tok {
        Token::Ident(s) | Token::StringLit(s) => s.clone(),
        Token::IntLit(n) => n.to_string(),
        Token::FloatLit(f) => format!("{}", f),
        Token::Fn => "fn".into(),
        Token::Pub => "pub".into(),
        Token::Let => "let".into(),
        Token::Mut => "mut".into(),
        Token::Import => "import".into(),
        Token::Requires => "requires".into(),
        Token::Ensures => "ensures".into(),
        Token::Verify => "verify".into(),
        Token::If => "if".into(),
        Token::Else => "else".into(),
        Token::Match => "match".into(),
        Token::While => "while".into(),
        Token::Return => "return".into(),
        Token::Type => "type".into(),
        Token::Where => "where".into(),
        Token::True => "true".into(),
        Token::False => "false".into(),
        Token::Plus => "+".into(),
        Token::Minus => "-".into(),
        Token::Star => "*".into(),
        Token::Slash => "/".into(),
        Token::EqEq => "==".into(),
        Token::Neq => "!=".into(),
        Token::Lt => "<".into(),
        Token::Lte => "<=".into(),
        Token::Gt => ">".into(),
        Token::Gte => ">=".into(),
        Token::AndAnd => "&&".into(),
        Token::OrOr => "||".into(),
        Token::Eq => "=".into(),
        Token::Arrow => "->".into(),
        Token::FatArrow => "=>".into(),
        Token::Question => "?".into(),
        Token::Exclamation => "!".into(),
        Token::Colon => ":".into(),
        Token::Semi => ";".into(),
        Token::Comma => ",".into(),
        Token::Dot => ".".into(),
        Token::DotDot => "..".into(),
        Token::DotDotEq => "..=".into(),
        Token::Ampersand => "&".into(),
        Token::Pipe => "|".into(),
        Token::LParen => "(".into(),
        Token::RParen => ")".into(),
        Token::LBrace => "{".into(),
        Token::RBrace => "}".into(),
        Token::LBracket => "[".into(),
        Token::RBracket => "]".into(),
        Token::Eof => "".into(),
    }
}

/// One line per token: KIND\tLINE\tCOL\tTEXT (TEXT has tabs/newlines escaped)
pub fn format_token_dump(tokens: &[SpannedToken]) -> String {
    let mut out = String::new();
    for t in tokens {
        let kind = token_kind_name(&t.token);
        let text = escape_field(&token_text(&t.token));
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            kind, t.line, t.col, text
        ));
    }
    out
}

fn escape_field(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Stable structural AST dump (line-oriented, not JSON) for golden parity.
pub fn format_ast_dump(program: &Program) -> String {
    let mut out = String::new();
    out.push_str("PROGRAM\n");
    for (i, item) in program.items.iter().enumerate() {
        dump_item(&mut out, item, i, 1);
    }
    out
}

fn indent(n: usize) -> String {
    "  ".repeat(n)
}

fn dump_item(out: &mut String, item: &Item, idx: usize, depth: usize) {
    let pad = indent(depth);
    match item {
        Item::Import { path, span } => {
            out.push_str(&format!(
                "{}ITEM[{}] IMPORT path={} @{}:{}\n",
                pad, idx, path, span.line, span.col
            ));
        }
        Item::TypeAlias(name, ty) => {
            out.push_str(&format!(
                "{}ITEM[{}] TYPEALIAS name={} ty={}\n",
                pad,
                idx,
                name,
                format_type(ty)
            ));
        }
        Item::Function(f) => {
            out.push_str(&format!(
                "{}ITEM[{}] FN name={} pub={} ret={} @{}:{}\n",
                pad,
                idx,
                f.name,
                f.is_pub,
                format_type(&f.return_type),
                f.span.line,
                f.span.col
            ));
            for (pi, p) in f.params.iter().enumerate() {
                out.push_str(&format!(
                    "{}  PARAM[{}] {} ty={} ref={}\n",
                    pad,
                    pi,
                    p.name,
                    format_type(&p.param_type),
                    p.is_ref
                ));
            }
            dump_block(out, &f.body, depth + 1);
        }
    }
}

fn format_type(t: &Type) -> String {
    match t {
        Type::Int => "Int".into(),
        Type::Float => "Float".into(),
        Type::String => "String".into(),
        Type::Bool => "Bool".into(),
        Type::Void => "Void".into(),
        Type::Custom(s) => s.clone(),
        Type::Option(i) => format!("Option[{}]", format_type(i)),
        Type::Result(a, b) => format!("Result[{},{}]", format_type(a), format_type(b)),
        Type::List(i) => format!("List[{}]", format_type(i)),
        Type::Struct { name, fields } => {
            let fs: Vec<String> = fields
                .iter()
                .map(|(n, t)| format!("{}:{}", n, format_type(t)))
                .collect();
            match name {
                Some(n) => format!("struct:{}{{{}}}", n, fs.join(",")),
                None => format!("struct{{{}}}", fs.join(",")),
            }
        }
        Type::NetCap => "NetCap".into(),
        Type::FsCap => "FsCap".into(),
        Type::EnvCap => "EnvCap".into(),
        Type::SysCap => "SysCap".into(),
    }
}

fn dump_block(out: &mut String, block: &Block, depth: usize) {
    let pad = indent(depth);
    out.push_str(&format!("{}BLOCK stmts={}\n", pad, block.stmts.len()));
    for (i, s) in block.stmts.iter().enumerate() {
        dump_stmt(out, s, i, depth + 1);
    }
    if let Some(e) = &block.expr {
        out.push_str(&format!("{}TAIL\n", pad));
        dump_expr(out, e, depth + 1);
    }
}

fn dump_stmt(out: &mut String, stmt: &Statement, idx: usize, depth: usize) {
    let pad = indent(depth);
    match stmt {
        Statement::Let {
            name,
            mutable,
            type_annotation,
            init,
            span,
        } => {
            let ann = type_annotation
                .as_ref()
                .map(format_type)
                .unwrap_or_else(|| "_".into());
            out.push_str(&format!(
                "{}STMT[{}] LET mut={} name={} ann={} @{}:{}\n",
                pad, idx, mutable, name, ann, span.line, span.col
            ));
            dump_expr(out, init, depth + 1);
        }
        Statement::Assign { name, value, span } => {
            out.push_str(&format!(
                "{}STMT[{}] ASSIGN name={} @{}:{}\n",
                pad, idx, name, span.line, span.col
            ));
            dump_expr(out, value, depth + 1);
        }
        Statement::Return(Some(e), span) => {
            out.push_str(&format!(
                "{}STMT[{}] RETURN @{}:{}\n",
                pad, idx, span.line, span.col
            ));
            dump_expr(out, e, depth + 1);
        }
        Statement::Return(None, span) => {
            out.push_str(&format!(
                "{}STMT[{}] RETURN_VOID @{}:{}\n",
                pad, idx, span.line, span.col
            ));
        }
        Statement::Expr(e, span) => {
            out.push_str(&format!(
                "{}STMT[{}] EXPR @{}:{}\n",
                pad, idx, span.line, span.col
            ));
            dump_expr(out, e, depth + 1);
        }
        Statement::While { cond, body, span } => {
            out.push_str(&format!(
                "{}STMT[{}] WHILE @{}:{}\n",
                pad, idx, span.line, span.col
            ));
            dump_expr(out, cond, depth + 1);
            dump_block(out, body, depth + 1);
        }
    }
}

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
