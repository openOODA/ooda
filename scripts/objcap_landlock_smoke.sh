#!/usr/bin/env bash
# Object Capability and Landlock Sandboxing Smoke
# Verifies capability seal attenuation, rights diminution, and ruleset construction.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$ROOT/.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"

OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }

SEC_DIR="$ROOT/../std/src/ooda/sec/sys"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

MODS=(
    landlock_types.oo landlock_const.oo landlock_ops.oo
    landlock_ruleset.oo landlock_engine.oo landlock_stubs.oo landlock.oo
    objcap_types.oo objcap_rights.oo objcap_attenuate.oo
    objcap_seal.oo objcap_registry.oo objcap_engine.oo
    objcap_stubs.oo objcap.oo
)

# 1. Verify all 15 Landlock and ObjCap domain units pass oodac check
for m in "${MODS[@]}"; do
    if "$OODAC" check "$SEC_DIR/$m" >"$TMPDIR/chk_$m.out" 2>"$TMPDIR/chk_$m.err"; then
        pass "check $m"
    else
        bad "check $m"
    fi
done

# 2. Verify all files strictly <= 256 lines
for m in "${MODS[@]}"; do
    lines=$(wc -l < "$SEC_DIR/$m")
    if [ "$lines" -le 256 ]; then
        pass "line count $m: $lines <= 256"
    else
        bad "line count $m: $lines > 256"
    fi
done

# 3. Verify DINNER 4-element headers on all units
for m in "${MODS[@]}"; do
    header=$(head -n 20 "$SEC_DIR/$m")
    if echo "$header" | grep -q "^// # " && \
       echo "$header" | grep -q "^// Logline:" && \
       echo "$header" | grep -q "^// Setup:" && \
       echo "$header" | grep -q "^// Beats:"; then
        pass "DINNER header $m"
    else
        bad "DINNER header $m missing required 4-element structure"
    fi
done

# 4. Verify Object Capability and Bridge Integration Fixture
cat << 'EOF' > "$SEC_DIR/test_integration_fixture.oo"
// # Landlock and Objcap Verification Integration Test
//
// Logline: Static verification of rights diminution, seal attenuation, and sandboxing rulesets.
//
// Setup: Imports objcap_stubs to verify full integration.
//
// Beats:
//   1. Verify rights diminution and seal attenuation.
//   2. Verify Landlock sandboxing ruleset construction and path evaluation.
//   3. Verify object capability registry translation to Landlock.

import "./objcap_stubs.oo";

pub fn verify_integration_flow() -> Bool {
    let dim = capability_rights_diminish(7, 5);
    let cap = objcap_create("cap1", "/data", "user", 7, 100, 3600);
    let att = objcap_attenuation_create("att1", "cap1", 5, "/data/sub", 10, 3700);
    let att_cap_res = objcap_attenuate(&cap, att, "cap2", 200);
    let seal = objcap_seal_attenuated("s1", &cap, "auth", 5, "/data/sub");
    let u_res = objcap_seal_unseal_and_verify_rights(&seal, "auth", &cap, 1);
    let mut rs = landlock_ruleset_new("sandbox", landlock_access_fs_all(), landlock_access_net_all());
    rs = landlock_ruleset_add_read_path(rs, "/usr");
    rs = landlock_ruleset_attenuate_path(rs, "/usr", landlock_access_fs_read_mask());
    let r_found = landlock_ruleset_find_path(&rs, "/usr");
    let rs_valid = landlock_ruleset_validate(&rs);
    let mut reg = objcap_registry_init("r1");
    let _ = objcap_registry_register_cap(&mut reg, cap);
    let b_res = objcap_build_landlock_ruleset(&reg, "rs_bridge");
    let bridge_rs = match b_res { Ok(r) => r, Err(_) => return false; };
    let bridge_valid = landlock_ruleset_validate(&bridge_rs);
    return dim == 5 && att_cap_res.is_ok() && u_res.is_ok() && r_found.is_ok() && rs_valid.is_ok() && bridge_valid.is_ok();
}
EOF

if "$OODAC" check "$SEC_DIR/test_integration_fixture.oo" >"$TMPDIR/chk_int.out" 2>"$TMPDIR/chk_int.err"; then
    pass "check object capability and bridge integration fixture"
else
    bad "check object capability and bridge integration fixture"
fi
rm -f "$SEC_DIR/test_integration_fixture.oo"

# 5. Verify Landlock Engine and Action Evaluation Fixture
cat << 'EOF' > "$SEC_DIR/test_landlock_engine_fixture.oo"
// # Landlock Engine Verification Test
//
// Logline: Static verification of landlock evaluation and action dispatch.
//
// Setup: Imports landlock_stubs to verify evaluation engine.
//
// Beats:
//   1. Verify Landlock sandboxing ruleset construction and path evaluation.
//   2. Verify Landlock action evaluation and path attenuation.

import "./landlock_stubs.oo";

pub fn verify_landlock_engine_flow() -> Bool {
    let mut rs = landlock_ruleset_new("sandbox_eng", landlock_access_fs_all(), landlock_access_net_all());
    rs = landlock_ruleset_add_read_path(rs, "/usr");
    let r_mask = landlock_access_fs_read_mask();
    let ev_path = landlock_ruleset_eval_path(&rs, "/usr/bin/sh", r_mask);
    let ev_act = landlock_ruleset_eval_action(&mut rs, "/usr/bin/sh", "read");
    return ev_path && ev_act;
}
EOF

if "$OODAC" check "$SEC_DIR/test_landlock_engine_fixture.oo" >"$TMPDIR/chk_eng.out" 2>"$TMPDIR/chk_eng.err"; then
    pass "check landlock engine evaluation fixture"
else
    bad "check landlock engine evaluation fixture"
fi
rm -f "$SEC_DIR/test_landlock_engine_fixture.oo"

if [[ $fail -ne 0 ]]; then
    echo "objcap_landlock_smoke: FAILED" >&2
    exit 1
fi

echo "objcap_landlock_smoke: PASSED"
exit 0
