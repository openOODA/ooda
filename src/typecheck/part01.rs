
/// Parse `Int[lo..hi]` refinement bounds from a type annotation.
pub fn int_refinement_bounds(ty: &Type) -> Option<(i64, i64)> {
    if let Type::Custom(s) = ty {
        if let Some(rest) = s.strip_prefix("Int[").and_then(|r| r.strip_suffix(']')) {
            if let Some((min_s, max_s)) = rest.split_once("..") {
                let min_v: i64 = min_s.parse().ok()?;
                let max_v: i64 = max_s.parse().ok()?;
                return Some((min_v, max_v));
            }
        }
    }
    None
}

