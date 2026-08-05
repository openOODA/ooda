impl WasmCodeGen {
    fn emit_expr_binary(expr: &Expression, locals: &BTreeMap<String, &'static str>) -> Result<String> {
        match expr {
            Expression::Binary { op, left, right, .. } => {
                let mut wat = String::new();
                // Infer operand types. Prefer the declared type of a
                // local; fall back to the literal shape for fresh
                // expressions.
                let lhs_ty = Self::infer_expr_type(left, locals);
                let rhs_ty = Self::infer_expr_type(right, locals);
                let either_str = lhs_ty == "i32" || rhs_ty == "i32";
                let either_list = lhs_ty == "list" || rhs_ty == "list" || lhs_ty == "list_str" || rhs_ty == "list_str";
                // String/list pointers are not numbers: refuse arithmetic / ordering.
                // String Eq/Neq → $streq (content). List Eq/Neq → $list_eq (deep Int content).
                if either_str {
                    match op {
                        BinOp::Eq | BinOp::Neq => {
                            if lhs_ty != "i32" || rhs_ty != "i32" {
                                bail!(
                                    "WASM backend does not mix String pointers with numeric types in binary ops; use `ooda run`."
                                );
                            }
                        }
                        BinOp::Add => {
                            // String + String → bump-heap concat (pure WAT). No silent pointer math.
                            if lhs_ty != "i32" || rhs_ty != "i32" {
                                bail!(
                                    "WASM string concat requires String + String (got {} + {}); use `ooda run`.",
                                    lhs_ty,
                                    rhs_ty
                                );
                            }
                            wat.push_str(&Self::emit_str_concat(left, right, locals)?);
                            return Ok(wat);
                        }
                        BinOp::Sub | BinOp::Mul | BinOp::Div => {
                            bail!(
                                "WASM backend does not lower string arithmetic (`{:?}`); \
                                 use `ooda run` for numeric conversion (no silent pointer math).",
                                op
                            );
                        }
                        BinOp::Gt | BinOp::Lt | BinOp::Gte | BinOp::Lte => {
                            bail!(
                                "WASM backend does not lower ordered compare on String pointers; use `ooda run`."
                            );
                        }
                        BinOp::And | BinOp::Or => {
                            bail!("WASM backend does not lower &&/|| on String; use `ooda run`.");
                        }
                        BinOp::DotDot | BinOp::DotDotEq => {
                            bail!("WASM backend does not yet lower range operators (`..`, `..=`). Use `ooda run`.")
                        }
                    }
                }
                if either_list {
                    match op {
                        BinOp::Eq | BinOp::Neq => {
                            // Homogeneous only: Int lists use $list_eq (i64 content);
                            // String lists use $list_str_eq (streq per element — not pointer eq).
                            let both_int = lhs_ty == "list" && rhs_ty == "list";
                            let both_str = lhs_ty == "list_str" && rhs_ty == "list_str";
                            if !both_int && !both_str {
                                bail!(
                                    "WASM backend does not mix List kinds in ==/!= (got {} vs {}); use `ooda run`.",
                                    lhs_ty,
                                    rhs_ty
                                );
                            }
                        }
                        _ => bail!(
                            "WASM backend does not lower {:?} on List pointers; use list_get/list_len.",
                            op
                        ),
                    }
                }
                // Promote i64 → f64 if either operand is f64.
                let promote = |ty: &'static str, code: &mut String| {
                    if ty == "i64" && (lhs_ty == "f64" || rhs_ty == "f64") {
                        code.push_str("    f64.convert_i64_s\n");
                    }
                };
                let l_wat = Self::emit_expr(left, locals)?;
                let mut l_code = String::new();
                promote(lhs_ty, &mut l_code);
                wat.push_str(&l_wat);
                wat.push_str(&l_code);

                let r_wat = Self::emit_expr(right, locals)?;
                let mut r_code = String::new();
                promote(rhs_ty, &mut r_code);
                wat.push_str(&r_wat);
                wat.push_str(&r_code);

                // Both operands are now on the stack as the same type.
                let ty = if lhs_ty == "f64" || rhs_ty == "f64" {
                    "f64"
                } else if either_str || either_list {
                    "i32"
                } else {
                    "i64"
                };
                match op {
                    BinOp::Add => wat.push_str(&format!("    {}.add\n", ty)),
                    BinOp::Sub => wat.push_str(&format!("    {}.sub\n", ty)),
                    BinOp::Mul => wat.push_str(&format!("    {}.mul\n", ty)),
                    BinOp::Div if ty == "i64" => wat.push_str("    i64.div_s\n"),
                    BinOp::Div => wat.push_str("    f64.div\n"),
                    // Comparisons yield i32 in WebAssembly; extend to i64 Bool model.
                    // String: $streq content. List[Int]: $list_eq i64 elements.
                    // List[String]: $list_str_eq (streq each element — matches interpreter content eq).
                    BinOp::Eq => {
                        if either_str {
                            wat.push_str("    call $streq\n");
                        } else if either_list {
                            if lhs_ty == "list_str" {
                                wat.push_str("    call $list_str_eq\n");
                            } else {
                                wat.push_str("    call $list_eq\n");
                            }
                        } else {
                            wat.push_str(&format!("    {}.eq\n", ty));
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Neq => {
                        if either_str {
                            wat.push_str("    call $streq\n");
                            wat.push_str("    i32.eqz\n");
                        } else if either_list {
                            if lhs_ty == "list_str" {
                                wat.push_str("    call $list_str_eq\n");
                            } else {
                                wat.push_str("    call $list_eq\n");
                            }
                            wat.push_str("    i32.eqz\n");
                        } else {
                            wat.push_str(&format!("    {}.ne\n", ty));
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Gt => {
                        if ty == "i64" {
                            wat.push_str("    i64.gt_s\n")
                        } else {
                            wat.push_str("    f64.gt\n")
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Lt => {
                        if ty == "i64" {
                            wat.push_str("    i64.lt_s\n")
                        } else {
                            wat.push_str("    f64.lt\n")
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Gte => {
                        if ty == "i64" {
                            wat.push_str("    i64.ge_s\n")
                        } else {
                            wat.push_str("    f64.ge\n")
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    BinOp::Lte => {
                        if ty == "i64" {
                            wat.push_str("    i64.le_s\n")
                        } else {
                            wat.push_str("    f64.le\n")
                        }
                        wat.push_str("    i64.extend_i32_u\n");
                    }
                    // Boolean ops stay i64.
                    BinOp::And | BinOp::Or if ty == "i64" => match op {
                        BinOp::And => wat.push_str("    i64.and\n"),
                        BinOp::Or => wat.push_str("    i64.or\n"),
                        _ => unreachable!(),
                    },
                    BinOp::And | BinOp::Or => bail!(
                        "WASM backend does not yet lower {} on Float operands in this alpha.",
                        match op {
                            BinOp::And => "&&",
                            BinOp::Or => "||",
                            _ => "?"
                        }
                    ),
                    BinOp::DotDot | BinOp::DotDotEq => {
                        bail!("WASM backend does not yet lower range operators (`..`, `..=`). Use `ooda run`.")
                    }
                }
            
                Ok(wat)
            }
            _ => unreachable!("emit_expr_binary"),
        }
    }

}
