
/// Lexical token kinds (location is carried by `SpannedToken`).
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Fn,
    Pub,
    Let,
    Mut,
    Import,
    Requires,
    Ensures,
    Verify,
    If,
    Else,
    Match,
    While,
    For,
    In,
    Break,
    Continue,
    Return,
    Type,
    Where,
    True,
    False,
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    Plus,
    Minus,
    Star,
    Slash,
    EqEq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    AndAnd,
    OrOr,
    Eq,
    Arrow,
    FatArrow,
    Question,
    Exclamation,
    Colon,
    Semi,
    Comma,
    Dot,
    DotDot,
    DotDotEq,
    Ampersand,
    Pipe,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Eof,
}

/// Token with 1-based source location for diagnostics.
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

impl PartialEq for SpannedToken {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token
    }
}

impl PartialEq<Token> for SpannedToken {
    fn eq(&self, other: &Token) -> bool {
        &self.token == other
    }
}

impl PartialEq<SpannedToken> for Token {
    fn eq(&self, other: &SpannedToken) -> bool {
        self == &other.token
    }
}

impl SpannedToken {
    pub(crate) fn new(token: Token, line: usize, col: usize) -> Self {
        Self { token, line, col }
    }
}

