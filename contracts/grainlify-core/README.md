# Grainlify core

This SDK 21 Soroban contract provides governance and upgrade controls, including multisig authorization, timelocked proposals, versioning, configuration snapshots, and a registry of deployed contracts. Its Rust library also supplies shared types and helpers to other packages.

## Deployment status

No network deployment is verified in this repository. The [core manifest](../grainlify-core-manifest.json) has an empty deployment networks list and provides deployment instructions, but no deployed contract ID.

## Crate relationships

- Depends on: no other crate in this repository; the direct external dependencies are soroban-sdk 21.7.7 and ethnum.
- Depended on by: [bounty-escrow](../bounty_escrow/contracts/escrow/README.md), [program-escrow](../program-escrow/README.md), and [view-facade](../view-facade/README.md) declare Cargo path dependencies on this crate.
- [Escrow view facade](../escrow-view-facade/README.md) tracks related contract ABI types, but does not declare a Cargo path dependency on grainlify-core.
