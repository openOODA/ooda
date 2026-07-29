use serde::{Deserialize, Serialize};

/// Source location (1-indexed line and column) for AST nodes.
/// Used by `--json-errors` to give AI agents the exact location of
/// the offending construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub const fn synthetic() -> Self {
        Span { line: 1, col: 1 }
    }
    #[allow(dead_code)]
    pub fn format(&self) -> String {
        format!("at {}:{}", self.line, self.col)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Void,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Void,
    Custom(String),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    /// Homogeneous list / vector. Element type may be `Custom("_")` when inferred loosely.
    List(Box<Type>),
    /// Anonymous or named product type. `name` is set for `type Foo = struct { ... }` aliases
    /// and for struct literals that reference a named alias.
    Struct {
        name: Option<String>,
        fields: Vec<(String, Type)>,
    },
    NetCap,
    FsCap,
    EnvCap,
    SysCap,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    DotDot,
    DotDotEq,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Literal(Literal, Span),
    Variable(String, Span),
    Binary {
        op: BinOp,
        left: Box<Expression>,
        right: Box<Expression>,
        span: Span,
    },
    Call {
        name: String,
        args: Vec<Expression>,
        propagate_err: bool,
        span: Span,
    },
    If {
        cond: Box<Expression>,
        then_branch: Block,
        else_branch: Option<Block>,
        span: Span,
    },
    Match {
        expr: Box<Expression>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
        span: Span,
    },
    While {
        cond: Box<Expression>,
        body: Block,
        span: Span,
    },
    /// Named struct literal: `Token { kind: 1, text: "fn" }`.
    StructLit {
        name: String,
        fields: Vec<(String, Expression)>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}

impl Expression {
    /// Return the source span of this expression. Every variant carries
    /// the span of its leading token.
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal(_, s)
            | Expression::Variable(_, s)
            | Expression::Binary { span: s, .. }
            | Expression::Call { span: s, .. }
            | Expression::If { span: s, .. }
            | Expression::Match { span: s, .. }
            | Expression::Unary { span: s, .. }
            | Expression::While { span: s, .. }
            | Expression::StructLit { span: s, .. } => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Literal(Literal),
    Variant { name: String, arg: Option<String> },
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Let {
        name: String,
        mutable: bool,
        type_annotation: Option<Type>,
        init: Expression,
        span: Span,
    },
    /// Assignment to an existing binding (`x = expr;`). Requires `let mut`.
    Assign {
        name: String,
        value: Expression,
        span: Span,
    },
    Return(Option<Expression>, Span),
    Expr(Expression, Span),
    While {
        cond: Expression,
        body: Block,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub stmts: Vec<Statement>,
    pub expr: Option<Box<Expression>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub is_pub: bool,
    pub name: String,
    pub span: Span,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub requires: Vec<Expression>,
    pub ensures: Vec<Expression>,
    pub body: Block,
    pub verify_block: Option<Block>,
}

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
        Statement::Return(Some(e), _) => expression_calls_old(e),
        Statement::Return(None, _) => false,
        Statement::Expr(e, _) => expression_calls_old(e),
        Statement::While { cond, body, .. } => expression_calls_old(cond) || block_calls_old(body),
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
