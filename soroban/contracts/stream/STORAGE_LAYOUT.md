# Grainlify Stream Storage Layout

This document defines the storage layout for the `stream` contract. The stream contract is a gas regression fixture module used for deterministic gas measurements across the Grainlify smart contract ecosystem.

## Storage Schema: No on-chain storage

The `stream` contract does not define persistent on-chain storage keys. It is a utility crate providing:

- `GasRegressionFixture`: A test fixture for measuring gas costs
- `measure`, `snapshot`, `delta_from`: Budget capture helpers
- Gas regression tests for the ecosystem

Since the stream contract has no on-chain storage state, there are no `DataKey` enums or storage layout constraints to track. All gas measurements are performed against external contracts (escrow, program-escrow, etc.) whose storage layouts are documented separately.

## Usage
This crate is used for gas regression testing and does not require storage layout validation. See the [Gas Regression Guardrails](https://github.com/CollinsKRO/grainlify/blob/main/docs/gas-optimization/) documentation for related gas tracking.

## Migration Steps
No migration is required since there is no on-chain storage state. Changes to the fixture module do not affect contract storage layouts.