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
}

/// Parse `Int[lo..hi]` refinement bounds from a type annotation.
pub fn int_refinement_bounds(ty: &Type) -> Option<(i64, i64)> {
    if let Type::Custom(s) = ty {
        if let Some(rest) = s.strip_prefix("Int[").and_then(|r| r.strip_suffix(']')) {
            if let Some((min_s, max_s)) = rest.split_once("..") {
                let min_v: i64 = min_s.parse().ok()?;
                let max_v: i64 = max_s.parse().ok()?;
                return Some((min_v, max_v));
            }
        }
    }
    None
}

impl TypeChecker {
    fn unify(&self, a: &Ty, b: &Ty) -> bool {
        Ty::unifyable_with_aliases(a, b, &self.type_aliases)
    }

    fn unify_or_hole(&self, a: &Ty, b: &Ty) -> bool {
        Ty::unifyable_or_unknown_hole_with_aliases(a, b, &self.type_aliases)
    }

    fn norm(&self, t: &Ty) -> Ty {
        t.normalize(&self.type_aliases)
    }

    /// `Int[lo..hi]` or a type alias that expands to one (`type Port = Int[1..10]`).
    fn bounds_from_type_ann(&self, ann: &Type) -> Option<(i64, i64)> {
        int_refinement_bounds(ann).or_else(|| {
            if let Type::Custom(name) = ann {
                self.alias_refinements.get(name).copied()
            } else {
                None
            }
        })
    }

    pub fn check_program(program: &Program) -> Result<()> {
        let mut tc = TypeChecker {
            functions: HashMap::new(),
            param_refinements: HashMap::new(),
            type_aliases: HashMap::new(),
            alias_refinements: HashMap::new(),
            active_list_lens: std::cell::RefCell::new(HashMap::new()),
            current_return: std::cell::RefCell::new(None),
        };

        // Collect type aliases first (named structs for StructLit).
        for item in &program.items {
            if let Item::TypeAlias(name, ty) = item {
                if let Some(b) = int_refinement_bounds(ty) {
                    tc.alias_refinements.insert(name.clone(), b);
                }
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
        // Host helpers: object-cap shape (live handle first, then op args).
        let res_v_host = Ty::Result(Box::new(Ty::Void), Box::new(Ty::String));
        let res_s_host = Ty::Result(Box::new(Ty::String), Box::new(Ty::String));
        for n in ["mkdir_p", "chmod_exec"] {
            // (cap, path)
            tc.functions
                .insert(n.into(), (vec![Ty::Unknown, Ty::Unknown], res_v_host.clone()));
        }
        for n in ["copy_file", "http_download", "extract_tar_gz"] {
            // (cap, a, b)
            tc.functions.insert(
                n.into(),
                (
                    vec![Ty::Unknown, Ty::Unknown, Ty::Unknown],
                    res_v_host.clone(),
                ),
            );
        }
        // path_exists(cap, path) — Bool, not Result
        tc.functions.insert(
            "path_exists".into(),
            (vec![Ty::Unknown, Ty::Unknown], Ty::Bool),
        );
        // sys_exec is varargs at runtime; registered for lookup, arity special-cased below.
        tc.functions.insert(
            "sys_exec".into(),
            (
                vec![Ty::Unknown, Ty::Unknown],
                res_s_host.clone(),
            ),
        );
        tc.functions.insert(
            "exec".into(),
            (
                vec![Ty::Unknown, Ty::Unknown],
                res_s_host.clone(),
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
            (vec![Ty::Unknown, Ty::Unknown], Ty::String),
        );
        tc.functions.insert(
            "async_join_internal".into(),
            (
                vec![Ty::Unknown, Ty::Unknown],
                Ty::Result(Box::new(Ty::String), Box::new(Ty::String)),
            ),
        );
        tc.functions.insert(
            "python_embed_internal".into(),
            (
                vec![Ty::Unknown, Ty::Unknown],
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
                let bounds: Vec<Option<(i64, i64)>> = f
                    .params
                    .iter()
                    .map(|p| {
                        int_refinement_bounds(&p.param_type).or_else(|| {
                            if let Type::Custom(name) = &p.param_type {
                                tc.alias_refinements.get(name).copied()
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                tc.functions.insert(f.name.clone(), (params, ret));
                tc.param_refinements.insert(f.name.clone(), bounds);
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
        let ret_bounds = self.bounds_from_type_ann(&func.return_type);
        *self.current_return.borrow_mut() = Some(expected_ret.clone());
        let body_ty = self.check_block(
            &func.body,
            &mut env,
            &mut mutable,
            &func.name,
            Some(&expected_ret),
            &empty_refinements,
            ret_bounds,
        );
        *self.current_return.borrow_mut() = None;
        let body_ty = body_ty?;

        let expected = Ty::from_ast(&func.return_type);
        // Fail-closed: non-Void functions must produce a value on every path.
        // Body type Void is OK only when every path hits `return <expr>` (if/else, etc.).
        if !matches!(expected, Ty::Void) {
            if matches!(body_ty, Ty::Void) {
                if !block_always_returns(&func.body) {
                    return Err(anyhow!(
                        "Type error in '{}': function declares return type {} but body has type Void (missing return value)",
                        func.name,
                        expected.display()
                    ));
                }
                // All paths return; per-return types already checked in check_block.
            } else if !matches!(body_ty, Ty::Unknown) && !self.unify(&body_ty, &expected) {
                return Err(anyhow!(
                    "Type error in '{}': function declares return type {} but body has type {}",
                    func.name,
                    expected.display(),
                    body_ty.display()
                ));
            }
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
                None,
            )?;
        }

        Ok(())
    }


    /// Const length of list_new / list_push chains, using env of known binding lengths.
    fn const_list_len(expr: &Expression, env_lens: &HashMap<String, i64>) -> Option<i64> {
        match expr {
            Expression::Call { name, args, .. } if name == "list_new" && args.is_empty() => Some(0),
            Expression::Call { name, args, .. } if name == "list_push" && args.len() == 2 => {
                Self::const_list_len(&args[0], env_lens).map(|n| n + 1)
            }
            Expression::Variable(name, _) => env_lens.get(name).copied(),
            _ => None,
        }
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
        // Const return-type Int[lo..hi] bounds (incl. aliases); enforced on every return + tail.
        return_bounds: Option<(i64, i64)>,
    ) -> Result<Ty> {
        let mut last = Ty::Void;
        let mut refinements: HashMap<String, (i64, i64)> = parent_refinements.clone();
        // Const list lengths for list_new / list_push chains (fail-closed list_get OOB).
        let mut list_lens: HashMap<String, i64> = HashMap::new();
        let mut path_returned = false;
        // Sync for list_get const checks inside infer_expr.
        *self.active_list_lens.borrow_mut() = list_lens.clone();

        for stmt in &block.stmts {
            if path_returned {
                let sp = stmt_span(stmt);
                return Err(anyhow!(
                    "Type error at {}:{}: unreachable code after return",
                    sp.line,
                    sp.col
                ));
            }
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
                    // Fail-closed: do not bind Void (e.g. `let x = while …` / discarded unit).
                    if matches!(init_ty, Ty::Void) && name != "_" {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot bind Void value to '{}'; while/if-as-stmt produce Void — use a value expression",
                            span.line,
                            span.col,
                            name
                        ));
                    }
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
                        // Bare Int[lo..hi] or type alias that carries those bounds.
                        if let Some((min_v, max_v)) = self.bounds_from_type_ann(ann) {
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
                        if !self.unify(&init_ty, &want) {
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
                    if let Some(len) = Self::const_list_len(init, &list_lens) {
                        list_lens.insert(name.clone(), len);
                    } else {
                        list_lens.remove(name);
                    }
                    *self.active_list_lens.borrow_mut() = list_lens.clone();
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
                    if !self.unify(&vty, &want) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign {} to '{}' of type {}",
                            span.line,
                            span.col,
                            vty.display(),
                            name,
                            want.display()
                        ));
                    }
                    if let Some(len) = Self::const_list_len(value, &list_lens) {
                        list_lens.insert(name.clone(), len);
                    } else {
                        list_lens.remove(name);
                    }
                    *self.active_list_lens.borrow_mut() = list_lens.clone();
                    last = Ty::Void;
                }
                Statement::FieldAssign {
                    object,
                    field,
                    value,
                    span,
                } => {
                    let obj_ty = self.infer_expr(object, env)?;
                    // Mutability: only `let mut` root Variable may be field-assigned.
                    match object {
                        Expression::Variable(name, _) => {
                            if !mutable.get(name).copied().unwrap_or(false) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: cannot assign to field of immutable binding '{}'; use `let mut {}`",
                                    span.line,
                                    span.col,
                                    name,
                                    name
                                ));
                            }
                        }
                        _ => {
                            return Err(anyhow!(
                                "Type error at {}:{}: field assign requires a simple variable receiver (e.g. p.x = …)",
                                span.line,
                                span.col
                            ));
                        }
                    }
                    let fields = match &obj_ty {
                        Ty::Struct { fields, .. } => fields.clone(),
                        Ty::Custom(n) => match self.type_aliases.get(n) {
                            Some(Ty::Struct { fields, .. }) => fields.clone(),
                            _ => {
                                return Err(anyhow!(
                                    "Type error at {}:{}: field assign on non-struct type {}",
                                    span.line,
                                    span.col,
                                    obj_ty.display()
                                ));
                            }
                        },
                        other => {
                            return Err(anyhow!(
                                "Type error at {}:{}: field assign on non-struct type {}",
                                span.line,
                                span.col,
                                other.display()
                            ));
                        }
                    };
                    let want = fields
                        .iter()
                        .find(|(n, _)| n == field)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| {
                            anyhow!(
                                "Type error at {}:{}: struct has no field '{}'",
                                span.line,
                                span.col,
                                field
                            )
                        })?;
                    let vty = self.infer_expr(value, env)?;
                    if !self.unify(&vty, &want) {
                        return Err(anyhow!(
                            "Type error at {}:{}: cannot assign {} to field '{}' of type {}",
                            span.line,
                            span.col,
                            vty.display(),
                            field,
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
                            && !self.unify(&last, exp)
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
                    if let Some((min_v, max_v)) = return_bounds {
                        if let Some(val) = Ty::const_int(expr) {
                            if val < min_v || val > max_v {
                                let sp = expr.span();
                                return Err(anyhow!(
                                    "Type error at {}:{}: RefinementTypeViolation: Returned value {} out of refinement bounds [{}..{}] for return type in '{}'",
                                    sp.line,
                                    sp.col,
                                    val,
                                    min_v,
                                    max_v,
                                    ctx
                                ));
                            }
                        }
                    }
                    path_returned = true;
                }
                Statement::Return(None, _) => {
                    last = Ty::Void;
                    path_returned = true;
                }
                Statement::Expr(expr, span) => {
                    // Statement-level if/while must inherit mutability so
                    // `let mut x` can be assigned inside branches (CHS oodac).
                    // Nested blocks clone env/mutable so `let` bindings do not
                    // leak into the outer scope (assign to outer mut still works).
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
                            let mut env_then = env.clone();
                            let mut mut_then = mutable.clone();
                            self.check_block(
                                then_branch,
                                &mut env_then,
                                &mut mut_then,
                                "if-then",
                                expected_ret,
                                &refinements,
                                return_bounds,
                            )?;
                            if let Some(eb) = else_branch {
                                let mut env_else = env.clone();
                                let mut mut_else = mutable.clone();
                                self.check_block(
                                    eb,
                                    &mut env_else,
                                    &mut mut_else,
                                    "if-else",
                                    expected_ret,
                                    &refinements,
                                    return_bounds,
                                )?;
                            }
                            last = Ty::Void;
                            if expr_paths_return(expr) {
                                path_returned = true;
                            }
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
                            let mut env_w = env.clone();
                            let mut mut_w = mutable.clone();
                            self.check_block(
                                body,
                                &mut env_w,
                                &mut mut_w,
                                "while-expr-stmt",
                                expected_ret,
                                &refinements,
                                return_bounds,
                            )?;
                            last = Ty::Void;
                        }
                        _ => {
                            let t = self.infer_expr_m(expr, env, mutable)?;
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
                    let mut env_w = env.clone();
                    let mut mut_w = mutable.clone();
                    self.check_block(
                        body,
                        &mut env_w,
                        &mut mut_w,
                        "while-body",
                        expected_ret,
                        &refinements,
                        return_bounds,
                    )?;
                    last = Ty::Void;
                }
            }
        }
        if let Some(expr) = &block.expr {
            if path_returned {
                let sp = expr.span();
                return Err(anyhow!(
                    "Type error at {}:{}: unreachable code after return",
                    sp.line,
                    sp.col
                ));
            }
            // Tail expression may be a nested `else if` chain or match.
            // Clone env/mutable per branch so `else if` desugar cannot leak
            // sibling `let`s (else if is a nested if as the else block's tail).
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
                    let mut env_then = env.clone();
                    let mut mut_then = mutable.clone();
                    self.check_block(
                        then_branch,
                        &mut env_then,
                        &mut mut_then,
                        "if-then-tail",
                        expected_ret,
                        &refinements,
                        return_bounds,
                    )?;
                    if let Some(eb) = else_branch {
                        let mut env_else = env.clone();
                        let mut mut_else = mutable.clone();
                        self.check_block(
                            eb,
                            &mut env_else,
                            &mut mut_else,
                            "if-else-tail",
                            expected_ret,
                            &refinements,
                            return_bounds,
                        )?;
                    }
                    last = Ty::Void;
                }
                Expression::Match { arms, span: mspan, .. } => {
                    last = self.infer_expr_m(expr, env, mutable)?;
                    // Const arm values as implicit return: enforce Int[lo..hi].
                    if let Some((min_v, max_v)) = return_bounds {
                        for arm in arms {
                            if let Some(val) = Ty::const_int(&arm.body) {
                                if val < min_v || val > max_v {
                                    let sp = arm.body.span();
                                    return Err(anyhow!(
                                        "Type error at {}:{}: RefinementTypeViolation: Returned value {} out of refinement bounds [{}..{}] for match arm return type in '{}' (match at {}:{})",
                                        sp.line,
                                        sp.col,
                                        val,
                                        min_v,
                                        max_v,
                                        ctx,
                                        mspan.line,
                                        mspan.col
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {
                    last = self.infer_expr_m(expr, env, mutable)?;
                    // Tail expression as implicit return: enforce Int[lo..hi] when applicable.
                    if let Some((min_v, max_v)) = return_bounds {
                        if let Some(val) = Ty::const_int(expr) {
                            if val < min_v || val > max_v {
                                let sp = expr.span();
                                return Err(anyhow!(
                                    "Type error at {}:{}: RefinementTypeViolation: Returned value {} out of refinement bounds [{}..{}] for return type in '{}'",
                                    sp.line,
                                    sp.col,
                                    val,
                                    min_v,
                                    max_v,
                                    ctx
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(last)
    }

    fn infer_expr(&self, expr: &Expression, env: &HashMap<String, Ty>) -> Result<Ty> {
        let empty_mut = HashMap::new();
        self.infer_expr_m(expr, env, &empty_mut)
    }

    fn infer_expr_m(
        &self,
        expr: &Expression,
        env: &HashMap<String, Ty>,
        mutable: &HashMap<String, bool>,
    ) -> Result<Ty> {
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
                // Normalize type aliases (`type Port = Int`) before numeric/string shape checks.
                let ln = self.norm(&lt);
                let rn = self.norm(&rt);
                match op {
                    BinOp::Add => {
                        if matches!(ln, Ty::String) || matches!(rn, Ty::String) {
                            if matches!(ln, Ty::String) && matches!(rn, Ty::String) {
                                return Ok(Ty::String);
                            }
                            if matches!(ln, Ty::String) && matches!(rn, Ty::Int | Ty::Float)
                                || matches!(rn, Ty::String) && matches!(ln, Ty::Int | Ty::Float)
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
                        if matches!((&ln, &rn), (Ty::Int, Ty::Int)) {
                            return Ok(Ty::Int);
                        }
                        if matches!((&ln, &rn), (Ty::Float, Ty::Float)) {
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
                        if matches!(op, BinOp::Div) {
                            if let (Some(_), Some(0)) = (Ty::const_int(left), Ty::const_int(right)) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: integer division by zero",
                                    expr.span().line,
                                    expr.span().col
                                ));
                            }
                            if let (
                                Expression::Literal(Literal::Float(_), _),
                                Expression::Literal(Literal::Float(r), _),
                            ) = (left.as_ref(), right.as_ref())
                            {
                                if *r == 0.0 {
                                    return Err(anyhow!(
                                        "Type error at {}:{}: float division by zero",
                                        expr.span().line,
                                        expr.span().col
                                    ));
                                }
                            }
                        }
                        if matches!((&ln, &rn), (Ty::Int, Ty::Int)) {
                            Ok(Ty::Int)
                        } else if matches!((&ln, &rn), (Ty::Float, Ty::Float)) {
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
                        // Fail-closed: matching types only (no Int == Float soft-Bool).
                        // Aliases normalize so `Port == Int` works when Port = Int.
                        if self.unify(&lt, &rt)
                            || matches!((&ln, &rn), (Ty::Int, Ty::Int) | (Ty::Float, Ty::Float))
                        {
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
                        if matches!((&ln, &rn), (Ty::Int, Ty::Int) | (Ty::Float, Ty::Float)) {
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
                        if matches!(ln, Ty::Bool) && matches!(rn, Ty::Bool) {
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
            Expression::Call {
                name,
                args,
                span,
                propagate_err,
                ..
            } => {
                // Apply `?`: Result[T, E] → T. Only legal in Result-returning functions.
                let apply_try = |ty: Ty| -> Result<Ty> {
                    if !*propagate_err {
                        return Ok(ty);
                    }
                    let Ty::Result(ok, err) = ty else {
                        return Err(anyhow!(
                            "Type error at {}:{}: `?` requires Result, found {}",
                            span.line,
                            span.col,
                            ty.display()
                        ));
                    };
                    let encl = self.current_return.borrow().clone();
                    match encl {
                        Some(Ty::Result(_, e_err)) => {
                            if !self.unify_or_hole(&err, &e_err)
                                && !matches!(*err, Ty::Unknown)
                                && !matches!(*e_err, Ty::Unknown)
                            {
                                return Err(anyhow!(
                                    "Type error at {}:{}: `?` error type {} does not match function Err type {}",
                                    span.line,
                                    span.col,
                                    err.display(),
                                    e_err.display()
                                ));
                            }
                            Ok(*ok)
                        }
                        Some(other) => Err(anyhow!(
                            "Type error at {}:{}: `?` only allowed in functions returning Result, found return type {}",
                            span.line,
                            span.col,
                            other.display()
                        )),
                        None => Err(anyhow!(
                            "Type error at {}:{}: `?` only allowed inside a function body",
                            span.line,
                            span.col
                        )),
                    }
                };
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

                // Methods: .len, .trim, sealed object-cap methods, etc.
                // `args[0]` is the receiver (desugared).
                if name.starts_with('.') {
                    let recv = args
                        .first()
                        .ok_or_else(|| anyhow!("Type error: method '{}' missing receiver", name))?;
                    let recv_ty = self.infer_expr(recv, env)?;
                    let mut method_arg_tys = Vec::new();
                    for a in args.iter().skip(1) {
                        method_arg_tys.push(self.infer_expr(a, env)?);
                    }
                    // Object-cap method arities (including receiver). Fail-closed.
                    let method_arity_ok = match name.as_str() {
                        ".write_file" => args.len() == 3, // recv, path, content
                        ".read_file" | ".env_get" | ".get" | ".sys_exec" | ".contains" => args.len() == 2, // recv, arg
                        ".len" | ".trim" | ".to_lowercase" | ".to_string"
                        | ".is_ok" | ".is_err" | ".is_some" | ".is_none" => args.len() == 1,
                        ".char_at" => args.len() == 2, // recv, index
                        ".str_slice" => args.len() == 3, // recv, start, end
                        ".push" => args.len() == 2,
                        _ => true, // field access / unknown handled below
                    };
                    if !method_arity_ok {
                        let expected = match name.as_str() {
                            ".write_file" => 3,
                            ".str_slice" => 3,
                            ".read_file" | ".env_get" | ".get" | ".push" | ".char_at" | ".sys_exec" | ".contains" => 2,
                            _ => 1,
                        };
                        return Err(anyhow!(
                            "Type error at {}:{}: function '{}' expects {} argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            name,
                            expected,
                            args.len()
                        ));
                    }
                    let method_ty = match name.as_str() {
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
                        ".char_at" => {
                            if !matches!(recv_ty, Ty::String) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: .char_at requires String receiver, found {}",
                                    expr.span().line,
                                    expr.span().col,
                                    recv_ty.display()
                                ));
                            }
                            let idx_ty = method_arg_tys.first().cloned().unwrap_or(Ty::Unknown);
                            if !self.unify_or_hole(&idx_ty, &Ty::Int) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: .char_at index expects Int, found {}",
                                    expr.span().line,
                                    expr.span().col,
                                    idx_ty.display()
                                ));
                            }
                            // Const bounds when receiver is a string literal.
                            if let (Some(s), Some(idx)) = (
                                args.first().and_then(|a| Ty::const_str(a)),
                                args.get(1).and_then(|a| Ty::const_int(a)),
                            ) {
                                let len = s.chars().count() as i64;
                                if idx < 0 || idx >= len {
                                    return Err(anyhow!(
                                        "Type error at {}:{}: char_at index {} out of bounds for string literal of length {} (const bounds check)",
                                        expr.span().line,
                                        expr.span().col,
                                        idx,
                                        len
                                    ));
                                }
                            }
                            Ok(Ty::String)
                        }
                        ".str_slice" => {
                            if !matches!(recv_ty, Ty::String) {
                                return Err(anyhow!(
                                    "Type error at {}:{}: .str_slice requires String receiver, found {}",
                                    expr.span().line,
                                    expr.span().col,
                                    recv_ty.display()
                                ));
                            }
                            for (i, expect_name) in [(0, "start"), (1, "end")] {
                                let t = method_arg_tys.get(i).cloned().unwrap_or(Ty::Unknown);
                                if !self.unify_or_hole(&t, &Ty::Int) {
                                    return Err(anyhow!(
                                        "Type error at {}:{}: .str_slice {} expects Int, found {}",
                                        expr.span().line,
                                        expr.span().col,
                                        expect_name,
                                        t.display()
                                    ));
                                }
                            }
                            if let (Some(s), Some(start), Some(end)) = (
                                args.first().and_then(|a| Ty::const_str(a)),
                                args.get(1).and_then(|a| Ty::const_int(a)),
                                args.get(2).and_then(|a| Ty::const_int(a)),
                            ) {
                                let len = s.chars().count() as i64;
                                if start < 0 || end < start || end > len {
                                    return Err(anyhow!(
                                        "Type error at {}:{}: str_slice[{}..{}] out of bounds for string literal of length {} (const bounds check)",
                                        expr.span().line,
                                        expr.span().col,
                                        start,
                                        end,
                                        len
                                    ));
                                }
                            }
                            Ok(Ty::String)
                        }
                        ".sys_exec" => Ok(Ty::Int),
                        ".trim" | ".to_lowercase" | ".to_string" => Ok(Ty::String),
                        ".contains" | ".is_ok" | ".is_err" | ".is_some" | ".is_none" => Ok(Ty::Bool),
                        ".get" | ".read_file" | ".env_get" => Ok(Ty::Result(
                            Box::new(Ty::String),
                            Box::new(Ty::String),
                        )),
                        ".write_file" => Ok(Ty::Result(
                            Box::new(Ty::Void),
                            Box::new(Ty::String),
                        )),
                        ".push" => {
                            // recv.push(elem)
                            match &recv_ty {
                                Ty::List(inner) => {
                                    let elem_ty =
                                        method_arg_tys.first().cloned().unwrap_or(Ty::Unknown);
                                    let out = if matches!(inner.as_ref(), Ty::Unknown) {
                                        elem_ty
                                    } else if Ty::unifyable_or_unknown_hole(inner, &elem_ty) {
                                        (**inner).clone()
                                    } else {
                                        return Err(anyhow!(
                                            "Type error at {}:{}: list element type mismatch: List[{}] cannot push {}",
                                            expr.span().line,
                                            expr.span().col,
                                            inner.display(),
                                            elem_ty.display()
                                        ));
                                    };
                                    Ok(Ty::List(Box::new(out)))
                                }
                                other => Err(anyhow!(
                                    "Type error at {}:{}: .push requires List receiver, found {}",
                                    expr.span().line,
                                    expr.span().col,
                                    other.display()
                                )),
                            }
                        }
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
                    }?;
                    return apply_try(method_ty);
                }

                let mut arg_tys = Vec::new();
                for a in args {
                    arg_tys.push(self.infer_expr(a, env)?);
                }

                // List surface: track element types (no soft List[Unknown] forever).
                if name == "list_new" {
                    if !arg_tys.is_empty() {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'list_new' expects 0 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    return Ok(Ty::List(Box::new(Ty::Unknown)));
                }
                if name == "list_push" {
                    if arg_tys.len() != 2 {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'list_push' expects 2 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    let elem = match &arg_tys[0] {
                        Ty::List(inner) => (**inner).clone(),
                        other => {
                            return Err(anyhow!(
                                "Type error at {}:{}: function 'list_push' argument 0 expects List, found {}",
                                expr.span().line,
                                expr.span().col,
                                other.display()
                            ));
                        }
                    };
                    let pushed = &arg_tys[1];
                    let out_elem = if matches!(elem, Ty::Unknown) {
                        pushed.clone()
                    } else if Ty::unifyable_or_unknown_hole(&elem, pushed) {
                        // Prefer concrete list element over Unknown push (shouldn't happen often).
                        if matches!(pushed, Ty::Unknown) {
                            elem
                        } else if Ty::unifyable(&elem, pushed) {
                            elem
                        } else {
                            // hole on one side only — keep non-Unknown
                            elem
                        }
                    } else {
                        return Err(anyhow!(
                            "Type error at {}:{}: list element type mismatch: List[{}] cannot push {}",
                            expr.span().line,
                            expr.span().col,
                            elem.display(),
                            pushed.display()
                        ));
                    };
                    return Ok(Ty::List(Box::new(out_elem)));
                }
                if name == "list_get" {
                    if arg_tys.len() != 2 {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'list_get' expects 2 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    if !Ty::unifyable_or_unknown_hole(&arg_tys[1], &Ty::Int) {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'list_get' argument 1 expects Int, found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys[1].display()
                        ));
                    }
                    // Const index bounds: negative always fail; known list lengths fail OOB.
                    if let Some(idx) = Ty::const_int(&args[1]) {
                        if idx < 0 {
                            return Err(anyhow!(
                                "Type error at {}:{}: list_get index {} is negative (const bounds check)",
                                expr.span().line,
                                expr.span().col,
                                idx
                            ));
                        }
                        let lens = self.active_list_lens.borrow();
                        if let Some(len) = Self::const_list_len(&args[0], &lens) {
                            if idx >= len {
                                return Err(anyhow!(
                                    "Type error at {}:{}: list_get index {} out of bounds for list of length {} (const bounds check)",
                                    expr.span().line,
                                    expr.span().col,
                                    idx,
                                    len
                                ));
                            }
                        }
                    }
                    return match &arg_tys[0] {
                        Ty::List(inner) => Ok((**inner).clone()),
                        other => Err(anyhow!(
                            "Type error at {}:{}: function 'list_get' argument 0 expects List, found {}",
                            expr.span().line,
                            expr.span().col,
                            other.display()
                        )),
                    };
                }
                if name == "list_len" {
                    if arg_tys.len() != 1 {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'list_len' expects 1 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    return match &arg_tys[0] {
                        Ty::List(_) => Ok(Ty::Int),
                        other => Err(anyhow!(
                            "Type error at {}:{}: function 'list_len' argument 0 expects List, found {}",
                            expr.span().line,
                            expr.span().col,
                            other.display()
                        )),
                    };
                }

                // Const string indexing — fail-closed OOB (was typecheck-green, runtime trap).
                if name == "char_at" {
                    if arg_tys.len() != 2 {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'char_at' expects 2 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    if !Ty::unifyable_or_unknown_hole(&arg_tys[0], &Ty::String) {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'char_at' argument 0 expects String, found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys[0].display()
                        ));
                    }
                    if !Ty::unifyable_or_unknown_hole(&arg_tys[1], &Ty::Int) {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'char_at' argument 1 expects Int, found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys[1].display()
                        ));
                    }
                    if let (Some(s), Some(idx)) =
                        (Ty::const_str(&args[0]), Ty::const_int(&args[1]))
                    {
                        let len = s.chars().count() as i64;
                        if idx < 0 || idx >= len {
                            return Err(anyhow!(
                                "Type error at {}:{}: char_at index {} out of bounds for string literal of length {} (const bounds check)",
                                expr.span().line,
                                expr.span().col,
                                idx,
                                len
                            ));
                        }
                    }
                    return Ok(Ty::String);
                }
                if name == "str_slice" {
                    if arg_tys.len() != 3 {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'str_slice' expects 3 argument(s), found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys.len()
                        ));
                    }
                    if !Ty::unifyable_or_unknown_hole(&arg_tys[0], &Ty::String) {
                        return Err(anyhow!(
                            "Type error at {}:{}: function 'str_slice' argument 0 expects String, found {}",
                            expr.span().line,
                            expr.span().col,
                            arg_tys[0].display()
                        ));
                    }
                    for (i, expect) in [(1, "start"), (2, "end")] {
                        if !Ty::unifyable_or_unknown_hole(&arg_tys[i], &Ty::Int) {
                            return Err(anyhow!(
                                "Type error at {}:{}: function 'str_slice' argument {} ({}) expects Int, found {}",
                                expr.span().line,
                                expr.span().col,
                                i,
                                expect,
                                arg_tys[i].display()
                            ));
                        }
                    }
                    if let (Some(s), Some(start), Some(end)) = (
                        Ty::const_str(&args[0]),
                        Ty::const_int(&args[1]),
                        Ty::const_int(&args[2]),
                    ) {
                        let len = s.chars().count() as i64;
                        if start < 0 || end < 0 || start > end || end > len {
                            return Err(anyhow!(
                                "Type error at {}:{}: str_slice[{}..{}] out of bounds for string literal of length {} (const bounds check)",
                                expr.span().line,
                                expr.span().col,
                                start,
                                end,
                                len
                            ));
                        }
                    }
                    return Ok(Ty::String);
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
                    if !self.unify(a, b)
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

                // sys_exec/exec: varargs (optional cap handle + cmd + argv strings).
                if name == "sys_exec" || name == "exec" || name == "spawn_process" {
                    if arg_tys.is_empty() {
                        return Err(anyhow!(
                            "Type error at {}:{}: function '{}' expects at least 1 argument(s), found 0",
                            expr.span().line,
                            expr.span().col,
                            name
                        ));
                    }
                    return Ok(Ty::Result(
                        Box::new(Ty::String),
                        Box::new(Ty::String),
                    ));
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
                        if !self.unify_or_hole(pt, at) {
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
                    // Const call-site refinement: Int[lo..hi] params reject out-of-bounds literals.
                    if let Some(bounds) = self.param_refinements.get(name) {
                        for (i, bound) in bounds.iter().enumerate() {
                            if let Some((lo, hi)) = bound {
                                if let Some(arg_expr) = args.get(i) {
                                    if let Some(val) = Ty::const_int(arg_expr) {
                                        if val < *lo || val > *hi {
                                            let sp = arg_expr.span();
                                            return Err(anyhow!(
                                                "Type error at {}:{}: RefinementTypeViolation: argument {} value {} out of refinement bounds [{}..{}] for parameter of function '{}'",
                                                sp.line,
                                                sp.col,
                                                i,
                                                val,
                                                lo,
                                                hi,
                                                name
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    return apply_try(ret.clone());
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
                // Expression-level if: inherit env. Mut map is empty here (infer_expr
                // has no parent mut); statement-level if in check_block carries real mut.
                // Match/value-if that assign outer `let mut` use eval shadow-restore;
                // typecheck of those assigns is best-effort via env-only (see check_block Match).
                let mut env_then = env.clone();
                let mut mut_then = mutable.clone();
                // Expression-level if inherits mutability from parent (match arms, value-if).
                let empty_ref = HashMap::new();
                let t1 = self.check_block(
                    then_branch,
                    &mut env_then,
                    &mut mut_then,
                    "if-then",
                    None,
                    &empty_ref,
                    None,
                )?;
                if let Some(else_b) = else_branch {
                    let mut env_else = env.clone();
                    let mut mut_else = mutable.clone();
                    let t2 = self.check_block(
                        else_b,
                        &mut env_else,
                        &mut mut_else,
                        "if-else",
                        None,
                        &empty_ref,
                        None,
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
                    // Fail-closed: value-producing if without else has no type on false path
                    // (was runtime () / Void while typecheck claimed Int).
                    if !matches!(t1, Ty::Void | Ty::Unknown) {
                        return Err(anyhow!(
                            "Type error at {}:{}: if expression producing {} requires an else branch",
                            expr.span().line,
                            expr.span().col,
                            t1.display()
                        ));
                    }
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
                self.check_block(
                    body,
                    &mut env.clone(),
                    &mut m,
                    "while-expr",
                    None,
                    &empty_ref,
                    None,
                )?;
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
                let mut has_true = false;
                let mut has_false = false;
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
                        Pattern::Literal(Literal::Bool(true)) => has_true = true,
                        Pattern::Literal(Literal::Bool(false)) => has_false = true,
                        Pattern::Literal(_) => {}
                    }
                    let t = self.infer_expr_m(&arm.body, &arm_env, mutable)?;
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

                // DESIGN: exhaustive matching for Result/Option/Bool (no silent fall-through).
                if !has_wildcard {
                    match &scrutinee_ty {
                        Ty::Result(_, _) if !(has_ok && has_err) => {
                            return Err(anyhow!(
                                "Type error at {}:{}: non-exhaustive match on Result — cover both Ok(_) and Err(_), or use `_`",
                                span.line,
                                span.col
                            ));
                        }
                        Ty::Bool if !(has_true && has_false) => {
                            return Err(anyhow!(
                                "Type error at {}:{}: non-exhaustive match on Bool — cover both true and false, or use `_`",
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

/// True when every control-flow path through `block` executes `return`.
/// Expression-bodied blocks (tail value) are NOT "returns" — they yield a value.
/// An early unconditional `return` makes the rest of the block dead (still returns).
/// Conservative: unknown constructs → false.
fn block_always_returns(block: &Block) -> bool {
    for stmt in &block.stmts {
        match stmt {
            Statement::Return(_, _) => return true,
            Statement::Expr(e, _) if expr_paths_return(e) => return true,
            // Other statements may fall through.
            _ => {}
        }
    }
    if let Some(expr) = &block.expr {
        return expr_paths_return(expr);
    }
    false
}

fn expr_paths_return(expr: &Expression) -> bool {
    match expr {
        Expression::If {
            then_branch,
            else_branch,
            ..
        } => {
            block_always_returns(then_branch)
                && else_branch
                    .as_ref()
                    .map(|b| block_always_returns(b))
                    .unwrap_or(false)
        }
        Expression::Match { arms, .. } => {
            !arms.is_empty() && arms.iter().all(|arm| expr_paths_return(&arm.body))
        }
        // Bare values / calls are not return statements.
        _ => false,
    }
}

/// Span of a statement for unreachable-code diagnostics.
fn stmt_span(stmt: &Statement) -> crate::ast::Span {
    match stmt {
        Statement::Let { span, .. }
        | Statement::Assign { span, .. }
        | Statement::FieldAssign { span, .. }
        | Statement::Return(_, span)
        | Statement::Expr(_, span)
        | Statement::While { span, .. } => *span,
    }
}


/// Names of functions that use `?` (try) — not lowered outside the interpreter yet.
pub fn program_uses_try_operator(program: &Program) -> bool {
    fn expr_has_try(e: &Expression) -> bool {
        match e {
            Expression::Call { propagate_err, args, .. } => {
                *propagate_err || args.iter().any(expr_has_try)
            }
            Expression::Binary { left, right, .. } => expr_has_try(left) || expr_has_try(right),
            Expression::Unary { expr, .. } => expr_has_try(expr),
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                expr_has_try(cond)
                    || block_has_try(then_branch)
                    || else_branch.as_ref().map(|b| block_has_try(b)).unwrap_or(false)
            }
            Expression::While { cond, body, .. } => expr_has_try(cond) || block_has_try(body),
            Expression::Match { expr, arms, .. } => {
                expr_has_try(expr) || arms.iter().any(|a| expr_has_try(&a.body))
            }
            Expression::StructLit { fields, .. } => fields.iter().any(|(_, e)| expr_has_try(e)),
            Expression::Literal(_, _) | Expression::Variable(_, _) => false,
        }
    }
    fn block_has_try(b: &Block) -> bool {
        b.stmts.iter().any(|s| match s {
            Statement::Let { init, .. } => expr_has_try(init),
            Statement::Assign { value, .. } => expr_has_try(value),
            Statement::FieldAssign { object, value, .. } => {
                expr_has_try(object) || expr_has_try(value)
            }
            Statement::Return(Some(e), _) | Statement::Expr(e, _) => expr_has_try(e),
            Statement::Return(None, _) => false,
            Statement::While { cond, body, .. } => expr_has_try(cond) || block_has_try(body),
        }) || b.expr.as_ref().map(|e| expr_has_try(e)).unwrap_or(false)
    }
    for item in &program.items {
        if let Item::Function(f) = item {
            if block_has_try(&f.body) {
                return true;
            }
            if let Some(v) = &f.verify_block {
                if block_has_try(v) {
                    return true;
                }
            }
        }
    }
    false
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

    #[test]
    fn ok_constructor_carries_payload_type_into_match() {
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
        assert!(
            check(src).is_ok(),
            "Ok(1) payload Int must type match arm: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn write_file_wrong_arity_fails_closed() {
        let src = r#"
            pub fn bad(fs: &FsCap) {
                let r = write_file(fs, "/tmp/x");
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("expects 3 argument") || err.contains("found 2"),
            "write_file object-cap arity: {}",
            err
        );
    }

    #[test]
    fn path_exists_object_cap_arity_ok() {
        let src = r#"
            pub fn main(fs: &FsCap) {
                let b = path_exists(fs, "/tmp");
                println(b);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "path_exists(cap, path): {:?}",
            check(src).err()
        );
    }

    #[test]
    fn sys_exec_varargs_typechecks() {
        let src = r#"
            pub fn main(sys: &SysCap) {
                let v = sys_exec(sys, "true");
                let w = sys_exec(sys, "echo", "hi");
                println(1);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "sys_exec varargs: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn method_write_file_wrong_arity_fails() {
        let src = r#"
            pub fn bad(fs: &FsCap) {
                let r = fs.write_file("/tmp/x");
                match r { Ok(_) => 0, Err(_) => 1 };
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains(".write_file") && (err.contains("expects 3") || err.contains("found 2")),
            "method write_file arity: {}",
            err
        );
    }

    #[test]
    fn method_write_file_full_arity_ok() {
        let src = r#"
            pub fn ok(fs: &FsCap) {
                let r = fs.write_file("app.log", "hi");
                match r { Ok(_) => 0, Err(_) => 1 };
            }
        "#;
        assert!(
            check(src).is_ok(),
            "method write_file(path, content): {:?}",
            check(src).err()
        );
    }

    #[test]
    fn rejects_missing_return_on_non_void_fn() {
        let src = r#"
            pub fn f() -> Int {
            }
            pub fn main() {
                println(f());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("missing return") || err.contains("Void"),
            "empty body -> Int must fail, got: {}",
            err
        );
    }

    #[test]
    fn rejects_statement_body_without_return_for_int_fn() {
        let src = r#"
            pub fn f(x: Int) -> Int {
                let y = x + 1;
            }
            pub fn main() {
                println(f(1));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("missing return") || err.contains("Void"),
            "no return in Int fn must fail, got: {}",
            err
        );
    }

    #[test]
    fn rejects_int_eq_float() {
        let src = r#"
            pub fn main() {
                let b = 1 == 1.0;
                println(b);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("cannot compare") || err.contains("equality"),
            "Int == Float must fail, got: {}",
            err
        );
    }

    #[test]
    fn rejects_if_expr_without_else_non_void() {
        let src = r#"
            pub fn main() {
                let x = if true { 1 };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("else") || err.contains("if expression"),
            "if-as-value without else must fail, got: {}",
            err
        );
    }

    #[test]
    fn accepts_if_stmt_without_else() {
        let src = r#"
            pub fn main() {
                if true {
                    println(1);
                }
            }
        "#;
        assert!(
            check(src).is_ok(),
            "statement if without else is fine: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn accepts_if_expr_with_else() {
        let src = r#"
            pub fn main() {
                let x = if true { 1 } else { 0 };
                println(x);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "if/else value: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn accepts_if_else_both_return() {
        let src = r#"
            pub fn f(x: Int) -> Int {
                if x > 0 {
                    return x;
                } else {
                    return 0;
                }
            }
            pub fn main() {
                println(f(1));
            }
        "#;
        assert!(
            check(src).is_ok(),
            "if/else both return must typecheck: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn rejects_partial_if_return_fallthrough() {
        let src = r#"
            pub fn f(x: Int) -> Int {
                if x > 0 {
                    return x;
                }
            }
            pub fn main() {
                println(f(0));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("missing return") || err.contains("Void"),
            "partial if return must fail: {}",
            err
        );
    }

    #[test]
    fn rejects_bind_void_from_while() {
        let src = r#"
            pub fn main() {
                let x = while false {
                    println(1);
                };
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("Void") || err.contains("bind"),
            "let x = while must fail: {}",
            err
        );
    }

    #[test]
    fn rejects_const_int_division_by_zero() {
        let src = r#"
            pub fn main() {
                let x = 1 / 0;
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("division by zero"),
            "const /0 must fail closed: {}",
            err
        );
    }

    #[test]
    fn rejects_const_float_division_by_zero() {
        let src = r#"
            pub fn main() {
                let x = 1.0 / 0.0;
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("division by zero"),
            "const float /0 must fail closed: {}",
            err
        );
    }

    #[test]
    fn accepts_early_return_with_dead_code_after() {
        // Early return means the function always returns; trailing stmts are
        // unreachable (separate diagnostic) — first ensure always-returns works.
        let src = r#"
            pub fn f() -> Int {
                return 1;
            }
            pub fn main() {
                println(f());
            }
        "#;
        assert!(
            check(src).is_ok(),
            "plain early return: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn rejects_unreachable_after_return() {
        let src = r#"
            pub fn f() -> Int {
                return 1;
                let y = 2;
            }
            pub fn main() {
                println(f());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("unreachable"),
            "dead code after return must fail: {}",
            err
        );
    }

    #[test]
    fn rejects_unreachable_after_if_else_return() {
        let src = r#"
            pub fn f(x: Int) -> Int {
                if x > 0 {
                    return x;
                } else {
                    return 0;
                }
                let z = 1;
            }
            pub fn main() {
                println(f(1));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("unreachable"),
            "dead code after if/else return: {}",
            err
        );
    }

    #[test]
    fn rejects_list_push_element_type_mismatch() {
        let src = r#"
            pub fn main() {
                let xs = list_new();
                let ys = list_push(xs, 1);
                let zs = list_push(ys, "a");
                println(list_len(zs));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("list element type mismatch")
                || (err.contains("List[Int]") && err.contains("String")),
            "heterogeneous list push must fail: {}",
            err
        );
    }

    #[test]
    fn accepts_homogeneous_list_push_and_get() {
        let src = r#"
            pub fn main() {
                let xs = list_new();
                let ys = list_push(xs, 1);
                let zs = list_push(ys, 2);
                let n = list_get(zs, 0);
                println(n);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "homogeneous Int list: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn list_get_element_type_flows_to_use() {
        // After push Int, list_get yields Int — cannot use as String return.
        let src = r#"
            pub fn bad() -> String {
                let xs = list_new();
                let ys = list_push(xs, 1);
                return list_get(ys, 0);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("return type") || err.contains("String") || err.contains("Int"),
            "list_get Int must not soft-accept as String: {}",
            err
        );
    }

    #[test]
    fn rejects_const_char_at_out_of_bounds() {
        let src = r#"
            pub fn main() {
                let c = char_at("hi", 99);
                println(c);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("out of bounds") && err.contains("char_at"),
            "const char_at OOB must fail at typecheck: {}",
            err
        );
    }

    #[test]
    fn accepts_const_char_at_in_bounds() {
        let src = r#"
            pub fn main() {
                let c = char_at("hi", 0);
                println(c);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "in-bounds char_at: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn rejects_const_str_slice_out_of_bounds() {
        let src = r#"
            pub fn main() {
                let s = str_slice("hi", 0, 9);
                println(s);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("out of bounds") && err.contains("str_slice"),
            "const str_slice OOB: {}",
            err
        );
    }

    #[test]
    fn nested_let_does_not_pollute_outer_type_env() {
        // Was: `let x = "hi"` inside if retyped outer Int x → String (scope leak).
        let src = r#"
            pub fn main() {
                let x = 1;
                if true {
                    let x = "hi";
                    println(x);
                }
                let y = x + 1;
                println(y);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "outer x must stay Int after nested shadow: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn nested_while_let_does_not_pollute_outer_type_env() {
        let src = r#"
            pub fn main() {
                let x = 1;
                let mut i = 0;
                while i < 1 {
                    let x = "hi";
                    println(x);
                    i = i + 1;
                }
                let y = x + 1;
                println(y);
            }
        "#;
        assert!(
            check(src).is_ok(),
            "while-body let must not leak: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn rejects_const_arg_out_of_param_refinement_bounds() {
        let src = r#"
            pub fn port(p: Int[1..65535]) -> Int {
                return p;
            }
            pub fn main() {
                println(port(0));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("0"),
            "const arg must enforce param Int[lo..hi]: {}",
            err
        );
    }

    #[test]
    fn accepts_const_arg_in_param_refinement_bounds() {
        let src = r#"
            pub fn port(p: Int[1..65535]) -> Int {
                return p;
            }
            pub fn main() {
                println(port(8080));
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn outer_let_mut_assign_inside_if_still_typechecks() {
        let src = r#"
            pub fn main() {
                let mut x = 1;
                if true {
                    x = 2;
                }
                println(x);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn else_if_sibling_let_does_not_leak_across_branches() {
        // else if desugars to nested tail-if; sibling lets must not pollute.
        let src = r#"
            pub fn main() {
                if false {
                } else if false {
                    let x = 1;
                } else {
                    println(x);
                }
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("undefined variable") && err.contains("'x'"),
            "else-branch must not see else-if let x: {}",
            err
        );
    }

    #[test]
    fn type_alias_int_unifies_for_arith_and_return() {
        let src = r#"
            type Port = Int;
            pub fn bump(p: Port) -> Int {
                return p + 1;
            }
            pub fn main() {
                println(bump(3));
            }
        "#;
        assert!(
            check(src).is_ok(),
            "Port=Int must unify for + and return: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn type_alias_refinement_param_const_oob_fails() {
        let src = r#"
            type Port = Int[1..65535];
            pub fn take(p: Port) -> Int {
                return 1;
            }
            pub fn main() {
                println(take(0));
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation"),
            "alias Int[lo..hi] param must enforce const OOB: {}",
            err
        );
    }

    #[test]
    fn method_char_at_on_string_literal_in_bounds() {
        let src = r#"
            pub fn main() {
                let c = "hi".char_at(0);
                println(c);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn method_char_at_const_oob_fails() {
        let src = r#"
            pub fn main() {
                let c = "hi".char_at(99);
                println(c);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("out of bounds") && err.contains("char_at"),
            "{}",
            err
        );
    }

    #[test]
    fn match_if_outer_mut_assign_typechecks() {
        let src = r#"
            pub fn main() -> Int {
                let mut x = 0;
                let r = Ok(5);
                match r {
                    Ok(v) => if true {
                        x = v;
                        v
                    } else {
                        0
                    },
                    Err(e) => 0,
                };
                return x;
            }
        "#;
        assert!(
            check(src).is_ok(),
            "match+if assign to outer let mut: {:?}",
            check(src).err()
        );
    }

    #[test]
    fn alias_let_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn main() {
                let p: Port = 99;
                println(p);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "alias let ann must enforce bounds: {}",
            err
        );
    }

    #[test]
    fn alias_return_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f() -> Port {
                return 99;
            }
            pub fn main() {
                println(f());
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "alias return must enforce bounds: {}",
            err
        );
    }

    #[test]
    fn alias_let_refinement_in_bounds_ok() {
        let src = r#"
            type Port = Int[1..10];
            pub fn main() {
                let p: Port = 5;
                println(p);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn nested_return_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f(b: Bool) -> Port {
                if b {
                    return 99;
                }
                return 1;
            }
            pub fn main() { println(f(true)); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "nested if return must enforce bounds: {}",
            err
        );
    }

    #[test]
    fn while_return_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f() -> Port {
                let mut i = 0;
                while i < 1 {
                    return 99;
                }
                return 1;
            }
            pub fn main() { println(f()); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "while return must enforce bounds: {}",
            err
        );
    }

    #[test]
    fn tail_expr_refinement_const_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f() -> Port { 99 }
            pub fn main() { println(f()); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "tail expr return must enforce bounds: {}",
            err
        );
    }

    #[test]
    fn match_arm_const_return_refinement_oob_fails() {
        let src = r#"
            type Port = Int[1..10];
            pub fn f() -> Port {
                match Ok(1) {
                    Ok(v) => 99,
                    Err(e) => 1,
                }
            }
            pub fn main() { println(f()); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("RefinementTypeViolation") && err.contains("99"),
            "match arm const return: {}",
            err
        );
    }

    #[test]
    fn list_get_const_oob_fails() {
        let src = r#"
            pub fn main() {
                let xs = list_new();
                let ys = list_push(xs, 1);
                let z = list_get(ys, 5);
                println(z);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("out of bounds") || err.contains("list_get"),
            "list_get const OOB: {}",
            err
        );
    }

    #[test]
    fn list_get_negative_const_fails() {
        let src = r#"
            pub fn main() {
                let xs = list_new();
                let ys = list_push(xs, 1);
                let z = list_get(ys, 0 - 1);
                println(z);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(err.contains("negative") || err.contains("out of bounds"), "{}", err);
    }

    #[test]
    fn field_assign_typechecks_and_rejects_immutable() {
        let ok = r#"
            type Pt = struct { x: Int, y: Int };
            pub fn main() {
                let mut p = Pt { x: 1, y: 2 };
                p.x = 3;
                println(p.x);
            }
        "#;
        assert!(check(ok).is_ok(), "{:?}", check(ok).err());
        let bad = r#"
            type Pt = struct { x: Int, y: Int };
            pub fn main() {
                let p = Pt { x: 1, y: 2 };
                p.x = 3;
            }
        "#;
        let err = check(bad).unwrap_err().to_string();
        assert!(err.contains("immutable") || err.contains("let mut"), "{}", err);
    }

    #[test]
    fn question_mark_unwraps_result() {
        let src = r#"
            pub fn f() -> Result[Int, String] { return Ok(1); }
            pub fn g() -> Result[Int, String] {
                let x = f()?;
                return Ok(x);
            }
            pub fn main() {
                match g() {
                    Ok(v) => println(v),
                    Err(e) => println(e),
                }
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn question_mark_on_non_result_fails() {
        let src = r#"
            pub fn main() {
                let x = 1?;
                println(x);
            }
        "#;
        // may fail parse or type
        let r = check(src);
        assert!(r.is_err(), "1? must fail");
    }

    #[test]
    fn bool_match_true_false_exhaustive() {
        let src = r#"
            pub fn f(b: Bool) -> Int {
                match b {
                    true => 1,
                    false => 0,
                }
            }
            pub fn main() { println(f(true)); }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn bool_match_nonexhaustive_fails() {
        let src = r#"
            pub fn f(b: Bool) -> Int {
                match b {
                    true => 1,
                }
            }
            pub fn main() { println(f(true)); }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(err.contains("non-exhaustive") || err.contains("Bool"), "{}", err);
    }

    #[test]
    fn contains_method_typechecks() {
        let src = r#"
            pub fn main() {
                let ok = "hello".contains("ell");
                println(ok);
            }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src).err());
    }

    #[test]
    fn question_mark_in_void_fn_fails() {
        let src = r#"
            pub fn f() -> Result[Int, String] { return Ok(1); }
            pub fn main() {
                let x = f()?;
                println(x);
            }
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("`?` only allowed") || err.contains("Result"),
            "void main cannot use ?: {}",
            err
        );
    }

    #[test]
    fn question_mark_err_type_must_match() {
        let src = r#"
            pub fn f() -> Result[Int, String] { return Err("e"); }
            pub fn g() -> Result[Int, Int] {
                let x = f()?;
                return Ok(x);
            }
            pub fn main() {}
        "#;
        let err = check(src).unwrap_err().to_string();
        assert!(
            err.contains("error type") || err.contains("`?`"),
            "Err types must match: {}",
            err
        );
    }
}
