#!/usr/bin/env bash
# ==============================================================================
# Grainlify - Feature-Flag Matrix Build & Verification Script (Issue #1885)
# ==============================================================================
# Builds every supported feature combination of grainlify-core and verifies
# that downstream contracts and facades build cleanly with default-features = false.
#
# USAGE:
#   ./contracts/scripts/check-feature-matrix.sh
# ==============================================================================

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CORE_MANIFEST="$ROOT_DIR/contracts/grainlify-core/Cargo.toml"

echo "======================================================================"
echo "==> [1/12] Building grainlify-core (default features: contract)"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST"

echo "======================================================================"
echo "==> [2/12] Building grainlify-core (--no-default-features) [Library Mode]"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST" --no-default-features

echo "======================================================================"
echo "==> [3/12] Building grainlify-core (--no-default-features --features contract) [Explicit Contract]"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST" --no-default-features --features contract

echo "======================================================================"
echo "==> [4/12] Building grainlify-core (--no-default-features --features strict-mode) [Strict Library]"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST" --no-default-features --features strict-mode

echo "======================================================================"
echo "==> [5/12] Building grainlify-core (--features strict-mode) [Strict Contract]"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST" --features strict-mode

echo "======================================================================"
echo "==> [6/12] Building grainlify-core (--features testutils) [Test Utilities]"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST" --features testutils

echo "======================================================================"
echo "==> [7/12] Building grainlify-core (--features upgrade_rollback_tests)"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST" --features upgrade_rollback_tests

echo "======================================================================"
echo "==> [8/12] Building grainlify-core (--features governance_contract_tests)"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST" --features governance_contract_tests

echo "======================================================================"
echo "==> [9/12] Building grainlify-core (--features wasm_tests)"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST" --features wasm_tests

echo "======================================================================"
echo "==> [10/12] Building grainlify-core (--all-features)"
echo "======================================================================"
cargo check --manifest-path "$CORE_MANIFEST" --all-features

echo "======================================================================"
echo "==> [11/12] Building grainlify-core wasm32 (default: deployable contract)"
echo "======================================================================"
cargo build --manifest-path "$CORE_MANIFEST" --target wasm32-unknown-unknown --release

echo "======================================================================"
echo "==> [12/12] Building grainlify-core wasm32 (--no-default-features) [wasm library]"
echo "======================================================================"
cargo build --manifest-path "$CORE_MANIFEST" --target wasm32-unknown-unknown --release --no-default-features

echo "======================================================================"
echo "==> [Test] Running feature matrix & facade default-features assertion tests"
echo "======================================================================"
cargo test --manifest-path "$CORE_MANIFEST" --test test_feature_matrix

echo "======================================================================"
echo "==> All supported feature combinations and build tests passed successfully!"
echo "======================================================================"
