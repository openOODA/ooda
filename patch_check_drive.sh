#!/bin/bash
cat << 'INNER' > oodac/check_drive.oo.new
// job: check command orchestration; stage: check
// in: source String; out: OK/ERR side effects
// quiet: run_check_core for emit gate (no OK line on stdout)

pub fn hash_tokens(toks: List[String], start: Int, end: Int) -> String {
    let mut s = "";
    let mut i = start;
    while i < end {
        s = s + field_at(list_get(toks, i), 3) + " ";
        i = i + 1;
    }
    return crypto_sha256_internal(s);
}

pub fn run_check_from_src(fs: &FsCap, sys: &SysCap, src: String) {
    run_check_core(fs, sys, src);
    println("OK");
}

pub fn run_check_core(fs: &FsCap, sys: &SysCap, src: String) {
    if chars_len(src) == 0 {
        println("ERR\tcheck\tempty");
        process_exit(1);
    }
    // H6: expanded source bound (import concat); per-file is 64KiB at load
    if chars_len(src) > 1048576 {
        println("ERR\tcheck\tsource exceeds 1MiB expanded limit");
        process_exit(1);
    }
    let probe = lex_all(src);
    if probe.is_err() {
        let e = match probe {
            Ok(s) => s,
            Err(msg) => msg,
        };
        println("ERR\tlex\t" + e);
        process_exit(1);
    }
    let toks: List[String] = tokenize_lines(src);
    let n = list_len(toks);
    if n <= 1 {
        println("ERR\tcheck\tno_fn");
        process_exit(1);
    }

    let mut pos = 0;
    let mut saw_fn = false;
    while pos < n {
        let k = field_at(list_get(toks, pos), 0);
        if k == "EOF" {
            pos = n;
        } else if k == "KW_PUB" || k == "KW_FN" {
            saw_fn = true;
            pos = check_function(toks, pos);
        } else if k == "KW_TYPE" || k == "KW_IMPORT" {
            pos = skip_until_semi(toks, pos);
        } else {
            println("ERR\tparse\tunexpected " + k);
            process_exit(1);
        }
    }
    if !saw_fn {
        println("ERR\tcheck\tno_fn");
        process_exit(1);
    }

    // Incremental validation: Hash global environment (signatures only)
    let mut sigs = "";
    let mut p = 0;
    while p < n {
        let k = field_at(list_get(toks, p), 0);
        if k == "KW_PUB" || k == "KW_FN" {
            while p < n {
                let tk = field_at(list_get(toks, p), 0);
                sigs = sigs + field_at(list_get(toks, p), 3) + " ";
                if tk == "LBRACE" {
                    p = skip_balanced(toks, p);
                    break;
                }
                p = p + 1;
            }
        } else if k == "KW_TYPE" {
            while p < n {
                let tk = field_at(list_get(toks, p), 0);
                sigs = sigs + field_at(list_get(toks, p), 3) + " ";
                if tk == "SEMI" {
                    p = p + 1;
                    break;
                }
                p = p + 1;
            }
        } else {
            p = p + 1;
        }
    }
    let global_hash = crypto_sha256_internal(sigs);

    let mut active_toks: List[String] = list_new();
    p = 0;
    while p < n {
        let k = field_at(list_get(toks, p), 0);
        if k == "KW_PUB" || k == "KW_FN" {
            while p < n {
                let tk = field_at(list_get(toks, p), 0);
                active_toks = list_push(active_toks, list_get(toks, p));
                if tk == "LBRACE" {
                    let body_start = p + 1;
                    p = skip_balanced(toks, p);
                    let body_end = p - 1; // RBRACE is at p-1
                    let fn_hash = hash_tokens(toks, body_start, body_end);
                    let cache_key = "/tmp/.oodac_fn_" + global_hash + "_" + fn_hash;
                    if path_exists(fs, cache_key) {
                        // cached! skip body
                        active_toks = list_push(active_toks, list_get(toks, p - 1));
                    } else {
                        // not cached, keep body
                        let mut bi = body_start;
                        while bi < p {
                            active_toks = list_push(active_toks, list_get(toks, bi));
                            bi = bi + 1;
                        }
                        let wr = write_file(fs, cache_key, "1");
                    }
                    break;
                }
                p = p + 1;
            }
        } else {
            active_toks = list_push(active_toks, list_get(toks, p));
            p = p + 1;
        }
    }

    typecheck_ann_and_return_lits(active_toks);
    typecheck_undefined_vars(active_toks);
    typecheck_call_arity(active_toks);
    typecheck_immut_assign(active_toks);
    typecheck_mut_assign_types(active_toks);
    typecheck_unary_bang_lit(active_toks);
    typecheck_unary_minus_lit(active_toks);
    typecheck_cmp_numeric_lits(active_toks);
    typecheck_if_while_lit_cond(active_toks);
    typecheck_control_flow_branches(active_toks);
    typecheck_logic_binop_lits(active_toks);
    typecheck_reject_amp_pipe_binop(active_toks);
    typecheck_reject_shift_ops(active_toks);
    typecheck_missing_return(active_toks);
    typecheck_refinements(active_toks);
    typecheck_must_use_result(active_toks);
    typecheck_call_arg_lits(active_toks);
    typecheck_return_and_assign_calls(active_toks);
    typecheck_let_ann_call_init(active_toks);
    typecheck_call_eq_lits(active_toks);
    typecheck_call_order_lits(active_toks);
    typecheck_call_binop_lits(active_toks);
    typecheck_call_logic_lits(active_toks);
    typecheck_field_method(active_toks);
    typecheck_field_binop_uses(active_toks);
    typecheck_struct_lit_inits(active_toks);
    typecheck_field_assign(active_toks);
}
INNER
mv oodac/check_drive.oo.new oodac/check_drive.oo
