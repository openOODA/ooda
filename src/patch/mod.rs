// job: surgical AST/text patcher for named functions
// in:  file path + JSON patch
// out: validated rewritten source or error
// stage: host
include!("core.rs");
include!("contracts_layout.rs");
include!("write.rs");
include!("tests.rs");
