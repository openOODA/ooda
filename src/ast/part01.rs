
impl FunctionDecl {
    /// True iff any postcondition (`ensures`) in this function (or its
    /// verify block) calls `old(x)`. Used by the interpreter to skip
    /// the parameter snapshot when no `old()` reference exists — a
    /// real E-M win: zero `HashMap` allocation per call for the
    /// common case where contracts don't reach for prior state.
    pub fn uses_old_state(&self) -> bool {
        block_calls_old(&self.body)
            || self.ensures.iter().any(expression_calls_old)
            || self
                .verify_block
                .as_ref()
                .map_or(false, block_calls_old)
    }
}

/// Recursively check whether an expression contains a call to `old`.
fn expression_calls_old(e: &Expression) -> bool {
    match e {
        Expression::Call { name, args, .. } if name == "old" => true,
        Expression::Binary { left, right, .. } => {
            expression_calls_old(left) || expression_calls_old(right)
        }
        Expression::Unary { expr, .. } => expression_calls_old(expr),
        Expression::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expression_calls_old(cond)
                || block_calls_old(then_branch)
                || else_branch
                    .as_ref()
                    .map(|b| block_calls_old(b))
                    .unwrap_or(false)
        }
        Expression::Call { args, .. } => args.iter().any(expression_calls_old),
        Expression::Match { expr, arms, .. } => {
            expression_calls_old(expr) || arms.iter().any(|a| expression_calls_old(&a.body))
        }
        Expression::While { cond, body, .. } => {
            expression_calls_old(cond) || block_calls_old(body)
        }
        Expression::Literal(_, _) | Expression::Variable(_, _) | Expression::StructLit { .. } => {
            false
        }
    }
}

fn block_calls_old(b: &Block) -> bool {
    b.stmts.iter().any(stmt_calls_old) || b.expr.as_deref().map_or(false, expression_calls_old)
}

fn stmt_calls_old(s: &Statement) -> bool {
    match s {
        Statement::Let { init, .. } => expression_calls_old(init),
        Statement::Assign { value, .. } => expression_calls_old(value),
        Statement::FieldAssign { object, value, .. } => {
            expression_calls_old(object) || expression_calls_old(value)
        }
        Statement::Return(Some(e), _) => expression_calls_old(e),
        Statement::Return(None, _) => false,
        Statement::Expr(e, _) => expression_calls_old(e),
        Statement::While { cond, body, .. } => expression_calls_old(cond) || block_calls_old(body),
        Statement::Break(_) | Statement::Continue(_) => false,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: Type,
    pub is_ref: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Function(FunctionDecl),
    TypeAlias(String, Type),
    /// `import "path/to/module.oo";` — load another .oo source (userland modules).
    Import { path: String, span: Span },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<Item>,
}

impl Program {
    pub fn collect_type_aliases(&self) -> std::collections::HashMap<String, Type> {
        let mut aliases = std::collections::HashMap::new();
        for item in &self.items {
            if let Item::TypeAlias(name, ty) = item {
                aliases.insert(name.clone(), ty.clone());
            }
        }
        aliases
    }
}

impl Type {
    pub fn resolve_alias(&self, aliases: &std::collections::HashMap<String, Type>) -> Type {
        self.resolve_alias_depth(aliases, 0)
    }

    fn resolve_alias_depth(&self, aliases: &std::collections::HashMap<String, Type>, depth: usize) -> Type {
        if depth > 10 {
            return self.clone();
        }
        match self {
            Type::Custom(s) => {
                if let Some(target) = aliases.get(s) {
                    target.resolve_alias_depth(aliases, depth + 1)
                } else if s.starts_with("Int[") && s.ends_with(']') {
                    Type::Int
                } else {
                    Type::Custom(s.clone())
                }
            }
            Type::Option(inner) => Type::Option(Box::new(inner.resolve_alias_depth(aliases, depth + 1))),
            Type::Result(ok, err) => Type::Result(
                Box::new(ok.resolve_alias_depth(aliases, depth + 1)),
                Box::new(err.resolve_alias_depth(aliases, depth + 1)),
            ),
            Type::List(inner) => Type::List(Box::new(inner.resolve_alias_depth(aliases, depth + 1))),
            other => other.clone(),
        }
    }
}
