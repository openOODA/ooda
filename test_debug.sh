sed -i 's/while p < n {/while p < n {\n        println("DEBUG: p=" + p.to_string() + " k=" + field_at(list_get(toks, p), 0) + " txt=" + field_at(list_get(toks, p), 3));/' oodac/llvm_emit_stmt.oo
PURE_SKIP_CHECK=1 OODAC_BIN=$PWD/dist/oodac_llvm_capable bash scripts/my_pure_build.sh oodac/main.oo dist/oodac_debug
./dist/oodac_debug emit-llvm oodac/lex.oo > test_lex_debug.ll
