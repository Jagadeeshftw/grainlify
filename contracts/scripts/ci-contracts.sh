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

echo "==> [2/5] Storage-key collision audit (grainlify-contracts)"
# `contracts/` used to belong to no workspace, so its storage-collision audit
# was never built. It is now a workspace of its own and is exercised here, which
# pins the audit to a check that runs on every pull request.
cargo test --manifest-path "$ROOT_DIR/contracts/Cargo.toml" --workspace

echo "==> [3/5] Facade conformance — view-facade (issue #1870)"
# Each facade's cross-contract conformance gate is its own integration test
# target so it runs independently of the crate's legacy `#[cfg(test)]` unit
# modules (which are maintained separately and are not part of this gate).
cargo test \
  --manifest-path "$ROOT_DIR/contracts/view-facade/Cargo.toml" \
  --test conformance \
  --test dependency_gating

echo "==> [4/5] Facade conformance — escrow-view-facade (issue #1870)"
cargo test \
  --manifest-path "$ROOT_DIR/contracts/escrow-view-facade/Cargo.toml" \
  --test conformance \
  --test dependency_gating

echo "==> [5/5] Deployable wasm release build"
echo "==> [1/3] Clippy lint check"
cargo clippy --manifest-path "$MANIFEST" --workspace --all-targets -- -D warnings

echo "==> [2/3] Workspace tests (host target)"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [3/3] Deployable wasm release build"
echo "==> [1/3] Workspace tests (host target)"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [2/3] Deployable wasm release build"
cargo build --manifest-path "$MANIFEST" --workspace --target wasm32-unknown-unknown --release

echo "==> [3/3] Feature-flag matrix & facade assertion gates (issue #1885)"
bash "$ROOT_DIR/contracts/scripts/check-feature-matrix.sh"

echo "==> All contract CI gates passed."
