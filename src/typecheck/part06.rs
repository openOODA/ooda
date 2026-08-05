impl TypeChecker {
    fn check_stmt_let(
        &self,
        env: &mut HashMap<String, Ty>,
        mutable: &mut HashMap<String, bool>,
        refinements: &mut HashMap<String, (i64, i64)>,
        list_lens: &mut HashMap<String, i64>,
        last: &mut Ty,
        path_returned: &mut bool,
        expected_ret: Option<&Ty>,
        return_bounds: Option<(i64, i64)>,
        ctx: &str,
        stmt: &Statement,
    ) -> Result<()> {
        match stmt {
                Statement::Let {
                    name,
                    mutable: is_mut,
                    type_annotation,
                    init,
                    span,
                    ..
                } => {
                    let init_ty = self.infer_expr(init, env)?;
                    // Fail-closed: do not bind Void (e.g. `let x = while …` / discarded unit).
                    if matches!(init_ty, Ty::Void) && name != "_" {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot bind Void value to '{}'; while/if-as-stmt produce Void — use a value expression",
                            span.line,
                            span.col,
                            name
                        ));
                    }
                    // DESIGN must-use: binding to `_` does not discharge Result/Option.
                    if name == "_" && matches!(init_ty, Ty::Result(_, _) | Ty::Option(_)) {
                        return Err(anyhow!(
                            "Type error at {}:{}: unused {} value (must-use); `let _ = ...` does not handle Result/Option — use `match` or `?`",
                            span.line,
                            span.col,
                            init_ty.display()
                        ));
                    }
                    if let Some(ann) = type_annotation {
                        let want = Ty::from_ast(ann);
                        // Bare Int[lo..hi] or type alias that carries those bounds.
                        if let Some((min_v, max_v)) = self.bounds_from_type_ann(ann) {
                            refinements.insert(name.clone(), (min_v, max_v));
                            if let Some(val) = Ty::const_int(init) {
                                if val < min_v || val > max_v {
                                    let sp = init.span();
                                    return Err(anyhow!(
                                        "Type error at {}:{}: RefinementTypeViolation: Value {} out of refinement bounds [{}..{}] for '{}'",
                                        sp.line,
                                        sp.col,
                                        val,
                                        min_v,
                                        max_v,
                                        name
                                    ));
                                }
                            }
                        }
                        if !self.unify(&init_ty, &want) {
                            return Err(anyhow!(
                                "Type error at {}:{} in '{}': let '{}' annotated as {} but initializer has type {}",
                                span.line,
                                span.col,
                                ctx,
                                name,
                                want.display(),
                                init_ty.display()
                            ));
                        }
                        env.insert(name.clone(), want);
                    } else {
                        env.insert(name.clone(), init_ty);
                    }
                    if let Some(len) = Self::const_list_len(init, &list_lens) {
                        list_lens.insert(name.clone(), len);
                    } else {
                        list_lens.remove(name);
                    }
                    *self.active_list_lens.borrow_mut() = list_lens.clone();
                    mutable.insert(name.clone(), *is_mut);
                    *last = Ty::Void;
                }
            _ => unreachable!("check_stmt_let"),
        }
        Ok(())
    }


    fn check_stmt_assign(
        &self,
        env: &mut HashMap<String, Ty>,
        mutable: &mut HashMap<String, bool>,
        refinements: &mut HashMap<String, (i64, i64)>,
        list_lens: &mut HashMap<String, i64>,
        last: &mut Ty,
        path_returned: &mut bool,
        expected_ret: Option<&Ty>,
        return_bounds: Option<(i64, i64)>,
        ctx: &str,
        stmt: &Statement,
    ) -> Result<()> {
        match stmt {
                Statement::Assign { name, value, span } => {
                    if !env.contains_key(name) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign to undefined variable '{}'",
                            span.line,
                            span.col,
                            name
                        ));
                    }
                    if !mutable.get(name).copied().unwrap_or(false) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign to immutable binding '{}'; use `let mut {}`",
                            span.line,
                            span.col,
                            name,
                            name
                        ));
                    }
                    if let Some(&(min_v, max_v)) = refinements.get(name) {
                        if let Some(val) = Ty::const_int(value) {
                            if val < min_v || val > max_v {
                                let sp = value.span();
                                return Err(anyhow!(
                                    "Type error at {}:{}: RefinementTypeViolation: Value {} out of refinement bounds [{}..{}] for assignment to '{}'",
                                    sp.line,
                                    sp.col,
                                    val,
                                    min_v,
                                    max_v,
                                    name
                                ));
                            }
                        }
                    }
                    let vty = self.infer_expr(value, env)?;
                    let want = env.get(name).cloned().unwrap_or(Ty::Unknown);
                    if !self.unify(&vty, &want) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign {} to '{}' of type {}",
                            span.line,
                            span.col,
                            vty.display(),
                            name,
                            want.display()
                        ));
                    }
                    // Refine env when assignment narrows Unknown holes (e.g. List[_]
                    // from list_new becomes List[Int] after list_push). Critical for
                    // `for x in xs` after unannotated list building.
                    if let Some(refined) = Self::refine_type(&want, &vty) {
                        env.insert(name.clone(), refined);
                    }
                    if let Some(len) = Self::const_list_len(value, &list_lens) {
                        list_lens.insert(name.clone(), len);
                    } else {
                        list_lens.remove(name);
                    }
                    *self.active_list_lens.borrow_mut() = list_lens.clone();
                    *last = Ty::Void;
                }
            _ => unreachable!("check_stmt_assign"),
        }
        Ok(())
    }

}
