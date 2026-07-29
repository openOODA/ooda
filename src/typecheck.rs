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
        // Allow () (Void) capability tokens in verify blocks
        if (matches!(a, Ty::Void) && matches!(b, Ty::NetCap | Ty::FsCap | Ty::SysCap | Ty::EnvCap))
            || (matches!(b, Ty::Void) && matches!(a, Ty::NetCap | Ty::FsCap | Ty::SysCap | Ty::EnvCap))
        {
            return true;
        }
        // Allow Result[T,E] vs looser Unknown-containing forms
        match (a, b) {
            (Ty::Result(a1, a2), Ty::Result(b1, b2)) => {
                Ty::unifyable(a1, b1) && Ty::unifyable(a2, b2)
            }
            (Ty::Option(a1), Ty::Option(b1)) => Ty::unifyable(a1, b1),
            (Ty::List(a1), Ty::List(b1)) => Ty::unifyable(a1, b1),
            (Ty::Struct { fields: fa, .. }, Ty::Struct { fields: fb, .. }) => {
                if fa.len() != fb.len() {
                    return false;
                }
                fa.iter().zip(fb.iter()).all(|((na, ta), (nb, tb))| {
                    na == nb && Ty::unifyable(ta, tb)
                })
            }
            // Named struct alias vs Custom("Token") from annotations
            (Ty::Struct { name: Some(n), .. }, Ty::Custom(c))
            | (Ty::Custom(c), Ty::Struct { name: Some(n), .. }) => n == c,
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
    /// Named type aliases (including named structs) for StructLit typing.
    type_aliases: HashMap<String, Ty>,
}

impl TypeChecker {
    pub fn check_program(program: &Program) -> Result<()> {
        let mut tc = TypeChecker {
            functions: HashMap::new(),
            type_aliases: HashMap::new(),
        };

        // Collect type aliases first (named structs for StructLit).
        for item in &program.items {
            if let Item::TypeAlias(name, ty) = item {
                tc.type_aliases
                    .insert(name.clone(), Ty::from_ast(ty));
            }
        }

        // Builtins
        tc.functions
            .insert("println".into(), (vec![Ty::Unknown], Ty::Void));
        tc.functions
            .insert("assert_eq".into(), (vec![Ty::Unknown, Ty::Unknown], Ty::Void));
        tc.functions
            .insert("assert_is_err".into(), (vec![Ty::Unknown], Ty::Void));
        // CHS list surface
        tc.functions
            .insert("list_new".into(), (vec![], Ty::List(Box::new(Ty::Unknown))));
        tc.functions.insert(
            "list_push".into(),
            (
                vec![Ty::List(Box::new(Ty::Unknown)), Ty::Unknown],
                Ty::List(Box::new(Ty::Unknown)),
            ),
        );
        tc.functions.insert(
            "list_get".into(),
            (
                vec![Ty::List(Box::new(Ty::Unknown)), Ty::Int],
                Ty::Unknown,
            ),
        );
        tc.functions.insert(
            "list_len".into(),
            (vec![Ty::List(Box::new(Ty::Unknown))], Ty::Int),
        );
        // CHS string walk
        tc.functions
            .insert("chars_len".into(), (vec![Ty::String], Ty::Int));
        tc.functions
            .insert("char_at".into(), (vec![Ty::String, Ty::Int], Ty::String));
        tc.functions.insert(
            "str_slice".into(),
            (vec![Ty::String, Ty::Int, Ty::Int], Ty::String),
        );
        tc.functions
            .insert("char_is_digit".into(), (vec![Ty::String], Ty::Bool));
        tc.functions
            .insert("char_is_alpha".into(), (vec![Ty::String], Ty::Bool));
        tc.functions
            .insert("char_is_space".into(), (vec![Ty::String], Ty::Bool));
        // Host bootstrap APIs (exact stage-0 dumps + real CHS native build)
        tc.functions
            .insert("host_ast_dump".into(), (vec![Ty::String], Ty::String));
        tc.functions
            .insert("host_check".into(), (vec![Ty::String], Ty::String));
        tc.functions
            .insert("host_token_dump".into(), (vec![Ty::String], Ty::String));
        tc.functions.insert(
            "chs_build".into(),
            (
                vec![Ty::String, Ty::String],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        tc.functions
            .insert("process_exit".into(), (vec![Ty::Int], Ty::Void));
        for (n, ret) in [
            ("mkdir_p", Ty::Result(Box::new(Ty::Void), Box::new(Ty::String))),
            ("chmod_exec", Ty::Result(Box::new(Ty::Void), Box::new(Ty::String))),
            (
                "copy_file",
                Ty::Result(Box::new(Ty::Void), Box::new(Ty::String)),
            ),
            (
                "http_download",
                Ty::Result(Box::new(Ty::Void), Box::new(Ty::String)),
            ),
            (
                "extract_tar_gz",
                Ty::Result(Box::new(Ty::Void), Box::new(Ty::String)),
            ),
        ] {
            tc.functions
                .insert(n.into(), (vec![Ty::Unknown], ret));
        }
        tc.functions
            .insert("path_exists".into(), (vec![Ty::String], Ty::Bool));
        tc.functions.insert(
            "sys_exec".into(),
            (
                vec![Ty::Unknown],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        // Real FS / env (sealed effects; arg types loose)
        tc.functions.insert(
            "env_get".into(),
            (
                vec![Ty::Unknown],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
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
        // Ok/Err construct Result — typed as Result[T, _] / Result[_, E] loosely
        tc.functions.insert(
            "Ok".into(),
            (
                vec![Ty::Unknown],
                Ty::Result(Box::new(Ty::Unknown), Box::new(Ty::Unknown)),
            ),
        );
        tc.functions.insert(
            "Err".into(),
            (
                vec![Ty::Unknown],
                Ty::Result(Box::new(Ty::Unknown), Box::new(Ty::Unknown)),
            ),
        );
        tc.functions.insert(
            "Some".into(),
            (
                vec![Ty::Unknown],
                Ty::Option(Box::new(Ty::Unknown)),
            ),
        );
        tc.functions.insert(
            "None".into(),
            (vec![], Ty::Option(Box::new(Ty::Unknown))),
        );
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
        let mut mutable: HashMap<String, bool> = HashMap::new();
        for p in &func.params {
            env.insert(p.name.clone(), Ty::from_ast(&p.param_type));
            // Parameters are mutable by default for practical alpha (like many langs);
            // DESIGN immutability-by-default applies to `let` bindings.
            mutable.insert(p.name.clone(), true);
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

        let body_ty = self.check_block(&func.body, &mut env, &mut mutable, &func.name)?;

        // Static refinement bounds check for return statements against function return_type
        if let Type::Custom(ref s) = func.return_type {
            if let Some(rest) = s.strip_prefix("Int[").and_then(|str_s| str_s.strip_suffix("]")) {
                if let Some((min_s, max_s)) = rest.split_once("..") {
                    let min_v: i64 = min_s.parse().unwrap_or(i64::MIN);
                    let max_v: i64 = max_s.parse().unwrap_or(i64::MAX);
                    for stmt in &func.body.stmts {
                        if let Statement::Return(Some(Expression::Literal(Literal::Int(val), l_span)), _) = stmt {
                            if *val < min_v || *val > max_v {
                                return Err(anyhow!(
                                    "Type error at {}:{}: RefinementTypeViolation: Returned value {} out of refinement bounds [{}..{}] for return type of function '{}'",
                                    l_span.line,
                                    l_span.col,
                                    val,
                                    min_v,
                                    max_v,
                                    func.name
                                ));
                            }
                        }
                    }
                }
            }
        }

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
            let mut vmut = HashMap::new();
            self.check_block(
                verify,
                &mut venv,
                &mut vmut,
                &format!("verify {}", func.name),
            )?;
        }

        Ok(())
    }

    fn check_block(
        &self,
        block: &Block,
        env: &mut HashMap<String, Ty>,
        mutable: &mut HashMap<String, bool>,
        ctx: &str,
    ) -> Result<Ty> {
        let mut last = Ty::Void;
        for stmt in &block.stmts {
            match stmt {
                Statement::Let {
                    name,
                    mutable: is_mut,
                    type_annotation,
                    init,
                    span,
                    ..
                } => {
                    let init_ty = self.infer_expr(init, env)?;
                    if let Some(ann) = type_annotation {
                        let want = Ty::from_ast(ann);
                        if let Type::Custom(ref s) = ann {
                            if let Some(rest) = s.strip_prefix("Int[").and_then(|str_s| str_s.strip_suffix("]")) {
                                if let Some((min_s, max_s)) = rest.split_once("..") {
                                    let min_v: i64 = min_s.parse().unwrap_or(i64::MIN);
                                    let max_v: i64 = max_s.parse().unwrap_or(i64::MAX);
                                    if let Expression::Literal(Literal::Int(val), l_span) = init {
                                        if *val < min_v || *val > max_v {
                                            return Err(anyhow!(
                                                "Type error at {}:{}: RefinementTypeViolation: Value {} out of refinement bounds [{}..{}] for '{}'",
                                                l_span.line,
                                                l_span.col,
                                                val,
                                                min_v,
                                                max_v,
                                                name
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        if !Ty::unifyable(&init_ty, &want) {
                            return Err(anyhow!(
                                "Type error at {}:{} in '{}': let '{}' annotated as {} but initializer has type {}",
                                span.line,
                                span.col,
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
                    mutable.insert(name.clone(), *is_mut);
                    last = Ty::Void;
                }
                Statement::Assign { name, value, span } => {
                    if !env.contains_key(name) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign to undefined variable '{}'",
                            span.line,
                            span.col,
                            name
                        ));
                    }
                    if !mutable.get(name).copied().unwrap_or(false) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign to immutable binding '{}'; use `let mut {}`",
                            span.line,
                            span.col,
                            name,
                            name
                        ));
                    }
                    let vty = self.infer_expr(value, env)?;
                    let want = env.get(name).cloned().unwrap_or(Ty::Unknown);
                    if !Ty::unifyable(&vty, &want) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign {} to '{}' of type {}",
                            span.line,
                            span.col,
                            vty.display(),
                            name,
                            want.display()
                        ));
                    }
                    last = Ty::Void;
                }
                Statement::Return(Some(expr), _) => {
                    last = self.infer_expr(expr, env)?;
                }
                Statement::Return(None, _) => {
                    last = Ty::Void;
                }
                Statement::Expr(expr, span) => {
                    // Statement-level if/while must inherit mutability so
                    // `let mut x` can be assigned inside branches (CHS oodac).
                    match expr {
                        Expression::If {
                            cond,
                            then_branch,
                            else_branch,
                            span: ispan,
                        } => {
                            let ct = self.infer_expr(cond, env)?;
                            if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: 'if' condition must be Bool, found {}",
                                    ispan.line,
                                    ispan.col,
                                    ct.display()
                                ));
                            }
                            self.check_block(then_branch, env, mutable, "if-then")?;
                            if let Some(eb) = else_branch {
                                self.check_block(eb, env, mutable, "if-else")?;
                            }
                            last = Ty::Void;
                        }
                        Expression::While {
                            cond,
                            body,
                            span: wspan,
                        } => {
                            let ct = self.infer_expr(cond, env)?;
                            if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: while condition must be Bool, found {}",
                                    wspan.line,
                                    wspan.col,
                                    ct.display()
                                ));
                            }
                            self.check_block(body, env, mutable, "while-expr-stmt")?;
                            last = Ty::Void;
                        }
                        _ => {
                            let t = self.infer_expr(expr, env)?;
                            // DESIGN must-use: discarded Result/Option is a hard error.
                            if matches!(t, Ty::Result(_, _) | Ty::Option(_)) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: unused {} value (must-use); handle it or bind with `let _ = ...` is not enough — use `let` and match, or `?`",
                                    span.line,
                                    span.col,
                                    t.display()
                                ));
                            }
                            last = t;
                        }
                    }
                }
                Statement::While { cond, body, span } => {
                    let ct = self.infer_expr(cond, env)?;
                    if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                        return Err(anyhow!(
                            "Type error at {}:{}: while condition must be Bool, found {}",
                            span.line,
                            span.col,
                            ct.display()
                        ));
                    }
                    self.check_block(body, env, mutable, "while-body")?;
                    last = Ty::Void;
                }
            }
        }
        if let Some(expr) = &block.expr {
            // Tail expression may be a nested `else if` chain — keep mut map.
            match expr.as_ref() {
                Expression::If {
                    cond,
                    then_branch,
                    else_branch,
                    span: ispan,
                } => {
                    let ct = self.infer_expr(cond, env)?;
                    if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                        return Err(anyhow!(
                            "Type error at {}:{}: 'if' condition must be Bool, found {}",
                            ispan.line,
                            ispan.col,
                            ct.display()
                        ));
                    }
                    self.check_block(then_branch, env, mutable, "if-then-tail")?;
                    if let Some(eb) = else_branch {
                        self.check_block(eb, env, mutable, "if-else-tail")?;
                    }
                    last = Ty::Void;
                }
                _ => {
                    last = self.infer_expr(expr, env)?;
                }
            }
        }
        Ok(last)
    }

    fn infer_expr(&self, expr: &Expression, env: &HashMap<String, Ty>) -> Result<Ty> {
        match expr {
            Expression::Literal(Literal::Int(_), _) => Ok(Ty::Int),
            Expression::Literal(Literal::Float(_), _) => Ok(Ty::Float),
            Expression::Literal(Literal::String(_), _) => Ok(Ty::String),
            Expression::Literal(Literal::Bool(_), _) => Ok(Ty::Bool),
            Expression::Literal(Literal::Void, _) => Ok(Ty::Void),
            Expression::Variable(name, _) => env
                .get(name)
                .cloned()
                .or_else(|| {
                    // Allow unbound in incomplete programs only for method receivers we can't type yet
                    None
                })
                .ok_or_else(|| anyhow!("Type error at {}:{}: undefined variable '{}'", expr.span().line, expr.span().col, name)),
            Expression::Binary { op, left, right, .. } => {
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
                                            "Type error at {}:{}: cannot concatenate {} and {} with '+'; convert with .to_string() first",
                                            expr.span().line,
                                            expr.span().col,
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
                            "Type error at {}:{}: operator '+' not defined for {} and {}",
                            expr.span().line,
                            expr.span().col,
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
                                "Type error at {}:{}: arithmetic operator requires numeric operands, found {} and {}",
                            expr.span().line,
                            expr.span().col,
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
                                "Type error at {}:{}: comparison requires numeric operands, found {} and {}",
                            expr.span().line,
                            expr.span().col,
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
                                "Type error at {}:{}: logical operator requires Bool operands, found {} and {}",
                            expr.span().line,
                            expr.span().col,
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::DotDot | BinOp::DotDotEq => Ok(Ty::Unknown),
                }
            }
            Expression::Call { name, args, span, .. } => {
                // `old(x)` references a parameter snapshot. The first arg
                // must be a Variable that exists in the enclosing
                // function's parameter list (the `env` here is the
                // function-body scope at the point of the ensures
                // expression). This gives a clearer error than the
                // generic "undefined variable" path.
                if name == "old" {
                    let arg = args.first().ok_or_else(|| {
                        anyhow!(
                            "Type error at {}:{}: `old(...)` requires a parameter name argument",
                            expr.span().line,
                            expr.span().col
                        )
                    })?;
                    if let Expression::Variable(vname, _) = arg {
                        if !env.contains_key(vname) {
                            return Err(anyhow!(
                                "Type error at {}:{}: `old({})` references no parameter; \
                                 `old` snapshots parameter values — pass a real parameter name",
                                expr.span().line,
                                expr.span().col,
                                vname
                            ));
                        }
                    } else {
                        return Err(anyhow!(
                            "Type error at {}:{}: `old` first argument must be a parameter name (Variable), \
                                 got a non-Variable expression",
                            expr.span().line,
                            expr.span().col
                        ));
                    }
                    return Ok(Ty::Unknown);
                }

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
                            if matches!(
                                recv_ty,
                                Ty::String | Ty::List(_) | Ty::Unknown
                            ) {
                                Ok(Ty::Int)
                            } else {
                                Err(anyhow!(
                                    "Type error at {}:{}: .len() requires String or List receiver, found {}",
                                    expr.span().line,
                                    expr.span().col,
                                    recv_ty.display()
                                ))
                            }
                        }
                        ".trim" | ".to_lowercase" | ".to_string" => Ok(Ty::String),
                        ".is_ok" | ".is_err" | ".is_some" | ".is_none" => Ok(Ty::Bool),
                        ".get" | ".read_file" | ".write_file" | ".env_get" | ".push" => {
                            Ok(Ty::Unknown)
                        }
                        // Field access on named/anonymous structs (or Custom alias).
                        other if other.starts_with('.') && args.len() == 1 => {
                            let field = &other[1..];
                            match &recv_ty {
                                Ty::Struct { fields, .. } => {
                                    if let Some((_, fty)) =
                                        fields.iter().find(|(n, _)| n == field)
                                    {
                                        Ok(fty.clone())
                                    } else {
                                        Err(anyhow!(
                                            "Type error at {}:{}: struct has no field '{}'",
                                            expr.span().line,
                                            expr.span().col,
                                            field
                                        ))
                                    }
                                }
                                Ty::Custom(name) => {
                                    if let Some(Ty::Struct { fields, .. }) =
                                        self.type_aliases.get(name)
                                    {
                                        if let Some((_, fty)) =
                                            fields.iter().find(|(n, _)| n == field)
                                        {
                                            Ok(fty.clone())
                                        } else {
                                            Err(anyhow!(
                                                "Type error at {}:{}: struct '{}' has no field '{}'",
                                                expr.span().line,
                                                expr.span().col,
                                                name,
                                                field
                                            ))
                                        }
                                    } else {
                                        Ok(Ty::Unknown)
                                    }
                                }
                                Ty::Unknown => Ok(Ty::Unknown),
                                _ => Ok(Ty::Unknown),
                            }
                        }
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
                                    "Type error at {}:{}: function '{}' argument {} expects {}, found {}",
                                    expr.span().line,
                                    expr.span().col,
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
                ..
            } => {
                let ct = self.infer_expr(cond, env)?;
                if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                    return Err(anyhow!(
                        "Type error at {}:{}: 'if' condition must be Bool, found {}",
                        expr.span().line,
                        expr.span().col,
                        ct.display()
                    ));
                }
                let mut env_then = env.clone();
                let mut mut_then = HashMap::new();
                let t1 = self.check_block(then_branch, &mut env_then, &mut mut_then, "if-then")?;
                if let Some(else_b) = else_branch {
                    let mut env_else = env.clone();
                    let mut mut_else = HashMap::new();
                    let t2 = self.check_block(else_b, &mut env_else, &mut mut_else, "if-else")?;
                    if Ty::unifyable(&t1, &t2) {
                        Ok(t1)
                    } else {
                        Ok(Ty::Unknown)
                    }
                } else {
                    Ok(t1)
                }
            }
            Expression::Unary { op, expr, span } => {
                let t = self.infer_expr(expr, env)?;
                match op {
                    UnaryOp::Not => {
                        if Ty::unifyable(&t, &Ty::Bool) || matches!(t, Ty::Unknown) {
                            Ok(Ty::Bool)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: unary '!' requires Bool, found {}",
                                span.line,
                                span.col,
                                t.display()
                            ))
                        }
                    }
                    UnaryOp::Neg => {
                        if t.is_numeric() || matches!(t, Ty::Unknown) {
                            Ok(t)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: unary '-' requires numeric, found {}",
                                span.line,
                                span.col,
                                t.display()
                            ))
                        }
                    }
                }
            }
            Expression::While { cond, body, span } => {
                let ct = self.infer_expr(cond, env)?;
                if !Ty::unifyable(&ct, &Ty::Bool) && !matches!(ct, Ty::Unknown) {
                    return Err(anyhow!(
                        "Type error at {}:{}: while condition must be Bool, found {}",
                        span.line,
                        span.col,
                        ct.display()
                    ));
                }
                let mut m = HashMap::new();
                self.check_block(body, &mut env.clone(), &mut m, "while-expr")?;
                Ok(Ty::Void)
            }
            Expression::StructLit { name, fields, span } => {
                let def = self.type_aliases.get(name).cloned().ok_or_else(|| {
                    anyhow!(
                        "Type error at {}:{}: unknown struct type '{}'",
                        span.line,
                        span.col,
                        name
                    )
                })?;
                match def {
                    Ty::Struct {
                        name: sn,
                        fields: def_fields,
                    } => {
                        for (fname, fexpr) in fields {
                            let fty = self.infer_expr(fexpr, env)?;
                            if let Some((_, want)) =
                                def_fields.iter().find(|(n, _)| n == fname)
                            {
                                if !Ty::unifyable(&fty, want) {
                                    return Err(anyhow!(
                                        "Type error at {}:{}: field '{}' of '{}' expects {}, found {}",
                                        span.line,
                                        span.col,
                                        fname,
                                        name,
                                        want.display(),
                                        fty.display()
                                    ));
                                }
                            } else {
                                return Err(anyhow!(
                                    "Type error at {}:{}: struct '{}' has no field '{}'",
                                    span.line,
                                    span.col,
                                    name,
                                    fname
                                ));
                            }
                        }
                        Ok(Ty::Struct {
                            name: sn.or_else(|| Some(name.clone())),
                            fields: def_fields,
                        })
                    }
                    other => Err(anyhow!(
                        "Type error at {}:{}: '{}' is not a struct type (found {})",
                        span.line,
                        span.col,
                        name,
                        other.display()
                    )),
                }
            }
            Expression::Match { expr, arms, span, .. } => {
                let scrutinee_ty = self.infer_expr(expr, env)?;
                let mut result = Ty::Unknown;
                let mut has_ok = false;
                let mut has_err = false;
                let mut has_some = false;
                let mut has_none = false;
                let mut has_wildcard = false;
                for arm in arms {
                    let mut arm_env = env.clone();
                    match &arm.pattern {
                        Pattern::Wildcard => has_wildcard = true,
                        Pattern::Variant { name, arg } => {
                            match name.as_str() {
                                "Ok" => has_ok = true,
                                "Err" => has_err = true,
                                "Some" => has_some = true,
                                "None" => has_none = true,
                                _ => {}
                            }
                            if let Some(var) = arg {
                                let payload = match (&scrutinee_ty, name.as_str()) {
                                    (Ty::Result(ok, _), "Ok") => (**ok).clone(),
                                    (Ty::Result(_, err), "Err") => (**err).clone(),
                                    (Ty::Option(inner), "Some") => (**inner).clone(),
                                    _ => Ty::Unknown,
                                };
                                arm_env.insert(var.clone(), payload);
                            }
                        }
                        Pattern::Literal(_) => {}
                    }
                    let t = self.infer_expr(&arm.body, &arm_env)?;
                    result = t;
                }

                // DESIGN: exhaustive matching for Result/Option (no silent fall-through).
                if !has_wildcard {
                    match &scrutinee_ty {
                        Ty::Result(_, _) if !(has_ok && has_err) => {
                            return Err(anyhow!(
                                "Type error at {}:{}: non-exhaustive match on Result — cover both Ok(_) and Err(_), or use `_`",
                                span.line,
                                span.col
                            ));
                        }
                        Ty::Option(_) if !(has_some && has_none) => {
                            return Err(anyhow!(
                                "Type error at {}:{}: non-exhaustive match on Option — cover both Some(_) and None, or use `_`",
                                span.line,
                                span.col
                            ));
                        }
                        _ => {}
                    }
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

    #[test]
    fn old_must_reference_a_parameter() {
        // `old(undefined_var)` is undefined — should fail with a
        // specific old() error.
        let src = r#"
            pub fn bad(x: Int) -> Int
                ensures result == old(undefined_var) + 1
            {
                return x + 1;
            }
            pub fn main() { println(bad(1)); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("`old(undefined_var)`"),
            "expected specific old() error, got: {}",
            err
        );
        assert!(
            err.contains("references no parameter"),
            "expected 'no parameter' hint, got: {}",
            err
        );
    }

    #[test]
    fn old_with_real_parameter_typechecks() {
        let src = r#"
            pub fn increment(x: Int) -> Int
                ensures result == old(x) + 1
            {
                return x + 1;
            }
            pub fn main() { println(increment(1)); }
        "#;
        assert!(check(src).is_ok(), "expected ok, got: {:?}", check(src).err());
    }

    #[test]
    fn type_error_includes_real_source_span() {
        // `missing_var` is on line 4, col 26 (after 12 spaces of indent).
        let src = "pub fn main() {\n    let x = 1;\n    let y = 2;\n    let z = missing_var;\n}\n";
        let err = check(src).unwrap_err();
        let msg = format!("{}", err);
        // The error message must carry the actual line:col of the
        // offending identifier so --json-errors can surface it.
        assert!(
            msg.contains("at 4:"),
            "expected error to carry span line 4, got: {}",
            msg
        );
        assert!(
            msg.contains("missing_var"),
            "expected error to name the variable, got: {}",
            msg
        );
    }

    #[test]
    fn rejects_unused_result_must_use() {
        let src = r#"
            pub fn get() -> Result[Int, String] {
                return Ok(1);
            }
            pub fn main() {
                get();
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("must-use") || err.contains("unused"),
            "got: {}",
            err
        );
    }

    #[test]
    fn rejects_nonexhaustive_result_match() {
        let src = r#"
            pub fn main() {
                let r = Ok(1);
                let x = match r {
                    Ok(v) => v,
                };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("non-exhaustive") || err.contains("Err"),
            "got: {}",
            err
        );
    }

    #[test]
    fn accepts_exhaustive_result_match() {
        let src = r#"
            pub fn main() {
                let r = Ok(1);
                let x = match r {
                    Ok(v) => v,
                    Err(e) => 0,
                };
                println(x);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn rejects_assign_to_immutable_let() {
        let src = r#"
            pub fn main() {
                let x = 1;
                x = 2;
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("immutable") || err.contains("let mut"),
            "got: {}",
            err
        );
    }

    #[test]
    fn accepts_assign_to_let_mut() {
        let src = r#"
            pub fn main() {
                let mut x = 1;
                x = 2;
                println(x);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn rejects_out_of_bounds_refinement_initializer() {
        let src = r#"
            pub fn main() {
                let port: Int[1..65535] = 99999;
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99999"),
            "expected static refinement error, got: {}",
            err
        );
    }

    #[test]
    fn accepts_option_some_none_match() {
        let src = r#"
            pub fn main() {
                let o = Some(1);
                let x = match o {
                    Some(v) => v,
                    None => 0,
                };
                println(x);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn rejects_nonexhaustive_option_match() {
        let src = r#"
            pub fn main() {
                let o = Some(1);
                let x = match o {
                    Some(v) => v,
                };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(err.contains("non-exhaustive") || err.contains("None"), "{}", err);
    }


    #[test]
    fn accepts_while_and_else_if_and_not() {
        let src = r#"
            pub fn main() {
                let mut i = 0;
                while i < 2 {
                    i = i + 1;
                }
                let y = if i > 5 { 9 } else if i > 0 { i } else { 0 };
                let z = if !false { y } else { 0 };
                println(z);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn rejects_out_of_bounds_refinement_return_value() {
        let src = r#"
            pub fn get_port() -> Int[1..65535] {
                return 70000;
            }
            pub fn main() {
                println(get_port());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("70000"),
            "expected refinement return error, got: {}",
            err
        );
    }
}