#!/usr/bin/env bash
# Why not .oo: this starts the oodac binary and checks exit codes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${OODAC_BIN:-/tmp/oodac_ts}"
if [[ ! -x "$BIN" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then BIN="$ROOT/oodac/oodac"
  else echo "ERR_NO_OODAC" >&2; exit 1; fi
fi
cd "$ROOT"
rm -rf .ooda-cache/check

must_ok() {
  if ! "$BIN" check "$1" >/dev/null; then
    echo "ERR typestate_smoke: $1 should pass" >&2
    exit 1
  fi
}
must_bad() {
  if "$BIN" check "$1" >/dev/null 2>&1; then
    echo "ERR typestate_smoke: $1 should refuse" >&2
    exit 1
  fi
}

must_ok fixtures/typestate_ok.oo
must_ok fixtures/typestate_marker.oo
must_ok fixtures/typestate_if_join_ok.oo
must_ok fixtures/typestate_while_ok.oo
must_ok fixtures/typestate_if_return_ok.oo
must_ok fixtures/typestate_if_return_join.oo
must_ok fixtures/typestate_else_if_ok.oo
must_ok fixtures/typestate_nested_if_return.oo
must_ok fixtures/typestate_let_connect.oo
must_ok fixtures/typestate_callee_compose_ok.oo
must_ok fixtures/typestate_callee_return_ok.oo
must_ok fixtures/typestate_callee_empty_ok.oo
must_ok fixtures/typestate_callee_process_exit_ok.oo
must_ok fixtures/typestate_alias_ok.oo
must_ok fixtures/typestate_alias_assign.oo
must_ok fixtures/typestate_alias_ok2.oo
must_ok fixtures/typestate_match_join_ok.oo
must_ok fixtures/typestate_match_return_ok.oo
must_ok fixtures/typestate_end_ok.oo
must_ok fixtures/typestate_two_types.oo
must_ok fixtures/typestate_method_ok.oo
must_ok fixtures/typestate_cross_ok.oo
must_bad fixtures/typestate_bad_order.oo
must_bad fixtures/typestate_end_bad.oo
must_bad fixtures/typestate_alias_bad.oo
must_bad fixtures/typestate_alias_two_socks.oo
must_bad fixtures/typestate_callee_send_bad.oo
must_bad fixtures/typestate_callee_compose_bad.oo
must_bad fixtures/typestate_callee_ctor_only_bad.oo
must_bad fixtures/typestate_callee_dead_call_bad.oo
must_bad fixtures/typestate_callee_early_return_bad.oo
must_bad fixtures/typestate_callee_missing_param_bad.oo
must_bad fixtures/typestate_callee_other_obj_bad.oo
must_bad fixtures/typestate_callee_shadow_bad.oo
must_bad fixtures/typestate_callee_method_dead_bad.oo
must_bad fixtures/typestate_if_blocked.oo
must_bad fixtures/typestate_if_join_bad.oo
must_bad fixtures/typestate_while_bad.oo
must_bad fixtures/typestate_field_arg.oo
must_bad fixtures/typestate_match_blocked.oo
must_bad fixtures/typestate_while_cond_bad.oo
must_bad fixtures/typestate_fn_no_arrow.oo
must_bad fixtures/typestate_if_cond_bad.oo
must_bad fixtures/typestate_return_send.oo
echo "typestate_check_smoke: ALL OK"
