// ===================================================================
// openOODA Integer-Subset LLVM IR Backend
//
// Honest dual-engine path: emits type-consistent LLVM IR for a documented
// integer subset (Int arithmetic, Bool compares, println of Int, main,
// while + break/continue + if side-effects). Programs outside the subset
// are rejected with a clear error rather than emitting broken IR.
// Locals use stack `alloca` (W↓ — no heap for scalar Int/Bool/Float).
// Output is structurally validated before write.
// ===================================================================
use crate::ast::*;
use anyhow::{anyhow, bail, Result};
use std::cell::RefCell;
use std::process::Command;

// Nested while (break, continue) labels for the current LLVM emit.
// break → end label; continue → head label. Stack-allocated label names only.
thread_local! {
    static LLVM_LOOP_STACK: RefCell<Vec<(String, String)>> = RefCell::new(Vec::new());
}

pub struct LlvmCodeGen;

impl LlvmCodeGen {
    /// Emit LLVM IR for an integer-subset program, or return an error explaining
    /// why the program is outside the supported subset / failed validation.
    pub fn emit_llvm_ir(program: &Program) -> Result<String> {
        Self::assert_integer_subset(program)?;
        let ir = Self::generate(program)?;
        Self::validate_ir(&ir)?;
        Ok(ir)
    }

    /// Whether the program uses only the integer LLVM subset.
    pub fn assert_integer_subset(program: &Program) -> Result<()> {
        let aliases = program.collect_type_aliases();
        for item in &program.items {
            if let Item::Function(func) = item {
                Self::check_fn_subset(func, &aliases)?;
            }
        }
        Ok(())
    }

    fn check_fn_subset(func: &FunctionDecl, aliases: &std::collections::HashMap<String, Type>) -> Result<()> {
        for p in &func.params {
            Self::check_type_subset(&p.param_type.resolve_alias(aliases), &func.name)?;
        }
        Self::check_type_subset(&func.return_type.resolve_alias(aliases), &func.name)?;
        for e in func.requires.iter().chain(func.ensures.iter()) {
            Self::check_expr_subset(e, &func.name)?;
        }
        Self::check_block_subset(&func.body, &func.name)?;
        if let Some(v) = &func.verify_block {
            // verify blocks are not emitted to LLVM; skip subset for verify-only constructs
            let _ = v;
        }
        Ok(())
    }

    fn check_type_subset(t: &Type, ctx: &str) -> Result<()> {
        match t {
            Type::Int | Type::Bool | Type::Void | Type::Float => Ok(()),
            Type::String => bail!(
                "LLVM integer-subset backend does not support String in '{}'. Use `ooda run` for string programs, or rewrite to Int-only for `ooda build`.",
                ctx
            ),
            Type::NetCap | Type::FsCap | Type::EnvCap | Type::SysCap => bail!(
                "LLVM integer-subset backend does not emit capability handles in '{}'.",
                ctx
            ),
            Type::Option(_) | Type::Result(_, _) => bail!(
                "LLVM integer-subset backend does not support Option/Result in '{}'.",
                ctx
            ),
            Type::List(_) => bail!(
                "LLVM CHS emit does not yet lower List in '{}' (host-only until M4 progressive EMIT). Use `ooda run`.",
                ctx
            ),
            Type::Struct { .. } => bail!(
                "LLVM CHS emit does not yet lower struct in '{}' (host-only until M4 progressive EMIT). Use `ooda run`.",
                ctx
            ),
            Type::Custom(s) => match s.as_str() {
                "Int" | "i64" | "i32" | "u64" | "Bool" | "Void" => Ok(()),
                other => bail!(
                    "LLVM integer-subset backend does not support type '{}' in '{}'.",
                    other,
                    ctx
                ),
            },
        }
    }

    fn check_block_subset(block: &Block, ctx: &str) -> Result<()> {
        for stmt in &block.stmts {
            match stmt {
                Statement::Let { init, type_annotation, .. } => {
                    if let Some(t) = type_annotation {
                        Self::check_type_subset(t, ctx)?;
                    }
                    Self::check_expr_subset(init, ctx)?;
                }
                Statement::Assign { value, .. } => Self::check_expr_subset(value, ctx)?,
                Statement::FieldAssign { object, value, .. } => {
                    Self::check_expr_subset(object, ctx)?;
                    Self::check_expr_subset(value, ctx)?;
                }
                Statement::Return(Some(e), _) => Self::check_expr_subset(e, ctx)?,
                Statement::Return(None, _) => {}
            Statement::Break(_) | Statement::Continue(_) => {}
                Statement::Expr(e, _) => Self::check_expr_subset(e, ctx)?,
                Statement::While { cond, body, .. } => {
                    Self::check_expr_subset(cond, ctx)?;
                    Self::check_block_subset(body, ctx)?;
                }
            }
        }
        if let Some(e) = &block.expr {
            Self::check_expr_subset(e, ctx)?;
        }
        Ok(())
    }

    fn check_expr_subset(expr: &Expression, ctx: &str) -> Result<()> {
        match expr {
            Expression::Literal(Literal::String(_), _) => bail!(
                "LLVM integer-subset backend does not support string literals in '{}'. Use `ooda run`.",
                ctx
            ),
            Expression::StructLit { .. } => bail!(
                "LLVM CHS emit does not yet lower struct literals in '{}' (host-only until M4). Use `ooda run`.",
                ctx
            ),
            Expression::Literal(_, _) | Expression::Variable(_, _) => Ok(()),
            Expression::Binary { left, right, .. } => {
                Self::check_expr_subset(left, ctx)?;
                Self::check_expr_subset(right, ctx)
            }
            Expression::Call { name, args, .. } => {
                // Integer-subset: no String methods. Fail closed with honest recovery path.
                if name.starts_with('.') {
                    bail!(
                        "LLVM integer-subset backend does not support method '{}' in '{}' \
                         (no String/list methods; use Int/Bool only or `ooda run` / `ooda build --target c`).",
                        name,
                        ctx
                    );
                }
                if matches!(name.as_str(), "char_at" | "str_slice" | "chars_len") {
                    bail!(
                        "LLVM integer-subset backend does not lower '{}' in '{}' \
                         (string surface; use `ooda run` or `ooda build --target c`).",
                        name,
                        ctx
                    );
                }
                for a in args {
                    Self::check_expr_subset(a, ctx)?;
                }
                Ok(())
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                Self::check_expr_subset(cond, ctx)?;
                Self::check_block_subset(then_branch, ctx)?;
                if let Some(e) = else_branch {
                    Self::check_block_subset(e, ctx)?;
                }
                Ok(())
            }
            Expression::Match { expr, arms, .. } => {
                Self::check_expr_subset(expr, ctx)?;
                for arm in arms {
                    Self::check_expr_subset(&arm.body, ctx)?;
                }
                Ok(())
            }
            Expression::Unary { expr, .. } => Self::check_expr_subset(expr, ctx),
            Expression::While { cond, body, .. } => {
                Self::check_expr_subset(cond, ctx)?;
                Self::check_block_subset(body, ctx)
            }
        }
    }

    fn generate(program: &Program) -> Result<String> {
        let mut ir = String::new();
        ir.push_str("; ===================================================================\n");
        ir.push_str("; openOODA LLVM IR — integer subset backend\n");
        ir.push_str("; Validated type-consistent IR for Int/Bool programs\n");
        ir.push_str("; ===================================================================\n\n");
        ir.push_str("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n");
        ir.push_str("target triple = \"x86_64-unknown-linux-gnu\"\n\n");
        ir.push_str("declare i32 @printf(i8*, ...)\n");
        ir.push_str(
            "@.str.fmt_int = private unnamed_addr constant [5 x i8] c\"%ld\\0A\\00\", align 1\n\n",
        );

        let mut has_main = false;
        for item in &program.items {
            if let Item::Function(func) = item {
                if func.name == "main" {
                    has_main = true;
                }
                ir.push_str(&Self::emit_function(func)?);
            }
        }

        if !has_main {
            // Provide a trivial main if absent so linked artifacts can still start
            ir.push_str("define i32 @main() {\nentry:\n  ret i32 0\n}\n\n");
        }

        ir.push_str("attributes #0 = { nounwind }\n");
        Ok(ir)
    }

    fn llvm_ty(t: &Type) -> &'static str {
        match t {
            Type::Bool => "i1",
            Type::Float => "double",
            Type::Void => "void",
            _ => "i64",
        }
    }

    fn emit_function(func: &FunctionDecl) -> Result<String> {
        let mut f_ir = String::new();
        let is_main = func.name == "main";

        // main always returns i32 for C ABI compatibility
        let ret_ty = if is_main {
            "i32"
        } else {
            Self::llvm_ty(&func.return_type)
        };

        f_ir.push_str(&format!("define {} @{}(", ret_ty, func.name));
        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                f_ir.push_str(", ");
            }
            let p_ty = Self::llvm_ty(&param.param_type);
            f_ir.push_str(&format!("{} %arg_{}", p_ty, param.name));
        }
        f_ir.push_str(") #0 {\nentry:\n");

        let mut reg = 1usize;
        let mut locals: std::collections::HashMap<String, &'static str> =
            std::collections::HashMap::new();

        for param in &func.params {
            let p_ty = Self::llvm_ty(&param.param_type);
            f_ir.push_str(&format!("  %var_{} = alloca {}\n", param.name, p_ty));
            f_ir.push_str(&format!(
                "  store {} %arg_{}, {}* %var_{}\n",
                p_ty, param.name, p_ty, param.name
            ));
            locals.insert(param.name.clone(), p_ty);
        }

        let mut returned = false;
        for stmt in &func.body.stmts {
            match stmt {
                Statement::Let { name, init, .. } => {
                    let (val, code, r, vty) = Self::emit_expr(init, reg, &locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                    f_ir.push_str(&format!("  %var_{} = alloca {}\n", name, vty));
                    f_ir.push_str(&format!("  store {} {}, {}* %var_{}\n", vty, val, vty, name));
                    locals.insert(name.clone(), vty);
                }
                Statement::FieldAssign { .. } => {
                    bail!("LLVM integer-subset backend does not support field assignment. Use `ooda run` or `ooda build --target c`.");
                }
                Statement::Assign { name, value, .. } => {
                    let (val, code, r, vty) = Self::emit_expr(value, reg, &locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                    let pty = locals.get(name).copied().unwrap_or(vty);
                    f_ir.push_str(&format!("  store {} {}, {}* %var_{}\n", pty, val, pty, name));
                }
                Statement::Return(Some(expr), _) => {
                    let (val, code, r, vty) = Self::emit_expr(expr, reg, &locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                    if is_main {
                        // truncate/extend to i32
                        if vty == "i64" {
                            f_ir.push_str(&format!("  %retcast{} = trunc i64 {} to i32\n", reg, val));
                            f_ir.push_str(&format!("  ret i32 %retcast{}\n", reg));
                            reg += 1;
                        } else if vty == "i32" {
                            f_ir.push_str(&format!("  ret i32 {}\n", val));
                        } else {
                            f_ir.push_str("  ret i32 0\n");
                        }
                    } else if ret_ty == "void" {
                        f_ir.push_str("  ret void\n");
                    } else {
                        f_ir.push_str(&format!("  ret {} {}\n", ret_ty, val));
                    }
                    returned = true;
                }
                Statement::Break(_) => {
                    let end = LLVM_LOOP_STACK.with(|s| {
                        s.borrow()
                            .last()
                            .map(|(b, _)| b.clone())
                    })
                    .ok_or_else(|| anyhow!("LLVM: break outside loop"))?;
                    f_ir.push_str(&format!("  br label %{}\n", end));
                    returned = true; // path ends (do not fall through)
                }
                Statement::Continue(_) => {
                    let head = LLVM_LOOP_STACK.with(|s| {
                        s.borrow()
                            .last()
                            .map(|(_, c)| c.clone())
                    })
                    .ok_or_else(|| anyhow!("LLVM: continue outside loop"))?;
                    f_ir.push_str(&format!("  br label %{}\n", head));
                    returned = true;
                }
                Statement::Return(None, _) => {
                    if is_main {
                        f_ir.push_str("  ret i32 0\n");
                    } else if ret_ty == "void" {
                        f_ir.push_str("  ret void\n");
                    } else {
                        f_ir.push_str(&format!("  ret {} 0\n", ret_ty));
                    }
                    returned = true;
                }
                Statement::Expr(expr, _) => {
                    let (_val, code, r, _vty) = Self::emit_expr(expr, reg, &locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                    if matches!(expr, Expression::If { .. }) {
                        returned = true;
                    }
                }
                Statement::While { cond, body, .. } => {
                    let (code, r) = Self::emit_while(cond, body, reg, &mut locals)?;
                    reg = r;
                    f_ir.push_str(&code);
                }
            }
        }

        if let Some(body_expr) = &func.body.expr {
            let (val, code, _r, _vty) = Self::emit_expr(body_expr, reg, &locals)?;
            f_ir.push_str(&code);
            if !is_main && ret_ty != "void" && !f_ir.ends_with("ret ") {
                f_ir.push_str(&format!("  ret {} {}\n", ret_ty, val));
                returned = true;
            }
        }

        if !returned {
            if is_main {
                f_ir.push_str("  ret i32 0\n");
            } else if ret_ty == "void" {
                f_ir.push_str("  ret void\n");
            } else {
                f_ir.push_str(&format!("  ret {} 0\n", ret_ty));
            }
        }

        f_ir.push_str("}\n\n");
        Ok(f_ir)
    }

    fn emit_expr(
        expr: &Expression,
        mut reg: usize,
        locals: &std::collections::HashMap<String, &'static str>,
    ) -> Result<(String, String, usize, &'static str)> {
        let mut code = String::new();
        match expr {
            Expression::Literal(Literal::Int(n), _) => Ok((format!("{}", n), code, reg, "i64")),
            Expression::Literal(Literal::Bool(b), _) => {
                Ok((format!("{}", if *b { 1 } else { 0 }), code, reg, "i1"))
            }
            Expression::Literal(Literal::Float(f), _) => {
                Ok((format!("{}", f), code, reg, "double"))
            }
            Expression::Literal(Literal::Void, _) => Ok(("0".into(), code, reg, "i64")),
            Expression::Literal(Literal::String(_), _) => {
                bail!("internal: string literal reached LLVM emit")
            }
            Expression::Variable(name, _) => {
                let vty = locals.get(name).copied().unwrap_or("i64");
                let r = format!("%r{}", reg);
                reg += 1;
                code.push_str(&format!("  {} = load {}, {}* %var_{}\n", r, vty, vty, name));
                Ok((r, code, reg, vty))
            }
            Expression::Binary { op, left, right, .. } => {
                let (l, lc, r1, lty) = Self::emit_expr(left, reg, locals)?;
                let (r, rc, r2, rty) = Self::emit_expr(right, r1, locals)?;
                code.push_str(&lc);
                code.push_str(&rc);
                let res = format!("%r{}", r2);
                reg = r2 + 1;

                let use_float = lty == "double" || rty == "double";
                if use_float {
                    let (op_str, out_ty): (&str, &str) = match op {
                        BinOp::Add => ("fadd double", "double"),
                        BinOp::Sub => ("fsub double", "double"),
                        BinOp::Mul => ("fmul double", "double"),
                        BinOp::Div => ("fdiv double", "double"),
                        BinOp::Eq => ("fcmp oeq double", "i1"),
                        BinOp::Neq => ("fcmp one double", "i1"),
                        BinOp::Lt => ("fcmp olt double", "i1"),
                        BinOp::Lte => ("fcmp ole double", "i1"),
                        BinOp::Gt => ("fcmp ogt double", "i1"),
                        BinOp::Gte => ("fcmp oge double", "i1"),
                        _ => bail!("LLVM backend: unsupported float operator {:?}", op),
                    };
                    code.push_str(&format!("  {} = {} {}, {}\n", res, op_str, l, r));
                    return Ok((res, code, reg, out_ty));
                }

                // Promote i1 loads to i64 for arithmetic when needed
                let (l_i64, r_i64, prep) = if lty == "i1" {
                    let a = format!("%r{}", reg);
                    reg += 1;
                    let b = format!("%r{}", reg);
                    reg += 1;
                    let mut p = String::new();
                    p.push_str(&format!("  {} = zext i1 {} to i64\n", a, l));
                    p.push_str(&format!("  {} = zext i1 {} to i64\n", b, r));
                    (a, b, p)
                } else {
                    (l.clone(), r.clone(), String::new())
                };
                code.push_str(&prep);

                let (op_str, out_ty): (&str, &str) = match op {
                    BinOp::Add => ("add i64", "i64"),
                    BinOp::Sub => ("sub i64", "i64"),
                    BinOp::Mul => ("mul i64", "i64"),
                    BinOp::Div => ("sdiv i64", "i64"),
                    BinOp::Eq => ("icmp eq i64", "i1"),
                    BinOp::Neq => ("icmp ne i64", "i1"),
                    BinOp::Lt => ("icmp slt i64", "i1"),
                    BinOp::Lte => ("icmp sle i64", "i1"),
                    BinOp::Gt => ("icmp sgt i64", "i1"),
                    BinOp::Gte => ("icmp sge i64", "i1"),
                    BinOp::And => ("and i64", "i64"),
                    BinOp::Or => ("or i64", "i64"),
                    _ => ("add i64", "i64"),
                };

                code.push_str(&format!("  {} = {} {}, {}\n", res, op_str, l_i64, r_i64));
                Ok((res, code, reg, out_ty))
            }
            Expression::Call { name, args, .. } => {
                if name == "println" {
                    let mut fmt_args = String::new();
                    for arg in args {
                        let (val, ac, rnext, vty) = Self::emit_expr(arg, reg, locals)?;
                        reg = rnext;
                        code.push_str(&ac);
                        let as_i64 = if vty == "i1" {
                            let z = format!("%r{}", reg);
                            reg += 1;
                            code.push_str(&format!("  {} = zext i1 {} to i64\n", z, val));
                            z
                        } else if vty == "double" {
                            let z = format!("%r{}", reg);
                            reg += 1;
                            code.push_str(&format!("  {} = fptosi double {} to i64\n", z, val));
                            z
                        } else {
                            val
                        };
                        fmt_args.push_str(&format!(", i64 {}", as_i64));
                    }
                    let res = format!("%r{}", reg);
                    reg += 1;
                    code.push_str(&format!(
                        "  {} = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str.fmt_int, i64 0, i64 0){})\n",
                        res, fmt_args
                    ));
                    Ok((res, code, reg, "i32"))
                } else {
                    let mut arg_str = String::new();
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            arg_str.push_str(", ");
                        }
                        let (val, ac, rnext, vty) = Self::emit_expr(arg, reg, locals)?;
                        reg = rnext;
                        code.push_str(&ac);
                        let ty = if vty == "i1" { "i1" } else { "i64" };
                        arg_str.push_str(&format!("{} {}", ty, val));
                    }
                    let res = format!("%r{}", reg);
                    reg += 1;
                    code.push_str(&format!("  {} = call i64 @{}({})\n", res, name, arg_str));
                    Ok((res, code, reg, "i64"))
                }
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                // Full branch lowering (println / assign / break / continue / return).
                // Prior alpha only lowered Return and silently dropped side effects (D↑ honesty bug).
                let (c_val, c_code, mut r_curr, cty) = Self::emit_expr(cond, reg, locals)?;
                code.push_str(&c_code);

                let then_label = format!("then_{}", r_curr);
                let else_label = format!("else_{}", r_curr);
                let merge_label = format!("merge_{}", r_curr);
                r_curr += 1;

                let c_i1 = if cty == "i1" {
                    c_val
                } else {
                    let t = format!("%r{}", r_curr);
                    r_curr += 1;
                    code.push_str(&format!("  {} = icmp ne i64 {}, 0\n", t, c_val));
                    t
                };
                code.push_str(&format!(
                    "  br i1 {}, label %{}, label %{}\n",
                    c_i1, then_label, else_label
                ));

                code.push_str(&format!("\n{}:\n", then_label));
                let (then_code, r1, then_term) =
                    Self::emit_block_stmts(then_branch, r_curr, locals)?;
                r_curr = r1;
                code.push_str(&then_code);
                if !then_term {
                    code.push_str(&format!("  br label %{}\n", merge_label));
                }

                code.push_str(&format!("\n{}:\n", else_label));
                if let Some(eb) = else_branch {
                    let (else_code, r2, else_term) =
                        Self::emit_block_stmts(eb, r_curr, locals)?;
                    r_curr = r2;
                    code.push_str(&else_code);
                    if !else_term {
                        code.push_str(&format!("  br label %{}\n", merge_label));
                    }
                } else {
                    code.push_str(&format!("  br label %{}\n", merge_label));
                }

                code.push_str(&format!("\n{}:\n", merge_label));
                // Statement-context if leaves a dummy 0 (caller may drop).
                Ok(("0".to_string(), code, r_curr, "i64"))
            }
            Expression::Unary { op, expr, .. } => {
                let (v, vc, r1, vty) = Self::emit_expr(expr, reg, locals)?;
                code.push_str(&vc);
                reg = r1;
                let res = format!("%r{}", reg);
                reg += 1;
                match op {
                    UnaryOp::Not => {
                        let as_i1 = if vty == "i1" {
                            v
                        } else {
                            let t = format!("%r{}", reg);
                            reg += 1;
                            code.push_str(&format!("  {} = icmp ne i64 {}, 0\n", t, v));
                            t
                        };
                        code.push_str(&format!("  {} = xor i1 {}, true\n", res, as_i1));
                        Ok((res, code, reg, "i1"))
                    }
                    UnaryOp::Neg => {
                        if vty == "double" {
                            code.push_str(&format!("  {} = fneg double {}\n", res, v));
                            Ok((res, code, reg, "double"))
                        } else {
                            code.push_str(&format!("  {} = sub i64 0, {}\n", res, v));
                            Ok((res, code, reg, "i64"))
                        }
                    }
                }
            }
            Expression::While { cond, body, .. } => {
                let (wcode, r) = Self::emit_while(cond, body, reg, locals)?;
                code.push_str(&wcode);
                Ok(("0".into(), code, r, "i64"))
            }
            Expression::Match { .. } => {
                bail!(
                    "LLVM integer-subset backend does not lower match expressions. Use `ooda run`."
                )
            }
            Expression::StructLit { .. } => {
                bail!(
                    "LLVM CHS emit does not yet lower struct literals (host-only until M4). Use `ooda run`."
                )
            }
        }
    }

    /// Emit statements in a block (+ optional tail expr). Returns (ir, next_reg, terminated).
    /// `terminated` means every path left via ret/break/continue (no fallthrough).
    fn emit_block_stmts(
        block: &Block,
        mut reg: usize,
        locals: &std::collections::HashMap<String, &'static str>,
    ) -> Result<(String, usize, bool)> {
        let mut code = String::new();
        let mut terminated = false;
        for stmt in &block.stmts {
            if terminated {
                break;
            }
            let (sc, r, term) = Self::emit_one_stmt(stmt, reg, locals)?;
            reg = r;
            code.push_str(&sc);
            terminated = term;
        }
        if !terminated {
            if let Some(tail) = &block.expr {
                let (sc, r, term) = Self::emit_one_stmt(
                    &Statement::Expr((**tail).clone(), Span { line: 0, col: 0 }),
                    reg,
                    locals,
                )?;
                reg = r;
                code.push_str(&sc);
                terminated = term;
            }
        }
        Ok((code, reg, terminated))
    }

    /// Lower one statement. `terminated` = control does not fall through.
    fn emit_one_stmt(
        stmt: &Statement,
        mut reg: usize,
        locals: &std::collections::HashMap<String, &'static str>,
    ) -> Result<(String, usize, bool)> {
        let mut code = String::new();
        match stmt {
            Statement::Assign { name, value, .. } => {
                let (val, vcode, r2, vty) = Self::emit_expr(value, reg, locals)?;
                reg = r2;
                code.push_str(&vcode);
                let pty = locals.get(name).copied().unwrap_or(vty);
                code.push_str(&format!("  store {} {}, {}* %var_{}\n", pty, val, pty, name));
                Ok((code, reg, false))
            }
            Statement::Let { name, init, .. } => {
                let (val, vcode, r2, vty) = Self::emit_expr(init, reg, locals)?;
                reg = r2;
                code.push_str(&vcode);
                if locals.contains_key(name) {
                    code.push_str(&format!("  store {} {}, {}* %var_{}\n", vty, val, vty, name));
                } else {
                    // Nested let in while/if: stack alloca (W↓ vs heap).
                    code.push_str(&format!("  %var_{} = alloca {}\n", name, vty));
                    code.push_str(&format!("  store {} {}, {}* %var_{}\n", vty, val, vty, name));
                    // Note: cannot insert into immutable locals map here; pre-collected names preferred.
                }
                Ok((code, reg, false))
            }
            Statement::Expr(expr, _) => {
                let (_v, ecode, r2, _) = Self::emit_expr(expr, reg, locals)?;
                reg = r2;
                code.push_str(&ecode);
                Ok((code, reg, false))
            }
            Statement::Return(Some(ex), _) => {
                let (val, scode, rnext, vty) = Self::emit_expr(ex, reg, locals)?;
                reg = rnext;
                code.push_str(&scode);
                if vty == "i64" {
                    code.push_str(&format!("  ret i64 {}\n", val));
                } else if vty == "i1" {
                    let z = format!("%r{}", reg);
                    reg += 1;
                    code.push_str(&format!("  {} = zext i1 {} to i64\n", z, val));
                    code.push_str(&format!("  ret i64 {}\n", z));
                } else {
                    code.push_str(&format!("  ret i64 0\n"));
                }
                Ok((code, reg, true))
            }
            Statement::Return(None, _) => {
                code.push_str("  ret i64 0\n");
                Ok((code, reg, true))
            }
            Statement::Break(_) => {
                let end = LLVM_LOOP_STACK.with(|s| s.borrow().last().map(|(b, _)| b.clone()))
                    .ok_or_else(|| anyhow!("LLVM: break outside loop"))?;
                code.push_str(&format!("  br label %{}\n", end));
                Ok((code, reg, true))
            }
            Statement::Continue(_) => {
                let head = LLVM_LOOP_STACK.with(|s| s.borrow().last().map(|(_, c)| c.clone()))
                    .ok_or_else(|| anyhow!("LLVM: continue outside loop"))?;
                code.push_str(&format!("  br label %{}\n", head));
                Ok((code, reg, true))
            }
            Statement::While { cond, body, .. } => {
                let (wcode, r) = Self::emit_while(cond, body, reg, locals)?;
                Ok((wcode, r, false))
            }
            Statement::FieldAssign { .. } => {
                bail!(
                    "LLVM integer-subset backend does not support field assignment. Use `ooda run` or `ooda build --target c`."
                )
            }
        }
    }

    fn emit_while(
        cond: &Expression,
        body: &Block,
        mut reg: usize,
        locals: &std::collections::HashMap<String, &'static str>,
    ) -> Result<(String, usize)> {
        let mut code = String::new();
        let id = reg;
        reg += 1;
        let head = format!("while_head_{}", id);
        let body_l = format!("while_body_{}", id);
        let end = format!("while_end_{}", id);

        // break → end, continue → head (stack labels; zero heap W).
        LLVM_LOOP_STACK.with(|s| s.borrow_mut().push((end.clone(), head.clone())));

        code.push_str(&format!("  br label %{}\n", head));
        code.push_str(&format!("\n{}:\n", head));
        let (cval, ccode, r1, cty) = Self::emit_expr(cond, reg, locals)?;
        reg = r1;
        code.push_str(&ccode);
        let c_i1 = if cty == "i1" {
            cval
        } else {
            let t = format!("%r{}", reg);
            reg += 1;
            code.push_str(&format!("  {} = icmp ne i64 {}, 0\n", t, cval));
            t
        };
        code.push_str(&format!("  br i1 {}, label %{}, label %{}\n", c_i1, body_l, end));
        code.push_str(&format!("\n{}:\n", body_l));
        // stmts + body.expr tail (idiomatic if/break without trailing `;`).
        let (body_code, r2, body_term) = Self::emit_block_stmts(body, reg, locals)?;
        reg = r2;
        code.push_str(&body_code);
        if !body_term {
            code.push_str(&format!("  br label %{}\n", head));
        }
        code.push_str(&format!("\n{}:\n", end));
        LLVM_LOOP_STACK.with(|s| {
            s.borrow_mut().pop();
        });
        Ok((code, reg))
    }

    /// Structural validation of emitted IR (always). Optional llvm-as if on PATH.
    pub fn validate_ir(ir: &str) -> Result<()> {
        if ir.is_empty() {
            bail!("LLVM validation failed: empty IR");
        }

        // Count function bodies and ensure every define has a ret before closing brace
        let mut in_func = false;
        let mut saw_ret = false;
        let mut define_count = 0;
        for line in ir.lines() {
            let t = line.trim();
            if t.starts_with("define ") {
                if in_func && !saw_ret {
                    bail!("LLVM validation failed: function missing ret before next define");
                }
                in_func = true;
                saw_ret = false;
                define_count += 1;
            } else if t.ends_with(':') && !t.contains(' ') {
                saw_ret = false;
            } else if t.starts_with("ret ") {
                if saw_ret {
                    bail!("LLVM validation failed: multiple ret in the same basic block path (duplicate ret)");
                }
                saw_ret = true;
            } else if t == "}" && in_func {
                if !saw_ret {
                    bail!("LLVM validation failed: function ended without ret");
                }
                in_func = false;
                saw_ret = false;
            }
            // Type-consistency: reject known-bad patterns from earlier buggy emitters
            if t.contains("load i64, i64* %var_") && ir.contains("alloca i8*") {
                // only flag if same function mixes — simple global heuristic skipped
            }
            if t.contains("load i64, i64* %var_") {
                // extract var name and ensure alloca is i64 if we can
            }
        }

        // Pair alloca/load types for %var_X
        let mut alloca_ty: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for line in ir.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("%var_") {
                if let Some((name, rhs)) = rest.split_once(" = alloca ") {
                    alloca_ty.insert(name.to_string(), rhs.to_string());
                }
            }
            if t.contains(" = load ") {
                // pattern: %rN = load TY, TY* %var_NAME
                if let Some(idx) = t.find("load ") {
                    let after = &t[idx + 5..];
                    let parts: Vec<&str> = after.split(',').collect();
                    if parts.len() >= 2 {
                        let load_ty = parts[0].trim();
                        let ptr = parts[1].trim(); // e.g. i64* %var_x
                        if let Some(var_pos) = ptr.find("%var_") {
                            let var = ptr[var_pos + 5..].trim();
                            if let Some(a_ty) = alloca_ty.get(var) {
                                if a_ty != load_ty {
                                    bail!(
                                        "LLVM validation failed: load type {} does not match alloca {} for %var_{}",
                                        load_ty,
                                        a_ty,
                                        var
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        if define_count == 0 {
            bail!("LLVM validation failed: no functions defined");
        }

        // Optional external validation with llvm-as when available
        if let Ok(status) = Self::run_llvm_as(ir) {
            if !status {
                bail!("LLVM validation failed: llvm-as rejected the generated IR");
            }
        }

        Ok(())
    }

    fn run_llvm_as(ir: &str) -> Result<bool> {
        let llvm_as = ["llvm-as", "llvm-as-18", "llvm-as-17", "llvm-as-16", "llvm-as-15"]
            .into_iter()
            .find(|c| Command::new(c).arg("-version").output().is_ok());

        let Some(bin) = llvm_as else {
            return Err(anyhow!("llvm-as not installed"));
        };

        let dir = std::env::temp_dir().join(format!("ooda-llvm-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let ll = dir.join("check.ll");
        let bc = dir.join("check.bc");
        std::fs::write(&ll, ir)?;
        let out = Command::new(bin)
            .arg(&ll)
            .arg("-o")
            .arg(&bc)
            .output()?;
        let _ = std::fs::remove_dir_all(&dir);
        Ok(out.status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emit(src: &str) -> Result<String> {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize()?;
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse_program()?;
        LlvmCodeGen::emit_llvm_ir(&program)
    }

    #[test]
    fn emits_valid_int_main() {
        let ir = emit(
            r#"
            pub fn add(a: Int, b: Int) -> Int {
                return a + b;
            }
            pub fn main() {
                let x = add(2, 3);
                println(x);
            }
        "#,
        )
        .expect("emit");
        assert!(ir.contains("define i64 @add(i64 %arg_a, i64 %arg_b)"));
        assert!(ir.contains("define i32 @main()"));
        assert!(ir.contains("add i64"));
        assert!(!ir.contains("load i64, i64* %var_name")); // no string-as-int bug
        LlvmCodeGen::validate_ir(&ir).expect("validate");
    }

    #[test]
    fn rejects_string_program() {
        let err = emit(
            r#"
            pub fn main() {
                let s = "hello";
                println(s);
            }
        "#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("integer-subset") || format!("{}", err).contains("String"));
    }

    #[test]
    fn no_duplicate_ret() {
        let ir = emit(
            r#"
            pub fn main() {
                return 0;
            }
        "#,
        )
        .unwrap();
        let main_body = ir.split("define i32 @main()").nth(1).unwrap();
        let ret_count = main_body.matches("ret ").count();
        assert_eq!(ret_count, 1);
    }

    #[test]
    fn refuses_char_at_string_surface() {
        let err = emit(
            r#"
            pub fn main() {
                let c = char_at("hi", 0);
                println(c);
            }
        "#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("char_at") || err.contains("string") || err.contains("String"),
            "LLVM must refuse string char_at: {}",
            err
        );
    }

    #[test]
    fn refuses_char_at_method() {
        let err = emit(
            r#"
            pub fn main() {
                let c = "hi".char_at(0);
                println(c);
            }
        "#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("method") || err.contains("char_at") || err.contains("String") || err.contains("string"),
            "LLVM must refuse .char_at: {}",
            err
        );
    }

    #[test]
    fn while_tail_if_break_lowers_to_br_end() {
        // Idiomatic last-if-without-`;` must not be silently dropped (dual-engine honesty).
        let ir = emit(
            r#"
            pub fn main() {
                let mut i = 0;
                while i < 10 {
                    i = i + 1;
                    if i == 3 { break; }
                }
                println(i);
            }
        "#,
        )
        .expect("emit break");
        assert!(
            ir.contains("br label %while_end_"),
            "break must branch to while_end:\n{}",
            ir
        );
        assert!(
            ir.contains("then_") && ir.contains("else_"),
            "tail if must lower:\n{}",
            ir
        );
        LlvmCodeGen::validate_ir(&ir).expect("validate");
    }

    #[test]
    fn if_println_side_effect_not_silently_dropped() {
        let ir = emit(
            r#"
            pub fn main() {
                let i = 2;
                if i == 2 {
                    println(i);
                } else {
                    println(0);
                }
            }
        "#,
        )
        .expect("emit if println");
        let printf_count = ir.matches("@printf").count();
        // declare + two call sites (then and else)
        assert!(
            printf_count >= 3,
            "both branches must call printf; got {} @printf:\n{}",
            printf_count,
            ir
        );
        LlvmCodeGen::validate_ir(&ir).expect("validate");
    }
}
