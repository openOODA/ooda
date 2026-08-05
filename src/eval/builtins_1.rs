impl Interpreter {

    fn call_builtins_1(&mut self, name: &str, args: &[Value]) -> Result<Value> {
    if name == "sys_exec" || name == "exec" {
        // Minimal: run argv[0] with remaining string args; return stdout or Err.
        if args.is_empty() {
            return Err(anyhow!("sys_exec: need command"));
        }
        let mut idx = 0;
        if matches!(args.first(), Some(Value::Capability(_))) {
            idx = 1;
        }
        let cmd = match args.get(idx) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(anyhow!("sys_exec: command must be String")),
        };
        let mut c = std::process::Command::new(&cmd);
        for a in args.iter().skip(idx + 1) {
            if let Value::String(s) = a {
                c.arg(s);
            }
        }
        return match c.output() {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                Ok(Value::Ok(Box::new(Value::String(stdout))))
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                Ok(Value::Err(Box::new(Value::String(err))))
            }
            Err(e) => Ok(Value::Err(Box::new(Value::String(e.to_string())))),
        };
    } else if name == "host_ast_dump" {
        // Exact stage-0 AST dump for path (CHS oodac parity).
        let path = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(anyhow!("host_ast_dump expects String path")),
        };
        match crate::host_api::host_ast_dump_path(std::path::Path::new(&path)) {
            Ok(s) => return Ok(Value::String(s)),
            Err(e) => {
                return Ok(Value::String(crate::dump::format_check_err("ast", &e)))
            }
        }
    } else if name == "host_check" {
        let path = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(anyhow!("host_check expects String path")),
        };
        return Ok(Value::String(crate::host_api::host_check_path(
            std::path::Path::new(&path),
        )));
    } else if name == "host_token_dump" {
        let path = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(anyhow!("host_token_dump expects String path")),
        };
        match crate::host_api::host_token_dump_path(std::path::Path::new(&path)) {
            Ok(s) => return Ok(Value::String(s)),
            Err(e) => {
                return Ok(Value::String(crate::dump::format_check_err("tokens", &e)))
            }
        }
    } else if name == "chs_build" {
        // Real CHS native build: path_src, path_out_bin
        let src = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(anyhow!("chs_build expects src String")),
        };
        let out = match args.get(1) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(anyhow!("chs_build expects out_bin String")),
        };
        match crate::host_api::host_chs_build(
            std::path::Path::new(&src),
            std::path::Path::new(&out),
        ) {
            Ok(()) => {
                return Ok(Value::Ok(Box::new(Value::String(out))))
            }
            Err(e) => {
                return Ok(Value::Err(Box::new(Value::String(e))))
            }
        }
    } else if name == "process_exit" {
        let code = match args.get(0) {
            Some(Value::Int(n)) => *n as i32,
            _ => 1,
        };
        std::process::exit(code);
    } else if name == "list_new" {
        return Ok(Value::List(Vec::new()));
    } else if name == "list_push" || name == ".push" {
        // list_push(list, x) or list.push(x) → new list with x appended
        let (base, item) = if name == ".push" {
            (
                args.get(0).cloned().unwrap_or(Value::List(vec![])),
                args.get(1).cloned().unwrap_or(Value::Void),
            )
        } else {
            (
                args.get(0).cloned().unwrap_or(Value::List(vec![])),
                args.get(1).cloned().unwrap_or(Value::Void),
            )
        };
        match base {
            Value::List(mut items) => {
                items.push(item);
                return Ok(Value::List(items));
            }
            other => {
                return Err(anyhow!(
                    "list_push expects List as first argument, found {}",
                    other
                ))
            }
        }
    } else if name == "list_get" {
        let list = args.get(0).cloned().unwrap_or(Value::List(vec![]));
        let idx = match args.get(1) {
            Some(Value::Int(i)) => *i,
            _ => return Err(anyhow!("list_get expects Int index")),
        };
        match list {
            Value::List(items) => {
                if idx < 0 || idx as usize >= items.len() {
                    return Err(anyhow!(
                        "list_get: index {} out of bounds (len {})",
                        idx,
                        items.len()
                    ));
                }
                return Ok(items[idx as usize].clone());
            }
            other => {
                return Err(anyhow!("list_get expects List, found {}", other))
            }
        }
    } else if name == "list_len" {
        match args.get(0) {
            Some(Value::List(items)) => return Ok(Value::Int(items.len() as i64)),
            Some(other) => {
                return Err(anyhow!("list_len expects List, found {}", other))
            }
            None => return Err(anyhow!("list_len expects one argument")),
        }
    } else if name == "chars_len" {
        match args.get(0) {
            Some(Value::String(s)) => {
                return Ok(Value::Int(s.chars().count() as i64))
            }
            _ => return Err(anyhow!("chars_len expects String")),
        }
    } else if name == "char_at" || name == ".char_at" {
        let s = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err(anyhow!("char_at expects String")),
        };
        let idx = match args.get(1) {
            Some(Value::Int(i)) => *i,
            _ => return Err(anyhow!("char_at expects Int index")),
        };
        if idx < 0 {
            return Err(anyhow!("char_at: negative index {}", idx));
        }
        return match s.chars().nth(idx as usize) {
            Some(c) => Ok(Value::String(c.to_string())),
            None => Err(anyhow!(
                "char_at: index {} out of bounds (chars_len {})",
                idx,
                s.chars().count()
            )),
        };
    }
        unreachable!("builtin group 1: {}", name);
    }

}
