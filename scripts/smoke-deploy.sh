#!/usr/bin/env bash
# Smoke deployment test for each deployable contract artifact (#1744)
#
# PASS CONDITION (explicit, not just a zero exit code):
#   For every entry in CONTRACTS below, the smoke deploy passes only when ALL
#   of the following succeed in order:
#     1. The crate builds a wasm32-unknown-unknown release artifact.
#     2. The artifact deploys to the local network and yields a contract ID.
#     3. The init function (init_fn) invokes successfully.
#     4. The read function (read_fn) invokes successfully and returns output.
#   A run passes only if PASS == number of CONTRACTS and FAIL == 0.
#   Any build/deploy/init/read failure is reported with the artifact name and
#   the failing stage, so a failure is actionable from output alone.
#
# CADENCE: This script is run by .github/workflows/smoke-deploy.yml on a
# scheduled cadence (see that workflow's `on.schedule` cron) and on demand.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

CONTRACTS=(
  "contracts/program-escrow|program_escrow|initialize|get_version"
  "contracts/bounty_escrow/contracts/escrow|escrow|initialize|get_version"
  "contracts/grainlify-core|grainlify_core|initialize|get_version"
  "soroban/contracts/escrow|escrow|initialize|get_version"
  "soroban/contracts/program-escrow|soroban_program_escrow|initialize|get_version"
  "soroban/contracts/stream|stream|initialize|get_version"
)

PASS=0
FAIL=0
FAILED_ARTIFACTS=()

for entry in "${CONTRACTS[@]}"; do
  IFS='|' read -r crate_path contract_name init_fn read_fn <<< "$entry"

  echo "Smoke test: $contract_name"
  cd "$ROOT_DIR/$crate_path"

  echo "  Building..."
  if ! cargo build --target wasm32-unknown-unknown --release; then
    echo "  FAIL [$contract_name] stage=build: cargo build --target wasm32-unknown-unknown --release failed in $crate_path"
    FAIL=$((FAIL+1))
    FAILED_ARTIFACTS+=("$contract_name (build)")
    continue
  fi

  echo "  Deploying..."
  WASM_PATH="target/wasm32-unknown-unknown/release/${contract_name//-/_}.wasm"
  CONTRACT_ID=$(stellar contract deploy --wasm "$WASM_PATH" --source alice --network local 2>/dev/null || true)
  if [ -z "$CONTRACT_ID" ]; then
    echo "  FAIL [$contract_name] stage=deploy: no contract ID returned for wasm $WASM_PATH"
    FAIL=$((FAIL+1))
    FAILED_ARTIFACTS+=("$contract_name (deploy)")
    continue
  fi

  echo "  Initializing..."
  stellar contract invoke --id "$CONTRACT_ID" --source alice --network local -- "$init_fn" --admin alice || {
    echo "  FAIL [$contract_name] stage=init: '$init_fn' failed for contract $CONTRACT_ID"
    FAIL=$((FAIL+1))
    FAILED_ARTIFACTS+=("$contract_name (init)")
    continue
  }

  echo "  Read call..."
  stellar contract invoke --id "$CONTRACT_ID" --source alice --network local -- "$read_fn" || {
    echo "  FAIL [$contract_name] stage=read: '$read_fn' failed for contract $CONTRACT_ID"
    FAIL=$((FAIL+1))
    FAILED_ARTIFACTS+=("$contract_name (read)")
    continue
  }

  PASS=$((PASS+1))
done

echo "Results: $PASS passed, $FAIL failed"
if [ "$FAIL" -ne 0 ]; then
  echo "Failed artifacts:"
  for artifact in "${FAILED_ARTIFACTS[@]}"; do
    echo "  - $artifact"
  done
fi
if [ "$PASS" -ne "${#CONTRACTS[@]}" ] || [ "$FAIL" -ne 0 ]; then
  echo "Smoke deploy FAILED: expected ${#CONTRACTS[@]} artifacts to pass, got $PASS passed / $FAIL failed"
  exit 1
fi
echo "Smoke deploy PASSED: all ${#CONTRACTS[@]} deployable artifacts built, deployed, initialized, and read back."
exit $FAIL