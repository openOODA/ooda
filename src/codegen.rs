// ===================================================================
// openOODA AST-to-LLVM IR Code Generator Engine
// Converts OODA AST nodes into compilable LLVM Assembly Text (.ll)
// ===================================================================
use crate::ast::*;

pub struct LlvmCodeGen;

impl LlvmCodeGen {
    pub fn emit_llvm_ir(program: &Program) -> String {
        let mut ir = String::new();

        ir.push_str("; ===================================================================\n");
        ir.push_str("; openOODA LLVM IR Target Code Generator Output\n");
        ir.push_str("; Target Architecture: x86_64 / ARM64 Native Bare-Metal\n");
        ir.push_str("; ===================================================================\n\n");

        ir.push_str("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n");
        ir.push_str("target triple = \"x86_64-unknown-linux-gnu\"\n\n");

        ir.push_str("declare i32 @printf(i8*, ...)\n");
        ir.push_str("@.str.fmt_int = private unnamed_addr constant [5 x i8] c\"%ld\\0A\\00\", align 1\n");
        ir.push_str("@.str.fmt_str = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\", align 1\n\n");

        for item in &program.items {
            if let Item::Function(func) = item {
                ir.push_str(&Self::emit_function(func));
            }
        }

        ir.push_str("attributes #0 = { nounwind }\n");
        ir
    }

    fn emit_function(func: &FunctionDecl) -> String {
        let mut f_ir = String::new();

        let ret_type = match func.return_type {
            Type::Int => "i64",
            Type::Float => "double",
            Type::Bool => "i1",
            _ => "i32",
        };

        f_ir.push_str(&format!("define {} @{}(", ret_type, func.name));
        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                f_ir.push_str(", ");
            }
            let p_type = match param.param_type {
                Type::Int => "i64",
                Type::Float => "double",
                Type::Bool => "i1",
                _ => "i8*",
            };
            f_ir.push_str(&format!("{} %arg_{}", p_type, param.name));
        }
        f_ir.push_str(") #0 {\nentry:\n");

        let mut reg_counter = 1;

        // Allocate stack space for parameters
        for param in &func.params {
            let p_type = match param.param_type {
                Type::Int => "i64",
                Type::Float => "double",
                Type::Bool => "i1",
                _ => "i8*",
            };
            f_ir.push_str(&format!("  %var_{} = alloca {}\n", param.name, p_type));
            f_ir.push_str(&format!("  store {} %arg_{}, {}* %var_{}\n", p_type, param.name, p_type, param.name));
        }

        // Emit statements
        let mut last_reg = "%reg_zero".to_string();
        for stmt in &func.body.stmts {
            match stmt {
                Statement::Let { name, init, .. } => {
                    f_ir.push_str(&format!("  %var_{} = alloca i64\n", name));
                    let (val_reg, code, r_count) = Self::emit_expr(init, reg_counter);
                    reg_counter = r_count;
                    f_ir.push_str(&code);
                    f_ir.push_str(&format!("  store i64 {}, i64* %var_{}\n", val_reg, name));
                }
                Statement::Return(Some(expr)) => {
                    let (val_reg, code, r_count) = Self::emit_expr(expr, reg_counter);
                    reg_counter = r_count;
                    f_ir.push_str(&code);
                    f_ir.push_str(&format!("  ret {} {}\n", ret_type, val_reg));
                }
                Statement::Return(None) => {
                    f_ir.push_str("  ret void\n");
                }
                Statement::Expr(expr) => {
                    let (val_reg, code, r_count) = Self::emit_expr(expr, reg_counter);
                    reg_counter = r_count;
                    last_reg = val_reg;
                    f_ir.push_str(&code);
                }
            }
        }

        if func.name == "main" && ret_type == "i32" {
            f_ir.push_str("  ret i32 0\n");
        } else if ret_type == "void" {
            f_ir.push_str("  ret void\n");
        } else if !f_ir.ends_with("ret void\n") && !f_ir.contains("ret i64") && !f_ir.contains("ret i32") {
            f_ir.push_str(&format!("  ret {} 0\n", ret_type));
        }

        f_ir.push_str("}\n\n");
        f_ir
    }

    fn emit_expr(expr: &Expression, mut reg_counter: usize) -> (String, String, usize) {
        let mut code = String::new();
        match expr {
            Expression::Literal(Literal::Int(n)) => (format!("{}", n), code, reg_counter),
            Expression::Literal(Literal::Bool(b)) => (format!("{}", if *b { 1 } else { 0 }), code, reg_counter),
            Expression::Variable(name) => {
                let reg = format!("%r{}", reg_counter);
                reg_counter += 1;
                code.push_str(&format!("  {} = load i64, i64* %var_{}\n", reg, name));
                (reg, code, reg_counter)
            }
            Expression::Binary { op, left, right } => {
                let (l_reg, l_code, r1) = Self::emit_expr(left, reg_counter);
                let (r_reg, r_code, r2) = Self::emit_expr(right, r1);
                code.push_str(&l_code);
                code.push_str(&r_code);

                let res_reg = format!("%r{}", r2);
                reg_counter = r2 + 1;

                let op_str = match op {
                    BinOp::Add => "add i64",
                    BinOp::Sub => "sub i64",
                    BinOp::Mul => "mul i64",
                    BinOp::Div => "sdiv i64",
                    BinOp::Eq  => "icmp eq i64",
                    BinOp::Neq => "icmp ne i64",
                    BinOp::Lt  => "icmp slt i64",
                    BinOp::Lte => "icmp sle i64",
                    BinOp::Gt  => "icmp sgt i64",
                    BinOp::Gte => "icmp sge i64",
                    _ => "add i64",
                };

                code.push_str(&format!("  {} = {} {}, {}\n", res_reg, op_str, l_reg, r_reg));
                (res_reg, code, reg_counter)
            }
            Expression::Call { name, args, .. } => {
                if name == "println" {
                    let mut fmt_args = String::new();
                    for arg in args {
                        let (val_reg, a_code, r_next) = Self::emit_expr(arg, reg_counter);
                        reg_counter = r_next;
                        code.push_str(&a_code);
                        fmt_args.push_str(&format!(", i64 {}", val_reg));
                    }
                    let res_reg = format!("%r{}", reg_counter);
                    reg_counter += 1;
                    code.push_str(&format!("  {} = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str.fmt_int, i64 0, i64 0){})\n", res_reg, fmt_args));
                    (res_reg, code, reg_counter)
                } else {
                    let mut arg_str = String::new();
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            arg_str.push_str(", ");
                        }
                        let (val_reg, a_code, r_next) = Self::emit_expr(arg, reg_counter);
                        reg_counter = r_next;
                        code.push_str(&a_code);
                        arg_str.push_str(&format!("i64 {}", val_reg));
                    }
                    let res_reg = format!("%r{}", reg_counter);
                    reg_counter += 1;
                    code.push_str(&format!("  {} = call i64 @{}({})\n", res_reg, name, arg_str));
                    (res_reg, code, reg_counter)
                }
            }
            _ => ("0".to_string(), code, reg_counter),
        }
    }
}
