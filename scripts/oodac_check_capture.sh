#!/bin/sh
# helper for --json-errors re-run (argv only; no complex quoting)
# $1=source $2=out $3=ecfile
EM="${OODAC_BIN:-}"
if [ -z "$EM" ] || [ ! -x "$EM" ]; then
  if [ -x ./oodac/oodac ]; then EM=./oodac/oodac
  elif [ -x oodac/oodac ]; then EM=oodac/oodac
  else echo ERR_NO_OODAC >"$2"; echo 1 >"$3"; exit 0; fi
fi
set +e
"$EM" check "$1" >"$2" 2>&1
echo $? >"$3"
exit 0
