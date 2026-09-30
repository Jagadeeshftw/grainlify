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

- `benchmarks/` contains contract performance baselines and thresholds.
- `scripts/` and `fix/` contain contract validation, testing, upgrade, and maintenance utilities.

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
