// ===================================================================
// CHS → C backend (native stage-1 path without clang).
// Emits ISO C99 + runtime/chs_rt.c, linked with gcc.
// ===================================================================
use crate::ast::*;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct CCodeGen;

/// Host FFI builtins that require linking `libooda.a` (stage-0 staticlib).
/// Pure CHS programs never call these — they must not force Cargo/staticlib.
const HOST_FFI_CALLS: &[&str] = &[
    "chs_build",
    "host_ast_dump",
    "host_check",
    "host_token_dump",
];

/// Sealed effectful builtins that CHS C + `chs_rt.c` actually lower.
///
/// **Honesty:** C erases capability tokens at `main` (compile-time cap check
/// via typecheck/capability stays). Interpreter still runtime-gates tokens.
/// Anything outside this set must fail-closed on C/native (no silent `oo_fetch`).
/// Free and method forms (`.name`) both listed — `collect_sealed` records either.
const C_LOWERED_SEALED: &[&str] = &[
    "read_file",
    "write_file",
    "fs_read",
    "fs_write",
    ".read_file",
    ".write_file",
    "path_exists",
    "fs_exists",
    ".path_exists",
    "file_size",
    ".file_size",
    "env_get",
    ".env_get",
    "sys_exec",
    "system_exec",
    ".sys_exec",
];

/// True if the sealed effect name is lowered by the C backend + chs_rt.
pub fn c_backend_lowers_sealed(name: &str) -> bool {
    C_LOWERED_SEALED.iter().any(|n| *n == name)
}


/// Sealed effect names used in `program` that C does **not** lower (fail-closed).
pub fn sealed_effects_not_lowered_on_c(program: &Program) -> Vec<String> {
    crate::capabilities::collect_sealed_effect_names(program)
        .into_iter()
        .filter(|n| !c_backend_lowers_sealed(n))
        .collect()
}


/// True if any call in the program needs stage-0 host FFI (`libooda.a`).
pub fn program_needs_host_ffi(program: &Program) -> bool {
    for item in &program.items {
        if let Item::Function(f) = item {
            if block_needs_host_ffi(&f.body) {
                return true;
            }
        }
    }
    false
}


fn is_host_ffi_name(name: &str) -> bool {
    HOST_FFI_CALLS.iter().any(|n| *n == name)
}


fn block_needs_host_ffi(b: &Block) -> bool {
    b.stmts.iter().any(stmt_needs_host_ffi)
        || b.expr.as_deref().map_or(false, expr_needs_host_ffi)
}


fn stmt_needs_host_ffi(s: &Statement) -> bool {
    match s {
        Statement::Let { init, .. } => expr_needs_host_ffi(init),
        Statement::Assign { value, .. } => expr_needs_host_ffi(value),
        Statement::FieldAssign { object, value, .. } => {
            expr_needs_host_ffi(object) || expr_needs_host_ffi(value)
        }
        Statement::Return(Some(e), _) => expr_needs_host_ffi(e),
        Statement::Return(None, _) | Statement::Break(_) | Statement::Continue(_) => false,
        Statement::Expr(e, _) => expr_needs_host_ffi(e),
        Statement::While { cond, body, .. } => {
            expr_needs_host_ffi(cond) || block_needs_host_ffi(body)
        }
    }
}


fn expr_needs_host_ffi(e: &Expression) -> bool {
    match e {
        Expression::Call { name, args, .. } => {
            if is_host_ffi_name(name) {
                return true;
            }
            args.iter().any(expr_needs_host_ffi)
        }
        Expression::Binary { left, right, .. } => {
            expr_needs_host_ffi(left) || expr_needs_host_ffi(right)
        }
        Expression::Unary { expr, .. } => expr_needs_host_ffi(expr),
        Expression::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expr_needs_host_ffi(cond)
                || block_needs_host_ffi(then_branch)
                || else_branch
                    .as_ref()
                    .map_or(false, |b| block_needs_host_ffi(b))
        }
        Expression::While { cond, body, .. } => {
            expr_needs_host_ffi(cond) || block_needs_host_ffi(body)
        }
        Expression::Match { expr, arms, .. } => {
            expr_needs_host_ffi(expr) || arms.iter().any(|a| expr_needs_host_ffi(&a.body))
        }
        Expression::StructLit { fields, .. } => fields.iter().any(|(_, e)| expr_needs_host_ffi(e)),
        Expression::Literal(_, _) | Expression::Variable(_, _) => false,
    }
}


