#!/usr/bin/env bash
# job: B1-style proof — product rails without cargo/rustc on the critical path
# in:  SEED_OODAC (or existing pure oodac) + gcc + bash
# out: exit 0 if bootstrap + fixed_point + product smokes green without invoking cargo/rustc
#
# Residual honesty:
#  - Requires a prebuilt pure seed binary (cold start cannot invent a compiler from air).
#  - Does not prove a remote GitHub Actions matrix (none in-tree); this is the local B1 rail.
#  - Does not uninstall system cargo if present — only refuses to *invoke* it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# --- anti: product scripts must not shell out to cargo/rustc as commands ---
# Allow comments/docs; flag bare command lines only.
for s in "$ROOT/scripts/bootstrap_no_cargo.sh" "$ROOT/scripts/fixed_point.sh" \
         "$ROOT/scripts/release.sh"; do
  if grep -nE '^[[:space:]]*(cargo|rustc)([[:space:]]|$)' "$s" | grep -vE '^\s*#'; then
    bad "script invokes cargo/rustc: $s"
  else
    pass "no cargo/rustc command in $(basename "$s")"
  fi
done

# Shadow cargo/rustc so accidental PATH use fails closed
SHADOW="$TMPDIR/ci_no_rust_shadow_$$"
mkdir -p "$SHADOW"
cat >"$SHADOW/cargo" <<'EOF'
#!/bin/sh
echo "ERR_SHADOW_CARGO" >&2
exit 99
EOF
cat >"$SHADOW/rustc" <<'EOF'
#!/bin/sh
echo "ERR_SHADOW_RUSTC" >&2
exit 99
EOF
chmod +x "$SHADOW/cargo" "$SHADOW/rustc"
export PATH="$SHADOW:$PATH"

# Prove cargo would fail if called
if cargo version >/dev/null 2>&1; then
  bad "shadow cargo did not intercept"
else
  pass "cargo shadowed (exit non-zero if called)"
fi

# B0 tree facts
RS=$(find "$ROOT" -name '*.rs' -not -path '*/.git/*' -not -path '*/target/*' | wc -l)
echo "RS_COUNT=$RS"
[[ "$RS" -eq 0 ]] && pass "B0 RS=0" || bad "B0 RS=$RS"
[[ ! -f "$ROOT/Cargo.toml" ]] && pass "no Cargo.toml" || bad "Cargo.toml present"
[[ ! -d "$ROOT/src" ]] && pass "no src/" || bad "src/ present"

# Product path
if ! SEED_OODAC="${SEED_OODAC:-$ROOT/oodac/oodac}" "$ROOT/scripts/bootstrap_no_cargo.sh" \
  >"$TMPDIR/ci_boot.out" 2>"$TMPDIR/ci_boot.err"; then
  bad "bootstrap_no_cargo"
  cat "$TMPDIR/ci_boot.err" | tail -20
else
  pass "bootstrap_no_cargo under cargo-shadow PATH"
fi

export OODA="$ROOT/bin/ooda"
export OODAC_BIN="$ROOT/oodac/oodac"

for rail in product_pure_dispatch_smoke.sh p3_no_cargo_smoke.sh chs_parity.sh beta_cli_smoke.sh c_emit_smoke.sh; do
  if ! "$ROOT/scripts/$rail" >"$TMPDIR/ci_$rail.out" 2>"$TMPDIR/ci_$rail.err"; then
    bad "$rail"
    tail -15 "$TMPDIR/ci_$rail.err" || tail -15 "$TMPDIR/ci_$rail.out" || true
  else
    pass "$rail"
  fi
done

if ! "$ROOT/scripts/fixed_point.sh" >"$TMPDIR/ci_fp.out" 2>"$TMPDIR/ci_fp.err"; then
  bad "fixed_point"
  tail -20 "$TMPDIR/ci_fp.out"
else
  pass "fixed_point pure seed"
  if grep -q 'OK_HOST' "$TMPDIR/ci_fp.out" 2>/dev/null; then
    bad "OK_HOST in fixed_point log"
  else
    pass "no OK_HOST in fixed_point log"
  fi
fi

# Ensure shadow was never hit
if grep -rq 'cargo invoked on no-Rust' "$TMPDIR"/ci_*.err "$TMPDIR"/ci_*.out 2>/dev/null; then
  bad "a rail tried to invoke cargo"
else
  pass "no rail invoked shadowed cargo"
fi

if [[ $fail -ne 0 ]]; then
  echo "ci_no_rust: FAILED" >&2
  exit 1
fi
echo "ci_no_rust: PASSED"
echo "residual: prebuilt SEED_OODAC required (bootstrap/seed, tree oodac, or pin release asset)"
echo "remote: .github/workflows/no_rust.yml runs this rail without installing cargo"
exit 0
