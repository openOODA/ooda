#!/usr/bin/env bash
# job: ooda product CLI migrate backend — FAIL-CLOSED residual
# in:  args (ignored for rewrite; only diagnostics)
# out: stderr honesty + non-zero exit (RULES §1.3)
#
# SPRINT Issue #14: previous implementation ran
#   sed -i 's/\blet X =/let mut X =/g'
# which silently destroyed immutability. Disabled until a real AST codemod
# lands (ooda fix / edition engine). Do not re-enable soft sed.
set -euo pipefail

json_mode=""
for arg in "$@"; do
  if [[ "$arg" == "--json" ]]; then
    json_mode=1
  fi
done

msg='ooda migrate is disabled (SPRINT Issue #14). Prior sed let→let mut was unsafe. Use ooda fix / manual edit; back up first if you rewrite yourself.'

if [[ -n "$json_mode" ]]; then
  echo "{\"error\":\"E_MIGRATE_DISABLED\",\"msg\":\"$msg\",\"changed\":false,\"let_mut_fixes\":0}"
else
  echo "ERR	migrate	$msg" >&2
fi
exit 1
