#!/usr/bin/env bash
# F20 std inventory smoke — count, existence, line-count invariants, phantom guard.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STD="$ROOT/std"
fail=0; pass() { echo "OK $*"; }; bad() { echo "FAIL $*" >&2; fail=1; }

ALLOW="archive/gzip.oo|archive/tar.oo|archive/zip.oo|byte.oo|core/crypto.oo|core/json.oo|core/option.oo|core/result.oo|core/str.oo|hash/md5.oo|hash/sha1.oo|hash/sha256.oo|markup/json_schema.oo|markup/toml.oo|markup/xml.oo|markup/yaml.oo|math.oo|math/tensor.oo|option.oo|os/async.oo|os/cmd.oo|os/env.oo|os/fs.oo|os/gpu.oo|os/net.oo|os/process.oo|os/python.oo|os/sync.oo|os/thread.oo|result.oo|semver.oo|src/net/dns.oo|str.oo|test.oo"
IFS='|' read -r -a WANT <<<"$ALLOW"

# 1) count = 34
set +e; n=$(find "$STD" -name '*.oo' | wc -l); set -e
[[ "$n" -eq 34 ]] && pass "count=34" || bad "count=$n (expected 34)"

# 2) every allowlisted file exists
for rel in "${WANT[@]}"; do [[ -f "$STD/$rel" ]] || bad "missing $rel"; done
[[ $fail -eq 0 ]] && pass "all 34 files exist"

# 3) line-count samples (f20 invariants + new env.oo)
chk() { local r="$1" w="$2" g; g=$(wc -l <"$STD/$r" | tr -d ' '); [[ "$g" == "$w" ]] && pass "lines $r=$w" || bad "lines $r got=$g want=$w"; }
chk core/crypto.oo 39; chk os/fs.oo 83; chk hash/sha256.oo 9; chk os/env.oo 7; chk os/cmd.oo 55; chk os/process.oo 61; chk str.oo 92; chk test.oo 67

# 4) phantom guard: .oo on disk not in allowlist → FAIL
set +e
phantom=$(find "$STD" -name '*.oo' -printf '%P\n' | sort | comm -23 - <(printf '%s\n' "${WANT[@]}" | sort))
set -e
if [[ -z "$phantom" ]]; then pass "no phantom files"; else bad "phantom: $phantom"; fi

# verdict
if [[ $fail -ne 0 ]]; then echo "harness_f20_inventory_smoke: FAILED" >&2; exit 1; fi
echo "harness_f20_inventory_smoke: ALL OK"
exit 0
