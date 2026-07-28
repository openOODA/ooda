// ===================================================================
// openOODA Capability Effect System (default-deny)
//
// Real I/O is only allowed through a sealed table of effectful builtins.
// Each entry requires a specific capability type on the enclosing function.
// Renaming calls cannot invent new I/O primitives outside this table.
// ===================================================================
use crate::ast::*;
use anyhow::{anyhow, Result};

/// Which capability token an effectful operation requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapKind {
    Net,
    Fs,
    Sys,
    Env,
}

impl CapKind {
    pub fn type_name(self) -> &'static str {
        match self {
            CapKind::Net => "&NetCap",
            CapKind::Fs => "&FsCap",
            CapKind::Sys => "&SysCap",
            CapKind::Env => "&EnvCap",
        }
    }

    pub fn matches_type(self, t: &Type) -> bool {
        match (self, t) {
            (CapKind::Net, Type::NetCap) => true,
            (CapKind::Fs, Type::FsCap) => true,
            (CapKind::Sys, Type::SysCap) => true,
            (CapKind::Env, Type::EnvCap) => true,
            _ => false,
        }
    }
}

/// Sealed effectful builtin: only these names may perform side-effecting I/O.
#[derive(Debug, Clone, Copy)]
pub struct EffectBuiltin {
    /// Canonical call name as it appears after parsing (methods use ".name").
    pub name: &'static str,
    pub requires: CapKind,
    /// When true, args[0] (method receiver) must be a capability parameter handle.
    /// When false, the enclosing function must declare the cap (ambient grant).
    pub receiver_is_cap: bool,
}

/// Complete sealed surface of effectful operations for this alpha.
pub const EFFECT_BUILTINS: &[EffectBuiltin] = &[
    // Network (free functions: ambient cap on the enclosing fn)
    EffectBuiltin {
        name: "fetch",
        requires: CapKind::Net,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "downloadData",
        requires: CapKind::Net,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "http_get",
        requires: CapKind::Net,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "net_get",
        requires: CapKind::Net,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "net_connect",
        requires: CapKind::Net,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "query_remote_api",
        requires: CapKind::Net,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: ".get",
        requires: CapKind::Net,
        receiver_is_cap: true,
    },
    // Filesystem
    EffectBuiltin {
        name: "read_file",
        requires: CapKind::Fs,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "write_file",
        requires: CapKind::Fs,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "fs_read",
        requires: CapKind::Fs,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "fs_write",
        requires: CapKind::Fs,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: ".read_file",
        requires: CapKind::Fs,
        receiver_is_cap: true,
    },
    EffectBuiltin {
        name: ".write_file",
        requires: CapKind::Fs,
        receiver_is_cap: true,
    },
    // Process / system
    EffectBuiltin {
        name: "sys_exec",
        requires: CapKind::Sys,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "exec",
        requires: CapKind::Sys,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "spawn_process",
        requires: CapKind::Sys,
        receiver_is_cap: false,
    },
    // Environment
    EffectBuiltin {
        name: "env_get",
        requires: CapKind::Env,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "env_set",
        requires: CapKind::Env,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: ".env_get",
        requires: CapKind::Env,
        receiver_is_cap: true,
    },
    // Sealed stdlib internals (callable from .oo but require a SysCap because
    // they spawn threads or invoke out-of-process runtimes).
    EffectBuiltin {
        name: "async_spawn_internal",
        requires: CapKind::Sys,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "async_join_internal",
        requires: CapKind::Sys,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: "python_embed_internal",
        requires: CapKind::Sys,
        receiver_is_cap: false,
    },
];

pub fn lookup_effect(name: &str) -> Option<&'static EffectBuiltin> {
    EFFECT_BUILTINS.iter().find(|e| e.name == name)
}

pub struct CapabilityChecker;

impl CapabilityChecker {
    pub fn check_program(program: &Program) -> Result<()> {
        use std::collections::HashMap;
        let mut funcs: HashMap<String, &FunctionDecl> = HashMap::new();
        for item in &program.items {
            if let Item::Function(func) = item {
                funcs.insert(func.name.clone(), func);
            }
        }
        for item in &program.items {
            if let Item::Function(func) = item {
                Self::check_function(func, &funcs)?;
            }
        }
        Ok(())
    }

    fn function_has_cap(func: &FunctionDecl, kind: CapKind) -> bool {
        func.params.iter().any(|p| kind.matches_type(&p.param_type))
    }

    fn check_function(
        func: &FunctionDecl,
        funcs: &std::collections::HashMap<String, &FunctionDecl>,
    ) -> Result<()> {
        Self::check_block(&func.body, func, funcs)?;
        if let Some(verify) = &func.verify_block {
            // verify blocks run in a trusted test context but still cannot invent
            // ambient I/O without caps on the function under test.
            Self::check_block(verify, func, funcs)?;
        }
        Ok(())
    }

    fn check_block(
        block: &Block,
        func: &FunctionDecl,
        funcs: &std::collections::HashMap<String, &FunctionDecl>,
    ) -> Result<()> {
        for stmt in &block.stmts {
            match stmt {
                Statement::Let { init, .. } => Self::check_expr(init, func, funcs)?,
                Statement::Assign { value, .. } => Self::check_expr(value, func, funcs)?,
                Statement::Return(Some(expr), _) => Self::check_expr(expr, func, funcs)?,
                Statement::Expr(expr, _) => Self::check_expr(expr, func, funcs)?,
                Statement::While { cond, body, .. } => {
                    Self::check_expr(cond, func, funcs)?;
                    Self::check_block(body, func, funcs)?;
                }
                Statement::Return(None, _) => {}
            }
        }
        if let Some(expr) = &block.expr {
            Self::check_expr(expr, func, funcs)?;
        }
        Ok(())
    }

    fn check_expr(
        expr: &Expression,
        func: &FunctionDecl,
        funcs: &std::collections::HashMap<String, &FunctionDecl>,
    ) -> Result<()> {
        match expr {
            Expression::Call { name, args, .. } => {
                if let Some(effect) = lookup_effect(name) {
                    if !Self::function_has_cap(func, effect.requires) {
                        let span = expr.span();
                        return Err(anyhow!(
                            "Security Capability Violation: Function '{}' calls sealed effectful builtin '{}' which requires a {} parameter, but none was declared at line {}, col {}. Default-deny: grant the capability token explicitly.",
                            func.name,
                            name,
                            effect.requires.type_name(),
                            span.line,
                            span.col
                        ));
                    }
                    // Method style: receiver (args[0]) must be the declared cap parameter.
                    if effect.receiver_is_cap {
                        let span = expr.span();
                        match args.first() {
                            Some(recv) if Self::expr_is_cap_handle(recv, effect.requires, func) => {}
                            Some(_) => {
                                return Err(anyhow!(
                                    "Security Capability Violation: Function '{}' calls '{}' at line {}, col {} but the receiver is not a {} capability handle parameter.",
                                    func.name,
                                    name,
                                    span.line,
                                    span.col,
                                    effect.requires.type_name()
                                ));
                            }
                            None => {
                                return Err(anyhow!(
                                    "Security Capability Violation: Function '{}' calls method-style effect '{}' at line {}, col {} without a capability receiver.",
                                    func.name,
                                    name,
                                    span.line,
                                    span.col
                                ));
                            }
                        }
                    }
                }

                // Interprocedural: capability parameters must be real handles from the caller
                // (not forged literals). Call graph integrity for DESIGN default-deny.
                if let Some(callee) = funcs.get(name) {
                    for (i, param) in callee.params.iter().enumerate() {
                        let kind = match param.param_type {
                            Type::NetCap => Some(CapKind::Net),
                            Type::FsCap => Some(CapKind::Fs),
                            Type::SysCap => Some(CapKind::Sys),
                            Type::EnvCap => Some(CapKind::Env),
                            _ => None,
                        };
                        if let Some(k) = kind {
                            let span = expr.span();
                            match args.get(i) {
                                Some(arg) if Self::expr_is_cap_handle(arg, k, func) => {}
                                Some(_) => {
                                    return Err(anyhow!(
                                        "Security Capability Violation: Function '{}' calls '{}' at line {}, col {} but argument {} is not a live {} handle from the caller's parameter list (capability forgery denied).",
                                        func.name,
                                        name,
                                        span.line,
                                        span.col,
                                        i,
                                        k.type_name()
                                    ));
                                }
                                None => {
                                    return Err(anyhow!(
                                        "Security Capability Violation: Function '{}' calls '{}' at line {}, col {} missing required {} argument.",
                                        func.name,
                                        name,
                                        span.line,
                                        span.col,
                                        k.type_name()
                                    ));
                                }
                            }
                        }
                    }
                }

                for arg in args {
                    Self::check_expr(arg, func, funcs)?;
                }
            }
            Expression::Binary { left, right, .. } => {
                Self::check_expr(left, func, funcs)?;
                Self::check_expr(right, func, funcs)?;
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::check_expr(cond, func, funcs)?;
                Self::check_block(then_branch, func, funcs)?;
                if let Some(else_b) = else_branch {
                    Self::check_block(else_b, func, funcs)?;
                }
            }
            Expression::Match { expr, arms, .. } => {
                Self::check_expr(expr, func, funcs)?;
                for arm in arms {
                    Self::check_expr(&arm.body, func, funcs)?;
                }
            }
            Expression::Unary { expr, .. } => Self::check_expr(expr, func, funcs)?,
            Expression::While { cond, body, .. } => {
                Self::check_expr(cond, func, funcs)?;
                Self::check_block(body, func, funcs)?;
            }
            Expression::Literal(_, _) | Expression::Variable(_, _) => {}
        }
        Ok(())
    }

    /// Cap handle is either a parameter of the right type, or a variable known to be that param.
    fn expr_is_cap_handle(expr: &Expression, kind: CapKind, func: &FunctionDecl) -> bool {
        match expr {
            Expression::Variable(name, _) => func
                .params
                .iter()
                .any(|p| p.name == *name && kind.matches_type(&p.param_type)),
            // Free-function style: fetch(url) with net: &NetCap on the function —
            // no cap in args; treat as ambient grant (already checked function_has_cap).
            // When cap_arg_index points past available design, callers skip this.
            _ => {
                // Non-variable receivers are not trusted as forged caps.
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_program(src: &str) -> Program {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = crate::parser::Parser::new(tokens);
        parser.parse_program().expect("parse")
    }

    #[test]
    fn denies_fetch_without_netcap() {
        let prog = parse_program(
            r#"
            pub fn rogue() {
                let res = fetch("https://evil.example");
            }
        "#,
        );
        assert!(CapabilityChecker::check_program(&prog).is_err());
    }

    #[test]
    fn allows_fetch_with_netcap() {
        let prog = parse_program(
            r#"
            pub fn ok(net: &NetCap, url: String) {
                let res = fetch(url);
            }
        "#,
        );
        assert!(CapabilityChecker::check_program(&prog).is_ok());
    }

    #[test]
    fn unknown_name_is_not_ambient_io() {
        // network_read is not a sealed effectful builtin — cannot invent I/O by renaming.
        let prog = parse_program(
            r#"
            pub fn steal() {
                let x = network_read("https://evil.com");
            }
        "#,
        );
        // Capability check passes (no sealed effect); runtime will reject undefined.
        assert!(CapabilityChecker::check_program(&prog).is_ok());
        assert!(lookup_effect("network_read").is_none());
    }

    #[test]
    fn method_write_file_requires_fscap() {
        let prog = parse_program(
            r#"
            pub fn bad(msg: String) {
                fs.write_file("app.log", msg);
            }
        "#,
        );
        // fs is a variable, .write_file is sealed Fs effect
        assert!(CapabilityChecker::check_program(&prog).is_err());
    }

    #[test]
    fn method_write_file_with_fscap_ok() {
        let prog = parse_program(
            r#"
            pub fn log_event(fs: &FsCap, message: String) {
                fs.write_file("app.log", message);
            }
        "#,
        );
        assert!(CapabilityChecker::check_program(&prog).is_ok());
    }
}
