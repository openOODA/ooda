impl TypeChecker {

    fn infer_struct_lit_expr(
        &self,
        expr: &Expression,
        env: &HashMap<String, Ty>,
        mutable: &HashMap<String, bool>,
    ) -> Result<Ty> {
        match expr {
            Expression::StructLit { name, fields, span } => {
                let def = self.type_aliases.get(name).cloned().ok_or_else(|| {
                    anyhow!(
                        "Type error at {}:{}: unknown struct type '{}'",
                        span.line,
                        span.col,
                        name
                    )
                })?;
                match def {
                    Ty::Struct {
                        name: sn,
                        fields: def_fields,
                    } => {
                        for (fname, fexpr) in fields {
                            let fty = self.infer_expr(fexpr, env)?;
                            if let Some((_, want)) =
                                def_fields.iter().find(|(n, _)| n == fname)
                            {
                                if !Ty::unifyable(&fty, want) {
                                    return Err(anyhow!(
                                        "Type error at {}:{}: field '{}' of '{}' expects {}, found {}",
                                        span.line,
                                        span.col,
                                        fname,
                                        name,
                                        want.display(),
                                        fty.display()
                                    ));
                                }
                            } else {
                                return Err(anyhow!(
                                    "Type error at {}:{}: struct '{}' has no field '{}'",
                                    span.line,
                                    span.col,
                                    name,
                                    fname
                                ));
                            }
                        }
                        for (def_name, _) in &def_fields {
                            if !fields.iter().any(|(n, _)| n == def_name) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: missing required field '{}' in struct literal for '{}'",
                                    span.line,
                                    span.col,
                                    def_name,
                                    name
                                ));
                            }
                        }
                        Ok(Ty::Struct {
                            name: sn.or_else(|| Some(name.clone())),
                            fields: def_fields,
                        })
                    }
                    other => Err(anyhow!(
                        "Type error at {}:{}: '{}' is not a struct type (found {})",
                        span.line,
                        span.col,
                        name,
                        other.display()
                    )),
                }
            }
            _ => unreachable!("infer_struct_lit_expr"),
        }
    }


    fn infer_call(
        &self,
        name: &str,
        args: &[Expression],
        span: &crate::ast::Span,
        propagate_err: &bool,
        env: &HashMap<String, Ty>,
        expr: &Expression,
    ) -> Result<Ty> {
        // Apply `?`: Result[T, E] → T. Only legal in Result-returning functions.
        let apply_try = |ty: Ty| -> Result<Ty> {
            if !*propagate_err {
                return Ok(ty);
            }
            let Ty::Result(ok, err) = ty else {
                return Err(anyhow!(
                    "Type error at {}:{}: `?` requires Result, found {}",
                    span.line,
                    span.col,
                    ty.display()
                ));
            };
            let encl = self.current_return.borrow().clone();
            match encl {
                Some(Ty::Result(_, e_err)) => {
                    if !self.unify_or_hole(&err, &e_err)
                        && !matches!(*err, Ty::Unknown)
                        && !matches!(*e_err, Ty::Unknown)
                    {
                        return Err(anyhow!(
                            "Type error at {}:{}: `?` error type {} does not match function Err type {}",
                            span.line,
                            span.col,
                            err.display(),
                            e_err.display()
                        ));
                    }
                    Ok(*ok)
                }
                Some(other) => Err(anyhow!(
                    "Type error at {}:{}: `?` only allowed in functions returning Result, found return type {}",
                    span.line,
                    span.col,
                    other.display()
                )),
                None => Err(anyhow!(
                    "Type error at {}:{}: `?` only allowed inside a function body",
                    span.line,
                    span.col
                )),
            }
        };
        // `old(x)` references a parameter snapshot. The first arg
        // must be a Variable that exists in the enclosing
        // function's parameter list (the `env` here is the
        // function-body scope at the point of the ensures
        // expression). This gives a clearer error than the
        // generic "undefined variable" path.
        if name == "old" {
            let arg = args.first().ok_or_else(|| {
                anyhow!(
                    "Type error at {}:{}: `old(...)` requires a parameter name argument",
                    expr.span().line,
                    expr.span().col
                )
            })?;
            if let Expression::Variable(vname, _) = arg {
                if let Some(ty) = env.get(vname) {
                    return Ok(ty.clone());
                }
                return Err(anyhow!(
                    "Type error at {}:{}: `old({})` references no parameter; \
                     `old` snapshots parameter values — pass a real parameter name",
                    expr.span().line,
                    expr.span().col,
                    vname
                ));
            }
            return Err(anyhow!(
                "Type error at {}:{}: `old` first argument must be a parameter name (Variable), \
                     got a non-Variable expression",
                expr.span().line,
                expr.span().col
            ));
        }

        // Methods: .len, .trim, sealed object-cap methods, etc.
        // `args[0]` is the receiver (desugared).
        if name.starts_with('.') {
            return apply_try(self.infer_method_call(name, args, env, expr)?);
        }

        let mut arg_tys = Vec::new();
        for a in args {
            arg_tys.push(self.infer_expr(a, env)?);
        }

        // List surface: track element types (no soft List[Unknown] forever).
        if name == "list_new" || name == "list_push" || name == "list_get" || name == "list_len" || name == "char_at" {
            return self.infer_call_specials_0(name, args, &arg_tys, env, expr);
        }
        if name == "str_slice" || name == "Ok" || name == "Err" || name == "Some" || name == "assert_eq" || name == "sys_exec" || name == "exec" || name == "spawn_process" {
            return self.infer_call_specials_1(name, args, &arg_tys, env, expr);
        }
        if let Some((params, ret)) = self.functions.get(name) {
            // println is varargs at runtime (prints every arg).
            let is_println = name == "println";
            if !is_println && params.len() != arg_tys.len() {
                return Err(anyhow!(
                    "Type error at {}:{}: function '{}' expects {} argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    name,
                    params.len(),
                    arg_tys.len()
                ));
            }
            let n = params.len().min(arg_tys.len());
            for (i, (pt, at)) in params.iter().zip(arg_tys.iter()).take(n).enumerate() {
                // Unknown in builtin signatures is a polymorphic hole, not a wildcard
                // for user annotations (those still fail-closed via unifyable).
                if !self.unify_or_hole(pt, at) {
                    return Err(anyhow!(
                        "Type error at {}:{}: function '{}' argument {} expects {}, found {}",
                        expr.span().line,
                        expr.span().col,
                        name,
                        i,
                        pt.display(),
                        at.display()
                    ));
                }
            }
            // Const call-site refinement: Int[lo..hi] params reject out-of-bounds literals.
            if let Some(bounds) = self.param_refinements.get(name) {
                for (i, bound) in bounds.iter().enumerate() {
                    if let Some((lo, hi)) = bound {
                        if let Some(arg_expr) = args.get(i) {
                            if let Some(val) = Ty::const_int(arg_expr) {
                                if val < *lo || val > *hi {
                                    let sp = arg_expr.span();
                                    return Err(anyhow!(
                                        "Type error at {}:{}: RefinementTypeViolation: argument {} value {} out of refinement bounds [{}..{}] for parameter of function '{}'",
                                        sp.line,
                                        sp.col,
                                        i,
                                        val,
                                        lo,
                                        hi,
                                        name
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            return apply_try(ret.clone());
        }

        // Fail-closed: unknown free functions must not soft-accept as Ty::Unknown.
        // (Methods and registered builtins are handled above.)
        Err(anyhow!(
            "Type error at {}:{}: undefined function '{}'",
            expr.span().line,
            expr.span().col,
            name
        ))
    }

}
