# Program Escrow Benchmarks

This directory stores reviewable benchmark artifacts for `contracts/program-escrow`.

## Expected files

- `testnet-baseline.json` — committed benchmark snapshot captured from Stellar testnet.
- `testnet-baseline.example.json` — example schema for the committed snapshot.
- `thresholds.json` — CI gate thresholds.

## Required snapshot fields

Each benchmark record should include:

- `operation`
- `batch_size`
- `simulation_latest_ledger`
- `inclusion_ledger`
- `actual_fee_charged_stroops`
- `min_resource_fee_stroops`
- `cpu_instructions`

## Status

A real `testnet-baseline.json` has not been committed on this branch because `program-escrow` still has pre-existing compile errors that block a trustworthy deploy/build. See `docs/gas-optimization/batch-payout-benchmarks.md` for the current blocker details and the collection workflow.
