#!/usr/bin/env bash
# job: source-level immune rail — scan_fn_names stays cheap (no tokenize)
# in:  oodac/check_scan.oo + oodac/check_collect.oo (source only; no host)
# out: exit 0 iff scan_fn_names has no tokenize_lines and has comment/string skip
#
# Dual-repro (does not need a rebuilt host):
#   1) scan_fn_names body must not call tokenize_lines  (1.8GiB RSS regress)
#   2) comment-skip or string-skip exists near scan     ("//" / "\"" handling)
# Later: a tiny .oo fixture can unit-test the walker; this rail is grep-only.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCAN="$ROOT/oodac/check_scan.oo"
COLLECT="$ROOT/oodac/check_collect.oo"

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

if bash -n "$0"; then
  pass "bash -n scan_fn_light_smoke.sh"
else
  bad "bash -n scan_fn_light_smoke.sh"
fi

files=()
[[ -f "$SCAN" ]] && files+=("$SCAN")
[[ -f "$COLLECT" ]] && files+=("$COLLECT")

if [[ ${#files[@]} -eq 0 ]]; then
  bad "neither check_scan.oo nor check_collect.oo exists"
  echo "scan_fn_light_smoke: FAILED" >&2
  exit 1
fi

# Extract top-level scan_fn_names from a .oo file (empty if absent).
extract_scan_fn() {
  awk '
    /^pub fn scan_fn_names\(/ || /^fn scan_fn_names\(/ { p=1; print; next }
    p && (/^pub fn / || /^fn /) { exit }
    p { print }
  ' "$1"
}

defined=0
for f in "${files[@]}"; do
  rel="${f#"$ROOT"/}"
  body="$(extract_scan_fn "$f" || true)"
  if [[ -z "$body" ]]; then
    pass "$rel: no scan_fn_names def (ok if imported)"
    continue
  fi
  defined=1
  if grep -q 'tokenize_lines' <<<"$body"; then
    bad "$rel: scan_fn_names contains tokenize_lines (1.8GiB name-scan regress)"
    grep -n 'tokenize_lines' <<<"$body" >&2 || true
  else
    pass "$rel: scan_fn_names has no tokenize_lines"
  fi

  # comment-skip or string-skip near scan (helpers in same file, or literals)
  if grep -qE 'scan_skip_line_comment|scan_skip_string' "$f" \
    || grep -qF '"//"' "$f" \
    || grep -qF '"\""' "$f"; then
    pass "$rel: comment-skip or string-skip present near scan"
  else
    bad "$rel: scan_fn_names missing comment-skip / string-skip (\"//\" or \"\\\"\")"
  fi
done

if [[ $defined -eq 0 ]]; then
  bad "scan_fn_names not defined in check_scan.oo / check_collect.oo"
fi

if [[ $fail -ne 0 ]]; then
  echo "scan_fn_light_smoke: FAILED" >&2
  exit 1
fi
echo "scan_fn_light_smoke: ALL OK"
exit 0
