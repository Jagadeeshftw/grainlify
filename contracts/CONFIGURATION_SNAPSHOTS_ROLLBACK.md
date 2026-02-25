# On-Chain Configuration Snapshots & Rollback

This document explains how to use the new on-chain configuration snapshot capability for fast recovery from misconfiguration.

## What is captured

### `contracts/program-escrow`
Each snapshot stores:
- `FeeConfig` (`lock_fee_rate`, `payout_fee_rate`, `fee_recipient`, `fee_enabled`)
- Anti-abuse config (`window_size`, `max_operations`, `cooldown_period`)
- Anti-abuse admin address (`Option<Address>`)
- Global pause flag

### `contracts/grainlify-core`
Each snapshot stores:
- Admin address (`Option<Address>`)
- Current contract version
- Previous version
- Multisig config (`signers`, `threshold`) when present

## Retention and pruning

Both contracts retain the most recent **20 snapshots**.
When a new snapshot exceeds this limit, the oldest snapshot is pruned automatically.

## Operational workflow

1. Before changing fees/limits/roles/flags, create a snapshot.
2. Apply configuration updates.
3. Validate behavior in monitoring/observability.
4. If behavior regresses, restore a prior snapshot by id.

## Contract methods

### Program Escrow
- `create_config_snapshot() -> u64`
- `list_config_snapshots() -> Vec<ConfigSnapshot>`
- `restore_config_snapshot(snapshot_id: u64)`

### Grainlify Core
- `create_config_snapshot() -> u64`
- `list_config_snapshots() -> Vec<CoreConfigSnapshot>`
- `restore_config_snapshot(snapshot_id: u64)`

## Authorization

- Program escrow snapshot operations are **admin-only** (anti-abuse admin).
- Core snapshot operations are **admin-only** (core admin).

## Recommendation

Use snapshot creation as a mandatory step in your release runbook for any on-chain config changes, similar to a database migration backup checkpoint.
