#!/usr/bin/env bash
# ==============================================================================
# Grainlify - Smart Contract CI gates (local + GitHub Actions)
# ==============================================================================
# Runs the exact commands executed by .github/workflows/contracts-ci.yml so the
# same gates can be reproduced locally. Requires a Rust toolchain with the
# wasm32-unknown-unknown target installed:
#
#   rustup target add wasm32-unknown-unknown
#
# USAGE:
#   ./contracts/scripts/ci-contracts.sh
# ==============================================================================

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$ROOT_DIR/contracts/bounty_escrow/Cargo.toml"

echo "==> [1/2] Workspace tests (host target)"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [2/2] Deployable wasm release build"
cargo build --manifest-path "$MANIFEST" --workspace --target wasm32-unknown-unknown --release

echo "==> All contract CI gates passed."
