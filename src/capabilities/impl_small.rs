use crate::ast::*;
use anyhow::Result;
use super::*;
impl super::CapabilityChecker {
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
    pub(crate) fn function_has_cap(func: &FunctionDecl, kind: CapKind) -> bool {
        func.params.iter().any(|p| kind.matches_type(&p.param_type))
    }
    pub(crate) fn check_function(
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
    pub(crate) fn check_block(
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
            Statement::Break(_) | Statement::Continue(_) => {}
            }
        }
        if let Some(expr) = &block.expr {
            Self::check_expr(expr, func, funcs)?;
        }
        Ok(())
    }
    pub(crate) fn expr_is_cap_handle(expr: &Expression, kind: CapKind, func: &FunctionDecl) -> bool {
        match expr {
            Expression::Variable(name, _) => {
                Self::cap_handle_names(func, kind).contains(name)
            }
            _ => false,
        }
    }
    pub(crate) fn cap_handle_names(func: &FunctionDecl, kind: CapKind) -> std::collections::HashSet<String> {
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
    pub(crate) fn collect_cap_aliases_in_block(
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
            Statement::Break(_) | Statement::Continue(_) => {}
            }
        }
        if let Some(expr) = &block.expr {
            Self::collect_cap_aliases_in_expr(expr, handles);
        }
    }
    pub(crate) fn collect_cap_aliases_in_expr(
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
