use super::token::Token;

pub(crate) fn keyword_or_ident(ident: String) -> Token {
    match ident.as_str() {
        "fn" => Token::Fn,
        "pub" => Token::Pub,
        "let" => Token::Let,
        "mut" => Token::Mut,
        "import" => Token::Import,
        "requires" => Token::Requires,
        "ensures" => Token::Ensures,
        "verify" => Token::Verify,
        "if" => Token::If,
        "else" => Token::Else,
        "match" => Token::Match,
        "while" => Token::While,
        "for" => Token::For,
        "in" => Token::In,
        "break" => Token::Break,
        "continue" => Token::Continue,
        "return" => Token::Return,
        "type" => Token::Type,
        "where" => Token::Where,
        "true" => Token::True,
        "false" => Token::False,
        _ => Token::Ident(ident),
    }
}
