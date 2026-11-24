# Bounty escrow

This SDK 21 Soroban contract holds funds for individual bounties and supports locking, contributor release, refunds, authorization controls, and administrative safeguards. It is the member package of the [bounty escrow workspace](../../README.md).

## Deployment status

No network deployment is verified in this repository. The [bounty manifest](../../../bounty-escrow-manifest.json) has an empty deployment networks list and no deployed contract ID.

## Crate relationships

- Depends on: [grainlify-core](../../../grainlify-core/README.md) through a Cargo path dependency, plus the workspace's soroban-sdk 21.7.7 and ethnum dependencies. At runtime it interacts with a configured token contract.
- Depended on by: no in-repository crate has a Cargo path dependency on bounty-escrow. [Escrow view facade](../../../escrow-view-facade/README.md) queries its ABI through local bindings and must track its data types.
- Same-name distinction: [soroban/contracts/escrow](../../../../soroban/contracts/escrow/README.md) is a separate SDK 23 contract in another workspace. It follows some bounty escrow behavior for parity tests and has no Cargo dependency on this package.

## Conditional refunds

The admin can configure an oracle address with `set_oracle(oracle, enabled)`.
When enabled, only that address can call `oracle_refund` and authenticate its
attestation; this is address-based authorization, not an external oracle
contract or price-feed integration. `get_oracle` returns the configured
address, including when oracle refunds are disabled.

Anyone can call `auto_refund` after an escrow deadline. Both conditional refund
paths return remaining funds to the original depositor, reject non-refundable
escrow states, and remain subject to pause, freeze, and pending-claim checks.
Successful refunds emit `FundsRefunded` with `OracleAttestation` or
`DeadlineExpired` as the trigger type.
