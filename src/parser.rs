use crate::ast::*;
use crate::lexer::Token;
use anyhow::{anyhow, Result};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        let idx = self.pos;
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        self.tokens.get(idx).unwrap_or(&Token::Eof)
    }

    fn consume(&mut self, expected: Token) -> Result<()> {
        let current = self.peek();
        if std::mem::discriminant(current) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(anyhow!("Expected token {:?}, found {:?}", expected, current))
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
                other => return Err(anyhow!("Expected type name, found {:?}", other)),
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
            Err(anyhow!("Unexpected top-level token: {:?}", self.peek()))
        }
    }

    fn parse_function_decl(&mut self, is_pub: bool) -> Result<FunctionDecl> {
        self.consume(Token::Fn)?;

        let name = match self.advance() {
            Token::Ident(s) => s.clone(),
            other => return Err(anyhow!("Expected function name, found {:?}", other)),
        };

        self.consume(Token::LParen)?;
        let mut params = Vec::new();
        if self.peek() != &Token::RParen {
            loop {
                let p_name = match self.advance() {
                    Token::Ident(s) => s.clone(),
                    other => return Err(anyhow!("Expected parameter name, found {:?}", other)),
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
            other => Err(anyhow!("Expected type, found {:?}", other)),
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
                    stmts.push(Statement::Expr(expr));
                } else if self.peek() == &Token::RBrace {
                    final_expr = Some(Box::new(expr));
                    break;
                } else {
                    stmts.push(Statement::Expr(expr));
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
            other => return Err(anyhow!("Expected variable name, found {:?}", other)),
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
        Ok(Statement::Return(expr))
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
            if let Expression::Variable(name) = &primary {
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
                    };
                    continue;
                }
            }

            if self.peek() == &Token::Dot {
                self.advance(); // consume .
                let method = match self.advance() {
                    Token::Ident(m) => m.clone(),
                    other => return Err(anyhow!("Expected method name after '.', found {:?}", other)),
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
                        Ok(Expression::Literal(Literal::Int(-n)))
                    }
                    Token::FloatLit(f) => {
                        self.advance();
                        Ok(Expression::Literal(Literal::Float(-f)))
                    }
                    other => Err(anyhow!("Expected number after '-', found {:?}", other)),
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
                })
            }
            Token::IntLit(n) => { self.advance(); Ok(Expression::Literal(Literal::Int(n))) }
            Token::FloatLit(f) => { self.advance(); Ok(Expression::Literal(Literal::Float(f))) }
            Token::StringLit(s) => { self.advance(); Ok(Expression::Literal(Literal::String(s))) }
            Token::True => { self.advance(); Ok(Expression::Literal(Literal::Bool(true))) }
            Token::False => { self.advance(); Ok(Expression::Literal(Literal::Bool(false))) }
            Token::Ident(id) => { self.advance(); Ok(Expression::Variable(id)) }
            Token::LParen => {
                self.advance();
                if self.peek() == &Token::RParen {
                    self.advance();
                    Ok(Expression::Literal(Literal::Void))
                } else {
                    let expr = self.parse_expression()?;
                    self.consume(Token::RParen)?;
                    Ok(expr)
                }
            }
            other => Err(anyhow!("Unexpected expression token {:?}", other)),
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
            other => Err(anyhow!("Expected pattern, found {:?}", other)),
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
