#!/usr/bin/env bash
# =============================================================================
# scripts/tests/test_check_wasm_budgets.sh
# =============================================================================
# Regression tests for scripts/check-wasm-budgets.sh.
#
# Runs without a Rust toolchain: it builds a throwaway budget file and synthetic
# wasm files in a temp directory, then exercises the checker through the
# WASM_BUDGETS_FILE / WASM_BUDGETS_ROOT overrides.
#
# USAGE:
#   bash scripts/tests/test_check_wasm_budgets.sh
#
# EXIT CODES:
#   0  — all tests passed
#   1  — one or more tests failed
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECKER="$SCRIPT_DIR/../check-wasm-budgets.sh"
TMP_BASE="$(mktemp -d)"
trap 'rm -rf "$TMP_BASE"' EXIT

PASS=0
FAIL=0

pass_test() { PASS=$(( PASS + 1 )); printf "  ✅ PASS  %s\n" "$1"; }
fail_test() { FAIL=$(( FAIL + 1 )); printf "  ❌ FAIL  %s\n" "$1" >&2; }

# make_wasm <path> <bytes>
make_wasm() {
  local path="$1" bytes="$2"
  mkdir -p "$(dirname "$path")"
  head -c "$bytes" /dev/zero > "$path"
}

# run_checker <root> -> sets OUT and RC
run_checker() {
  local root="$1"
  local rc=0
  OUT=$(WASM_BUDGETS_ROOT="$root" WASM_BUDGETS_FILE="$root/.github/wasm-budgets.json" \
        bash "$CHECKER" 2>&1) || rc=$?
  RC=$rc
}

# write_budget <root> ; fixed schema used by every case:
#   ceiling 1000, tolerance 10%
#   small  baseline 100   (limit 110)      fake 100 -> pass
#   big    baseline 100   (growth limit 110) fake 200 -> growth fail
#   huge   baseline 10000 (limit = ceiling 1000) fake 1500 -> ceiling fail
#   legacy baseline 100, target wasm32v1-none, artifact custom.wasm
write_budget() {
  local root="$1"
  mkdir -p "$root/.github"
  cat > "$root/.github/wasm-budgets.json" <<'JSON'
{
  "absolute_ceiling_bytes": 1000,
  "tolerance_percent": 10,
  "budgets": {
    "small": {
      "package": "small",
      "target_dir": "a",
      "baseline_bytes": 100
    },
    "big": {
      "package": "big",
      "target_dir": "b",
      "baseline_bytes": 100
    },
    "huge": {
      "package": "huge",
      "target_dir": "c",
      "baseline_bytes": 10000
    },
    "legacy": {
      "package": "legacy-pkg",
      "target_dir": "soroban",
      "target": "wasm32v1-none",
      "artifact": "custom.wasm",
      "baseline_bytes": 100
    }
  }
}
JSON
}

echo "Running check-wasm-budgets.sh regression tests"
echo ""

# ── Case 1: all artifacts within budget ────────────────────────────────────
ROOT1="$TMP_BASE/pass"; write_budget "$ROOT1"
make_wasm "$ROOT1/a/target/wasm32-unknown-unknown/release/small.wasm" 100
make_wasm "$ROOT1/b/target/wasm32-unknown-unknown/release/big.wasm" 100
make_wasm "$ROOT1/c/target/wasm32-unknown-unknown/release/huge.wasm" 100
make_wasm "$ROOT1/soroban/target/wasm32v1-none/release/custom.wasm" 100
run_checker "$ROOT1"
if [[ "$RC" -eq 0 ]]; then pass_test "all within budget exits 0"; else fail_test "all within budget exits 0 (got $RC)"; fi

# ── Case 2: growth beyond tolerance but below ceiling ──────────────────────
ROOT2="$TMP_BASE/growth"; write_budget "$ROOT2"
make_wasm "$ROOT2/a/target/wasm32-unknown-unknown/release/small.wasm" 100
make_wasm "$ROOT2/b/target/wasm32-unknown-unknown/release/big.wasm" 200
make_wasm "$ROOT2/c/target/wasm32-unknown-unknown/release/huge.wasm" 100
make_wasm "$ROOT2/soroban/target/wasm32v1-none/release/custom.wasm" 100
run_checker "$ROOT2"
if [[ "$RC" -eq 1 ]] && grep -q "FAIL (growth)" <<<"$OUT"; then pass_test "growth over tolerance fails with growth reason"; else fail_test "growth over tolerance fails with growth reason (rc=$RC)"; fi

# ── Case 3: crossing the absolute ceiling ──────────────────────────────────
ROOT3="$TMP_BASE/ceiling"; write_budget "$ROOT3"
make_wasm "$ROOT3/a/target/wasm32-unknown-unknown/release/small.wasm" 100
make_wasm "$ROOT3/b/target/wasm32-unknown-unknown/release/big.wasm" 100
make_wasm "$ROOT3/c/target/wasm32-unknown-unknown/release/huge.wasm" 1500
make_wasm "$ROOT3/soroban/target/wasm32v1-none/release/custom.wasm" 100
run_checker "$ROOT3"
if [[ "$RC" -eq 1 ]] && grep -q "FAIL (ceiling)" <<<"$OUT"; then pass_test "over absolute ceiling fails with ceiling reason"; else fail_test "over absolute ceiling fails with ceiling reason (rc=$RC)"; fi

# ── Case 4: custom target/artifact path is honoured ────────────────────────
ROOT4="$TMP_BASE/artifact"; write_budget "$ROOT4"
make_wasm "$ROOT4/a/target/wasm32-unknown-unknown/release/small.wasm" 100
make_wasm "$ROOT4/b/target/wasm32-unknown-unknown/release/big.wasm" 100
make_wasm "$ROOT4/c/target/wasm32-unknown-unknown/release/huge.wasm" 100
# deliberately NOT creating custom.wasm: the entry must be reported as not built
run_checker "$ROOT4"
if [[ "$RC" -eq 0 ]] && grep -q "legacy" <<<"$OUT" && grep -q "not found" <<<"$OUT"; then
  pass_test "custom target/artifact with missing file is skipped"
else
  fail_test "custom target/artifact with missing file is skipped (rc=$RC)"
fi

# ── Case 5: zero built artifacts is a warning, not a hard failure ──────────
ROOT5="$TMP_BASE/none"; write_budget "$ROOT5"
run_checker "$ROOT5"
if [[ "$RC" -eq 0 ]] && grep -q "No wasm files found" <<<"$OUT"; then pass_test "no artifacts exits 0 with warning"; else fail_test "no artifacts exits 0 with warning (rc=$RC)"; fi

# ── Case 6: malformed baseline fails closed ────────────────────────────────
ROOT6="$TMP_BASE/malformed"; write_budget "$ROOT6"
python3 - "$ROOT6/.github/wasm-budgets.json" <<'PY'
import json,sys
p=sys.argv[1]; d=json.load(open(p)); d['budgets']['small']['baseline_bytes']='not-a-number'
json.dump(d, open(p,'w'))
PY
make_wasm "$ROOT6/a/target/wasm32-unknown-unknown/release/small.wasm" 100
run_checker "$ROOT6"
if [[ "$RC" -eq 1 ]]; then pass_test "malformed baseline fails closed"; else fail_test "malformed baseline fails closed (rc=$RC)"; fi

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ "$FAIL" -gt 0 ]] && exit 1
exit 0
