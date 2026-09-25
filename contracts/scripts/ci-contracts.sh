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

echo "==> [1/5] Workspace tests (host target)"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [2/5] Storage-key collision audit (grainify-contracts)"
# `contracts/` used to belong to no workspace, so its storage-collision audit
# was never built. It is now a workspace of its own and is exercised here, which
# pins the audit to a check that runs on every pull request.
cargo test --manifest-path "$ROOT_DIR/contracts/Cargo.toml" --workspace

echo "==> [3/5] program-escrow + escrow batch suites"
# Delegates to a dedicated script rather than inlining the module list. The
# batch suites also run as their own job in contracts-ci.yml, because this
# script currently aborts at step 1 on pre-existing bounty-escrow failures
# unrelated to batching — which would mean a batch gate inlined here never
# actually ran on a pull request.
bash "$ROOT_DIR/contracts/scripts/ci-batch-tests.sh"

echo "==> [4/5] Deployable wasm release build"
cargo build --manifest-path "$MANIFEST" --workspace --target wasm32-unknown-unknown --release

echo "==> [5/5] All contract CI gates passed."
