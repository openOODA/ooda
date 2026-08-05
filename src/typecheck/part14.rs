impl TypeChecker {

    fn infer_call_specials_1(
        &self,
        name: &str,
        args: &[Expression],
        arg_tys: &[Ty],
        env: &HashMap<String, Ty>,
        expr: &Expression,
    ) -> Result<Ty> {
        if name == "str_slice" {
            if arg_tys.len() != 3 {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'str_slice' expects 3 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            if !Ty::unifyable_or_unknown_hole(&arg_tys[0], &Ty::String) {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'str_slice' argument 0 expects String, found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys[0].display()
                ));
            }
            for (i, expect) in [(1, "start"), (2, "end")] {
                if !Ty::unifyable_or_unknown_hole(&arg_tys[i], &Ty::Int) {
                    return Err(anyhow!(
                        "Type error at {}:{}: function 'str_slice' argument {} ({}) expects Int, found {}",
                        expr.span().line,
                        expr.span().col,
                        i,
                        expect,
                        arg_tys[i].display()
                    ));
                }
            }
            if let (Some(s), Some(start), Some(end)) = (
                Ty::const_str(&args[0]),
                Ty::const_int(&args[1]),
                Ty::const_int(&args[2]),
            ) {
                let len = s.chars().count() as i64;
                if start < 0 || end < 0 || start > end || end > len {
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
            return Ok(Ty::String);
        }

        // ADT constructors: payload-driven Result/Option (cuts match-arm Unknown vs Int).
        if name == "Ok" {
            if arg_tys.len() != 1 {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'Ok' expects 1 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            return Ok(Ty::Result(
                Box::new(arg_tys[0].clone()),
                Box::new(Ty::Unknown),
            ));
        }
        if name == "Err" {
            if arg_tys.len() != 1 {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'Err' expects 1 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            return Ok(Ty::Result(
                Box::new(Ty::Unknown),
                Box::new(arg_tys[0].clone()),
            ));
        }
        if name == "Some" {
            if arg_tys.len() != 1 {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'Some' expects 1 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            return Ok(Ty::Option(Box::new(arg_tys[0].clone())));
        }

        // assert_eq(a, b): require comparable types (no soft Unknown-only).
        if name == "assert_eq" {
            if arg_tys.len() != 2 {
                return Err(anyhow!(
                    "Type error at {}:{}: function 'assert_eq' expects 2 argument(s), found {}",
                    expr.span().line,
                    expr.span().col,
                    arg_tys.len()
                ));
            }
            let (a, b) = (&arg_tys[0], &arg_tys[1]);
            if !self.unify(a, b)
                && !(matches!(a, Ty::Unknown) && matches!(b, Ty::Unknown))
            {
                return Err(anyhow!(
                    "Type error at {}:{}: assert_eq arguments must have matching types, found {} and {}",
                    expr.span().line,
                    expr.span().col,
                    a.display(),
                    b.display()
                ));
            }
            return Ok(Ty::Void);
        }

        // sys_exec/exec: varargs (optional cap handle + cmd + argv strings).
        if name == "sys_exec" || name == "exec" || name == "spawn_process" {
            if arg_tys.is_empty() {
                return Err(anyhow!(
                    "Type error at {}:{}: function '{}' expects at least 1 argument(s), found 0",
                    expr.span().line,
                    expr.span().col,
                    name
                ));
            }
            return Ok(Ty::Result(
                Box::new(Ty::String),
                Box::new(Ty::String),
            ));
        }

        unreachable!("specials 1: {}", name);
    }

}
