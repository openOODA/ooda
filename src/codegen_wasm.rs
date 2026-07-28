// ===================================================================
// openOODA WebAssembly (.wat) Code Generator (First Principles)
// ===================================================================
use crate::ast::*;
use anyhow::Result;

pub struct WasmCodeGen;

impl WasmCodeGen {
    pub fn emit_wat(program: &Program) -> Result<String> {
        let mut wat = String::new();
        wat.push_str(";; ===================================================================\n");
        wat.push_str(";; openOODA WebAssembly Text Format (.wat) Target Backend\n");
        wat.push_str(";; ===================================================================\n\n");
        wat.push_str("(module\n");
        wat.push_str("  (import \"env\" \"println\" (func $println (param i64)))\n");

        for item in &program.items {
            if let Item::Function(func) = item {
                wat.push_str(&Self::emit_function(func)?);
            }
        }

        wat.push_str(")\n");
        Ok(wat)
    }

    fn emit_function(func: &FunctionDecl) -> Result<String> {
        let mut f_wat = String::new();
        let is_main = func.name == "main";
        
        f_wat.push_str(&format!("  (func ${}", func.name));
        if is_main {
            f_wat.push_str(" (export \"main\")");
        }
        
        for param in &func.params {
            f_wat.push_str(&format!(" (param ${} i64)", param.name));
        }

        if is_main {
            f_wat.push_str(" (result i32)\n");
        } else {
            f_wat.push_str(" (result i64)\n");
        }

        for stmt in &func.body.stmts {
            match stmt {
                Statement::Return(Some(expr)) => {
                    let e_wat = Self::emit_expr(expr)?;
                    f_wat.push_str(&e_wat);
                    if is_main {
                        f_wat.push_str("    i32.wrap_i64\n");
                    }
                    f_wat.push_str("    return\n");
                }
                Statement::Expr(expr) => {
                    let e_wat = Self::emit_expr(expr)?;
                    f_wat.push_str(&e_wat);
                }
                _ => {}
            }
        }

        if let Some(body_expr) = &func.body.expr {
            let e_wat = Self::emit_expr(body_expr)?;
            f_wat.push_str(&e_wat);
            if is_main {
                f_wat.push_str("    i32.wrap_i64\n");
            }
        } else if is_main {
            f_wat.push_str("    i32.const 0\n");
        }

        f_wat.push_str("  )\n");
        Ok(f_wat)
    }

    fn emit_expr(expr: &Expression) -> Result<String> {
        let mut wat = String::new();
        match expr {
            Expression::Literal(Literal::Int(n)) => {
                wat.push_str(&format!("    i64.const {}\n", n));
            }
            Expression::Literal(Literal::Bool(b)) => {
                wat.push_str(&format!("    i64.const {}\n", if *b { 1 } else { 0 }));
            }
            Expression::Variable(name) => {
                wat.push_str(&format!("    local.get ${}\n", name));
            }
            Expression::Binary { op, left, right } => {
                wat.push_str(&Self::emit_expr(left)?);
                wat.push_str(&Self::emit_expr(right)?);
                match op {
                    BinOp::Add => wat.push_str("    i64.add\n"),
                    BinOp::Sub => wat.push_str("    i64.sub\n"),
                    BinOp::Mul => wat.push_str("    i64.mul\n"),
                    BinOp::Div => wat.push_str("    i64.div_s\n"),
                    BinOp::Eq => wat.push_str("    i64.eq\n    i64.extend_i32_u\n"),
                    BinOp::Gt => wat.push_str("    i64.gt_s\n    i64.extend_i32_u\n"),
                    BinOp::Lt => wat.push_str("    i64.lt_s\n    i64.extend_i32_u\n"),
                    _ => wat.push_str("    i64.add\n"),
                }
            }
            Expression::Call { name, args, .. } => {
                for arg in args {
                    wat.push_str(&Self::emit_expr(arg)?);
                }
                wat.push_str(&format!("    call ${}\n", name));
            }
            _ => {
                wat.push_str("    i64.const 0\n");
            }
        }
        Ok(wat)
    }
}
