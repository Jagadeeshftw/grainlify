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
PROGRAM_ESCROW_MANIFEST="$ROOT_DIR/contracts/program-escrow/Cargo.toml"

echo "==> [1/5] Workspace tests (host target)"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [2/5] Storage-key collision audit (grainlify-contracts)"
echo "==> [1/4] Workspace tests (host target)"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [2/4] Storage-key collision audit (grainlify-contracts)"
WASM_DIR="$ROOT_DIR/contracts/bounty_escrow/target/wasm32-unknown-unknown/release"
WASM_FILE="$WASM_DIR/bounty_escrow.wasm"
ANALYTICS_SIZE_FILE="$ROOT_DIR/contracts/bounty_escrow/docs/analytics_wasm_size.md"

echo "==> [1/5] Workspace tests (host target) – includes analytics & monitoring suite"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [2/5] Storage-key collision audit (grainlify-contracts)"
echo "==> [1/4] Workspace tests (host target)"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [2/4] Storage-key collision audit (grainlify-contracts)"
echo "==> [2/5] Storage-key collision audit (grainify-contracts)"
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
echo "==> [3/4] Token allowlist + FoT routing suites (program-escrow)"
cargo test --manifest-path "$ROOT_DIR/contracts/program-escrow/Cargo.toml" -- test_token_allowlist test_fot_routing

echo "==> [4/4] Deployable wasm release build"
echo "==> [1/3] Workspace tests (host target)"
cargo test --manifest-path "$MANIFEST" --workspace

echo "==> [2/3] Program escrow payout-splits suite (host target)"
cargo test --manifest-path "$PROGRAM_ESCROW_MANIFEST" --lib test_payout_splits

echo "==> [3/3] Deployable wasm release build"
echo "==> [3/4] Program escrow reentrancy guard tests"
cargo test --locked --manifest-path "$ROOT_DIR/contracts/program-escrow/Cargo.toml" reentrancy_tests::

echo "==> [4/4] Deployable wasm release build"
cargo build --manifest-path "$MANIFEST" --workspace --target wasm32-unknown-unknown --release

echo "==> [3/3] Feature-flag matrix & facade assertion gates (issue #1885)"
bash "$ROOT_DIR/contracts/scripts/check-feature-matrix.sh"

echo "==> [3/5] Deployable wasm release build (full, WITH analytics surface)"
cargo build --manifest-path "$MANIFEST" --workspace --target wasm32-unknown-unknown --release

WASM_WITH=""
if [ -f "$WASM_FILE" ]; then
  WASM_WITH=$(wc -c < "$WASM_FILE")
  echo "    bounty_escrow.wasm (with analytics): ${WASM_WITH} bytes"
else
  echo "    WARNING: $WASM_FILE not found – skipping size measurement"
fi

echo "==> [4/5] Wasm release build (WITHOUT analytics – compile-flag stub)"
# We measure the size contribution of the analytics surface by building with
# the 'no_analytics' cfg flag.  That flag gates all analytics entry points
# behind cfg(not(feature = "no_analytics")) in the contract, allowing a
# side-by-side comparison without maintaining a separate build profile.
#
# NOTE: The 'no_analytics' feature is intentionally NOT shipped; it exists
# solely so this measurement step can produce a meaningful delta.
WASM_WITHOUT=""
if cargo build --manifest-path "$MANIFEST" --workspace \
      --target wasm32-unknown-unknown \
      --release \
      --features no_analytics \
      2>/dev/null; then
  if [ -f "$WASM_FILE" ]; then
    WASM_WITHOUT=$(wc -c < "$WASM_FILE")
    echo "    bounty_escrow.wasm (without analytics): ${WASM_WITHOUT} bytes"
  fi
else
  # 'no_analytics' feature does not exist (or cargo rejected it).
  # This is expected on a standard build – skip the delta measurement and
  # record the full build size only.
  echo "    INFO: 'no_analytics' feature not defined – recording full-build size only."
fi

echo "==> [5/5] Record wasm size contribution of analytics surface"
mkdir -p "$(dirname "$ANALYTICS_SIZE_FILE")"
{
  echo "# Analytics Surface – Wasm Size Contribution"
  echo ""
  echo "Generated by \`contracts/scripts/ci-contracts.sh\` on \`$(date -u +%Y-%m-%dT%H:%M:%SZ)\`."
  echo ""
  echo "## Measurements"
  echo ""
  echo "| Build variant         | wasm size (bytes) |"
  echo "|-----------------------|-------------------|"
  if [ -n "$WASM_WITH" ]; then
    echo "| With analytics surface  | ${WASM_WITH} |"
  else
    echo "| With analytics surface  | not measured |"
  fi
  if [ -n "$WASM_WITHOUT" ]; then
    DELTA=$((WASM_WITH - WASM_WITHOUT))
    echo "| Without analytics surface | ${WASM_WITHOUT} |"
    echo "| **Delta (analytics cost)** | **${DELTA}** |"
  else
    echo "| Without analytics surface | not measured (feature flag absent) |"
    echo "| **Delta (analytics cost)** | **N/A** |"
  fi
  echo ""
  echo "## Analytics Entry Points Covered"
  echo ""
  echo "All entry points below are exercised by \`test_analytics_monitoring\`"
  echo "and \`test_query_filters\` on every pull request."
  echo ""
  echo "| Function                     | Complexity | Consumer(s)                              |"
  echo "|------------------------------|------------|------------------------------------------|"
  echo "| \`get_aggregate_stats\`         | O(n)       | test_analytics_monitoring, escrow-view-facade |"
  echo "| \`get_escrow_count\`            | O(1)       | test_analytics_monitoring, test_query_filters |"
  echo "| \`query_escrows_by_status\`     | O(n)       | test_analytics_monitoring, test_query_filters, escrow-view-facade |"
  echo "| \`query_escrows_by_amount\`     | O(n)       | test_analytics_monitoring, test_query_filters |"
  echo "| \`query_escrows_by_deadline\`   | O(n)       | test_analytics_monitoring, test_query_filters |"
  echo "| \`query_escrows_by_depositor\`  | O(k)/O(n)  | test_analytics_monitoring, test_query_filters |"
  echo "| \`get_escrow_ids_by_status\`    | O(n)       | test_analytics_monitoring, test_query_filters |"
  echo "| \`get_refund_eligibility\`      | O(1)       | test_analytics_monitoring, off-chain indexers |"
  echo "| \`get_refund_history\`          | O(k)       | test_analytics_monitoring, off-chain indexers |"
  echo "| \`get_balance\`                 | O(1)       | test_analytics_monitoring, escrow-view-facade |"
  echo "| \`health_check\`                | O(1)       | test_analytics_monitoring, ops dashboards |"
  echo "| \`get_analytics\`               | O(1)       | test_analytics_monitoring, ops dashboards |"
  echo "| \`get_state_snapshot\`          | O(1)       | test_analytics_monitoring, ops dashboards |"
} > "$ANALYTICS_SIZE_FILE"

echo "    Size report written to: $ANALYTICS_SIZE_FILE"
echo "==> All contract CI gates passed."
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
