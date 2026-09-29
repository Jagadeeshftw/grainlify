# Grainlify stream fixtures

This crate supplies reusable Soroban environment and budget measurement fixtures for gas regression tests. Despite its name, this is a test support library, not a stream payment contract.

## Deployment status

Not deployed. Its manifest builds a Rust library with soroban-sdk testutils; it defines no deployable contract Wasm target or network contract ID.

## Crate relationships

- Depends on: no other in-repository crate; it inherits soroban-sdk 23.4.1 with testutils from the [Soroban workspace](../../README.md).
- Depended on by: its own integration tests import grainlify_stream. No other in-repository Cargo package declares it as a dependency.
