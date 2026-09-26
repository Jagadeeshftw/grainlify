# Grainlify contracts package

The Cargo package in this directory is grainlify-contracts: a Rust library of storage-key audit utilities and local view/simulation types, plus a small CLI. It is separate from the deployable contract packages in the directories below.

## Deployment status

Not deployed as a standalone Soroban contract. This manifest builds a Rust library and CLI, not a contract Wasm artifact. It records no network contract ID.

## Crate relationships

- Depends on: no other crate in this repository. Its external dependencies are soroban-sdk 21.7.7 and ethnum.
- Depended on by: no other in-repository Cargo package declares a dependency on grainlify-contracts.
- Related packages: [grainlify-core](grainlify-core/README.md), [program-escrow](program-escrow/README.md), [bounty-escrow](bounty_escrow/contracts/escrow/README.md), [view-facade](view-facade/README.md), and [escrow-view-facade](escrow-view-facade/README.md) have their own manifests and build outputs. The [bounty escrow workspace](bounty_escrow/README.md) owns its member package.
