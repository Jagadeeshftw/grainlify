#!/usr/bin/env bash
# ==============================================================================
# Grainlify - Regression Test Suite for upgrade_contract.sh
# ==============================================================================
# Validates input checking, artifact validation, checksum output, dry-run mode,
# and write confirmation behavior.
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TARGET_SCRIPT="$SCRIPT_DIR/upgrade_contract.sh"

# Test fixtures
TMP_DIR="$(mktemp -d -t grainlify_upgrade_test_XXXXXX)"
VALID_WASM="$TMP_DIR/valid_contract.wasm"
INVALID_WASM="$TMP_DIR/invalid_contract.wasm"
EMPTY_WASM="$TMP_DIR/empty_contract.wasm"

# Setup fixtures
echo -n -e "\x00\x61\x73\x6D\x01\x00\x00\x00" > "$VALID_WASM"
echo -n "THIS_IS_NOT_A_WASM_FILE" > "$INVALID_WASM"
touch "$EMPTY_WASM"

VALID_CONTRACT_ID="CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
INVALID_CONTRACT_ID_PREFIX="BAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
INVALID_CONTRACT_ID_LENGTH="CAAAAAA"

TESTS_PASSED=0
TESTS_FAILED=0

# Formatting
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    GREEN=''
    RED=''
    BLUE=''
    NC=''
fi

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

log_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

assert_fail() {
    local desc="$1"
    local expected_pattern="$2"
    shift 2

    log_test "$desc"
    set +e
    local output
    output=$("$TARGET_SCRIPT" "$@" 2>&1)
    local exit_code=$?
    set -e

    if [ "$exit_code" -eq 0 ]; then
        echo -e "${RED}✘ FAIL${NC}: Expected failure but command succeeded (exit code 0)"
        echo "Output was:"
        echo "$output"
        ((TESTS_FAILED++))
        return 1
    fi

    if [[ -n "$expected_pattern" ]] && ! echo "$output" | grep -Eq "$expected_pattern"; then
        echo -e "${RED}✘ FAIL${NC}: Output did not match pattern: '$expected_pattern'"
        echo "Output was:"
        echo "$output"
        ((TESTS_FAILED++))
        return 1
    fi

    echo -e "${GREEN}✔ PASS${NC}: $desc"
    ((TESTS_PASSED++))
    return 0
}

assert_success() {
    local desc="$1"
    local expected_pattern="$2"
    shift 2

    log_test "$desc"
    set +e
    local output
    output=$("$TARGET_SCRIPT" "$@" 2>&1)
    local exit_code=$?
    set -e

    if [ "$exit_code" -ne 0 ]; then
        echo -e "${RED}✘ FAIL${NC}: Expected success but command failed (exit code $exit_code)"
        echo "Output was:"
        echo "$output"
        ((TESTS_FAILED++))
        return 1
    fi

    if [[ -n "$expected_pattern" ]] && ! echo "$output" | grep -Eq "$expected_pattern"; then
        echo -e "${RED}✘ FAIL${NC}: Output did not match pattern: '$expected_pattern'"
        echo "Output was:"
        echo "$output"
        ((TESTS_FAILED++))
        return 1
    fi

    echo -e "${GREEN}✔ PASS${NC}: $desc"
    ((TESTS_PASSED++))
    return 0
}

echo "=============================================================================="
echo "Running Regression Tests for scripts/upgrade_contract.sh"
echo "=============================================================================="

# 1. Help flag
assert_success "Help option displays usage and exits 0" "Usage: .*upgrade_contract\.sh" --help
assert_success "Short help option displays usage and exits 0" "Usage: .*upgrade_contract\.sh" -h

# 2. Missing arguments
assert_fail "No arguments provided fails" "Missing required arguments"
assert_fail "Missing contract ID and WASM path fails" "Missing required arguments" testnet
assert_fail "Missing WASM path fails" "Missing required arguments" testnet "$VALID_CONTRACT_ID"

# 3. Invalid network
assert_fail "Invalid network name is rejected" "Invalid network: devnet_unknown" devnet_unknown "$VALID_CONTRACT_ID" "$VALID_WASM"

# 4. Invalid Contract ID format
assert_fail "Contract ID with invalid prefix is rejected" "Invalid contract ID format" testnet "$INVALID_CONTRACT_ID_PREFIX" "$VALID_WASM"
assert_fail "Contract ID with invalid length is rejected" "Invalid contract ID format" testnet "$INVALID_CONTRACT_ID_LENGTH" "$VALID_WASM"
assert_fail "Contract ID with invalid chars is rejected" "Invalid contract ID format" testnet "C!!!234567890123456789012345678901234567890123456789012345" "$VALID_WASM"

# 5. Artifact validation: non-existent file
assert_fail "Non-existent WASM file is rejected" "WASM file not found" testnet "$VALID_CONTRACT_ID" "$TMP_DIR/missing.wasm"

# 6. Artifact validation: empty file
assert_fail "Empty WASM file is rejected" "WASM file is empty" testnet "$VALID_CONTRACT_ID" "$EMPTY_WASM"

# 7. Artifact validation: non-WASM header
assert_fail "Non-WASM binary header is rejected" "not a valid WASM binary" testnet "$VALID_CONTRACT_ID" "$INVALID_WASM"

# 8. Unknown option
assert_fail "Unknown flag is rejected" "Unknown option: --invalid-flag" --invalid-flag

# 9. Dry-run mode with valid inputs
assert_success "Valid dry-run mode succeeds with deterministic output" "DRY-RUN COMPLETED" testnet "$VALID_CONTRACT_ID" "$VALID_WASM" --dry-run
assert_success "Dry-run prints WASM checksum" "WASM Checksum:" testnet "$VALID_CONTRACT_ID" "$VALID_WASM" --dry-run
assert_success "Dry-run prints network" "Network:[[:space:]]+testnet" testnet "$VALID_CONTRACT_ID" "$VALID_WASM" --dry-run
assert_success "Dry-run shows transaction preview" "DRY-RUN TRANSACTION PREVIEW" testnet "$VALID_CONTRACT_ID" "$VALID_WASM" --dry-run

# 10. Dry-run with named flags and other networks
assert_success "Named flags dry-run on mainnet" "Network:[[:space:]]+mainnet" --network mainnet --contract-id "$VALID_CONTRACT_ID" --wasm "$VALID_WASM" --source admin_key --dry-run
assert_success "Named flags dry-run on futurenet" "Network:[[:space:]]+futurenet" -n futurenet -c "$VALID_CONTRACT_ID" -w "$VALID_WASM" -s deployer --dry-run
assert_success "Dry-run on local network" "Network:[[:space:]]+local" local "$VALID_CONTRACT_ID" "$VALID_WASM" --dry-run
assert_success "Dry-run on standalone network" "Network:[[:space:]]+standalone" standalone "$VALID_CONTRACT_ID" "$VALID_WASM" --dry-run

# 11. Write confirmation rejection
log_test "Write mode: explicit rejection cancels upgrade"
set +e
output=$(echo "no" | "$TARGET_SCRIPT" testnet "$VALID_CONTRACT_ID" "$VALID_WASM" 2>&1)
exit_code=$?
set -e
if [ "$exit_code" -eq 0 ] && echo "$output" | grep -q "Upgrade cancelled by user"; then
    echo -e "${GREEN}✔ PASS${NC}: Write mode: explicit rejection cancels upgrade"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✘ FAIL${NC}: Write cancellation failed. Exit code: $exit_code. Output: $output"
    ((TESTS_FAILED++))
fi

# 11. Write mode: non-interactive without auto-confirm aborts safely
log_test "Write mode: non-interactive without --yes aborts safely"
set +e
output=$("$TARGET_SCRIPT" testnet "$VALID_CONTRACT_ID" "$VALID_WASM" < /dev/null 2>&1)
exit_code=$?
set -e
if [ "$exit_code" -ne 0 ] && echo "$output" | grep -q "Explicit confirmation required"; then
    echo -e "${GREEN}✔ PASS${NC}: Write mode: non-interactive without --yes aborts safely"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✘ FAIL${NC}: Non-interactive abort failed. Exit code: $exit_code. Output: $output"
    ((TESTS_FAILED++))
fi

echo ""
echo "=============================================================================="
echo "Test Results: $TESTS_PASSED passed, $TESTS_FAILED failed"
echo "=============================================================================="

if [ "$TESTS_FAILED" -gt 0 ]; then
    exit 1
fi
