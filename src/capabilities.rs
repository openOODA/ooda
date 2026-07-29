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
    EffectBuiltin {
        name: "mkdir_p",
        requires: CapKind::Fs,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: ".mkdir_p",
        requires: CapKind::Fs,
        receiver_is_cap: true,
    },
    EffectBuiltin {
        name: "copy_file",
        requires: CapKind::Fs,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: ".copy_file",
        requires: CapKind::Fs,
        receiver_is_cap: true,
    },
    EffectBuiltin {
        name: "chmod_exec",
        requires: CapKind::Fs,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: ".chmod_exec",
        requires: CapKind::Fs,
        receiver_is_cap: true,
    },
    EffectBuiltin {
        name: "path_exists",
        requires: CapKind::Fs,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: ".path_exists",
        requires: CapKind::Fs,
        receiver_is_cap: true,
    },
    EffectBuiltin {
        name: "file_size",
        requires: CapKind::Fs,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: ".file_size",
        requires: CapKind::Fs,
        receiver_is_cap: true,
    },
    EffectBuiltin {
        name: "http_download",
        requires: CapKind::Net,
        receiver_is_cap: false,
    },
    // Process / system
    EffectBuiltin {
        name: "sys_exec",
        requires: CapKind::Sys,
        receiver_is_cap: false,
    },
    EffectBuiltin {
        name: ".sys_exec",
        requires: CapKind::Sys,
        receiver_is_cap: true,
    },
    EffectBuiltin {
        name: "extract_tar_gz",
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

/// Collect sealed effectful builtin names used in a program (for dual-engine refuse).
pub fn collect_sealed_effect_names(program: &Program) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut found = BTreeSet::new();
    for item in &program.items {
        if let Item::Function(f) = item {
            collect_sealed_in_block(&f.body, &mut found);
            if let Some(v) = &f.verify_block {
                collect_sealed_in_block(v, &mut found);
            }
        }
    }
    found.into_iter().collect()
}

fn collect_sealed_in_block(block: &Block, found: &mut std::collections::BTreeSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Statement::Let { init, .. } => collect_sealed_in_expr(init, found),
            Statement::Assign { value, .. } => collect_sealed_in_expr(value, found),
            Statement::FieldAssign { object, value, .. } => {
                collect_sealed_in_expr(object, found);
                collect_sealed_in_expr(value, found);
            }
            Statement::Return(Some(e), _) | Statement::Expr(e, _) => {
                collect_sealed_in_expr(e, found)
            }
            Statement::While { cond, body, .. } => {
                collect_sealed_in_expr(cond, found);
                collect_sealed_in_block(body, found);
            }
            Statement::Return(None, _) => {}
        }
    }
    if let Some(e) = &block.expr {
        collect_sealed_in_expr(e, found);
    }
}

fn collect_sealed_in_expr(expr: &Expression, found: &mut std::collections::BTreeSet<String>) {
    match expr {
        Expression::Call { name, args, .. } => {
            if lookup_effect(name).is_some() {
                found.insert(name.clone());
            }
            for a in args {
                collect_sealed_in_expr(a, found);
            }
        }
        Expression::Binary { left, right, .. } => {
            collect_sealed_in_expr(left, found);
            collect_sealed_in_expr(right, found);
        }
        Expression::Unary { expr, .. } => collect_sealed_in_expr(expr, found),
        Expression::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_sealed_in_expr(cond, found);
            collect_sealed_in_block(then_branch, found);
            if let Some(eb) = else_branch {
                collect_sealed_in_block(eb, found);
            }
        }
        Expression::While { cond, body, .. } => {
            collect_sealed_in_expr(cond, found);
            collect_sealed_in_block(body, found);
        }
        Expression::Match { expr, arms, .. } => {
            collect_sealed_in_expr(expr, found);
            for arm in arms {
                collect_sealed_in_expr(&arm.body, found);
            }
        }
        Expression::StructLit { fields, .. } => {
            for (_, e) in fields {
                collect_sealed_in_expr(e, found);
            }
        }
        Expression::Literal(_, _) | Expression::Variable(_, _) => {}
    }
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
                Statement::FieldAssign { object, value, .. } => {
                    Self::check_expr(object, func, funcs)?;
                    Self::check_expr(value, func, funcs)?;
                }
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
                    let span = expr.span();
                    let has_correct_handle = args
                        .iter()
                        .any(|a| Self::expr_is_cap_handle(a, effect.requires, func));
                    let wrong_kind = [
                        CapKind::Net,
                        CapKind::Fs,
                        CapKind::Sys,
                        CapKind::Env,
                    ]
                    .into_iter()
                    .find(|&k| {
                        k != effect.requires
                            && args
                                .iter()
                                .any(|a| Self::expr_is_cap_handle(a, k, func))
                    });

                    // Method style: receiver (args[0]) must be a live cap handle.
                    if effect.receiver_is_cap {
                        match args.first() {
                            Some(recv) if Self::expr_is_cap_handle(recv, effect.requires, func) => {}
                            Some(recv) => {
                                // Prefer wrong-kind naming when receiver is a different cap.
                                if let Some(got) = [
                                    CapKind::Net,
                                    CapKind::Fs,
                                    CapKind::Sys,
                                    CapKind::Env,
                                ]
                                .into_iter()
                                .find(|&k| {
                                    k != effect.requires
                                        && Self::expr_is_cap_handle(recv, k, func)
                                })
                                {
                                    return Err(anyhow!(
                                        "Security Capability Violation: Function '{}' calls '{}' at line {}, col {} with wrong-kind handle {} (requires live {} — object-capability: kinds are not interchangeable).",
                                        func.name,
                                        name,
                                        span.line,
                                        span.col,
                                        got.type_name(),
                                        effect.requires.type_name()
                                    ));
                                }
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
                    } else if has_correct_handle {
                        // Free sealed form with live correct handle — ok (even if ambient also present).
                    } else if let Some(got) = wrong_kind {
                        // Wrong-kind before ambient-missing: write_file(net, …) with only &NetCap.
                        return Err(anyhow!(
                            "Security Capability Violation: Function '{}' calls sealed '{}' at line {}, col {} with wrong-kind handle {} (requires live {} — object-capability: kinds are not interchangeable).",
                            func.name,
                            name,
                            span.line,
                            span.col,
                            got.type_name(),
                            effect.requires.type_name()
                        ));
                    } else if !Self::function_has_cap(func, effect.requires) {
                        return Err(anyhow!(
                            "Security Capability Violation: Function '{}' calls sealed effectful builtin '{}' which requires a {} parameter, but none was declared at line {}, col {}. Default-deny: grant the capability token explicitly.",
                            func.name,
                            name,
                            effect.requires.type_name(),
                            span.line,
                            span.col
                        ));
                    } else {
                        // Ambient grant alone is not enough — must thread live handle.
                        return Err(anyhow!(
                            "Security Capability Violation: Function '{}' calls sealed '{}' at line {}, col {} without passing a live {} handle argument (object-capability: ambient grant alone is not enough — use `{}(cap, …)` or a method-style receiver).",
                            func.name,
                            name,
                            span.line,
                            span.col,
                            effect.requires.type_name(),
                            name
                        ));
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
            Expression::StructLit { fields, .. } => {
                for (_, fexpr) in fields {
                    Self::check_expr(fexpr, func, funcs)?;
                }
            }
            Expression::Literal(_, _) | Expression::Variable(_, _) => {}
        }
        Ok(())
    }

    /// Cap handle is a parameter of the right type, or a variable that aliases one
    /// via `let` / assign — including aliases inside nested `if`/`while`/`match`.
    fn expr_is_cap_handle(expr: &Expression, kind: CapKind, func: &FunctionDecl) -> bool {
        match expr {
            Expression::Variable(name, _) => {
                Self::cap_handle_names(func, kind).contains(name)
            }
            _ => false,
        }
    }

    /// Fixed-point set of names that are live capability handles of `kind` in `func`.
    fn cap_handle_names(func: &FunctionDecl, kind: CapKind) -> std::collections::HashSet<String> {
        use std::collections::HashSet;
        let mut handles = HashSet::new();
        for p in &func.params {
            if kind.matches_type(&p.param_type) {
                handles.insert(p.name.clone());
            }
        }
        // Fixed-point so chains (`let a = fs; let b = a;`) and nested blocks converge.
        for _ in 0..64 {
            let before = handles.len();
            Self::collect_cap_aliases_in_block(&func.body, &mut handles);
            if handles.len() == before {
                break;
            }
        }
        handles
    }

    fn collect_cap_aliases_in_block(
        block: &Block,
        handles: &mut std::collections::HashSet<String>,
    ) {
        for stmt in &block.stmts {
            match stmt {
                Statement::Let { name, init, .. } => {
                    if let Expression::Variable(init_name, _) = init {
                        if handles.contains(init_name) {
                            handles.insert(name.clone());
                        }
                    }
                    Self::collect_cap_aliases_in_expr(init, handles);
                }
                Statement::Assign { name, value, .. } => {
                    if let Expression::Variable(val_name, _) = value {
                        if handles.contains(val_name) {
                            handles.insert(name.clone());
                        }
                    }
                    Self::collect_cap_aliases_in_expr(value, handles);
                }
                Statement::FieldAssign { object, value, .. } => {
                    Self::collect_cap_aliases_in_expr(object, handles);
                    Self::collect_cap_aliases_in_expr(value, handles);
                }
                Statement::Return(Some(e), _) | Statement::Expr(e, _) => {
                    Self::collect_cap_aliases_in_expr(e, handles);
                }
                Statement::While { cond, body, .. } => {
                    Self::collect_cap_aliases_in_expr(cond, handles);
                    Self::collect_cap_aliases_in_block(body, handles);
                }
                Statement::Return(None, _) => {}
            }
        }
        if let Some(expr) = &block.expr {
            Self::collect_cap_aliases_in_expr(expr, handles);
        }
    }

    fn collect_cap_aliases_in_expr(
        expr: &Expression,
        handles: &mut std::collections::HashSet<String>,
    ) {
        match expr {
            Expression::Literal(_, _) | Expression::Variable(_, _) => {}
            Expression::Binary { left, right, .. } => {
                Self::collect_cap_aliases_in_expr(left, handles);
                Self::collect_cap_aliases_in_expr(right, handles);
            }
            Expression::Unary { expr, .. } => Self::collect_cap_aliases_in_expr(expr, handles),
            Expression::Call { args, .. } => {
                for a in args {
                    Self::collect_cap_aliases_in_expr(a, handles);
                }
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::collect_cap_aliases_in_expr(cond, handles);
                Self::collect_cap_aliases_in_block(then_branch, handles);
                if let Some(eb) = else_branch {
                    Self::collect_cap_aliases_in_block(eb, handles);
                }
            }
            Expression::While { cond, body, .. } => {
                Self::collect_cap_aliases_in_expr(cond, handles);
                Self::collect_cap_aliases_in_block(body, handles);
            }
            Expression::Match { expr, arms, .. } => {
                // Pattern-trace: `match Some(cap) { Some(h) => … }` — bind `h` as handle.
                let scrutinee_handle = match expr.as_ref() {
                    Expression::Variable(v, _) if handles.contains(v) => Some(v.clone()),
                    Expression::Call { name, args, .. }
                        if (name == "Some" || name == "Ok") && args.len() == 1 =>
                    {
                        if let Expression::Variable(v, _) = &args[0] {
                            if handles.contains(v) {
                                Some(v.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                Self::collect_cap_aliases_in_expr(expr, handles);
                for arm in arms {
                    if let (Some(_), Pattern::Variant { arg: Some(bind), .. }) =
                        (&scrutinee_handle, &arm.pattern)
                    {
                        handles.insert(bind.clone());
                    }
                    Self::collect_cap_aliases_in_expr(&arm.body, handles);
                }
            }
            Expression::StructLit { fields, .. } => {
                for (_, e) in fields {
                    Self::collect_cap_aliases_in_expr(e, handles);
                }
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
    fn method_forms_are_sealed_for_dual_engine() {
        // Method-style FS/Sys/Env must appear in collect_sealed so build refuses.
        let prog = parse_program(
            r#"
            pub fn fs_m(fs: &FsCap) {
                let _ = fs.path_exists("/tmp");
                let _ = fs.file_size("/tmp/x");
                let _ = fs.mkdir_p("/tmp/ooda_test_dir");
            }
            pub fn sys_m(sys: &SysCap) {
                let _ = sys.sys_exec("true");
            }
            pub fn env_m(env: &EnvCap) {
                let _ = env.env_get("PATH");
            }
        "#,
        );
        let sealed = collect_sealed_effect_names(&prog);
        for need in [
            ".path_exists",
            ".file_size",
            ".sys_exec",
            ".env_get",
            ".mkdir_p",
        ] {
            assert!(
                sealed.iter().any(|s| s == need),
                "missing sealed method {} in {:?}",
                need,
                sealed
            );
        }
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "with live receivers, method forms must typecheck caps: {:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }

    #[test]
    fn denies_path_exists_method_without_fscap_receiver() {
        let prog = parse_program(
            r#"
            pub fn rogue() {
                let b = path_exists("/tmp");
            }
        "#,
        );
        // free path_exists without handle
        assert!(CapabilityChecker::check_program(&prog).is_err());
        let prog2 = parse_program(
            r#"
            pub fn rogue(net: &NetCap) {
                let b = net.path_exists("/tmp");
            }
        "#,
        );
        let err = CapabilityChecker::check_program(&prog2).unwrap_err().to_string();
        assert!(
            err.contains("wrong-kind")
                || err.contains("not a")
                || err.contains("FsCap")
                || err.contains("capability"),
            "wrong receiver kind: {}",
            err
        );
    }

    #[test]
    fn allows_fetch_with_netcap() {
        let prog = parse_program(
            r#"
            pub fn ok(net: &NetCap, url: String) {
                let res = fetch(net, url);
            }
        "#,
        );
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "{:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }

    #[test]
    fn denies_ambient_only_fetch_without_handle_arg() {
        // Object-cap: declaring &NetCap is not enough — must pass the handle.
        let prog = parse_program(
            r#"
            pub fn ambient(net: &NetCap, url: String) {
                let res = fetch(url);
            }
        "#,
        );
        let err = CapabilityChecker::check_program(&prog).unwrap_err().to_string();
        assert!(
            err.contains("object-capability") || err.contains("live"),
            "ambient-only fetch must fail: {}",
            err
        );
    }

    #[test]
    fn denies_wrong_kind_handle_for_write_file() {
        // NetCap is live but wrong kind for Fs sealed write_file.
        let prog = parse_program(
            r#"
            pub fn mix(net: &NetCap, fs: &FsCap) {
                let r = write_file(net, "/tmp/x", "y");
            }
        "#,
        );
        let err = CapabilityChecker::check_program(&prog).unwrap_err().to_string();
        assert!(
            err.contains("wrong-kind")
                || err.contains("object-capability")
                || err.contains("live")
                || err.contains("FsCap")
                || err.contains("write_file"),
            "wrong-kind handle must fail: {}",
            err
        );
        assert!(
            err.contains("wrong-kind") && err.contains("NetCap") && err.contains("FsCap"),
            "must name both kinds: {}",
            err
        );
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
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "receiver fs param must be accepted: {:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }

    #[test]
    fn allows_assign_re_aliased_capability_handle() {
        let prog = parse_program(
            r#"
            pub fn main(fs: &FsCap) {
                let mut fs_var = fs;
                fs_var = fs;
                fs_var.write_file("note.txt", "hello");
            }
            "#,
        );
        assert!(CapabilityChecker::check_program(&prog).is_ok());
    }

    #[test]
    fn allows_nested_if_let_aliased_capability_handle() {
        let prog = parse_program(
            r#"
            pub fn main(fs: &FsCap) {
                if true {
                    let fs2 = fs;
                    fs2.write_file("note.txt", "hello");
                }
            }
            "#,
        );
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "nested let-alias of FsCap must be accepted: {:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }

    #[test]
    fn allows_match_some_pattern_capability_handle() {
        let prog = parse_program(
            r#"
            pub fn main(fs: &FsCap) {
                match Some(fs) {
                    Some(h) => h.write_file("note.txt", "hello"),
                    None => process_exit(1),
                }
            }
            "#,
        );
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "match Some(cap) pattern bind must be a handle: {:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }

    #[test]
    fn wrong_kind_write_file_net_only_names_kinds() {
        let src = r#"
            pub fn main(net: &NetCap) {
                let r = write_file(net, "/tmp/x", "y");
                println(r);
            }
        "#;
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = crate::parser::Parser::new(tokens);
        let prog = parser.parse_program().expect("parse");
        let err = CapabilityChecker::check_program(&prog).unwrap_err().to_string();
        assert!(
            err.contains("wrong-kind") && err.contains("NetCap") && err.contains("FsCap"),
            "write_file(net) without FsCap must wrong-kind: {}",
            err
        );
    }
}
