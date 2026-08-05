impl Interpreter {

    pub fn call_function(&mut self, name: &str, args: Vec<Value>, _caller_env: &mut HashMap<String, Value>) -> Result<Value> {
        // ------------------------------------------------------------------
        // Runtime capability gate (default-deny at the point of action).
        // Even if a caller bypasses the static CapabilityChecker, the
        // interpreter refuses to invoke a sealed effectful builtin unless
        // the enclosing function declared the required capability token in
        // its parameter list.
        // ------------------------------------------------------------------
        if let Some(effect) = lookup_effect(name) {
            let caller = self
                .current_func
                .clone()
                .unwrap_or_else(|| "<top-level>".to_string());
            let caps = self
                .func_caps
                .get(&caller)
                .copied()
                .unwrap_or_default();
            if !caps.has(effect.requires) {
                return Err(anyhow!(
                    "Runtime Security Capability Violation: function '{}' invoked sealed effect '{}' without holding the required {} token. Default-deny enforced at runtime.",
                    caller,
                    name,
                    effect.requires.type_name()
                ));
            }
            // Object-capability at runtime: free sealed ops need a live handle
            // Value in the arg list; method-style needs receiver Capability.
            // Ambient declaration alone is not enough (matches static checker).
            let kind_name = effect.requires.type_name().trim_start_matches('&');
            let arg_is_live_handle = |v: &Value| match v {
                Value::Capability(c) => c == kind_name || c.ends_with(kind_name),
                _ => false,
            };
            if effect.receiver_is_cap {
                match args.first() {
                    Some(v) if arg_is_live_handle(v) => {}
                    _ => {
                        return Err(anyhow!(
                            "Runtime Security Capability Violation: function '{}' invoked method-style sealed '{}' without a live {} receiver handle (object-capability).",
                            caller,
                            name,
                            effect.requires.type_name()
                        ));
                    }
                }
            } else if !args.iter().any(arg_is_live_handle) {
                return Err(anyhow!(
                    "Runtime Security Capability Violation: function '{}' invoked sealed '{}' without passing a live {} handle argument (object-capability: ambient grant alone is not enough).",
                    caller,
                    name,
                    effect.requires.type_name()
                ));
            }
        }

        // Built-in functions
        if name == "println" || name == "read_file" || name == ".read_file" || name == "fs_read" || name == "write_file" || name == ".write_file" || name == "fs_write" || name == "env_get" || name == ".env_get" || name == "mkdir_p" || name == "copy_file" || name == "chmod_exec" || name == "path_exists" || name == "fetch" || name == "http_get" || name == "net_get" || name == "downloadData" || name == ".get" || name == "http_download" || name == "extract_tar_gz" {
            return self.call_builtins_0(name, &args);
        }
        if name == "sys_exec" || name == "exec" || name == "host_ast_dump" || name == "host_check" || name == "host_token_dump" || name == "chs_build" || name == "process_exit" || name == "list_new" || name == "list_push" || name == ".push" || name == "list_get" || name == "list_len" || name == "chars_len" || name == "char_at" || name == ".char_at" {
            return self.call_builtins_1(name, &args);
        }
        if name == "str_slice" || name == ".str_slice" || name == "char_is_digit" || name == "char_is_alpha" || name == "char_is_space" || name == ".len" || name == ".contains" || name == ".to_string" || name == ".trim" || name == ".is_ok" || name == ".is_err" || name == ".to_lowercase" || name == "assert_eq" || name == "assert_is_err" || name == "json_parse_internal" || name == "json_stringify_internal" || name == "crypto_sha256_internal" || name == "crypto_hmac_sha256_internal" || name == "async_spawn_internal" || name == "async_join_internal" {
            return self.call_builtins_2(name, &args);
        }
        if name == "python_embed_internal" || name == "Ok" || name == "Err" || name == "Some" || name == "None" || name == ".is_some" || name == ".is_none" {
            return self.call_builtins_3(name, &args);
        }
        if name.starts_with('.') && args.len() == 1 {
            // Field access on records: `tok.kind` parses as Call(".kind", [tok]).
            if let Some(Value::Record { type_name, fields }) = args.get(0) {
                let field = &name[1..];
                if let Some(v) = fields.get(field) {
                    return Ok(v.clone());
                }
                return Err(anyhow!(
                    "Record '{}' has no field '{}'",
                    type_name,
                    field
                ));
            }
        }

        let func = self.functions.get(name).cloned()
            .ok_or_else(|| anyhow!("Undefined function '{}'", name))?;

        if func.params.len() != args.len() {
            return Err(anyhow!("Function '{}' expects {} arguments, received {}", name, func.params.len(), args.len()));
        }

        // Runtime refinement: Int[lo..hi] params, including `type Port = Int[lo..hi]`.
        for (param, arg) in func.params.iter().zip(args.iter()) {
            let bounds = crate::typecheck::int_refinement_bounds(&param.param_type).or_else(|| {
                if let Type::Custom(alias) = &param.param_type {
                    self.alias_refinements.get(alias).copied()
                } else {
                    None
                }
            });
            if let Some((lo, hi)) = bounds {
                if let Value::Int(v) = arg {
                    if *v < lo || *v > hi {
                        return Err(anyhow!(
                            "RefinementTypeViolation: parameter '{}' value {} out of refinement bounds [{}..{}] for function '{}' (call site at {}:{})",
                            param.name,
                            v,
                            lo,
                            hi,
                            name,
                            self.last_call_span.line,
                            self.last_call_span.col
                        ));
                    }
                }
            }
        }

        let mut local_env = HashMap::new();
        for (param, arg) in func.params.iter().zip(args.into_iter()) {
            local_env.insert(param.name.clone(), arg);
        }

        // 1. Evaluate Preconditions (requires)
        for pre in &func.requires {
            let res = self.eval_expr(pre, &mut local_env)?;
            if res != Value::Bool(true) {
                return Err(anyhow!(
                    "Precondition Violation: 'requires' contract failed for function '{}' (call site at {}:{})",
                    name,
                    self.last_call_span.line,
                    self.last_call_span.col
                ));
            }
        }

        // Snapshot initial parameter values for old(param) postconditions
        // — but ONLY if any postcondition in this function (or its
        // verify block) actually calls `old(x)`. Skipping the snapshot
        // for the common case saves an E-M-sized HashMap allocation
        // per call: fewer bytes touched (W), less work (D), same
        // outcome.
        let uses_old = func.uses_old_state();
        let mut old_snapshot: HashMap<String, Value> = HashMap::new();
        if uses_old {
            for (k, v) in &local_env {
                old_snapshot.insert(format!("old({})", k), v.clone());
            }
        }

        // 2. Evaluate Function Body
        let prev_func = self.current_func.take();
        self.current_func = Some(name.to_string());
        let prev_ret = self.pending_return.take();
        let body_result = self.eval_block(&func.body, &mut local_env);
        let early = self.pending_return.take();
        self.pending_return = prev_ret;
        self.current_func = prev_func;
        let return_val = if let Some(v) = early {
            v
        } else {
            body_result?
        };

        // Return-type Int[lo..hi] (bare or type alias) — fail closed for non-const paths.
        if let Some((lo, hi)) = crate::typecheck::int_refinement_bounds(&func.return_type).or_else(
            || {
                if let Type::Custom(alias) = &func.return_type {
                    self.alias_refinements.get(alias).copied()
                } else {
                    None
                }
            },
        ) {
            if let Value::Int(v) = &return_val {
                if *v < lo || *v > hi {
                    return Err(anyhow!(
                        "RefinementTypeViolation: return value {} out of refinement bounds [{}..{}] for function '{}' (call site at {}:{})",
                        v,
                        lo,
                        hi,
                        name,
                        self.last_call_span.line,
                        self.last_call_span.col
                    ));
                }
            }
        }

        // 3. Evaluate Postconditions (ensures)
        if !func.ensures.is_empty() {
            let mut post_env = local_env.clone();
            post_env.extend(old_snapshot);
            post_env.insert("result".to_string(), return_val.clone());
            for post in &func.ensures {
                let res = self.eval_expr(post, &mut post_env)?;
                if res != Value::Bool(true) {
                    return Err(anyhow!(
                        "Postcondition Violation: 'ensures' contract failed for function '{}' (call site at {}:{})",
                        name,
                        self.last_call_span.line,
                        self.last_call_span.col
                    ));
                }
            }
        }

        Ok(return_val)
    }

}
