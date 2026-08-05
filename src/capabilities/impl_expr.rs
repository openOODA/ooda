use crate::ast::*;
use anyhow::{anyhow, Result};
use super::*;
impl super::CapabilityChecker {
    pub(crate) fn check_expr(
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
}
