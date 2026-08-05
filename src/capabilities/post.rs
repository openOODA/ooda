#[cfg(test)]
mod tests {
    use super::*;
    fn parse_program(src: &str) -> Program {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = crate::parser::Parser::new(tokens);
        parser.parse_program().expect("parse")
    }
    #[test]
    fn denies_fetch_without_netcap() {
        let prog = parse_program(
            r#"
            pub fn rogue() {
                let res = fetch("https://evil.example");
            }
        "#,
        );
        assert!(CapabilityChecker::check_program(&prog).is_err());
    }
    #[test]
    fn method_forms_are_sealed_for_dual_engine() {
        let prog = parse_program(
            r#"
            pub fn fs_m(fs: &FsCap) {
                let _ = fs.path_exists("/tmp");
                let _ = fs.file_size("/tmp/x");
                let _ = fs.mkdir_p("/tmp/ooda_test_dir");
            }
            pub fn sys_m(sys: &SysCap) {
                let _ = sys.sys_exec("true");
            }
            pub fn env_m(env: &EnvCap) {
                let _ = env.env_get("PATH");
            }
        "#,
        );
        let sealed = collect_sealed_effect_names(&prog);
        for need in [
            ".path_exists",
            ".file_size",
            ".sys_exec",
            ".env_get",
            ".mkdir_p",
        ] {
            assert!(
                sealed.iter().any(|s| s == need),
                "missing sealed method {} in {:?}",
                need,
                sealed
            );
        }
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "with live receivers, method forms must typecheck caps: {:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }
    #[test]
    fn denies_path_exists_method_without_fscap_receiver() {
        let prog = parse_program(
            r#"
            pub fn rogue() {
                let b = path_exists("/tmp");
            }
        "#,
        );
        assert!(CapabilityChecker::check_program(&prog).is_err());
        let prog2 = parse_program(
            r#"
            pub fn rogue(net: &NetCap) {
                let b = net.path_exists("/tmp");
            }
        "#,
        );
        let err = CapabilityChecker::check_program(&prog2).unwrap_err().to_string();
        assert!(
            err.contains("wrong-kind")
                || err.contains("not a")
                || err.contains("FsCap")
                || err.contains("capability"),
            "wrong receiver kind: {}",
            err
        );
    }
    #[test]
    fn allows_fetch_with_netcap() {
        let prog = parse_program(
            r#"
            pub fn ok(net: &NetCap, url: String) {
                let res = fetch(net, url);
            }
        "#,
        );
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "{:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }
    #[test]
    fn denies_ambient_only_fetch_without_handle_arg() {
        let prog = parse_program(
            r#"
            pub fn ambient(net: &NetCap, url: String) {
                let res = fetch(url);
            }
        "#,
        );
        let err = CapabilityChecker::check_program(&prog).unwrap_err().to_string();
        assert!(
            err.contains("object-capability") || err.contains("live"),
            "ambient-only fetch must fail: {}",
            err
        );
    }
    #[test]
    fn denies_wrong_kind_handle_for_write_file() {
        let prog = parse_program(
            r#"
            pub fn mix(net: &NetCap, fs: &FsCap) {
                let r = write_file(net, "/tmp/x", "y");
            }
        "#,
        );
        let err = CapabilityChecker::check_program(&prog).unwrap_err().to_string();
        assert!(
            err.contains("wrong-kind")
                || err.contains("object-capability")
                || err.contains("live")
                || err.contains("FsCap")
                || err.contains("write_file"),
            "wrong-kind handle must fail: {}",
            err
        );
        assert!(
            err.contains("wrong-kind") && err.contains("NetCap") && err.contains("FsCap"),
            "must name both kinds: {}",
            err
        );
    }
    #[test]
    fn unknown_name_is_not_ambient_io() {
        let prog = parse_program(
            r#"
            pub fn steal() {
                let x = network_read("https://evil.com");
            }
        "#,
        );
        assert!(CapabilityChecker::check_program(&prog).is_ok());
        assert!(lookup_effect("network_read").is_none());
    }
    #[test]
    fn method_write_file_requires_fscap() {
        let prog = parse_program(
            r#"
            pub fn bad(msg: String) {
                fs.write_file("app.log", msg);
            }
        "#,
        );
        assert!(CapabilityChecker::check_program(&prog).is_err());
    }
    #[test]
    fn method_write_file_with_fscap_ok() {
        let prog = parse_program(
            r#"
            pub fn log_event(fs: &FsCap, message: String) {
                fs.write_file("app.log", message);
            }
        "#,
        );
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "receiver fs param must be accepted: {:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }
    #[test]
    fn allows_assign_re_aliased_capability_handle() {
        let prog = parse_program(
            r#"
            pub fn main(fs: &FsCap) {
                let mut fs_var = fs;
                fs_var = fs;
                fs_var.write_file("note.txt", "hello");
            }
            "#,
        );
        assert!(CapabilityChecker::check_program(&prog).is_ok());
    }
    #[test]
    fn allows_nested_if_let_aliased_capability_handle() {
        let prog = parse_program(
            r#"
            pub fn main(fs: &FsCap) {
                if true {
                    let fs2 = fs;
                    fs2.write_file("note.txt", "hello");
                }
            }
            "#,
        );
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "nested let-alias of FsCap must be accepted: {:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }
    #[test]
    fn allows_match_some_pattern_capability_handle() {
        let prog = parse_program(
            r#"
            pub fn main(fs: &FsCap) {
                match Some(fs) {
                    Some(h) => h.write_file("note.txt", "hello"),
                    None => process_exit(1),
                }
            }
            "#,
        );
        assert!(
            CapabilityChecker::check_program(&prog).is_ok(),
            "match Some(cap) pattern bind must be a handle: {:?}",
            CapabilityChecker::check_program(&prog).err()
        );
    }
    #[test]
    fn wrong_kind_write_file_net_only_names_kinds() {
        let src = r#"
            pub fn main(net: &NetCap) {
                let r = write_file(net, "/tmp/x", "y");
                println(r);
            }
        "#;
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = crate::parser::Parser::new(tokens);
        let prog = parser.parse_program().expect("parse");
        let err = CapabilityChecker::check_program(&prog).unwrap_err().to_string();
        assert!(
            err.contains("wrong-kind") && err.contains("NetCap") && err.contains("FsCap"),
            "write_file(net) without FsCap must wrong-kind: {}",
            err
        );
    }
}
