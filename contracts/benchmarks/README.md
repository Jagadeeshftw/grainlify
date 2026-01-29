# Contract Benchmarking Suite

This directory contains a **benchmarking suite** for Soroban smart contracts in this repo.

## What it measures

- **Execution time**: via Criterion wall-clock timings
- **Soroban budget** (proxy for gas-like cost): budget CPU instructions + memory bytes
- **Scenario coverage**: single ops, refunds, and batch operations

> Note: Soroban “gas” is represented by the host budget (CPU/memory). The benches record these values alongside wall-clock timing to help compare versions.

## Running benchmarks

From repo root:

```bash
cd contracts
cargo bench -p contract-benchmarks
```

## Interpreting results

Criterion prints timing summaries per benchmark. Each benchmark also takes a **Soroban budget snapshot** after the call, so you can extend the harness to emit structured reports if needed (CSV/JSON).

## Extending the suite

- Add benchmarks for additional contracts by adding dependencies in `Cargo.toml`
- Create another `benches/<contract>.rs` file following the same pattern
- Add more scenarios (large batches, repeated refunds, custom refunds, etc.)

