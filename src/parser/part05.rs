impl Parser {

    fn parse_primary_or_call(&mut self) -> Result<Expression> {
        let mut primary = self.parse_primary()?;

        loop {
            if let Expression::Variable(name, _) = &primary {
                let func_name = name.clone();
                if self.peek() == &Token::Exclamation {
                    self.advance(); // consume !
                }

                if self.peek() == &Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != &Token::RParen {
                        loop {
                            args.push(self.parse_expression()?);
                            if self.peek() == &Token::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.consume(Token::RParen)?;

                    let propagate = if self.peek() == &Token::Question {
                        self.advance();
                        true
                    } else {
                        false
                    };

                    primary = Expression::Call {
                        name: func_name,
                        args,
                        propagate_err: propagate,
                        span: self.last_span(),
                    };
                    continue;
                }
            }

            if self.peek() == &Token::Dot {
                self.advance(); // consume .
                let method = match self.advance() {
                    Token::Ident(m) => m.clone(),
                    other => { let (l,c)=self.loc(); return Err(anyhow!("Expected method name after '.' at {}:{}, found {:?}", l, c, other)); },
                };
                let mut method_args = vec![primary];
                if self.peek() == &Token::LParen {
                    self.advance();
                    if self.peek() != &Token::RParen {
                        loop {
                            method_args.push(self.parse_expression()?);
                            if self.peek() == &Token::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.consume(Token::RParen)?;
                }

                let propagate = if self.peek() == &Token::Question {
                    self.advance();
                    true
                } else {
                    false
                };

                primary = Expression::Call {
                    name: format!(".{}", method),
                    args: method_args,
                    propagate_err: propagate,
                    span: self.last_span(),
                };
                continue;
            }

            break;
        }

        Ok(primary)
    }


    fn parse_primary(&mut self) -> Result<Expression> {
        match self.peek().clone() {
            Token::Minus => {
                self.advance();
                match self.peek().clone() {
                    Token::IntLit(n) => {
                        self.advance();
                        Ok(Expression::Literal(Literal::Int(-n), self.last_span()))
                    }
                    Token::FloatLit(f) => {
                        self.advance();
                        Ok(Expression::Literal(Literal::Float(-f), self.last_span()))
                    }
                    other => { let (l,c)=self.loc(); Err(anyhow!("Expected number after '-' at {}:{}, found {:?}", l, c, other)) },
                }
            }
            Token::If => self.parse_if_expr(),
            Token::Exclamation => {
                self.advance();
                let span = self.last_span();
                // Unary ! binds tight: !expr
                let inner = self.parse_primary_or_call()?;
                Ok(Expression::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(inner),
                    span,
                })
            }
            Token::While => {
                self.advance();
                let span = self.last_span();
                let cond = self.parse_expression()?;
                let body = self.parse_block()?;
                Ok(Expression::While {
                    cond: Box::new(cond),
                    body,
                    span,
                })
            }
            Token::Match => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(Token::LBrace)?;
                let mut arms = Vec::new();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                    let pattern = self.parse_pattern()?;
                    self.consume(Token::FatArrow)?;
                    let body = self.parse_expression()?;
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                    arms.push(MatchArm { pattern, body });
                }
                self.consume(Token::RBrace)?;
                Ok(Expression::Match {
                    expr: Box::new(expr),
                    arms,
                    span: self.last_span(),
                })
            }
            Token::IntLit(n) => { let s = self.last_span(); self.advance(); Ok(Expression::Literal(Literal::Int(n), s)) }
            Token::FloatLit(f) => { let s = self.last_span(); self.advance(); Ok(Expression::Literal(Literal::Float(f), s)) }
            Token::StringLit(s) => { let sp = self.last_span(); self.advance(); Ok(Expression::Literal(Literal::String(s), sp)) }
            Token::True => { let s = self.last_span(); self.advance(); Ok(Expression::Literal(Literal::Bool(true), s)) }
            Token::False => { let s = self.last_span(); self.advance(); Ok(Expression::Literal(Literal::Bool(false), s)) }
            Token::Ident(id) => {
                let s = self.last_span();
                self.advance();
                // Bare `None` constructor (no argument list).
                if id == "None" && self.peek() != &Token::LParen {
                    return Ok(Expression::Call {
                        name: "None".into(),
                        args: vec![],
                        propagate_err: false,
                        span: s,
                    });
                }
                // Struct literal: `Token { field: expr, ... }`
                // Must not steal `match scrutinee { arms }` or `if cond {` — require
                // empty `{}` or `field: …` after the brace.
                if self.peek() == &Token::LBrace && self.looks_like_struct_literal() {
                    return self.parse_struct_literal(id, s);
                }
                Ok(Expression::Variable(id, s))
            }
            Token::LParen => {
                self.advance();
                if self.peek() == &Token::RParen {
                    self.advance();
                    Ok(Expression::Literal(Literal::Void, self.last_span()))
                } else {
                    let expr = self.parse_expression()?;
                    self.consume(Token::RParen)?;
                    Ok(expr)
                }
            }
            other => { let (l,c)=self.loc(); Err(anyhow!("Unexpected expression token {:?} at {}:{}", other, l, c)) },
        }
    }


    /// True when `{` after a type/name is a struct literal (`T { f: e }`), not a block.
    fn looks_like_struct_literal(&self) -> bool {
        // self.pos is at LBrace
        let after = self.tokens.get(self.pos + 1).map(|t| &t.token);
        match after {
            Some(Token::RBrace) => true, // empty struct
            Some(Token::Ident(_)) => {
                matches!(
                    self.tokens.get(self.pos + 2).map(|t| &t.token),
                    Some(Token::Colon)
                )
            }
            _ => false,
        }
    }


    fn parse_struct_literal(&mut self, name: String, span: Span) -> Result<Expression> {
        self.consume(Token::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let fname = match self.advance() {
                Token::Ident(s) => s,
                other => {
                    let (l, c) = self.loc();
                    return Err(anyhow!(
                        "Expected field name in struct literal at {}:{}, found {:?}",
                        l,
                        c,
                        other
                    ));
                }
            };
            self.consume(Token::Colon)?;
            let fexpr = self.parse_expression()?;
            fields.push((fname, fexpr));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.consume(Token::RBrace)?;
        Ok(Expression::StructLit { name, fields, span })
    }

}
