impl Interpreter {


    /// Assign `val` to `object.field`, where `object` is a Variable or a nested
    /// `.field` Call chain (e.g. `p.inner` desugared). Mutates the root record in env.
    fn assign_field_path(
        env: &mut HashMap<String, Value>,
        object: &Expression,
        field: &str,
        val: Value,
    ) -> Result<()> {
        // Flatten path: root var + intermediate field names + final field.
        let mut chain: Vec<String> = Vec::new();
        let mut cur = object;
        loop {
            match cur {
                Expression::Variable(name, _) => {
                    chain.insert(0, name.clone());
                    break;
                }
                Expression::Call { name, args, .. }
                    if name.starts_with('.') && args.len() == 1 =>
                {
                    chain.insert(0, name[1..].to_string());
                    cur = &args[0];
                }
                _ => {
                    return Err(anyhow!(
                        "Runtime error: field assign requires a variable or field path receiver"
                    ));
                }
            }
        }
        chain.push(field.to_string());
        // chain = [root, f1, f2, ..., final_field]
        if chain.len() < 2 {
            return Err(anyhow!("Runtime error: empty field assign path"));
        }
        let root = chain[0].clone();
        let entry = env.get_mut(&root).ok_or_else(|| {
            anyhow!("Runtime error: undefined variable '{}'", root)
        })?;
        let mut cursor: &mut Value = entry;
        for seg in &chain[1..chain.len() - 1] {
            match cursor {
                Value::Record { fields, .. } => {
                    cursor = fields.get_mut(seg).ok_or_else(|| {
                        anyhow!("Runtime error: struct has no field '{}'", seg)
                    })?;
                }
                other => {
                    return Err(anyhow!(
                        "Runtime error: field assign path through non-struct {:?}",
                        other
                    ));
                }
            }
        }
        let last = chain.last().unwrap();
        match cursor {
            Value::Record { fields, .. } => {
                if !fields.contains_key(last) {
                    return Err(anyhow!(
                        "Runtime error: struct has no field '{}'",
                        last
                    ));
                }
                fields.insert(last.clone(), val);
                Ok(())
            }
            other => Err(anyhow!(
                "Runtime error: field assign on non-struct value {:?}",
                other
            )),
        }
    }


    fn eval_block(&mut self, block: &Block, env: &mut HashMap<String, Value>) -> Result<Value> {
        // Scope: restore `let` bindings introduced in this block so nested
        // if/while cannot pollute outer frames. Assignments to outer names stick.
        let mut let_shadows: Vec<(String, Option<Value>)> = Vec::new();
        let result = self.eval_block_inner(block, env, &mut let_shadows);
        for (name, old) in let_shadows.into_iter().rev() {
            match old {
                Some(v) => {
                    env.insert(name, v);
                }
                None => {
                    env.remove(&name);
                }
            }
        }
        result
    }


    fn eval_block_inner(
        &mut self,
        block: &Block,
        env: &mut HashMap<String, Value>,
        let_shadows: &mut Vec<(String, Option<Value>)>,
    ) -> Result<Value> {
        for stmt in &block.stmts {
            if self.pending_return.is_some() {
                break;
            }
            match stmt {
                Statement::Let { name, init, .. } => {
                    let val = self.eval_expr(init, env)?;
                    // `?` may set pending_return (Err early-exit).
                    if self.pending_return.is_some() {
                        return Ok(self.pending_return.clone().unwrap_or(Value::Void));
                    }
                    if !let_shadows.iter().any(|(n, _)| n == name) {
                        let_shadows.push((name.clone(), env.get(name).cloned()));
                    }
                    env.insert(name.clone(), val);
                }
                Statement::Assign { name, value, .. } => {
                    if !env.contains_key(name) {
                        return Err(anyhow!("Runtime error: assign to undefined variable '{}'", name));
                    }
                    let val = self.eval_expr(value, env)?;
                    if self.pending_return.is_some() {
                        return Ok(self.pending_return.clone().unwrap_or(Value::Void));
                    }
                    env.insert(name.clone(), val);
                }
                Statement::FieldAssign {
                    object,
                    field,
                    value,
                    ..
                } => {
                    let val = self.eval_expr(value, env)?;
                    if self.pending_return.is_some() {
                        return Ok(self.pending_return.clone().unwrap_or(Value::Void));
                    }
                    Self::assign_field_path(env, object, field, val)?;
                }
                Statement::Return(Some(expr), _) => {
                    let v = self.eval_expr(expr, env)?;
                    self.pending_return = Some(v.clone());
                    return Ok(v);
                }
                Statement::Return(None, _) => {
                    self.pending_return = Some(Value::Void);
                    return Ok(Value::Void);
                }
                Statement::Break(_) => {
                    self.pending_break = true;
                    return Ok(Value::Void);
                }
                Statement::Continue(_) => {
                    self.pending_continue = true;
                    return Ok(Value::Void);
                }
                Statement::Expr(expr, _) => {
                    self.eval_expr(expr, env)?;
                    if self.pending_return.is_some()
                        || self.pending_break
                        || self.pending_continue
                    {
                        return Ok(self.pending_return.clone().unwrap_or(Value::Void));
                    }
                }
                Statement::While { cond, body, .. } => {
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
                }
            }
        }

        if self.pending_return.is_some() || self.pending_break || self.pending_continue {
            return Ok(self.pending_return.clone().unwrap_or(Value::Void));
        }

        if let Some(expr) = &block.expr {
            let v = self.eval_expr(expr, env)?;
            if self.pending_return.is_some() {
                return Ok(self.pending_return.clone().unwrap_or(v));
            }
            Ok(v)
        } else {
            Ok(Value::Void)
        }
    }

}
