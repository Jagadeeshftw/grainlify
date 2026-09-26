# Grainlify Smart Contracts

This repository contains Grainlify's Stellar Soroban smart contracts and their supporting tests, manifests, benchmarks, and deployment tooling.

## Contract workspaces

- `contracts/` contains the primary contract packages, SDK, manifests, and contract-focused documentation.
- `soroban/` contains the Soroban workspace and its escrow, program-escrow, and stream contracts.
- `benchmarks/` contains contract performance baselines and thresholds.
- `scripts/` and `fix/` contain contract validation, testing, upgrade, and maintenance utilities.

## Contract crate guide

The deployment status in each README reflects evidence recorded in this repository. Missing network IDs mean a live deployment is unverified here, not that one has never existed.

| Manifest directory | Purpose |
| --- | --- |
| [contracts](contracts/README.md) | Rust utility library and CLI |
| [contracts/bounty_escrow](contracts/bounty_escrow/README.md) | SDK 21 workspace for the bounty contract |
| [contracts/bounty_escrow/contracts/escrow](contracts/bounty_escrow/contracts/escrow/README.md) | SDK 21 bounty fund escrow |
| [contracts/escrow-view-facade](contracts/escrow-view-facade/README.md) | Bounty and program read facade |
| [contracts/grainlify-core](contracts/grainlify-core/README.md) | Governance, upgrades, and shared types |
| [contracts/program-escrow](contracts/program-escrow/README.md) | SDK 21 program funds and payouts |
| [contracts/view-facade](contracts/view-facade/README.md) | Registry and program read facade |
| [soroban](soroban/README.md) | SDK 23 workspace |
| [soroban/contracts/escrow](soroban/contracts/escrow/README.md) | Separate SDK 23 bounty-style escrow |
| [soroban/contracts/program-escrow](soroban/contracts/program-escrow/README.md) | Separate SDK 23 program registry and search |
| [soroban/contracts/stream](soroban/contracts/stream/README.md) | Gas regression test fixtures |

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
