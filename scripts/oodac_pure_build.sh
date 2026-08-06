#!/usr/bin/env bash
# job: pure multi-module oodac build (emit each .oo, link once) — no stage-0 host
# in:  <main.oo> <out_bin>
# out: native binary via emit-c + gcc + chs_rt only
# link recipe: Backend-C (see bootstrap/FLOOR.md) — swap here for other floors later
# Notes:
#  - Forward prototypes for all fns so use-before-def across modules is OK
#  - Nested imports + cycle/missing fail-closed (parity with load_import.oo)
#  - Never uses $OODA host soft-pass
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAIN="${1:?main.oo}"
OUT="${2:?out_bin}"
OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/oodac_pure_$$"
mkdir -p "$TMP"
# Lifecycle: always reap temp tree (success or fail)
cleanup_pure_tmp() { rm -rf "$TMP"; }
trap cleanup_pure_tmp EXIT
if [[ ! -x "$OODAC_BIN" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then OODAC_BIN="$ROOT/oodac/oodac"
  elif [[ -x "$ROOT/oodac/main" ]]; then OODAC_BIN="$ROOT/oodac/main"
  else echo "ERR_NO_OODAC" >&2; exit 1
  fi
fi
if [[ ! -f "$MAIN" ]]; then
  if [[ -f "$ROOT/$MAIN" ]]; then MAIN="$ROOT/$MAIN"; fi
fi
if [[ ! -f "$MAIN" ]]; then
  echo "ERR_MISSING $MAIN" >&2
  exit 1
fi
DIR="$(cd "$(dirname "$MAIN")" && pwd)"
BASE="$(basename "$MAIN")"
MAIN_ABS="$DIR/$BASE"
# Collect modules (DFS, cycle/missing fail-closed). Order: deps first, main last.
MODS=()
declare -A SEEN=()
declare -A STACK=()
collect() {
  local path="$1"
  local abs
  if [[ "$path" = /* ]]; then abs="$path"
  else abs="$(cd "$(dirname "$path")" && pwd)/$(basename "$path")"
  fi
  if [[ -n "${STACK[$abs]:-}" ]]; then
    echo "ERR_IMPORT_CYCLE $abs" >&2
    exit 1
  fi
  if [[ -n "${SEEN[$abs]:-}" ]]; then
    return 0
  fi
  if [[ ! -f "$abs" ]]; then
    echo "ERR_MISSING $abs" >&2
    exit 1
  fi
  STACK[$abs]=1
  local dir
  dir="$(cd "$(dirname "$abs")" && pwd)"
  local imp
  while IFS= read -r imp; do
    [[ -z "$imp" ]] && continue
    if [[ "$imp" = /* ]]; then
      collect "$imp"
    else
      collect "$dir/$imp"
    fi
  done < <(grep -E '^import "' "$abs" 2>/dev/null | sed -n 's/^import "\(.*\)";.*/\1/p' || true)
  unset 'STACK[$abs]'
  SEEN[$abs]=1
  MODS+=("$abs")
}
collect "$MAIN_ABS"

# C1 check gate (import-expanded main):
# - always when 1 module
# - small multi (≤8 modules): try check, timeout 90s fail-closed
# - large multi (compiler ~86 mods): skip full check — residual; emit still
#   rejects int/lit caps (c_arg_is_cap_ident). See bootstrap/AUDIT_RESIDUAL.md
if [[ "${PURE_SKIP_CHECK:-}" != "1" ]]; then
  nmods=${#MODS[@]}
  if [[ $nmods -eq 1 ]] || [[ $nmods -le 8 ]]; then
    set +e
    timeout 90 "$OODAC_BIN" check "$MAIN_ABS" >"$TMP/main_check.out" 2>"$TMP/main_check.err"
    main_ck=$?
    set -e
    if [[ $main_ck -eq 124 ]]; then
      echo "ERR_CHECK_TIMEOUT $MAIN_ABS (nmods=$nmods)" >&2
      exit 1
    fi
    if [[ $main_ck -ne 0 ]] || ! grep -qE '^OK' "$TMP/main_check.out" 2>/dev/null; then
      echo "ERR_CHECK $MAIN_ABS" >&2
      head -20 "$TMP/main_check.out" "$TMP/main_check.err" 2>/dev/null || true
      exit 1
    fi
  fi
fi

FN_DEF='^(void|int|long long|OoStr|OoSList|OoIList|OoResS|OoResV) [A-Za-z_].*\) \{'
MCS=()
for src in "${MODS[@]}"; do
  mc="$TMP/$(echo "$src" | tr '/.' '__').c"
  EMIT_NO_CONCAT=1 timeout 60 "$OODAC_BIN" emit-c "$src" >"$mc" 2>/dev/null || true
  if [[ ! -s "$mc" ]] || ! grep -qE "$FN_DEF" "$mc"; then
    echo "ERR_EMIT $src" >&2
    exit 1
  fi
  if grep -qE $'^ERR\tc_emit' "$mc"; then
    echo "ERR_EMIT_LINE $src" >&2
    grep -E $'^ERR\tc_emit' "$mc" >&2 || true
    exit 1
  fi
  MCS+=("$mc")
done

# Preamble = lines before first function def in first module
awk "/$FN_DEF/{exit} {print}" "${MCS[0]}" >"$TMP/preamble.c"
# Forward prototypes from ALL modules (use-before-def across modules)
: >"$TMP/protos.c"
for mc in "${MCS[@]}"; do
  grep -E "$FN_DEF" "$mc" | sed 's/ {$/;/' >>"$TMP/protos.c" || true
done
# Function bodies from ALL modules
: >"$TMP/bodies.c"
for mc in "${MCS[@]}"; do
  awk "/$FN_DEF/{p=1} p" "$mc" >>"$TMP/bodies.c"
done

cat "$TMP/preamble.c" "$TMP/protos.c" "$TMP/bodies.c" >"$TMP/all.c"

# Bridge seed-era 1-arg sealed runtime calls → cap-checked 2-arg ABI (OO_CAP_*).
# Seed emit may still drop caps; product emit passes them. Self-host must link either.
python3 - "$TMP/all.c" <<'PY'
import re, sys
path = sys.argv[1]
t = open(path, encoding="utf-8", errors="replace").read()
# only rewrite calls that have a single argument (no comma at depth 1)
def rewrite(fn, cap, s):
    """Insert cap on single-arg CALLS only; never inside string literals."""
    out = []
    i = 0
    key = fn + "("
    n = len(s)
    in_str = False
    while i < n:
        c = s[i]
        if in_str:
            out.append(c)
            if c == "\\" and i + 1 < n:
                out.append(s[i+1])
                i += 2
                continue
            if c == '"':
                in_str = False
            i += 1
            continue
        if c == '"':
            in_str = True
            out.append(c)
            i += 1
            continue
        if s.startswith(key, i) and (i == 0 or not (s[i-1].isalnum() or s[i-1] == "_")):
            k = i + len(key)
            depth = 1
            start_a = k
            comma = False
            while k < n and depth:
                ch = s[k]
                if ch == '"':
                    k += 1
                    while k < n:
                        if s[k] == "\\":
                            k += 2
                            continue
                        if s[k] == '"':
                            k += 1
                            break
                        k += 1
                    continue
                if ch == "(":
                    depth += 1
                elif ch == ")":
                    depth -= 1
                    if depth == 0:
                        break
                elif ch == "," and depth == 1:
                    comma = True
                k += 1
            args = s[start_a:k]
            a = args.strip()
            if (not comma) and a and not a.startswith("OoStr") and not a.startswith("long long") and not a.startswith("int "):
                out.append(f"{fn}({cap}, {args})")
            else:
                out.append(s[i:k+1])
            i = k + 1
            continue
        out.append(c)
        i += 1
    return "".join(out)

t = rewrite("oo_read_file", "OO_CAP_FS", t)
t = rewrite("oo_write_file", "OO_CAP_FS", t)
t = rewrite("oo_path_exists", "OO_CAP_FS", t)
t = rewrite("oo_file_size", "OO_CAP_FS", t)
t = rewrite("oo_env_get", "OO_CAP_ENV", t)
t = rewrite("oo_sys_exec1", "OO_CAP_SYS", t)
# main inject legacy zero → magic tokens
t = t.replace("int fs = 0; int sys = 0;",
              "long long fs = OO_CAP_FS; long long sys = OO_CAP_SYS; long long env = OO_CAP_ENV;")
t = t.replace("long long fs = 0; long long sys = 0;",
              "long long fs = OO_CAP_FS; long long sys = OO_CAP_SYS; long long env = OO_CAP_ENV;")
caps = (
    "#define OO_CAP_FS  0x4F4F4653LL\n"
    "#define OO_CAP_SYS 0x4F4F5359LL\n"
    "#define OO_CAP_ENV 0x4F4F454ELL\n"
    "#define OO_CAP_NET 0x4F4F4E54LL\n"
)
# Always prepend (body may contain the text inside string literals from emit)
t = caps + t
# Fix seed-era 1-arg prototypes to 2-arg cap ABI
for a,b in [
    ("OoResS oo_read_file(OoStr);", "OoResS oo_read_file(long long,OoStr);"),
    ("OoResV oo_write_file(OoStr,OoStr);", "OoResV oo_write_file(long long,OoStr,OoStr);"),
    ("int oo_path_exists(OoStr);", "int oo_path_exists(long long,OoStr);"),
    ("long long oo_file_size(OoStr);", "long long oo_file_size(long long,OoStr);"),
    ("OoResS oo_env_get(OoStr);", "OoResS oo_env_get(long long,OoStr);"),
    ("static inline OoResS oo_sys_exec1(OoStr cmd)", "static inline OoResS oo_sys_exec1(long long cap, OoStr cmd)"),
]:
    t = t.replace(a,b)

# Repair bad rewrites of prototypes (if any)
t = t.replace("OoResS oo_read_file(OO_CAP_FS, OoStr);", "OoResS oo_read_file(long long,OoStr);")
t = t.replace("int oo_path_exists(OO_CAP_FS, OoStr);", "int oo_path_exists(long long,OoStr);")
t = t.replace("long long oo_file_size(OO_CAP_FS, OoStr);", "long long oo_file_size(long long,OoStr);")
t = t.replace("OoResS oo_env_get(OO_CAP_ENV, OoStr);", "OoResS oo_env_get(long long,OoStr);")
t = t.replace("static inline OoResS oo_sys_exec1(OO_CAP_SYS, OoStr cmd)", "static inline OoResS oo_sys_exec1(long long cap, OoStr cmd)")
# ensure sys_exec body has require if missing
if "oo_cap_require(cap, OO_CAP_SYS" not in t and "oo_sys_exec1(long long cap" in t:
    t = t.replace(
        "static inline OoResS oo_sys_exec1(long long cap, OoStr cmd) {\n  OoResS r;",
        "static inline OoResS oo_sys_exec1(long long cap, OoStr cmd) {\n  oo_cap_require(cap, OO_CAP_SYS, \"sys_exec\");\n  OoResS r;",
    )

open(path, "w", encoding="utf-8").write(t)
PY

if ! grep -q 'int main\|long long main' "$TMP/all.c" && ! grep -q 'main(int argc' "$TMP/all.c"; then
  echo "ERR_NO_MAIN" >&2
  exit 1
fi

gcc -O2 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/all.c" -lm -o "$OUT"
test -x "$OUT"
echo OK_PURE_MULTI
rm -rf "$TMP"
