impl Interpreter {

    /// URL extraction for free/method net GETs.
    /// - `fetch(url)` / `http_get(url)` / `net_get(url)` / `downloadData(url)`
    /// - `.get(cap, url)` / `fetch(net, url)` with leading capability token
    fn net_url_arg(name: &str, args: &[Value]) -> Result<String> {
        if name.starts_with('.') || matches!(args.first(), Some(Value::Capability(_))) {
            match args.get(1) {
                Some(Value::String(s)) => Ok(s.clone()),
                Some(other) => Err(anyhow!(
                    "{}: url must be String, found {}",
                    name,
                    other
                )),
                None => Err(anyhow!("{}: missing url argument", name)),
            }
        } else {
            match args.get(0) {
                Some(Value::String(s)) => Ok(s.clone()),
                Some(other) => Err(anyhow!(
                    "{}: url must be String, found {}",
                    name,
                    other
                )),
                None => Err(anyhow!("{}: missing url argument", name)),
            }
        }
    }


    /// Real HTTPS GET of response body via curl. Returns `Ok(body)` or `Err(msg)`.
    fn http_get_body(url: &str) -> Value {
        let out = std::process::Command::new("curl")
            .args([
                "-fsSL",
                "--proto",
                "=https",
                "--tlsv1.2",
                "--max-time",
                "15",
                url,
            ])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let body = String::from_utf8_lossy(&o.stdout).into_owned();
                Value::Ok(Box::new(Value::String(body)))
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr);
                Value::Err(Box::new(Value::String(format!(
                    "fetch: curl failed for '{}': {}",
                    url,
                    err.chars().take(240).collect::<String>()
                ))))
            }
            Err(e) => Value::Err(Box::new(Value::String(format!(
                "fetch: curl not available ({}); cannot perform HTTPS GET without curl in this alpha",
                e
            )))),
        }
    }


    /// Path extraction for free/method FS reads.
    /// - `read_file(path)` / `fs_read(path)`
    /// - `read_file(fs, path)` / `.read_file` receiver+path
    fn fs_path_arg(name: &str, args: &[Value]) -> Result<String> {
        if name.starts_with('.') || matches!(args.first(), Some(Value::Capability(_))) {
            match args.get(1) {
                Some(Value::String(s)) => Ok(s.clone()),
                Some(other) => Err(anyhow!(
                    "{}: path must be String, found {}",
                    name,
                    other
                )),
                None => Err(anyhow!("{}: missing path argument", name)),
            }
        } else {
            match args.get(0) {
                Some(Value::String(s)) => Ok(s.clone()),
                Some(other) => Err(anyhow!(
                    "{}: path must be String, found {}",
                    name,
                    other
                )),
                None => Err(anyhow!("{}: missing path argument", name)),
            }
        }
    }


    fn fs_write_args(name: &str, args: &[Value]) -> Result<(String, String)> {
        // .write_file(cap, path, content) or write_file(cap, path, content) or write_file(path, content)
        let (path_i, content_i) = if name.starts_with('.')
            || matches!(args.first(), Some(Value::Capability(_)))
        {
            (1, 2)
        } else {
            (0, 1)
        };
        let path = match args.get(path_i) {
            Some(Value::String(s)) => s.clone(),
            Some(other) => {
                return Err(anyhow!(
                    "{}: path must be String, found {}",
                    name,
                    other
                ))
            }
            None => return Err(anyhow!("{}: missing path", name)),
        };
        let content = match args.get(content_i) {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => return Err(anyhow!("{}: missing content", name)),
        };
        Ok((path, content))
    }


    fn env_key_arg(name: &str, args: &[Value]) -> Result<String> {
        if name.starts_with('.') || matches!(args.first(), Some(Value::Capability(_))) {
            match args.get(1) {
                Some(Value::String(s)) => Ok(s.clone()),
                _ => Err(anyhow!("{}: key must be String", name)),
            }
        } else {
            match args.get(0) {
                Some(Value::String(s)) => Ok(s.clone()),
                _ => Err(anyhow!("{}: key must be String", name)),
            }
        }
    }


    fn first_char_pred(args: &[Value], pred: impl Fn(char) -> bool) -> Result<bool> {
        match args.get(0) {
            Some(Value::String(s)) => {
                let mut chars = s.chars();
                match chars.next() {
                    Some(c) if chars.next().is_none() => Ok(pred(c)),
                    Some(_) => Ok(false),
                    None => Ok(false),
                }
            }
            _ => Err(anyhow!("char classifier expects a single-character String")),
        }
    }

}
