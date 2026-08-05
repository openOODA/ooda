
/// End (exclusive) of a simple token starting at `start` byte: string, number, or ident.
fn scan_token_end(source: &str, start: usize) -> Option<usize> {
    let b = source.as_bytes();
    if start >= b.len() {
        return None;
    }
    let mut i = start;
    // String literal "..."
    if b[i] == b'"' {
        i += 1;
        while i < b.len() {
            if b[i] == b'\\' {
                i = (i + 2).min(b.len());
                continue;
            }
            if b[i] == b'"' {
                return Some(i + 1);
            }
            i += 1;
        }
        return None;
    }
    // Number (optional leading -)
    if b[i] == b'-' || b[i].is_ascii_digit() {
        if b[i] == b'-' {
            i += 1;
        }
        let num_start = i;
        while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
            i += 1;
        }
        if i > num_start || (start < b.len() && b[start].is_ascii_digit()) {
            return Some(i);
        }
    }
    // Identifier / true / false
    if b[i].is_ascii_alphabetic() || b[i] == b'_' {
        i += 1;
        while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
            i += 1;
        }
        return Some(i);
    }
    None
}

/// Span of the simple arg token near a diagnostic byte offset.
/// Handles typechecker pointing at the call's closing `)` (exclusive end of last arg).
fn arg_token_span_near(source: &str, at: usize) -> Option<(usize, usize)> {
    let b = source.as_bytes();
    if b.is_empty() {
        return None;
    }
    let mut i = at.min(b.len());
    // If pointing at whitespace, skip forward then treat delimiters.
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    // Call-site `)` / `,` → exclusive end of the preceding argument token.
    let end = if i < b.len() && (b[i] == b')' || b[i] == b',') {
        i
    } else if i < b.len() {
        // Pointing at the arg itself
        return scan_token_end(source, i).map(|e| (i, e));
    } else {
        b.len()
    };
    // Walk back over whitespace before delimiter.
    let mut e = end;
    while e > 0 && b[e - 1].is_ascii_whitespace() {
        e -= 1;
    }
    if e == 0 {
        return None;
    }
    // String ending with "
    if b[e - 1] == b'"' {
        let mut j = e - 1;
        if j == 0 {
            return Some((0, e));
        }
        j -= 1;
        while j > 0 {
            if b[j] == b'"' {
                let mut bs = 0usize;
                let mut k = j;
                while k > 0 && b[k - 1] == b'\\' {
                    bs += 1;
                    k -= 1;
                }
                if bs % 2 == 0 {
                    return Some((j, e));
                }
            }
            j -= 1;
        }
        if b[0] == b'"' {
            return Some((0, e));
        }
        return None;
    }
    // Ident / number: walk back
    let mut s = e;
    while s > 0 {
        let c = b[s - 1];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b'.' {
            s -= 1;
        } else if c == b'-' && s >= 2 && b[s - 2].is_ascii_digit() {
            // don't treat binary minus as part of number; only leading -
            break;
        } else if c == b'-' && (s == 1 || !b[s - 2].is_ascii_alphanumeric()) {
            s -= 1;
            break;
        } else {
            break;
        }
    }
    if s >= e {
        return None;
    }
    Some((s, e))
}


/// Leading whitespace of 0-indexed line `line_0` (spaces/tabs only).
fn line_indent(source: &str, line_0: usize) -> String {
    let mut current = 0usize;
    let mut start = 0usize;
    for (i, ch) in source.char_indices() {
        if current == line_0 {
            start = i;
            break;
        }
        if ch == '\n' {
            current += 1;
            start = i + 1;
        }
    }
    if current != line_0 && line_0 > 0 {
        // past EOF — no indent
        return String::new();
    }
    source[start..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}


/// Locate the declared return type text after `->` for `fn name` (or first `fn` if name empty).
/// Returns half-open byte range of the type token(s) (simple: single identifier / bare type).
fn find_fn_return_type_span(
    source: &str,
    name: &str,
    expected_declared: &str,
) -> Option<(usize, usize)> {
    let needle = if name.is_empty() {
        "fn ".to_string()
    } else {
        format!("fn {}(", name)
    };
    let start = source.find(&needle)?;
    let after = &source[start..];
    let arrow = after.find("->")?;
    let mut i = start + arrow + 2;
    while i < source.len() && source.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    let type_start = i;
    // Consume a simple type name / bracketed form until space or `{` or requires/ensures.
    let rest = &source[type_start..];
    let mut end = 0usize;
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    while end < bytes.len() {
        let b = bytes[end];
        if b == b'[' {
            depth += 1;
            end += 1;
            continue;
        }
        if b == b']' {
            depth -= 1;
            end += 1;
            continue;
        }
        if depth == 0
            && (b.is_ascii_whitespace()
                || b == b'{'
                || rest[end..].starts_with("requires")
                || rest[end..].starts_with("ensures"))
        {
            break;
        }
        end += 1;
    }
    if end == 0 {
        return None;
    }
    let span = &source[type_start..type_start + end];
    if !span.starts_with(expected_declared) {
        // Still accept if declared type is a prefix (e.g. Int vs Int[0..10])
        if span != expected_declared {
            return None;
        }
    }
    Some((type_start, type_start + expected_declared.len().min(end)))
}

