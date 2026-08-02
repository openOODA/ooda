// ===================================================================
// CHS → C backend (native stage-1 path without clang).
// Emits ISO C99 + runtime/chs_rt.c, linked with gcc.
// ===================================================================
use crate::ast::*;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct CCodeGen;

/// Host FFI builtins that require linking `libooda.a` (stage-0 staticlib).
/// Pure CHS programs never call these — they must not force Cargo/staticlib.
const HOST_FFI_CALLS: &[&str] = &[
    "chs_build",
    "host_ast_dump",
    "host_check",
    "host_token_dump",
];

/// True if any call in the program needs stage-0 host FFI (`libooda.a`).
pub fn program_needs_host_ffi(program: &Program) -> bool {
    for item in &program.items {
        if let Item::Function(f) = item {
            if block_needs_host_ffi(&f.body) {
                return true;
            }
        }
    }
    false
}

fn is_host_ffi_name(name: &str) -> bool {
    HOST_FFI_CALLS.iter().any(|n| *n == name)
}

fn block_needs_host_ffi(b: &Block) -> bool {
    b.stmts.iter().any(stmt_needs_host_ffi)
        || b.expr.as_deref().map_or(false, expr_needs_host_ffi)
}

fn stmt_needs_host_ffi(s: &Statement) -> bool {
    match s {
        Statement::Let { init, .. } => expr_needs_host_ffi(init),
        Statement::Assign { value, .. } => expr_needs_host_ffi(value),
        Statement::FieldAssign { object, value, .. } => {
            expr_needs_host_ffi(object) || expr_needs_host_ffi(value)
        }
        Statement::Return(Some(e), _) => expr_needs_host_ffi(e),
        Statement::Return(None, _) | Statement::Break(_) | Statement::Continue(_) => false,
        Statement::Expr(e, _) => expr_needs_host_ffi(e),
        Statement::While { cond, body, .. } => {
            expr_needs_host_ffi(cond) || block_needs_host_ffi(body)
        }
    }
}

fn expr_needs_host_ffi(e: &Expression) -> bool {
    match e {
        Expression::Call { name, args, .. } => {
            if is_host_ffi_name(name) {
                return true;
            }
            args.iter().any(expr_needs_host_ffi)
        }
        Expression::Binary { left, right, .. } => {
            expr_needs_host_ffi(left) || expr_needs_host_ffi(right)
        }
        Expression::Unary { expr, .. } => expr_needs_host_ffi(expr),
        Expression::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expr_needs_host_ffi(cond)
                || block_needs_host_ffi(then_branch)
                || else_branch
                    .as_ref()
                    .map_or(false, |b| block_needs_host_ffi(b))
        }
        Expression::While { cond, body, .. } => {
            expr_needs_host_ffi(cond) || block_needs_host_ffi(body)
        }
        Expression::Match { expr, arms, .. } => {
            expr_needs_host_ffi(expr) || arms.iter().any(|a| expr_needs_host_ffi(&a.body))
        }
        Expression::StructLit { fields, .. } => fields.iter().any(|(_, e)| expr_needs_host_ffi(e)),
        Expression::Literal(_, _) | Expression::Variable(_, _) => false,
    }
}

impl CCodeGen {
    pub fn emit_c(program: &Program) -> Result<String> {
        Self::assert_chs_c_subset(program)?;
        let mut g = Gen::new();
        g.with_host_ffi = program_needs_host_ffi(program);
        g.emit_program(program)?;
        Ok(g.finish())
    }

    fn assert_chs_c_subset(program: &Program) -> Result<()> {
        let aliases = program.collect_type_aliases();
        for item in &program.items {
            if let Item::Function(f) = item {
                for p in &f.params {
                    Self::check_ty(&p.param_type.resolve_alias(&aliases), &f.name)?;
                }
                Self::check_ty(&f.return_type.resolve_alias(&aliases), &f.name)?;
            }
        }
        Ok(())
    }

    fn check_ty(t: &Type, ctx: &str) -> Result<()> {
        match t {
            Type::Int | Type::Bool | Type::Void | Type::String | Type::Float => Ok(()),
            Type::FsCap | Type::EnvCap | Type::SysCap | Type::NetCap => Ok(()),
            Type::List(inner) => match **inner {
                Type::Int | Type::String => Ok(()),
                _ => bail!("C backend List only supports List[Int]|List[String] in '{}'", ctx),
            },
            Type::Struct { .. } => Ok(()),
            Type::Option(_) | Type::Result(_, _) => Ok(()),
            Type::Custom(s) => match s.as_str() {
                "Int" | "Bool" | "String" | "Void" | "Float" => Ok(()),
                _ => Ok(()), // named struct aliases
            },
        }
    }

    /// Compile .oo → native binary via gcc + chs_rt.c. Returns path to binary.
    ///
    /// **Assembly depth:** pure CHS programs (no `chs_build` / host dumps) link
    /// with **only** gcc + `chs_rt.c` — no `libooda.a` / Cargo staticlib.
    /// Host-FFI programs still require stage-0 `libooda.a` (fail closed if missing).
    pub fn build_native(program: &Program, out_bin: &Path, rt_c: &Path, release: bool) -> Result<()> {
        let need_host = program_needs_host_ffi(program);
        let c_src = Self::emit_c(program)?;
        let out_c = out_bin.with_extension("c");
        std::fs::write(&out_c, &c_src)?;
        let gcc = which_gcc()?;
        // Prefer HOME cache for compiler temp files — /tmp may be quota-limited.
        let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| {
            let p = dirs_tmp();
            let _ = std::fs::create_dir_all(&p);
            p
        });
        let mut cmd = Command::new(&gcc);
        let opt_flag = if release { "-O3" } else { "-O0" };
        cmd.env("TMPDIR", &tmp)
            .env("TMP", &tmp)
            .env("TEMP", &tmp)
            .arg(opt_flag);

        if release {
            cmd.arg("-flto");
        }

        cmd.arg("-std=c99");
        if need_host {
            // Enable host FFI wrappers in chs_rt.c; link stage-0 staticlib.
            cmd.arg("-DOODA_WITH_HOST_FFI");
            let lib_dir = find_ooda_staticlib_dir().ok_or_else(|| {
                anyhow::anyhow!(
                    "CHS C backend: program uses host FFI (chs_build/host_* dumps) but \
                     libooda.a not found under target/{{release,debug}}. \
                     Run `cargo build --release` or use pure CHS without host builtins."
                )
            })?;
            cmd.arg(&out_c).arg(rt_c);
            cmd.arg(format!("-L{}", lib_dir.display()));
            cmd.arg("-looda");
            cmd.arg("-lpthread");
            cmd.arg("-ldl");
            cmd.arg("-lm");
            // Rust staticlib may need libgcc_s / libc
            cmd.arg("-Wl,--allow-multiple-definition");
        } else {
            // Pure CHS: gcc + runtime only — no Cargo/staticlib (B0 assembly depth).
            cmd.arg(&out_c).arg(rt_c);
        }
        cmd.arg("-o").arg(out_bin);
        let out = cmd
            .output()
            .map_err(|e| anyhow::anyhow!("failed to spawn {}: {}", gcc, e))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            bail!("gcc failed linking CHS C backend:\n{}", err.chars().take(1200).collect::<String>());
        }
        Ok(())
    }
}

/// Locate `libooda.a` from cargo target dir (release preferred).
fn find_ooda_staticlib_dir() -> Option<std::path::PathBuf> {
    let mut candidates = vec![
        std::path::PathBuf::from("target/release"),
        std::path::PathBuf::from("target/debug"),
    ];
    // Crate-relative targets (works regardless of host home path)
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest.join("target/release"));
    candidates.push(manifest.join("target/debug"));
    for c in candidates {
        if c.join("libooda.a").exists() {
            return Some(c);
        }
    }
    None
}

fn c_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn which_gcc() -> Result<String> {
    for t in ["gcc", "cc"] {
        if Command::new(t).arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
            return Ok(t.into());
        }
    }
    bail!("no gcc/cc in PATH for CHS C backend native link")
}

fn dirs_tmp() -> String {
    if let Ok(h) = std::env::var("HOME") {
        format!("{}/.cache/ooda-tmp", h)
    } else {
        "/var/tmp".into()
    }
}

struct Gen {
    structs: HashMap<String, Vec<(String, Type)>>,
    type_aliases: HashMap<String, Type>,
    /// name → C return type string
    fn_ret: HashMap<String, String>,
    functions: Vec<String>,
    prelude: String,
    body: String,
    tmp: usize,
    /// When true, bare `return;` becomes `return 0;` (C main).
    c_main: bool,
    /// Current OODA function returns void (bare return;).
    fn_void: bool,
    /// Emit host FFI decls (only when program calls chs_build/host_*).
    with_host_ffi: bool,
}

impl Gen {
    fn new() -> Self {
        Self {
            structs: HashMap::new(),
            type_aliases: HashMap::new(),
            fn_ret: HashMap::new(),
            functions: Vec::new(),
            prelude: String::new(),
            body: String::new(),
            tmp: 0,
            c_main: false,
            fn_void: false,
            with_host_ffi: false,
        }
    }

    fn fresh(&mut self, p: &str) -> String {
        self.tmp += 1;
        format!("{}_{}", p, self.tmp)
    }

    /// Build C lvalue for `object.field` where object is Variable or nested `.f` Calls.
    fn c_field_lvalue(object: &Expression, field: &str) -> Result<String> {
        let mut segs: Vec<String> = Vec::new();
        let mut cur = object;
        loop {
            match cur {
                Expression::Variable(name, _) => {
                    segs.insert(0, name.clone());
                    break;
                }
                Expression::Call { name, args, .. }
                    if name.starts_with('.') && args.len() == 1 =>
                {
                    segs.insert(0, name[1..].to_string());
                    cur = &args[0];
                }
                _ => bail!("C backend: field assign requires variable or field-path receiver"),
            }
        }
        segs.push(field.to_string());
        Ok(segs.join("."))
    }

    fn finish(self) -> String {
        let mut s = String::new();
        s.push_str("/* generated by openOODA CHS C backend — do not edit */\n");
        s.push_str("#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <stdbool.h>\n#include <unistd.h>\n");
        s.push_str("/* runtime decls (defined in chs_rt.c) */\n");
        s.push_str("typedef struct { char *data; long long len; } OoStr;\n");
        s.push_str("typedef struct { long long *data; long long len; long long cap; } OoIList;\n");
        s.push_str("typedef struct { OoStr *data; long long len; long long cap; } OoSList;\n");
        s.push_str("typedef struct { int ok; OoStr val; } OoResS;\n");
        s.push_str("typedef struct { int ok; OoStr err; } OoResV;\n");
        s.push_str("OoStr oo_str_lit(const char*); OoStr oo_str_concat(OoStr,OoStr);\n");
        s.push_str("long long oo_str_byte_len(OoStr); long long oo_chars_len(OoStr);\n");
        s.push_str("OoStr oo_char_at(OoStr,long long); OoStr oo_str_slice(OoStr,long long,long long);\n");
        s.push_str("int oo_str_contains(OoStr,OoStr);\n");
        s.push_str("int oo_char_is_digit(OoStr); int oo_char_is_alpha(OoStr); int oo_char_is_space(OoStr);\n");
        s.push_str("OoIList oo_ilist_new(void); OoIList oo_ilist_push(OoIList,long long);\n");
        s.push_str("long long oo_ilist_get(OoIList,long long); long long oo_ilist_len(OoIList);\n");
        s.push_str("OoSList oo_slist_new(void); OoSList oo_slist_push(OoSList,OoStr);\n");
        s.push_str("OoStr oo_slist_get(OoSList,long long); long long oo_slist_len(OoSList);\n");
        s.push_str("OoResS oo_read_file(OoStr); OoResV oo_write_file(OoStr,OoStr); int oo_path_exists(OoStr); long long oo_file_size(OoStr); OoResS oo_env_get(OoStr);\n");
        s.push_str("void oo_print_str(OoStr); void oo_print_int(long long); void oo_print_bool(int); void oo_println(void);\n");
        s.push_str("int oo_str_eq(OoStr,OoStr); int oo_str_contains(OoStr,OoStr);\n");
        s.push_str("OoStr oo_int_to_str(long long); OoStr oo_str_trim(OoStr); OoStr oo_str_to_lowercase(OoStr);\n");
        // Host FFI only when the program calls it — pure CHS never needs libooda.a.
        if self.with_host_ffi {
            s.push_str("/* Host FFI (libooda.a) — stage-0 dumps + chs_build */\n");
            s.push_str("char *ooda_host_ast_dump(const char *path);\n");
            s.push_str("char *ooda_host_check(const char *path);\n");
            s.push_str("char *ooda_host_token_dump(const char *path);\n");
            s.push_str("int ooda_host_chs_build(const char *src, const char *out_bin);\n");
            s.push_str("void ooda_host_free(char *p);\n");
            s.push_str("OoStr oo_host_ast_dump(OoStr path);\n");
            s.push_str("OoStr oo_host_check(OoStr path);\n");
            s.push_str("OoStr oo_host_token_dump(OoStr path);\n");
            s.push_str("OoResS oo_chs_build(OoStr src, OoStr out_bin);\n");
        }
        s.push('\n');
        s.push_str(&self.prelude);
        for f in &self.functions {
            s.push_str(f);
            s.push('\n');
        }
        s.push_str(&self.body);
        s
    }

    fn c_ty(&self, t: &Type) -> String {
        let resolved = t.resolve_alias(&self.type_aliases);
        match &resolved {
            Type::Int | Type::Float => "long long".into(),
            Type::Bool => "int".into(),
            Type::Void => "void".into(),
            Type::String => "OoStr".into(),
            Type::FsCap | Type::EnvCap | Type::SysCap | Type::NetCap => "int".into(),
            Type::List(inner) => match **inner {
                Type::Int => "OoIList".into(),
                Type::String => "OoSList".into(),
                _ => "OoIList".into(),
            },
            Type::Result(_, _) => "OoResS".into(),
            Type::Option(_) => "OoResS".into(),
            Type::Struct { name: Some(n), .. } => format!("struct {}", n),
            Type::Struct { name: None, .. } => "/*anon*/ int".into(),
            Type::Custom(s) => {
                if self.structs.contains_key(s) {
                    format!("struct {}", s)
                } else {
                    match s.as_str() {
                        "Int" | "Float" => "long long".into(),
                        "Bool" => "int".into(),
                        "String" => "OoStr".into(),
                        "Void" => "void".into(),
                        _ => format!("struct {}", s),
                    }
                }
            }
        }
    }

    fn emit_program(&mut self, program: &Program) -> Result<()> {
        self.type_aliases = program.collect_type_aliases();
        for item in &program.items {
            if let Item::TypeAlias(name, Type::Struct { fields, .. }) = item {
                self.structs.insert(name.clone(), fields.clone());
                let mut decl = format!("struct {} {{\n", name);
                for (fnm, fty) in fields {
                    decl.push_str(&format!("  {} {};\n", self.c_ty(fty), fnm));
                }
                decl.push_str("};\n");
                self.prelude.push_str(&decl);
            }
        }
        // forward decls
        for item in &program.items {
            if let Item::Function(f) = item {
                let ret = self.c_ty(&f.return_type);
                self.fn_ret.insert(f.name.clone(), ret.clone());
                let mut params = Vec::new();
                for p in &f.params {
                    // Keep cap params as int tokens so call-sites stay arity-correct.
                    params.push(format!("{} {}", self.c_ty(&p.param_type), p.name));
                }
                if f.name == "main" {
                    // real C main below
                    continue;
                }
                self.prelude.push_str(&format!(
                    "{} oo_{}({});\n",
                    ret,
                    f.name,
                    if params.is_empty() {
                        "void".into()
                    } else {
                        params.join(", ")
                    }
                ));
            }
        }
        for item in &program.items {
            if let Item::Function(f) = item {
                if f.name == "main" {
                    self.emit_main(f)?;
                } else {
                    self.emit_function(f)?;
                }
            }
        }
        Ok(())
    }

    fn emit_function(&mut self, f: &FunctionDecl) -> Result<()> {
        self.c_main = false;
        self.fn_void = matches!(f.return_type, Type::Void);
        let ret = self.c_ty(&f.return_type);
        let mut params = Vec::new();
        let mut env = HashMap::new();
        for p in &f.params {
            params.push(format!("{} {}", self.c_ty(&p.param_type), p.name));
            let env_ty = match &p.param_type {
                Type::FsCap | Type::EnvCap | Type::SysCap | Type::NetCap => "/*cap*/".into(),
                other => self.c_ty(other),
            };
            env.insert(p.name.clone(), env_ty);
        }
        let mut code = format!(
            "{} oo_{}({}) {{\n",
            ret,
            f.name,
            if params.is_empty() {
                "void".into()
            } else {
                params.join(", ")
            }
        );
        code.push_str(&self.emit_block(&f.body, &mut env, &f.return_type, true)?);
        if matches!(f.return_type, Type::Void) {
            code.push_str("  return;\n");
        }
        code.push_str("}\n");
        self.functions.push(code);
        Ok(())
    }

    fn emit_main(&mut self, f: &FunctionDecl) -> Result<()> {
        self.c_main = true;
        self.fn_void = false;
        let mut env = HashMap::new();
        let mut code = String::from("int main(int argc, char **argv) {\n");
        // inject caps as dummy ints
        for p in &f.params {
            match &p.param_type {
                Type::FsCap | Type::EnvCap | Type::SysCap | Type::NetCap => {
                    // Compile-only placeholder. Runtime object-cap is interpreter-only;
                    // dual-engine refuses sealed I/O before this path for sealed programs.
                    code.push_str(&format!(
                        "  int {} = 1; /* cap token erased on C (no runtime gate) */\n",
                        p.name
                    ));
                    env.insert(p.name.clone(), "/*cap*/".into());
                }
                Type::List(inner) if matches!(**inner, Type::String) || p.name == "args" || p.name == "argv" => {
                    code.push_str("  OoSList args = oo_slist_new();\n");
                    code.push_str("  for (int i = 1; i < argc; i++) {\n");
                    code.push_str("    args = oo_slist_push(args, oo_str_lit(argv[i]));\n");
                    code.push_str("  }\n");
                    // also bind param name if not args
                    if p.name != "args" {
                        code.push_str(&format!("  OoSList {} = args;\n", p.name));
                    }
                    env.insert(p.name.clone(), "OoSList".into());
                }
                other => {
                    code.push_str(&format!(
                        "  {} {} = {{0}}; /* default main param */\n",
                        self.c_ty(other),
                        p.name
                    ));
                    env.insert(p.name.clone(), self.c_ty(other));
                }
            }
        }
        // Use Int return type so `return;` in OODA main becomes `return 0;` in C.
        code.push_str(&self.emit_block(&f.body, &mut env, &Type::Int, true)?);
        code.push_str("  return 0;\n}\n");
        self.body.push_str(&code);
        Ok(())
    }

    fn emit_block(
        &mut self,
        block: &Block,
        env: &mut HashMap<String, String>,
        ret_ty: &Type,
        tail_is_fn_return: bool,
    ) -> Result<String> {
        let mut code = String::new();
        for stmt in &block.stmts {
            code.push_str(&self.emit_stmt(stmt, env, ret_ty)?);
        }
        if let Some(e) = &block.expr {
            let (c, v, ty) = self.emit_expr(e, env)?;
            code.push_str(&c);
            // Only function bodies should turn a trailing expression into `return`.
            // while/if bodies often end with a trailing `if` expression.
            if tail_is_fn_return && !matches!(ret_ty, Type::Void) {
                code.push_str(&format!("  return {};\n", v));
            } else {
                let _ = (ty, v);
            }
        }
        Ok(code)
    }

    fn emit_stmt(
        &mut self,
        stmt: &Statement,
        env: &mut HashMap<String, String>,
        ret_ty: &Type,
    ) -> Result<String> {
        match stmt {
            Statement::Let {
                name,
                type_annotation,
                init,
                ..
            } => {
                // Prefer annotation for empty list_new() so List[String] vs List[Int] is correct.
                let ann_ty = type_annotation.as_ref().map(|t| self.c_ty(t));
                // Unannotated bare list_new: defer C type until first list_push (E-M: no
                // dual-representation union; zero drag until first element).
                if matches!(
                    init,
                    Expression::Call { name: n, args, .. } if n == "list_new" && args.is_empty()
                ) && ann_ty.is_none()
                {
                    env.insert(name.clone(), "OoListPending".into());
                    return Ok(format!(
                        "  /* pending list {} — kind fixed on first push */\n",
                        name
                    ));
                }
                let (mut c, mut v, ty) = self.emit_expr(init, env)?;
                let cty = ann_ty.clone().unwrap_or(ty.clone());
                // Relower bare list_new to matching empty list type when annotated.
                if matches!(init, Expression::Call { name: n, args, .. } if n == "list_new" && args.is_empty())
                {
                    if cty == "OoSList" {
                        let t = self.fresh("sl");
                        c = format!("  OoSList {} = oo_slist_new();\n", t);
                        v = t;
                    } else if cty == "OoIList" {
                        let t = self.fresh("il");
                        c = format!("  OoIList {} = oo_ilist_new();\n", t);
                        v = t;
                    }
                }
                env.insert(name.clone(), cty.clone());
                Ok(format!("{}  {} {} = {};\n", c, cty, name, v))
            }
            Statement::FieldAssign { object, field, value, .. } => {
                // CHS structs are C structs: p.x = v or nested p.inner.n = v
                let lval = Self::c_field_lvalue(object, field)?;
                let (vcode, vtmp, vty) = self.emit_expr(value, env)?;
                let mut code = vcode;
                code.push_str(&format!("  {} = {};\n", lval, vtmp));
                let _ = vty;
                Ok(code)
            }
            Statement::Assign { name, value, .. } => {
                let (c, v, ty) = self.emit_expr(value, env)?;
                // First write into a pending list: declare with concrete OoIList/OoSList.
                if env.get(name).map(|s| s.as_str()) == Some("OoListPending") {
                    env.insert(name.clone(), ty.clone());
                    return Ok(format!("{}  {} {} = {};\n", c, ty, name, v));
                }
                // Refine env if push produced a more specific list kind.
                if (ty == "OoSList" || ty == "OoIList")
                    && env.get(name).map(|s| s.as_str()) != Some(ty.as_str())
                {
                    env.insert(name.clone(), ty.clone());
                }
                Ok(format!("{}  {} = {};\n", c, name, v))
            }
            Statement::Return(Some(e), _) => {
                let (c, v, _) = self.emit_expr(e, env)?;
                match ret_ty {
                    Type::Void => Ok(format!("{}  return;\n", c)),
                    Type::Custom(s) if s == "_ret" => {
                        // Nested in if/while: emit real return value (function returns non-void).
                        Ok(format!("{}  return {};\n", c, v))
                    }
                    _ => Ok(format!("{}  return {};\n", c, v)),
                }
            }
            Statement::Return(None, _) => {
                if self.c_main {
                    Ok("  return 0;\n".into())
                } else {
                    Ok("  return;\n".into())
                }
            }
            Statement::Expr(e, _) => {
                // println and side-effecting calls
                if let Expression::Call { name, args, .. } = e {
                    if name == "println" {
                        return self.emit_println(args, env);
                    }
                }
                let (c, _v, _) = self.emit_expr(e, env)?;
                Ok(c)
            }
            Statement::While { cond, body, .. } => {
                let mut code = String::from("  while (1) {\n");
                let (cc2, cv2, _) = self.emit_expr(cond, env)?;
                code.push_str(&cc2);
                code.push_str(&format!("    if (!({})) break;\n", cv2));
                code.push_str(&self.emit_block(body, env, ret_ty, false)?);
                code.push_str("  }\n");
                Ok(code)
            }
            Statement::Break(_) => Ok("  break;\n".into()),
            Statement::Continue(_) => Ok("  continue;\n".into()),
        }
    }

    fn emit_println(&mut self, args: &[Expression], env: &mut HashMap<String, String>) -> Result<String> {
        let mut code = String::new();
        for a in args {
            let (c, v, ty) = self.emit_expr(a, env)?;
            code.push_str(&c);
            if ty == "OoStr" {
                code.push_str(&format!("  oo_print_str({});\n", v));
            } else if ty == "int" {
                code.push_str(&format!("  oo_print_bool({});\n", v));
            } else {
                code.push_str(&format!("  oo_print_int({});\n", v));
            }
        }
        code.push_str("  oo_println();\n");
        Ok(code)
    }

    fn emit_expr(
        &mut self,
        expr: &Expression,
        env: &mut HashMap<String, String>,
    ) -> Result<(String, String, String)> {
        match expr {
            Expression::Literal(Literal::Int(n), _) => {
                Ok((String::new(), format!("{}LL", n), "long long".into()))
            }
            Expression::Literal(Literal::Bool(b), _) => {
                Ok((String::new(), if *b { "1" } else { "0" }.into(), "int".into()))
            }
            Expression::Literal(Literal::String(s), _) => {
                let lit = c_escape_string(s);
                let t = self.fresh("s");
                Ok((
                    format!("  OoStr {} = oo_str_lit(\"{}\");\n", t, lit),
                    t,
                    "OoStr".into(),
                ))
            }
            Expression::Literal(Literal::Float(f), _) => {
                Ok((String::new(), format!("{}LL", *f as i64), "long long".into()))
            }
            Expression::Literal(Literal::Void, _) => {
                Ok((String::new(), "0".into(), "int".into()))
            }
            Expression::Variable(name, _) => {
                let ty = env.get(name).cloned().unwrap_or_else(|| "long long".into());
                Ok((String::new(), name.clone(), ty))
            }
            Expression::Binary { op, left, right, .. } => {
                let (lc, lv, lty) = self.emit_expr(left, env)?;
                let (rc, rv, rty) = self.emit_expr(right, env)?;
                let mut code = lc;
                code.push_str(&rc);
                if matches!(op, BinOp::Add) && (lty == "OoStr" || rty == "OoStr") {
                    let t = self.fresh("cat");
                    code.push_str(&format!(
                        "  OoStr {} = oo_str_concat({}, {});\n",
                        t, lv, rv
                    ));
                    return Ok((code, t, "OoStr".into()));
                }
                if matches!(op, BinOp::Eq | BinOp::Neq) && lty == "OoStr" {
                    let t = self.fresh("eq");
                    if matches!(op, BinOp::Eq) {
                        code.push_str(&format!("  int {} = oo_str_eq({}, {});\n", t, lv, rv));
                    } else {
                        code.push_str(&format!("  int {} = !oo_str_eq({}, {});\n", t, lv, rv));
                    }
                    return Ok((code, t, "int".into()));
                }
                let cop = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Eq => "==",
                    BinOp::Neq => "!=",
                    BinOp::Lt => "<",
                    BinOp::Lte => "<=",
                    BinOp::Gt => ">",
                    BinOp::Gte => ">=",
                    BinOp::And => "&&",
                    BinOp::Or => "||",
                    _ => bail!("C backend: unsupported binop {:?}", op),
                };
                let t = self.fresh("b");
                let rty = if matches!(
                    op,
                    BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte | BinOp::And | BinOp::Or
                ) {
                    "int"
                } else {
                    "long long"
                };
                code.push_str(&format!(
                    "  {} {} = ({}) {} ({});\n",
                    rty, t, lv, cop, rv
                ));
                Ok((code, t, rty.into()))
            }
            Expression::Unary { op, expr, .. } => {
                let (c, v, _) = self.emit_expr(expr, env)?;
                let t = self.fresh("u");
                let mut code = c;
                match op {
                    UnaryOp::Not => {
                        code.push_str(&format!("  int {} = !({});\n", t, v));
                        Ok((code, t, "int".into()))
                    }
                    UnaryOp::Neg => {
                        code.push_str(&format!("  long long {} = -({});\n", t, v));
                        Ok((code, t, "long long".into()))
                    }
                }
            }
            Expression::Call { name, args, .. } => self.emit_call(name, args, env),
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                // Statement-style if (no value): emit control flow only.
                // Value-style if is limited to int results from tails.
                let (cc, cv, _) = self.emit_expr(cond, env)?;
                let mut code = cc;
                // Prefer statement-if when branches have returns/stmts without shared value.
                let t = self.fresh("ifv");
                code.push_str(&format!("  long long {} = 0;\n", t));
                code.push_str(&format!("  if ({}) {{\n", cv));
                // Use Unknown ret so Return(Some) still emits the value; Void would force `return 0`.
                for s in &then_branch.stmts {
                    code.push_str(&self.emit_stmt(s, env, &Type::Custom("_ret".into()))?);
                }
                if let Some(e) = &then_branch.expr {
                    let (tc, tv, tty) = self.emit_expr(e, env)?;
                    code.push_str(&tc);
                    if tty == "long long" || tty == "int" {
                        code.push_str(&format!("    {} = {};\n", t, tv));
                    }
                }
                code.push_str("  }");
                if let Some(eb) = else_branch {
                    code.push_str(" else {\n");
                    for s in &eb.stmts {
                        code.push_str(&self.emit_stmt(s, env, &Type::Custom("_ret".into()))?);
                    }
                    if let Some(e) = &eb.expr {
                        let (ec, ev, ety) = self.emit_expr(e, env)?;
                        code.push_str(&ec);
                        if ety == "long long" || ety == "int" {
                            code.push_str(&format!("    {} = {};\n", t, ev));
                        }
                    }
                    code.push_str("  }\n");
                } else {
                    code.push_str("\n");
                }
                Ok((code, t, "long long".into()))
            }
            Expression::While { .. } => {
                bail!("C backend: while as expression not supported; use statement while")
            }
            Expression::Match { expr, arms, .. } => {
                // Lower Result match: Ok/Err only, int/string payload loosely
                let (ec, ev, ety) = self.emit_expr(expr, env)?;
                let t = self.fresh("mv");
                let mut code = ec;
                if ety == "OoResS" {
                    code.push_str(&format!("  OoStr {} = oo_str_lit(\"\");\n", t));
                    code.push_str(&format!("  if (({}).ok) {{\n", ev));
                    for arm in arms {
                        if let Pattern::Variant { name, arg } = &arm.pattern {
                            if name == "Ok" {
                                if let Some(bind) = arg {
                                    code.push_str(&format!(
                                        "    OoStr {} = ({}).val;\n",
                                        bind, ev
                                    ));
                                    env.insert(bind.clone(), "OoStr".into());
                                }
                                let (bc, bv, bty) = self.emit_expr(&arm.body, env)?;
                                code.push_str(&bc);
                                if bty == "OoStr" {
                                    code.push_str(&format!("    {} = {};\n", t, bv));
                                }
                            }
                        }
                    }
                    code.push_str("  } else {\n");
                    for arm in arms {
                        if let Pattern::Variant { name, arg } = &arm.pattern {
                            if name == "Err" {
                                if let Some(bind) = arg {
                                    code.push_str(&format!(
                                        "    OoStr {} = ({}).val;\n",
                                        bind, ev
                                    ));
                                    env.insert(bind.clone(), "OoStr".into());
                                }
                                let (bc, bv, bty) = self.emit_expr(&arm.body, env)?;
                                code.push_str(&bc);
                                if bty == "OoStr" {
                                    code.push_str(&format!("    {} = {};\n", t, bv));
                                }
                            }
                        }
                    }
                    code.push_str("  }\n");
                    Ok((code, t, "OoStr".into()))
                } else {
                    // int match on scrutinee
                    code.push_str(&format!("  long long {} = 0;\n", t));
                    for arm in arms {
                        match &arm.pattern {
                            Pattern::Literal(Literal::Int(n)) => {
                                code.push_str(&format!("  if (({}) == {}LL) {{\n", ev, n));
                                let (bc, bv, _) = self.emit_expr(&arm.body, env)?;
                                code.push_str(&bc);
                                code.push_str(&format!("    {} = {};\n", t, bv));
                                code.push_str("  } else ");
                            }
                            Pattern::Wildcard => {
                                code.push_str("  {\n");
                                let (bc, bv, _) = self.emit_expr(&arm.body, env)?;
                                code.push_str(&bc);
                                code.push_str(&format!("    {} = {};\n", t, bv));
                                code.push_str("  }\n");
                            }
                            _ => {}
                        }
                    }
                    Ok((code, t, "long long".into()))
                }
            }
            Expression::StructLit { name, fields, .. } => {
                let t = self.fresh("st");
                let mut code = format!("  struct {} {} ;\n", name, t);
                for (fnm, fex) in fields {
                    let (fc, fv, _) = self.emit_expr(fex, env)?;
                    code.push_str(&fc);
                    code.push_str(&format!("  {} .{} = {};\n", t, fnm, fv));
                }
                Ok((code, t, format!("struct {}", name)))
            }
        }
    }

    fn emit_call(
        &mut self,
        name: &str,
        args: &[Expression],
        env: &mut HashMap<String, String>,
    ) -> Result<(String, String, String)> {
        // Field access .foo with one arg
        if let Some(field) = name.strip_prefix('.') {
            if args.len() == 1 {
                if field == "to_string" {
                    let (c, v, ty) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("ts");
                    let mut code = c;
                    if ty == "OoStr" {
                        code.push_str(&format!("  OoStr {} = {};\n", t, v));
                    } else if ty == "int" {
                        // bool
                        code.push_str(&format!(
                            "  OoStr {} = oo_str_lit(({}) ? \"true\" : \"false\");\n",
                            t, v
                        ));
                    } else {
                        // int → decimal via snprintf
                        let buf = self.fresh("buf");
                        code.push_str(&format!("  char {}[32];\n", buf));
                        code.push_str(&format!(
                            "  snprintf({}, sizeof({}), \"%lld\", (long long)({}));\n",
                            buf, buf, v
                        ));
                        code.push_str(&format!("  OoStr {} = oo_str_lit({});\n", t, buf));
                    }
                    return Ok((code, t, "OoStr".into()));
                }
                if field == "len" {
                    let (c, v, ty) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("ln");
                    let mut code = c;
                    if ty == "OoStr" {
                        code.push_str(&format!(
                            "  long long {} = oo_str_byte_len({});\n",
                            t, v
                        ));
                    } else if ty == "OoIList" {
                        code.push_str(&format!(
                            "  long long {} = oo_ilist_len({});\n",
                            t, v
                        ));
                    } else if ty == "OoSList" {
                        code.push_str(&format!(
                            "  long long {} = oo_slist_len({});\n",
                            t, v
                        ));
                    } else {
                        bail!(".len on unsupported type {}", ty);
                    }
                    return Ok((code, t, "long long".into()));
                }
                // Option/Result both use OoResS.ok (not a distinct is_some field).
                if field == "is_ok" || field == "is_err" || field == "is_some" || field == "is_none"
                {
                    let (c, v, _) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("io");
                    let mut code = c;
                    if field == "is_ok" || field == "is_some" {
                        code.push_str(&format!("  int {} = ({}).ok;\n", t, v));
                    } else {
                        code.push_str(&format!("  int {} = !({}).ok;\n", t, v));
                    }
                    return Ok((code, t, "int".into()));
                }
                if field == "trim" {
                    let (c, v, _) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("tr");
                    let mut code = c;
                    code.push_str(&format!("  OoStr {} = oo_str_trim({});\n", t, v));
                    return Ok((code, t, "OoStr".into()));
                }
                if field == "to_lowercase" {
                    let (c, v, _) = self.emit_expr(&args[0], env)?;
                    let t = self.fresh("lc");
                    let mut code = c;
                    code.push_str(&format!("  OoStr {} = oo_str_to_lowercase({});\n", t, v));
                    return Ok((code, t, "OoStr".into()));
                }
                // struct field
                let (c, v, ty) = self.emit_expr(&args[0], env)?;
                let t = self.fresh("fld");
                let mut code = c;
                // Guess field type from known structs
                let fty = if let Some(sname) = ty.strip_prefix("struct ") {
                    self.structs
                        .get(sname)
                        .and_then(|fs| fs.iter().find(|(n, _)| n == field))
                        .map(|(_, t)| self.c_ty(t))
                        .unwrap_or_else(|| "long long".into())
                } else {
                    "long long".into()
                };
                code.push_str(&format!("  {} {} = {}.{};\n", fty, t, v, field));
                return Ok((code, t, fty));
            }
        }

        let mut code = String::new();
        let mut cargs = Vec::new();
        let mut arg_tys = Vec::new();
        let method_name_early = name.strip_prefix('.').unwrap_or(name);
        let skip_cap_args = matches!(
            method_name_early,
            "read_file"
                | "write_file"
                | "fs_read"
                | "fs_write"
                | "env_get"
                | "path_exists"
                | "fs_exists"
                | "file_size"
                | "sys_exec"
        );
        for a in args {
            let (c, v, ty) = self.emit_expr(a, env)?;
            code.push_str(&c);
            // Skip erased capability tokens by type (not by parameter name).
            if skip_cap_args && (ty == "/*cap*/" || ty == "int") {
                if let Expression::Variable(n, _) = a {
                    if env.get(n).map(|t| t.as_str()) == Some("/*cap*/") {
                        continue;
                    }
                }
            }
            cargs.push(v);
            arg_tys.push(ty);
        }

        let t = self.fresh("r");
        let method_name = name.strip_prefix('.').unwrap_or(name);
        match method_name {
            "list_new" => {
                // Bare expression form (not through pending let): default int list.
                code.push_str(&format!("  OoIList {} = oo_ilist_new();\n", t));
                Ok((code, t, "OoIList".into()))
            }
            "push" | "list_push" => {
                let list = &cargs[0];
                let item = &cargs[1];
                let lty = arg_tys.first().map(|s| s.as_str()).unwrap_or("");
                let item_ty = arg_tys.get(1).map(|s| s.as_str()).unwrap_or("");
                // Kind from list type, or first element when list is still pending.
                let as_str = lty == "OoSList"
                    || item_ty == "OoStr"
                    || (lty == "OoListPending" && item_ty == "OoStr");
                if as_str {
                    if lty == "OoListPending" {
                        let empty = self.fresh("sl0");
                        code.push_str(&format!("  OoSList {} = oo_slist_new();\n", empty));
                        code.push_str(&format!(
                            "  OoSList {} = oo_slist_push({}, {});\n",
                            t, empty, item
                        ));
                    } else {
                        code.push_str(&format!(
                            "  OoSList {} = oo_slist_push({}, {});\n",
                            t, list, item
                        ));
                    }
                    Ok((code, t, "OoSList".into()))
                } else if lty == "OoListPending" {
                    let empty = self.fresh("il0");
                    code.push_str(&format!("  OoIList {} = oo_ilist_new();\n", empty));
                    code.push_str(&format!(
                        "  OoIList {} = oo_ilist_push({}, {});\n",
                        t, empty, item
                    ));
                    Ok((code, t, "OoIList".into()))
                } else {
                    code.push_str(&format!(
                        "  OoIList {} = oo_ilist_push({}, {});\n",
                        t, list, item
                    ));
                    Ok((code, t, "OoIList".into()))
                }
            }
            "list_get" => {
                let lty = arg_tys.first().map(|s| s.as_str()).unwrap_or("");
                if lty == "OoSList" {
                    code.push_str(&format!(
                        "  OoStr {} = oo_slist_get({}, {});\n",
                        t, cargs[0], cargs[1]
                    ));
                    Ok((code, t, "OoStr".into()))
                } else if lty == "OoListPending" {
                    // Empty pending — should not be read; emit typed zero for compile.
                    code.push_str(&format!("  long long {} = 0; /* empty pending list_get */\n", t));
                    Ok((code, t, "long long".into()))
                } else {
                    code.push_str(&format!(
                        "  long long {} = oo_ilist_get({}, {});\n",
                        t, cargs[0], cargs[1]
                    ));
                    Ok((code, t, "long long".into()))
                }
            }
            "list_len" => {
                let lty = arg_tys.first().map(|s| s.as_str()).unwrap_or("");
                if lty == "OoSList" {
                    code.push_str(&format!(
                        "  long long {} = oo_slist_len({});\n",
                        t, cargs[0]
                    ));
                } else if lty == "OoListPending" {
                    code.push_str(&format!("  long long {} = 0; /* empty pending list */\n", t));
                } else {
                    code.push_str(&format!(
                        "  long long {} = oo_ilist_len({});\n",
                        t, cargs[0]
                    ));
                }
                Ok((code, t, "long long".into()))
            }
            "chars_len" => {
                code.push_str(&format!(
                    "  long long {} = oo_chars_len({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "long long".into()))
            }
            "char_at" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_char_at({}, {});\n",
                    t, cargs[0], cargs[1]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "str_slice" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_str_slice({}, {}, {});\n",
                    t, cargs[0], cargs[1], cargs[2]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "contains" | "str_contains" => {
                code.push_str(&format!(
                    "  int {} = (strstr({}.data ? {}.data : \"\", {}.data ? {}.data : \"\") != NULL);\n",
                    t, cargs[0], cargs[0], cargs[1], cargs[1]
                ));
                Ok((code, t, "int".into()))
            }
            "char_is_digit" => {
                code.push_str(&format!(
                    "  int {} = oo_char_is_digit({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "int".into()))
            }
            "char_is_alpha" => {
                code.push_str(&format!(
                    "  int {} = oo_char_is_alpha({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "int".into()))
            }
            "char_is_space" => {
                code.push_str(&format!(
                    "  int {} = oo_char_is_space({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "int".into()))
            }
            "read_file" | "fs_read" | ".read_file" => {
                let path = cargs.last().unwrap();
                code.push_str(&format!("  OoResS {} = oo_read_file({});\n", t, path));
                Ok((code, t, "OoResS".into()))
            }
            "write_file" | "fs_write" | ".write_file" => {
                // path, content — last two stringish
                let path = if cargs.len() >= 2 {
                    &cargs[cargs.len() - 2]
                } else {
                    &cargs[0]
                };
                let content = cargs.last().unwrap();
                code.push_str(&format!(
                    "  OoResV {} = oo_write_file({}, {});\n",
                    t, path, content
                ));
                // map to OoResS-like for is_ok: use ok field
                Ok((code, t, "OoResV".into()))
            }
            "path_exists" | "fs_exists" => {
                let path = cargs.last().unwrap();
                code.push_str(&format!("  int {} = oo_path_exists({});\n", t, path));
                Ok((code, t, "int".into()))
            }
            "file_size" => {
                let path = cargs.last().unwrap();
                code.push_str(&format!("  long long {} = oo_file_size({});\n", t, path));
                Ok((code, t, "long long".into()))
            }
            // Option and Result both lower to OoResS { int ok; OoStr val; }.
            "is_some" | "is_ok" => {
                if cargs.is_empty() {
                    bail!("C backend: .{} needs a receiver", method_name);
                }
                code.push_str(&format!("  int {} = ({}.ok);\n", t, cargs[0]));
                Ok((code, t, "int".into()))
            }
            "is_none" | "is_err" => {
                if cargs.is_empty() {
                    bail!("C backend: .{} needs a receiver", method_name);
                }
                code.push_str(&format!("  int {} = !({}.ok);\n", t, cargs[0]));
                Ok((code, t, "int".into()))
            }
            "env_get" => {
                // Dead for sealed programs: dual-engine refuse in main.rs. Kept for
                // host/smoke paths that emit C without the sealed gate.
                if cargs.is_empty() {
                    bail!("C backend: env_get needs a key argument");
                }
                let key = cargs.last().unwrap();
                code.push_str(&format!("  OoResS {} = oo_env_get({});\n", t, key));
                Ok((code, t, "OoResS".into()))
            }
            "to_string" => {
                code.push_str(&format!("  OoStr {} = oo_int_to_str({});\n", t, cargs[0]));
                Ok((code, t, "OoStr".into()))
            }
            "trim" => {
                code.push_str(&format!("  OoStr {} = oo_str_trim({});\n", t, cargs[0]));
                Ok((code, t, "OoStr".into()))
            }
            "to_lowercase" => {
                code.push_str(&format!("  OoStr {} = oo_str_to_lowercase({});\n", t, cargs[0]));
                Ok((code, t, "OoStr".into()))
            }
            "host_ast_dump" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_host_ast_dump({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "host_check" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_host_check({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "host_token_dump" => {
                code.push_str(&format!(
                    "  OoStr {} = oo_host_token_dump({});\n",
                    t, cargs[0]
                ));
                Ok((code, t, "OoStr".into()))
            }
            "chs_build" => {
                code.push_str(&format!(
                    "  OoResS {} = oo_chs_build({}, {});\n",
                    t, cargs[0], cargs[1]
                ));
                Ok((code, t, "OoResS".into()))
            }
            "process_exit" => {
                code.push_str(&format!("  exit((int)({}));\n", cargs[0]));
                Ok((code, "0".into(), "int".into()))
            }
            "sys_exec" | "system_exec" => {
                let cmd = cargs.last().unwrap();
                code.push_str(&format!(
                    "  int {} = system({}.data ? {}.data : \"\");\n",
                    t, cmd, cmd
                ));
                Ok((code, t, "int".into()))
            }
            "Ok" => {
                // Result ok — payload is String or generic; use OoResS
                let v = cargs.get(0).cloned().unwrap_or_else(|| "oo_str_lit(\"\")".into());
                let ty = arg_tys.get(0).map(|s| s.as_str()).unwrap_or("OoStr");
                if ty == "OoStr" {
                    code.push_str(&format!(
                        "  OoResS {} = {{ .ok = 1, .val = {} }};\n",
                        t, v
                    ));
                } else {
                    // box int as string for simplicity
                    let buf = self.fresh("okb");
                    code.push_str(&format!("  char {}[32];\n", buf));
                    code.push_str(&format!(
                        "  snprintf({}, sizeof({}), \"%lld\", (long long)({}));\n",
                        buf, buf, v
                    ));
                    code.push_str(&format!(
                        "  OoResS {} = {{ .ok = 1, .val = oo_str_lit({}) }};\n",
                        t, buf
                    ));
                }
                Ok((code, t, "OoResS".into()))
            }
            "Err" => {
                let v = cargs.get(0).cloned().unwrap_or_else(|| "oo_str_lit(\"err\")".into());
                let ty = arg_tys.get(0).map(|s| s.as_str()).unwrap_or("OoStr");
                if ty == "OoStr" {
                    code.push_str(&format!(
                        "  OoResS {} = {{ .ok = 0, .val = {} }};\n",
                        t, v
                    ));
                } else {
                    let buf = self.fresh("erb");
                    code.push_str(&format!("  char {}[64];\n", buf));
                    code.push_str(&format!(
                        "  snprintf({}, sizeof({}), \"%lld\", (long long)({}));\n",
                        buf, buf, v
                    ));
                    code.push_str(&format!(
                        "  OoResS {} = {{ .ok = 0, .val = oo_str_lit({}) }};\n",
                        t, buf
                    ));
                }
                Ok((code, t, "OoResS".into()))
            }
            "println" => {
                // handled at stmt level usually
                for (i, a) in cargs.iter().enumerate() {
                    let ty = &arg_tys[i];
                    if ty == "OoStr" {
                        code.push_str(&format!("  oo_print_str({});\n", a));
                    } else if ty == "int" {
                        code.push_str(&format!("  oo_print_bool({});\n", a));
                    } else {
                        code.push_str(&format!("  oo_print_int({});\n", a));
                    }
                }
                code.push_str("  oo_println();\n");
                Ok((code, "0".into(), "int".into()))
            }
            ".contains" => {
                if cargs.len() != 2 {
                    bail!("C backend: .contains expects receiver + needle");
                }
                code.push_str(&format!(
                    "  int {} = oo_str_contains({}, {});\n",
                    t, cargs[0], cargs[1]
                ));
                Ok((code, t, "int".into()))
            }
            // Method-style string ops: same runtime as free functions (dual-engine parity).
            ".char_at" => {
                if cargs.len() != 2 {
                    bail!("C backend: .char_at expects receiver + index");
                }
                code.push_str(&format!(
                    "  OoStr {} = oo_char_at({}, {});\n",
                    t, cargs[0], cargs[1]
                ));
                Ok((code, t, "OoStr".into()))
            }
            ".str_slice" => {
                if cargs.len() != 3 {
                    bail!("C backend: .str_slice expects receiver + start + end");
                }
                code.push_str(&format!(
                    "  OoStr {} = oo_str_slice({}, {}, {});\n",
                    t, cargs[0], cargs[1], cargs[2]
                ));
                Ok((code, t, "OoStr".into()))
            }
            other if other.starts_with('.') => {
                bail!("C backend: unsupported method {}", other)
            }
            other => {
                let rty = self
                    .fn_ret
                    .get(other)
                    .cloned()
                    .unwrap_or_else(|| "long long".into());
                if rty == "void" {
                    code.push_str(&format!(
                        "  oo_{}({});\n",
                        other,
                        cargs.join(", ")
                    ));
                    Ok((code, "0".into(), "int".into()))
                } else {
                    code.push_str(&format!(
                        "  {} {} = oo_{}({});\n",
                        rty,
                        t,
                        other,
                        cargs.join(", ")
                    ));
                    Ok((code, t, rty))
                }
            }
        }
    }
}

pub fn runtime_c_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        PathBuf::from("runtime/chs_rt.c"),
        manifest.join("runtime/chs_rt.c"),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    PathBuf::from("runtime/chs_rt.c")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(src: &str) -> Program {
        let mut l = Lexer::new(src);
        let t = l.tokenize().unwrap();
        Parser::new(t).parse_program().unwrap()
    }

    #[test]
    fn emits_c_for_int_main() {
        let p = parse("pub fn main() { println(42); }\n");
        let c = CCodeGen::emit_c(&p).expect("emit");
        assert!(c.contains("int main"), "{}", c);
        assert!(c.contains("oo_print_int"), "{}", c);
    }

    #[test]
    fn emits_c_for_list_and_string() {
        let p = parse(
            r#"
            pub fn main() {
                let mut xs = list_new();
                xs = list_push(xs, 10);
                println(list_len(xs));
                println(chars_len("ab"));
            }
            "#,
        );
        let c = CCodeGen::emit_c(&p).expect("emit");
        assert!(c.contains("oo_ilist_new") || c.contains("oo_ilist_push"), "{}", c);
        assert!(c.contains("oo_chars_len"), "{}", c);
    }

    #[test]
    fn emits_c_for_string_list_pending() {
        let p = parse(
            r#"
            pub fn main() {
                let mut xs = list_new();
                xs = list_push(xs, "a");
                xs = list_push(xs, "b");
                println(list_len(xs));
            }
            "#,
        );
        let c = CCodeGen::emit_c(&p).expect("emit");
        let main = c.split("int main").nth(1).unwrap_or(&c);
        assert!(
            main.contains("oo_slist_new") && main.contains("oo_slist_push"),
            "string list body must use slist: {}",
            main
        );
        assert!(
            !main.contains("oo_ilist_push") && !main.contains("OoIList xs"),
            "must not use int list for string elements: {}",
            main
        );
    }

    #[test]
    fn pure_chs_emit_omits_host_ffi_decls() {
        let p = parse(
            r#"
            pub fn main() {
                let mut xs = list_new();
                xs = list_push(xs, 1);
                println(list_len(xs));
            }
            "#,
        );
        assert!(
            !super::program_needs_host_ffi(&p),
            "pure list program must not need host FFI"
        );
        let c = CCodeGen::emit_c(&p).expect("emit");
        assert!(
            !c.contains("ooda_host") && !c.contains("oo_chs_build"),
            "pure emit must not declare host FFI (assembly depth): {}",
            c.lines().take(40).collect::<Vec<_>>().join("\n")
        );
    }

    #[test]
    fn chs_build_call_needs_host_ffi() {
        let p = parse(
            r#"
            pub fn main() {
                let r = chs_build("a.oo", "a.bin");
            }
            "#,
        );
        assert!(
            super::program_needs_host_ffi(&p),
            "chs_build must require host FFI / libooda"
        );
        let c = CCodeGen::emit_c(&p).expect("emit");
        assert!(
            c.contains("oo_chs_build"),
            "host-using emit must declare oo_chs_build"
        );
    }

    #[test]
    fn pure_chs_build_native_without_staticlib() {
        // Integration: pure program links with gcc+chs_rt only (no libooda.a required).
        let p = parse(
            r#"
            pub fn main() {
                println(1 + 2);
            }
            "#,
        );
        let rt = super::runtime_c_path();
        let out = std::env::temp_dir().join(format!("ooda_pure_chs_{}", std::process::id()));
        let _ = std::fs::remove_file(&out);
        CCodeGen::build_native(&p, &out, &rt, false).expect("pure build_native");
        assert!(out.exists(), "binary missing");
        let status = std::process::Command::new(&out).output().expect("run");
        assert!(status.status.success(), "pure binary failed");
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(out.with_extension("c"));
    }
}
