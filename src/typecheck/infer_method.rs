impl TypeChecker {
    fn infer_method_call(
        &self,
        name: &str,
        args: &[Expression],
        env: &HashMap<String, Ty>,
        expr: &Expression,
    ) -> Result<Ty> {
            let recv = args
                .first()
                .ok_or_else(|| anyhow!("Type error: method '{}' missing receiver", name))?;
            let recv_ty = self.infer_expr(recv, env)?;
            let mut method_arg_tys = Vec::new();
            for a in args.iter().skip(1) {
                method_arg_tys.push(self.infer_expr(a, env)?);
            }
            // Object-cap method arities (including receiver). Fail-closed.
            let method_arity_ok = match name {
                ".write_file" => args.len() == 3, // recv, path, content
                ".read_file" | ".env_get" | ".get" | ".sys_exec" | ".contains" | ".path_exists" | ".file_size" => args.len() == 2, // recv, arg
                ".len" | ".trim" | ".to_lowercase" | ".to_string"
                | ".is_ok" | ".is_err" | ".is_some" | ".is_none" => args.len() == 1,
                ".char_at" => args.len() == 2, // recv, index
                ".str_slice" => args.len() == 3, // recv, start, end
                ".push" => args.len() == 2,
                _ => true, // field access / unknown handled below
            };
            if !method_arity_ok {
                let expected = match name {
                    ".write_file" => 3,
                    ".str_slice" => 3,
                    ".read_file" | ".env_get" | ".get" | ".push" | ".char_at" | ".sys_exec" | ".contains" | ".path_exists" | ".file_size" => 2,
                    _ => 1,
                };
                return Err(anyhow!(
                    "Type error at {}:{}: function '{}' expects {} argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    name,
                    expected,
                    args.len()
                ));
            }
        let method_ty = self.method_return_ty(name, recv_ty.clone(), &method_arg_tys, args, expr)?;
            return Ok(method_ty);
    }

    fn method_return_ty(
        &self,
        name: &str,
        recv_ty: Ty,
        method_arg_tys: &[Ty],
        args: &[Expression],
        expr: &Expression,
    ) -> Result<Ty> {
        if matches!(name, ".len" | ".char_at" | ".str_slice" | ".sys_exec" | ".file_size" | ".path_exists" | ".trim" | ".to_lowercase" | ".to_string" | ".contains" | ".is_ok" | ".is_err") {
            return self.method_return_ty_0(name, recv_ty, method_arg_tys, args, expr);
        }
        if matches!(name, ".is_some" | ".is_none" | ".get" | ".read_file" | ".env_get" | ".write_file" | ".push")
            || (name.starts_with('.') && args.len() == 1)
        {
            return self.method_return_ty_1(name, recv_ty, method_arg_tys, args, expr);
        }
        Err(anyhow!(
            "Type error at {}:{}: unknown method '{}' on {}",
            expr.span().line,
            expr.span().col,
            name,
            recv_ty.display()
        ))
    }


    fn method_return_ty_0(
        &self,
        name: &str,
        recv_ty: Ty,
        method_arg_tys: &[Ty],
        args: &[Expression],
        expr: &Expression,
    ) -> Result<Ty> {
        match name {
            ".len" => {
                if matches!(recv_ty, Ty::String | Ty::List(_)) {
                    Ok(Ty::Int)
                } else {
                    Err(anyhow!(
                        "Type error at {}:{}: .len() requires String or List receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        recv_ty.display()
                    ))
                }
            }
            ".char_at" => {
                if !matches!(recv_ty, Ty::String) {
                    return Err(anyhow!(
                        "Type error at {}:{}: .char_at requires String receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        recv_ty.display()
                    ));
                }
                let idx_ty = method_arg_tys.first().cloned().unwrap_or(Ty::Unknown);
                if !self.unify_or_hole(&idx_ty, &Ty::Int) {
                    return Err(anyhow!(
                        "Type error at {}:{}: .char_at index expects Int, found {}",
                        expr.span().line,
                        expr.span().col,
                        idx_ty.display()
                    ));
                }
                // Const bounds when receiver is a string literal.
                if let (Some(s), Some(idx)) = (
                    args.first().and_then(|a| Ty::const_str(a)),
                    args.get(1).and_then(|a| Ty::const_int(a)),
                ) {
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
                Ok(Ty::String)
            }
            ".str_slice" => {
                if !matches!(recv_ty, Ty::String) {
                    return Err(anyhow!(
                        "Type error at {}:{}: .str_slice requires String receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        recv_ty.display()
                    ));
                }
                for (i, expect_name) in [(0, "start"), (1, "end")] {
                    let t = method_arg_tys.get(i).cloned().unwrap_or(Ty::Unknown);
                    if !self.unify_or_hole(&t, &Ty::Int) {
                        return Err(anyhow!(
                            "Type error at {}:{}: .str_slice {} expects Int, found {}",
                            expr.span().line,
                            expr.span().col,
                            expect_name,
                            t.display()
                        ));
                    }
                }
                if let (Some(s), Some(start), Some(end)) = (
                    args.first().and_then(|a| Ty::const_str(a)),
                    args.get(1).and_then(|a| Ty::const_int(a)),
                    args.get(2).and_then(|a| Ty::const_int(a)),
                ) {
                    let len = s.chars().count() as i64;
                    if start < 0 || end < start || end > len {
                        return Err(anyhow!(
                            "Type error at {}:{}: str_slice[{}..{}] out of bounds for string literal of length {} (const bounds check)",
                            expr.span().line,
                            expr.span().col,
                            start,
                            end,
                            len
                        ));
                    }
                }
                Ok(Ty::String)
            }
            ".sys_exec" => {
                if !matches!(recv_ty, Ty::SysCap) {
                    return Err(anyhow!(
                        "Type error at {}:{}: .sys_exec requires SysCap receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        recv_ty.display()
                    ));
                }
                Ok(Ty::Int)
            }
            ".file_size" | ".path_exists" => {
                if !matches!(recv_ty, Ty::FsCap) {
                    return Err(anyhow!(
                        "Type error at {}:{}: {} requires FsCap receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        name,
                        recv_ty.display()
                    ));
                }
                if name == ".file_size" {
                    Ok(Ty::Int)
                } else {
                    Ok(Ty::Bool)
                }
            }
            ".trim" | ".to_lowercase" => {
                if !matches!(recv_ty, Ty::String) {
                    return Err(anyhow!(
                        "Type error at {}:{}: {} requires String receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        name,
                        recv_ty.display()
                    ));
                }
                Ok(Ty::String)
            }
            ".to_string" => Ok(Ty::String),
            ".contains" => {
                if !matches!(recv_ty, Ty::String) {
                    return Err(anyhow!(
                        "Type error at {}:{}: .contains requires String receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        recv_ty.display()
                    ));
                }
                Ok(Ty::Bool)
            }
            ".is_ok" | ".is_err" => {
                if !matches!(recv_ty, Ty::Result(_, _) | Ty::Unknown) {
                    return Err(anyhow!(
                        "Type error at {}:{}: {} requires Result receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        name,
                        recv_ty.display()
                    ));
                }
                Ok(Ty::Bool)
            }
            _ => unreachable!("method_return_ty_0: {}", name),
        }
    }

}
