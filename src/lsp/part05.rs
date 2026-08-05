
/// Find the byte index of the closing `}` of `fn NAME` / `pub fn NAME` body.
fn find_fn_body_close(source: &str, name: &str) -> Option<usize> {
    let patterns = [format!("fn {}(", name), format!("fn {} (", name)];
    let mut start = None;
    for p in &patterns {
        if let Some(idx) = source.find(p.as_str()) {
            start = Some(idx);
            break;
        }
    }
    let start = start?;
    let brace = source[start..].find('{')? + start;
    let mut depth: i32 = 0;
    let bytes = source.as_bytes();
    let mut i = brace;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'"' => {
                // skip string
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        break;
                    }
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}


fn indent_before(source: &str, byte: usize) -> String {
    let line_start = source[..byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line = &source[line_start..byte];
    let n = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    // Body indent is typically closing-brace indent + 4 spaces.
    format!("{}    ", &line[..n])
}

