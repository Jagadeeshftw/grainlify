#!/usr/bin/env bash
# ==============================================================================
# ci-batch-tests.sh — run every batch test suite in the repository.
# ==============================================================================
# Kept separate from ci-contracts.sh so that batch semantics are gated by their
# own result. The main contract gate runs the full `bounty_escrow` workspace
# suite, which currently has pre-existing failures unrelated to batching
# (anti-abuse cooldown in test_deterministic_event_ordering, fixture drift in
# test_event_payload_fixtures, a gas budget in test_gas_ci_thresholds). With
# `set -e`, that failure aborts ci-contracts.sh before it reaches later steps —
# so a batch gate appended to that script would silently never run on a pull
# request, which is precisely the failure mode issue #1877 is about.
#
# A dedicated job in .github/workflows/contracts-ci.yml calls this script, so the
# batch suites execute and report on every pull request regardless of the state
# of anything else. It is also called from ci-contracts.sh so a local run of the
# contract gates covers batching too.
#
# Every module listed here must be green. If one starts failing, fix it or
# remove it from the list deliberately — do not widen the list to include suites
# that are red for unrelated reasons, or the gate stops being informative.
#
# USAGE:
#   ./contracts/scripts/ci-batch-tests.sh
# ==============================================================================

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ESCROW_MANIFEST="$ROOT_DIR/contracts/bounty_escrow/Cargo.toml"
PROGRAM_ESCROW_MANIFEST="$ROOT_DIR/contracts/program-escrow/Cargo.toml"

# Module name -> manifest. Each is a separate cargo workspace, so each needs its
# own `cargo test` invocation.
#
# `test_batch_limits` was an orphan: the file existed but was never declared as a
# module in lib.rs, so none of its MAX_BATCH_SIZE enforcement tests ever ran.
# `test_batch_failure_modes` / `test_batch_failure_mode` (escrow) were commented
# out in lib.rs. All were registered in #1877.
ESCROW_BATCH_MODULES=(
  test_batch_failure_semantics   # new in #1877: the all-or-nothing contract
  test_batch_failure_modes       # 24 tests: error/condition matrix
  test_batch_failure_mode        # 44 tests: rollback matrix
)

PROGRAM_ESCROW_BATCH_MODULES=(
  test_batch_operations          # 40 tests: incl. atomicity + prevalidation bench
  test_batch_limits              # 6 tests: MAX_BATCH_SIZE enforcement
  test_program_batch_registration # 14 tests: registry writes across a batch
)

echo "==> bounty-escrow batch suites"
for module in "${ESCROW_BATCH_MODULES[@]}"; do
  echo "    -> ${module}"
  cargo test --manifest-path "$ESCROW_MANIFEST" --workspace "$module"
done

echo "==> program-escrow batch suites"
for module in "${PROGRAM_ESCROW_BATCH_MODULES[@]}"; do
  echo "    -> ${module}"
  cargo test --manifest-path "$PROGRAM_ESCROW_MANIFEST" "$module"
done

echo "==> All batch suites passed."
