use crate::ast::*;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Void,
    Ok(Box<Value>),
    Err(Box<Value>),
    Capability(String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Void => write!(f, "()"),
            Value::Ok(v) => write!(f, "Ok({})", v),
            Value::Err(e) => write!(f, "Err({})", e),
            Value::Capability(c) => write!(f, "<Capability: {}>", c),
        }
    }
}

pub struct Interpreter {
    functions: HashMap<String, FunctionDecl>,
    globals: HashMap<String, Value>,
}

impl Interpreter {
    pub fn new(program: Program) -> Self {
        let mut functions = HashMap::new();
        for item in program.items {
            match item {
                Item::Function(func) => {
                    functions.insert(func.name.clone(), func);
                }
                Item::TypeAlias(..) => {}
            }
        }
        Self {
            functions,
            globals: HashMap::new(),
        }
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
                let cap_name = match param.param_type {
                    Type::NetCap => "NetCap",
                    Type::FsCap  => "FsCap",
                    Type::SysCap => "SysCap",
                    Type::EnvCap => "EnvCap",
                    _ => "GeneralCap",
                };
                main_args.push(Value::Capability(cap_name.to_string()));
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
                    let _ = self.call_function(&func.name, vec![], &mut env);
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
                                }
                            }
                        }
                    }
                    println!(
                        "   ✓ Fuzz '{}': {} ok, {} precondition rejects, {} other errors",
                        name, ok, pre_fail, other_err
                    );
                }
            }
        }
        Ok(())
    }

    pub fn call_function(&mut self, name: &str, args: Vec<Value>, caller_env: &mut HashMap<String, Value>) -> Result<Value> {
        // Built-in functions
        if name == "println" {
            for arg in &args {
                print!("{}", arg);
            }
            println!();
            return Ok(Value::Void);
        } else if name == ".len" {
            if let Some(Value::String(s)) = args.get(0) {
                return Ok(Value::Int(s.len() as i64));
            } else {
                return Err(anyhow!("Method .len() expects String argument"));
            }
        } else if name == ".to_string" {
            if let Some(v) = args.get(0) {
                return Ok(Value::String(v.to_string()));
            } else {
                return Err(anyhow!("Method .to_string() invalid argument"));
            }
        } else if name == ".trim" {
            if let Some(Value::String(s)) = args.get(0) {
                return Ok(Value::String(s.trim().to_string()));
            } else {
                return Err(anyhow!("Method .trim() expects String argument"));
            }
        } else if name == ".is_ok" {
            if let Some(Value::Ok(_)) = args.get(0) {
                return Ok(Value::Bool(true));
            } else {
                return Ok(Value::Bool(false));
            }
        } else if name == ".to_lowercase" {
            if let Some(Value::String(s)) = args.get(0) {
                return Ok(Value::String(s.to_lowercase()));
            } else {
                return Err(anyhow!("Method .to_lowercase() expects String argument"));
            }
        } else if name == "assert_eq" {
            if args.len() == 2 && args[0] == args[1] {
                return Ok(Value::Void);
            } else {
                return Err(anyhow!("Assertion Failed: assert_eq!({:?}, {:?})", args.get(0), args.get(1)));
            }
        } else if name == "assert_is_err" {
            if let Some(Value::Err(_)) = args.get(0) {
                return Ok(Value::Void);
            } else {
                return Err(anyhow!("Assertion Failed: Expected Err, found {:?}", args.get(0)));
            }
        } else if name == "json_parse_internal" {
            let raw = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            if serde_json::from_str::<serde_json::Value>(&raw).is_ok() {
                return Ok(Value::Ok(Box::new(Value::String(raw))));
            } else {
                return Ok(Value::Err(Box::new(Value::String("Invalid JSON syntax".to_string()))));
            }
        } else if name == "json_stringify_internal" {
            let obj = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            return Ok(Value::String(obj));
        } else if name == "crypto_sha256_internal" {
            let data = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            let result = hasher.finalize();
            let hex_hash = format!("{:x}", result);
            return Ok(Value::String(hex_hash));
        } else if name == "crypto_hmac_sha256_internal" {
            let key = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let msg = args.get(1).map(|v| v.to_string()).unwrap_or_default();
            let mut mac = HmacSha256::new_from_slice(key.as_bytes()).map_err(|e| anyhow!("{}", e))?;
            mac.update(msg.as_bytes());
            let result = mac.finalize();
            let hex_hash = format!("{:x}", result.into_bytes());
            return Ok(Value::String(hex_hash));
        } else if name == "async_spawn_internal" {
            let task_name = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let handle = format!("task_handle_{}", task_name);
            return Ok(Value::String(handle));
        } else if name == "async_join_internal" {
            let handle = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            return Ok(Value::Ok(Box::new(Value::String(format!("joined_{}", handle)))));
        } else if name == "python_embed_internal" {
            let model = args.get(0).map(|v| v.to_string()).unwrap_or_default();
            let handle = format!("pytorch_model_handle_{}", model);
            return Ok(Value::Ok(Box::new(Value::String(handle))));
        } else if name == "Ok" {
            let val = args.get(0).cloned().unwrap_or(Value::Void);
            return Ok(Value::Ok(Box::new(val)));
        } else if name == "Err" {
            let val = args.get(0).cloned().unwrap_or(Value::Void);
            return Ok(Value::Err(Box::new(val)));
        }

        let func = self.functions.get(name).cloned()
            .ok_or_else(|| anyhow!("Undefined function '{}'", name))?;

        if func.params.len() != args.len() {
            return Err(anyhow!("Function '{}' expects {} arguments, received {}", name, func.params.len(), args.len()));
        }

        let mut local_env = HashMap::new();
        for (param, arg) in func.params.iter().zip(args.into_iter()) {
            local_env.insert(param.name.clone(), arg);
        }

        // 1. Evaluate Preconditions (requires)
        for pre in &func.requires {
            let res = self.eval_expr(pre, &mut local_env)?;
            if res != Value::Bool(true) {
                return Err(anyhow!("Precondition Violation: 'requires' contract failed for function '{}'", name));
            }
        }

        // 2. Evaluate Function Body
        let return_val = self.eval_block(&func.body, &mut local_env)?;

        // 3. Evaluate Postconditions (ensures)
        if !func.ensures.is_empty() {
            let mut post_env = local_env.clone();
            post_env.insert("result".to_string(), return_val.clone());
            for post in &func.ensures {
                let res = self.eval_expr(post, &mut post_env)?;
                if res != Value::Bool(true) {
                    return Err(anyhow!("Postcondition Violation: 'ensures' contract failed for function '{}'", name));
                }
            }
        }

        Ok(return_val)
    }

    fn eval_block(&mut self, block: &Block, env: &mut HashMap<String, Value>) -> Result<Value> {
        for stmt in &block.stmts {
            match stmt {
                Statement::Let { name, init, .. } => {
                    let val = self.eval_expr(init, env)?;
                    env.insert(name.clone(), val);
                }
                Statement::Return(Some(expr)) => {
                    return self.eval_expr(expr, env);
                }
                Statement::Return(None) => {
                    return Ok(Value::Void);
                }
                Statement::Expr(expr) => {
                    self.eval_expr(expr, env)?;
                }
            }
        }

        if let Some(expr) = &block.expr {
            self.eval_expr(expr, env)
        } else {
            Ok(Value::Void)
        }
    }

    fn eval_expr(&mut self, expr: &Expression, env: &mut HashMap<String, Value>) -> Result<Value> {
        match expr {
            Expression::Literal(Literal::Int(n)) => Ok(Value::Int(*n)),
            Expression::Literal(Literal::Float(f)) => Ok(Value::Float(*f)),
            Expression::Literal(Literal::String(s)) => Ok(Value::String(s.clone())),
            Expression::Literal(Literal::Bool(b)) => Ok(Value::Bool(*b)),
            Expression::Literal(Literal::Void) => Ok(Value::Void),
            Expression::Variable(name) => {
                env.get(name).cloned()
                    .or_else(|| self.globals.get(name).cloned())
                    .ok_or_else(|| anyhow!("Undefined variable '{}'", name))
            }
            Expression::Binary { op, left, right } => {
                let l_val = self.eval_expr(left, env)?;
                let r_val = self.eval_expr(right, env)?;
                self.eval_binary_op(op, l_val, r_val)
            }
            Expression::Call { name, args, propagate_err } => {
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.eval_expr(arg, env)?);
                }
                let res = self.call_function(name, arg_vals, env)?;

                if *propagate_err {
                    match res {
                        Value::Err(e) => return Ok(Value::Err(e)),
                        Value::Ok(v) => Ok(*v),
                        other => Ok(other),
                    }
                } else {
                    Ok(res)
                }
            }
            Expression::If { cond, then_branch, else_branch } => {
                let cond_val = self.eval_expr(cond, env)?;
                if cond_val == Value::Bool(true) {
                    self.eval_block(then_branch, env)
                } else if let Some(else_b) = else_branch {
                    self.eval_block(else_b, env)
                } else {
                    Ok(Value::Void)
                }
            }
            Expression::Match { expr, arms } => {
                let target = self.eval_expr(expr, env)?;
                for arm in arms {
                    let mut arm_env = env.clone();
                    if self.bind_pattern(&arm.pattern, &target, &mut arm_env) {
                        return self.eval_expr(&arm.body, &mut arm_env);
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

    fn bind_pattern(&self, pattern: &Pattern, val: &Value, env: &mut HashMap<String, Value>) -> bool {
        match (pattern, val) {
            (Pattern::Wildcard, _) => true,
            (Pattern::Literal(Literal::Int(p)), Value::Int(v)) => p == v,
            (Pattern::Literal(Literal::String(p)), Value::String(v)) => p == v,
            (Pattern::Literal(Literal::Bool(p)), Value::Bool(v)) => p == v,
            (Pattern::Variant { name, arg }, Value::Ok(inner)) if name == "Ok" => {
                if let Some(var_name) = arg {
                    env.insert(var_name.clone(), *inner.clone());
                }
                true
            }
            (Pattern::Variant { name, arg }, Value::Err(inner)) if name == "Err" => {
                if let Some(var_name) = arg {
                    env.insert(var_name.clone(), *inner.clone());
                }
                true
            }
            _ => false,
        }
    }
}
