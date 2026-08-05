impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self {
            tokens,
            pos: 0,
            last_span: Span::synthetic(),
        }
    }


    fn loc(&self) -> (usize, usize) {
        self.tokens
            .get(self.pos)
            .map(|t| (t.line, t.col))
            .unwrap_or((1, 1))
    }


    fn peek(&self) -> &Token {
        static EOF: Token = Token::Eof;
        self.tokens
            .get(self.pos)
            .map(|t| &t.token)
            .unwrap_or(&EOF)
    }


    fn advance(&mut self) -> Token {
        let idx = self.pos;
        if let Some(t) = self.tokens.get(idx) {
            self.last_span = Span {
                line: t.line,
                col: t.col,
            };
            self.pos += 1;
            t.token.clone()
        } else {
            Token::Eof
        }
    }


    /// Span of the most recently consumed token. Used to stamp AST nodes
    /// with the location of their leading token.
    fn last_span(&self) -> Span {
        self.last_span
    }


    fn consume(&mut self, expected: Token) -> Result<()> {
        let current = self.peek().clone();
        if std::mem::discriminant(&current) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            let (line, col) = self.loc();
            Err(anyhow!(
                "Expected token {:?} at {}:{}, found {:?}",
                expected, line, col, current
            ))
        }
    }


    pub fn parse_program(&mut self) -> Result<Program> {
        let mut items = Vec::new();
        while self.peek() != &Token::Eof {
            let item = self.parse_item()?;
            items.push(item);
        }
        Ok(Program { items })
    }


    fn parse_item(&mut self) -> Result<Item> {
        let is_pub = if self.peek() == &Token::Pub {
            self.advance();
            true
        } else {
            false
        };

        if self.peek() == &Token::Import {
            if is_pub {
                let (l, c) = self.loc();
                return Err(anyhow!("`pub import` is not supported at {}:{}", l, c));
            }
            self.advance(); // import
            let span = self.last_span();
            let path = match self.advance() {
                Token::StringLit(s) => s,
                // `import std::crypto;` → resolve as crypto.oo under OODA_STD
                Token::Ident(first) => {
                    let mut parts = vec![first];
                    while self.peek() == &Token::Colon {
                        // could be :: 
                        self.advance();
                        if self.peek() != &Token::Colon {
                            let (l, c) = self.loc();
                            return Err(anyhow!("Expected `::` in import path at {}:{}", l, c));
                        }
                        self.advance();
                        match self.advance() {
                            Token::Ident(seg) => parts.push(seg),
                            other => {
                                let (l, c) = self.loc();
                                return Err(anyhow!(
                                    "Expected identifier in import path at {}:{}, found {:?}",
                                    l,
                                    c,
                                    other
                                ));
                            }
                        }
                    }
                    // std::crypto → crypto.oo (std is a search root)
                    if parts.len() >= 2 && parts[0] == "std" {
                        format!("{}.oo", parts[1..].join("/"))
                    } else {
                        format!("{}.oo", parts.join("/"))
                    }
                }
                other => {
                    let (l, c) = self.loc();
                    return Err(anyhow!(
                        "Expected string path or module path after import at {}:{}, found {:?}",
                        l,
                        c,
                        other
                    ));
                }
            };
            self.consume(Token::Semi)?;
            Ok(Item::Import { path, span })
        } else if self.peek() == &Token::Type {
            self.advance();
            let name = match self.advance() {
                Token::Ident(s) => s.clone(),
                other => { let (l,c)=self.loc(); return Err(anyhow!("Expected type name at {}:{}, found {:?}", l, c, other)); },
            };
            self.consume(Token::Eq)?;
            // `type Token = struct { ... };` — attach the alias name onto the struct.
            let target_type = if matches!(self.peek(), Token::Ident(s) if s == "struct") {
                self.advance(); // struct
                self.parse_struct_type(Some(name.clone()))?
            } else {
                self.parse_type()?
            };
            let mut final_type = target_type;
            if self.peek() == &Token::Where {
                // Honest subset: only `type T = Int where lo..hi` / `lo..=hi` with const Ints.
                let base_is_int = matches!(&final_type, Type::Int)
                    || matches!(&final_type, Type::Custom(s) if s == "Int");
                if !base_is_int {
                    let (l, c) = self.loc();
                    return Err(anyhow!(
                        "Parse error at {}:{}: type alias `where` only supported on Int (alias '{}').",
                        l, c, name
                    ));
                }
                self.advance(); // where
                let expr = self.parse_expression()?;
                let bounds = match expr {
                    Expression::Binary {
                        op: BinOp::DotDot | BinOp::DotDotEq,
                        left,
                        right,
                        ..
                    } => match (*left, *right) {
                        (
                            Expression::Literal(Literal::Int(lo), _),
                            Expression::Literal(Literal::Int(hi), _),
                        ) => Some((lo, hi)),
                        _ => None,
                    },
                    _ => None,
                };
                let Some((lo, hi)) = bounds else {
                    let (l, c) = self.loc();
                    return Err(anyhow!(
                        "Parse error at {}:{}: type alias `where` requires const Int range lo..hi or lo..=hi (alias '{}').",
                        l, c, name
                    ));
                };
                if lo > hi {
                    let (l, c) = self.loc();
                    return Err(anyhow!(
                        "Parse error at {}:{}: empty refinement range {}..{} for alias '{}'",
                        l, c, lo, hi, name
                    ));
                }
                final_type = Type::Custom(format!("Int[{}..{}]", lo, hi));
            }
            self.consume(Token::Semi)?;
            Ok(Item::TypeAlias(name, final_type))
        } else if self.peek() == &Token::Fn {
            let func = self.parse_function_decl(is_pub)?;
            Ok(Item::Function(func))
        } else {
            Err(anyhow!("Unexpected top-level token at {}:{:?}: {:?}", self.loc().0, self.loc().1, self.peek()))
        }
    }

}
