
fn format_type(t: &Type) -> String {
    match t {
        Type::Int => "Int".into(),
        Type::Float => "Float".into(),
        Type::String => "String".into(),
        Type::Bool => "Bool".into(),
        Type::Void => "Void".into(),
        Type::Custom(s) => s.clone(),
        Type::Option(i) => format!("Option[{}]", format_type(i)),
        Type::Result(a, b) => format!("Result[{},{}]", format_type(a), format_type(b)),
        Type::List(i) => format!("List[{}]", format_type(i)),
        Type::Struct { name, fields } => {
            let fs: Vec<String> = fields
                .iter()
                .map(|(n, t)| format!("{}:{}", n, format_type(t)))
                .collect();
            match name {
                Some(n) => format!("struct:{}{{{}}}", n, fs.join(",")),
                None => format!("struct{{{}}}", fs.join(",")),
            }
        }
        Type::NetCap => "NetCap".into(),
        Type::FsCap => "FsCap".into(),
        Type::EnvCap => "EnvCap".into(),
        Type::SysCap => "SysCap".into(),
    }
}

fn dump_block(out: &mut String, block: &Block, depth: usize) {
    let pad = indent(depth);
    out.push_str(&format!("{}BLOCK stmts={}\n", pad, block.stmts.len()));
    for (i, s) in block.stmts.iter().enumerate() {
        dump_stmt(out, s, i, depth + 1);
    }
    if let Some(e) = &block.expr {
        out.push_str(&format!("{}TAIL\n", pad));
        dump_expr(out, e, depth + 1);
    }
}

fn dump_stmt(out: &mut String, stmt: &Statement, idx: usize, depth: usize) {
    let pad = indent(depth);
    match stmt {
        Statement::Let {
            name,
            mutable,
            type_annotation,
            init,
            span,
        } => {
            let ann = type_annotation
                .as_ref()
                .map(format_type)
                .unwrap_or_else(|| "_".into());
            out.push_str(&format!(
                "{}STMT[{}] LET mut={} name={} ann={} @{}:{}\n",
                pad, idx, mutable, name, ann, span.line, span.col
            ));
            dump_expr(out, init, depth + 1);
        }
        Statement::FieldAssign { object, field, value, span } => {
            out.push_str(&format!(
                "{}STMT[{}] FIELD_ASSIGN .{} @{}:{}
",
                pad, idx, field, span.line, span.col
            ));
            dump_expr(out, object, depth + 1);
            dump_expr(out, value, depth + 1);
        }
        Statement::Assign { name, value, span } => {
            out.push_str(&format!(
                "{}STMT[{}] ASSIGN name={} @{}:{}\n",
                pad, idx, name, span.line, span.col
            ));
            dump_expr(out, value, depth + 1);
        }
        Statement::Return(Some(e), span) => {
            out.push_str(&format!(
                "{}STMT[{}] RETURN @{}:{}\n",
                pad, idx, span.line, span.col
            ));
            dump_expr(out, e, depth + 1);
        }
        Statement::Break(span) => {
            out.push_str(&format!(
                "{}STMT[{}] BREAK @{}:{}\n",
                pad, idx, span.line, span.col
            ));
        }
        Statement::Continue(span) => {
            out.push_str(&format!(
                "{}STMT[{}] CONTINUE @{}:{}\n",
                pad, idx, span.line, span.col
            ));
        }
        Statement::Return(None, span) => {
            out.push_str(&format!(
                "{}STMT[{}] RETURN_VOID @{}:{}\n",
                pad, idx, span.line, span.col
            ));
        }
        Statement::Expr(e, span) => {
            out.push_str(&format!(
                "{}STMT[{}] EXPR @{}:{}\n",
                pad, idx, span.line, span.col
            ));
            dump_expr(out, e, depth + 1);
        }
        Statement::While { cond, body, span } => {
            out.push_str(&format!(
                "{}STMT[{}] WHILE @{}:{}\n",
                pad, idx, span.line, span.col
            ));
            dump_expr(out, cond, depth + 1);
            dump_block(out, body, depth + 1);
        }
    }
}
