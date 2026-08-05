impl WasmCodeGen {

    fn program_needs_list_runtime(program: &Program) -> bool {
        for item in &program.items {
            if let Item::Function(f) = item {
                if Self::type_is_list(&f.return_type) {
                    return true;
                }
                for p in &f.params {
                    if Self::type_is_list(&p.param_type) {
                        return true;
                    }
                }
                if Self::block_needs_list(&f.body) {
                    return true;
                }
            }
        }
        false
    }


    
    fn is_type_string(t: &Type) -> bool {
        matches!(t, Type::String) || matches!(t, Type::Custom(s) if s == "String")
    }


    fn type_is_list(t: &Type) -> bool {
        matches!(t, Type::List(_))
    }


    fn block_needs_list(block: &Block) -> bool {
        for stmt in &block.stmts {
            if Self::stmt_needs_list(stmt) {
                return true;
            }
        }
        if let Some(e) = &block.expr {
            return Self::expr_needs_list(e);
        }
        false
    }


    fn stmt_needs_list(stmt: &Statement) -> bool {
        match stmt {
            Statement::Let {
                type_annotation,
                init,
                ..
            } => {
                if type_annotation
                    .as_ref()
                    .map(Self::type_is_list)
                    .unwrap_or(false)
                {
                    return true;
                }
                Self::expr_needs_list(init)
            }
            Statement::Assign { value, .. } | Statement::Return(Some(value), _) => {
                Self::expr_needs_list(value)
            }
            Statement::Expr(e, _) => Self::expr_needs_list(e),
            Statement::While { cond, body, .. } => {
                Self::expr_needs_list(cond) || Self::block_needs_list(body)
            }
            Statement::FieldAssign { object, value, .. } => {
                Self::expr_needs_list(object) || Self::expr_needs_list(value)
            }
            Statement::Return(None, _) | Statement::Break(_) | Statement::Continue(_) => false,
        }
    }


    fn expr_needs_list(expr: &Expression) -> bool {
        match expr {
            Expression::Call { name, args, .. } => {
                match name.as_str() {
                    // Explicit list ops always need the bump-heap list runtime.
                    "list_new" | "list_push" | "list_get" | "list_len" | ".push" => return true,
                    ".len" => {
                        // E-M W↓: String `.len` is pure WAT (NUL scan), whether the
                        // receiver is a literal or a String-typed local (`i32`).
                        // List `.len` needs `$list_len` only when the receiver is
                        // list-shaped (list_new/list_push/.push or nested list expr).
                        // Do NOT inject list RT for every non-literal `.len` — that
                        // falsely bloated `let s = "hi"; s.len()` and string_ops.oo.
                        if let Some(recv) = args.first() {
                            if Self::expr_is_list_shaped(recv) {
                                return true;
                            }
                        }
                    }
                    ".char_at" | ".contains" | ".str_slice" => {} // string surface, not list RT
                    _ => {}
                }
                args.iter().any(Self::expr_needs_list)
            }
            Expression::Binary { left, right, .. } => {
                Self::expr_needs_list(left) || Self::expr_needs_list(right)
            }
            Expression::Unary { expr, .. } => Self::expr_needs_list(expr),
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::expr_needs_list(cond)
                    || Self::block_needs_list(then_branch)
                    || else_branch
                        .as_ref()
                        .map(|b| Self::block_needs_list(b))
                        .unwrap_or(false)
            }
            Expression::While { cond, body, .. } => {
                Self::expr_needs_list(cond) || Self::block_needs_list(body)
            }
            Expression::Match { expr: scrut, arms, .. } => {
                Self::expr_needs_list(scrut) || arms.iter().any(|a| Self::expr_needs_list(&a.body))
            }
            Expression::StructLit { fields, .. } => {
                fields.iter().any(|(_, e)| Self::expr_needs_list(e))
            }
            Expression::Literal(_, _) | Expression::Variable(_, _) => false,
        }
    }


    /// True when `expr` is known to produce a List pointer (not String i32).
    /// Used to decide whether `.len` needs `$list_len` RT vs pure string WAT.
    /// List-typed parameters / annotations still force RT via `program_needs_list_runtime`
    /// and `stmt_needs_list`; this only classifies expression shape at a use site.
    fn expr_is_list_shaped(expr: &Expression) -> bool {
        match expr {
            Expression::Call { name, .. } => {
                matches!(name.as_str(), "list_new" | "list_push" | ".push")
            }
            Expression::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::block_tail_is_list_shaped(then_branch)
                    || else_branch
                        .as_ref()
                        .map(|b| Self::block_tail_is_list_shaped(b))
                        .unwrap_or(false)
            }
            _ => false,
        }
    }


    fn block_tail_is_list_shaped(block: &Block) -> bool {
        block
            .expr
            .as_ref()
            .map(|e| Self::expr_is_list_shaped(e))
            .unwrap_or(false)
    }


    fn require_list_supported(inner: &Type, ctx: &str) -> Result<()> {
        match inner {
            Type::Int | Type::String => Ok(()),
            Type::Custom(s) if s == "Int" || s == "String" || s == "_" => Ok(()), // unrefined / pending
            other => bail!(
                "WASM backend only supports List[Int]/List[String] (not {:?}) in '{}'.",
                other,
                ctx
            ),
        }
    }


    /// Map semantic local tags (`list` vs string `i32`) to WAT storage types.
    fn wat_storage_ty(sem: &str) -> &'static str {
        match sem {
            "list" | "list_str" | "i32" => "i32",
            "f64" => "f64",
            _ => "i64",
        }
    }

}
