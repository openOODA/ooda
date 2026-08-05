impl Interpreter {
    pub fn new(program: Program) -> Self {
        let mut functions = HashMap::new();
        let mut func_caps = HashMap::new();
        let mut struct_defs = HashMap::new();
        let mut alias_refinements = HashMap::new();
        for item in program.items {
            match item {
                Item::Function(func) => {
                    func_caps.insert(func.name.clone(), CapSet::from_params(&func));
                    functions.insert(func.name.clone(), func);
                }
                Item::TypeAlias(name, Type::Struct { fields, .. }) => {
                    struct_defs.insert(name, fields);
                }
                Item::TypeAlias(name, ty) => {
                    if let Some(b) = crate::typecheck::int_refinement_bounds(&ty) {
                        alias_refinements.insert(name, b);
                    }
                }
                Item::Import { .. } => {}
            }
        }
        Self {
            functions,
            globals: HashMap::new(),
            func_caps,
            current_func: None,
            last_call_span: Span::synthetic(),
            threads: HashMap::new(),
            next_thread_id: 1,
            argv: Vec::new(),
            struct_defs,
            alias_refinements,
            pending_return: None,
            pending_break: false,
            pending_continue: false,
        }
    }


    /// Inject host argv for CHS programs whose `main` takes `List[String]` (or named `args`).
    pub fn with_argv(mut self, argv: Vec<String>) -> Self {
        self.argv = argv;
        self
    }


    pub fn execute_all(&mut self) -> Result<()> {
        let func_names: Vec<String> = self.functions.keys().cloned().collect();

        // 1. Run all verify blocks across all functions
        for name in &func_names {
            if let Some(func) = self.functions.get(name).cloned() {
                if let Some(verify_block) = &func.verify_block {
                    println!("🧪 [Contract Verify] Testing function '{}'", name);
                    let mut env = HashMap::new();
                    self.eval_block(verify_block, &mut env)?;
                    println!("   ✓ Verify passed for '{}'", name);
                }
            }
        }

        // 2. Execute main() if present
        if let Some(main_fn) = self.functions.get("main").cloned() {
            println!("🚀 [Execution] Running main()");
            let mut main_args = Vec::new();
            for param in &main_fn.params {
                let arg = match &param.param_type {
                    Type::NetCap => Value::Capability("NetCap".into()),
                    Type::FsCap => Value::Capability("FsCap".into()),
                    Type::SysCap => Value::Capability("SysCap".into()),
                    Type::EnvCap => Value::Capability("EnvCap".into()),
                    Type::List(inner)
                        if matches!(**inner, Type::String)
                            || param.name == "args"
                            || param.name == "argv" =>
                    {
                        Value::List(
                            self.argv
                                .iter()
                                .map(|s| Value::String(s.clone()))
                                .collect(),
                        )
                    }
                    _ => Value::Capability("GeneralCap".into()),
                };
                main_args.push(arg);
            }
            self.call_function("main", main_args, &mut HashMap::new())?;
        }

        Ok(())
    }


    pub fn fuzz_all(&mut self) -> Result<()> {
        println!("🎲 [Automated Fuzzer] Stress-testing function contracts with boundary inputs...");
        let func_names: Vec<String> = self.functions.keys().cloned().collect();

        for name in &func_names {
            if let Some(func) = self.functions.get(name).cloned() {
                if func.name == "main" {
                    continue;
                }
                // Skip functions that require capability handles (no ambient caps in fuzz harness).
                let needs_cap = func.params.iter().any(|p| {
                    matches!(
                        p.param_type,
                        Type::NetCap | Type::FsCap | Type::SysCap | Type::EnvCap
                    )
                });
                if needs_cap {
                    println!(
                        "  ⏭  Skipping '{}' (requires capability parameters)",
                        name
                    );
                    continue;
                }

                println!("  🧪 Fuzzing '{}' across boundary test matrix...", name);

                let mut domains: Vec<Vec<Value>> = Vec::new();
                for param in &func.params {
                    match param.param_type {
                        Type::Int => domains.push(vec![
                            Value::Int(0),
                            Value::Int(1),
                            Value::Int(-1),
                            Value::Int(i64::MAX),
                            Value::Int(i64::MIN),
                        ]),
                        Type::Float => domains.push(vec![
                            Value::Float(0.0),
                            Value::Float(1.0),
                            Value::Float(-1.0),
                            Value::Float(f64::MAX),
                        ]),
                        Type::String => domains.push(vec![
                            Value::String(String::new()),
                            Value::String("fuzz".into()),
                            Value::String("\0".into()),
                        ]),
                        Type::Bool => {
                            domains.push(vec![Value::Bool(true), Value::Bool(false)])
                        }
                        _ => domains.push(vec![Value::Void]),
                    }
                }

                if domains.is_empty() {
                    let mut env = HashMap::new();
                    if let Err(e) = self.call_function(&func.name, vec![], &mut env) {
                        let msg = format!("{}", e);
                        if !msg.contains("Precondition Violation") {
                            return Err(anyhow!(
                                "Fuzz '{}': unexpected error on zero-arg call: {}",
                                name,
                                msg
                            ));
                        }
                    }
                } else {
                    // Cartesian product capped for multi-param functions.
                    let mut combos: Vec<Vec<Value>> = vec![vec![]];
                    for domain in &domains {
                        let mut next = Vec::new();
                        for prefix in &combos {
                            for v in domain {
                                if next.len() >= 64 {
                                    break;
                                }
                                let mut row = prefix.clone();
                                row.push(v.clone());
                                next.push(row);
                            }
                        }
                        combos = next;
                    }
                    let mut pre_fail = 0u32;
                    let mut ok = 0u32;
                    let mut other_err = 0u32;
                    let mut other_msgs: Vec<String> = Vec::new();
                    for args in combos {
                        let mut env = HashMap::new();
                        match self.call_function(&func.name, args, &mut env) {
                            Ok(_) => ok += 1,
                            Err(e) => {
                                let msg = format!("{}", e);
                                if msg.contains("Precondition Violation") {
                                    pre_fail += 1;
                                } else {
                                    other_err += 1;
                                    if other_msgs.len() < 3 {
                                        other_msgs.push(msg);
                                    }
                                }
                            }
                        }
                    }
                    // Fail closed: unexpected errors (postconditions, panics, type traps)
                    // must not soft-pass as a green fuzz report.
                    if other_err > 0 {
                        return Err(anyhow!(
                            "Fuzz '{}': {} unexpected error(s) (ok={}, precondition_rejects={}). Sample: {}",
                            name,
                            other_err,
                            ok,
                            pre_fail,
                            other_msgs.join(" | ")
                        ));
                    }
                    println!(
                        "   ✓ Fuzz '{}': {} ok, {} precondition rejects, 0 unexpected errors",
                        name, ok, pre_fail
                    );
                }
            }
        }
        Ok(())
    }

}
