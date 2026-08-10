sed -i 's/if p < n && field_at(list_get(toks, p), 0) == "LBRACE" {/println("FN_NAME: " + fn_name + " p: " + p.to_string() + " tok_at_p: " + field_at(list_get(toks, p), 0));\n    if p < n \&\& field_at(list_get(toks, p), 0) == "LBRACE" {/' oodac/llvm_emit_fn.oo
PURE_SKIP_CHECK=1 OODAC_BIN=$PWD/dist/oodac_llvm_capable bash scripts/my_pure_build.sh oodac/main.oo dist/oodac_debug2
./dist/oodac_debug2 emit-llvm oodac/lex.oo > test_lex_debug2.ll
