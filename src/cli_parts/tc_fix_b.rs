/// Second half of typecheck error → fix suggestion map.
fn tc_fix_part_b(msg: &str, line: usize) -> (String, String, bool) {
if msg.contains("out of bounds")
    && (msg.contains("char_at") || msg.contains("str_slice"))
{
    (
        "Fix const string index / slice bounds".into(),
        r#"{"codemod":"str_bounds","hint":"use an index in 0..chars_len(s) or a valid [start..end] slice"}"#.into(),
        true,
    )
} else if msg.contains("RefinementTypeViolation") {
    // Int[lo..hi] on let / return / call-site arg
    let val = msg
        .split("value ")
        .nth(1)
        .and_then(|s| {
            s.split(' ')
                .next()
                .map(|t| t.trim_end_matches(',').to_string())
        })
        .unwrap_or_else(|| "?".into());
    let bounds = msg
        .split("bounds [")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .unwrap_or("lo..hi");
    (
        "Fix refinement bounds".into(),
        format!(
            "{{\"codemod\":\"refinement_bounds\",\"value\":\"{}\",\"bounds\":\"[{}]\",\"hint\":\"pass/return/assign a value inside Int[{}]\"}}",
            val, bounds, bounds
        ),
        true,
    )
} else if msg.contains("cannot assign") && msg.contains("to '") {
    // cannot assign String to 'x' of type Int
    let vname = msg
        .split("to '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or("x");
    let found = msg
        .split("cannot assign ")
        .nth(1)
        .and_then(|s| s.split(' ').next())
        .unwrap_or("?");
    let expected = msg
        .split("of type ")
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "?".into());
    (
        "Fix assignment types".into(),
        format!(
            "{{\"codemod\":\"assign_type\",\"binding\":\"{}\",\"expected\":\"{}\",\"found\":\"{}\",\"hint\":\"assign a {} value or change the binding's type\"}}",
            vname, expected, found, expected
        ),
        true,
    )
} else if msg.contains("list element type mismatch") {
    let expected = msg
        .split("List[")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .unwrap_or("?");
    let found = msg
        .split("cannot push ")
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "?".into());
    (
        "Fix list element type".into(),
        format!(
            "{{\"codemod\":\"list_elem\",\"expected\":\"{}\",\"found\":\"{}\",\"hint\":\"push only {} values or start a new list\"}}",
            expected, found, expected
        ),
        true,
    )
} else if msg.contains("matching numeric types")
    || (msg.contains("arithmetic") && msg.contains("found"))
{
    (
        "Annotate list/element types before arithmetic".into(),
        r#"{"codemod":"arith_types","hint":"use List[Int]/List[String] annotations or push homogeneous elements before `for` so element type is not `_`"}"#.into(),
        true,
    )
} else if msg.contains("return type") && msg.contains("does not match declared") {
    // return type String does not match declared Int — in 'f'
    let fname = msg
        .split("in '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or("f");
    let found = msg
        .split("return type ")
        .nth(1)
        .and_then(|s| s.split(' ').next())
        .unwrap_or("?");
    let expected = msg
        .split("declared ")
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "?".into());
    (
        "Align return type and body".into(),
        format!(
            "{{\"codemod\":\"return_type\",\"target_function\":\"{}\",\"declared\":\"{}\",\"found\":\"{}\",\"hint\":\"change body to return {} or patch new_return_type\"}}",
            fname, expected, found, expected
        ),
        true,
    )
} else if msg.contains("argument ") && msg.contains("expects ") && msg.contains("found ") {
    // Arg type: function 'f' argument N expects T, found U
    let fname = msg
        .split("function '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or("f");
    let arg_index = msg
        .split("argument ")
        .nth(1)
        .and_then(|s| s.split(' ').next())
        .unwrap_or("0");
    let expected = msg
        .split("expects ")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .unwrap_or("?")
        .trim()
        .to_string();
    let found = msg
        .split("found ")
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "?".into());
    (
        "Fix call argument type".into(),
        format!(
            "{{\"codemod\":\"arg_type\",\"function\":\"{}\",\"arg_index\":{},\"expected\":\"{}\",\"found\":\"{}\",\"hint\":\"pass a value of the expected type or change the callee param\"}}",
            fname, arg_index, expected, found
        ),
        true,
    )
} else if msg.contains("unknown method") {
    let mname = msg
        .split("unknown method '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or(".method");
    let on_ty = msg
        .split(" on ")
        .nth(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Type".into());
    (
        "Fix method / field access".into(),
        format!(
            "{{\"codemod\":\"unknown_method\",\"method\":\"{}\",\"receiver\":\"{}\",\"hint\":\"use a real method (.len/.char_at/.push/…) or a struct field; free-form builtins use name(recv, …)\"}}",
            mname, on_ty
        ),
        true,
    )
} else if msg.contains("undefined variable") {
    let vname = msg
        .split("undefined variable '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or("x");
    (
        "Define or bind variable".into(),
        format!(
            "{{\"codemod\":\"undefined_var\",\"name\":\"{}\",\"hint\":\"add `let {} = …` before use, or fix the name\"}}",
            vname, vname
        ),
        true,
    )
} else if msg.contains("`?` only allowed") || msg.contains("`?` requires Result") {
    (
        "Fix try-operator usage".into(),
        r#"{"codemod":"try_op","hint":"`?` only on Result values inside functions that return Result[T,E] with matching E"}"#.into(),
        true,
    )
} else {
    (
        "Fix types".into(),
        "Ensure operands and annotations agree (Int/Float/String/Bool/caps).".into(),
        false,
    )
}
}

fn typecheck_fix_suggestion(msg: &str, line: usize) -> (String, String, bool) {
    if let Some(x) = tc_fix_part_a(msg, line) {
        return x;
    }
    tc_fix_part_b(msg, line)
}

