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

echo "==> [1/3] Workspace tests (host target)"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [2/3] Storage-key collision audit (grainlify-contracts)"
# `contracts/` used to belong to no workspace, so its storage-collision audit
# was never built. It is now a workspace of its own and is exercised here, which
# pins the audit to a check that runs on every pull request.
cargo test --manifest-path "$ROOT_DIR/contracts/Cargo.toml" --workspace

echo "==> [3/3] Deployable wasm release build"
cargo build --manifest-path "$MANIFEST" --workspace --target wasm32-unknown-unknown --release

echo "==> All contract CI gates passed."
