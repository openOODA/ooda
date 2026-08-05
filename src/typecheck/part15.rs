
/// True when every control-flow path through `block` executes `return`.
/// Expression-bodied blocks (tail value) are NOT "returns" — they yield a value.
/// An early unconditional `return` makes the rest of the block dead (still returns).
/// Conservative: unknown constructs → false.
fn block_always_returns(block: &Block) -> bool {
    for stmt in &block.stmts {
        match stmt {
            Statement::Return(_, _) | Statement::Break(_) | Statement::Continue(_) => return true,
            Statement::Expr(e, _) if expr_paths_return(e) => return true,
            // Other statements may fall through.
            _ => {}
        }
    }
    if let Some(expr) = &block.expr {
        return expr_paths_return(expr);
    }
    false
}


fn expr_paths_return(expr: &Expression) -> bool {
    match expr {
        Expression::If {
            then_branch,
            else_branch,
            ..
        } => {
            block_always_returns(then_branch)
                && else_branch
                    .as_ref()
                    .map(|b| block_always_returns(b))
                    .unwrap_or(false)
        }
        Expression::Match { arms, .. } => {
            !arms.is_empty() && arms.iter().all(|arm| expr_paths_return(&arm.body))
        }
        // Bare values / calls are not return statements.
        _ => false,
    }
}


/// Span of a statement for unreachable-code diagnostics.
fn stmt_span(stmt: &Statement) -> crate::ast::Span {
    match stmt {
        Statement::Let { span, .. }
        | Statement::Assign { span, .. }
        | Statement::FieldAssign { span, .. }
        | Statement::Return(_, span)
        | Statement::Break(span)
        | Statement::Continue(span)
        | Statement::Expr(_, span)
        | Statement::While { span, .. } => *span,
    }
}



/// Names of functions that use `?` (try) — not lowered outside the interpreter yet.
pub fn program_uses_try_operator(program: &Program) -> bool {
    fn expr_has_try(e: &Expression) -> bool {
        match e {
            Expression::Call { propagate_err, args, .. } => {
                *propagate_err || args.iter().any(expr_has_try)
            }
            Expression::Binary { left, right, .. } => expr_has_try(left) || expr_has_try(right),
            Expression::Unary { expr, .. } => expr_has_try(expr),
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                expr_has_try(cond)
                    || block_has_try(then_branch)
                    || else_branch.as_ref().map(|b| block_has_try(b)).unwrap_or(false)
            }
            Expression::While { cond, body, .. } => expr_has_try(cond) || block_has_try(body),
            Expression::Match { expr, arms, .. } => {
                expr_has_try(expr) || arms.iter().any(|a| expr_has_try(&a.body))
            }
            Expression::StructLit { fields, .. } => fields.iter().any(|(_, e)| expr_has_try(e)),
            Expression::Literal(_, _) | Expression::Variable(_, _) => false,
        }
    }
    fn block_has_try(b: &Block) -> bool {
        b.stmts.iter().any(|s| match s {
            Statement::Let { init, .. } => expr_has_try(init),
            Statement::Assign { value, .. } => expr_has_try(value),
            Statement::FieldAssign { object, value, .. } => {
                expr_has_try(object) || expr_has_try(value)
            }
            Statement::Return(Some(e), _) | Statement::Expr(e, _) => expr_has_try(e),
            Statement::Return(None, _) | Statement::Break(_) | Statement::Continue(_) => false,
            Statement::While { cond, body, .. } => expr_has_try(cond) || block_has_try(body),
        }) || b.expr.as_ref().map(|e| expr_has_try(e)).unwrap_or(false)
    }
    for item in &program.items {
        if let Item::Function(f) = item {
            if block_has_try(&f.body) {
                return true;
            }
            if let Some(v) = &f.verify_block {
                if block_has_try(v) {
                    return true;
                }
            }
        }
    }
    false
}

