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
        Token::For => "KW_FOR".into(),
        Token::In => "KW_IN".into(),
        Token::Break => "KW_BREAK".into(),
        Token::Continue => "KW_CONTINUE".into(),
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
        Token::For => "for".into(),
        Token::In => "in".into(),
        Token::Break => "break".into(),
        Token::Continue => "continue".into(),
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
