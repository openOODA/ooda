impl WasmCodeGen {


    /// Best-effort type inference for an expression: declared local
    /// type > literal shape > default i64.
    fn infer_expr_type(expr: &Expression, locals: &BTreeMap<String, &'static str>) -> &'static str {
        match expr {
            Expression::Literal(Literal::Float(_), _) => "f64",
            Expression::Literal(Literal::Bool(_), _) => "i64",
            Expression::Literal(Literal::Int(_), _) => "i64",
            Expression::Literal(Literal::String(_), _) => "i32",
            Expression::StructLit { name, .. } => Box::leak(name.clone().into_boxed_str()),
            Expression::Variable(name, _) => locals.get(name).copied().unwrap_or("i64"),
            Expression::Binary { op, left, right, .. } => {
                // Comparisons always yield Bool (i64 0/1). Arithmetic preserves operand class.
                match op {
                    BinOp::Eq
                    | BinOp::Neq
                    | BinOp::Gt
                    | BinOp::Lt
                    | BinOp::Gte
                    | BinOp::Lte
                    | BinOp::And
                    | BinOp::Or => "i64",
                    _ => {
                        let lt = Self::infer_expr_type(left, locals);
                        let rt = Self::infer_expr_type(right, locals);
                        if lt == "f64" || rt == "f64" {
                            "f64"
                        } else if lt == "i32" || rt == "i32" {
                            "i32"
                        } else {
                            "i64"
                        }
                    }
                }
            }
            Expression::Call { name, args, .. } => {
                if name == "list_new" || name == "list_push" || name == ".push" {
                    if let Some(arg0) = args.first() {
                        let ty = Self::infer_expr_type(arg0, locals);
                        if ty == "list_str" { return "list_str"; }
                    }
                    "list"
                } else if name == "list_get" {
                    if let Some(arg0) = args.first() {
                        let ty = Self::infer_expr_type(arg0, locals);
                        if ty == "list_str" {
                            return "i32";
                        }
                    }
                    "i64"
                } else if name == "list_len" || name == ".char_at" || name == ".contains" {
                    "i64"
                } else if name == ".len" {
                    // List .len → i64; String .len → i64 (same numeric width).
                    "i64"
                } else if name == ".str_slice" {
                    "i32" // new string pointer
                } else {
                    let _ = args;
                    "i64"
                }
            }
            // Default to i64 for everything else (if, match, etc.).
            _ => "i64",
        }
    }


    /// Collect `let` bindings in a block and nested while/if (stack-only map updates).
    fn collect_locals_in_block(
        block: &Block,
        locals: &mut BTreeMap<String, &'static str>,
    ) {
        for stmt in &block.stmts {
            Self::collect_locals_in_stmt(stmt, locals);
        }
        if let Some(e) = &block.expr {
            Self::collect_locals_in_expr(e, locals);
        }
    }


    fn collect_locals_in_stmt(stmt: &Statement, locals: &mut BTreeMap<String, &'static str>) {
        match stmt {
            Statement::Let {
                name,
                init,
                type_annotation,
                ..
            } => {
                let ty = if let Some(Type::List(inner)) = type_annotation {
                    if Self::is_type_string(inner.as_ref()) { "list_str" } else { "list" }
                } else {
                    Self::infer_expr_type(init, locals)
                };
                locals.insert(name.clone(), ty);
                Self::collect_locals_in_expr(init, locals);
            }
            Statement::Assign { name, value, .. } => {
                Self::collect_locals_in_expr(value, locals);
                // Refine untyped list → list_str when assigned from push of String (matches typecheck).
                if let Expression::Call {
                    name: cname, args, ..
                } = value
                {
                    if (cname == "list_push" || cname == ".push") && args.len() >= 2 {
                        let elem_ty = Self::infer_expr_type(&args[1], locals);
                        if elem_ty == "i32" {
                            locals.insert(name.clone(), "list_str");
                        }
                    }
                }
            }
            Statement::Return(Some(value), _) => {
                Self::collect_locals_in_expr(value, locals);
            }
            Statement::Expr(e, _) => Self::collect_locals_in_expr(e, locals),
            Statement::While { cond, body, .. } => {
                Self::collect_locals_in_expr(cond, locals);
                Self::collect_locals_in_block(body, locals);
            }
            Statement::FieldAssign { object, value, .. } => {
                Self::collect_locals_in_expr(object, locals);
                Self::collect_locals_in_expr(value, locals);
            }
            Statement::Return(None, _) | Statement::Break(_) | Statement::Continue(_) => {}
        }
    }

}
