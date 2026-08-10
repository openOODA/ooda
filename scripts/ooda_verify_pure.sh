#!/usr/bin/env bash
# job: pure verify harness (assert_eq!/assert_ne!/assert!/let; no Python)
# in:  OODA_TEST_SRC, OODA_TEST_HARNESS
# out: writes harness .oo; empty = check-only; exit 0 ok; exit 1 fail-closed
set -euo pipefail
SRC="${OODA_TEST_SRC:?OODA_TEST_SRC}"
OUT="${OODA_TEST_HARNESS:?OODA_TEST_HARNESS}"
[[ -f "$SRC" ]] || { echo "ERR	test	unreadable file: $SRC" >&2; exit 2; }

# Strip requires/ensures lines, then scan (assert_eq/ne/assert + let).
STRIPPED="$(mktemp "${TMPDIR:-/tmp}/ooda_verify_strip.XXXXXX")"
cleanup() { rm -f "$STRIPPED"; }
trap cleanup EXIT
grep -vE '^[[:space:]]*(requires|ensures)[[:space:]]' "$SRC" >"$STRIPPED" || true

set +e
awk -v out="$OUT" -v src="$STRIPPED" '
# /dev/null argv: all work is in BEGIN (avoids stdin hang)
function err(msg) { printf "ERR\ttest\tharness: %s\n", msg > "/dev/stderr"; exit 1 }
function skip_ws(   c) {
  while (i <= n) {
    c = substr(text, i, 1)
    if (c == " " || c == "\t" || c == "\r" || c == "\n") i++
    else break
  }
}
function match_kw(kw,   L, end, c) {
  L = length(kw)
  if (substr(text, i, L) != kw) return 0
  end = i + L
  if (end <= n) {
    c = substr(text, end, 1)
    if ((c >= "A" && c <= "Z") || (c >= "a" && c <= "z") || (c >= "0" && c <= "9") || c == "_") return 0
  }
  return end
}
function skip_balanced(open_c, close_c,   depth, in_str, esc, c) {
  if (i > n || substr(text, i, 1) != open_c) err("expected " open_c " at " i)
  depth = 0; in_str = 0; esc = 0
  while (i <= n) {
    c = substr(text, i, 1)
    if (in_str) {
      if (esc) esc = 0
      else if (c == "\\") esc = 1
      else if (c == "\"") in_str = 0
      i++; continue
    }
    if (c == "\"") { in_str = 1; i++; continue }
    if (c == open_c) depth++
    else if (c == close_c) {
      depth--; i++
      if (depth == 0) return
      continue
    }
    i++
  }
  err("unbalanced " open_c close_c)
}
function parse_assert_args(inner,   depth, in_str, esc, k, c, lhs, rhs) {
  depth = 0; in_str = 0; esc = 0
  for (k = 1; k <= length(inner); k++) {
    c = substr(inner, k, 1)
    if (in_str) {
      if (esc) esc = 0
      else if (c == "\\") esc = 1
      else if (c == "\"") in_str = 0
      continue
    }
    if (c == "\"") { in_str = 1; continue }
    if (c == "(" || c == "[" || c == "{") depth++
    else if (c == ")" || c == "]" || c == "}") depth--
    else if (c == "," && depth == 0) {
      lhs = substr(inner, 1, k - 1); sub(/^[[:space:]]+/, "", lhs); sub(/[[:space:]]+$/, "", lhs)
      rhs = substr(inner, k + 1); sub(/^[[:space:]]+/, "", rhs); sub(/[[:space:]]+$/, "", rhs)
      if (lhs == "" || rhs == "") err("empty assert_eq arg")
      a_lhs = lhs; a_rhs = rhs
      return
    }
  }
  err("assert_eq needs two args")
}
function trim(s) { sub(/^[[:space:]]+/, "", s); sub(/[[:space:]]+$/, "", s); return s }
function id_at() {
  if (match(substr(text, i), /^[A-Za-z_][A-Za-z0-9_]*/)) return substr(text, i, RLENGTH)
  return ""
}
BEGIN {
  while ((getline line < src) > 0) text = text line "\n"
  close(src)
  n = length(text); i = 1
  fn_n = 0; as_n = 0; verify_count = 0
  while (i <= n) {
    skip_ws()
    if (i > n) break
    if (substr(text, i, 2) == "//") {
      while (i <= n && substr(text, i, 1) != "\n") i++
      continue
    }
    if (substr(text, i, 2) == "/*") {
      end = index(substr(text, i + 2), "*/")
      if (end == 0) err("unclosed block comment")
      i = i + end + 3
      continue
    }
    start_item = i
    j = match_kw("pub"); if (j) { i = j; skip_ws() }
    j = match_kw("fn")
    if (j) {
      i = j; skip_ws()
      fname = id_at(); if (fname == "") err("fn without name")
      i += length(fname); skip_ws()
      if (i > n || substr(text, i, 1) != "(") err("fn " fname ": expected (")
      skip_balanced("(", ")"); skip_ws()
      if (substr(text, i, 2) == "->") {
        i += 2; skip_ws()
        while (i <= n && substr(text, i, 1) != "{") i++
      }
      skip_ws()
      if (i > n || substr(text, i, 1) != "{") err("fn " fname ": expected body {")
      skip_balanced("{", "}")
      chunk = substr(text, start_item, i - start_item)
      sub(/[[:space:]]+$/, "", chunk)
      if (fname != "main") { fn_n++; fns[fn_n] = chunk "\n" }
      continue
    }
    i = start_item
    j = match_kw("verify")
    if (j) {
      i = j; skip_ws()
      vname = id_at(); if (vname == "") err("verify without name")
      i += length(vname); skip_ws()
      if (i > n || substr(text, i, 1) != "{") err("verify " vname ": expected {")
      body_start = i + 1
      skip_balanced("{", "}")
      body = substr(text, body_start, i - 1 - body_start)
      verify_count++
      pos = 1; blen = length(body); aidx = 0
      while (pos <= blen) {
        while (pos <= blen) {
          c = substr(body, pos, 1)
          if (c == " " || c == "\t" || c == "\r" || c == "\n") pos++
          else break
        }
        if (pos > blen) break
        if (substr(body, pos, 2) == "//") {
          while (pos <= blen && substr(body, pos, 1) != "\n") pos++
          continue
        }
        rest = substr(body, pos); kind = ""
        if (match(rest, /^assert_eq![[:space:]]*\(/)) { kind = "eq"; pos += RLENGTH - 1 }
        else if (match(rest, /^assert_ne![[:space:]]*\(/)) { kind = "ne"; pos += RLENGTH - 1 }
        else if (match(rest, /^assert![[:space:]]*\(/)) { kind = "assert"; pos += RLENGTH - 1 }
        else if (match(rest, /^let[[:space:]]+/)) {
          semi = index(substr(body, pos), ";")
          if (semi == 0) err("verify " vname ": unclosed let statement")
          stmt = substr(body, pos, semi)
          as_n++; kinds[as_n] = "stmt"; a1[as_n] = stmt; a2[as_n] = ""; vn[as_n] = vname; ai[as_n] = aidx
          pos += semi; continue
        } else {
          snip = substr(body, pos, 48); gsub(/\n/, " ", snip)
          err("verify " vname ": only assert_eq!/assert_ne!/assert! (" snip ")")
        }
        save_i = i; save_text = text; save_n = n
        text = body; n = blen; i = pos
        skip_balanced("(", ")")
        endp = i
        inner = substr(body, pos + 1, endp - pos - 2)
        i = save_i; text = save_text; n = save_n
        aidx++
        if (kind == "assert") {
          expr = trim(inner)
          if (expr == "") err("verify " vname ": empty assert!")
          as_n++; kinds[as_n] = "assert"; a1[as_n] = expr; a2[as_n] = ""; vn[as_n] = vname; ai[as_n] = aidx
        } else {
          parse_assert_args(inner)
          as_n++; kinds[as_n] = kind; a1[as_n] = a_lhs; a2[as_n] = a_rhs; vn[as_n] = vname; ai[as_n] = aidx
        }
        pos = endp
        while (pos <= blen) {
          c = substr(body, pos, 1)
          if (c == " " || c == "\t" || c == "\r" || c == "\n") pos++
          else break
        }
        if (pos <= blen && substr(body, pos, 1) == ";") pos++
      }
      continue
    }
    j = match_kw("type"); if (!j) j = match_kw("import")
    if (j) {
      start_i = i; is_imp = (match_kw("import") > 0)
      while (i <= n && substr(text, i, 1) != ";") i++
      if (i <= n) i++
      if (is_imp) { fn_n++; fns[fn_n] = substr(text, start_i, i - start_i) }
      continue
    }
    snip = substr(text, i, 40); gsub(/\n/, " ", snip)
    err("unsupported top-level near: " snip)
  }
  if (verify_count == 0) {
    printf "OK\ttest\tcheck-only (no verify blocks)\n"
    printf "" > out
    close(out)
    exit 0
  }
  real = 0
  for (k = 1; k <= as_n; k++) if (kinds[k] != "stmt") real++
  if (real == 0) err("verify blocks present but no asserts")
  print "// generated by ooda_verify_pure.sh (pure verify; no Python)" > out
  for (k = 1; k <= fn_n; k++) printf "%s", fns[k] >> out
  print "\npub fn main() {" >> out
  cur = ""
  for (k = 1; k <= as_n; k++) {
    if (vn[k] != cur) { printf "    // verify %s\n", vn[k] >> out; cur = vn[k] }
    if (kinds[k] == "stmt") printf "    %s\n", a1[k] >> out
    else if (kinds[k] == "eq") {
      printf "    if %s != %s {\n", a1[k], a2[k] >> out
      printf "        println(\"FAIL assert_eq %s#%d\");\n", vn[k], ai[k] >> out
      print "        process_exit(1);\n    }" >> out
    } else if (kinds[k] == "ne") {
      printf "    if %s == %s {\n", a1[k], a2[k] >> out
      printf "        println(\"FAIL assert_ne %s#%d\");\n", vn[k], ai[k] >> out
      print "        process_exit(1);\n    }" >> out
    } else {
      printf "    if !(%s) {\n", a1[k] >> out
      printf "        println(\"FAIL assert %s#%d\");\n", vn[k], ai[k] >> out
      print "        process_exit(1);\n    }" >> out
    }
  }
  printf "    println(\"OK verify (%d asserts)\");\n}\n", real >> out
  printf "OK\ttest\tharness %d asserts from %d verify\n", real, verify_count > "/dev/stderr"
  exit 0
}
' /dev/null
rc=$?
set -e
exit "$rc"
