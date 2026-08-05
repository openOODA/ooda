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

}
