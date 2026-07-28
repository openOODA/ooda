; ===================================================================
; openOODA LLVM IR Target Code Generator Output
; Target Architecture: x86_64 / ARM64 Native Bare-Metal
; ===================================================================

target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

declare i32 @printf(i8*, ...)
@.str.hello = private unnamed_addr constant [16 x i8] c"Hello from OODA\0A\00", align 1

define void @greet(i8* %name) #0 {
entry:
  ret void
}

define void @main() #0 {
entry:
  %1 = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([16 x i8], [16 x i8]* @.str.hello, i64 0, i64 0))
  ret void
}

