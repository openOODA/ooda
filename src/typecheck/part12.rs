impl TypeChecker {

    fn method_return_ty_1(
        &self,
        name: &str,
        recv_ty: Ty,
        method_arg_tys: &[Ty],
        args: &[Expression],
        expr: &Expression,
    ) -> Result<Ty> {
        match name {
            ".is_some" | ".is_none" => {
                if !matches!(recv_ty, Ty::Option(_) | Ty::Unknown) {
                    return Err(anyhow!(
                        "Type error at {}:{}: {} requires Option receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        name,
                        recv_ty.display()
                    ));
                }
                Ok(Ty::Bool)
            }
            ".get" => {
                if !matches!(recv_ty, Ty::NetCap) {
                    return Err(anyhow!(
                        "Type error at {}:{}: .get requires NetCap receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        recv_ty.display()
                    ));
                }
                Ok(Ty::Result(Box::new(Ty::String), Box::new(Ty::String)))
            }
            ".read_file" | ".env_get" => {
                let need = if name == ".read_file" {
                    Ty::FsCap
                } else {
                    Ty::EnvCap
                };
                if recv_ty != need {
                    return Err(anyhow!(
                        "Type error at {}:{}: {} requires {} receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        name,
                        need.display(),
                        recv_ty.display()
                    ));
                }
                Ok(Ty::Result(
                    Box::new(Ty::String),
                    Box::new(Ty::String),
                ))
            }
            ".write_file" => {
                if !matches!(recv_ty, Ty::FsCap) {
                    return Err(anyhow!(
                        "Type error at {}:{}: .write_file requires FsCap receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        recv_ty.display()
                    ));
                }
                Ok(Ty::Result(
                    Box::new(Ty::Void),
                    Box::new(Ty::String),
                ))
            }
            ".push" => {
                // recv.push(elem)
                match &recv_ty {
                    Ty::List(inner) => {
                        let elem_ty =
                            method_arg_tys.first().cloned().unwrap_or(Ty::Unknown);
                        let out = if matches!(inner.as_ref(), Ty::Unknown) {
                            elem_ty
                        } else if Ty::unifyable_or_unknown_hole(inner, &elem_ty) {
                            (**inner).clone()
                        } else {
                            return Err(anyhow!(
                                "Type error at {}:{}: list element type mismatch: List[{}] cannot push {}",
                                expr.span().line,
                                expr.span().col,
                                inner.display(),
                                elem_ty.display()
                            ));
                        };
                        Ok(Ty::List(Box::new(out)))
                    }
                    other => Err(anyhow!(
                        "Type error at {}:{}: .push requires List receiver, found {}",
                        expr.span().line,
                        expr.span().col,
                        other.display()
                    )),
                }
            }
            // Field access only on struct types (fail-closed on Int/String/etc.).
            other if other.starts_with('.') && args.len() == 1 => {
                let field = &other[1..];
                match &recv_ty {
                    Ty::Struct { fields, .. } => {
                        if let Some((_, fty)) =
                            fields.iter().find(|(n, _)| n == field)
                        {
                            Ok(fty.clone())
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: struct has no field '{}'",
                                expr.span().line,
                                expr.span().col,
                                field
                            ))
                        }
                    }
                    Ty::Custom(name) => {
                        if let Some(Ty::Struct { fields, .. }) =
                            self.type_aliases.get(name)
                        {
                            if let Some((_, fty)) =
                                fields.iter().find(|(n, _)| n == field)
                            {
                                Ok(fty.clone())
                            } else {
                                Err(anyhow!(
                                    "Type error at {}:{}: struct '{}' has no field '{}'",
                                    expr.span().line,
                                    expr.span().col,
                                    name,
                                    field
                                ))
                            }
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: unknown type '{}' for field access '.{}'",
                                expr.span().line,
                                expr.span().col,
                                name,
                                field
                            ))
                        }
                    }
                    Ty::Unknown => Ok(Ty::Unknown),
                    other_ty => Err(anyhow!(
                        "Type error at {}:{}: unknown method '{}' on {}",
                        expr.span().line,
                        expr.span().col,
                        other,
                        other_ty.display()
                    )),
                }
            }
            other => Err(anyhow!(
                "Type error at {}:{}: unknown method '{}'",
                expr.span().line,
                expr.span().col,
                other
            )),
            _ => unreachable!("method_return_ty_1: {}", name),
        }
    }

}
