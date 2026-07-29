// ===================================================================
// Multi-file .oo loader (`import "path.oo";`)
// Resolves userland modules so org code can stay in .oo form.
// ===================================================================
use crate::ast::*;
use crate::lexer::Lexer;
use crate::parser::Parser;
use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Parse a root .oo file and recursively merge `import "…"` modules.
pub fn load_program(root: &Path) -> Result<Program> {
    let mut visited = HashSet::new();
    load_program_inner(root, &mut visited)
}

fn load_program_inner(path: &Path, visited: &mut HashSet<PathBuf>) -> Result<Program> {
    let canon = path
        .canonicalize()
        .with_context(|| format!("Cannot open OODA source '{}'", path.display()))?;
    if !visited.insert(canon.clone()) {
        bail!(
            "Import cycle detected involving '{}'",
            path.display()
        );
    }

    let code = fs::read_to_string(&canon)
        .with_context(|| format!("Failed to read '{}'", canon.display()))?;
    let mut lexer = Lexer::new(&code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse_program()?;

    let mut merged_items: Vec<Item> = Vec::new();
    let parent = canon.parent().unwrap_or_else(|| Path::new("."));

    for item in program.items.drain(..) {
        match item {
            Item::Import { path: rel, span } => {
                let resolved = resolve_import(parent, &rel).map_err(|e| {
                    anyhow!(
                        "Import \"{}\" at {}:{}: {}",
                        rel,
                        span.line,
                        span.col,
                        e
                    )
                })?;
                let imported = load_program_inner(&resolved, visited)?;
                // Imported functions/types first (local items may override by name later at runtime last-wins if we append).
                for it in imported.items {
                    match it {
                        Item::Import { .. } => {} // already expanded
                        other => merged_items.push(other),
                    }
                }
            }
            other => merged_items.push(other),
        }
    }
    program.items = merged_items;
    Ok(program)
}

/// Resolve import path: relative to importer, then OODA_PATH, OODA_STD, sibling openooda-std.
fn resolve_import(from_dir: &Path, rel: &str) -> Result<PathBuf> {
    let rel_path = Path::new(rel);
    let candidates: Vec<PathBuf> = {
        let mut v = vec![from_dir.join(rel_path)];
        if let Ok(paths) = std::env::var("OODA_PATH") {
            for p in std::env::split_paths(&paths) {
                v.push(p.join(rel_path));
            }
        }
        if let Ok(std_root) = std::env::var("OODA_STD") {
            let root = PathBuf::from(&std_root);
            v.push(root.join(rel_path));
            // also allow import "crypto.oo" from std root files
            if let Some(name) = rel_path.file_name() {
                v.push(PathBuf::from(&std_root).join(name));
            }
        }
        // Convenience: repo-adjacent openooda-std when developing locally
        if let Some(home) = from_dir
            .ancestors()
            .find(|a| a.join("openooda-std").is_dir() || a.file_name().map(|n| n == "openooda").unwrap_or(false))
        {
            let std_dir = if home.file_name().map(|n| n == "openooda").unwrap_or(false) {
                home.parent().map(|p| p.join("openooda-std"))
            } else {
                Some(home.join("openooda-std"))
            };
            if let Some(sd) = std_dir {
                v.push(sd.join(rel_path));
                if let Some(name) = rel_path.file_name() {
                    v.push(sd.join(name));
                }
            }
        }
        // Sibling std layouts: openooda-std, std (monorepo Projects/openOODA/std)
        for std_name in ["openooda-std", "std"] {
            if let Some(parent) = std::env::current_dir()
                .ok()
                .and_then(|c| c.parent().map(|p| p.join(std_name)))
            {
                if parent.is_dir() {
                    v.push(parent.join(rel_path));
                    if let Some(name) = rel_path.file_name() {
                        v.push(parent.join(name));
                    }
                }
            }
            if let Ok(home) = std::env::var("HOME") {
                let p = PathBuf::from(home).join("Projects/openOODA").join(std_name);
                if p.is_dir() {
                    v.push(p.join(rel_path));
                    if let Some(name) = rel_path.file_name() {
                        v.push(p.join(name));
                    }
                }
            }
        }
        v
    };

    for c in &candidates {
        if c.is_file() {
            return Ok(c.clone());
        }
    }
    Err(anyhow!(
        "could not resolve module '{}'. Searched relative path, OODA_PATH, OODA_STD, openooda-std. Candidates tried: {:?}",
        rel,
        candidates
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
        #[test]
    fn loads_import_relative() {
        let base = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(".cache")
            .join(format!("ooda-import-{}", std::process::id()));
        let _ = fs::create_dir_all(&base);
        let lib = base.join("lib.oo");
        let main = base.join("main.oo");
        fs::write(&lib, "pub fn add(a: Int, b: Int) -> Int { return a + b; }\n").unwrap();
        fs::write(
            &main,
            "import \"lib.oo\";\npub fn main() { let x = add(1, 2); println(x); }\n",
        )
        .unwrap();
        let prog = load_program(&main).expect("load");
        let fns: Vec<_> = prog
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Function(f) => Some(f.name.as_str()),
                _ => None,
            })
            .collect();
        assert!(fns.contains(&"add"), "{:?}", fns);
        assert!(fns.contains(&"main"), "{:?}", fns);
        let _ = fs::remove_dir_all(&base);
    }
}
