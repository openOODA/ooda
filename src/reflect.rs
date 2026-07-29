use crate::ast::*;
use crate::outline::{format_expr, format_type};
use anyhow::{anyhow, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SymbolReflection {
    pub symbol: String,
    pub kind: String,
    pub is_pub: bool,
    pub parameters: Vec<ParameterReflection>,
    pub return_type: String,
    /// Source-like requires clauses (not Debug AST — cuts AI context weight).
    pub preconditions: Vec<String>,
    /// Source-like ensures clauses.
    pub postconditions: Vec<String>,
    pub capabilities_required: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ParameterReflection {
    pub name: String,
    pub param_type: String,
    pub is_ref: bool,
}

pub fn reflect_symbol(program: &Program, target_name: &str) -> Result<String> {
    for item in &program.items {
        match item {
            Item::Function(func) if func.name == target_name => {
                let mut caps = Vec::new();
                let params = func
                    .params
                    .iter()
                    .map(|p| {
                        match p.param_type {
                            Type::NetCap => caps.push("NetCap".to_string()),
                            Type::FsCap => caps.push("FsCap".to_string()),
                            Type::SysCap => caps.push("SysCap".to_string()),
                            Type::EnvCap => caps.push("EnvCap".to_string()),
                            _ => {}
                        }
                        ParameterReflection {
                            name: p.name.clone(),
                            param_type: format_type(&p.param_type),
                            is_ref: p.is_ref,
                        }
                    })
                    .collect();

                let reflection = SymbolReflection {
                    symbol: func.name.clone(),
                    kind: "Function".to_string(),
                    is_pub: func.is_pub,
                    parameters: params,
                    return_type: format_type(&func.return_type),
                    preconditions: func.requires.iter().map(format_expr).collect(),
                    postconditions: func.ensures.iter().map(format_expr).collect(),
                    capabilities_required: caps,
                };

                return Ok(serde_json::to_string_pretty(&reflection)?);
            }
            Item::TypeAlias(name, t) if name == target_name => {
                let reflection = serde_json::json!({
                    "symbol": name,
                    "kind": "TypeAlias",
                    "target_type": format_type(t),
                });
                return Ok(serde_json::to_string_pretty(&reflection)?);
            }
            _ => {}
        }
    }

    Err(anyhow!("Symbol '{}' not found in program AST", target_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(src: &str) -> Program {
        let mut l = Lexer::new(src);
        let toks = l.tokenize().expect("lex");
        let mut p = Parser::new(toks);
        p.parse_program().expect("parse")
    }

    #[test]
    fn reflect_uses_source_like_contracts_not_debug_ast() {
        let prog = parse(
            r#"
            pub fn greet(name: String) -> String
                requires name.len() > 0
                ensures result.len() > 0
            {
                return "hi";
            }
            "#,
        );
        let js = reflect_symbol(&prog, "greet").expect("reflect");
        assert!(js.contains("name.len() > 0"), "source-like requires: {}", js);
        assert!(js.contains("result.len() > 0"), "source-like ensures: {}", js);
        assert!(!js.contains("Binary {"), "must not dump Debug AST: {}", js);
        assert!(!js.contains("Span {"), "must not dump spans: {}", js);
        assert!(js.contains("\"return_type\": \"String\""), "pretty type: {}", js);
    }

    #[test]
    fn reflect_cap_params_listed() {
        let prog = parse(
            r#"
            pub fn fetch_url(net: &NetCap, url: String) -> Result[String, String] {
                return fetch(net, url);
            }
            "#,
        );
        let js = reflect_symbol(&prog, "fetch_url").expect("reflect");
        assert!(js.contains("NetCap"), "caps: {}", js);
        assert!(js.contains("Result[String, String]") || js.contains("Result"), "ret: {}", js);
    }
}
