import re
with open("oodac/c_emit.oo", "r") as f:
    text = f.read()

pre_pass = """    // Pre-pass: collect all fn return types to populate fn_env with __fr__ flags
    let mut pre_pos = 0;
    while pre_pos < n {
        let k = field_at(list_get(toks, pre_pos), 0);
        if k == "KW_PUB" || k == "KW_FN" {
            let mut p = pre_pos;
            if k == "KW_PUB" { p = p + 1; }
            p = p + 1;
            let fn_name = field_at(list_get(toks, p), 3);
            p = p + 1;
            p = p + 1;
            while p < n {
                if field_at(list_get(toks, p), 0) == "RPAREN" {
                    p = p + 1;
                    break;
                }
                p = p + 1;
            }
            if p < n && field_at(list_get(toks, p), 0) == "ARROW" {
                p = p + 1;
                let ret_ty = field_at(list_get(toks, p), 3);
                if ret_ty == "String" {
                    fn_env = c_env_put(fn_env, "__fr__" + fn_name, "T");
                }
            }
            pre_pos = p;
        } else {
            pre_pos = pre_pos + 1;
        }
    }
"""

text = text.replace('    let mut fn_env = "";', '    let mut fn_env = "";\n' + pre_pass)
with open("oodac/c_emit.oo", "w") as f:
    f.write(text)
