// openOODA CLI binary — logic modules live in the `ooda` library.
use ooda::ast::Program;
use ooda::bench;
use ooda::capabilities::CapabilityChecker;
use ooda::codegen::LlvmCodeGen;
use ooda::codegen_c::{runtime_c_path, CCodeGen};
use ooda::codegen_wasm::WasmCodeGen;
use ooda::diagnostics::{parse_loc, AiDiagnostic};
use ooda::dump::{format_ast_dump, format_check_err, format_check_ok, format_token_dump};
use ooda::eval::Interpreter;
use ooda::fmt;
use ooda::lexer::Lexer;
use ooda::loader::load_program;
use ooda::lsp;
use ooda::migrate;
use ooda::outline;
use ooda::parser::Parser;
use ooda::patch;
use ooda::pkg;
use ooda::reflect;
use ooda::replay;
use ooda::typecheck::TypeChecker;
use ooda::context::ContextEngine;

use clap::{Parser as ClapParser, Subcommand};
use std::path::PathBuf;
use std::fs;
use std::time::Instant;
use anyhow::{Context, Result};

