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
            | Expression::Match { span: s, .. } => *s,
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
