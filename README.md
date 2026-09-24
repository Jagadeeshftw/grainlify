# Grainlify Smart Contracts

This repository contains Grainlify's Stellar Soroban smart contracts and their supporting tests, manifests, benchmarks, and deployment tooling.

## Contract workspaces

- `contracts/` contains the primary contract packages, SDK, manifests, and contract-focused documentation.
- `soroban/` contains the Soroban workspace and its program-escrow and stream contracts; its `escrow` crate is superseded (reference/parity only).
- `benchmarks/` contains contract performance baselines and thresholds.
- `scripts/` and `fix/` contain contract validation, testing, upgrade, and maintenance utilities.

> **Escrow authority:** the deployed escrow contract is
> `contracts/bounty_escrow/contracts/escrow` (`bounty_escrow.wasm`). The
> `soroban/contracts/escrow` crate is superseded and is never deployed. See
> [`docs/contracts/escrow-implementation-authority.md`](docs/contracts/escrow-implementation-authority.md).

## Local validation

Run the primary workspace tests with:

```bash
cargo test --manifest-path soroban/Cargo.toml
```

Validate the contract manifests with:

```bash
cd contracts
npm test
```

The contract workflows under `.github/workflows/` run the repository's contract checks, benchmark gate, upgrade smoke tests, and storage-layout validation.

## Scope

The repository is intentionally limited to on-chain contract code and contract-related tooling. Application backend, frontend, website, design, and deployment material are maintained elsewhere.
