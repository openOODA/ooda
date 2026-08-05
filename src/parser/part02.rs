impl Parser {

    fn parse_function_decl(&mut self, is_pub: bool) -> Result<FunctionDecl> {
        self.consume(Token::Fn)?;

        let name = match self.advance() {
            Token::Ident(s) => s.clone(),
            other => { let (l,c)=self.loc(); return Err(anyhow!("Expected function name at {}:{}, found {:?}", l, c, other)); },
        };

        self.consume(Token::LParen)?;
        let mut params = Vec::new();
        if self.peek() != &Token::RParen {
            loop {
                let p_name = match self.advance() {
                    Token::Ident(s) => s.clone(),
                    other => { let (l,c)=self.loc(); return Err(anyhow!("Expected parameter name at {}:{}, found {:?}", l, c, other)); },
                };

                self.consume(Token::Colon)?;

                let is_ref = if self.peek() == &Token::Ampersand {
                    self.advance();
                    true
                } else {
                    false
                };

                let param_type = self.parse_type()?;
                params.push(Parameter {
                    name: p_name,
                    param_type,
                    is_ref,
                });

                if self.peek() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.consume(Token::RParen)?;

        let return_type = if self.peek() == &Token::Arrow {
            self.advance();
            self.parse_type()?
        } else {
            Type::Void
        };

        let mut requires = Vec::new();
        let mut ensures = Vec::new();

        while self.peek() == &Token::Requires || self.peek() == &Token::Ensures {
            if self.peek() == &Token::Requires {
                self.advance();
                requires.push(self.parse_expression()?);
            } else if self.peek() == &Token::Ensures {
                self.advance();
                ensures.push(self.parse_expression()?);
            }
        }

        let body = self.parse_block()?;

        let verify_block = if self.peek() == &Token::Verify {
            self.advance();
            // consume identifier matching function name if present
            if let Token::Ident(_) = self.peek() {
                self.advance();
            }
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(FunctionDecl {
            is_pub,
            name,
            span: self.last_span(),
            params,
            return_type,
            requires,
            ensures,
            body,
            verify_block,
        })
    }


    fn parse_type(&mut self) -> Result<Type> {
        match self.advance() {
            Token::Ident(name) => match name.as_str() {
                "Int" | "i32" | "u64" => {
                    if self.peek() == &Token::LBracket {
                        self.advance();
                        let range_e = self.parse_expression()?;
                        self.consume(Token::RBracket)?;
                        let (min_s, max_s) = match range_e {
                            Expression::Binary {
                                op: BinOp::DotDot,
                                left,
                                right,
                                ..
                            } => {
                                let min_str = match *left {
                                    Expression::Literal(Literal::Int(n), _) => n.to_string(),
                                    _ => "1".to_string(),
                                };
                                let max_str = match *right {
                                    Expression::Literal(Literal::Int(n), _) => n.to_string(),
                                    _ => "65535".to_string(),
                                };
                                (min_str, max_str)
                            }
                            Expression::Literal(Literal::Int(n), _) => (n.to_string(), "65535".to_string()),
                            _ => ("1".to_string(), "65535".to_string()),
                        };
                        Ok(Type::Custom(format!("Int[{}..{}]", min_s, max_s)))
                    } else {
                        Ok(Type::Int)
                    }
                }
                "Float" => Ok(Type::Float),
                "String" => Ok(Type::String),
                "Bool" => Ok(Type::Bool),
                "Void" => Ok(Type::Void),
                "NetCap" => Ok(Type::NetCap),
                "FsCap" => Ok(Type::FsCap),
                "EnvCap" => Ok(Type::EnvCap),
                "SysCap" => Ok(Type::SysCap),
                "Result" => {
                    self.consume(Token::LBracket)?;
                    let ok_t = self.parse_type()?;
                    self.consume(Token::Comma)?;
                    let err_t = self.parse_type()?;
                    self.consume(Token::RBracket)?;
                    Ok(Type::Result(Box::new(ok_t), Box::new(err_t)))
                }
                "Option" => {
                    self.consume(Token::LBracket)?;
                    let opt_t = self.parse_type()?;
                    self.consume(Token::RBracket)?;
                    Ok(Type::Option(Box::new(opt_t)))
                }
                "List" => {
                    self.consume(Token::LBracket)?;
                    let elem = self.parse_type()?;
                    self.consume(Token::RBracket)?;
                    Ok(Type::List(Box::new(elem)))
                }
                "struct" => self.parse_struct_type(None),
                other => Ok(Type::Custom(other.to_string())),
            },
            other => { let (l,c)=self.loc(); Err(anyhow!("Expected type at {}:{}, found {:?}", l, c, other)) },
        }
    }


    /// Parse `struct { field: Type, ... }` (optionally named via type alias).
    fn parse_struct_type(&mut self, name: Option<String>) -> Result<Type> {
        self.consume(Token::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let fname = match self.advance() {
                Token::Ident(s) => s,
                other => {
                    let (l, c) = self.loc();
                    return Err(anyhow!(
                        "Expected field name in struct at {}:{}, found {:?}",
                        l,
                        c,
                        other
                    ));
                }
            };
            self.consume(Token::Colon)?;
            let fty = self.parse_type()?;
            fields.push((fname, fty));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.consume(Token::RBrace)?;
        Ok(Type::Struct { name, fields })
    }

}
