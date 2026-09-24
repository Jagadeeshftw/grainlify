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
