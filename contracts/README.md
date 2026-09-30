# Grainlify Contract Tree (`contracts/`)

## Status: authoritative

This is the **authoritative contract tree** for Grainlify. All production contracts are developed, tested, and built for deployment from here. If a contract you are looking for exists in both `contracts/` and the legacy `soroban/` tree, this one is the source of truth.

Why two trees exist at all is explained in the [root README](../README.md#contract-workspaces-why-there-are-two-trees). In short: the two trees target different Soroban SDK majors (`contracts/` on SDK 21.x, `soroban/` on SDK 23.x) with different APIs and sizes, which made it ambiguous for contributors which one is real. This tree is the one CI builds deployable WASM from and runs every contract gate against; `soroban/` is retained as a legacy/reference tree for behavioral parity testing — not for deployment.

## What lives here

| Crate | Purpose |
|---|---|
| `bounty_escrow/` | Production bounty escrow contract (the deployed WASM CI builds). |
| `program-escrow/` | Production program escrow contract. |
| `grainlify-core/` | Upgrade, timelock, and governance core (`init_admin`, upgrade proposals, snapshots, watchdog). |
| `escrow-view-facade/` | Read-only view facade over the bounty escrow. |
| `view-facade/` | Read-only registry/view facade. |
| `sdk/` | TypeScript/SDK bindings and error mapping for the contracts. |
| `scripts/` | CI gates (`ci-contracts.sh`), deployment, and validation tooling. |
| `*.json` manifests | Machine-readable contract metadata validated by `npm test` here and `validate-manifests.yml` in CI. |

## Pinning

- Toolchain: Soroban SDK **`21.7.7`** (exact pin in every crate manifest and `bounty_escrow/Cargo.toml` workspace dependency).
- `rust-toolchain.toml` pins the Rust toolchain used by CI.

## Validation

```bash
# Full gate set CI runs (tests + wasm release build)
bash scripts/ci-contracts.sh

# Manifest validation only (no Rust toolchain needed)
npm test
```

CI workflows that exercise this tree: `contracts-ci.yml`, `benchmark-gate.yml`, `wasm-size-budget.yml`, `storage-layout.yml`, `build-reproducibility.yml`, `e2e-upgrade-tests.yml`, `sdk-version-check.yml`, `validate-manifests.yml`.

## Where changes belong

Open changes against this tree by default. Only target the legacy `soroban/` tree when fixing its own contracts or keeping its parity tests aligned with behavior defined here.
