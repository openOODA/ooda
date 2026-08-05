impl TypeChecker {



    fn infer_call_specials_0(
        &self,
        name: &str,
        args: &[Expression],
        arg_tys: &[Ty],
        env: &HashMap<String, Ty>,
        expr: &Expression,
    ) -> Result<Ty> {
        if name == "list_new" {
            if !arg_tys.is_empty() {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'list_new' expects 0 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            return Ok(Ty::List(Box::new(Ty::Unknown)));
        }
        if name == "list_push" {
            if arg_tys.len() != 2 {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'list_push' expects 2 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            let elem = match &arg_tys[0] {
                Ty::List(inner) => (**inner).clone(),
                other => {
                    return Err(anyhow!(
                        "Type error at {}:{}: function 'list_push' argument 0 expects List, found {}",
                        expr.span().line,
                        expr.span().col,
                        other.display()
                    ));
                }
            };
            let pushed = &arg_tys[1];
            let out_elem = if matches!(elem, Ty::Unknown) {
                pushed.clone()
            } else if Ty::unifyable_or_unknown_hole(&elem, pushed) {
                // Prefer concrete list element over Unknown push (shouldn't happen often).
                if matches!(pushed, Ty::Unknown) {
                    elem
                } else if Ty::unifyable(&elem, pushed) {
                    elem
                } else {
                    // hole on one side only — keep non-Unknown
                    elem
                }
            } else {
                return Err(anyhow!(
                    "Type error at {}:{}: list element type mismatch: List[{}] cannot push {}",
                    expr.span().line,
                    expr.span().col,
                    elem.display(),
                    pushed.display()
                ));
            };
            return Ok(Ty::List(Box::new(out_elem)));
        }
        if name == "list_get" {
            if arg_tys.len() != 2 {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'list_get' expects 2 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            if !Ty::unifyable_or_unknown_hole(&arg_tys[1], &Ty::Int) {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'list_get' argument 1 expects Int, found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys[1].display()
                ));
            }
            // Const index bounds: negative always fail; known list lengths fail OOB.
            if let Some(idx) = Ty::const_int(&args[1]) {
                if idx < 0 {
                    return Err(anyhow!(
                        "Type error at {}:{}: list_get index {} is negative (const bounds check)",
                        expr.span().line,
                        expr.span().col,
                        idx
                    ));
                }
                let lens = self.active_list_lens.borrow();
                if let Some(len) = Self::const_list_len(&args[0], &lens) {
                    if idx >= len {
                        return Err(anyhow!(
                            "Type error at {}:{}: list_get index {} out of bounds for list of length {} (const bounds check)",
                            expr.span().line,
                            expr.span().col,
                            idx,
                            len
                        ));
                    }
                }
            }
            return match &arg_tys[0] {
                Ty::List(inner) => Ok((**inner).clone()),
                other => Err(anyhow!(
                    "Type error at {}:{}: function 'list_get' argument 0 expects List, found {}",
                    expr.span().line,
                    expr.span().col,
                    other.display()
                )),
            };
        }
        if name == "list_len" {
            if arg_tys.len() != 1 {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'list_len' expects 1 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            return match &arg_tys[0] {
                Ty::List(_) => Ok(Ty::Int),
                other => Err(anyhow!(
                    "Type error at {}:{}: function 'list_len' argument 0 expects List, found {}",
                    expr.span().line,
                    expr.span().col,
                    other.display()
                )),
            };
        }

        // Const string indexing — fail-closed OOB (was typecheck-green, runtime trap).
        if name == "char_at" {
            if arg_tys.len() != 2 {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'char_at' expects 2 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            if !Ty::unifyable_or_unknown_hole(&arg_tys[0], &Ty::String) {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'char_at' argument 0 expects String, found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys[0].display()
                ));
            }
            if !Ty::unifyable_or_unknown_hole(&arg_tys[1], &Ty::Int) {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'char_at' argument 1 expects Int, found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys[1].display()
                ));
            }
            if let (Some(s), Some(idx)) =
                (Ty::const_str(&args[0]), Ty::const_int(&args[1]))
            {
                let len = s.chars().count() as i64;
                if idx < 0 || idx >= len {
                    return Err(anyhow!(
                        "Type error at {}:{}: char_at index {} out of bounds for string literal of length {} (const bounds check)",
                        expr.span().line,
                        expr.span().col,
                        idx,
                        len
                    ));
                }
            }
            return Ok(Ty::String);
        }
        unreachable!("specials 0: {}", name);
    }

}
