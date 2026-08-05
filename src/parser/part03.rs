impl Parser {

    fn parse_block(&mut self) -> Result<Block> {
        self.consume(Token::LBrace)?;
        let mut stmts = Vec::new();
        let mut final_expr = None;

        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            if self.peek() == &Token::Let {
                stmts.push(self.parse_let_stmt()?);
            } else if self.peek() == &Token::Return {
                stmts.push(self.parse_return_stmt()?);
            } else if self.peek() == &Token::While {
                stmts.push(self.parse_while_stmt()?);
            } else if self.peek() == &Token::For {
                let mut for_stmts = self.parse_for_stmts()?;
                stmts.append(&mut for_stmts);
            } else if self.peek() == &Token::Break {
                self.advance();
                let span = self.last_span();
                self.consume(Token::Semi)?;
                stmts.push(Statement::Break(span));
            } else if self.peek() == &Token::Continue {
                self.advance();
                let span = self.last_span();
                self.consume(Token::Semi)?;
                stmts.push(Statement::Continue(span));
            } else {
                let expr = self.parse_expression()?;
                // Assignment: `name = expr;` (Token::Eq, not ==)
                if let Expression::Variable(name, span) = &expr {
                    if self.peek() == &Token::Eq {
                        self.advance();
                        let value = self.parse_expression()?;
                        self.consume(Token::Semi)?;
                        stmts.push(Statement::Assign {
                            name: name.clone(),
                            value,
                            span: *span,
                        });
                        continue;
                    }
                }
                // Field assignment: `obj.field = expr;` (desugared field access is Call .field)
                if let Expression::Call { name, args, span, .. } = &expr {
                    if name.starts_with('.')
                        && args.len() == 1
                        && self.peek() == &Token::Eq
                    {
                        self.advance();
                        let value = self.parse_expression()?;
                        self.consume(Token::Semi)?;
                        stmts.push(Statement::FieldAssign {
                            object: args[0].clone(),
                            field: name[1..].to_string(),
                            value,
                            span: *span,
                        });
                        continue;
                    }
                }
                if self.peek() == &Token::Semi {
                    self.advance();
                    stmts.push(Statement::Expr(expr, self.last_span()));
                } else if self.peek() == &Token::RBrace {
                    final_expr = Some(Box::new(expr));
                    break;
                } else {
                    stmts.push(Statement::Expr(expr, self.last_span()));
                }
            }
        }

        self.consume(Token::RBrace)?;
        Ok(Block { stmts, expr: final_expr })
    }


    fn parse_let_stmt(&mut self) -> Result<Statement> {
        self.consume(Token::Let)?;
        // Span of the `let` keyword (not the trailing `;`) so migrators and
        // diagnostics can locate the binding site.
        let let_span = self.last_span();
        let mutable = if self.peek() == &Token::Mut {
            self.advance();
            true
        } else {
            false
        };

        let name = match self.advance() {
            Token::Ident(s) => s.clone(),
            other => { let (l,c)=self.loc(); return Err(anyhow!("Expected variable name at {}:{}, found {:?}", l, c, other)); },
        };

        let type_annotation = if self.peek() == &Token::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(Token::Eq)?;
        let init = self.parse_expression()?;
        self.consume(Token::Semi)?;

        Ok(Statement::Let {
            name,
            mutable,
            type_annotation,
            init,
            span: let_span,
        })
    }


    fn parse_return_stmt(&mut self) -> Result<Statement> {
        self.consume(Token::Return)?;
        let expr = if self.peek() != &Token::Semi {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.consume(Token::Semi)?;
        Ok(Statement::Return(expr, self.last_span()))
    }


    fn parse_while_stmt(&mut self) -> Result<Statement> {
        self.consume(Token::While)?;
        let span = self.last_span();
        let cond = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Statement::While { cond, body, span })
    }


    /// `for i in lo..hi { body }` or `for item in list { body }` — desugars into primitive statements.
    fn parse_for_stmts(&mut self) -> Result<Vec<Statement>> {
        self.consume(Token::For)?;
        let span = self.last_span();
        let iter_name = match self.advance() {
            Token::Ident(n) => n,
            other => {
                let (l, c) = self.loc();
                return Err(anyhow::anyhow!(
                    "Expected loop variable after `for` at {}:{}, found {:?}",
                    l, c, other
                ));
            }
        };
        self.consume(Token::In)?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;

        match iterable {
            Expression::Binary { op: BinOp::DotDot, left, right, .. } => {
                self.desugar_range_for(iter_name, *left, *right, false, body, span)
            }
            Expression::Binary { op: BinOp::DotDotEq, left, right, .. } => {
                self.desugar_range_for(iter_name, *left, *right, true, body, span)
            }
            expr => self.desugar_list_for(iter_name, expr, body, span),
        }
    }


    fn desugar_range_for(&self, iter_name: String, lo: Expression, hi: Expression, inclusive: bool, body: Block, span: Span) -> Result<Vec<Statement>> {
        let init_stmt = Statement::Let {
            name: iter_name.clone(),
            mutable: true,
            type_annotation: None,
            init: lo,
            span,
        };

        let cmp = if inclusive { BinOp::Lte } else { BinOp::Lt };
        let mut while_stmts = body.stmts;
        if let Some(tail) = body.expr {
            while_stmts.push(Statement::Expr(*tail, span));
        }
        while_stmts.push(Statement::Assign {
            name: iter_name.clone(),
            value: Expression::Binary {
                op: BinOp::Add,
                left: Box::new(Expression::Variable(iter_name.clone(), span)),
                right: Box::new(Expression::Literal(Literal::Int(1), span)),
                span,
            },
            span,
        });

        let while_stmt = Statement::While {
            cond: Expression::Binary {
                op: cmp,
                left: Box::new(Expression::Variable(iter_name.clone(), span)),
                right: Box::new(hi),
                span,
            },
            body: Block {
                stmts: while_stmts,
                expr: None,
            },
            span,
        };
        Ok(vec![init_stmt, while_stmt])
    }

}
