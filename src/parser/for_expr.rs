impl Parser {

    fn desugar_list_for(&mut self, iter_name: String, expr: Expression, body: Block, span: Span) -> Result<Vec<Statement>> {
        // let _list = expr;
        // let mut _i = 0;
        // while _i < .len(_list) {
        //     let item = _list[_i];
        //     body
        //     _i = _i + 1;
        // }
        let list_var = format!("__list_{}_{}", span.line, span.col);
        let idx_var = format!("__idx_{}_{}", span.line, span.col);

        let init_list = Statement::Let {
            name: list_var.clone(),
            mutable: false,
            type_annotation: None,
            init: expr,
            span,
        };

        let init_idx = Statement::Let {
            name: idx_var.clone(),
            mutable: true,
            type_annotation: None,
            init: Expression::Literal(Literal::Int(0), span),
            span,
        };

        let mut while_stmts = vec![];
        
        // let iter_name = _list[_idx];
        let bind_item = Statement::Let {
            name: iter_name,
            mutable: false,
            type_annotation: None,
            init: Expression::Call {
                name: "list_get".into(),
                args: vec![
                    Expression::Variable(list_var.clone(), span),
                    Expression::Variable(idx_var.clone(), span),
                ],
                span,
                propagate_err: false,
            },
            span,
        };
        while_stmts.push(bind_item);

        while_stmts.extend(body.stmts);
        if let Some(tail) = body.expr {
            while_stmts.push(Statement::Expr(*tail, span));
        }

        // _idx = _idx + 1;
        while_stmts.push(Statement::Assign {
            name: idx_var.clone(),
            value: Expression::Binary {
                op: BinOp::Add,
                left: Box::new(Expression::Variable(idx_var.clone(), span)),
                right: Box::new(Expression::Literal(Literal::Int(1), span)),
                span,
            },
            span,
        });

        let cond = Expression::Binary {
            op: BinOp::Lt,
            left: Box::new(Expression::Variable(idx_var.clone(), span)),
            right: Box::new(Expression::Call {
                name: "list_len".into(),
                args: vec![Expression::Variable(list_var, span)],
                span,
                propagate_err: false,
            }),
            span,
        };

        let while_stmt = Statement::While {
            cond,
            body: Block {
                stmts: while_stmts,
                expr: None,
            },
            span,
        };

        Ok(vec![init_list, init_idx, while_stmt])
    }


    /// `if cond { ... } else if ... else { ... }`
    fn parse_if_expr(&mut self) -> Result<Expression> {
        self.consume(Token::If)?;
        let span = self.last_span();
        let cond = self.parse_expression()?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.peek() == &Token::Else {
            self.advance();
            if self.peek() == &Token::If {
                // else if → nested if as a block tail expression
                let nested = self.parse_if_expr()?;
                Some(Block {
                    stmts: vec![],
                    expr: Some(Box::new(nested)),
                })
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(Expression::If {
            cond: Box::new(cond),
            then_branch,
            else_branch,
            span,
        })
    }


    fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_binary_expr(0)
    }


    fn parse_binary_expr(&mut self, min_prec: u8) -> Result<Expression> {
        let mut left = self.parse_primary_or_call()?;

        while let Some(op) = self.peek_binop() {
            let prec = binop_prec(&op);
            if prec < min_prec {
                break;
            }
            self.advance(); // consume op
            let right = self.parse_binary_expr(prec + 1)?;
            left = Expression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span: self.last_span(),
            };
        }

        Ok(left)
    }


    fn peek_binop(&self) -> Option<BinOp> {
        match self.peek() {
            Token::Plus => Some(BinOp::Add),
            Token::Minus => Some(BinOp::Sub),
            Token::Star => Some(BinOp::Mul),
            Token::Slash => Some(BinOp::Div),
            Token::EqEq => Some(BinOp::Eq),
            Token::Neq => Some(BinOp::Neq),
            Token::Lt => Some(BinOp::Lt),
            Token::Lte => Some(BinOp::Lte),
            Token::Gt => Some(BinOp::Gt),
            Token::Gte => Some(BinOp::Gte),
            Token::AndAnd => Some(BinOp::And),
            Token::OrOr => Some(BinOp::Or),
            Token::DotDot => Some(BinOp::DotDot),
            Token::DotDotEq => Some(BinOp::DotDotEq),
            _ => None,
        }
    }

}
