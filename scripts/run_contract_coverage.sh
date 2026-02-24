#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/contracts/coverage"
mkdir -p "$OUT_DIR"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "cargo-llvm-cov is required. Install with: cargo install cargo-llvm-cov"
  exit 1
fi

run_cov() {
  local name="$1"
  local dir="$2"
  local lcov="$OUT_DIR/${name}.lcov"

  echo "==> ${name}: coverage (line coverage >= 90%)"
  (
    cd "$dir"
    cargo llvm-cov \
      --all-features \
      --workspace \
      --lcov \
      --output-path "$lcov" \
      --fail-under-lines 90
  )
}

run_cov "bounty-escrow" "$ROOT_DIR/contracts/bounty_escrow"
run_cov "program-escrow" "$ROOT_DIR/contracts/program-escrow"

echo "Coverage reports written to $OUT_DIR"
