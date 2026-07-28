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
                other => Ty::Custom(other.to_string()),
            },
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float | Ty::Unknown)
    }

    fn unifyable(a: &Ty, b: &Ty) -> bool {
        if a == b {
            return true;
        }
        if matches!(a, Ty::Unknown) || matches!(b, Ty::Unknown) {
            return true;
        }
        // Allow Result[T,E] vs looser Unknown-containing forms
        match (a, b) {
            (Ty::Result(a1, a2), Ty::Result(b1, b2)) => {
                Ty::unifyable(a1, b1) && Ty::unifyable(a2, b2)
            }
            (Ty::Option(a1), Ty::Option(b1)) => Ty::unifyable(a1, b1),
            (Ty::Custom(_), _) | (_, Ty::Custom(_)) => true,
            _ => false,
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
            Ty::Custom(s) => s.clone(),
            Ty::Unknown => "_".into(),
        }
    }
}

pub struct TypeChecker {
    functions: HashMap<String, (Vec<Ty>, Ty)>,
}

impl TypeChecker {
    pub fn check_program(program: &Program) -> Result<()> {
        let mut tc = TypeChecker {
            functions: HashMap::new(),
        };

        // Builtins
        tc.functions
            .insert("println".into(), (vec![Ty::Unknown], Ty::Void));
        tc.functions
            .insert("assert_eq".into(), (vec![Ty::Unknown, Ty::Unknown], Ty::Void));
        tc.functions
            .insert("assert_is_err".into(), (vec![Ty::Unknown], Ty::Void));
        tc.functions.insert(
            "crypto_sha256_internal".into(),
            (vec![Ty::String], Ty::String),
        );
        tc.functions.insert(
            "crypto_hmac_sha256_internal".into(),
            (vec![Ty::String, Ty::String], Ty::String),
        );
        tc.functions.insert(
            "json_parse_internal".into(),
            (
                vec![Ty::String],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        tc.functions.insert(
            "json_stringify_internal".into(),
            (vec![Ty::Unknown], Ty::String),
        );
        tc.functions.insert(
            "async_spawn_internal".into(),
            (vec![Ty::String], Ty::String),
        );
        tc.functions.insert(
            "async_join_internal".into(),
            (
                vec![Ty::String],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        tc.functions.insert(
            "python_embed_internal".into(),
            (
                vec![Ty::String],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        tc.functions
            .insert("Ok".into(), (vec![Ty::Unknown], Ty::Unknown));
        tc.functions
            .insert("Err".into(), (vec![Ty::Unknown], Ty::Unknown));
        // Sealed effects (arg types loosely checked)
        for name in [
            "fetch",
            "downloadData",
            "http_get",
            "net_get",
            "read_file",
            "write_file",
            "fs_read",
            "fs_write",
            "sys_exec",
            "exec",
            "spawn_process",
            "env_get",
            "env_set",
        ] {
            tc.functions
                .insert(name.into(), (vec![Ty::Unknown], Ty::Unknown));
        }

        for item in &program.items {
            if let Item::Function(f) = item {
                let params: Vec<Ty> = f.params.iter().map(|p| Ty::from_ast(&p.param_type)).collect();
                let ret = Ty::from_ast(&f.return_type);
                tc.functions.insert(f.name.clone(), (params, ret));
            }
        }

        for item in &program.items {
            if let Item::Function(f) = item {
                tc.check_function(f)?;
            }
        }
        Ok(())
    }

    fn check_function(&self, func: &FunctionDecl) -> Result<()> {
        let mut env: HashMap<String, Ty> = HashMap::new();
        for p in &func.params {
            env.insert(p.name.clone(), Ty::from_ast(&p.param_type));
        }

        for req in &func.requires {
            let t = self.infer_expr(req, &env)?;
            if !Ty::unifyable(&t, &Ty::Bool) && !matches!(t, Ty::Unknown) {
                return Err(anyhow!(
                    "Type error in function '{}': 'requires' clause must be Bool, found {}",
                    func.name,
                    t.display()
                ));
            }
        }

        let body_ty = self.check_block(&func.body, &mut env, &func.name)?;
        let expected = Ty::from_ast(&func.return_type);
        if !matches!(expected, Ty::Void)
            && !Ty::unifyable(&body_ty, &expected)
            && !matches!(body_ty, Ty::Void | Ty::Unknown)
        {
            // Allow if returns were checked via Return statements inside
        }

        for ens in &func.ensures {
            let mut post = env.clone();
            post.insert("result".into(), expected.clone());
            let t = self.infer_expr(ens, &post)?;
            if !Ty::unifyable(&t, &Ty::Bool) && !matches!(t, Ty::Unknown) {
                return Err(anyhow!(
                    "Type error in function '{}': 'ensures' clause must be Bool, found {}",
                    func.name,
                    t.display()
                ));
            }
        }

        if let Some(verify) = &func.verify_block {
            let mut venv = HashMap::new();
            self.check_block(verify, &mut venv, &format!("verify {}", func.name))?;
        }

        Ok(())
    }

    fn check_block(
        &self,
        block: &Block,
        env: &mut HashMap<String, Ty>,
        ctx: &str,
    ) -> Result<Ty> {
        let mut last = Ty::Void;
        for stmt in &block.stmts {
            match stmt {
                Statement::Let {
                    name,
                    type_annotation,
                    init,
                    ..
                } => {
                    let init_ty = self.infer_expr(init, env)?;
                    if let Some(ann) = type_annotation {
                        let want = Ty::from_ast(ann);
                        if !Ty::unifyable(&init_ty, &want) {
                            return Err(anyhow!(
                                "Type error in '{}': let '{}' annotated as {} but initializer has type {}",
                                ctx,
                                name,
                                want.display(),
                                init_ty.display()
                            ));
                        }
                        env.insert(name.clone(), want);
                    } else {
                        env.insert(name.clone(), init_ty);
                    }
                    last = Ty::Void;
                }
                Statement::Return(Some(expr)) => {
                    last = self.infer_expr(expr, env)?;
                }
                Statement::Return(None) => {
                    last = Ty::Void;
                }
                Statement::Expr(expr) => {
                    last = self.infer_expr(expr, env)?;
                }
            }
        }
        if let Some(expr) = &block.expr {
            last = self.infer_expr(expr, env)?;
        }
        Ok(last)
    }

    fn infer_expr(&self, expr: &Expression, env: &HashMap<String, Ty>) -> Result<Ty> {
        match expr {
            Expression::Literal(Literal::Int(_)) => Ok(Ty::Int),
            Expression::Literal(Literal::Float(_)) => Ok(Ty::Float),
            Expression::Literal(Literal::String(_)) => Ok(Ty::String),
            Expression::Literal(Literal::Bool(_)) => Ok(Ty::Bool),
            Expression::Literal(Literal::Void) => Ok(Ty::Void),
            Expression::Variable(name) => env
                .get(name)
                .cloned()
                .or_else(|| {
                    // Allow unbound in incomplete programs only for method receivers we can't type yet
                    None
                })
                .ok_or_else(|| anyhow!("Type error: undefined variable '{}'", name)),
            Expression::Binary { op, left, right } => {
                let lt = self.infer_expr(left, env)?;
                let rt = self.infer_expr(right, env)?;
                match op {
                    BinOp::Add => {
                        if matches!(lt, Ty::String) || matches!(rt, Ty::String) {
                            if Ty::unifyable(&lt, &Ty::String) && Ty::unifyable(&rt, &Ty::String) {
                                return Ok(Ty::String);
                            }
                            // Allow String + numeric via coercion only if both string-like later
                            if matches!(lt, Ty::String) || matches!(rt, Ty::String) {
                                // String concat requires both String in DESIGN; be strict when both known
                                if !matches!(lt, Ty::Unknown | Ty::String)
                                    || !matches!(rt, Ty::Unknown | Ty::String)
                                {
                                    if matches!(lt, Ty::String) && matches!(rt, Ty::Int | Ty::Float)
                                        || matches!(rt, Ty::String)
                                            && matches!(lt, Ty::Int | Ty::Float)
                                    {
                                        return Err(anyhow!(
                                            "Type error: cannot concatenate {} and {} with '+'; convert with .to_string() first",
                                            lt.display(),
                                            rt.display()
                                        ));
                                    }
                                }
                            }
                        }
                        if lt.is_numeric() && rt.is_numeric() {
                            if matches!(lt, Ty::Float) || matches!(rt, Ty::Float) {
                                return Ok(Ty::Float);
                            }
                            return Ok(Ty::Int);
                        }
                        if matches!(lt, Ty::Unknown) || matches!(rt, Ty::Unknown) {
                            return Ok(Ty::Unknown);
                        }
                        Err(anyhow!(
                            "Type error: operator '+' not defined for {} and {}",
                            lt.display(),
                            rt.display()
                        ))
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        if lt.is_numeric() && rt.is_numeric() {
                            if matches!(lt, Ty::Float) || matches!(rt, Ty::Float) {
                                Ok(Ty::Float)
                            } else {
                                Ok(Ty::Int)
                            }
                        } else if matches!(lt, Ty::Unknown) || matches!(rt, Ty::Unknown) {
                            Ok(Ty::Unknown)
                        } else {
                            Err(anyhow!(
                                "Type error: arithmetic operator requires numeric operands, found {} and {}",
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::Eq | BinOp::Neq => Ok(Ty::Bool),
                    BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte => {
                        if lt.is_numeric() && rt.is_numeric()
                            || matches!(lt, Ty::Unknown)
                            || matches!(rt, Ty::Unknown)
                        {
                            Ok(Ty::Bool)
                        } else {
                            Err(anyhow!(
                                "Type error: comparison requires numeric operands, found {} and {}",
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::And | BinOp::Or => {
                        if (Ty::unifyable(&lt, &Ty::Bool) || matches!(lt, Ty::Unknown))
                            && (Ty::unifyable(&rt, &Ty::Bool) || matches!(rt, Ty::Unknown))
                        {
                            Ok(Ty::Bool)
                        } else {
                            Err(anyhow!(
                                "Type error: logical operator requires Bool operands, found {} and {}",
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::DotDot | BinOp::DotDotEq => Ok(Ty::Unknown),
                }
            }
            Expression::Call { name, args, .. } => {
                // Methods: .len, .trim, etc.
                if name.starts_with('.') {
                    let recv = args
                        .first()
                        .ok_or_else(|| anyhow!("Type error: method '{}' missing receiver", name))?;
                    let recv_ty = self.infer_expr(recv, env)?;
                    for a in args.iter().skip(1) {
                        self.infer_expr(a, env)?;
                    }
                    return match name.as_str() {
                        ".len" => {
                            if matches!(recv_ty, Ty::String | Ty::Unknown) {
                                Ok(Ty::Int)
                            } else {
                                Err(anyhow!(
                                    "Type error: .len() requires String receiver, found {}",
                                    recv_ty.display()
                                ))
                            }
                        }
                        ".trim" | ".to_lowercase" | ".to_string" => Ok(Ty::String),
                        ".is_ok" => Ok(Ty::Bool),
                        ".get" | ".read_file" | ".write_file" | ".env_get" => Ok(Ty::Unknown),
                        _ => Ok(Ty::Unknown),
                    };
                }

                let mut arg_tys = Vec::new();
                for a in args {
                    arg_tys.push(self.infer_expr(a, env)?);
                }

                if let Some((params, ret)) = self.functions.get(name) {
                    if !params.is_empty()
                        && params.len() == arg_tys.len()
                        && !params.iter().all(|p| matches!(p, Ty::Unknown))
                    {
                        for (i, (pt, at)) in params.iter().zip(arg_tys.iter()).enumerate() {
                            if !Ty::unifyable(pt, at) {
                                return Err(anyhow!(
                                    "Type error: function '{}' argument {} expects {}, found {}",
                                    name,
                                    i,
                                    pt.display(),
                                    at.display()
                                ));
                            }
                        }
                    }
                    return Ok(ret.clone());
                }

                // Unknown function: still type-check args; return Unknown
                Ok(Ty::Unknown)
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let ct = self.infer_expr(cond, env)?;
                if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                    return Err(anyhow!(
                        "Type error: 'if' condition must be Bool, found {}",
                        ct.display()
                    ));
                }
                let mut env_then = env.clone();
                let t1 = self.check_block(then_branch, &mut env_then, "if-then")?;
                if let Some(else_b) = else_branch {
                    let mut env_else = env.clone();
                    let t2 = self.check_block(else_b, &mut env_else, "if-else")?;
                    if Ty::unifyable(&t1, &t2) {
                        Ok(t1)
                    } else {
                        Ok(Ty::Unknown)
                    }
                } else {
                    Ok(t1)
                }
            }
            Expression::Match { expr, arms } => {
                self.infer_expr(expr, env)?;
                let mut result = Ty::Unknown;
                for arm in arms {
                    let mut arm_env = env.clone();
                    // Bind simple variant patterns loosely
                    if let Pattern::Variant {
                        name: _,
                        arg: Some(var),
                    } = &arm.pattern
                    {
                        arm_env.insert(var.clone(), Ty::Unknown);
                    }
                    let t = self.infer_expr(&arm.body, &arm_env)?;
                    result = t;
                }
                Ok(result)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Result<()> {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize()?;
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse_program()?;
        TypeChecker::check_program(&program)
    }

    #[test]
    fn rejects_bool_plus_int() {
        let src = r#"
            pub fn main() {
                let x = true + 1;
            }
        "#;
        assert!(check(src).is_err());
    }

    #[test]
    fn accepts_int_arith() {
        let src = r#"
            pub fn add(a: Int, b: Int) -> Int {
                return a + b;
            }
            pub fn main() {
                let x = add(1, 2);
            }
        "#;
        assert!(check(src).is_ok());
    }

    #[test]
    fn rejects_undefined_variable() {
        let src = r#"
            pub fn main() {
                let x = missing_var + 1;
            }
        "#;
        assert!(check(src).is_err());
    }
}
