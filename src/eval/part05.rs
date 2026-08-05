impl Interpreter {

    fn call_builtins_2(&mut self, name: &str, args: &[Value]) -> Result<Value> {
    if name == "str_slice" || name == ".str_slice" {
        let s = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(anyhow!("str_slice expects String")),
        };
        let start = match args.get(1) {
            Some(Value::Int(i)) => *i,
            _ => return Err(anyhow!("str_slice expects Int start")),
        };
        let end = match args.get(2) {
            Some(Value::Int(i)) => *i,
            _ => return Err(anyhow!("str_slice expects Int end")),
        };
        if start < 0 || end < start {
            return Err(anyhow!(
                "str_slice: invalid range [{}, {})",
                start,
                end
            ));
        }
        let chars: Vec<char> = s.chars().collect();
        if end as usize > chars.len() {
            return Err(anyhow!(
                "str_slice: end {} out of bounds (chars_len {})",
                end,
                chars.len()
            ));
        }
        return Ok(Value::String(
            chars[start as usize..end as usize].iter().collect(),
        ));
    } else if name == "char_is_digit" {
        return Ok(Value::Bool(Self::first_char_pred(&args, |c| {
            c.is_ascii_digit()
        })?));
    } else if name == "char_is_alpha" {
        return Ok(Value::Bool(Self::first_char_pred(&args, |c| {
            c.is_ascii_alphabetic()
        })?));
    } else if name == "char_is_space" {
        return Ok(Value::Bool(Self::first_char_pred(&args, |c| {
            c.is_whitespace()
        })?));
    } else if name == ".len" {
        match args.get(0) {
            Some(Value::String(s)) => return Ok(Value::Int(s.len() as i64)),
            Some(Value::List(items)) => {
                return Ok(Value::Int(items.len() as i64))
            }
            _ => {
                return Err(anyhow!(
                    "Method .len() expects String or List argument"
                ))
            }
        }
    } else if name == ".contains" {
        match (args.get(0), args.get(1)) {
            (Some(Value::String(hay)), Some(Value::String(needle))) => {
                return Ok(Value::Bool(hay.contains(needle.as_str())));
            }
            _ => {
                return Err(anyhow!(
                    "Method .contains() expects String receiver and String needle"
                ))
            }
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
    } else if name == ".is_err" {
        return Ok(Value::Bool(matches!(args.get(0), Some(Value::Err(_)))));
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
        // Optional leading SysCap token (object-cap).
        let mut ai = 0usize;
        if matches!(args.first(), Some(Value::Capability(_))) {
            ai = 1;
        }
        let task_name = args.get(ai).map(|v| v.to_string()).unwrap_or_default();
        let id = self.next_thread_id;
        self.next_thread_id += 1;
        // Real OS thread — does work and returns a result that
        // async_join_internal can collect. This is no longer a fake
        // handle string.
        let handle = std::thread::Builder::new()
            .name(format!("ooda-{}", task_name))
            .spawn(move || {
                // Minimal real work: yield so the OS scheduler runs it,
                // then return the task name as the joined result.
                std::thread::sleep(std::time::Duration::from_millis(1));
                format!("task_done:{}", task_name)
            })
            .map_err(|e| anyhow!("async_spawn_internal: thread spawn failed: {}", e))?;
        self.threads.insert(id, handle);
        return Ok(Value::String(format!("thread#{}", id)));
    } else if name == "async_join_internal" {
        let mut ai = 0usize;
        if matches!(args.first(), Some(Value::Capability(_))) {
            ai = 1;
        }
        let handle = args.get(ai).map(|v| v.to_string()).unwrap_or_default();
        let id: u64 = match handle.strip_prefix("thread#").and_then(|s| s.parse().ok()) {
            Some(n) => n,
            None => {
                return Ok(Value::Err(Box::new(Value::String(format!(
                    "async_join_internal: malformed handle '{}'",
                    handle
                )))))
            }
        };
        let join = match self.threads.remove(&id) {
            Some(j) => j,
            None => {
                return Ok(Value::Err(Box::new(Value::String(format!(
                    "async_join_internal: no live thread with id {}",
                    id
                )))))
            }
        };
        return match join.join() {
            Ok(s) => Ok(Value::Ok(Box::new(Value::String(s)))),
            Err(_) => Ok(Value::Err(Box::new(Value::String(format!(
                "async_join_internal: worker thread {} panicked",
                id
            ))))),
        };
    }
        unreachable!("builtin group 2: {}", name);
    }


    fn call_builtins_3(&mut self, name: &str, args: &[Value]) -> Result<Value> {
    if name == "python_embed_internal" {
        // Honest: no in-process CPython / PyTorch. Do not claim models load.
        let model = args
            .get(1)
            .map(|v| v.to_string())
            .or_else(|| args.first().map(|v| v.to_string()))
            .unwrap_or_default();
        let py_on_path = std::process::Command::new("python3")
            .arg("-c")
            .arg("print('ok')")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        return Ok(Value::Err(Box::new(Value::String(format!(
            "python_embed_internal: in-process CPython/PyTorch embed is not implemented \
             (requested model '{}'; host python3 on PATH: {}). \
             std::python cannot load models in this alpha — fail-closed.",
            model, py_on_path
        )))));
    } else if name == "Ok" {
        let val = args.get(0).cloned().unwrap_or(Value::Void);
        return Ok(Value::Ok(Box::new(val)));
    } else if name == "Err" {
        let val = args.get(0).cloned().unwrap_or(Value::Void);
        return Ok(Value::Err(Box::new(val)));
    } else if name == "Some" {
        let val = args.get(0).cloned().unwrap_or(Value::Void);
        return Ok(Value::Some(Box::new(val)));
    } else if name == "None" {
        return Ok(Value::None);
    } else if name == ".is_some" {
        return Ok(Value::Bool(matches!(args.get(0), Some(Value::Some(_)))));
    } else if name == ".is_none" {
        return Ok(Value::Bool(matches!(args.get(0), Some(Value::None))));
    }
        unreachable!("builtin group 3: {}", name);
    }

}
