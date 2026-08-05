impl TypeChecker {

    fn check_stmt_field_assign(
        &self,
        env: &mut HashMap<String, Ty>,
        mutable: &mut HashMap<String, bool>,
        last: &mut Ty,
        stmt: &Statement,
    ) -> Result<()> {
        match stmt {
                Statement::FieldAssign {
                    object,
                    field,
                    value,
                    span,
                } => {
                    // Allow `p.x = v` and nested `p.inner.n = v` (object may be
                    // a chain of desugared `.field` Calls ending at a Variable).
                    let (root_name, parent_ty) =
                        self.field_assign_parent_ty(object, env, *span)?;
                    if !mutable.get(&root_name).copied().unwrap_or(false) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign to field of immutable binding '{}'; use `let mut {}`",
                            span.line,
                            span.col,
                            root_name,
                            root_name
                        ));
                    }
                    let fields = match &parent_ty {
                        Ty::Struct { fields, .. } => fields.clone(),
                        Ty::Custom(n) => match self.type_aliases.get(n) {
                            Some(Ty::Struct { fields, .. }) => fields.clone(),
                            _ => {
                                return Err(anyhow!(
                                    "Type error at {}:{}: field assign on non-struct type {}",
                                    span.line,
                                    span.col,
                                    parent_ty.display()
                                ));
                            }
                        },
                        other => {
                            return Err(anyhow!(
                                "Type error at {}:{}: field assign on non-struct type {}",
                                span.line,
                                span.col,
                                other.display()
                            ));
                        }
                    };
                    let want = fields
                        .iter()
                        .find(|(n, _)| n == field)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| {
                            anyhow!(
                                "Type error at {}:{}: struct has no field '{}'",
                                span.line,
                                span.col,
                                field
                            )
                        })?;
                    let vty = self.infer_expr(value, env)?;
                    if !self.unify(&vty, &want) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign {} to field '{}' of type {}",
                            span.line,
                            span.col,
                            vty.display(),
                            field,
                            want.display()
                        ));
                    }
                    *last = Ty::Void;
                }
            _ => unreachable!("check_stmt_field_assign"),
        }
        Ok(())
    }


    fn check_stmt_expr(
        &self,
        env: &mut HashMap<String, Ty>,
        mutable: &mut HashMap<String, bool>,
        refinements: &mut HashMap<String, (i64, i64)>,
        last: &mut Ty,
        path_returned: &mut bool,
        expected_ret: Option<&Ty>,
        return_bounds: Option<(i64, i64)>,
        stmt: &Statement,
    ) -> Result<()> {
        match stmt {
                Statement::Expr(expr, span) => {
                    // Statement-level if/while must inherit mutability so
                    // `let mut x` can be assigned inside branches (CHS oodac).
                    // Nested blocks clone env/mutable so `let` bindings do not
                    // leak into the outer scope (assign to outer mut still works).
                    match expr {
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
                                "if-then",
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
                                    "if-else",
                                    expected_ret,
                                    &refinements,
                                    return_bounds,
                                )?;
                            }
                            *last = Ty::Void;
                            if expr_paths_return(expr) {
                                *path_returned = true;
                            }
                        }
                        Expression::While {
                            cond,
                            body,
                            span: wspan,
                        } => {
                            let ct = self.infer_expr(cond, env)?;
                            if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: while condition must be Bool, found {}",
                                    wspan.line,
                                    wspan.col,
                                    ct.display()
                                ));
                            }
                            let mut env_w = env.clone();
                            let mut mut_w = mutable.clone();
                            self.loop_depth.set(self.loop_depth.get() + 1);
                            let wres = self.check_block(
                                body,
                                &mut env_w,
                                &mut mut_w,
                                "while-expr-stmt",
                                expected_ret,
                                &refinements,
                                return_bounds,
                            );
                            self.loop_depth.set(self.loop_depth.get().saturating_sub(1));
                            wres?;
                            *last = Ty::Void;
                        }
                        _ => {
                            let t = self.infer_expr_m(expr, env, mutable)?;
                            // DESIGN must-use: discarded Result/Option is a hard error.
                            if matches!(t, Ty::Result(_, _) | Ty::Option(_)) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: unused {} value (must-use); handle with `match` / `?` — bare discard and `let _ = ...` are not enough",
                                    span.line,
                                    span.col,
                                    t.display()
                                ));
                            }
                            *last = t;
                        }
                    }
                }
            _ => unreachable!("check_stmt_expr"),
        }
        Ok(())
    }


    fn check_stmt_while(
        &self,
        env: &mut HashMap<String, Ty>,
        mutable: &mut HashMap<String, bool>,
        refinements: &mut HashMap<String, (i64, i64)>,
        last: &mut Ty,
        expected_ret: Option<&Ty>,
        return_bounds: Option<(i64, i64)>,
        stmt: &Statement,
    ) -> Result<()> {
        match stmt {
                Statement::While { cond, body, span } => {
                    let ct = self.infer_expr(cond, env)?;
                    if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                        return Err(anyhow!(
                            "Type error at {}:{}: while condition must be Bool, found {}",
                            span.line,
                            span.col,
                            ct.display()
                        ));
                    }
                    let mut env_w = env.clone();
                    let mut mut_w = mutable.clone();
                    self.loop_depth.set(self.loop_depth.get() + 1);
                    let wres = self.check_block(
                        body,
                        &mut env_w,
                        &mut mut_w,
                        "while-body",
                        expected_ret,
                        &refinements,
                        return_bounds,
                    );
                    self.loop_depth.set(self.loop_depth.get().saturating_sub(1));
                    wres?;
                    *last = Ty::Void;
                }
            _ => unreachable!("check_stmt_while"),
        }
        Ok(())
    }

}
