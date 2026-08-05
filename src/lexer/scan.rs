use anyhow::{anyhow, Result};
use super::token::{Token, SpannedToken};
use super::core::Lexer;

impl Lexer<'_> {
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
                    let kw = super::keywords::keyword_or_ident(ident);
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
