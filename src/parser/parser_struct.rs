use crate::ast::*;
use crate::lexer::{SpannedToken, Token};
use anyhow::{anyhow, Result};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    /// Span of the most recently consumed token (used to populate AST spans).
    last_span: Span,
}

