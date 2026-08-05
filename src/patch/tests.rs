
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_dir(label: &str) -> std::path::PathBuf {
        let base = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let dir = base.join(".cache").join(format!(
            "ooda-patch-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn patch_replaces_body() {
        let dir = test_dir("body");
        let path = dir.join("body.oo");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return a + b;\n}}\n"
        )
        .unwrap();
        apply_patch(
            &path,
            r#"{"target_function":"add","new_body":"return a * b;"}"#,
        )
        .unwrap();
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("return a * b;"), "got: {}", got);
        assert!(!got.contains("return a + b;"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn patch_replaces_return_type() {
        let dir = test_dir("ret");
        let path = dir.join("ret.oo");
        let mut f = fs::File::create(&path).unwrap();
        // Body must match new Float return type for validation to pass.
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return 1.0;\n}}\n"
        )
        .unwrap();
        apply_patch(
            &path,
            r#"{"target_function":"add","new_return_type":"Float"}"#,
        )
        .unwrap();
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("-> Float"), "got: {}", got);
        assert!(!got.contains("-> Int"), "got: {}", got);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn patch_replaces_parameter_list() {
        let dir = test_dir("params");
        let path = dir.join("params.oo");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return 1;\n}}\n"
        )
        .unwrap();
        apply_patch(
            &path,
            r#"{"target_function":"add","new_params":"x: Int, y: Int","new_body":"return x + y;"}"#,
        )
        .unwrap();
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("fn add(x: Int, y: Int)"), "got: {}", got);
        assert!(got.contains("return x + y;"), "got: {}", got);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn patch_sets_requires() {
        let dir = test_dir("req");
        let path = dir.join("req.oo");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return a + b;\n}}\n"
        )
        .unwrap();
        apply_patch(
            &path,
            r#"{"target_function":"add","new_requires":"requires a >= 0"}"#,
        )
        .unwrap();
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("requires a >= 0"), "got: {}", got);
        // still typechecks / runs structurally
        assert!(got.contains("return a + b;"), "got: {}", got);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn patch_return_type_rejects_inconsistent_body() {
        let dir = test_dir("bad");
        let path = dir.join("bad.oo");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return a + b;\n}}\n"
        )
        .unwrap();
        let err = apply_patch(
            &path,
            r#"{"target_function":"add","new_return_type":"Float"}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("type error") || err.contains("Type error"),
            "expected type validation fail, got: {}",
            err
        );
        // file unchanged
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("-> Int"));
        let _ = fs::remove_dir_all(&dir);
    }
}
