#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run_tests() {
  local name="$1"
  local dir="$2"
  echo "==> ${name}: full test suite"
  (
    cd "$dir"
    cargo test
  )
}

run_tests "bounty-escrow" "$ROOT_DIR/contracts/bounty_escrow"
run_tests "program-escrow" "$ROOT_DIR/contracts/program-escrow"

"$ROOT_DIR/scripts/run_contract_fuzz.sh"

if [[ "${WITH_COVERAGE:-0}" == "1" ]]; then
  "$ROOT_DIR/scripts/run_contract_coverage.sh"
fi
