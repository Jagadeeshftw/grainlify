# Escrow implementation authority

Grainlify currently has two Rust/Soroban crates whose source directory is named
`escrow`. They are **not** interchangeable, and only one of them is deployed.
This document records which implementation is authoritative, which is
superseded, and what the deployable artifact inventory is, so that no build,
deploy, or integration can pick up the wrong contract.

## Summary

| | Authoritative | Superseded |
|---|---|---|
| Source | `contracts/bounty_escrow/contracts/escrow/src/lib.rs` | `soroban/contracts/escrow/src/lib.rs` |
| Cargo package | `bounty-escrow` | `escrow` |
| Contract type | `BountyEscrowContract` | `EscrowContract` |
| Deployable artifact | `bounty_escrow.wasm` | `escrow.wasm` (not deployable) |
| Workspace | `contracts/bounty_escrow/Cargo.toml` | `soroban/Cargo.toml` |
| Size / surface | ~10,200 lines, 164 entry points | ~1,430 lines, ~20 entry points |
| Manifest | `contracts/bounty-escrow-manifest.json` | none |
| Status | **Authoritative — the only deployed escrow** | **Superseded — reference / parity only** |

### Which implementation is deployed

`contracts/bounty_escrow/contracts/escrow` (package `bounty-escrow`, artifact
`bounty_escrow.wasm`) is the authoritative escrow contract. It is the only
escrow that may be deployed to testnet or mainnet.

### Which implementation is not deployed

`soroban/contracts/escrow` (package `escrow`, artifact `escrow.wasm`) is
superseded. It is a minimal reference implementation retained only for
behavioural parity checks (`soroban/REFUND_SNAPSHOT_PARITY.md`,
`soroban/contracts/escrow/test_snapshots/`). It is **not deployed** and must not
be deployed.

The crate labels itself as superseded in its own README
([`soroban/contracts/escrow/README.md`](../../soroban/contracts/escrow/README.md)),
and the workspace overview in `soroban/README.md` repeats the notice.

## Evidence

- **Feature surface** — the authoritative crate exposes the production escrow
  surface (multi-token support, capability tokens, participant filters, fee
  routing, two-step admin rotation, timelocks) recorded in
  `contracts/bounty-escrow-manifest.json` (current version 2.6.0).
- **CI** — `.github/workflows/contracts-ci.yml` builds and uploads only
  `contracts/bounty_escrow` as the deployable escrow (`bounty-escrow-wasm`).
- **Size budgets** — `.github/wasm-budgets.json` tracks `bounty_escrow`
  (package `bounty-escrow`) only; the `soroban/contracts/escrow` crate has no
  budget entry because it is not deployable.
- **Stated intent** — `soroban/README.md` describes the soroban escrow as
  maintaining "parity with the main `contracts/bounty_escrow` behavioral
  intent".

## Deployable artifact inventory

There is exactly **one** deployable escrow wasm in the repository:

| Artifact | Produced by | Deployment |
|---|---|---|
| `bounty_escrow.wasm` | `contracts/bounty_escrow/Cargo.toml` (package `bounty-escrow`) | Authoritative — the only escrow that may be deployed |

Related but distinct artifacts (not escrow contracts):

- `program_escrow.wasm` — the program-escrow contract
  (`contracts/program-escrow`, package `program-escrow`; also built from the
  `soroban/` workspace).
- `grainlify_core.wasm`, `view_facade.wasm`, `escrow_view_facade.wasm`.

The superseded `soroban/contracts/escrow` crate is excluded from the deployable
build inventory in `.github/workflows/build-reproducibility.yml`, so no build
target emits a second escrow wasm that could be mistaken for the authoritative
one.

## Provenance

Deployments are recorded per workspace in JSON:

- `contracts/bounty_escrow/deployments/<network>.json` — authoritative escrow.

Deployment tooling and configuration target the authoritative `bounty_escrow`
artifact only:

- `contracts/scripts/deploy.sh`
- `contracts/scripts/deploy-sandbox.sh`
- `contracts/scripts/config/{testnet,mainnet}.env`

## Migration guidance

If any tooling, script, or documentation still references
`soroban/contracts/escrow` or `escrow.wasm` as a deployable contract, repoint it
at `contracts/bounty_escrow/contracts/escrow` / `bounty_escrow.wasm`.
