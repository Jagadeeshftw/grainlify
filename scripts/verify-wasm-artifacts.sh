#!/usr/bin/env bash
# =============================================================================
# scripts/verify-wasm-artifacts.sh
# =============================================================================
# Asserts that every expected wasm artifact:
#   1. exists under the given directory
#   2. is non-empty (size > 0 bytes)
#   3. starts with the WebAssembly magic bytes (\0asm)
#
# USAGE:
#   bash scripts/verify-wasm-artifacts.sh <artifact_dir> <name.wasm> [<name2.wasm> ...]
#
# EXAMPLE:
#   bash scripts/verify-wasm-artifacts.sh \
#     contracts/bounty_escrow/target/wasm32-unknown-unknown/release \
#     bounty_escrow.wasm escrow.wasm
#
# EXIT CODES:
#   0  — all assertions passed
#   1  — one or more assertions failed (details printed to stderr)
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
pass() { printf "  ✅ PASS  %s\n" "$1"; }
fail() { printf "  ❌ FAIL  %s — %s\n" "$1" "$2" >&2; }

# WebAssembly magic bytes: 0x00 0x61 0x73 0x6D (\0asm)
WASM_MAGIC=$'\x00\x61\x73\x6d'

# ---------------------------------------------------------------------------
# Argument validation
# ---------------------------------------------------------------------------
if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <artifact_dir> <name.wasm> [<name2.wasm> ...]" >&2
  exit 1
fi

ARTIFACT_DIR="$1"
shift
EXPECTED_ARTIFACTS=("$@")

# ---------------------------------------------------------------------------
# Check that the artifact directory exists
# ---------------------------------------------------------------------------
if [[ ! -d "$ARTIFACT_DIR" ]]; then
  echo "ERROR: artifact directory does not exist: $ARTIFACT_DIR" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Assertion loop
# ---------------------------------------------------------------------------
failures=0

echo "Verifying wasm artifacts in: $ARTIFACT_DIR"
echo "Expected: ${EXPECTED_ARTIFACTS[*]}"
echo ""

for name in "${EXPECTED_ARTIFACTS[@]}"; do
  path="$ARTIFACT_DIR/$name"

  # 1. File must exist
  if [[ ! -f "$path" ]]; then
    fail "$name" "file not found at $path"
    (( failures++ )) || true
    continue
  fi

  # 2. File must not be empty
  size=$(stat -c "%s" "$path" 2>/dev/null || stat -f "%z" "$path" 2>/dev/null)
  if [[ "$size" -eq 0 ]]; then
    fail "$name" "file is empty (0 bytes)"
    (( failures++ )) || true
    continue
  fi

  # 3. Must start with the WebAssembly magic bytes (\0asm)
  magic=$(dd if="$path" bs=1 count=4 2>/dev/null | od -An -tx1 | tr -d ' \n')
  if [[ "$magic" != "0061736d" ]]; then
    fail "$name" "does not start with wasm magic bytes (got: $magic, expected: 0061736d)"
    (( failures++ )) || true
    continue
  fi

  pass "$name  ($size bytes)"
done

# ---------------------------------------------------------------------------
# Final verdict
# ---------------------------------------------------------------------------
echo ""
if [[ "$failures" -eq 0 ]]; then
  echo "All ${#EXPECTED_ARTIFACTS[@]} expected wasm artifact(s) verified."
  exit 0
else
  echo "ERROR: $failures of ${#EXPECTED_ARTIFACTS[@]} wasm artifact assertion(s) failed." >&2
  exit 1
fi
