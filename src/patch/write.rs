
fn replace_function_body(source: &str, func_name: &str, new_body: &str) -> Result<String> {
    let layout = find_fn_layout(source, func_name)?;
    let open = layout.body_open;
    let j = layout.body_close;

    let mut out = String::new();
    out.push_str(&source[..open + 1]);
    out.push('\n');
    for line in new_body.lines() {
        if line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str("    ");
            out.push_str(line.trim());
            out.push('\n');
        }
    }
    out.push_str(&source[j..]);
    Ok(out)
}

fn validate_and_write(file_path: &Path, new_code: &str, target: &str) -> Result<()> {
    let mut check_lexer = Lexer::new(new_code);
    let check_tokens = check_lexer.tokenize().map_err(|e| {
        anyhow!(
            "Patch validation error: syntax error in patched source: {}",
            e
        )
    })?;
    let mut check_parser = Parser::new(check_tokens);
    let check_program = check_parser.parse_program().map_err(|e| {
        anyhow!(
            "Patch validation error: AST parse error in patched source: {}",
            e
        )
    })?;

    crate::capabilities::CapabilityChecker::check_program(&check_program).map_err(|e| {
        anyhow!(
            "Patch validation error: capability violation in patched source: {}",
            e
        )
    })?;
    crate::typecheck::TypeChecker::check_program(&check_program).map_err(|e| {
        anyhow!(
            "Patch validation error: type error in patched source: {}",
            e
        )
    })?;

    fs::write(file_path, new_code)?;
    // Success message is printed by the CLI (so `patch --json` can emit pure JSON).
    let _ = target;
    Ok(())
}
