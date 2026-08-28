#!/usr/bin/env bash
# Smoke deployment test for each deployable contract artifact (#1744)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

CONTRACTS=(
  "contracts/program-escrow|program_escrow|initialize|get_version"
  "contracts/bounty_escrow/contracts/escrow|escrow|initialize|get_version"
  "contracts/grainlify-core|grainlify_core|initialize|get_version"
  "soroban/contracts/escrow|escrow|initialize|get_version"
  "soroban/contracts/program-escrow|program_escrow|initialize|get_version"
  "soroban/contracts/stream|stream|initialize|get_version"
)

PASS=0
FAIL=0

for entry in "${CONTRACTS[@]}"; do
  IFS='|' read -r crate_path contract_name init_fn read_fn <<< "$entry"

  echo "Smoke test: $contract_name"
  cd "$ROOT_DIR/$crate_path"

  echo "  Building..."
  cargo build --target wasm32-unknown-unknown --release || { echo "Build failed"; FAIL=$((FAIL+1)); continue; }

  echo "  Deploying..."
  WASM_PATH="target/wasm32-unknown-unknown/release/${contract_name//-/_}.wasm"
  CONTRACT_ID=$(stellar contract deploy --wasm "$WASM_PATH" --source alice --network local 2>/dev/null || true)
  if [ -z "$CONTRACT_ID" ]; then
    echo "  Deploy failed"
    FAIL=$((FAIL+1))
    continue
  fi

  echo "  Initializing..."
  stellar contract invoke --id "$CONTRACT_ID" --source alice --network local -- "$init_fn" --admin alice || {
    echo "  Init failed"
    FAIL=$((FAIL+1))
    continue
  }

  echo "  Read call..."
  stellar contract invoke --id "$CONTRACT_ID" --source alice --network local -- "$read_fn" || {
    echo "  Read failed"
    FAIL=$((FAIL+1))
    continue
  }

  PASS=$((PASS+1))
done

echo "Results: $PASS passed, $FAIL failed"
exit $FAIL