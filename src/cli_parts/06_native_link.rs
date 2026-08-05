enum NativeLinkResult {
    Ok,
    NoTool,
    ToolFailed { tool: String, detail: String },
}

/// Link LLVM IR (.ll) to a native binary.
///
/// Only **clang** (and versioned clang-*) can consume LLVM IR text as input.
/// Plain `gcc`/`cc` treat `.ll` as a linker script and fail noisily — never try them.
fn try_native_link(ll: &std::path::Path, out_bin: &std::path::Path) -> NativeLinkResult {
    let mut tools: Vec<String> = Vec::new();
    // Prefer explicit OODA_CLANG / CC only if the name looks like clang.
    for key in ["OODA_CLANG", "CC"] {
        if let Ok(cc) = std::env::var(key) {
            let base = cc.rsplit('/').next().unwrap_or(&cc);
            if base.contains("clang") && !tools.iter().any(|x| x == &cc) {
                tools.push(cc);
            }
        }
    }
    for t in ["clang", "clang-18", "clang-17", "clang-16", "clang-15", "clang-14"] {
        if !tools.iter().any(|x| x == t) {
            tools.push(t.to_string());
        }
    }

    let mut last_fail: Option<(String, String)> = None;
    let mut saw_clang = false;
    for tool in tools {
        let probe = std::process::Command::new(&tool).arg("--version").output();
        let Ok(probe_out) = probe else {
            continue;
        };
        let ver = String::from_utf8_lossy(&probe_out.stdout);
        if !ver.to_ascii_lowercase().contains("clang") {
            // Refuse non-clang drivers even if named oddly.
            continue;
        }
        saw_clang = true;
        // `-x ir` forces IR input language so the suffix is unambiguous.
        let out = std::process::Command::new(&tool)
            .arg("-x")
            .arg("ir")
            .arg(ll)
            .arg("-Wno-override-module")
            .arg("-o")
            .arg(out_bin)
            .output();
        match out {
            Ok(o) if o.status.success() => return NativeLinkResult::Ok,
            Ok(o) => {
                let detail = String::from_utf8_lossy(&o.stderr).trim().to_string();
                last_fail = Some((
                    tool,
                    if detail.is_empty() {
                        format!("exit {}", o.status)
                    } else {
                        detail.chars().take(240).collect()
                    },
                ));
            }
            Err(e) => last_fail = Some((tool, e.to_string())),
        }
    }
    if let Some((tool, detail)) = last_fail {
        NativeLinkResult::ToolFailed { tool, detail }
    } else if !saw_clang {
        NativeLinkResult::NoTool
    } else {
        NativeLinkResult::NoTool
    }
}

