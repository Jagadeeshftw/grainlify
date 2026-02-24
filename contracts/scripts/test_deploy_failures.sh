#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEPLOY_SCRIPT="$SCRIPT_DIR/deploy.sh"

# --- Create a valid dummy WASM file so validation passes ---
FAKE_WASM=/tmp/fake_valid.wasm
echo -n -e "\x00\x61\x73\x6D\x01" > "$FAKE_WASM"  # Minimal WASM magic header

fail() { echo "✘ FAIL: $1"; exit 1; }
pass() { echo "✔ PASS: $1"; }

# --------------------- MOCKING HELPERS ----------------------

MOCK_BIN="$(pwd)/mock_bin"
mkdir -p "$MOCK_BIN"

enable_identity_mock() {
    echo '#!/usr/bin/env bash
if [[ "$1" = "keys" && "$2" = "address" ]]; then
    echo FAKE_ADDRESS
    exit 0
fi
echo "Mock stellar call"
exit 0' > "$MOCK_BIN/stellar"

    chmod +x "$MOCK_BIN/stellar"
    export PATH="$MOCK_BIN:$ORIGINAL_PATH"
}

disable_identity_mock() {
    export PATH="$ORIGINAL_PATH"
}

ORIGINAL_PATH="$PATH"

# --------------------- TEST RUNNER --------------------------

run_expect_fail() {
    desc="$1"
    expected="$2"
    shift 2

    set +e
    output=$("$DEPLOY_SCRIPT" "$@" 2>&1)
    exit_code=$?
    set -e

    if [[ $exit_code -eq 0 ]]; then
        echo "$output"
        fail "$desc (expected failure, got exit 0)"
    fi

    if ! echo "$output" | grep -q "$expected"; then
        echo "$output"
        fail "$desc (expected message '$expected')"
    fi

    pass "$desc"
}

echo "=== Deployment Script Failure Tests ==="

# ------------------------------------------------------------
# 1. Missing WASM file (NO mocking)
# ------------------------------------------------------------
disable_identity_mock
run_expect_fail "Missing WASM file" "No WASM file specified"

# ------------------------------------------------------------
# 2. Invalid WASM path (NO mocking)
# ------------------------------------------------------------
run_expect_fail "Invalid WASM file path" "WASM file not found" "/tmp/this_file_does_not_exist.wasm"

# ------------------------------------------------------------
# 3. Invalid identity should FAIL identity check (NO mocking)
# ------------------------------------------------------------
disable_identity_mock
run_expect_fail "Invalid identity" "Identity not found" "$FAKE_WASM" --identity "ghost_id"

# ------------------------------------------------------------
# 4. Missing CLI dependency (requires identity mock)
# ------------------------------------------------------------
enable_identity_mock
PATH="/usr/bin:/bin" run_expect_fail \
  "Missing soroban CLI" \
  "Neither 'stellar' nor 'soroban' CLI found" \
  "$FAKE_WASM"

echo "All deployment failure tests passed!"