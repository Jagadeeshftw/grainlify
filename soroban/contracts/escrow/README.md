# Escrow (Soroban workspace)

This SDK 23 Soroban contract implements bounty-style lock, release, and refund flows with identity, delegation, labels, jurisdiction, and ownership features. Its tests compare selected behavior with the main bounty escrow contract.

## Deployment status

No network deployment is verified in this repository. The Soroban workspace contains a testnet environment example, but no contract ID or deployment record for this crate.

## Crate relationships

- Depends on: no other in-repository crate. It inherits soroban-sdk 23.4.1 and ethnum from the [Soroban workspace](../../README.md), and uses a token contract at runtime.
- Depended on by: no in-repository Cargo package declares a dependency on this crate.
- Same-name distinction: [contracts/bounty_escrow/contracts/escrow](../../../contracts/bounty_escrow/contracts/escrow/README.md) is the separate SDK 21 bounty escrow implementation. The two are compared for behavior; they are not the same package and must not be joined by a Cargo path dependency.
# `escrow` — superseded

> ## SUPERSEDED — DO NOT DEPLOY
>
> This crate (`soroban/contracts/escrow`, package `escrow`, artifact
> `escrow.wasm`) is a minimal reference escrow (lock / release / refund,
> jurisdictions, labels, delegate permissions).
>
> The authoritative escrow implementation is
> [`contracts/bounty_escrow/contracts/escrow`](../../../contracts/bounty_escrow/contracts/escrow)
> (package `bounty-escrow`, artifact `bounty_escrow.wasm`). It is the **only**
> escrow contract that is deployed.
>
> This crate is retained solely for behavioural parity testing. It is not part
> of the deployable artifact inventory and is never deployed to testnet or
> mainnet.

See [`docs/contracts/escrow-implementation-authority.md`](../../../docs/contracts/escrow-implementation-authority.md)
for the full authority record, evidence, and the deployable artifact inventory.

## What this crate is used for

- Parity snapshots against the authoritative contract — see
  [`soroban/REFUND_SNAPSHOT_PARITY.md`](../../REFUND_SNAPSHOT_PARITY.md).
- Snapshot tests under `test_snapshots/` (`parity_*`, plus the label, identity,
  search, ownership-transfer, and max-count suites).

## Running the parity tests locally

```bash
cd soroban/contracts/escrow
cargo test --lib
```
