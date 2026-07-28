use anyhow::{anyhow, Result};

/// Lexical token kinds (location is carried by `SpannedToken`).
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Fn,
    Pub,
    Let,
    Mut,
    Requires,
    Ensures,
    Verify,
    If,
    Else,
    Match,
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
    fn new(token: Token, line: usize, col: usize) -> Self {
        Self { token, line, col }
    }
}

pub struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            line: 1,
            col: 1,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    fn push(&self, tokens: &mut Vec<SpannedToken>, token: Token, line: usize, col: usize) {
        tokens.push(SpannedToken::new(token, line, col));
    }

    pub fn tokenize(&mut self) -> Result<Vec<SpannedToken>> {
        let mut tokens = Vec::new();

        while let Some(&ch) = self.peek() {
            let start_line = self.line;
            let start_col = self.col;
            match ch {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' => {
                    self.advance();
                    if let Some('/') = self.peek() {
                        while let Some(&c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    } else {
                        self.push(&mut tokens, Token::Slash, start_line, start_col);
                    }
                }
                '+' => {
                    self.advance();
                    self.push(&mut tokens, Token::Plus, start_line, start_col);
                }
                '-' => {
                    self.advance();
                    if let Some('>') = self.peek() {
                        self.advance();
                        self.push(&mut tokens, Token::Arrow, start_line, start_col);
                    } else {
                        self.push(&mut tokens, Token::Minus, start_line, start_col);
                    }
                }
                '*' => {
                    self.advance();
                    self.push(&mut tokens, Token::Star, start_line, start_col);
                }
                '=' => {
                    self.advance();
                    if let Some('=') = self.peek() {
                        self.advance();
                        self.push(&mut tokens, Token::EqEq, start_line, start_col);
                    } else if let Some('>') = self.peek() {
                        self.advance();
                        self.push(&mut tokens, Token::FatArrow, start_line, start_col);
                    } else {
                        self.push(&mut tokens, Token::Eq, start_line, start_col);
                    }
                }
                '!' => {
                    self.advance();
                    if let Some('=') = self.peek() {
                        self.advance();
                        self.push(&mut tokens, Token::Neq, start_line, start_col);
                    } else {
                        self.push(&mut tokens, Token::Exclamation, start_line, start_col);
                    }
                }
                '<' => {
                    self.advance();
                    if let Some('=') = self.peek() {
                        self.advance();
                        self.push(&mut tokens, Token::Lte, start_line, start_col);
                    } else {
                        self.push(&mut tokens, Token::Lt, start_line, start_col);
                    }
                }
                '>' => {
                    self.advance();
                    if let Some('=') = self.peek() {
                        self.advance();
                        self.push(&mut tokens, Token::Gte, start_line, start_col);
                    } else {
                        self.push(&mut tokens, Token::Gt, start_line, start_col);
                    }
                }
                '&' => {
                    self.advance();
                    if let Some('&') = self.peek() {
                        self.advance();
                        self.push(&mut tokens, Token::AndAnd, start_line, start_col);
                    } else {
                        self.push(&mut tokens, Token::Ampersand, start_line, start_col);
                    }
                }
                '|' => {
                    self.advance();
                    if let Some('|') = self.peek() {
                        self.advance();
                        self.push(&mut tokens, Token::OrOr, start_line, start_col);
                    } else {
                        self.push(&mut tokens, Token::Pipe, start_line, start_col);
                    }
                }
                '?' => {
                    self.advance();
                    self.push(&mut tokens, Token::Question, start_line, start_col);
                }
                ':' => {
                    self.advance();
                    self.push(&mut tokens, Token::Colon, start_line, start_col);
                }
                ';' => {
                    self.advance();
                    self.push(&mut tokens, Token::Semi, start_line, start_col);
                }
                ',' => {
                    self.advance();
                    self.push(&mut tokens, Token::Comma, start_line, start_col);
                }
                '.' => {
                    self.advance();
                    if let Some('.') = self.peek() {
                        self.advance();
                        if let Some('=') = self.peek() {
                            self.advance();
                            self.push(&mut tokens, Token::DotDotEq, start_line, start_col);
                        } else {
                            self.push(&mut tokens, Token::DotDot, start_line, start_col);
                        }
                    } else {
                        self.push(&mut tokens, Token::Dot, start_line, start_col);
                    }
                }
                '(' => {
                    self.advance();
                    self.push(&mut tokens, Token::LParen, start_line, start_col);
                }
                ')' => {
                    self.advance();
                    self.push(&mut tokens, Token::RParen, start_line, start_col);
                }
                '{' => {
                    self.advance();
                    self.push(&mut tokens, Token::LBrace, start_line, start_col);
                }
                '}' => {
                    self.advance();
                    self.push(&mut tokens, Token::RBrace, start_line, start_col);
                }
                '[' => {
                    self.advance();
                    self.push(&mut tokens, Token::LBracket, start_line, start_col);
                }
                ']' => {
                    self.advance();
                    self.push(&mut tokens, Token::RBracket, start_line, start_col);
                }
                '"' => {
                    self.advance();
                    let mut s = String::new();
                    while let Some(&c) = self.peek() {
                        if c == '"' {
                            self.advance();
                            break;
                        } else if c == '\\' {
                            self.advance();
                            if let Some(&escaped) = self.peek() {
                                match escaped {
                                    '"' => s.push('"'),
                                    'n' => s.push('\n'),
                                    't' => s.push('\t'),
                                    '\\' => s.push('\\'),
                                    other => s.push(other),
                                }
                                self.advance();
                            }
                        } else {
                            s.push(c);
                            self.advance();
                        }
                    }
                    self.push(&mut tokens, Token::StringLit(s), start_line, start_col);
                }
                c if c.is_ascii_digit() => {
                    let mut num_str = String::new();
                    let mut is_float = false;
                    while let Some(&d) = self.peek() {
                        if d.is_ascii_digit() {
                            num_str.push(d);
                            self.advance();
                        } else if d == '.' && !is_float {
                            let mut peek_iter = self.chars.clone();
                            peek_iter.next();
                            if let Some(&next_c) = peek_iter.peek() {
                                if next_c.is_ascii_digit() {
                                    is_float = true;
                                    num_str.push(d);
                                    self.advance();
                                    continue;
                                }
                            }
                            break;
                        } else {
                            break;
                        }
                    }
                    if is_float {
                        self.push(
                            &mut tokens,
                            Token::FloatLit(num_str.parse()?),
                            start_line,
                            start_col,
                        );
                    } else {
                        self.push(
                            &mut tokens,
                            Token::IntLit(num_str.parse()?),
                            start_line,
                            start_col,
                        );
                    }
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let mut ident = String::new();
                    while let Some(&a) = self.peek() {
                        if a.is_ascii_alphanumeric() || a == '_' {
                            ident.push(a);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let kw = match ident.as_str() {
                        "fn" => Token::Fn,
                        "pub" => Token::Pub,
                        "let" => Token::Let,
                        "mut" => Token::Mut,
                        "requires" => Token::Requires,
                        "ensures" => Token::Ensures,
                        "verify" => Token::Verify,
                        "if" => Token::If,
                        "else" => Token::Else,
                        "match" => Token::Match,
                        "return" => Token::Return,
                        "type" => Token::Type,
                        "where" => Token::Where,
                        "true" => Token::True,
                        "false" => Token::False,
                        _ => Token::Ident(ident),
                    };
                    self.push(&mut tokens, kw, start_line, start_col);
                }
                other => {
                    return Err(anyhow!(
                        "Unexpected character '{}' at {}:{}",
                        other,
                        self.line,
                        self.col
                    ));
                }
            }
        }

        tokens.push(SpannedToken::new(Token::Eof, self.line, self.col));
        Ok(tokens)
    }
}
