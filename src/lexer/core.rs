use super::token::{Token, SpannedToken};

pub struct Lexer<'a> {
    pub(crate) chars: std::iter::Peekable<std::str::Chars<'a>>,
    pub(crate) line: usize,
    pub(crate) col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            line: 1,
            col: 1,
        }
    }

    pub(crate) fn advance(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    pub(crate) fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    pub(crate) fn push(
        &self,
        tokens: &mut Vec<SpannedToken>,
        token: Token,
        line: usize,
        col: usize,
    ) {
        tokens.push(SpannedToken::new(token, line, col));
    }
}
