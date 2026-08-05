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
    /// Loop control: break/continue the innermost while/for.
    pending_break: bool,
    pending_continue: bool,
}

