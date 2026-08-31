#!/usr/bin/env bash
# =============================================================================
# scripts/tests/test_verify_wasm_artifacts.sh
# =============================================================================
# Regression tests for scripts/verify-wasm-artifacts.sh.
# Runs entirely without a Rust toolchain by constructing minimal synthetic
# wasm files and invalid fixtures in a temp directory.
#
# USAGE:
#   bash scripts/tests/test_verify_wasm_artifacts.sh
#
# EXIT CODES:
#   0  — all tests passed
#   1  — one or more tests failed
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERIFIER="$SCRIPT_DIR/../verify-wasm-artifacts.sh"
TMPDIR_BASE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_BASE"' EXIT

PASS=0
FAIL=0

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
pass_test() { PASS=$(( PASS + 1 )); printf "  ✅ PASS  %s\n" "$1"; }
fail_test() { FAIL=$(( FAIL + 1 )); printf "  ❌ FAIL  %s\n" "$1" >&2; }

run_case() {
  local desc="$1"; local expected_exit="$2"; shift 2
  local actual_exit=0
  bash "$VERIFIER" "$@" >/dev/null 2>&1 || actual_exit=$?
  if [[ "$actual_exit" -eq "$expected_exit" ]]; then
    pass_test "$desc"
  else
    fail_test "$desc (expected exit $expected_exit, got $actual_exit)"
  fi
}

# Minimal valid wasm: magic (\x00asm) + version (1 as 4-byte LE)
make_valid_wasm() {
  local path="$1"
  printf '\x00\x61\x73\x6d\x01\x00\x00\x00' > "$path"
}

# ---------------------------------------------------------------------------
# Test fixtures
# ---------------------------------------------------------------------------
DIR_VALID="$TMPDIR_BASE/valid"
mkdir -p "$DIR_VALID"
make_valid_wasm "$DIR_VALID/foo.wasm"
make_valid_wasm "$DIR_VALID/bar.wasm"

DIR_EMPTY_FILE="$TMPDIR_BASE/empty"
mkdir -p "$DIR_EMPTY_FILE"
touch "$DIR_EMPTY_FILE/empty.wasm"

DIR_BAD_MAGIC="$TMPDIR_BASE/bad_magic"
mkdir -p "$DIR_BAD_MAGIC"
printf '\xDE\xAD\xBE\xEF' > "$DIR_BAD_MAGIC/notawasm.wasm"

# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------
echo "Running verify-wasm-artifacts.sh regression tests"
echo ""

# 1. Valid single artifact — should pass
run_case "single valid wasm passes" 0 "$DIR_VALID" foo.wasm

# 2. Multiple valid artifacts — should pass
run_case "multiple valid wasms pass" 0 "$DIR_VALID" foo.wasm bar.wasm

# 3. Missing artifact — should fail
run_case "missing wasm fails" 1 "$DIR_VALID" missing.wasm

# 4. Empty file — should fail
run_case "empty wasm fails" 1 "$DIR_EMPTY_FILE" empty.wasm

# 5. Wrong magic bytes — should fail
run_case "bad magic bytes fails" 1 "$DIR_BAD_MAGIC" notawasm.wasm

# 6. Non-existent directory — should fail
run_case "missing artifact dir fails" 1 "/nonexistent/dir" foo.wasm

# 7. Mix: one valid + one missing — should fail
run_case "partial miss fails (one valid, one missing)" 1 "$DIR_VALID" foo.wasm ghost.wasm

# 8. No arguments — should fail
run_case "no args fails" 1

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "Results: $PASS passed, $FAIL failed"
if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0
