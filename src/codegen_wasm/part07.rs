impl WasmCodeGen {

    fn emit_expr_call(expr: &Expression, locals: &BTreeMap<String, &'static str>) -> Result<String> {
        match expr {
            Expression::Call { name, args, .. } => {
                let mut wat = String::new();
                // List methods on List[Int] → list_*; String methods → WAT/host.
                if name == ".push"
                    || name == ".len"
                    || name == ".char_at"
                    || name == ".contains"
                    || name == ".str_slice"
                {
                    if args.is_empty() {
                        bail!("WASM method '{}' requires a receiver", name);
                    }
                    let recv_ty = Self::infer_expr_type(&args[0], locals);
                    if name == ".push" {
                        if recv_ty != "list" && recv_ty != "list_str" {
                            bail!("WASM .push requires List receiver");
                        }
                        if args.len() != 2 {
                            bail!("WASM .push expects receiver + one element");
                        }
                        let elem_ty = Self::infer_expr_type(&args[1], locals);
                        wat.push_str(&Self::emit_expr(&args[0], locals)?);
                        wat.push_str(&Self::emit_expr(&args[1], locals)?);
                        // list_str always stores i32 string ptrs as i64 slots; untyped `list`
                        // that typecheck refined to String also extends (dual-engine honesty).
                        if (recv_ty == "list_str" || recv_ty == "list") && elem_ty == "i32" {
                            wat.push_str("    i64.extend_i32_u\n");
                        } else if recv_ty == "list" && elem_ty == "i64" {
                            // List[Int]
                        } else {
                            bail!(
                                "WASM .push type mismatch (recv {}, elem {}); List[Int] needs Int, List[String] needs String.",
                                recv_ty,
                                elem_ty
                            );
                        }
                        wat.push_str("    call $list_push\n");
                    } else if name == ".len" {
                        if args.len() != 1 {
                            bail!("WASM .len expects only a receiver");
                        }
                        // List[Int] and List[String] share header layout — same $list_len (zero-cost).
                        if recv_ty == "list" || recv_ty == "list_str" {
                            wat.push_str(&Self::emit_expr(&args[0], locals)?);
                            wat.push_str("    call $list_len\n");
                        } else if recv_ty == "i32" {
                            wat.push_str(&Self::emit_string_len(&args[0], locals)?);
                        } else {
                            bail!(
                                "WASM .len requires List[Int], List[String], or String receiver (got {}); use `ooda run`.",
                                recv_ty
                            );
                        }
                    } else if name == ".char_at" {
                        // .char_at(index) on String → i64 byte value (ASCII subset)
                        if recv_ty != "i32" {
                            bail!(
                                "WASM .char_at requires String receiver (got {}); use `ooda run`.",
                                recv_ty
                            );
                        }
                        if args.len() != 2 {
                            bail!("WASM .char_at expects receiver + Int index");
                        }
                        let idx_ty = Self::infer_expr_type(&args[1], locals);
                        if idx_ty != "i64" {
                            bail!("WASM .char_at index must be Int (got {})", idx_ty);
                        }
                        wat.push_str(&Self::emit_expr(&args[0], locals)?);
                        wat.push_str(&Self::emit_expr(&args[1], locals)?);
                        wat.push_str("    i32.wrap_i64\n");
                        wat.push_str("    i32.add\n");
                        wat.push_str("    i32.load8_u\n");
                        wat.push_str("    i64.extend_i32_u\n");
                    } else if name == ".contains" {
                        // .contains(needle) on String → host str_contains → Bool i64
                        if recv_ty != "i32" {
                            bail!(
                                "WASM .contains requires String receiver (got {}); use `ooda run`.",
                                recv_ty
                            );
                        }
                        if args.len() != 2 {
                            bail!("WASM .contains expects receiver + String needle");
                        }
                        let needle_ty = Self::infer_expr_type(&args[1], locals);
                        if needle_ty != "i32" {
                            bail!("WASM .contains needle must be String (got {})", needle_ty);
                        }
                        wat.push_str(&Self::emit_expr(&args[0], locals)?);
                        wat.push_str(&Self::emit_expr(&args[1], locals)?);
                        wat.push_str("    call $str_contains\n");
                        wat.push_str("    i64.extend_i32_u\n");
                    } else {
                        // .str_slice(start, end) exclusive end → new NUL string on $heap
                        if recv_ty != "i32" {
                            bail!(
                                "WASM .str_slice requires String receiver (got {}); use `ooda run`.",
                                recv_ty
                            );
                        }
                        if args.len() != 3 {
                            bail!("WASM .str_slice expects receiver + start + end Ints");
                        }
                        if Self::infer_expr_type(&args[1], locals) != "i64"
                            || Self::infer_expr_type(&args[2], locals) != "i64"
                        {
                            bail!("WASM .str_slice start/end must be Int");
                        }
                        wat.push_str(&Self::emit_str_slice(
                            &args[0], &args[1], &args[2], locals,
                        )?);
                    }
                } else if name.starts_with('.') {
                    if args.len() != 1 {
                        bail!("Field access '{}' expects exactly one argument (the receiver)", name);
                    }
                    let field_name = &name[1..];
                    let recv_ty = Self::infer_expr_type(&args[0], locals);
                    let mut offset = None;
                    
                    WASM_STRUCTS.with(|s| {
                        if let Some(fields) = s.borrow().get(recv_ty) {
                            if let Some(idx) = fields.iter().position(|f| f == field_name) {
                                offset = Some(idx * 8);
                            }
                        }
                    });
                    
                    if let Some(off) = offset {
                        wat.push_str(&Self::emit_expr(&args[0], locals)?);
                        wat.push_str(&format!("    i64.load offset={}\n", off));
                    } else {
                        bail!(
                            "WASM backend could not find field '{}' on type '{}'",
                            field_name, recv_ty
                        );
                    }
                } else if name == "println" {
                    if args.is_empty() {
                        bail!("WASM println requires at least one Int or String argument");
                    }
                    for arg in args {
                        // Int → $println (i64); String (i32 offset) → $println_str; Float truncates.
                        let arg_ty = Self::infer_expr_type(arg, locals);
                        wat.push_str(&Self::emit_expr(arg, locals)?);
                        if arg_ty == "f64" {
                            wat.push_str("    i64.trunc_f64_s\n");
                            wat.push_str("    call $println\n");
                        } else if arg_ty == "i32" {
                            wat.push_str("    call $println_str\n");
                        } else if arg_ty == "list" {
                            bail!("WASM println cannot print List; use list_get/list_len");
                        } else {
                            wat.push_str("    call $println\n");
                        }
                    }
                } else {
                    let mut push_extend_str = false;
                    let mut get_wrap_str = false;
                    if name == "list_push" && args.len() >= 2 {
                        let recv_ty = Self::infer_expr_type(&args[0], locals);
                        let elem_ty = Self::infer_expr_type(&args[1], locals);
                        if (recv_ty == "list_str" || recv_ty == "list") && elem_ty == "i32" {
                            push_extend_str = true;
                        } else if recv_ty == "list" && elem_ty == "i64" {
                            // List[Int]
                        } else {
                            bail!(
                                "WASM list_push type mismatch (recv {}, elem {}); \
                                 List[Int] needs Int elements, List[String] needs String.",
                                recv_ty,
                                elem_ty
                            );
                        }
                    } else if name == "list_get" && !args.is_empty() {
                        let recv_ty = Self::infer_expr_type(&args[0], locals);
                        if recv_ty == "list_str" {
                            get_wrap_str = true;
                        }
                    }
                    for (i, arg) in args.iter().enumerate() {
                        wat.push_str(&Self::emit_expr(arg, locals)?);
                        if name == "list_push" && push_extend_str && i == 1 {
                            wat.push_str("    i64.extend_i32_u\n");
                        }
                    }
                    wat.push_str(&format!("    call ${}\n", name));
                    if name == "list_get" && get_wrap_str {
                        wat.push_str("    i32.wrap_i64\n");
                    }
                }
            
                Ok(wat)
            }
            _ => unreachable!("emit_expr_call"),
        }
    }

}
