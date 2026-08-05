impl TypeChecker {

    fn infer_if_expr(
        &self,
        expr: &Expression,
        env: &HashMap<String, Ty>,
        mutable: &HashMap<String, bool>,
    ) -> Result<Ty> {
        match expr {
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let ct = self.infer_expr(cond, env)?;
                if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                    return Err(anyhow!(
                        "Type error at {}:{}: 'if' condition must be Bool, found {}",
                        expr.span().line,
                        expr.span().col,
                        ct.display()
                    ));
                }
                // Expression-level if: inherit env. Mut map is empty here (infer_expr
                // has no parent mut); statement-level if in check_block carries real mut.
                // Match/value-if that assign outer `let mut` use eval shadow-restore;
                // typecheck of those assigns is best-effort via env-only (see check_block Match).
                let mut env_then = env.clone();
                let mut mut_then = mutable.clone();
                // Expression-level if inherits mutability from parent (match arms, value-if).
                let empty_ref = HashMap::new();
                let t1 = self.check_block(
                    then_branch,
                    &mut env_then,
                    &mut mut_then,
                    "if-then",
                    None,
                    &empty_ref,
                    None,
                )?;
                if let Some(else_b) = else_branch {
                    let mut env_else = env.clone();
                    let mut mut_else = mutable.clone();
                    let t2 = self.check_block(
                        else_b,
                        &mut env_else,
                        &mut mut_else,
                        "if-else",
                        None,
                        &empty_ref,
                        None,
                    )?;
                    if Ty::unifyable(&t1, &t2) {
                        Ok(t1)
                    } else if matches!(t1, Ty::Unknown) || matches!(t2, Ty::Unknown) {
                        Ok(Ty::Unknown)
                    } else if matches!(t1, Ty::Void) {
                        // Statement-like then-arm (e.g. nested if-as-stmt in else branch)
                        Ok(t2)
                    } else if matches!(t2, Ty::Void) {
                        Ok(t1)
                    } else {
                        Err(anyhow!(
                            "Type error at {}:{}: if/else branches have incompatible types {} vs {}",
                            expr.span().line,
                            expr.span().col,
                            t1.display(),
                            t2.display()
                        ))
                    }
                } else {
                    // Fail-closed: value-producing if without else has no type on false path
                    // (was runtime () / Void while typecheck claimed Int).
                    if !matches!(t1, Ty::Void | Ty::Unknown) {
                        return Err(anyhow!(
                            "Type error at {}:{}: if expression producing {} requires an else branch",
                            expr.span().line,
                            expr.span().col,
                            t1.display()
                        ));
                    }
                    Ok(t1)
                }
            }
            _ => unreachable!("infer_if_expr"),
        }
    }


    fn infer_match_expr(
        &self,
        expr: &Expression,
        env: &HashMap<String, Ty>,
        mutable: &HashMap<String, bool>,
    ) -> Result<Ty> {
        match expr {
            Expression::Match { expr, arms, span, .. } => {
                let scrutinee_ty = self.infer_expr(expr, env)?;
                let mut result: Option<Ty> = None;
                let mut has_ok = false;
                let mut has_err = false;
                let mut has_some = false;
                let mut has_none = false;
                let mut has_true = false;
                let mut has_false = false;
                let mut has_wildcard = false;
                for arm in arms {
                    let mut arm_env = env.clone();
                    match &arm.pattern {
                        Pattern::Wildcard => has_wildcard = true,
                        Pattern::Variant { name, arg } => {
                            match name.as_str() {
                                "Ok" => has_ok = true,
                                "Err" => has_err = true,
                                "Some" => has_some = true,
                                "None" => has_none = true,
                                _ => {}
                            }
                            if let Some(var) = arg {
                                let payload = match (&scrutinee_ty, name.as_str()) {
                                    (Ty::Result(ok, _), "Ok") => (**ok).clone(),
                                    (Ty::Result(_, err), "Err") => (**err).clone(),
                                    (Ty::Option(inner), "Some") => (**inner).clone(),
                                    _ => Ty::Unknown,
                                };
                                arm_env.insert(var.clone(), payload);
                            }
                        }
                        Pattern::Literal(Literal::Bool(true)) => has_true = true,
                        Pattern::Literal(Literal::Bool(false)) => has_false = true,
                        Pattern::Literal(_) => {}
                    }
                    let t = self.infer_expr_m(&arm.body, &arm_env, mutable)?;
                    match &result {
                        None => result = Some(t),
                        Some(prev) => {
                            // Unknown holes from Ok/Some constructors may unify with concrete arms.
                            if !Ty::unifyable_or_unknown_hole(prev, &t)
                                && !(matches!(prev, Ty::Void) || matches!(t, Ty::Void))
                            {
                                return Err(anyhow!(
                                    "Type error at {}:{}: match arms have incompatible types {} vs {}",
                                    span.line,
                                    span.col,
                                    prev.display(),
                                    t.display()
                                ));
                            }
                            // Prefer concrete type over Unknown/Void when possible.
                            if matches!(prev, Ty::Unknown | Ty::Void) && !matches!(t, Ty::Unknown | Ty::Void)
                            {
                                result = Some(t);
                            }
                        }
                    }
                }

                // DESIGN: exhaustive matching for Result/Option/Bool (no silent fall-through).
                if !has_wildcard {
                    match &scrutinee_ty {
                        Ty::Result(_, _) if !(has_ok && has_err) => {
                            return Err(anyhow!(
                                "Type error at {}:{}: non-exhaustive match on Result — cover both Ok(_) and Err(_), or use `_`",
                                span.line,
                                span.col
                            ));
                        }
                        Ty::Bool if !(has_true && has_false) => {
                            return Err(anyhow!(
                                "Type error at {}:{}: non-exhaustive match on Bool — cover both true and false, or use `_`",
                                span.line,
                                span.col
                            ));
                        }
                        Ty::Option(_) if !(has_some && has_none) => {
                            return Err(anyhow!(
                                "Type error at {}:{}: non-exhaustive match on Option — cover both Some(_) and None, or use `_`",
                                span.line,
                                span.col
                            ));
                        }
                        _ => {}
                    }
                }
                Ok(result.unwrap_or(Ty::Void))
            }
            _ => unreachable!("infer_match_expr"),
        }
    }

}
