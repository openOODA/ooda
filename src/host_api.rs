// ===================================================================
// Host API for oodac bootstrap (exact stage-0 dumps + real CHS native build).
// Used as interpreter builtins and as C FFI for native oodac.
// ===================================================================
use crate::capabilities::CapabilityChecker;
use crate::codegen_c::{runtime_c_path, CCodeGen};
use crate::dump::{format_ast_dump, format_check_err, format_check_ok, format_token_dump};
use crate::lexer::Lexer;
use crate::loader::load_program;
use crate::parser::Parser;
use crate::typecheck::TypeChecker;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::fs;

/// Canonical token dump for a source path (or ERR line).
pub fn host_token_dump_path(path: &Path) -> Result<String, String> {
    let src = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lexer = Lexer::new(&src);
    let tokens = lexer.tokenize().map_err(|e| e.to_string())?;
    Ok(format_token_dump(&tokens))
}

/// Canonical AST dump for a source path.
pub fn host_ast_dump_path(path: &Path) -> Result<String, String> {
    let src = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lexer = Lexer::new(&src);
    let tokens = lexer.tokenize().map_err(|e| format!("lex: {}", e))?;
    let mut parser = Parser::new(tokens);
    let prog = parser.parse_program().map_err(|e| format!("parse: {}", e))?;
    Ok(format_ast_dump(&prog))
}

/// Canonical check status: "OK\n" or "ERR\tkind\tmsg\n"
pub fn host_check_path(path: &Path) -> String {
    match load_program(path) {
        Err(e) => format_check_err("load", &format!("{}", e)),
        Ok(prog) => {
            if let Err(e) = CapabilityChecker::check_program(&prog) {
                return format_check_err("capability", &format!("{}", e));
            }
            if let Err(e) = TypeChecker::check_program(&prog) {
                return format_check_err("type", &format!("{}", e));
            }
            format_check_ok()
        }
    }
}

/// Real CHS native build: load → type/cap check → C emit → gcc link.
pub fn host_chs_build(src: &Path, out_bin: &Path) -> Result<(), String> {
    let prog = load_program(src).map_err(|e| format!("load: {}", e))?;
    CapabilityChecker::check_program(&prog).map_err(|e| format!("cap: {}", e))?;
    TypeChecker::check_program(&prog).map_err(|e| format!("type: {}", e))?;
    let rt = runtime_c_path();
    CCodeGen::build_native(&prog, out_bin, &rt).map_err(|e| format!("build: {}", e))
}

// ----- C FFI (linked into native oodac via -looda) -----

/// Returns heap C string (caller frees with ooda_host_free). NULL on OOM.
#[no_mangle]
pub unsafe extern "C" fn ooda_host_ast_dump(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let path = match CStr::from_ptr(path).to_str() {
        Ok(s) => Path::new(s),
        Err(_) => return cstring_or_null("ERR\tast\tbad_path\n"),
    };
    match host_ast_dump_path(path) {
        Ok(s) => cstring_or_null(&s),
        Err(e) => cstring_or_null(&format_check_err("ast", &e)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ooda_host_check(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return cstring_or_null("ERR\tcheck\tnull\n");
    }
    let path = match CStr::from_ptr(path).to_str() {
        Ok(s) => Path::new(s),
        Err(_) => return cstring_or_null("ERR\tcheck\tbad_path\n"),
    };
    cstring_or_null(&host_check_path(path))
}

#[no_mangle]
pub unsafe extern "C" fn ooda_host_token_dump(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return cstring_or_null("ERR\ttokens\tnull\n");
    }
    let path = match CStr::from_ptr(path).to_str() {
        Ok(s) => Path::new(s),
        Err(_) => return cstring_or_null("ERR\ttokens\tbad_path\n"),
    };
    match host_token_dump_path(path) {
        Ok(s) => cstring_or_null(&s),
        Err(e) => cstring_or_null(&format_check_err("tokens", &e)),
    }
}

/// 0 = success, non-zero = failure. Writes diagnostic to stderr.
#[no_mangle]
pub unsafe extern "C" fn ooda_host_chs_build(src: *const c_char, out_bin: *const c_char) -> i32 {
    if src.is_null() || out_bin.is_null() {
        eprintln!("ooda_host_chs_build: null arg");
        return 1;
    }
    let src = match CStr::from_ptr(src).to_str() {
        Ok(s) => Path::new(s),
        Err(_) => {
            eprintln!("ooda_host_chs_build: bad src path");
            return 2;
        }
    };
    let out = match CStr::from_ptr(out_bin).to_str() {
        Ok(s) => Path::new(s),
        Err(_) => {
            eprintln!("ooda_host_chs_build: bad out path");
            return 3;
        }
    };
    match host_chs_build(src, out) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("ooda_host_chs_build: {}", e);
            4
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ooda_host_free(p: *mut c_char) {
    if !p.is_null() {
        drop(CString::from_raw(p));
    }
}

fn cstring_or_null(s: &str) -> *mut c_char {
    match CString::new(s.replace('\0', "")) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn host_check_accepts_int_main() {
        let p = PathBuf::from("fixtures/int_main.oo");
        if !p.exists() {
            return;
        }
        let s = host_check_path(&p);
        assert!(s.starts_with("OK"), "{}", s);
    }

    #[test]
    fn host_check_rejects_no_cap_fetch() {
        let p = PathBuf::from("bootstrap/corpus/check/fail/no_cap_fetch.oo");
        if !p.exists() {
            return;
        }
        let s = host_check_path(&p);
        assert!(s.starts_with("ERR"), "{}", s);
        assert!(s.contains("capability") || s.contains("Capability") || s.contains("Security"), "{}", s);
    }

    #[test]
    fn host_ast_has_program() {
        let p = PathBuf::from("fixtures/int_main.oo");
        if !p.exists() {
            return;
        }
        let s = host_ast_dump_path(&p).expect("ast");
        assert!(s.starts_with("PROGRAM"), "{}", s);
        assert!(s.contains("FN name="), "{}", s);
    }

    #[test]
    fn host_chs_build_smoke() {
        let src = PathBuf::from("fixtures/chs_list_string.oo");
        if !src.exists() {
            return;
        }
        let out = PathBuf::from(format!(
            "{}/chs_build_test_bin",
            std::env::var("TMPDIR").unwrap_or_else(|_| "/var/tmp".into())
        ));
        let _ = std::fs::remove_file(&out);
        host_chs_build(&src, &out).expect("build");
        assert!(out.exists());
        let status = std::process::Command::new(&out).output().expect("run");
        assert!(status.status.success());
        let _ = std::fs::remove_file(&out);
    }
}
