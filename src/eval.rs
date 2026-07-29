use crate::ast::*;
use crate::capabilities::{lookup_effect, CapKind};
// UnaryOp used in eval_expr
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};

type HmacSha256 = Hmac<Sha256>;

/// Which capability tokens a function declares in its parameter list.
#[derive(Debug, Clone, Copy, Default)]
pub struct CapSet {
    pub net: bool,
    pub fs: bool,
    pub sys: bool,
    pub env: bool,
}

impl CapSet {
    fn from_params(func: &FunctionDecl) -> Self {
        let mut s = CapSet::default();
        for p in &func.params {
            match p.param_type {
                Type::NetCap => s.net = true,
                Type::FsCap => s.fs = true,
                Type::SysCap => s.sys = true,
                Type::EnvCap => s.env = true,
                _ => {}
            }
        }
        s
    }

    fn has(&self, k: CapKind) -> bool {
        match k {
            CapKind::Net => self.net,
            CapKind::Fs => self.fs,
            CapKind::Sys => self.sys,
            CapKind::Env => self.env,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Void,
    Ok(Box<Value>),
    Err(Box<Value>),
    Some(Box<Value>),
    None,
    Capability(String),
    /// Homogeneous list (element types checked loosely at runtime).
    List(Vec<Value>),
    /// Named product type instance from a struct literal / type alias.
    Record {
        type_name: String,
        fields: HashMap<String, Value>,
    },
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
            Value::Some(v) => write!(f, "Some({})", v),
            Value::None => write!(f, "None"),
            Value::Capability(c) => write!(f, "<Capability: {}>", c),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Record { type_name, fields } => {
                write!(f, "{} {{", type_name)?;
                let mut first = true;
                for (k, v) in fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, " {}: {}", k, v)?;
                }
                write!(f, " }}")
            }
        }
    }
}

pub struct Interpreter {
    functions: HashMap<String, FunctionDecl>,
    globals: HashMap<String, Value>,
    func_caps: HashMap<String, CapSet>,
    current_func: Option<String>,
    /// Most recent call-site span (for contract / runtime diagnostics).
    last_call_span: Span,
    /// Live OS threads spawned by `async_spawn_internal`. Keyed by numeric handle id.
    threads: HashMap<u64, std::thread::JoinHandle<String>>,
    next_thread_id: u64,
    /// CLI / host-injected program arguments for `main(args: List[String], ...)`.
    argv: Vec<String>,
    /// Named struct layouts from `type Name = struct { ... }`.
    struct_defs: HashMap<String, Vec<(String, Type)>>,
    /// `type Port = Int[lo..hi]` bounds (from_ast collapses RHS to Int).
    alias_refinements: HashMap<String, (i64, i64)>,
    /// When `return` executes inside nested if/while blocks, set this so outer
    /// frames propagate out of the function (CHS oodac relies on this).
    pending_return: Option<Value>,
}

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
        if name == "println" {
            for arg in &args {
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
        } else if name == "sys_exec" || name == "exec" {
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
        } else if name == "str_slice" || name == ".str_slice" {
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
        } else if name == "python_embed_internal" {
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
        } else if name.starts_with('.') && args.len() == 1 {
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
                Statement::Expr(expr, _) => {
                    self.eval_expr(expr, env)?;
                    if self.pending_return.is_some() {
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
                    }
                }
            }
        }

        if self.pending_return.is_some() {
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
                    let c = self.eval_expr(cond, env)?;
                    if c != Value::Bool(true) {
                        break;
                    }
                    self.eval_block(body, env)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Program {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = crate::parser::Parser::new(tokens);
        parser.parse_program().expect("parse")
    }

    #[test]
    fn nested_let_does_not_pollute_outer_runtime_env() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let x = 1;
                if true {
                    let x = 99;
                }
                return x;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(1), "outer x must remain 1 after nested let shadow");
    }

    #[test]
    fn outer_mut_assign_inside_if_persists() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let mut x = 1;
                if true {
                    x = 2;
                }
                return x;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(2), "assign to outer let mut must persist");
    }

    #[test]
    fn nested_while_let_does_not_pollute_outer() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let x = 1;
                let mut i = 0;
                while i < 1 {
                    let x = 42;
                    i = i + 1;
                }
                return x;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(1), "while-body let must not leak");
    }

    #[test]
    fn match_if_outer_mut_assign_persists() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let mut x = 0;
                let r = Ok(5);
                match r {
                    Ok(v) => if true {
                        x = v;
                        v
                    } else {
                        0
                    },
                    Err(e) => 0,
                };
                return x;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(5), "outer mut x must be 5 after match arm assign");
    }

    #[test]
    fn method_char_at_runtime() {
        let prog = parse(
            r#"
            pub fn main() -> String {
                return "hi".char_at(1);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::String("i".into()));
    }

    #[test]
    fn runtime_rejects_refinement_param_oob() {
        let prog = parse(
            r#"
            pub fn port(p: Int[1..65535]) -> Int {
                return p;
            }
            pub fn main() -> Int {
                let bad = 0;
                return port(bad);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let res = interp.call_function("main", vec![], &mut HashMap::new());
        assert!(res.is_err(), "non-const OOB refinement arg must fail at runtime");
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("RefinementTypeViolation"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn runtime_rejects_alias_refinement_param_oob() {
        let prog = parse(
            r#"
            type Port = Int[1..65535];
            pub fn take(p: Port) -> Int {
                return p;
            }
            pub fn main() -> Int {
                let bad = 0;
                return take(bad);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let res = interp.call_function("main", vec![], &mut HashMap::new());
        assert!(res.is_err(), "alias Port Int[lo..hi] non-const OOB must fail");
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("RefinementTypeViolation"), "got: {}", msg);
    }

    #[test]
    fn runtime_capability_blocks_syscall_without_cap() {
        // The static checker would also catch this, but the runtime check
        // is the last line of defense: it must fire even if static checks
        // are bypassed.
        let prog = parse(
            r#"
            pub fn rogue() {
                let h = async_spawn_internal("x");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("rogue".into());
        let res = interp.call_function("async_spawn_internal", vec![Value::String("x".into())], &mut HashMap::new());
        assert!(res.is_err());
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("Runtime Security Capability Violation"), "got: {}", msg);
        assert!(msg.contains("&SysCap"), "got: {}", msg);
    }

    #[test]
    fn runtime_capability_allows_with_correct_cap() {
        let prog = parse(
            r#"
            pub fn ok(sys: &SysCap) -> String {
                return async_spawn_internal(sys, "y");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("ok".into());
        let res = interp.call_function(
            "async_spawn_internal",
            vec![
                Value::Capability("SysCap".into()),
                Value::String("y".into()),
            ],
            &mut HashMap::new(),
        );
        assert!(res.is_ok(), "expected ok, got: {:?}", res);
    }

    #[test]
    fn runtime_capability_wrong_kind_still_blocks() {
        let prog = parse(
            r#"
            pub fn wrong(net: &NetCap) {
                let h = async_spawn_internal("z");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("wrong".into());
        let res = interp.call_function(
            "async_spawn_internal",
            vec![Value::String("z".into())],
            &mut HashMap::new(),
        );
        assert!(res.is_err());
        assert!(format!("{}", res.unwrap_err()).contains("&SysCap"));
    }

    #[test]
    fn real_read_write_file_roundtrip() {
        let base_dir = std::path::PathBuf::from("/home/jeryd/openooda/target/tmp");
        let dir = base_dir.join(format!(
            "ooda_m0_fs_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.txt");
        let path_s = path.to_string_lossy().to_string();

        let prog = parse(
            r#"
            pub fn main(fs: &FsCap) {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("main".into());

        let w = interp
            .call_function(
                "write_file",
                vec![
                    Value::Capability("FsCap".into()),
                    Value::String(path_s.clone()),
                    Value::String("hello-m0".into()),
                ],
                &mut HashMap::new(),
            )
            .expect("write");
        assert!(matches!(w, Value::Ok(_)), "write ok: {:?}", w);

        let r = interp
            .call_function(
                "read_file",
                vec![
                    Value::Capability("FsCap".into()),
                    Value::String(path_s),
                ],
                &mut HashMap::new(),
            )
            .expect("read");
        match r {
            Value::Ok(inner) => assert_eq!(*inner, Value::String("hello-m0".into())),
            other => panic!("expected Ok content, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_file_without_fscap_is_denied() {
        let prog = parse(
            r#"
            pub fn rogue() {
                let _ = read_file("/etc/passwd");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("rogue".into());
        let res = interp.call_function(
            "read_file",
            vec![Value::String("/etc/passwd".into())],
            &mut HashMap::new(),
        );
        assert!(res.is_err());
        assert!(format!("{}", res.unwrap_err()).contains("&FsCap"));
    }

    #[test]
    fn fetch_without_netcap_runtime_denies() {
        let prog = parse(
            r#"
            pub fn rogue() {
                let r = fetch("https://example.invalid");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("rogue".into());
        let res = interp.call_function(
            "fetch",
            vec![Value::String("https://example.invalid".into())],
            &mut HashMap::new(),
        );
        assert!(res.is_err(), "expected runtime deny, got {:?}", res);
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("Runtime Security Capability Violation"), "got: {}", msg);
        assert!(msg.contains("&NetCap"), "got: {}", msg);
    }

    #[test]
    fn write_file_with_wrong_kind_handle_runtime_denies() {
        // Live NetCap is not a valid handle for Fs sealed write_file.
        let prog = parse(
            r#"
            pub fn mix(net: &NetCap, fs: &FsCap) {
                let r = write_file(net, "/tmp/x", "y");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("mix".into());
        let res = interp.call_function(
            "write_file",
            vec![
                Value::Capability("NetCap".into()),
                Value::String("/tmp/x".into()),
                Value::String("y".into()),
            ],
            &mut HashMap::new(),
        );
        assert!(res.is_err(), "wrong-kind handle must deny: {:?}", res);
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("object-capability") || msg.contains("live") || msg.contains("FsCap"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn fetch_ambient_only_without_handle_arg_runtime_denies() {
        // Function declares &NetCap but call omits the live handle — object-cap deny.
        let prog = parse(
            r#"
            pub fn ambient(net: &NetCap) {
                let r = fetch("https://example.invalid");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("ambient".into());
        let res = interp.call_function(
            "fetch",
            vec![Value::String("https://example.invalid".into())],
            &mut HashMap::new(),
        );
        assert!(res.is_err(), "ambient-only fetch must deny: {:?}", res);
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("object-capability") || msg.contains("live"),
            "expected object-cap message, got: {}",
            msg
        );
    }

    #[test]
    fn fetch_with_netcap_returns_result_not_fake_ok() {
        // With NetCap granted AND live handle arg, fetch is allowed. A refused
        // loopback URL must yield Err (or Ok if something answers) — never "200 OK".
        let prog = parse(
            r#"
            pub fn ok(net: &NetCap) -> Result[String, String] {
                return fetch(net, "https://127.0.0.1:1/");
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("ok".into());
        let res = interp
            .call_function(
                "fetch",
                vec![
                    Value::Capability("NetCap".into()),
                    Value::String("https://127.0.0.1:1/".into()),
                ],
                &mut HashMap::new(),
            )
            .expect("fetch with live NetCap handle must be allowed");
        match res {
            Value::Err(e) => {
                let s = format!("{}", e);
                assert!(!s.is_empty(), "err message must be non-empty");
                assert!(!s.contains("200 OK"), "must not fake success: {}", s);
            }
            Value::Ok(body) => {
                // Unexpected but honest if something listened on :1
                assert!(matches!(*body, Value::String(_)));
            }
            other => panic!("fetch must return Result, got {:?}", other),
        }
    }

    #[test]
    fn where_type_alias_parses_successfully() {
        let src = r#"type Port = Int where 1..=65535; pub fn main() {}"#;
        let tokens = crate::lexer::Lexer::new(src).tokenize().expect("lex");
        let prog = crate::parser::Parser::new(tokens)
            .parse_program()
            .expect("where must parse successfully");
        if let crate::ast::Item::TypeAlias(name, target) = &prog.items[0] {
            assert_eq!(name, "Port");
            assert!(matches!(target, crate::ast::Type::Custom(ref s) if s == "Int[1..65535]"));
        } else {
            panic!("Expected TypeAlias");
        }
    }

    #[test]
    fn where_type_alias_rejects_non_const_range() {
        let src = r#"type Port = Int where x..y; pub fn main() {}"#;
        let tokens = crate::lexer::Lexer::new(src).tokenize().expect("lex");
        let err = crate::parser::Parser::new(tokens)
            .parse_program()
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("where") && (err.contains("const") || err.contains("range")),
            "got: {}",
            err
        );
    }

    #[test]
    fn for_range_loop_runtime() {
        let prog = parse(
            r#"
            pub fn main() -> Int {
                let mut s = 0;
                for i in 1..=3 {
                    s = s + i;
                }
                return s;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("for loop");
        assert_eq!(v, Value::Int(6)); // 1+2+3
    }

    #[test]
    fn python_embed_returns_honest_err() {
        let prog = parse(
            r#"
            pub fn main(sys: &SysCap) -> Result[String, String] {
                return python_embed_internal(sys, "torch");
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function(
                "main",
                vec![Value::Capability("SysCap".into())],
                &mut HashMap::new(),
            )
            .expect("call");
        match v {
            Value::Err(e) => {
                let s = format!("{:?}", e);
                assert!(
                    s.contains("not implemented") || s.contains("python_embed"),
                    "got: {}",
                    s
                );
            }
            other => panic!("expected Err, got {:?}", other),
        }
    }

    #[test]
    fn list_push_get_len() {
        let prog = parse(r#"pub fn main() {}"#);
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("main".into());
        let empty = interp
            .call_function("list_new", vec![], &mut HashMap::new())
            .unwrap();
        let one = interp
            .call_function(
                "list_push",
                vec![empty, Value::Int(7)],
                &mut HashMap::new(),
            )
            .unwrap();
        let two = interp
            .call_function(
                "list_push",
                vec![one, Value::Int(9)],
                &mut HashMap::new(),
            )
            .unwrap();
        let len = interp
            .call_function("list_len", vec![two.clone()], &mut HashMap::new())
            .unwrap();
        assert_eq!(len, Value::Int(2));
        let g0 = interp
            .call_function(
                "list_get",
                vec![two.clone(), Value::Int(0)],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(g0, Value::Int(7));
        let g1 = interp
            .call_function(
                "list_get",
                vec![two, Value::Int(1)],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(g1, Value::Int(9));
    }

    #[test]
    fn string_char_walk() {
        let prog = parse(r#"pub fn main() {}"#);
        let mut interp = Interpreter::new(prog);
        interp.current_func = Some("main".into());
        let s = Value::String("ab".into());
        let n = interp
            .call_function("chars_len", vec![s.clone()], &mut HashMap::new())
            .unwrap();
        assert_eq!(n, Value::Int(2));
        let c0 = interp
            .call_function(
                "char_at",
                vec![s.clone(), Value::Int(0)],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(c0, Value::String("a".into()));
        let slice = interp
            .call_function(
                "str_slice",
                vec![s, Value::Int(0), Value::Int(1)],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(slice, Value::String("a".into()));
        let dig = interp
            .call_function(
                "char_is_digit",
                vec![Value::String("9".into())],
                &mut HashMap::new(),
            )
            .unwrap();
        assert_eq!(dig, Value::Bool(true));
    }

    #[test]
    fn struct_literal_and_field_access() {
        let prog = parse(
            r#"
            type Token = struct {
                kind: Int,
                text: String
            };
            pub fn main() {
                let t = Token { kind: 1, text: "fn" };
                println(t.kind);
                println(t.text);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        assert!(interp.execute_all().is_ok());
    }

    #[test]
    fn argv_injected_into_main() {
        let prog = parse(
            r#"
            pub fn main(args: List[String]) {
                println(list_len(args));
            }
            "#,
        );
        let mut interp = Interpreter::new(prog).with_argv(vec!["a".into(), "b".into()]);
        assert!(interp.execute_all().is_ok());
    }

    #[test]
    fn fuzz_fails_closed_on_unexpected_errors() {
        // Division by zero / postcondition trap must not soft-pass as green fuzz.
        let prog = parse(
            r#"
            pub fn bad(x: Int) -> Int
                requires x >= 0
                ensures result >= 0
            {
                return x - 100;
            }
            pub fn main() {}
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let res = interp.fuzz_all();
        // Fuzz may ok if all combos either pass or pre-fail; with ensures result >= 0
        // and body x-100, x=0 yields -100 and postcondition fails → other_err.
        assert!(
            res.is_err(),
            "fuzz must fail closed when postconditions break: {:?}",
            res
        );
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("unexpected error") || msg.contains("Fuzz"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn postcondition_old_state_snapshot_verification() {
        let prog = parse(
            r#"
            pub fn increment(x: Int) -> Int
                ensures result == old(x) + 1
            {
                return x + 1;
            }
            pub fn main() {
                let y = increment(5);
                println(y);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        assert!(interp.execute_all().is_ok());
    }

    #[test]
    fn function_without_old_state_skips_snapshot() {
        // No `old()` references anywhere — interpreter should NOT
        // allocate a snapshot HashMap. We verify by checking that
        // a function with a requires clause (which doesn't need the
        // snapshot either) still runs and prints.
        let prog = parse(
            r#"
            pub fn double(x: Int) -> Int
                requires x >= 0
                ensures result == x * 2
            {
                return x * 2;
            }
            pub fn main() {
                let y = double(21);
                println(y);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        interp.execute_all().expect("must run without snapshot");
    }

    #[test]
    fn return_inside_if_returns_from_function() {
        let prog = parse(
            r#"
            pub fn pick(x: Int) -> Int {
                if x > 0 {
                    return 1;
                }
                return 2;
            }
            pub fn main() {
                assert_eq(pick(5), 1);
                assert_eq(pick(0), 2);
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        assert!(interp.execute_all().is_ok());
    }

    #[test]
    fn runtime_rejects_alias_return_refinement_oob() {
        let prog = parse(
            r#"
            type Port = Int[1..10];
            pub fn f(x: Int) -> Port {
                return x;
            }
            pub fn main() -> Int {
                let bad = 99;
                let _p = f(bad);
                return 0;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let res = interp.call_function("main", vec![], &mut HashMap::new());
        assert!(res.is_err(), "non-const alias return OOB must fail");
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("RefinementTypeViolation"), "got: {}", msg);
    }


    #[test]
    fn field_assign_runtime() {
        let prog = parse(
            r#"
            type Pt = struct { x: Int, y: Int };
            pub fn main() -> Int {
                let mut p = Pt { x: 1, y: 2 };
                p.x = 7;
                return p.x;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp.call_function("main", vec![], &mut HashMap::new()).expect("run");
        assert_eq!(v, Value::Int(7));
    }

    #[test]
    fn nested_field_assign_runtime() {
        let prog = parse(
            r#"
            type Inner = struct { n: Int };
            type Outer = struct { inner: Inner };
            pub fn main() -> Int {
                let mut o = Outer { inner: Inner { n: 1 } };
                o.inner.n = 9;
                return o.inner.n;
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("nested field assign");
        assert_eq!(v, Value::Int(9));
    }

    #[test]
    fn contains_method_runtime() {
        let prog = parse(
            r#"
            pub fn main() -> Bool {
                return "hello".contains("ell");
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp.call_function("main", vec![], &mut HashMap::new()).expect("run");
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn question_mark_runtime_ok() {
        let prog = parse(
            r#"
            pub fn f() -> Result[Int, String] { return Ok(7); }
            pub fn g() -> Result[Int, String] {
                let x = f()?;
                return Ok(x);
            }
            pub fn main() -> Int {
                match g() {
                    Ok(v) => v,
                    Err(e) => 0,
                }
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::Int(7));
    }

    #[test]
    fn question_mark_early_return_err() {
        let prog = parse(
            r#"
            pub fn fail() -> Result[Int, String] { return Err("nope"); }
            pub fn g() -> Result[Int, String] {
                let x = fail()?;
                return Ok(x + 1);
            }
            pub fn main() -> String {
                match g() {
                    Ok(v) => "ok",
                    Err(e) => e,
                }
            }
            "#,
        );
        let mut interp = Interpreter::new(prog);
        let v = interp
            .call_function("main", vec![], &mut HashMap::new())
            .expect("run");
        assert_eq!(v, Value::String("nope".into()));
    }
}
