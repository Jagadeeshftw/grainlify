# Grainlify Smart Contracts

This repository contains Grainlify's Stellar Soroban smart contracts and their supporting tests, manifests, benchmarks, and deployment tooling.

## Contract workspaces (why there are two trees)

This repository keeps **two parallel contract trees**. They are not duplicates of a rewrite in progress — one is authoritative, the other is a legacy/reference tree:

| Tree | Soroban SDK | Status |
|---|---|---|
| `contracts/` | 21.x (pinned `=21.7.7`) | **Authoritative.** The primary development and deployment source for Grainlify contracts. |
| `soroban/` | 23.x (pinned `=23.4.1`) | **Legacy / reference.** A standalone workspace whose escrow, program-escrow, and stream contracts are kept for behavioral parity testing. |

### What each tree is for

- `contracts/` is where Grainlify contract development happens. It holds the production bounty escrow (`contracts/bounty_escrow`), program escrow (`contracts/program-escrow`), the upgrade/governance core (`contracts/grainlify-core`), the read-only view facades, and the SDK bindings. CI builds the deployable WASM from this tree, and the benchmark, gas, WASM-size, storage-layout, and build-reproducibility gates all run against it.
- `soroban/` is a smaller, independent workspace with its own `escrow`, `program-escrow`, and `stream` contracts on a newer SDK major. It is retained as a reference: its escrow test suite deliberately mirrors `contracts/bounty_escrow` behavioral intent (see the parity tests in [`soroban/README.md`](soroban/README.md)). It is not the tree Grainlify ships contracts from.

### Intended end state

There is currently no active plan to converge the two trees. `contracts/` remains the single authoritative tree for production contracts, and `soroban/` stays as a reference/parity tree until maintainers decide otherwise. Any migration or consolidation effort should be agreed with the maintainers in an issue before work starts.

### Which tree should my change target?

- Almost always `contracts/`. New features, fixes, and documentation for Grainlify contracts belong there.
- Open changes against `soroban/` only when fixing or extending its own escrow, program-escrow, or stream contracts — for example, keeping its parity tests aligned with `contracts/bounty_escrow` behavior.
- If your change affects behavior both trees implement and you are unsure, target `contracts/` first; mirror it into `soroban/` only when its tests assert that shared behavior.

### Other directories

- `soroban/` contains the Soroban workspace and its program-escrow and stream contracts; its `escrow` and `program-escrow` crates are superseded (reference only) and no member of that workspace is deployable.
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
| [soroban/contracts/program-escrow](soroban/contracts/program-escrow/README.md) | Separate SDK 23 program registry and search (superseded, not deployed) |
| [soroban/contracts/stream](soroban/contracts/stream/README.md) | Gas regression test fixtures |
> **Escrow authority:** the deployed escrow contract is
> `contracts/bounty_escrow/contracts/escrow` (`bounty_escrow.wasm`). The
> `soroban/contracts/escrow` crate is superseded and is never deployed. See
> [`docs/contracts/escrow-implementation-authority.md`](docs/contracts/escrow-implementation-authority.md).
>
> **Program-escrow authority:** the deployable program escrow is
> `contracts/program-escrow` (`program_escrow.wasm`). The
> `soroban/contracts/program-escrow` crate is superseded and is never deployed;
> it is a program registry/search contract, not a payout contract, so the two
> are not interchangeable. See
> [`docs/contracts/program-escrow-implementation-authority.md`](docs/contracts/program-escrow-implementation-authority.md).

## System Documentation

The following documents describe the current architecture and behavior of the system:
- [Deployable Artifacts](DEPLOYABLE_ARTIFACTS.md) - Lists every deployable WebAssembly artifact produced by this repository.
- [Release Schedules Usage](RELEASE_SCHEDULES_USAGE.md) - Details the time-based release schedules (vesting) feature for escrow contracts.
- [Upgrade and Migration Policy](UPGRADE_AND_MIGRATION_POLICY.md) - Defines the rules and requirements for authorizing, executing, and reverting contract upgrades.

## Local validation

The authoritative tree (`contracts/`) is validated with the same gates CI runs:

```bash
bash contracts/scripts/ci-contracts.sh
```

The reference tree (`soroban/`) can be tested independently with:

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
