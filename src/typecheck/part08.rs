impl TypeChecker {


    fn infer_expr(&self, expr: &Expression, env: &HashMap<String, Ty>) -> Result<Ty> {
        let empty_mut = HashMap::new();
        self.infer_expr_m(expr, env, &empty_mut)
    }


    fn infer_expr_m(
        &self,
        expr: &Expression,
        env: &HashMap<String, Ty>,
        mutable: &HashMap<String, bool>,
    ) -> Result<Ty> {
        match expr {
            Expression::Literal(Literal::Int(_), _) => Ok(Ty::Int),
            Expression::Literal(Literal::Float(_), _) => Ok(Ty::Float),
            Expression::Literal(Literal::String(_), _) => Ok(Ty::String),
            Expression::Literal(Literal::Bool(_), _) => Ok(Ty::Bool),
            Expression::Literal(Literal::Void, _) => Ok(Ty::Void),
            Expression::Variable(name, _) => env
                .get(name)
                .cloned()
                .or_else(|| {
                    // Allow unbound in incomplete programs only for method receivers we can't type yet
                    None
                })
                .ok_or_else(|| anyhow!("Type error at {}:{}: undefined variable '{}'", expr.span().line, expr.span().col, name)),
            Expression::Binary { op, left, right, .. } => self.infer_binary_expr(expr, env, mutable),
            Expression::Call {
                name,
                args,
                span,
                propagate_err,
                ..
            } => self.infer_call(name, args, span, propagate_err, env, expr),
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => self.infer_if_expr(expr, env, mutable),
            Expression::Unary { op, expr, span } => {
                let t = self.infer_expr(expr, env)?;
                match op {
                    UnaryOp::Not => {
                        if Ty::unifyable(&t, &Ty::Bool) || matches!(t, Ty::Unknown) {
                            Ok(Ty::Bool)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: unary '!' requires Bool, found {}",
                                span.line,
                                span.col,
                                t.display()
                            ))
                        }
                    }
                    UnaryOp::Neg => {
                        if t.is_numeric() || matches!(t, Ty::Unknown) {
                            Ok(t)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: unary '-' requires numeric, found {}",
                                span.line,
                                span.col,
                                t.display()
                            ))
                        }
                    }
                }
            }
            Expression::While { cond, body, span } => {
                let ct = self.infer_expr(cond, env)?;
                if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                    return Err(anyhow!(
                        "Type error at {}:{}: while condition must be Bool, found {}",
                        span.line,
                        span.col,
                        ct.display()
                    ));
                }
                let mut m = HashMap::new();
                let empty_ref = HashMap::new();
                self.check_block(
                    body,
                    &mut env.clone(),
                    &mut m,
                    "while-expr",
                    None,
                    &empty_ref,
                    None,
                )?;
                Ok(Ty::Void)
            }
            Expression::StructLit { name, fields, span } => self.infer_struct_lit_expr(expr, env, mutable),
            Expression::Match { .. } => self.infer_match_expr(expr, env, mutable),
        }
    }

    fn infer_binary_expr(
        &self,
        expr: &Expression,
        env: &HashMap<String, Ty>,
        mutable: &HashMap<String, bool>,
    ) -> Result<Ty> {
        match expr {
            Expression::Binary { op, left, right, .. } => {
                let lt = self.infer_expr(left, env)?;
                let rt = self.infer_expr(right, env)?;
                // Normalize type aliases (`type Port = Int`) before numeric/string shape checks.
                let ln = self.norm(&lt);
                let rn = self.norm(&rt);
                match op {
                    BinOp::Add => {
                        if matches!(ln, Ty::String) || matches!(rn, Ty::String) {
                            if matches!(ln, Ty::String) && matches!(rn, Ty::String) {
                                return Ok(Ty::String);
                            }
                            if matches!(ln, Ty::String) && matches!(rn, Ty::Int | Ty::Float)
                                || matches!(rn, Ty::String) && matches!(ln, Ty::Int | Ty::Float)
                            {
                                return Err(anyhow!(
                                    "Type error at {}:{}: cannot concatenate {} and {} with '+'; convert with .to_string() first",
                                    expr.span().line,
                                    expr.span().col,
                                    lt.display(),
                                    rt.display()
                                ));
                            }
                            return Err(anyhow!(
                                "Type error at {}:{}: cannot apply '+' to {} and {}",
                                expr.span().line,
                                expr.span().col,
                                lt.display(),
                                rt.display()
                            ));
                        }
                        // Same-type numeric only — reject Int+Float (was typecheck-green, runtime trap).
                        if matches!((&ln, &rn), (Ty::Int, Ty::Int)) {
                            return Ok(Ty::Int);
                        }
                        if matches!((&ln, &rn), (Ty::Float, Ty::Float)) {
                            return Ok(Ty::Float);
                        }
                        return Err(anyhow!(
                            "Type error at {}:{}: arithmetic '+' requires matching numeric types (both Int or both Float) or String operands, found {} and {}",
                            expr.span().line,
                            expr.span().col,
                            lt.display(),
                            rt.display()
                        ));
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        // Same-type numeric only (Int+Float used to typecheck then trap at runtime).
                        if matches!(op, BinOp::Div) {
                            if let (Some(_), Some(0)) = (Ty::const_int(left), Ty::const_int(right)) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: integer division by zero",
                                    expr.span().line,
                                    expr.span().col
                                ));
                            }
                            if let (
                                Expression::Literal(Literal::Float(_), _),
                                Expression::Literal(Literal::Float(r), _),
                            ) = (left.as_ref(), right.as_ref())
                            {
                                if *r == 0.0 {
                                    return Err(anyhow!(
                                        "Type error at {}:{}: float division by zero",
                                        expr.span().line,
                                        expr.span().col
                                    ));
                                }
                            }
                        }
                        if matches!((&ln, &rn), (Ty::Int, Ty::Int)) {
                            Ok(Ty::Int)
                        } else if matches!((&ln, &rn), (Ty::Float, Ty::Float)) {
                            Ok(Ty::Float)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: arithmetic operator requires matching numeric types (both Int or both Float), found {} and {}",
                                expr.span().line,
                                expr.span().col,
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::Eq | BinOp::Neq => {
                        // Fail-closed: matching types only (no Int == Float soft-Bool).
                        // Aliases normalize so `Port == Int` works when Port = Int.
                        if self.unify(&lt, &rt)
                            || matches!((&ln, &rn), (Ty::Int, Ty::Int) | (Ty::Float, Ty::Float))
                        {
                            Ok(Ty::Bool)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: cannot compare {} and {} with equality",
                                expr.span().line,
                                expr.span().col,
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte => {
                        if matches!((&ln, &rn), (Ty::Int, Ty::Int) | (Ty::Float, Ty::Float)) {
                            Ok(Ty::Bool)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: comparison requires matching numeric types (both Int or both Float), found {} and {}",
                                expr.span().line,
                                expr.span().col,
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::And | BinOp::Or => {
                        if matches!(ln, Ty::Bool) && matches!(rn, Ty::Bool) {
                            Ok(Ty::Bool)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: logical operator requires Bool operands, found {} and {}",
                                expr.span().line,
                                expr.span().col,
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::DotDot | BinOp::DotDotEq => Ok(Ty::Int), // range sugar; not a full range type yet
                }
            }
            _ => unreachable!("infer_binary_expr"),
        }
    }

}
