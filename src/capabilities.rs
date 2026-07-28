// ===================================================================
// openOODA AST-Based Type-State Capability Security Checker
// Enforces default-deny access control across function call graphs
// ===================================================================
use crate::ast::*;
use anyhow::{anyhow, Result};

pub struct CapabilityChecker;

impl CapabilityChecker {
    pub fn check_program(program: &Program) -> Result<()> {
        for item in &program.items {
            if let Item::Function(func) = item {
                Self::check_function(func)?;
            }
        }
        Ok(())
    }

    fn check_function(func: &FunctionDecl) -> Result<()> {
        let has_net = func.params.iter().any(|p| matches!(p.param_type, Type::NetCap));
        let has_fs = func.params.iter().any(|p| matches!(p.param_type, Type::FsCap));
        let has_sys = func.params.iter().any(|p| matches!(p.param_type, Type::SysCap));

        Self::check_block(&func.body, func, has_net, has_fs, has_sys)
    }

    fn check_block(
        block: &Block,
        func: &FunctionDecl,
        has_net: bool,
        has_fs: bool,
        has_sys: bool,
    ) -> Result<()> {
        for stmt in &block.stmts {
            match stmt {
                Statement::Let { init, .. } => {
                    Self::check_expr(init, func, has_net, has_fs, has_sys)?;
                }
                Statement::Return(Some(expr)) => {
                    Self::check_expr(expr, func, has_net, has_fs, has_sys)?;
                }
                Statement::Expr(expr) => {
                    Self::check_expr(expr, func, has_net, has_fs, has_sys)?;
                }
                _ => {}
            }
        }

        if let Some(expr) = &block.expr {
            Self::check_expr(expr, func, has_net, has_fs, has_sys)?;
        }

        Ok(())
    }

    fn is_net_io(name: &str) -> bool {
        let s = name.to_lowercase();
        s.contains("http") || s.contains("fetch") || s.contains("downloaddata") || s.contains("exfiltrate") || s.contains("query_remote_api") || s.contains("net_connect")
    }

    fn is_fs_io(name: &str) -> bool {
        let s = name.to_lowercase();
        s.contains("read_file") || s.contains("write_file") || s.contains("fs_read") || s.contains("fs_write") || s.contains("fs.read_file")
    }

    fn is_sys_io(name: &str) -> bool {
        let s = name.to_lowercase();
        s.contains("exec") || s.contains("spawn") || s.contains("async_spawn") || s.contains("sys_exec")
    }

    fn check_expr(
        expr: &Expression,
        func: &FunctionDecl,
        has_net: bool,
        has_fs: bool,
        has_sys: bool,
    ) -> Result<()> {
        match expr {
            Expression::Call { name, args, .. } => {
                if Self::is_net_io(name) && !has_net {
                    return Err(anyhow!(
                        "Security Capability Violation: Function '{}' attempts unauthorized network access via '{}' without receiving a '&NetCap' capability handle.",
                        func.name,
                        name
                    ));
                }

                if Self::is_fs_io(name) && !has_fs {
                    return Err(anyhow!(
                        "Security Capability Violation: Function '{}' attempts unauthorized file system access via '{}' without receiving a '&FsCap' capability handle.",
                        func.name,
                        name
                    ));
                }

                if Self::is_sys_io(name) && !has_sys {
                    return Err(anyhow!(
                        "Security Capability Violation: Function '{}' attempts unauthorized system process access via '{}' without receiving a '&SysCap' capability handle.",
                        func.name,
                        name
                    ));
                }

                for arg in args {
                    Self::check_expr(arg, func, has_net, has_fs, has_sys)?;
                }
            }
            Expression::Binary { left, right, .. } => {
                Self::check_expr(left, func, has_net, has_fs, has_sys)?;
                Self::check_expr(right, func, has_net, has_fs, has_sys)?;
            }
            Expression::If { cond, then_branch, else_branch } => {
                Self::check_expr(cond, func, has_net, has_fs, has_sys)?;
                Self::check_block(then_branch, func, has_net, has_fs, has_sys)?;
                if let Some(else_b) = else_branch {
                    Self::check_block(else_b, func, has_net, has_fs, has_sys)?;
                }
            }
            Expression::Match { expr, arms } => {
                Self::check_expr(expr, func, has_net, has_fs, has_sys)?;
                for arm in arms {
                    Self::check_expr(&arm.body, func, has_net, has_fs, has_sys)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
