// ===================================================================
// openOODA Module Outline Generator
//
// Produces a token-minimized summary of a `.oo` program suitable for
// AI agent context. Emits clean source-like syntax — NOT the AST's
// `Debug` repr — so requires / ensures clauses are short and
// readable. The previous version printed
//   `Binary { op: Gte, left: Variable("a", Span { ... }), right: ... }`
// which was *longer than the source file itself*, defeating the
// 85–90% token-reduction promise.
//
// Format:
//   pub fn greet(name: String) -> String
//       requires name.len() > 0
//       ensures result.len() > 0
//   type Port = Int where 1..=65535
// ===================================================================
use crate::ast::*;
use anyhow::Result;
use serde_json::json;

pub fn generate_outline(program: &Program) -> String {
    let mut out = String::new();

    for (i, item) in program.items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match item {
            Item::TypeAlias(name, target_type) => {
                out.push_str(&format!("type {} = {}\n", name, format_type(target_type)));
            }
            Item::Import { path, .. } => {
                out.push_str(&format!("import \"{}\"\n", path));
            }
            Item::Function(func) => {
                out.push_str(&format_function(func));
            }
        }
    }

    out
}

/// Machine-readable outline for AI agents (`ooda outline --json`).
/// Source-like strings only — no Debug AST dumps (keeps W low).
pub fn generate_outline_json(program: &Program, file: &str) -> Result<String> {
    let mut functions = Vec::new();
    let mut types = Vec::new();
    let mut imports = Vec::new();
    for item in &program.items {
        match item {
            Item::Function(func) => {
                let params: Vec<_> = func
                    .params
                    .iter()
                    .map(|p| {
                        json!({
                            "name": p.name,
                            "type": format_param_type(p),
                            "is_ref": p.is_ref,
                        })
                    })
                    .collect();
                let caps: Vec<String> = func
                    .params
                    .iter()
                    .filter_map(|p| match p.param_type {
                        Type::NetCap => Some("NetCap".into()),
                        Type::FsCap => Some("FsCap".into()),
                        Type::SysCap => Some("SysCap".into()),
                        Type::EnvCap => Some("EnvCap".into()),
                        _ => None,
                    })
                    .collect();
                functions.push(json!({
                    "name": func.name,
                    "is_pub": func.is_pub,
                    "parameters": params,
                    "return_type": format_type(&func.return_type),
                    "requires": func.requires.iter().map(format_expr).collect::<Vec<_>>(),
                    "ensures": func.ensures.iter().map(format_expr).collect::<Vec<_>>(),
                    "capabilities_required": caps,
                }));
            }
            Item::TypeAlias(name, t) => {
                types.push(json!({
                    "name": name,
                    "type": format_type(t),
                }));
            }
            Item::Import { path, .. } => {
                imports.push(json!({ "path": path }));
            }
        }
    }
    let payload = json!({
        "file": file,
        "functions": functions,
        "types": types,
        "imports": imports,
    });
    Ok(serde_json::to_string_pretty(&payload)?)
}

fn format_param_type(p: &Parameter) -> String {
    let ref_str = if p.is_ref { "&" } else { "" };
    format!("{}{}", ref_str, format_type(&p.param_type))
}

fn format_function(func: &FunctionDecl) -> String {
    let mut s = String::new();

    let pub_str = if func.is_pub { "pub " } else { "" };
    let params_str = func
        .params
        .iter()
        .map(format_param)
        .collect::<Vec<_>>()
        .join(", ");
    let ret_str = match &func.return_type {
        Type::Void => String::new(),
        t => format!(" -> {}", format_type(t)),
    };

    s.push_str(&format!(
        "{}fn {}({}){}\n",
        pub_str, func.name, params_str, ret_str
    ));

    for req in &func.requires {
        s.push_str(&format!("    requires {}\n", format_expr(req)));
    }
    for ens in &func.ensures {
        s.push_str(&format!("    ensures {}\n", format_expr(ens)));
    }

    s
}

fn format_param(p: &Parameter) -> String {
    let ref_str = if p.is_ref { "&" } else { "" };
    format!("{}: {}{}", p.name, ref_str, format_type(&p.param_type))
}

/// Source-like type rendering for outlines / reflect (not Debug).
pub fn format_type(t: &Type) -> String {
    match t {
        Type::Int => "Int".into(),
        Type::Float => "Float".into(),
        Type::String => "String".into(),
        Type::Bool => "Bool".into(),
        Type::Void => "Void".into(),
        Type::Custom(s) => s.clone(),
        // Capability types do NOT include the leading `&` here —
        // `format_param` adds it based on the parameter's `is_ref`
        // flag, so emitting `&` here would produce `&&NetCap`.
        Type::NetCap => "NetCap".into(),
        Type::FsCap => "FsCap".into(),
        Type::EnvCap => "EnvCap".into(),
        Type::SysCap => "SysCap".into(),
        Type::List(inner) => format!("List[{}]", format_type(inner)),
        Type::Struct { name, fields } => {
            let body: Vec<String> = fields
                .iter()
                .map(|(n, t)| format!("{}: {}", n, format_type(t)))
                .collect();
            match name {
                Some(n) => format!("{} {{ {} }}", n, body.join(", ")),
                None => format!("struct {{ {} }}", body.join(", ")),
            }
        }
        Type::Option(inner) => format!("Option[{}]", format_type(inner)),
        Type::Result(ok, err) => {
            format!("Result[{}, {}]", format_type(ok), format_type(err))
        }
    }
}

fn format_literal(lit: &Literal) -> String {
    match lit {
        Literal::Int(n) => n.to_string(),
        Literal::Float(n) => n.to_string(),
        Literal::Bool(b) => b.to_string(),
        Literal::String(s) => format!("\"{}\"", s),
        Literal::Void => "()".into(),
    }
}
