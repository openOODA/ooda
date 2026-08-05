
/// Codemod #2: `let x` that is later assigned → `let mut x`.
fn collect_let_mut_rewrites(
    block: &Block,
    source: &str,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    let mut assigned: HashSet<String> = HashSet::new();
    collect_assigned_names(block, &mut assigned);
    collect_immutable_lets_needing_mut(block, source, &assigned, rewrites);
}

fn collect_assigned_names(block: &Block, assigned: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Statement::Assign { name, value, .. } => {
                assigned.insert(name.clone());
                collect_assigned_in_expr(value, assigned);
            }
            Statement::FieldAssign { object, value, .. } => {
                if let Expression::Variable(n, _) = object {
                    assigned.insert(n.clone());
                }
                collect_assigned_in_expr(object, assigned);
                collect_assigned_in_expr(value, assigned);
            }
            Statement::Let { init, .. } => collect_assigned_in_expr(init, assigned),
            Statement::Return(Some(e), _) | Statement::Expr(e, _) => {
                collect_assigned_in_expr(e, assigned)
            }
            Statement::Return(None, _) => {}
            Statement::Break(_) | Statement::Continue(_) => {}
            Statement::While { cond, body, .. } => {
                collect_assigned_in_expr(cond, assigned);
                collect_assigned_names(body, assigned);
            }
        }
    }
    if let Some(expr) = &block.expr {
        collect_assigned_in_expr(expr, assigned);
    }
}

fn collect_assigned_in_expr(expr: &Expression, assigned: &mut HashSet<String>) {
    match expr {
        Expression::Literal(_, _) | Expression::Variable(_, _) => {}
        Expression::Binary { left, right, .. } => {
            collect_assigned_in_expr(left, assigned);
            collect_assigned_in_expr(right, assigned);
        }
        Expression::Call { args, .. } => {
            for a in args {
                collect_assigned_in_expr(a, assigned);
            }
        }
        Expression::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_assigned_in_expr(cond, assigned);
            collect_assigned_names(then_branch, assigned);
            if let Some(eb) = else_branch {
                collect_assigned_names(eb, assigned);
            }
        }
        Expression::Unary { expr, .. } => collect_assigned_in_expr(expr, assigned),
        Expression::While { cond, body, .. } => {
            collect_assigned_in_expr(cond, assigned);
            collect_assigned_names(body, assigned);
        }
        Expression::Match { expr, arms, .. } => {
            collect_assigned_in_expr(expr, assigned);
            for arm in arms {
                collect_assigned_in_expr(&arm.body, assigned);
            }
        }
        Expression::StructLit { fields, .. } => {
            for (_, e) in fields {
                collect_assigned_in_expr(e, assigned);
            }
        }
    }
}

fn collect_immutable_lets_needing_mut(
    block: &Block,
    source: &str,
    assigned: &HashSet<String>,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    for stmt in &block.stmts {
        match stmt {
            Statement::FieldAssign { .. } => {
                // Field assigns do not rewrite `let` → `let mut` here.
            }
            Statement::Let {
                name,
                mutable,
                span,
                init,
                ..
            } => {
                if !*mutable && assigned.contains(name) {
                    if let Some(pos) = find_let_kw_byte(source, span.line, span.col) {
                        let rest = &source[pos..];
                        if rest.starts_with("let ") && !rest.starts_with("let mut ") {
                            let insert_at = pos + 4; // after "let "
                            rewrites.push((insert_at, insert_at, "mut ".to_string()));
                        }
                    }
                }
                collect_immutable_lets_in_expr(init, source, assigned, rewrites);
            }
            Statement::Assign { value, .. } => {
                collect_immutable_lets_in_expr(value, source, assigned, rewrites)
            }
            Statement::Return(Some(e), _) | Statement::Expr(e, _) => {
                collect_immutable_lets_in_expr(e, source, assigned, rewrites)
            }
            Statement::Return(None, _) => {}
            Statement::Break(_) | Statement::Continue(_) => {}
            Statement::While { cond, body, .. } => {
                collect_immutable_lets_in_expr(cond, source, assigned, rewrites);
                collect_immutable_lets_needing_mut(body, source, assigned, rewrites);
            }
        }
    }
    if let Some(expr) = &block.expr {
        collect_immutable_lets_in_expr(expr, source, assigned, rewrites);
    }
}

fn collect_immutable_lets_in_expr(
    expr: &Expression,
    source: &str,
    assigned: &HashSet<String>,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    match expr {
        Expression::Literal(_, _) | Expression::Variable(_, _) => {}
        Expression::Binary { left, right, .. } => {
            collect_immutable_lets_in_expr(left, source, assigned, rewrites);
            collect_immutable_lets_in_expr(right, source, assigned, rewrites);
        }
        Expression::Call { args, .. } => {
            for a in args {
                collect_immutable_lets_in_expr(a, source, assigned, rewrites);
            }
        }
        Expression::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_immutable_lets_in_expr(cond, source, assigned, rewrites);
            collect_immutable_lets_needing_mut(then_branch, source, assigned, rewrites);
            if let Some(eb) = else_branch {
                collect_immutable_lets_needing_mut(eb, source, assigned, rewrites);
            }
        }
        Expression::Unary { expr, .. } => {
            collect_immutable_lets_in_expr(expr, source, assigned, rewrites)
        }
        Expression::While { cond, body, .. } => {
            collect_immutable_lets_in_expr(cond, source, assigned, rewrites);
            collect_immutable_lets_needing_mut(body, source, assigned, rewrites);
        }
        Expression::Match { expr, arms, .. } => {
            collect_immutable_lets_in_expr(expr, source, assigned, rewrites);
            for arm in arms {
                collect_immutable_lets_in_expr(&arm.body, source, assigned, rewrites);
            }
        }
        Expression::StructLit { fields, .. } => {
            for (_, e) in fields {
                collect_immutable_lets_in_expr(e, source, assigned, rewrites);
            }
        }
    }
}

/// Convert 1-indexed line/col span to a byte offset in `source`.
fn span_to_byte(source: &str, line: usize, col: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut idx = 0usize;
    for _ in 0..line.saturating_sub(1) {
        while idx < bytes.len() && bytes[idx] != b'\n' {
            idx += 1;
        }
        if idx < bytes.len() {
            idx += 1;
        }
    }
    idx += col.saturating_sub(1);
    if idx >= bytes.len() {
        None
    } else {
        Some(idx)
    }
}

/// Locate the `let ` keyword for a Let binding. Prefer the recorded span
/// (now the `let` token); fall back to a short scan on the same line for
/// older ASTs that pointed at `;`.
fn find_let_kw_byte(source: &str, line: usize, col: usize) -> Option<usize> {
    if let Some(pos) = span_to_byte(source, line, col) {
        if source[pos..].starts_with("let ") {
            return Some(pos);
        }
        // Scan backward within ~64 bytes for `let `.
        let start = pos.saturating_sub(64);
        if let Some(rel) = source[start..=pos].rfind("let ") {
            return Some(start + rel);
        }
        // Scan forward on the same line.
        let line_end = source[pos..]
            .find('\n')
            .map(|i| pos + i)
            .unwrap_or(source.len());
        if let Some(rel) = source[pos..line_end].find("let ") {
            return Some(pos + rel);
        }
    }
    None
}
