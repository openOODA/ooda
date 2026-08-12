#!/usr/bin/env bash
# job: T4 hygiene / supply-chain debris gate (SPRINT 8.1, 9.x)
# in:  ooda tree (git + source); no compiler required
# out: exit 0 if bak binaries are gitignored, no cap-strip debug scripts,
#      tc_control_cond has no DBG_/DEBUG println, and this script is bash -n clean
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# --- 0) self bash -n (syntax gate) ---
if bash -n "$0"; then
  pass "bash -n hygiene_t4_smoke.sh"
else
  bad "bash -n hygiene_t4_smoke.sh"
fi

# --- 1) oodac/oodac.bak* must not be untracked (must be gitignored if present) ---
# Local prior-tip backups are fine; shipping them as untracked debris is not.
shopt -s nullglob
bak_files=(oodac/oodac.bak oodac/oodac.bak.*)
shopt -u nullglob
if [[ ${#bak_files[@]} -eq 0 ]]; then
  pass "no oodac/oodac.bak* present"
else
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    bak_bad=0
    for f in "${bak_files[@]}"; do
      [[ -e "$f" ]] || continue
      if git ls-files --error-unmatch "$f" >/dev/null 2>&1; then
        bad "tracked bak binary (must not ship): $f"
        bak_bad=1
      elif ! git check-ignore -q "$f" 2>/dev/null; then
        bad "untracked oodac.bak* (gitignore or delete): $f"
        bak_bad=1
      fi
    done
    if [[ $bak_bad -eq 0 ]]; then
      pass "oodac.bak* present and gitignored (${#bak_files[@]} file(s))"
    fi
  else
    # No git: require .gitignore pattern so bak cannot land unignored in CI checkout.
    if ! grep -qE 'oodac\.bak' .gitignore 2>/dev/null; then
      bad "oodac.bak* present but .gitignore lacks oodac.bak pattern"
    else
      pass "oodac.bak* present; .gitignore has bak pattern (no git)"
    fi
  fi
fi

# --- 2) no patch_awk / boot_dbg cap-strip scripts (or documented residual) ---
# Cap-gate strip helpers are supply-chain / debug sludge — not product.
debris_hits=""
while IFS= read -r -d '' f; do
  # report paths relative to ROOT when possible
  rel="${f#"$ROOT"/}"
  debris_hits+="$rel"$'\n'
done < <(find "$ROOT" \
  \( -path '*/.git/*' -o -path '*/dist/*' -o -path '*/target/*' -o -path '*/.agents/*' \) -prune -o \
  -type f \( \
    -name 'patch_awk.sh' -o \
    -name 'patch_awk' -o \
    -name '*boot_dbg*' -o \
    -name '*cap_strip*' -o \
    -name '*strip_cap*' -o \
    -name '*cap-strip*' \
  \) -print0 2>/dev/null)

if [[ -z "${debris_hits//[$'\n']/}" ]]; then
  pass "no patch_awk / boot_dbg / cap-strip scripts"
else
  residual_ok=0
  if grep -rqE 'RESIDUAL.*(patch_awk|boot_dbg|cap.?strip)|ACTIVE:.*RESIDUAL.*(patch_awk|boot_dbg)' \
    bootstrap scripts 2>/dev/null; then
    residual_ok=1
  fi
  if [[ $residual_ok -eq 1 ]]; then
    pass "patch_awk/boot_dbg/cap-strip present but documented residual"
    printf '%s' "$debris_hits" | sed 's/^/  residual: /'
  else
    bad "cap-strip / debug debris scripts present (delete or document residual):"
    printf '%s' "$debris_hits" | sed 's/^/  /' >&2
  fi
fi

# --- 3) tc_control_cond.oo: no DBG_/DEBUG println (ERR\ttype OK) ---
TC="$ROOT/oodac/tc_control_cond.oo"
if [[ ! -f "$TC" ]]; then
  bad "missing $TC"
else
  # Match DBG_ or DEBUG markers used as debug println payloads / labels.
  # Allow legitimate ERR\ttype diagnostics (product fail-closed messages).
  dbg_hits="$(grep -nE 'DBG_|DEBUG' "$TC" 2>/dev/null | grep -vE 'ERR\\ttype' || true)"
  if [[ -n "$dbg_hits" ]]; then
    bad "tc_control_cond.oo has DBG_/DEBUG (strip debug println leak):"
    echo "$dbg_hits" | sed 's/^/  /' >&2
  else
    if grep -qE 'ERR\\ttype|ERR\\t' "$TC"; then
      pass "tc_control_cond.oo: no DBG_/DEBUG println (ERR type OK)"
    else
      pass "tc_control_cond.oo: no DBG_/DEBUG markers"
    fi
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "hygiene_t4_smoke: FAILED" >&2
  exit 1
fi
echo "hygiene_t4_smoke: PASSED"
exit 0
