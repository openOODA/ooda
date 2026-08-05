impl TypeChecker {

    /// Typecheck a block. `parent_refinements` carries `Int[lo..hi]` bounds from
    /// enclosing scopes so nested `if`/`while` still enforce assignment bounds.
    fn check_block(
        &self,
        block: &Block,
        env: &mut HashMap<String, Ty>,
        mutable: &mut HashMap<String, bool>,
        ctx: &str,
        expected_ret: Option<&Ty>,
        parent_refinements: &HashMap<String, (i64, i64)>,
        // Const return-type Int[lo..hi] bounds (incl. aliases); enforced on every return + tail.
        return_bounds: Option<(i64, i64)>,
    ) -> Result<Ty> {
        let mut last = Ty::Void;
        let mut refinements: HashMap<String, (i64, i64)> = parent_refinements.clone();
        // Const list lengths for list_new / list_push chains (fail-closed list_get OOB).
        let mut list_lens: HashMap<String, i64> = HashMap::new();
        let mut path_returned = false;
        // Sync for list_get const checks inside infer_expr.
        *self.active_list_lens.borrow_mut() = list_lens.clone();

        for stmt in &block.stmts {
            if path_returned {
                let sp = stmt_span(stmt);
                return Err(anyhow!(
                    "Type error at {}:{}: unreachable code after return",
                    sp.line,
                    sp.col
                ));
            }
            match stmt {
                Statement::Let { .. } => {
                    self.check_stmt_let(env, mutable, &mut refinements, &mut list_lens, &mut last, ctx, stmt)?;
                }
                Statement::Assign { .. } => {
                    self.check_stmt_assign(env, mutable, &mut refinements, &mut list_lens, &mut last, stmt)?;
                }
                Statement::FieldAssign { .. } => {
                    self.check_stmt_field_assign(env, mutable, &mut last, stmt)?;
                }
                Statement::Return(Some(expr), span) => {
                    last = self.infer_expr(expr, env)?;
                    if let Some(exp) = expected_ret {
                        if !matches!(exp, Ty::Void)
                            && !matches!(last, Ty::Unknown)
                            && !self.unify(&last, exp)
                        {
                            return Err(anyhow!(
                                "Type error at {}:{} in '{}': return type {} does not match declared {}",
                                span.line,
                                span.col,
                                ctx,
                                last.display(),
                                exp.display()
                            ));
                        }
                    }
                    if let Some((min_v, max_v)) = return_bounds {
                        if let Some(val) = Ty::const_int(expr) {
                            if val < min_v || val > max_v {
                                let sp = expr.span();
                                return Err(anyhow!(
                                    "Type error at {}:{}: RefinementTypeViolation: Returned value {} out of refinement bounds [{}..{}] for return type in '{}'",
                                    sp.line,
                                    sp.col,
                                    val,
                                    min_v,
                                    max_v,
                                    ctx
                                ));
                            }
                        }
                    }
                    path_returned = true;
                }
                Statement::Return(None, _) => {
                    last = Ty::Void;
                    path_returned = true;
                }
                Statement::Break(span) => {
                    if self.loop_depth.get() == 0 {
                        return Err(anyhow!(
                            "Type error at {}:{}: `break` outside of loop",
                            span.line,
                            span.col
                        ));
                    }
                    last = Ty::Void;
                    path_returned = true;
                }
                Statement::Continue(span) => {
                    if self.loop_depth.get() == 0 {
                        return Err(anyhow!(
                            "Type error at {}:{}: `continue` outside of loop",
                            span.line,
                            span.col
                        ));
                    }
                    last = Ty::Void;
                    path_returned = true;
                }
                Statement::Expr { .. } => {
                    self.check_stmt_expr(env, mutable, &mut refinements, &mut last, &mut path_returned, expected_ret, return_bounds, stmt)?;
                }
                Statement::While { .. } => {
                    self.check_stmt_while(env, mutable, &mut refinements, &mut last, expected_ret, return_bounds, stmt)?;
                }
            }
        }
        if let Some(expr) = &block.expr {
            if path_returned {
                let sp = expr.span();
                return Err(anyhow!(
                    "Type error at {}:{}: unreachable code after return",
                    sp.line,
                    sp.col
                ));
            }
            // Tail expression may be a nested `else if` chain or match.
            // Clone env/mutable per branch so `else if` desugar cannot leak
            // sibling `let`s (else if is a nested if as the else block's tail).
            match expr.as_ref() {
                Expression::If {
                    cond,
                    then_branch,
                    else_branch,
                    span: ispan,
                } => {
                    let ct = self.infer_expr(cond, env)?;
                    if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                        return Err(anyhow!(
                            "Type error at {}:{}: 'if' condition must be Bool, found {}",
                            ispan.line,
                            ispan.col,
                            ct.display()
                        ));
                    }
                    let mut env_then = env.clone();
                    let mut mut_then = mutable.clone();
                    self.check_block(
                        then_branch,
                        &mut env_then,
                        &mut mut_then,
                        "if-then-tail",
                        expected_ret,
                        &refinements,
                        return_bounds,
                    )?;
                    if let Some(eb) = else_branch {
                        let mut env_else = env.clone();
                        let mut mut_else = mutable.clone();
                        self.check_block(
                            eb,
                            &mut env_else,
                            &mut mut_else,
                            "if-else-tail",
                            expected_ret,
                            &refinements,
                            return_bounds,
                        )?;
                    }
                    last = Ty::Void;
                }
                Expression::Match { arms, span: mspan, .. } => {
                    last = self.infer_expr_m(expr, env, mutable)?;
                    // Const arm values as implicit return: enforce Int[lo..hi].
                    if let Some((min_v, max_v)) = return_bounds {
                        for arm in arms {
                            if let Some(val) = Ty::const_int(&arm.body) {
                                if val < min_v || val > max_v {
                                    let sp = arm.body.span();
                                    return Err(anyhow!(
                                        "Type error at {}:{}: RefinementTypeViolation: Returned value {} out of refinement bounds [{}..{}] for match arm return type in '{}' (match at {}:{})",
                                        sp.line,
                                        sp.col,
                                        val,
                                        min_v,
                                        max_v,
                                        ctx,
                                        mspan.line,
                                        mspan.col
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {
                    last = self.infer_expr_m(expr, env, mutable)?;
                    // Tail expression as implicit return: enforce Int[lo..hi] when applicable.
                    if let Some((min_v, max_v)) = return_bounds {
                        if let Some(val) = Ty::const_int(expr) {
                            if val < min_v || val > max_v {
                                let sp = expr.span();
                                return Err(anyhow!(
                                    "Type error at {}:{}: RefinementTypeViolation: Returned value {} out of refinement bounds [{}..{}] for return type in '{}'",
                                    sp.line,
                                    sp.col,
                                    val,
                                    min_v,
                                    max_v,
                                    ctx
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(last)
    }

}
