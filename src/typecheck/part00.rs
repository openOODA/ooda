// ===================================================================
// openOODA Static Type Checker (alpha)
// Narrow but real: rejects type mismatches before evaluation.
// ===================================================================
use crate::ast::*;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int,
    Float,
    String,
    Bool,
    Void,
    NetCap,
    FsCap,
    EnvCap,
    SysCap,
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),
    List(Box<Ty>),
    Struct {
        name: Option<String>,
        fields: Vec<(String, Ty)>,
    },
    Custom(String),
    /// Unknown / not yet inferred (permissive for incomplete language surface).
    Unknown,
}


impl Ty {
    fn from_ast(t: &Type) -> Self {
        match t {
            Type::Int => Ty::Int,
            Type::Float => Ty::Float,
            Type::String => Ty::String,
            Type::Bool => Ty::Bool,
            Type::Void => Ty::Void,
            Type::NetCap => Ty::NetCap,
            Type::FsCap => Ty::FsCap,
            Type::EnvCap => Ty::EnvCap,
            Type::SysCap => Ty::SysCap,
            Type::Option(inner) => Ty::Option(Box::new(Ty::from_ast(inner))),
            Type::Result(ok, err) => {
                Ty::Result(Box::new(Ty::from_ast(ok)), Box::new(Ty::from_ast(err)))
            }
            Type::List(inner) => Ty::List(Box::new(Ty::from_ast(inner))),
            Type::Struct { name, fields } => Ty::Struct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|(n, t)| (n.clone(), Ty::from_ast(t)))
                    .collect(),
            },
            Type::Custom(s) => match s.as_str() {
                "Int" | "i64" | "u64" | "i32" => Ty::Int,
                "Float" | "f64" => Ty::Float,
                "String" => Ty::String,
                "Bool" => Ty::Bool,
                "Void" => Ty::Void,
                "NetCap" => Ty::NetCap,
                "FsCap" => Ty::FsCap,
                "EnvCap" => Ty::EnvCap,
                "SysCap" => Ty::SysCap,
                // Int[lo..hi] is still Int for unify; bounds enforced separately.
                other if other.starts_with("Int[") && other.ends_with(']') => Ty::Int,
                other => Ty::Custom(other.to_string()),
            },
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }

    pub fn normalize(&self, aliases: &HashMap<String, Ty>) -> Ty {
        self.normalize_with_depth(aliases, 0)
    }

    fn normalize_with_depth(&self, aliases: &HashMap<String, Ty>, depth: usize) -> Ty {
        if depth > 10 {
            return self.clone();
        }
        match self {
            Ty::Custom(name) => {
                if let Some(target) = aliases.get(name) {
                    target.normalize_with_depth(aliases, depth + 1)
                } else {
                    Ty::Custom(name.clone())
                }
            }
            Ty::Option(inner) => Ty::Option(Box::new(inner.normalize_with_depth(aliases, depth + 1))),
            Ty::Result(ok, err) => Ty::Result(
                Box::new(ok.normalize_with_depth(aliases, depth + 1)),
                Box::new(err.normalize_with_depth(aliases, depth + 1)),
            ),
            Ty::List(inner) => Ty::List(Box::new(inner.normalize_with_depth(aliases, depth + 1))),
            Ty::Struct { name, fields } => Ty::Struct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|(n, t)| (n.clone(), t.normalize_with_depth(aliases, depth + 1)))
                    .collect(),
            },
            other => other.clone(),
        }
    }

    pub fn unifyable_with_aliases(a: &Ty, b: &Ty, aliases: &HashMap<String, Ty>) -> bool {
        let norm_a = a.normalize(aliases);
        let norm_b = b.normalize(aliases);
        if norm_a == norm_b {
            return true;
        }
        if matches!(norm_a, Ty::Unknown) && matches!(norm_b, Ty::Unknown) {
            return true;
        }
        if (matches!(norm_a, Ty::Void) && matches!(norm_b, Ty::NetCap | Ty::FsCap | Ty::SysCap | Ty::EnvCap))
            || (matches!(norm_b, Ty::Void) && matches!(norm_a, Ty::NetCap | Ty::FsCap | Ty::SysCap | Ty::EnvCap))
        {
            return true;
        }
        match (&norm_a, &norm_b) {
            (Ty::Result(a1, a2), Ty::Result(b1, b2)) => {
                Ty::unifyable_or_unknown_hole_with_aliases(a1, b1, aliases)
                    && Ty::unifyable_or_unknown_hole_with_aliases(a2, b2, aliases)
            }
            (Ty::Option(a1), Ty::Option(b1)) => {
                Ty::unifyable_or_unknown_hole_with_aliases(a1, b1, aliases)
            }
            (Ty::List(a1), Ty::List(b1)) => {
                Ty::unifyable_or_unknown_hole_with_aliases(a1, b1, aliases)
            }
            (Ty::Struct { fields: fa, .. }, Ty::Struct { fields: fb, .. }) => {
                if fa.len() != fb.len() {
                    return false;
                }
                fa.iter().zip(fb.iter()).all(|((na, ta), (nb, tb))| {
                    na == nb && Ty::unifyable_with_aliases(ta, tb, aliases)
                })
            }
            (Ty::Struct { name: Some(n), .. }, Ty::Custom(c))
            | (Ty::Custom(c), Ty::Struct { name: Some(n), .. }) => n == c,
            (Ty::Custom(a), Ty::Custom(b)) => a == b,
            _ => false,
        }
    }

    pub fn unifyable_or_unknown_hole_with_aliases(a: &Ty, b: &Ty, aliases: &HashMap<String, Ty>) -> bool {
        let norm_a = a.normalize(aliases);
        let norm_b = b.normalize(aliases);
        matches!(norm_a, Ty::Unknown)
            || matches!(norm_b, Ty::Unknown)
            || Ty::unifyable_with_aliases(&norm_a, &norm_b, aliases)
    }

    /// Fail-closed unify: `Unknown` only unifies with `Unknown` (inference hole,
    /// not a wildcard). `Custom` only matches same name or a named struct alias.
    fn unifyable(a: &Ty, b: &Ty) -> bool {
        Ty::unifyable_with_aliases(a, b, &HashMap::new())
    }

    /// Like unifyable, but Unknown on either side is a polymorphic hole (Ok/Err/Some).
    fn unifyable_or_unknown_hole(a: &Ty, b: &Ty) -> bool {
        Ty::unifyable_or_unknown_hole_with_aliases(a, b, &HashMap::new())
    }

    /// Evaluate simple integer constant expressions for refinement checks.
    fn const_int(expr: &Expression) -> Option<i64> {
        match expr {
            Expression::Literal(Literal::Int(n), _) => Some(*n),
            Expression::Unary {
                op: UnaryOp::Neg,
                expr,
                ..
            } => Ty::const_int(expr).map(|n| n.saturating_neg()),
            Expression::Binary {
                op, left, right, ..
            } => {
                let l = Ty::const_int(left)?;
                let r = Ty::const_int(right)?;
                match op {
                    BinOp::Add => Some(l.saturating_add(r)),
                    BinOp::Sub => Some(l.saturating_sub(r)),
                    BinOp::Mul => Some(l.saturating_mul(r)),
                    BinOp::Div if r != 0 => Some(l / r),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// String literal only (for const char_at / str_slice bounds).
    fn const_str(expr: &Expression) -> Option<&str> {
        match expr {
            Expression::Literal(Literal::String(s), _) => Some(s.as_str()),
            _ => None,
        }
    }

    fn display(&self) -> String {
        match self {
            Ty::Int => "Int".into(),
            Ty::Float => "Float".into(),
            Ty::String => "String".into(),
            Ty::Bool => "Bool".into(),
            Ty::Void => "Void".into(),
            Ty::NetCap => "NetCap".into(),
            Ty::FsCap => "FsCap".into(),
            Ty::EnvCap => "EnvCap".into(),
            Ty::SysCap => "SysCap".into(),
            Ty::Option(t) => format!("Option[{}]", t.display()),
            Ty::Result(o, e) => format!("Result[{}, {}]", o.display(), e.display()),
            Ty::List(t) => format!("List[{}]", t.display()),
            Ty::Struct { name, fields } => {
                let body: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t.display()))
                    .collect();
                match name {
                    Some(n) => format!("{} {{ {} }}", n, body.join(", ")),
                    None => format!("struct {{ {} }}", body.join(", ")),
                }
            }
            Ty::Custom(s) => s.clone(),
            Ty::Unknown => "_".into(),
        }
    }
}


pub struct TypeChecker {
    functions: HashMap<String, (Vec<Ty>, Ty)>,
    /// Per-function `Int[lo..hi]` parameter bounds (`None` = unrefined).
    /// Call-sites with const args are fail-closed against these.
    param_refinements: HashMap<String, Vec<Option<(i64, i64)>>>,
    /// Named type aliases (including named structs) for StructLit typing.
    type_aliases: HashMap<String, Ty>,
    /// `type Port = Int[lo..hi]` — bounds keyed by alias name (from_ast collapses to Int).
    alias_refinements: HashMap<String, (i64, i64)>,
    /// Active block's const list lengths (set by check_block for list_get OOB).
    active_list_lens: std::cell::RefCell<HashMap<String, i64>>,
    /// Enclosing function return type (for `?` legality).
    current_return: std::cell::RefCell<Option<Ty>>,
    /// Nesting depth of while/for (desugared while) for break/continue.
    loop_depth: std::cell::Cell<u32>,
}
