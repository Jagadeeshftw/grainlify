# Escrow (Soroban workspace)

This SDK 23 Soroban contract implements bounty-style lock, release, and refund flows with identity, delegation, labels, jurisdiction, and ownership features. Its tests compare selected behavior with the main bounty escrow contract.

## Deployment status

No network deployment is verified in this repository. The Soroban workspace contains a testnet environment example, but no contract ID or deployment record for this crate.

## Crate relationships

- Depends on: no other in-repository crate. It inherits soroban-sdk 23.4.1 and ethnum from the [Soroban workspace](../../README.md), and uses a token contract at runtime.
- Depended on by: no in-repository Cargo package declares a dependency on this crate.
- Same-name distinction: [contracts/bounty_escrow/contracts/escrow](../../../contracts/bounty_escrow/contracts/escrow/README.md) is the separate SDK 21 bounty escrow implementation. The two are compared for behavior; they are not the same package and must not be joined by a Cargo path dependency.
