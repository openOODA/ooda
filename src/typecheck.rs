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

    /// Fail-closed unify: `Unknown` only unifies with `Unknown` (inference hole,
    /// not a wildcard). `Custom` only matches same name or a named struct alias.
    fn unifyable(a: &Ty, b: &Ty) -> bool {
        if a == b {
            return true;
        }
        // Unknown is not a soft-accept wildcard (was fail-open theater).
        if matches!(a, Ty::Unknown) && matches!(b, Ty::Unknown) {
            return true;
        }
        // Allow () (Void) capability tokens in verify blocks
        if (matches!(a, Ty::Void) && matches!(b, Ty::NetCap | Ty::FsCap | Ty::SysCap | Ty::EnvCap))
            || (matches!(b, Ty::Void) && matches!(a, Ty::NetCap | Ty::FsCap | Ty::SysCap | Ty::EnvCap))
        {
            return true;
        }
        match (a, b) {
            // ADT constructors (Ok/Err/Some) still use Unknown payloads until full
            // polymorphism exists — allow Unknown only *inside* Result/Option/List.
            (Ty::Result(a1, a2), Ty::Result(b1, b2)) => {
                Ty::unifyable_or_unknown_hole(a1, b1) && Ty::unifyable_or_unknown_hole(a2, b2)
            }
            (Ty::Option(a1), Ty::Option(b1)) => Ty::unifyable_or_unknown_hole(a1, b1),
            (Ty::List(a1), Ty::List(b1)) => Ty::unifyable_or_unknown_hole(a1, b1),
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
            (Ty::Custom(a), Ty::Custom(b)) => a == b,
            _ => false,
        }
    }

    /// Like unifyable, but Unknown on either side is a polymorphic hole (Ok/Err/Some).
    fn unifyable_or_unknown_hole(a: &Ty, b: &Ty) -> bool {
        matches!(a, Ty::Unknown) || matches!(b, Ty::Unknown) || Ty::unifyable(a, b)
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
        // Sealed effects (object-cap): first formal is the live handle; remaining
        // are operation args. Arity is enforced for user fns; these use concrete
        // counts so wrong call shape fails closed (no soft 1-arg theater).
        let res_s = Ty::Result(Box::new(Ty::String), Box::new(Ty::String));
        let res_v = Ty::Result(Box::new(Ty::Void), Box::new(Ty::String));
        for name in ["fetch", "downloadData", "http_get", "net_get", "net_connect"] {
            // (cap, url) or ambient-shaped still checked at cap pass
            tc.functions
                .insert(name.into(), (vec![Ty::Unknown, Ty::Unknown], res_s.clone()));
        }
        for name in ["read_file", "fs_read"] {
            tc.functions
                .insert(name.into(), (vec![Ty::Unknown, Ty::Unknown], res_s.clone()));
        }
        for name in ["env_get"] {
            tc.functions
                .insert(name.into(), (vec![Ty::Unknown, Ty::Unknown], res_s.clone()));
        }
        for name in ["write_file", "fs_write"] {
            // (cap, path, content)
            tc.functions.insert(
                name.into(),
                (vec![Ty::Unknown, Ty::Unknown, Ty::Unknown], res_v.clone()),
            );
        }
        for name in ["env_set"] {
            tc.functions.insert(
                name.into(),
                (vec![Ty::Unknown, Ty::Unknown, Ty::Unknown], res_v.clone()),
            );
        }
        for name in ["sys_exec", "exec", "spawn_process"] {
            // (cap, cmd) typical object-cap shape
            tc.functions
                .insert(name.into(), (vec![Ty::Unknown, Ty::Unknown], res_s.clone()));
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

        let expected_ret = Ty::from_ast(&func.return_type);
        let empty_refinements = HashMap::new();
        let body_ty = self.check_block(
            &func.body,
            &mut env,
            &mut mutable,
            &func.name,
            Some(&expected_ret),
            &empty_refinements,
        )?;

        // Static refinement bounds check for return statements against function return_type
        if let Type::Custom(ref s) = func.return_type {
            if let Some(rest) = s.strip_prefix("Int[").and_then(|str_s| str_s.strip_suffix("]")) {
                if let Some((min_s, max_s)) = rest.split_once("..") {
                    let min_v: i64 = min_s.parse().unwrap_or(i64::MIN);
                    let max_v: i64 = max_s.parse().unwrap_or(i64::MAX);
                    for stmt in &func.body.stmts {
                        if let Statement::Return(Some(expr), _) = stmt {
                            if let Some(val) = Ty::const_int(expr) {
                                if val < min_v || val > max_v {
                                    let sp = expr.span();
                                    return Err(anyhow!(
                                        "Type error at {}:{}: RefinementTypeViolation: Returned value {} out of refinement bounds [{}..{}] for return type of function '{}'",
                                        sp.line,
                                        sp.col,
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
        }

        let expected = Ty::from_ast(&func.return_type);
        // Fail-closed: expression-bodied functions must match declared return type.
        // (Statement returns checked in check_block Return arm below.)
        if !matches!(expected, Ty::Void)
            && !matches!(body_ty, Ty::Void | Ty::Unknown)
            && !Ty::unifyable(&body_ty, &expected)
        {
            return Err(anyhow!(
                "Type error in '{}': function declares return type {} but body has type {}",
                func.name,
                expected.display(),
                body_ty.display()
            ));
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
            let empty = HashMap::new();
            self.check_block(
                verify,
                &mut venv,
                &mut vmut,
                &format!("verify {}", func.name),
                None,
                &empty,
            )?;
        }

        Ok(())
    }

    /// Typecheck a block. `parent_refinements` carries `Int[lo..hi]` bounds from
    /// enclosing scopes so nested `if`/`while` still enforce assignment bounds.
    fn check_block(
        &self,
        block: &Block,
        env: &mut HashMap<String, Ty>,
        mutable: &mut HashMap<String, bool>,
        ctx: &str,
        expected_ret: Option<&Ty>,
        parent_refinements: &HashMap<String, (i64, i64)>,
    ) -> Result<Ty> {
        let mut last = Ty::Void;
        let mut refinements: HashMap<String, (i64, i64)> = parent_refinements.clone();
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
                    // DESIGN must-use: binding to `_` does not discharge Result/Option.
                    if name == "_" && matches!(init_ty, Ty::Result(_, _) | Ty::Option(_)) {
                        return Err(anyhow!(
                            "Type error at {}:{}: unused {} value (must-use); `let _ = ...` does not handle Result/Option — use `match` or `?`",
                            span.line,
                            span.col,
                            init_ty.display()
                        ));
                    }
                    if let Some(ann) = type_annotation {
                        let want = Ty::from_ast(ann);
                        if let Type::Custom(ref s) = ann {
                            if let Some(rest) = s.strip_prefix("Int[").and_then(|str_s| str_s.strip_suffix("]")) {
                                if let Some((min_s, max_s)) = rest.split_once("..") {
                                    let min_v: i64 = min_s.parse().unwrap_or(i64::MIN);
                                    let max_v: i64 = max_s.parse().unwrap_or(i64::MAX);
                                    refinements.insert(name.clone(), (min_v, max_v));
                                    if let Some(val) = Ty::const_int(init) {
                                        if val < min_v || val > max_v {
                                            let sp = init.span();
                                            return Err(anyhow!(
                                                "Type error at {}:{}: RefinementTypeViolation: Value {} out of refinement bounds [{}..{}] for '{}'",
                                                sp.line,
                                                sp.col,
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
                    if let Some(&(min_v, max_v)) = refinements.get(name) {
                        if let Some(val) = Ty::const_int(value) {
                            if val < min_v || val > max_v {
                                let sp = value.span();
                                return Err(anyhow!(
                                    "Type error at {}:{}: RefinementTypeViolation: Value {} out of refinement bounds [{}..{}] for assignment to '{}'",
                                    sp.line,
                                    sp.col,
                                    val,
                                    min_v,
                                    max_v,
                                    name
                                ));
                            }
                        }
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
                Statement::Return(Some(expr), span) => {
                    last = self.infer_expr(expr, env)?;
                    if let Some(exp) = expected_ret {
                        if !matches!(exp, Ty::Void)
                            && !matches!(last, Ty::Unknown)
                            && !Ty::unifyable(&last, exp)
                        {
                            return Err(anyhow!(
                                "Type error at {}:{} in '{}': return type {} does not match declared {}",
                                span.line,
                                span.col,
                                ctx,
                                last.display(),
                                exp.display()
                            ));
                        }
                    }
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
                            self.check_block(
                                then_branch,
                                env,
                                mutable,
                                "if-then",
                                expected_ret,
                                &refinements,
                            )?;
                            if let Some(eb) = else_branch {
                                self.check_block(
                                    eb,
                                    env,
                                    mutable,
                                    "if-else",
                                    expected_ret,
                                    &refinements,
                                )?;
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
                            self.check_block(
                                body,
                                env,
                                mutable,
                                "while-expr-stmt",
                                expected_ret,
                                &refinements,
                            )?;
                            last = Ty::Void;
                        }
                        _ => {
                            let t = self.infer_expr(expr, env)?;
                            // DESIGN must-use: discarded Result/Option is a hard error.
                            if matches!(t, Ty::Result(_, _) | Ty::Option(_)) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: unused {} value (must-use); handle with `match` / `?` — bare discard and `let _ = ...` are not enough",
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
                    self.check_block(
                        body,
                        env,
                        mutable,
                        "while-body",
                        expected_ret,
                        &refinements,
                    )?;
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
                    self.check_block(
                        then_branch,
                        env,
                        mutable,
                        "if-then-tail",
                        expected_ret,
                        &refinements,
                    )?;
                    if let Some(eb) = else_branch {
                        self.check_block(
                            eb,
                            env,
                            mutable,
                            "if-else-tail",
                            expected_ret,
                            &refinements,
                        )?;
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
                            if matches!(lt, Ty::String) && matches!(rt, Ty::Int | Ty::Float)
                                || matches!(rt, Ty::String) && matches!(lt, Ty::Int | Ty::Float)
                            {
                                return Err(anyhow!(
                                    "Type error at {}:{}: cannot concatenate {} and {} with '+'; convert with .to_string() first",
                                    expr.span().line,
                                    expr.span().col,
                                    lt.display(),
                                    rt.display()
                                ));
                            }
                            return Err(anyhow!(
                                "Type error at {}:{}: cannot apply '+' to {} and {}",
                                expr.span().line,
                                expr.span().col,
                                lt.display(),
                                rt.display()
                            ));
                        }
                        // Same-type numeric only — reject Int+Float (was typecheck-green, runtime trap).
                        if matches!((&lt, &rt), (Ty::Int, Ty::Int)) {
                            return Ok(Ty::Int);
                        }
                        if matches!((&lt, &rt), (Ty::Float, Ty::Float)) {
                            return Ok(Ty::Float);
                        }
                        return Err(anyhow!(
                            "Type error at {}:{}: arithmetic '+' requires matching numeric types (both Int or both Float) or String operands, found {} and {}",
                            expr.span().line,
                            expr.span().col,
                            lt.display(),
                            rt.display()
                        ));
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        // Same-type numeric only (Int+Float used to typecheck then trap at runtime).
                        if matches!((&lt, &rt), (Ty::Int, Ty::Int)) {
                            Ok(Ty::Int)
                        } else if matches!((&lt, &rt), (Ty::Float, Ty::Float)) {
                            Ok(Ty::Float)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: arithmetic operator requires matching numeric types (both Int or both Float), found {} and {}",
                                expr.span().line,
                                expr.span().col,
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::Eq | BinOp::Neq => {
                        // Fail-closed: no String == Int soft-Bool.
                        if Ty::unifyable(&lt, &rt) || (lt.is_numeric() && rt.is_numeric()) {
                            Ok(Ty::Bool)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: cannot compare {} and {} with equality",
                                expr.span().line,
                                expr.span().col,
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte => {
                        if matches!((&lt, &rt), (Ty::Int, Ty::Int) | (Ty::Float, Ty::Float)) {
                            Ok(Ty::Bool)
                        } else {
                            Err(anyhow!(
                                "Type error at {}:{}: comparison requires matching numeric types (both Int or both Float), found {} and {}",
                                expr.span().line,
                                expr.span().col,
                                lt.display(),
                                rt.display()
                            ))
                        }
                    }
                    BinOp::And | BinOp::Or => {
                        if Ty::unifyable(&lt, &Ty::Bool) && Ty::unifyable(&rt, &Ty::Bool) {
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
                    BinOp::DotDot | BinOp::DotDotEq => Ok(Ty::Int), // range sugar; not a full range type yet
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
                        if let Some(ty) = env.get(vname) {
                            return Ok(ty.clone());
                        }
                        return Err(anyhow!(
                            "Type error at {}:{}: `old({})` references no parameter; \
                             `old` snapshots parameter values — pass a real parameter name",
                            expr.span().line,
                            expr.span().col,
                            vname
                        ));
                    }
                    return Err(anyhow!(
                        "Type error at {}:{}: `old` first argument must be a parameter name (Variable), \
                             got a non-Variable expression",
                        expr.span().line,
                        expr.span().col
                    ));
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
                            if matches!(recv_ty, Ty::String | Ty::List(_)) {
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
                        ".get" | ".read_file" | ".env_get" => Ok(Ty::Result(
                            Box::new(Ty::String),
                            Box::new(Ty::String),
                        )),
                        ".write_file" => Ok(Ty::Result(
                            Box::new(Ty::Void),
                            Box::new(Ty::String),
                        )),
                        ".push" => Ok(Ty::List(Box::new(Ty::Unknown))),
                        // Field access only on struct types (fail-closed on Int/String/etc.).
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
                                        Err(anyhow!(
                                            "Type error at {}:{}: unknown type '{}' for field access '.{}'",
                                            expr.span().line,
                                            expr.span().col,
                                            name,
                                            field
                                        ))
                                    }
                                }
                                Ty::Unknown => Ok(Ty::Unknown),
                                other_ty => Err(anyhow!(
                                    "Type error at {}:{}: unknown method '{}' on {}",
                                    expr.span().line,
                                    expr.span().col,
                                    other,
                                    other_ty.display()
                                )),
                            }
                        }
                        other => Err(anyhow!(
                            "Type error at {}:{}: unknown method '{}'",
                            expr.span().line,
                            expr.span().col,
                            other
                        )),
                    };
                }

                let mut arg_tys = Vec::new();
                for a in args {
                    arg_tys.push(self.infer_expr(a, env)?);
                }

                // ADT constructors: payload-driven Result/Option (cuts match-arm Unknown vs Int).
                if name == "Ok" {
                    if arg_tys.len() != 1 {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'Ok' expects 1 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    return Ok(Ty::Result(
                        Box::new(arg_tys[0].clone()),
                        Box::new(Ty::Unknown),
                    ));
                }
                if name == "Err" {
                    if arg_tys.len() != 1 {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'Err' expects 1 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    return Ok(Ty::Result(
                        Box::new(Ty::Unknown),
                        Box::new(arg_tys[0].clone()),
                    ));
                }
                if name == "Some" {
                    if arg_tys.len() != 1 {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'Some' expects 1 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    return Ok(Ty::Option(Box::new(arg_tys[0].clone())));
                }

                // assert_eq(a, b): require comparable types (no soft Unknown-only).
                if name == "assert_eq" {
                    if arg_tys.len() != 2 {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'assert_eq' expects 2 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    let (a, b) = (&arg_tys[0], &arg_tys[1]);
                    if !Ty::unifyable(a, b)
                        && !(matches!(a, Ty::Unknown) && matches!(b, Ty::Unknown))
                    {
                        return Err(anyhow!(
                            "Type error at {}:{}: assert_eq arguments must have matching types, found {} and {}",
                            expr.span().line,
                            expr.span().col,
                            a.display(),
                            b.display()
                        ));
                    }
                    return Ok(Ty::Void);
                }

                if let Some((params, ret)) = self.functions.get(name) {
                    // println is varargs at runtime (prints every arg).
                    let is_println = name == "println";
                    if !is_println && params.len() != arg_tys.len() {
                        return Err(anyhow!(
                            "Type error at {}:{}: function '{}' expects {} argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            name,
                            params.len(),
                            arg_tys.len()
                        ));
                    }
                    let n = params.len().min(arg_tys.len());
                    for (i, (pt, at)) in params.iter().zip(arg_tys.iter()).take(n).enumerate() {
                        // Unknown in builtin signatures is a polymorphic hole, not a wildcard
                        // for user annotations (those still fail-closed via unifyable).
                        if !Ty::unifyable_or_unknown_hole(pt, at) {
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
                    return Ok(ret.clone());
                }

                // Fail-closed: unknown free functions must not soft-accept as Ty::Unknown.
                // (Methods and registered builtins are handled above.)
                Err(anyhow!(
                    "Type error at {}:{}: undefined function '{}'",
                    expr.span().line,
                    expr.span().col,
                    name
                ))
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
                // Expression-level if has no parent refinement map here; empty is correct
                // (statement-level if inherits via check_block's refinements param).
                let empty_ref = HashMap::new();
                let t1 = self.check_block(
                    then_branch,
                    &mut env_then,
                    &mut mut_then,
                    "if-then",
                    None,
                    &empty_ref,
                )?;
                if let Some(else_b) = else_branch {
                    let mut env_else = env.clone();
                    let mut mut_else = HashMap::new();
                    let t2 = self.check_block(
                        else_b,
                        &mut env_else,
                        &mut mut_else,
                        "if-else",
                        None,
                        &empty_ref,
                    )?;
                    if Ty::unifyable(&t1, &t2) {
                        Ok(t1)
                    } else if matches!(t1, Ty::Unknown) || matches!(t2, Ty::Unknown) {
                        Ok(Ty::Unknown)
                    } else if matches!(t1, Ty::Void) {
                        // Statement-like then-arm (e.g. nested if-as-stmt in else branch)
                        Ok(t2)
                    } else if matches!(t2, Ty::Void) {
                        Ok(t1)
                    } else {
                        Err(anyhow!(
                            "Type error at {}:{}: if/else branches have incompatible types {} vs {}",
                            expr.span().line,
                            expr.span().col,
                            t1.display(),
                            t2.display()
                        ))
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
                let empty_ref = HashMap::new();
                self.check_block(body, &mut env.clone(), &mut m, "while-expr", None, &empty_ref)?;
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
                let mut result: Option<Ty> = None;
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
                    match &result {
                        None => result = Some(t),
                        Some(prev) => {
                            // Unknown holes from Ok/Some constructors may unify with concrete arms.
                            if !Ty::unifyable_or_unknown_hole(prev, &t)
                                && !(matches!(prev, Ty::Void) || matches!(t, Ty::Void))
                            {
                                return Err(anyhow!(
                                    "Type error at {}:{}: match arms have incompatible types {} vs {}",
                                    span.line,
                                    span.col,
                                    prev.display(),
                                    t.display()
                                ));
                            }
                            // Prefer concrete type over Unknown/Void when possible.
                            if matches!(prev, Ty::Unknown | Ty::Void) && !matches!(t, Ty::Unknown | Ty::Void)
                            {
                                result = Some(t);
                            }
                        }
                    }
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
                Ok(result.unwrap_or(Ty::Void))
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
    fn rejects_let_underscore_result_must_use() {
        let src = r#"
            pub fn get() -> Result[Int, String] {
                return Ok(1);
            }
            pub fn main() {
                let _ = get();
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

    #[test]
    fn rejects_if_else_branch_type_mismatch() {
        let src = r#"
            pub fn main() {
                let x = if true { 1 } else { "nope" };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("incompatible types") || err.contains("if/else"),
            "got: {}",
            err
        );
    }

    #[test]
    fn rejects_unknown_method_on_int() {
        let src = r#"
            pub fn main() {
                let x = 1;
                let y = x.totally_fake();
                println(y);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("unknown method") && err.contains("totally_fake"),
            "got: {}",
            err
        );
    }

    #[test]
    fn rejects_return_type_mismatch() {
        let src = r#"
            pub fn bad() -> Int {
                return "hi";
            }
            pub fn main() {
                println(bad());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("return type") || err.contains("does not match"),
            "got: {}",
            err
        );
    }

    #[test]
    fn rejects_undefined_function_fail_closed() {
        let src = r#"
            pub fn main() {
                let x = totally_missing_builtin(1);
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("undefined function") && err.contains("totally_missing_builtin"),
            "expected undefined function error, got: {}",
            err
        );
    }

    #[test]
    fn fetch_is_typed_as_result() {
        let src = r#"
            pub fn ok(net: &NetCap) {
                let r = fetch(net, "https://example.invalid");
                assert_eq!(r.is_err(), true);
            }
            pub fn main(net: &NetCap) {
                ok(net);
            }
        "#;
        // Must typecheck: fetch returns Result so .is_err is valid
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn rejects_out_of_bounds_refinement_assignment() {
        let src = r#"
            pub fn main() {
                let mut port: Int[1..65535] = 8080;
                port = 70000;
                println(port);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("70000"),
            "expected assignment refinement error, got: {}",
            err
        );
    }

    #[test]
    fn rejects_out_of_bounds_refinement_assignment_in_nested_if() {
        let src = r#"
            pub fn main() {
                let mut port: Int[1..65535] = 8080;
                if true {
                    port = 70000;
                }
                println(port);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("70000"),
            "nested if must still enforce refinement bounds, got: {}",
            err
        );
    }

    #[test]
    fn rejects_out_of_bounds_refinement_assignment_in_while() {
        let src = r#"
            pub fn main() {
                let mut port: Int[1..65535] = 8080;
                while false {
                    port = 0;
                }
                println(port);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("0"),
            "while body must still enforce refinement bounds, got: {}",
            err
        );
    }

    #[test]
    fn rejects_const_expr_out_of_refinement_bounds() {
        let src = r#"
            pub fn main() {
                let port: Int[1..10] = 5 + 6;
                println(port);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("11"),
            "const-folded init must enforce refinement, got: {}",
            err
        );
    }

    #[test]
    fn rejects_int_as_string_return_fail_closed() {
        let src = r#"
            pub fn bad(x: Int) -> String {
                return x;
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("return type") || err.contains("does not match"),
            "Int must not soft-accept as String, got: {}",
            err
        );
    }

    #[test]
    fn rejects_string_eq_int() {
        let src = r#"
            pub fn main() {
                let b = "a" == 1;
                println(b);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("cannot compare") || err.contains("equality"),
            "String == Int must fail, got: {}",
            err
        );
    }

    #[test]
    fn rejects_call_arity_too_few() {
        let src = r#"
            pub fn add(a: Int, b: Int) -> Int {
                return a + b;
            }
            pub fn main() {
                let x = add(1);
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("expects 2 argument") || err.contains("found 1"),
            "arity too few must fail closed, got: {}",
            err
        );
    }

    #[test]
    fn rejects_call_arity_too_many() {
        let src = r#"
            pub fn add(a: Int, b: Int) -> Int {
                return a + b;
            }
            pub fn main() {
                let x = add(1, 2, 3);
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("expects 2 argument") || err.contains("found 3"),
            "arity too many must fail closed, got: {}",
            err
        );
    }

    #[test]
    fn rejects_zero_param_call_with_args() {
        let src = r#"
            pub fn conf() -> Int {
                return 1;
            }
            pub fn main() {
                let x = conf(99);
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("expects 0 argument") || err.contains("found 1"),
            "zero-param fn with args must fail, got: {}",
            err
        );
    }

    #[test]
    fn println_varargs_still_typechecks() {
        let src = r#"
            pub fn main() {
                println("a", 1);
                println(1);
                println();
            }
        "#;
        assert!(
            check(src).is_ok(),
            "println is varargs: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn rejects_int_plus_float_mixed_arith() {
        let src = r#"
            pub fn main() {
                let x = 1 + 2.0;
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("matching numeric")
                || err.contains("Float")
                || err.contains("Int"),
            "Int+Float must fail at typecheck (was runtime trap), got: {}",
            err
        );
    }

    #[test]
    fn rejects_match_arms_int_vs_string() {
        let src = r#"
            pub fn main() {
                let r: Result[Int, String] = Ok(1);
                let x = match r {
                    Ok(v) => v,
                    Err(e) => e,
                };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("incompatible types") || err.contains("match arms"),
            "Ok(Int) vs Err(String) arms must fail, got: {}",
            err
        );
    }

    #[test]
    fn rejects_assert_eq_mismatched_types() {
        let src = r#"
            pub fn main() {
                assert_eq(1, "x");
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("assert_eq")
                && (err.contains("matching") || err.contains("String") || err.contains("Int")),
            "assert_eq(Int, String) must fail, got: {}",
            err
        );
    }

    #[test]
    fn fetch_with_cap_arg_still_typechecks() {
        let src = r#"
            pub fn ok(net: &NetCap) {
                let r = fetch(net, "https://example.invalid");
                assert_eq(r.is_err(), true);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "fetch(net, url) must typecheck: {:?}",
            check(src).err()
        );
    }
}
