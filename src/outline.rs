use crate::ast::*;

pub fn generate_outline(program: &Program) -> String {
    let mut out = String::new();

    for item in &program.items {
        match item {
            Item::TypeAlias(name, target_type) => {
                out.push_str(&format!("type {} = {:?}\n", name, target_type));
            }
            Item::Function(func) => {
                let pub_str = if func.is_pub { "pub " } else { "" };
                let params_str = func
                    .params
                    .iter()
                    .map(|p| {
                        let ref_str = if p.is_ref { "&" } else { "" };
                        format!("{}: {}{:?}", p.name, ref_str, p.param_type)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                let ret_str = if func.return_type != Type::Void {
                    format!(" -> {:?}", func.return_type)
                } else {
                    String::new()
                };

                out.push_str(&format!("{}fn {}({}){}\n", pub_str, func.name, params_str, ret_str));

                for req in &func.requires {
                    out.push_str(&format!("  requires {:?}\n", req));
                }
                for ens in &func.ensures {
                    out.push_str(&format!("  ensures {:?}\n", ens));
                }
            }
        }
    }

    out
}
