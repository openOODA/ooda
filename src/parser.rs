use crate::ast::*;
use crate::lexer::{SpannedToken, Token};
use anyhow::{anyhow, Result};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    /// Span of the most recently consumed token (used to populate AST spans).
    last_span: Span,
}

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

        if self.peek() == &Token::Type {
            self.advance();
            let name = match self.advance() {
                Token::Ident(s) => s.clone(),
                other => { let (l,c)=self.loc(); return Err(anyhow!("Expected type name at {}:{}, found {:?}", l, c, other)); },
            };
            self.consume(Token::Eq)?;
            let target_type = self.parse_type()?;
            if self.peek() == &Token::Where {
                self.advance();
                let _expr = self.parse_expression()?;
            }
            self.consume(Token::Semi)?;
            Ok(Item::TypeAlias(name, target_type))
        } else if self.peek() == &Token::Fn {
            let func = self.parse_function_decl(is_pub)?;
            Ok(Item::Function(func))
        } else {
            Err(anyhow!("Unexpected top-level token at {}:{:?}: {:?}", self.loc().0, self.loc().1, self.peek()))
        }
    }

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
                "Int" | "i32" | "u64" => Ok(Type::Int),
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
                other => Ok(Type::Custom(other.to_string())),
            },
            other => { let (l,c)=self.loc(); Err(anyhow!("Expected type at {}:{}, found {:?}", l, c, other)) },
        }
    }

    fn parse_block(&mut self) -> Result<Block> {
        self.consume(Token::LBrace)?;
        let mut stmts = Vec::new();
        let mut final_expr = None;

        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            if self.peek() == &Token::Let {
                stmts.push(self.parse_let_stmt()?);
            } else if self.peek() == &Token::Return {
                stmts.push(self.parse_return_stmt()?);
            } else {
                let expr = self.parse_expression()?;
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
            span: self.last_span(),
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
            Token::If => {
                self.advance();
                let cond = self.parse_expression()?;
                let then_branch = self.parse_block()?;
                let else_branch = if self.peek() == &Token::Else {
                    self.advance();
                    Some(self.parse_block()?)
                } else {
                    None
                };
                Ok(Expression::If {
                    cond: Box::new(cond),
                    then_branch,
                    else_branch,
                    span: self.last_span(),
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
            Token::Ident(id) => { let s = self.last_span(); self.advance(); Ok(Expression::Variable(id, s)) }
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
            Token::IntLit(n) => { self.advance(); Ok(Pattern::Literal(Literal::Int(n))) }
            Token::StringLit(s) => { self.advance(); Ok(Pattern::Literal(Literal::String(s))) }
            other => { let (l,c)=self.loc(); Err(anyhow!("Expected pattern at {}:{}, found {:?}", l, c, other)) },
        }
    }
}

fn binop_prec(op: &BinOp) -> u8 {
    match op {
        BinOp::DotDot | BinOp::DotDotEq => 1,
        BinOp::Or => 2,
        BinOp::And => 3,
        BinOp::Eq | BinOp::Neq => 4,
        BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte => 5,
        BinOp::Add | BinOp::Sub => 6,
        BinOp::Mul | BinOp::Div => 7,
    }
}
