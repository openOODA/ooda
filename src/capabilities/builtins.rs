pub const EFFECT_BUILTINS: &[EffectBuiltin] = &[
    // Network (free functions: ambient cap on the enclosing fn)
    EffectBuiltin { name: "fetch", requires: CapKind::Net, receiver_is_cap: false, },
    EffectBuiltin { name: "downloadData", requires: CapKind::Net, receiver_is_cap: false, },
    EffectBuiltin { name: "http_get", requires: CapKind::Net, receiver_is_cap: false, },
    EffectBuiltin { name: "net_get", requires: CapKind::Net, receiver_is_cap: false, },
    EffectBuiltin { name: "net_connect", requires: CapKind::Net, receiver_is_cap: false, },
    EffectBuiltin { name: "query_remote_api", requires: CapKind::Net, receiver_is_cap: false, },
    EffectBuiltin { name: ".get", requires: CapKind::Net, receiver_is_cap: true, },
    // Filesystem
    EffectBuiltin { name: "read_file", requires: CapKind::Fs, receiver_is_cap: false, },
    EffectBuiltin { name: "write_file", requires: CapKind::Fs, receiver_is_cap: false, },
    EffectBuiltin { name: "fs_read", requires: CapKind::Fs, receiver_is_cap: false, },
    EffectBuiltin { name: "fs_write", requires: CapKind::Fs, receiver_is_cap: false, },
    EffectBuiltin { name: ".read_file", requires: CapKind::Fs, receiver_is_cap: true, },
    EffectBuiltin { name: ".write_file", requires: CapKind::Fs, receiver_is_cap: true, },
    EffectBuiltin { name: "mkdir_p", requires: CapKind::Fs, receiver_is_cap: false, },
    EffectBuiltin { name: ".mkdir_p", requires: CapKind::Fs, receiver_is_cap: true, },
    EffectBuiltin { name: "copy_file", requires: CapKind::Fs, receiver_is_cap: false, },
    EffectBuiltin { name: ".copy_file", requires: CapKind::Fs, receiver_is_cap: true, },
    EffectBuiltin { name: "chmod_exec", requires: CapKind::Fs, receiver_is_cap: false, },
    EffectBuiltin { name: ".chmod_exec", requires: CapKind::Fs, receiver_is_cap: true, },
    EffectBuiltin { name: "path_exists", requires: CapKind::Fs, receiver_is_cap: false, },
    EffectBuiltin { name: ".path_exists", requires: CapKind::Fs, receiver_is_cap: true, },
    EffectBuiltin { name: "file_size", requires: CapKind::Fs, receiver_is_cap: false, },
    EffectBuiltin { name: ".file_size", requires: CapKind::Fs, receiver_is_cap: true, },
    EffectBuiltin { name: "http_download", requires: CapKind::Net, receiver_is_cap: false, },
    // Process / system
    EffectBuiltin { name: "sys_exec", requires: CapKind::Sys, receiver_is_cap: false, },
    EffectBuiltin { name: ".sys_exec", requires: CapKind::Sys, receiver_is_cap: true, },
    EffectBuiltin { name: "extract_tar_gz", requires: CapKind::Sys, receiver_is_cap: false, },
    EffectBuiltin { name: "exec", requires: CapKind::Sys, receiver_is_cap: false, },
    EffectBuiltin { name: "spawn_process", requires: CapKind::Sys, receiver_is_cap: false, },
    // Environment
    EffectBuiltin { name: "env_get", requires: CapKind::Env, receiver_is_cap: false, },
    EffectBuiltin { name: "env_set", requires: CapKind::Env, receiver_is_cap: false, },
    EffectBuiltin { name: ".env_set", requires: CapKind::Env, receiver_is_cap: true, },
    EffectBuiltin { name: ".env_get", requires: CapKind::Env, receiver_is_cap: true, },
    // Sealed stdlib internals (callable from .oo but require a SysCap because
    // they spawn threads or invoke out-of-process runtimes).
    EffectBuiltin { name: "async_spawn_internal", requires: CapKind::Sys, receiver_is_cap: false, },
    EffectBuiltin { name: "async_join_internal", requires: CapKind::Sys, receiver_is_cap: false, },
    EffectBuiltin { name: "python_embed_internal", requires: CapKind::Sys, receiver_is_cap: false, },
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
            Statement::Break(_) | Statement::Continue(_) => {}
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
