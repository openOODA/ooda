// job: type checker
// stage: typecheck
include!("ty.rs");
include!("int_refinement.rs");
include!("unify.rs");
include!("check_program.rs");
include!("check_function.rs");
include!("check_block.rs");
include!("check_stmt_let_assign.rs");
include!("check_stmt_control.rs");
include!("infer_expr.rs");
include!("infer_if_match.rs");
include!("infer_call.rs");
include!("infer_method.rs");
include!("method_return.rs");
include!("call_specials_list.rs");
include!("call_specials_str.rs");
include!("control_flow_util.rs");
include!("tests_mod.rs");
