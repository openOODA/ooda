impl TypeChecker {

    fn check_function(&self, func: &FunctionDecl) -> Result<()> {
        let mut env: HashMap<String, Ty> = HashMap::new();
        let mut mutable: HashMap<String, bool> = HashMap::new();
        for p in &func.params {
            env.insert(p.name.clone(), Ty::from_ast(&p.param_type));
            // Parameters are mutable by default for practical alpha (like many langs);
            // DESIGN immutability-by-default applies to `let` bindings.
            mutable.insert(p.name.clone(), true);
        }

        for req in &func.requires {
            let t = self.infer_expr(req, &env)?;
            if !Ty::unifyable(&t, &Ty::Bool) && !matches!(t, Ty::Unknown) {
                return Err(anyhow!(
                    "Type error in function '{}': 'requires' clause must be Bool, found {}",
                    func.name,
                    t.display()
                ));
            }
        }

        let expected_ret = Ty::from_ast(&func.return_type);
        let empty_refinements = HashMap::new();
        let ret_bounds = self.bounds_from_type_ann(&func.return_type);
        *self.current_return.borrow_mut() = Some(expected_ret.clone());
        let body_ty = self.check_block(
            &func.body,
            &mut env,
            &mut mutable,
            &func.name,
            Some(&expected_ret),
            &empty_refinements,
            ret_bounds,
        );
        *self.current_return.borrow_mut() = None;
        let body_ty = body_ty?;

        let expected = Ty::from_ast(&func.return_type);
        // Fail-closed: non-Void functions must produce a value on every path.
        // Body type Void is OK only when every path hits `return <expr>` (if/else, etc.).
        if !matches!(expected, Ty::Void) {
            if matches!(body_ty, Ty::Void) {
                if !block_always_returns(&func.body) {
                    return Err(anyhow!(
                        "Type error in '{}': function declares return type {} but body has type Void (missing return value)",
                        func.name,
                        expected.display()
                    ));
                }
                // All paths return; per-return types already checked in check_block.
            } else if !matches!(body_ty, Ty::Unknown) && !self.unify(&body_ty, &expected) {
                return Err(anyhow!(
                    "Type error in '{}': function declares return type {} but body has type {}",
                    func.name,
                    expected.display(),
                    body_ty.display()
                ));
            }
        }

        for ens in &func.ensures {
            let mut post = env.clone();
            post.insert("result".into(), expected.clone());
            let t = self.infer_expr(ens, &post)?;
            if !Ty::unifyable(&t, &Ty::Bool) && !matches!(t, Ty::Unknown) {
                return Err(anyhow!(
                    "Type error in function '{}': 'ensures' clause must be Bool, found {}",
                    func.name,
                    t.display()
                ));
            }
        }

        if let Some(verify) = &func.verify_block {
            let mut venv = HashMap::new();
            let mut vmut = HashMap::new();
            let empty = HashMap::new();
            self.check_block(
                verify,
                &mut venv,
                &mut vmut,
                &format!("verify {}", func.name),
                None,
                &empty,
                None,
            )?;
        }

        Ok(())
    }



    /// If `want` has Unknown holes that `got` fills, return the refined type.
    /// Otherwise None (keep existing env binding).
    fn refine_type(want: &Ty, got: &Ty) -> Option<Ty> {
        match (want, got) {
            (Ty::Unknown, g) if !matches!(g, Ty::Unknown) => Some(g.clone()),
            (Ty::List(w), Ty::List(g)) => {
                if matches!(w.as_ref(), Ty::Unknown) && !matches!(g.as_ref(), Ty::Unknown) {
                    Some(Ty::List(g.clone()))
                } else if let Some(inner) = Self::refine_type(w, g) {
                    Some(Ty::List(Box::new(inner)))
                } else {
                    None
                }
            }
            (Ty::Option(w), Ty::Option(g)) => {
                Self::refine_type(w, g).map(|i| Ty::Option(Box::new(i)))
            }
            (Ty::Result(wa, we), Ty::Result(ga, ge)) => {
                let a = Self::refine_type(wa, ga);
                let e = Self::refine_type(we, ge);
                if a.is_none() && e.is_none() {
                    None
                } else {
                    Some(Ty::Result(
                        Box::new(a.unwrap_or_else(|| (**wa).clone())),
                        Box::new(e.unwrap_or_else(|| (**we).clone())),
                    ))
                }
            }
            _ => None,
        }
    }


    /// Const length of list_new / list_push chains, using env of known binding lengths.
    fn const_list_len(expr: &Expression, env_lens: &HashMap<String, i64>) -> Option<i64> {
        match expr {
            Expression::Call { name, args, .. } if name == "list_new" && args.is_empty() => Some(0),
            Expression::Call { name, args, .. } if name == "list_push" && args.len() == 2 => {
                Self::const_list_len(&args[0], env_lens).map(|n| n + 1)
            }
            Expression::Variable(name, _) => env_lens.get(name).copied(),
            _ => None,
        }
    }


    /// Resolve `p` or `p.a.b` (desugared `.field` Call chain) to root binding name
    /// and the type of the parent struct that owns the final assigned field.
    fn field_assign_parent_ty(
        &self,
        object: &Expression,
        env: &HashMap<String, Ty>,
        span: Span,
    ) -> Result<(String, Ty)> {
        match object {
            Expression::Variable(name, _) => {
                let ty = env.get(name).cloned().ok_or_else(|| {
                    anyhow!(
                        "Type error at {}:{}: undefined variable '{}'",
                        span.line,
                        span.col,
                        name
                    )
                })?;
                Ok((name.clone(), ty))
            }
            Expression::Call { name, args, .. }
                if name.starts_with('.') && args.len() == 1 =>
            {
                let field_name = &name[1..];
                let (root, parent_ty) =
                    self.field_assign_parent_ty(&args[0], env, span)?;
                let fields = match &parent_ty {
                    Ty::Struct { fields, .. } => fields,
                    Ty::Custom(n) => match self.type_aliases.get(n) {
                        Some(Ty::Struct { fields, .. }) => fields,
                        _ => {
                            return Err(anyhow!(
                                "Type error at {}:{}: field access on non-struct type {} in assign path",
                                span.line,
                                span.col,
                                parent_ty.display()
                            ));
                        }
                    },
                    other => {
                        return Err(anyhow!(
                            "Type error at {}:{}: field access on non-struct type {} in assign path",
                            span.line,
                            span.col,
                            other.display()
                        ));
                    }
                };
                let child = fields
                    .iter()
                    .find(|(n, _)| n == field_name)
                    .map(|(_, t)| t.clone())
                    .ok_or_else(|| {
                        anyhow!(
                            "Type error at {}:{}: struct has no field '{}'",
                            span.line,
                            span.col,
                            field_name
                        )
                    })?;
                Ok((root, child))
            }
            _ => Err(anyhow!(
                "Type error at {}:{}: field assign requires a variable or field path (e.g. p.x or p.inner.n)",
                span.line,
                span.col
            )),
        }
    }

}
