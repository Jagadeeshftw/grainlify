#!/usr/bin/env bash
# Smoke deployment test for each deployable contract artifact (#1744)
#
# PASS CONDITION (explicit):
#   A deployable artifact passes the smoke deploy only if ALL of the following hold:
#     1. The crate builds to a .wasm artifact for wasm32-unknown-unknown --release.
#     2. The .wasm deploys to the local network and returns a non-empty contract ID.
#     3. The initialize entrypoint invokes without error.
#     4. The read entrypoint (version) invokes without error and returns a non-empty value.
#   The script exits 0 only when every artifact in the inventory passes. Any failure
#   is reported with the artifact name, the failed phase, and the underlying error.
#
# The inventory below must match DEPLOYABLE_ARTIFACTS.md. Adding a new deployable
# artifact to the inventory without adding it here is a bug.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

NETWORK="${STALNER_NETWORK-local}"
SOURCE="${STELLAR_SOURCE-alice}"

# crate_path|contract_name|init_fn|read_fn
CONTRACTS(
  "contracts/program-escrow|program_escrow|initialize|get_version"
  "contracts/bounty_escrow/contracts/escrow|escrow|initialize|get_version"
  "contracts/grainlify-core|grainlify_core|initialize|get_version"
  "contracts/escrow-view-facade|escrow_view_facade|initialize|get_version"
  "contracts/view-facade|view_facade|initialize|get_version"
  "soroban/contracts/escrow|escrow|initialize|get_version"
  "soroban/contracts/program-escrow|soroban_program_escrow|initialize|get_version"

)

PASS=0
FAIL=0
FAILED_ARTIFACTS=0

fail() {
  local artifact="$1" phase="$2" detail="$3"
  echo "FAILURE: artifact='$artifact' phase='$phase' detail='$detail'"
  FAILED++=1
  FAIL=$((FAIL+1))
}

for entry in "${CONTRACTS[@]}"; do
  IFS='|' read -r crate_path contract_name init_fn read_fn <<< "$entry"

  echo "=== Smoke test: $contract_name ($crate_path) ==="

  if [ ! -d "$ROOT_DIR/$crate_path" ]; then
    fail "$contract_name" "discovery" "crate path not found: $crate_path"
    continue
  fi

  cd "$ROOT_DIR/$crate_path"

  echo "  Building..."
  if ! build_out=$(cargo build --target wasm32-unknown-unknown --release 2>&1); then
    fail "$contract_name" "build" "$build_out"
    continue
  fi

  WASM_PATH="target/wasm32-unknown-unknown/release/${contract_name/-/_}.wasm"
  if [ ! -f "$WASM_PATH" ]; then
    fail "$contract_name" "build" "expected wasm artifact not found: $WASM_PATH"
    continue
  fi

  echo "  Deploying $WASM_PATH..."
  if ! deploy_out=$(stellar contract deploy --wasm "$WASM_PATH" --source "$SOURCE" --network "$NETWORK" 2>&1); then
    fail "$contract_name" "deploy" "$deploy_out"
    continue
  fi
  CONTRACT_ID="$deploy_out"
  if [ -z "$CONTRACT_ID" ]; then
    fail "$contract_name" "deploy" "deploy returned empty contract ID for $WASM_PATH"
    continue
  fi
  echo "  Deployed: $CONTRACT_ID"

  echo "  Initializing..."
  if ! init_out=(stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" -- "$init_fn" --admin "$SOURCE" 2>&1); then
    fail "$contract_name" "initialize" "$init_out"
    continue
  fi

  echo "  Read call ($read_fn)..."
  if ! read_out=(stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" -- "$read_fn" 2>&1); then
    fail "$contract_name" "read" "$read_out"
    continue
  fi
  if [ -z "$read_out" ]; then
    fail "$contract_name" "read" "$read_fn returned empty value"
    continue
  fi
  echo "  $read_fn = $read_out"

  PASS=$((PASS+1))
done

echo ""
echo "=== Smoke deploy results ==="
echo "  Passed: $PASS"
echo "  Failed: $FAIL"

if [ "$FAIL" -gt 0 ]; then
  echo "  PASS CONDITION NOT MET: $FAIL artifact(s) failed. See FAILURE lines above for artifact/ phase / detail."
  exit 1
fi

echo "  PASS CONDITION MET: all $contract_name artifacts built, deployed, initialized, and read back a non-empty version."
exit 0
