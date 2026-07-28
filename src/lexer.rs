use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
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

    // Identifiers & Literals
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),

    // Symbols & Operators
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
    Arrow,     // ->
    FatArrow,  // =>
    Question,  // ?
    Exclamation, // !
    Colon,
    Semi,
    Comma,
    Dot,
    DotDot,
    DotDotEq,
    Ampersand,
    Pipe,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Eof,
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
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

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(&ch) = self.peek() {
            match ch {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' => {
                    self.advance();
                    if let Some('/') = self.peek() {
                        // Single-line comment
                        while let Some(&c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    } else {
                        tokens.push(Token::Slash);
                    }
                }
                '+' => { self.advance(); tokens.push(Token::Plus); }
                '-' => {
                    self.advance();
                    if let Some('>') = self.peek() {
                        self.advance();
                        tokens.push(Token::Arrow);
                    } else {
                        tokens.push(Token::Minus);
                    }
                }
                '*' => { self.advance(); tokens.push(Token::Star); }
                '=' => {
                    self.advance();
                    if let Some('=') = self.peek() {
                        self.advance();
                        tokens.push(Token::EqEq);
                    } else if let Some('>') = self.peek() {
                        self.advance();
                        tokens.push(Token::FatArrow);
                    } else {
                        tokens.push(Token::Eq);
                    }
                }
                '!' => {
                    self.advance();
                    if let Some('=') = self.peek() {
                        self.advance();
                        tokens.push(Token::Neq);
                    } else {
                        tokens.push(Token::Exclamation);
                    }
                }
                '<' => {
                    self.advance();
                    if let Some('=') = self.peek() {
                        self.advance();
                        tokens.push(Token::Lte);
                    } else {
                        tokens.push(Token::Lt);
                    }
                }
                '>' => {
                    self.advance();
                    if let Some('=') = self.peek() {
                        self.advance();
                        tokens.push(Token::Gte);
                    } else {
                        tokens.push(Token::Gt);
                    }
                }
                '&' => {
                    self.advance();
                    if let Some('&') = self.peek() {
                        self.advance();
                        tokens.push(Token::AndAnd);
                    } else {
                        tokens.push(Token::Ampersand);
                    }
                }
                '|' => {
                    self.advance();
                    if let Some('|') = self.peek() {
                        self.advance();
                        tokens.push(Token::OrOr);
                    } else {
                        tokens.push(Token::Pipe);
                    }
                }
                '?' => { self.advance(); tokens.push(Token::Question); }
                ':' => { self.advance(); tokens.push(Token::Colon); }
                ';' => { self.advance(); tokens.push(Token::Semi); }
                ',' => { self.advance(); tokens.push(Token::Comma); }
                '.' => {
                    self.advance();
                    if let Some('.') = self.peek() {
                        self.advance();
                        if let Some('=') = self.peek() {
                            self.advance();
                            tokens.push(Token::DotDotEq);
                        } else {
                            tokens.push(Token::DotDot);
                        }
                    } else {
                        tokens.push(Token::Dot);
                    }
                }
                '(' => { self.advance(); tokens.push(Token::LParen); }
                ')' => { self.advance(); tokens.push(Token::RParen); }
                '{' => { self.advance(); tokens.push(Token::LBrace); }
                '}' => { self.advance(); tokens.push(Token::RBrace); }
                '[' => { self.advance(); tokens.push(Token::LBracket); }
                ']' => { self.advance(); tokens.push(Token::RBracket); }
                '"' => {
                    self.advance();
                    let mut s = String::new();
                    while let Some(&c) = self.peek() {
                        if c == '"' {
                            self.advance();
                            break;
                        }
                        s.push(c);
                        self.advance();
                    }
                    tokens.push(Token::StringLit(s));
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
                            peek_iter.next(); // skip current '.'
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
                        tokens.push(Token::FloatLit(num_str.parse()?));
                    } else {
                        tokens.push(Token::IntLit(num_str.parse()?));
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
                    tokens.push(kw);
                }
                other => return Err(anyhow!("Unexpected character '{}' at line {}", other, self.line)),
            }
        }

        tokens.push(Token::Eof);
        Ok(tokens)
    }
}
