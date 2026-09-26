#!/usr/bin/env bash
# ==============================================================================
# Grainlify - grainlify-core CI gates (local + GitHub Actions)
# ==============================================================================
# Runs the exact commands executed by the `grainlify-core` job in
# .github/workflows/contracts-ci.yml so the same gates can be reproduced
# locally. Requires a Rust toolchain with the rustfmt and clippy components and
# the wasm32-unknown-unknown target installed:
#
#   rustup component add rustfmt clippy
#   rustup target add wasm32-unknown-unknown
#
# The test gate runs the crate's default-feature test suite, which includes the
# migration-replay and upgrade-rollback suites. A regression in any of them
# fails this script and therefore the workflow.
#
# USAGE:
#   ./contracts/scripts/ci-grainlify-core.sh
# ==============================================================================

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE_DIR="$ROOT_DIR/contracts/grainlify-core"

cd "$CRATE_DIR"

echo "==> [1/4] Formatting gate (cargo fmt --check)"
cargo fmt --check

echo "==> [2/4] Lint gate (cargo clippy --all-targets -- -D warnings)"
cargo clippy --all-targets -- -D warnings

echo "==> [3/4] Test gate (cargo test: unit, migration-replay, upgrade-rollback)"
cargo test

echo "==> [4/4] Deployable wasm release build"
cargo build --target wasm32-unknown-unknown --release

echo "==> All grainlify-core CI gates passed."
