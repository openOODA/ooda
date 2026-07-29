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
