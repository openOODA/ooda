; ===================================================================
; openOODA LLVM IR Target Code Generator Output
; Target Architecture: x86_64 / ARM64 Native Bare-Metal
; ===================================================================

target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

declare i32 @printf(i8*, ...)
@.str.fmt_int = private unnamed_addr constant [5 x i8] c"%ld\0A\00", align 1
@.str.fmt_str = private unnamed_addr constant [4 x i8] c"%s\0A\00", align 1

define i32 @greet(i8* %arg_name) #0 {
entry:
  %var_name = alloca i8*
  store i8* %arg_name, i8** %var_name
  %r1 = load i64, i64* %var_name
  %r2 = add i64 0, %r1
  %r3 = add i64 %r2, 0
  ret i32 %r3
}

define i32 @main() #0 {
entry:
  %var_message = alloca i64
  %r1 = call i64 @greet(i64 0)
  store i64 %r1, i64* %var_message
  %r2 = load i64, i64* %var_message
  %r3 = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str.fmt_int, i64 0, i64 0), i64 %r2)
  ret i32 0
}

attributes #0 = { nounwind }
