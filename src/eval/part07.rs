impl Interpreter {

    fn eval_expr(&mut self, expr: &Expression, env: &mut HashMap<String, Value>) -> Result<Value> {
        match expr {
            Expression::Literal(Literal::Int(n), _) => Ok(Value::Int(*n)),
            Expression::Literal(Literal::Float(f), _) => Ok(Value::Float(*f)),
            Expression::Literal(Literal::String(s), _) => Ok(Value::String(s.clone())),
            Expression::Literal(Literal::Bool(b), _) => Ok(Value::Bool(*b)),
            Expression::Literal(Literal::Void, _) => Ok(Value::Void),
            Expression::Variable(name, _) => {
                env.get(name).cloned()
                    .or_else(|| self.globals.get(name).cloned())
                    .ok_or_else(|| anyhow!("Undefined variable '{}'", name))
            }
            Expression::Binary { op, left, right, .. } => {
                let l_val = self.eval_expr(left, env)?;
                let r_val = self.eval_expr(right, env)?;
                self.eval_binary_op(op, l_val, r_val)
            }
            Expression::Call { name, args, propagate_err, span, .. } => {
                if name == "old" {
                    if let Some(arg) = args.first() {
                        if let Expression::Variable(ref var_name, _) = arg {
                            let key = format!("old({})", var_name);
                            if let Some(val) = env.get(&key) {
                                return Ok(val.clone());
                            }
                        }
                    }
                }
                self.last_call_span = *span;
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.eval_expr(arg, env)?);
                }
                let res = self.call_function(name, arg_vals, env)?;

                if *propagate_err {
                    match res {
                        // Early-return Err from the enclosing function (try semantics).
                        Value::Err(e) => {
                            self.pending_return = Some(Value::Err(e.clone()));
                            Ok(Value::Err(e))
                        }
                        Value::Ok(v) => Ok(*v),
                        other => Err(anyhow!(
                            "Runtime error: `?` requires Result, found {:?}",
                            other
                        )),
                    }
                } else {
                    Ok(res)
                }
            }
            Expression::Unary { op, expr, .. } => {
                let v = self.eval_expr(expr, env)?;
                match (op, v) {
                    (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (UnaryOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
                    (UnaryOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                    (op, other) => Err(anyhow!("Invalid unary {:?} on {:?}", op, other)),
                }
            }
            Expression::While { cond, body, .. } => {
                loop {
                    if self.pending_return.is_some() {
                        break;
                    }
                    let c = self.eval_expr(cond, env)?;
                    if c != Value::Bool(true) {
                        break;
                    }
                    self.eval_block(body, env)?;
                    if self.pending_return.is_some() {
                        break;
                    }
                    if self.pending_break {
                        self.pending_break = false;
                        break;
                    }
                    if self.pending_continue {
                        self.pending_continue = false;
                        continue;
                    }
                }
                Ok(Value::Void)
            }
            Expression::If { cond, then_branch, else_branch, .. } => {
                let cond_val = self.eval_expr(cond, env)?;
                if cond_val == Value::Bool(true) {
                    self.eval_block(then_branch, env)
                } else if let Some(else_b) = else_branch {
                    self.eval_block(else_b, env)
                } else {
                    Ok(Value::Void)
                }
                // pending_return (if any) is inspected by eval_block / call_function
            }
            Expression::StructLit { name, fields, .. } => {
                let def = self.struct_defs.get(name).cloned().ok_or_else(|| {
                    anyhow!(
                        "Unknown struct type '{}' (declare with `type {} = struct {{ ... }};`)",
                        name,
                        name
                    )
                })?;
                let mut map = HashMap::new();
                for (fname, fexpr) in fields {
                    if !def.iter().any(|(n, _)| n == fname) {
                        return Err(anyhow!(
                            "Struct '{}' has no field '{}'",
                            name,
                            fname
                        ));
                    }
                    map.insert(fname.clone(), self.eval_expr(fexpr, env)?);
                }
                // Fill missing fields? Require all fields for honesty.
                for (fname, _) in &def {
                    if !map.contains_key(fname) {
                        return Err(anyhow!(
                            "Struct literal '{}' missing field '{}'",
                            name,
                            fname
                        ));
                    }
                }
                Ok(Value::Record {
                    type_name: name.clone(),
                    fields: map,
                })
            }
            Expression::Match { expr, arms, .. } => {
                let target = self.eval_expr(expr, env)?;
                for arm in arms {
                    // Bind pattern vars into the shared env with shadow restore so
                    // outer `let mut` assigns inside arm bodies (e.g. if-expr) persist.
                    let mut pattern_shadows: Vec<(String, Option<Value>)> = Vec::new();
                    if self.bind_pattern_shadow(&arm.pattern, &target, env, &mut pattern_shadows) {
                        let result = self.eval_expr(&arm.body, env);
                        for (name, old) in pattern_shadows.into_iter().rev() {
                            match old {
                                Some(v) => {
                                    env.insert(name, v);
                                }
                                None => {
                                    env.remove(&name);
                                }
                            }
                        }
                        return result;
                    }
                }
                Err(anyhow!("Exhaustive match failure: no pattern matched {:?}", target))
            }
        }
    }


    fn eval_binary_op(&self, op: &BinOp, left: Value, right: Value) -> Result<Value> {
        match (op, left, right) {
            (BinOp::Add, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (BinOp::Sub, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (BinOp::Mul, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (BinOp::Div, Value::Int(a), Value::Int(b)) => {
                if b == 0 {
                    Err(anyhow!("Runtime error: integer division by zero"))
                } else {
                    Ok(Value::Int(a / b))
                }
            }
            (BinOp::Add, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (BinOp::Sub, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (BinOp::Mul, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (BinOp::Div, Value::Float(a), Value::Float(b)) => {
                if b == 0.0 {
                    Err(anyhow!("Runtime error: floating-point division by zero"))
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            (BinOp::Add, Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
            (BinOp::Eq, a, b) => Ok(Value::Bool(a == b)),
            (BinOp::Neq, a, b) => Ok(Value::Bool(a != b)),
            (BinOp::Gt, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
            (BinOp::Gt, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
            (BinOp::Lt, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
            (BinOp::Lt, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
            (BinOp::Gte, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
            (BinOp::Gte, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),
            (BinOp::Lte, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (BinOp::Lte, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
            (BinOp::And, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a && b)),
            (BinOp::Or, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a || b)),
            (BinOp::DotDot, Value::Int(_), Value::Int(_)) => Ok(Value::Bool(true)),
            (BinOp::DotDotEq, Value::Int(_), Value::Int(_)) => Ok(Value::Bool(true)),
            (op, l, r) => Err(anyhow!("Invalid binary operation {:?} on {:?} and {:?}", op, l, r)),
        }
    }


    /// Bind pattern variables into `env`, recording prior values in `shadows` for restore.
    fn bind_pattern_shadow(
        &self,
        pattern: &Pattern,
        val: &Value,
        env: &mut HashMap<String, Value>,
        shadows: &mut Vec<(String, Option<Value>)>,
    ) -> bool {
        let shadow_insert = |env: &mut HashMap<String, Value>,
                             shadows: &mut Vec<(String, Option<Value>)>,
                             name: &str,
                             v: Value| {
            if !shadows.iter().any(|(n, _)| n == name) {
                shadows.push((name.to_string(), env.get(name).cloned()));
            }
            env.insert(name.to_string(), v);
        };
        match (pattern, val) {
            (Pattern::Wildcard, _) => true,
            (Pattern::Literal(Literal::Int(p)), Value::Int(v)) => p == v,
            (Pattern::Literal(Literal::String(p)), Value::String(v)) => p == v,
            (Pattern::Literal(Literal::Bool(p)), Value::Bool(v)) => p == v,
            (Pattern::Variant { name, arg }, Value::Ok(inner)) if name == "Ok" => {
                if let Some(var_name) = arg {
                    shadow_insert(env, shadows, var_name, *inner.clone());
                }
                true
            }
            (Pattern::Variant { name, arg }, Value::Err(inner)) if name == "Err" => {
                if let Some(var_name) = arg {
                    shadow_insert(env, shadows, var_name, *inner.clone());
                }
                true
            }
            (Pattern::Variant { name, arg }, Value::Some(inner)) if name == "Some" => {
                if let Some(var_name) = arg {
                    shadow_insert(env, shadows, var_name, *inner.clone());
                }
                true
            }
            (Pattern::Variant { name, arg: _ }, Value::None) if name == "None" => true,
            _ => false,
        }
    }

}
