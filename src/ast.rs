use serde::{Deserialize, Serialize};

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
    Literal(Literal),
    Variable(String),
    Binary {
        op: BinOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Call {
        name: String,
        args: Vec<Expression>,
        propagate_err: bool,
    },
    If {
        cond: Box<Expression>,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    Match {
        expr: Box<Expression>,
        arms: Vec<MatchArm>,
    },
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
    },
    Return(Option<Expression>),
    Expr(Expression),
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<Item>,
}
