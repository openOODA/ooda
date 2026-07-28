use crate::ast::*;

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();

    for (i, item) in program.items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match item {
            Item::TypeAlias(name, target_type) => {
                out.push_str(&format!("type {} = {:?};\n", name, target_type));
            }
            Item::Function(func) => {
                out.push_str(&format_function(func));
            }
        }
    }

    out
}

fn format_function(func: &FunctionDecl) -> String {
    let mut s = String::new();

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

    s.push_str(&format!("{}fn {}({}){}\n", pub_str, func.name, params_str, ret_str));

    for req in &func.requires {
        s.push_str(&format!("    requires {:?}\n", req));
    }
    for ens in &func.ensures {
        s.push_str(&format!("    ensures {:?}\n", ens));
    }

    s.push_str("{\n");
    for stmt in &func.body.stmts {
        s.push_str(&format!("    {:?};\n", stmt));
    }
    s.push_str("}\n");

    if let Some(verify) = &func.verify_block {
        s.push_str(&format!("\nverify {} {{\n", func.name));
        for stmt in &verify.stmts {
            s.push_str(&format!("    {:?};\n", stmt));
        }
        s.push_str("}\n");
    }

    s
}
