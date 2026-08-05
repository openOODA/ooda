
pub fn runtime_c_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        PathBuf::from("runtime/chs_rt.c"),
        manifest.join("runtime/chs_rt.c"),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    PathBuf::from("runtime/chs_rt.c")
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(src: &str) -> Program {
        let mut l = Lexer::new(src);
        let t = l.tokenize().unwrap();
        Parser::new(t).parse_program().unwrap()
    }

    #[test]
    fn emits_c_for_int_main() {
        let p = parse("pub fn main() { println(42); }\n");
        let c = CCodeGen::emit_c(&p).expect("emit");
        assert!(c.contains("int main"), "{}", c);
        assert!(c.contains("oo_print_int"), "{}", c);
    }

    #[test]
    fn emits_c_for_list_and_string() {
        let p = parse(
            r#"
            pub fn main() {
                let mut xs = list_new();
                xs = list_push(xs, 10);
                println(list_len(xs));
                println(chars_len("ab"));
            }
            "#,
        );
        let c = CCodeGen::emit_c(&p).expect("emit");
        assert!(c.contains("oo_ilist_new") || c.contains("oo_ilist_push"), "{}", c);
        assert!(c.contains("oo_chars_len"), "{}", c);
    }

    #[test]
    fn emits_c_for_string_list_pending() {
        let p = parse(
            r#"
            pub fn main() {
                let mut xs = list_new();
                xs = list_push(xs, "a");
                xs = list_push(xs, "b");
                println(list_len(xs));
            }
            "#,
        );
        let c = CCodeGen::emit_c(&p).expect("emit");
        let main = c.split("int main").nth(1).unwrap_or(&c);
        assert!(
            main.contains("oo_slist_new") && main.contains("oo_slist_push"),
            "string list body must use slist: {}",
            main
        );
        assert!(
            !main.contains("oo_ilist_push") && !main.contains("OoIList xs"),
            "must not use int list for string elements: {}",
            main
        );
    }

    #[test]
    fn pure_chs_emit_omits_host_ffi_decls() {
        let p = parse(
            r#"
            pub fn main() {
                let mut xs = list_new();
                xs = list_push(xs, 1);
                println(list_len(xs));
            }
            "#,
        );
        assert!(
            !super::program_needs_host_ffi(&p),
            "pure list program must not need host FFI"
        );
        let c = CCodeGen::emit_c(&p).expect("emit");
        assert!(
            !c.contains("ooda_host") && !c.contains("oo_chs_build"),
            "pure emit must not declare host FFI (assembly depth): {}",
            c.lines().take(40).collect::<Vec<_>>().join("\n")
        );
    }

    #[test]
    fn chs_build_call_needs_host_ffi() {
        let p = parse(
            r#"
            pub fn main() {
                let r = chs_build("a.oo", "a.bin");
            }
            "#,
        );
        assert!(
            super::program_needs_host_ffi(&p),
            "chs_build must require host FFI / libooda"
        );
        let c = CCodeGen::emit_c(&p).expect("emit");
        assert!(
            c.contains("oo_chs_build"),
            "host-using emit must declare oo_chs_build"
        );
    }

    #[test]
    fn pure_chs_build_native_without_staticlib() {
        // Integration: pure program links with gcc+chs_rt only (no libooda.a required).
        let p = parse(
            r#"
            pub fn main() {
                println(1 + 2);
            }
            "#,
        );
        let rt = super::runtime_c_path();
        let out = std::env::temp_dir().join(format!("ooda_pure_chs_{}", std::process::id()));
        let _ = std::fs::remove_file(&out);
        CCodeGen::build_native(&p, &out, &rt, false).expect("pure build_native");
        assert!(out.exists(), "binary missing");
        let status = std::process::Command::new(&out).output().expect("run");
        assert!(status.status.success(), "pure binary failed");
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(out.with_extension("c"));
    }

    #[test]
    fn sealed_fs_is_lowered_on_c_net_is_not() {
        let fs = parse(
            r#"
            pub fn main(fs: &FsCap) {
                let r = read_file(fs, "x.oo");
                if r.is_ok() {
                    println("ok");
                }
            }
            "#,
        );
        assert!(
            super::sealed_effects_not_lowered_on_c(&fs).is_empty(),
            "read_file must be C-lowered"
        );
        assert!(super::c_backend_lowers_sealed("read_file"));
        assert!(super::c_backend_lowers_sealed(".path_exists"));
        assert!(!super::c_backend_lowers_sealed("fetch"));
        assert!(!super::c_backend_lowers_sealed("mkdir_p"));
        assert!(!super::c_backend_lowers_sealed(".mkdir_p"));

        let net = parse(
            r#"
            pub fn main(net: &NetCap) {
                let r = fetch(net, "http://example.com");
                println("x");
            }
            "#,
        );
        let bad = super::sealed_effects_not_lowered_on_c(&net);
        assert!(
            bad.iter().any(|s| s == "fetch"),
            "fetch must not be C-lowered: {:?}",
            bad
        );
    }

    #[test]
    fn fscap_program_builds_native_with_read_file() {
        // Bootstrap path: oodac-style FS I/O must link (no silent refuse).
        let p = parse(
            r#"
            pub fn main(fs: &FsCap) {
                let r = read_file(fs, "/etc/hosts");
                if r.is_ok() {
                    println("ok");
                }
            }
            "#,
        );
        let rt = super::runtime_c_path();
        let out = std::env::temp_dir().join(format!("ooda_fs_chs_{}", std::process::id()));
        let _ = std::fs::remove_file(&out);
        CCodeGen::build_native(&p, &out, &rt, false).expect("fs build_native");
        assert!(out.exists());
        let status = std::process::Command::new(&out).output().expect("run");
        assert!(status.status.success(), "fs binary failed");
        let stdout = String::from_utf8_lossy(&status.stdout);
        assert!(stdout.contains("ok"), "stdout={}", stdout);
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(out.with_extension("c"));
    }
}

