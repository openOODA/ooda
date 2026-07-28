//! openOODA stage-0 library (CLI + host FFI for native oodac).
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod eval;
pub mod diagnostics;
pub mod fmt;
pub mod outline;
pub mod capabilities;
pub mod typecheck;
pub mod codegen;
pub mod patch;
pub mod reflect;
pub mod bench;
pub mod pkg;
pub mod lsp;
pub mod context;
pub mod replay;
pub mod migrate;
pub mod codegen_wasm;
pub mod codegen_c;
pub mod dump;
pub mod loader;
pub mod em;
pub mod host_api;

// Re-export host API for tests and FFI.
pub use host_api::*;
