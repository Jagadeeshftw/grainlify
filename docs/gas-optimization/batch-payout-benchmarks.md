# Program Escrow Batch Payout Benchmarks

This document defines the benchmark collection and review process for `contracts/program-escrow/src/lib.rs`, with emphasis on `batch_payout` and `lock_program_funds`.

## Goal

Provide a committed Stellar testnet baseline so fee regressions are visible during review and can be gated in CI.

## Benchmark artifact layout

- `benchmarks/program-escrow/testnet-baseline.json` — committed benchmark snapshot
- `benchmarks/program-escrow/testnet-baseline.example.json` — schema example only
- `benchmarks/program-escrow/thresholds.json` — CI gate thresholds
- `scripts/check_program_escrow_benchmark.py` — CI gate implementation

## Required benchmark dimensions

Capture both operations for sizes `1`, `10`, `50`, and `100`.

Each committed record must include:

- simulation ledger sequence
- inclusion ledger sequence
- actual charged fee in stroops
- minimum resource fee in stroops
- CPU instruction count

## CI gate

The CI gate checks the committed `batch_payout` result for `batch_size = 50` and fails if `actual_fee_charged_stroops` exceeds the threshold in `benchmarks/program-escrow/thresholds.json`.

## Security notes

- Use a throwaway testnet deployer identity or Friendbot-funded account.
- Never commit private keys, seed phrases, or local identity stores.
- Record the deployed WASM hash in the snapshot so the benchmark stays tied to a specific build artifact.
- Confirm the benchmark snapshot comes from a finalized inclusion ledger, not simulation only.
- If `lock_program_funds` is benchmarked with a synthetic size dimension, document the harness semantics in the snapshot notes.

## Current blocker

A real benchmark snapshot is not committed on this branch yet.

Reason: `contracts/program-escrow` still has pre-existing compile/build failures unrelated to the benchmark scaffolding, so there is no trustworthy current-contract WASM to deploy to testnet from this branch.

At the time of this update:

- `contracts/program-escrow/src/lib.rs` parser corruption was repaired enough to surface semantic errors.
- `cargo test --manifest-path ./contracts/program-escrow/Cargo.toml` still fails with multiple pre-existing compile errors.
- `cargo build --manifest-path ./contracts/program-escrow/Cargo.toml --target wasm32v1-none --release` is additionally blocked by a missing installed target in this environment.

## Recommended next steps

1. Repair the remaining `program-escrow` compile errors.
2. Install the wasm target locally:
   - `rustup target add wasm32v1-none`
3. Build the contract wasm.
4. Deploy the built wasm to Stellar testnet.
5. Capture and commit `benchmarks/program-escrow/testnet-baseline.json`.
6. Tighten the provisional threshold in `benchmarks/program-escrow/thresholds.json` to a baseline-derived value with explicit headroom.
