
fn replace_contracts(source: &str, layout: &FnLayout, patch: &AstPatch) -> Result<String> {
    // Region from contracts_start to body_open (exclusive of `{`).
    let start = layout.contracts_start;
    let end = layout.body_open;

    // Parse existing requires/ensures if only one side is being replaced.
    let existing = &source[start..end];
    let (mut reqs, mut ens) = split_contracts(existing);

    if let Some(r) = &patch.new_requires {
        reqs = r.trim().to_string();
    }
    if let Some(e) = &patch.new_ensures {
        ens = e.trim().to_string();
    }

    let mut block = String::new();
    if !reqs.is_empty() {
        for line in reqs.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with("requires") {
                block.push_str("    ");
                block.push_str(t);
                block.push('\n');
            } else {
                block.push_str("    requires ");
                block.push_str(t);
                block.push('\n');
            }
        }
    }
    if !ens.is_empty() {
        for line in ens.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with("ensures") {
                block.push_str("    ");
                block.push_str(t);
                block.push('\n');
            } else {
                block.push_str("    ensures ");
                block.push_str(t);
                block.push('\n');
            }
        }
    }

    let mut s = String::new();
    s.push_str(&source[..start]);
    if !block.is_empty() {
        // ensure newline before contracts if needed
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&block);
    } else if !s.ends_with(|c: char| c.is_whitespace()) {
        s.push(' ');
    }
    s.push_str(&source[end..]);
    Ok(s)
}

/// Split contract region into requires-blob and ensures-blob (may include keywords).
fn split_contracts(region: &str) -> (String, String) {
    let mut reqs = Vec::new();
    let mut ens = Vec::new();
    for line in region.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("requires") {
            reqs.push(t.to_string());
        } else if t.starts_with("ensures") {
            ens.push(t.to_string());
        }
    }
    (reqs.join("\n"), ens.join("\n"))
}

fn find_fn_layout(source: &str, func_name: &str) -> Result<FnLayout> {
    let fn_pat = format!("fn {}", func_name);
    let Some(fn_idx) = source.find(&fn_pat) else {
        return Err(anyhow!("Could not locate 'fn {}' in source text", func_name));
    };

    let bytes = source.as_bytes();
    let mut i = fn_idx + fn_pat.len();

    // Skip whitespace to `(`
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'(' {
        return Err(anyhow!(
            "Could not find param list '(' for function '{}'",
            func_name
        ));
    }
    let paren_open = i;

    // Match parens
    let mut depth = 0i32;
    let mut j = paren_open;
    while j < bytes.len() {
        match bytes[j] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        j += 1;
    }
    if j >= bytes.len() {
        return Err(anyhow!(
            "Unbalanced parens in parameter list for '{}'",
            func_name
        ));
    }
    let paren_close = j;

    // After `)`: optional `-> Type`, then requires/ensures, then `{`
    let mut k = paren_close + 1;
    while k < bytes.len() && bytes[k].is_ascii_whitespace() {
        k += 1;
    }

    let mut ret_end = k;

    if k + 1 < bytes.len() && &source[k..k + 2] == "->" {
        k += 2;
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        // Return type: until whitespace-separated requires/ensures/{ or end
        // Types may include `Result[Int, String]`, so track brackets.
        let type_start = k;
        let mut bracket = 0i32;
        while k < bytes.len() {
            let c = bytes[k] as char;
            if c == '[' {
                bracket += 1;
                k += 1;
                continue;
            }
            if c == ']' {
                bracket -= 1;
                k += 1;
                continue;
            }
            if bracket == 0 {
                // stop at newline before requires/ensures or at `{`
                if c == '{' {
                    break;
                }
                // check keyword at line starts / after space
                if c.is_whitespace() {
                    let rest = source[k..].trim_start();
                    if rest.starts_with("requires")
                        || rest.starts_with("ensures")
                        || rest.starts_with('{')
                    {
                        // consume only the whitespace that isn't the only separator...
                        // ret_end is start of trailing whitespace before contracts
                        break;
                    }
                    // allow space inside? OODA return types don't have spaces usually
                    // but Result[Int, String] has space after comma — continue
                    k += 1;
                    continue;
                }
            }
            k += 1;
        }
        ret_end = k;
        // If we stopped on whitespace before requires, ret_end is that whitespace start — good
        let _ = type_start;
    }

    // contracts_start: skip ws after ret_end
    let mut cstart = ret_end;
    while cstart < bytes.len() && bytes[cstart].is_ascii_whitespace() {
        cstart += 1;
    }

    // Find body `{` at paren depth 0 (no nested)
    let mut b = cstart;
    while b < bytes.len() {
        if bytes[b] == b'{' {
            break;
        }
        b += 1;
    }
    if b >= bytes.len() {
        return Err(anyhow!(
            "Could not find body '{{' for function '{}'",
            func_name
        ));
    }
    let body_open = b;

    // If no requires/ensures keywords before body, contracts_start == body_open
    let contracts_start = {
        let region = &source[cstart..body_open];
        if region.contains("requires") || region.contains("ensures") {
            cstart
        } else {
            body_open
        }
    };

    // Match braces for body
    let mut depth_b = 0i32;
    let mut close = body_open;
    while close < bytes.len() {
        match bytes[close] {
            b'{' => depth_b += 1,
            b'}' => {
                depth_b -= 1;
                if depth_b == 0 {
                    break;
                }
            }
            _ => {}
        }
        close += 1;
    }
    if close >= bytes.len() {
        return Err(anyhow!("Unbalanced braces in function '{}'", func_name));
    }

    Ok(FnLayout {
        paren_open,
        paren_close,
        ret_end,
        contracts_start,
        body_open,
        body_close: close,
    })
}
