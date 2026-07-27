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

Both contracts retain the most recent **20 snapshots** (`CONFIG_SNAPSHOT_LIMIT`).
When a new snapshot exceeds this limit, the oldest snapshot is pruned automatically.

Grainlify Core also exposes `prune_old_snapshots(keep_count)` so an admin can
shrink the retention window further. Lower retention bounds listing CPU cost but
shortens rollback history depth — see `contracts/grainlify-core/STORAGE_LAYOUT.md`.

Listing should use paginated `list_config_snapshots(offset, limit)` for
indexer-style scans; `list_config_snapshots_all` remains available for small-N
/ legacy callers.

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
- `list_config_snapshots(offset, limit) -> Result<Vec<CoreConfigSnapshot>, ContractError>` — paginated listing
- `list_config_snapshots_all() -> Vec<CoreConfigSnapshot>` — full listing (legacy)
- `prune_old_snapshots(keep_count) -> u32` — admin prune; retains newest `keep_count`
- `restore_config_snapshot(snapshot_id: u64)`
- `get_config_snapshot(snapshot_id: u64) -> Option<CoreConfigSnapshot>` — retrieve a specific snapshot by ID
- `get_latest_config_snapshot() -> Option<CoreConfigSnapshot>` — most recent snapshot
- `get_snapshot_count() -> u32` — number of retained snapshots
- `get_config_snapshot_limit() -> u32` — hard retention ceiling
- `compare_snapshots(from_id: u64, to_id: u64) -> SnapshotDiff` — diff between two snapshots
- `get_rollback_info() -> RollbackInfo` — aggregated rollback intelligence for recovery drills

## Snapshot schema

### `CoreConfigSnapshot`
| Field                | Type             | Description                                |
|---------------------|------------------|--------------------------------------------|
| `id`                | `u64`            | Unique, monotonically increasing ID        |
| `timestamp`         | `u64`            | Ledger timestamp when snapshot was created  |
| `admin`             | `Option<Address>`| Admin address at snapshot time              |
| `version`           | `u32`            | Contract version at snapshot time           |
| `previous_version`  | `Option<u32>`    | Previous version (rollback target)         |
| `multisig_threshold`| `u32`            | Multisig threshold (0 = no multisig)       |
| `multisig_signers`  | `Vec<Address>`   | Multisig signer set                        |

### `SnapshotDiff`
| Field                          | Type   | Description                              |
|-------------------------------|--------|------------------------------------------|
| `from_id`                     | `u64`  | ID of the earlier snapshot               |
| `to_id`                       | `u64`  | ID of the later snapshot                 |
| `admin_changed`               | `bool` | Whether admin address changed            |
| `version_changed`             | `bool` | Whether contract version changed         |
| `previous_version_changed`    | `bool` | Whether previous_version field changed   |
| `multisig_threshold_changed`  | `bool` | Whether multisig threshold changed       |
| `multisig_signers_changed`    | `bool` | Whether signer set changed               |
| `from_version`                | `u32`  | Version in the "from" snapshot           |
| `to_version`                  | `u32`  | Version in the "to" snapshot             |

### `RollbackInfo`
| Field                      | Type   | Description                                        |
|---------------------------|--------|----------------------------------------------------|
| `current_version`         | `u32`  | Current on-chain contract version                  |
| `previous_version`        | `u32`  | Version before last upgrade (0 if never upgraded) |
| `rollback_available`      | `bool` | Whether a rollback target exists                   |
| `has_migration`           | `bool` | Whether a migration state record exists            |
| `migration_from_version`  | `u32`  | Migration source version (0 if none)               |
| `migration_to_version`    | `u32`  | Migration target version (0 if none)               |
| `migration_timestamp`     | `u64`  | Timestamp of last migration (0 if none)            |
| `snapshot_count`          | `u32`  | Number of retained snapshots                       |
| `has_snapshot`            | `bool` | Whether any snapshot exists                        |
| `latest_snapshot_id`      | `u64`  | ID of most recent snapshot (0 if none)             |
| `latest_snapshot_version` | `u32`  | Version in most recent snapshot (0 if none)        |

## Authorization

- Program escrow snapshot operations are **admin-only** (anti-abuse admin).
- Core snapshot operations are **admin-only** (core admin).
- All new query functions (`get_config_snapshot`, `get_latest_config_snapshot`, `get_snapshot_count`, `compare_snapshots`, `get_rollback_info`) are **view-only** — no authorization required.

## Recommendation

Use snapshot creation as a mandatory step in your release runbook for any on-chain config changes, similar to a database migration backup checkpoint.

### Recovery drill checklist

1. Call `get_rollback_info()` to assess rollback feasibility.
2. If `rollback_available` is true, the `previous_version` field shows the rollback target.
3. Call `get_latest_config_snapshot()` to verify the last known-good configuration.
4. Use `compare_snapshots(old_id, new_id)` to identify exactly what changed.
5. If rollback is needed, call `restore_config_snapshot(snapshot_id)` to restore state.
