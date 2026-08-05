impl Parser {

    fn parse_pattern(&mut self) -> Result<Pattern> {
        match self.peek().clone() {
            Token::Ident(id) if id == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::Ident(id) => {
                self.advance();
                if self.peek() == &Token::LParen {
                    self.advance();
                    let arg = match self.advance() {
                        Token::Ident(s) => Some(s.clone()),
                        _ => None,
                    };
                    self.consume(Token::RParen)?;
                    Ok(Pattern::Variant { name: id, arg })
                } else {
                    Ok(Pattern::Variant { name: id, arg: None })
                }
            }
            Token::True => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(true)))
            }
            Token::False => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(false)))
            }
            Token::IntLit(n) => { self.advance(); Ok(Pattern::Literal(Literal::Int(n))) }
            Token::StringLit(s) => { self.advance(); Ok(Pattern::Literal(Literal::String(s))) }
            other => { let (l,c)=self.loc(); Err(anyhow!("Expected pattern at {}:{}, found {:?}", l, c, other)) },
        }
    }

}
