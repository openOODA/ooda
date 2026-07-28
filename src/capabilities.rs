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

    fn check_expr(
        expr: &Expression,
        func: &FunctionDecl,
        has_net: bool,
        has_fs: bool,
        has_sys: bool,
    ) -> Result<()> {
        match expr {
            Expression::Call { name, args, .. } => {
                if name.contains("get") || name.contains("fetch") || name.contains("http") {
                    if !has_net {
                        return Err(anyhow!(
                            "Security Capability Violation: Function '{}' attempts network access via '{}' without receiving a '&NetCap' capability token.",
                            func.name,
                            name
                        ));
                    }
                }

                if name.contains("write_file") || name.contains("read_file") {
                    if !has_fs {
                        return Err(anyhow!(
                            "Security Capability Violation: Function '{}' attempts file system access via '{}' without receiving a '&FsCap' capability token.",
                            func.name,
                            name
                        ));
                    }
                }

                if name.contains("exec") || name.contains("spawn") {
                    if !has_sys {
                        return Err(anyhow!(
                            "Security Capability Violation: Function '{}' attempts system process access via '{}' without receiving a '&SysCap' capability token.",
                            func.name,
                            name
                        ));
                    }
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
