#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run_suite() {
  local name="$1"
  local dir="$2"

  echo "==> ${name}: property/fuzz/stress tests"
  (
    cd "$dir"
    cargo test -- --nocapture test_property_fuzz
    cargo test -- --nocapture test_fuzz
    cargo test -- --nocapture test_stress
    cargo test -- --nocapture test_gas
  )
}

run_suite "bounty-escrow" "$ROOT_DIR/contracts/bounty_escrow"
run_suite "program-escrow" "$ROOT_DIR/contracts/program-escrow"
