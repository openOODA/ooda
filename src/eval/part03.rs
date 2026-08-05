impl Interpreter {
    fn call_builtins_0(&mut self, name: &str, args: &[Value]) -> Result<Value> {
    if name == "println" {
        for arg in args {
            print!("{}", arg);
        }
        println!();
        return Ok(Value::Void);
    } else if name == "read_file" || name == ".read_file" || name == "fs_read" {
        let path = Self::fs_path_arg(name, &args)?;
        return match std::fs::read_to_string(&path) {
            Ok(s) => Ok(Value::Ok(Box::new(Value::String(s)))),
            Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                "read_file('{}'): {}",
                path, e
            ))))),
        };
    } else if name == "write_file" || name == ".write_file" || name == "fs_write" {
        let (path, content) = Self::fs_write_args(name, &args)?;
        return match std::fs::write(&path, &content) {
            Ok(()) => Ok(Value::Ok(Box::new(Value::Void))),
            Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                "write_file('{}'): {}",
                path, e
            ))))),
        };
    } else if name == "env_get" || name == ".env_get" {
        let key = Self::env_key_arg(name, &args)?;
        return match std::env::var(&key) {
            Ok(v) => Ok(Value::Ok(Box::new(Value::String(v)))),
            Err(_) => Ok(Value::Err(Box::new(Value::String(format!(
                "env_get: '{}' not set",
                key
            ))))),
        };
    } else if name == "mkdir_p" {
        let path = Self::fs_path_arg(name, &args)?;
        return match std::fs::create_dir_all(&path) {
            Ok(()) => Ok(Value::Ok(Box::new(Value::Void))),
            Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                "mkdir_p('{}'): {}",
                path, e
            ))))),
        };
    } else if name == "copy_file" {
        let src = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Capability(_)) => match args.get(1) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err(anyhow!("copy_file: missing src")),
            },
            _ => return Err(anyhow!("copy_file: src must be String")),
        };
        let dst = match args.get(args.len().saturating_sub(1)) {
            Some(Value::String(s)) if args.len() >= 2 => s.clone(),
            _ => return Err(anyhow!("copy_file: dst must be String")),
        };
        // if first is cap, dst is still last string; src is args[1]
        let (src, dst) = if matches!(args.first(), Some(Value::Capability(_))) {
            (
                match args.get(1) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(anyhow!("copy_file: src must be String")),
                },
                match args.get(2) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(anyhow!("copy_file: dst must be String")),
                },
            )
        } else {
            (src, dst)
        };
        if let Some(parent) = std::path::Path::new(&dst).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        return match std::fs::copy(&src, &dst) {
            Ok(_) => Ok(Value::Ok(Box::new(Value::Void))),
            Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                "copy_file('{}' -> '{}'): {}",
                src, dst, e
            ))))),
        };
    } else if name == "chmod_exec" {
        let path = Self::fs_path_arg(name, &args)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            return match std::fs::metadata(&path).and_then(|m| {
                let mut p = m.permissions();
                p.set_mode(p.mode() | 0o111);
                std::fs::set_permissions(&path, p)
            }) {
                Ok(()) => Ok(Value::Ok(Box::new(Value::Void))),
                Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                    "chmod_exec('{}'): {}",
                    path, e
                ))))),
            };
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Ok(Value::Ok(Box::new(Value::Void)))
        }
    } else if name == "path_exists" {
        let path = Self::fs_path_arg(name, &args)?;
        return Ok(Value::Bool(std::path::Path::new(&path).exists()));
    } else if name == "fetch"
        || name == "http_get"
        || name == "net_get"
        || name == "downloadData"
        || name == ".get"
    {
        // HTTPS GET via curl → Result[String, String] body.
        // Skip optional leading capability token (method receiver or ambient arg).
        let url = Self::net_url_arg(name, &args)?;
        return Ok(Self::http_get_body(&url));
    } else if name == "http_download" {
        // url, dest_path (optional leading cap token skipped)
        let (url, dest) = {
            let mut i = 0;
            if matches!(args.first(), Some(Value::Capability(_))) {
                i = 1;
            }
            let url = match args.get(i) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err(anyhow!("http_download: url must be String")),
            };
            let dest = match args.get(i + 1) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err(anyhow!("http_download: dest must be String")),
            };
            (url, dest)
        };
        if let Some(parent) = std::path::Path::new(&dest).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Prefer curl (HTTPS); fall back to error with honest message.
        let out = std::process::Command::new("curl")
            .args(["-fsSL", "--proto", "=https", "--tlsv1.2", "-o", &dest, &url])
            .output();
        return match out {
            Ok(o) if o.status.success() => Ok(Value::Ok(Box::new(Value::Void))),
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr);
                Ok(Value::Err(Box::new(Value::String(format!(
                    "http_download: curl failed for '{}': {}",
                    url,
                    err.chars().take(240).collect::<String>()
                )))))
            }
            Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                "http_download: curl not available ({}); cannot fetch HTTPS without curl in this alpha",
                e
            ))))),
        };
    } else if name == "extract_tar_gz" {
        let (archive, dest) = {
            let mut i = 0;
            if matches!(args.first(), Some(Value::Capability(_))) {
                i = 1;
            }
            let a = match args.get(i) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err(anyhow!("extract_tar_gz: archive must be String")),
            };
            let d = match args.get(i + 1) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err(anyhow!("extract_tar_gz: dest must be String")),
            };
            (a, d)
        };
        let _ = std::fs::create_dir_all(&dest);
        let out = std::process::Command::new("tar")
            .args(["-xzf", &archive, "-C", &dest])
            .output();
        return match out {
            Ok(o) if o.status.success() => Ok(Value::Ok(Box::new(Value::Void))),
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr);
                Ok(Value::Err(Box::new(Value::String(format!(
                    "extract_tar_gz: {}",
                    err.chars().take(240).collect::<String>()
                )))))
            }
            Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                "extract_tar_gz: tar not available: {}",
                e
            ))))),
        };
    }
        unreachable!("builtin group 0: {}", name);
    }

}
