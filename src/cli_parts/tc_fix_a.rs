/// First half of typecheck error → fix suggestion map.
fn tc_fix_part_a(msg: &str, line: usize) -> Option<(String, String, bool)> {
if msg.contains("must-use")
    || msg.contains("unused Result")
{
    Some((
        "Handle Result/Option with match".into(),
        r#"{"target_function":"<fn>","new_body":"let r = <expr>;\nmatch r {\n  Ok(v) => { /* use v */ },\n  Err(e) => { /* handle e */ }\n}"}"#.into(),
        true,
    ))
} else if msg.contains("non-exhaustive match") {
    Some((
        "Cover all match variants".into(),
        r#"{"target_function":"<fn>","new_body":"match r {\n  Ok(v) => …,\n  Err(e) => …\n  // or add `_ => process_exit(1)` then replace\n}"}"#.into(),
        true,
    ))
} else if msg.contains("immutable") || msg.contains("let mut") {
    let vname = msg
        .split('\'')
        .nth(1)
        .unwrap_or("x");
    Some((
        "Use let mut for assigned binding".into(),
        format!(
            "{{\"codemod\":\"let_mut\",\"binding\":\"{}\",\"hint\":\"ooda migrate --edition 2026 rewrites assigned immutable let → let mut\"}}",
            vname
        ),
        true,
    ))
} else if msg.contains("missing return") {
    // Message shape: "declares return type {T} but body has type Void (missing return value)"
    let ret_ty = msg
        .split("declares return type ")
        .nth(1)
        .and_then(|s| s.split(" but body").next())
        .unwrap_or("Int")
        .trim();
    let stub = match ret_ty {
        "Int" | "Float" => "return 0;",
        "Bool" => "return false;",
        "String" => "return \"\";",
        "Void" => "return;",
        t if t.starts_with("Option") => "return None;",
        t if t.starts_with("Result") => {
            "return Err(\"TODO: missing return\");"
        }
        _ => "return 0; /* TODO: match declared return type */",
    };
    Some((
        format!("Add return value on every path (declared {})", ret_ty),
        format!(
            r#"{{"codemod":"missing_return","declared_return":"{}","target_line":{},"new_code":"{}"}}"#,
            ret_ty.replace('"', "\\\""),
            line,
            stub.replace('"', "\\\"")
        ),
        true,
    ))
} else if msg.contains("unreachable code after return") {
    Some((
        "Remove dead code after return".into(),
        r#"{"hint":"delete statements after `return` — they never execute"}"#.into(),
        true,
    ))
} else if msg.contains("division by zero") {
    Some((
        "Fix zero divisor".into(),
        r#"{"hint":"const divisor is 0 — change the literal or guard the division"}"#.into(),
        true,
    ))
} else if msg.contains("undefined function") {
    let fname = msg
        .split("undefined function '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or("name");
    Some((
        "Define or import function".into(),
        format!(
            "{{\"target_function\":\"{}\",\"new_body\":\"// implement {}\\nreturn 0;\"}}",
            fname, fname
        ),
        true,
    ))
} else if msg.contains("argument(s), found") {
    // Arity: function 'f' expects N argument(s), found M
    let fname = msg
        .split("function '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or("f");
    let expected = msg
        .split("expects ")
        .nth(1)
        .and_then(|s| s.split(' ').next())
        .unwrap_or("?");
    let found = msg
        .split("found ")
        .nth(1)
        .map(|s| {
            s.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "?".into());
    Some((
        "Fix call argument count".into(),
        format!(
            "{{\"codemod\":\"arg_count\",\"function\":\"{}\",\"expected_arity\":{},\"found_arity\":{},\"hint\":\"supply exactly the declared parameters (or change the callee signature)\"}}",
            fname, expected, found
        ),
        true,
    ))
} else if msg.contains("cannot concatenate") || msg.contains("convert with .to_string()")
{
    Some((
        "Convert non-String operand before concat".into(),
        r#"{"codemod":"str_concat","hint":"use left + right.to_string() (or both String) for concatenation"}"#.into(),
        true,
    ))
} else if msg.contains("assert_eq arguments must have matching types") {
    let found = msg
        .split("found ")
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "?".into());
    Some((
        "Fix assert_eq operand types".into(),
        format!(
            "{{\"codemod\":\"assert_eq_types\",\"found\":\"{}\",\"hint\":\"assert_eq requires identical static types on both sides\"}}",
            found
        ),
        true,
    ))
} else {
    None
}
}

