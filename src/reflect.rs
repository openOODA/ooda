use crate::ast::*;
use serde::Serialize;
use anyhow::{anyhow, Result};

#[derive(Debug, Serialize)]
pub struct SymbolReflection {
    pub symbol: String,
    pub kind: String,
    pub is_pub: bool,
    pub parameters: Vec<ParameterReflection>,
    pub return_type: String,
    pub preconditions: Vec<String>,
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
                let params = func.params.iter().map(|p| {
                    match p.param_type {
                        Type::NetCap => caps.push("NetCap".to_string()),
                        Type::FsCap => caps.push("FsCap".to_string()),
                        Type::SysCap => caps.push("SysCap".to_string()),
                        Type::EnvCap => caps.push("EnvCap".to_string()),
                        _ => {}
                    }
                    ParameterReflection {
                        name: p.name.clone(),
                        param_type: format!("{:?}", p.param_type),
                        is_ref: p.is_ref,
                    }
                }).collect();

                let reflection = SymbolReflection {
                    symbol: func.name.clone(),
                    kind: "Function".to_string(),
                    is_pub: func.is_pub,
                    parameters: params,
                    return_type: format!("{:?}", func.return_type),
                    preconditions: func.requires.iter().map(|r| format!("{:?}", r)).collect(),
                    postconditions: func.ensures.iter().map(|e| format!("{:?}", e)).collect(),
                    capabilities_required: caps,
                };

                return Ok(serde_json::to_string_pretty(&reflection)?);
            }
            Item::TypeAlias(name, t) if name == target_name => {
                let reflection = serde_json::json!({
                    "symbol": name,
                    "kind": "TypeAlias",
                    "target_type": format!("{:?}", t),
                });
                return Ok(serde_json::to_string_pretty(&reflection)?);
            }
            _ => {}
        }
    }

    Err(anyhow!("Symbol '{}' not found in program AST", target_name))
}
