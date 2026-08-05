impl WasmCodeGen {

    fn emit_function(func: &FunctionDecl, aliases: &std::collections::HashMap<String, Type>) -> Result<String> {
        let mut f_wat = String::new();
        let is_main = func.name == "main";

        // Reject capability parameters — the WASM subset has no IO model.
        for p in &func.params {
            let resolved = p.param_type.resolve_alias(aliases);
            match resolved {
                Type::NetCap | Type::FsCap | Type::SysCap | Type::EnvCap => {
                    bail!(
                        "WASM backend does not support capability parameters in '{}' (parameter '{}'). \
                         Use `ooda run` for programs that exercise capability IO.",
                        func.name,
                        p.name
                    );
                }
                Type::String => {}
                Type::Option(_) | Type::Result(_, _) => bail!(
                    "WASM backend does not yet support Option/Result in '{}'. Use `ooda run`.",
                    func.name
                ),
                Type::List(inner) => {
                    Self::require_list_supported(inner.as_ref(), &func.name)?;
                }
                Type::Struct { .. } => {}
                Type::Float | Type::Int | Type::Bool | Type::Void => {}
                Type::Custom(_) => {}
            }
        }
        if let Type::List(inner) = &func.return_type {
            Self::require_list_supported(inner.as_ref(), &func.name)?;
        }
        // String returns are allowed (i32 data offset — not a full string object).

        // Helper: OODA `Type` → wasm primitive type name.
        let wat_param_ty = |t: &Type| -> Result<&'static str> {
            let resolved = t.resolve_alias(aliases);
            Ok(match resolved {
                Type::Int | Type::Bool => "i64",
                Type::String => "i32",
                Type::Float => "f64",
                Type::Void => "i64",
                Type::List(inner) => {
                    Self::require_list_supported(inner.as_ref(), "param")?;
                    if Self::is_type_string(inner.as_ref()) {
                        "list_str"
                    } else {
                        "list" // semantic tag; WAT storage is i32 pointer
                    }
                }
                Type::Struct { .. } => "i32",
                Type::Custom(_) => "i32",
                _ => bail!("unsupported param type in WASM: {:?}", t),
            })
        };

        // Header
        f_wat.push_str(&format!("  (func ${}", func.name));
        if is_main {
            f_wat.push_str(" (export \"main\")");
        }
        for param in &func.params {
            let ty = wat_param_ty(&param.param_type)?;
            f_wat.push_str(&format!(
                " (param ${} {})",
                param.name,
                Self::wat_storage_ty(ty)
            ));
        }
        // The function's return type. `main` is special: the wasm
        // host entry-point returns i32, so we wrap i64 → i32 at the
        // end (and let f64 fall through unchanged).
        let ret_ty: &'static str = match &func.return_type {
            Type::Int | Type::Bool => "i64",
            Type::String => "i32",
            Type::Float => "f64",
            Type::Void => "i64",
            Type::List(_) => "list",
            Type::Custom(name) => Box::leak(name.clone().into_boxed_str()),
            Type::Struct { .. } => "i32",
            _ => "i64",
        };
        if is_main {
            f_wat.push_str(" (result i32)\n");
        } else {
            f_wat.push_str(&format!(" (result {})\n", Self::wat_storage_ty(ret_ty)));
        }

        // Collect locals including nested while/if (for-list desugar binds loop vars inside while).
        let mut locals: BTreeMap<String, &'static str> = BTreeMap::new();
        for p in &func.params {
            locals.insert(p.name.clone(), wat_param_ty(&p.param_type)?);
        }
        Self::collect_locals_in_block(&func.body, &mut locals);
        for (name, ty) in &locals {
            // Params already appear as (param …); re-declaring as local is invalid.
            if func.params.iter().any(|p| p.name == *name) {
                continue;
            }
            f_wat.push_str(&format!(
                "    (local ${} {})\n",
                name,
                Self::wat_storage_ty(ty)
            ));
        }

        // Walk statements, emitting body
        let mut emitted_return = false;
        for stmt in &func.body.stmts {
            match stmt {
                Statement::Let { name, init, .. } => {
                    let e_wat = Self::emit_expr(init, &locals)?;
                    f_wat.push_str(&e_wat);
                    f_wat.push_str(&format!("    local.set ${}\n", name));
                }
                Statement::FieldAssign { object, field, value, .. } => {
                    let recv_ty = Self::infer_expr_type(object, &locals);
                    let mut offset = None;
                    
                    WASM_STRUCTS.with(|s| {
                        if let Some(fields) = s.borrow().get(recv_ty) {
                            if let Some(idx) = fields.iter().position(|f| f == field) {
                                offset = Some(idx * 8);
                            }
                        }
                    });
                    
                    if let Some(off) = offset {
                        f_wat.push_str(&Self::emit_expr(object, &locals)?);
                        f_wat.push_str(&Self::emit_expr(value, &locals)?);
                        let ty = Self::infer_expr_type(value, &locals);
                        if ty != "i64" && ty != "f64" {
                            f_wat.push_str("    i64.extend_i32_u\n");
                        } else if ty == "f64" {
                            f_wat.push_str("    i64.reinterpret_f64\n");
                        }
                        f_wat.push_str(&format!("    i64.store offset={}\n", off));
                    } else {
                        bail!(
                            "WASM backend could not find field '{}' on type '{}'",
                            field, recv_ty
                        );
                    }
                }
                Statement::Assign { name, value, .. } => {
                    if !locals.contains_key(name) {
                        // Allow assign to params (also addressable as locals in WASM)
                    }
                    let e_wat = Self::emit_expr(value, &locals)?;
                    f_wat.push_str(&e_wat);
                    f_wat.push_str(&format!("    local.set ${}\n", name));
                }
                Statement::Return(Some(expr), _) => {
                    let e_wat = Self::emit_expr(expr, &locals)?;
                    f_wat.push_str(&e_wat);
                    if is_main {
                        f_wat.push_str("    i32.wrap_i64\n");
                    }
                    f_wat.push_str("    return\n");
                    emitted_return = true;
                }
                Statement::Return(None, _) => {
                    if is_main {
                        f_wat.push_str("    i32.const 0\n");
                        f_wat.push_str("    return\n");
                    } else {
                        f_wat.push_str("    return\n");
                    }
                    emitted_return = true;
                }
                other => {
                    // Let / assign / expr / while / break / continue
                    f_wat.push_str(&Self::emit_stmt_wat(other, &locals)?);
                }
            }
        }

        // Tail expression (if any)
        if let Some(body_expr) = &func.body.expr {
            let e_wat = Self::emit_expr(body_expr, &locals)?;
            f_wat.push_str(&e_wat);
            if is_main {
                f_wat.push_str("    i32.wrap_i64\n");
            }
            f_wat.push_str("    return\n");
            emitted_return = true;
        }

        if !emitted_return {
            if is_main {
                f_wat.push_str("    i32.const 0\n    return\n");
            } else {
                f_wat.push_str("    return\n");
            }
        }

        f_wat.push_str("  )\n");
        Ok(f_wat)
    }

}
