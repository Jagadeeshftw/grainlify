# Asset Identifier Strategy

## Canonical Representation

All token/asset identifiers in Grainlify contracts are represented as:

- `grainlify_core::asset::AssetId`
- Current alias: `soroban_sdk::Address`

## Invariants

Asset ids used for token storage/transfer must satisfy:

1. The identifier is a Soroban **contract** address (`ScAddress::Contract`)
2. Account addresses (`ScAddress::Account`) are rejected
3. Normalization is identity after validation, so callers always see one canonical type

## Shared Validation Helper

Use shared helpers from `contracts/grainlify-core/src/asset.rs`:

- `normalize_asset_id(env, raw_asset_id)`
- `validate_asset_id(env, asset_id)`

These helpers are used by:

- `program-escrow::init_program(...)`
- `bounty-escrow::init(...)`

## Accepted Inputs

At contract boundaries, accepted asset ids are:

- Token contract ids / contract addresses

Rejected inputs:

- Account addresses (including Stellar asset issuer account addresses)

## Error Surface

- Program escrow: panics with `Invalid asset identifier: expected contract address`
- Bounty escrow: returns contract error `InvalidAssetId`

## Rationale

This removes format drift between escrow modules by enforcing one asset-id type and one validator. It prevents subtle mismatches where one module might store or accept account-style addresses while another expects contract ids.
