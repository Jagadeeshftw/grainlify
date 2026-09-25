# Grainlify Smart Contracts

This repository contains Grainlify's Stellar Soroban smart contracts and their supporting tests, manifests, benchmarks, and deployment tooling.

## Contract workspaces

- `contracts/` contains the primary contract packages, SDK, manifests, and contract-focused documentation.
- `soroban/` contains the Soroban workspace and its escrow, program-escrow, and stream contracts.
- `benchmarks/` contains contract performance baselines and thresholds.
- `scripts/` and `fix/` contain contract validation, testing, upgrade, and maintenance utilities.

## System Documentation

The following documents describe the current architecture and behavior of the system:
- [Deployable Artifacts](DEPLOYABLE_ARTIFACTS.md) - Lists every deployable WebAssembly artifact produced by this repository.
- [Release Schedules Usage](RELEASE_SCHEDULES_USAGE.md) - Details the time-based release schedules (vesting) feature for escrow contracts.
- [Upgrade and Migration Policy](UPGRADE_AND_MIGRATION_POLICY.md) - Defines the rules and requirements for authorizing, executing, and reverting contract upgrades.

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
