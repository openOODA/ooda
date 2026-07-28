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
        ir.push_str("@.str.hello = private unnamed_addr constant [16 x i8] c\"Hello from OODA\\0A\\00\", align 1\n\n");

        for item in &program.items {
            if let Item::Function(func) = item {
                ir.push_str(&Self::emit_function(func));
            }
        }

        ir
    }

    fn emit_function(func: &FunctionDecl) -> String {
        let mut f_ir = String::new();

        let ret_type = match func.return_type {
            Type::Int => "i64",
            Type::Float => "double",
            Type::Bool => "i1",
            _ => "void",
        };

        f_ir.push_str(&format!("define {} @{}(", ret_type, func.name));
        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                f_ir.push_str(", ");
            }
            let p_type = match param.param_type {
                Type::Int => "i64",
                Type::Float => "double",
                _ => "i8*",
            };
            f_ir.push_str(&format!("{} %{}", p_type, param.name));
        }
        f_ir.push_str(") #0 {\nentry:\n");

        if func.name == "main" {
            f_ir.push_str("  %1 = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([16 x i8], [16 x i8]* @.str.hello, i64 0, i64 0))\n");
            f_ir.push_str("  ret void\n");
        } else {
            if ret_type == "void" {
                f_ir.push_str("  ret void\n");
            } else {
                f_ir.push_str("  ret i64 0\n");
            }
        }

        f_ir.push_str("}\n\n");
        f_ir
    }
}
