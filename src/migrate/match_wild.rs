
/// Walk a block collecting rewrites for non-exhaustive match
/// expressions on Result/Option. Each rewrite is (pos, end,
/// replacement) — a half-open byte range to overwrite.
fn collect_match_rewrites(
    block: &Block,
    source: &str,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    for stmt in &block.stmts {
        match stmt {
            Statement::Let { init, .. } => {
                collect_in_expr(init, source, rewrites);
            }
            Statement::Assign { value, .. } => {
                collect_in_expr(value, source, rewrites);
            }
            Statement::FieldAssign { object, value, .. } => {
                collect_in_expr(object, source, rewrites);
                collect_in_expr(value, source, rewrites);
            }
            Statement::Return(Some(expr), _) => {
                collect_in_expr(expr, source, rewrites);
            }
            Statement::Return(None, _) => {}
            Statement::Break(_) | Statement::Continue(_) => {}
            Statement::Expr(expr, _) => {
                collect_in_expr(expr, source, rewrites);
            }
            Statement::While { cond, body, .. } => {
                collect_in_expr(cond, source, rewrites);
                collect_match_rewrites(body, source, rewrites);
            }
        }
    }
    if let Some(expr) = &block.expr {
        collect_in_expr(expr, source, rewrites);
    }
}

fn collect_in_expr(
    expr: &Expression,
    source: &str,
    rewrites: &mut Vec<(usize, usize, String)>,
) {
    match expr {
        Expression::Literal(_, _) | Expression::Variable(_, _) => {}
        Expression::Binary { left, right, .. } => {
            collect_in_expr(left, source, rewrites);
            collect_in_expr(right, source, rewrites);
        }
        Expression::Call { args, .. } => {
            for a in args {
                collect_in_expr(a, source, rewrites);
            }
        }
        Expression::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_in_expr(cond, source, rewrites);
            collect_match_rewrites(then_branch, source, rewrites);
            if let Some(eb) = else_branch {
                collect_match_rewrites(eb, source, rewrites);
            }
        }
        Expression::Unary { expr, .. } => {
            collect_in_expr(expr, source, rewrites);
        }
        Expression::While { cond, body, .. } => {
            collect_in_expr(cond, source, rewrites);
            collect_match_rewrites(body, source, rewrites);
        }
        Expression::Match { expr, arms, span } => {
            // Recurse into the scrutinee.
            collect_in_expr(expr, source, rewrites);

            // Detect non-exhaustive Result/Option match.
            let mut has_ok = false;
            let mut has_err = false;
            let mut has_some = false;
            let mut has_none = false;
            let mut has_wildcard = false;
            for arm in arms {
                match &arm.pattern {
                    Pattern::Wildcard => has_wildcard = true,
                    Pattern::Variant { name, .. } => match name.as_str() {
                        "Ok" => has_ok = true,
                        "Err" => has_err = true,
                        "Some" => has_some = true,
                        "None" => has_none = true,
                        _ => {}
                    },
                    Pattern::Literal(_) => {}
                }
            }
            let needs_fix = (has_ok && !has_err)
                || (has_err && !has_ok)
                || (has_some && !has_none)
                || (has_none && !has_some);
            if !needs_fix || has_wildcard {
                return;
            }

            // Locate the byte range of the match-block's closing `}`.
            // We start at span.line/span.col (which may point at the
            // closing brace itself) and walk to the matching `}`.
            if let Some(close_pos) = find_matching_rbrace(source, span.line, span.col) {
                // Look backward from the close-brace, skipping
                // whitespace (spaces, tabs, newlines), for a `,`
                // separator. If we find one, replace it with our
                // wildcard arm so we don't end up with two commas.
                // Otherwise insert a comma + the arm before the
                // close-brace.
                let bytes = source.as_bytes();
                let mut j = close_pos;
                while j > 0
                    && (bytes[j - 1] == b' '
                        || bytes[j - 1] == b'\t'
                        || bytes[j - 1] == b'\n'
                        || bytes[j - 1] == b'\r')
                {
                    j -= 1;
                }
                if j > 0 && bytes[j - 1] == b',' {
                    // The previous arm already has a trailing `,`
                    // (the parser consumed it as a separator). We
                    // INSERT our arm after that comma; the original
                    // comma is kept as the separator.
                    rewrites.push((
                        j,
                        j,
                        " _ => process_exit(1),".to_string(),
                    ));
                } else {
                    // No trailing comma — insert our arm followed
                    // by a comma, before the close-brace.
                    rewrites.push((
                        close_pos,
                        close_pos,
                        " _ => process_exit(1),".to_string(),
                    ));
                }
            } else {
                // Could not locate a close-brace — skip this match.
                // The user will still see the typecheck error pointing
                // at the offending span, which is the next-best signal.
            }
        }
        Expression::StructLit { .. } => {}
    }
}

/// Find the byte offset of the `}` that closes the *match block*
/// whose AST span is at (1-indexed) line / col. The hint position
/// may point anywhere in the match expression (often the closing
/// brace or one of the arms). We treat the hint as *outside* the
/// block (depth = 0) and scan forward: the next time depth returns
/// to 0 we are at the matching `}`. Skips strings and comments.
fn find_matching_rbrace(source: &str, line: usize, col: usize) -> Option<usize> {
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
        return None;
    }

        // The hint may sit on the open-brace, the close-brace, or
    // anywhere between. If it sits on the close-brace itself,
    // that's already the answer. Otherwise treat it as outside the
    // block (depth 0) and scan forward for the first time depth
    // returns to 0.
    if bytes.get(idx) == Some(&b'}') {
        return Some(idx);
    }
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut i = idx;
    while i < bytes.len() {
        let b = bytes[i];
        if in_line_comment {
            if b == b'\n' {
                in_line_comment = false;
            }
        } else if in_block_comment {
            if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                in_block_comment = false;
                i += 1;
            }
        } else if in_string {
            if b == b'"' {
                in_string = false;
            } else if b == b'\\' && i + 1 < bytes.len() {
                i += 1;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                    in_line_comment = true;
                    i += 1;
                }
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                    in_block_comment = true;
                    i += 1;
                }
                b'{' => {
                    depth += 1;
                }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    eprintln!("[debug] no rbrace found, returning None");
    None
}
