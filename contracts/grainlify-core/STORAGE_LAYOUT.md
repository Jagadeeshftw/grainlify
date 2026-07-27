# Grainlify Core Storage Layout

This document defines the storage layout for the `grainlify-core` contract. Any modifications to structural types or addition of keys must be reflected here.

## Storage Schema Version: 1

Below are all storage keys utilized by the contract.

| Key | Variant/Constant | Tier | Type | Notes |
|-----|-----------------|------|------|-------|
| `DataKey::Admin` | `Admin` | Instance | `Address` | Set once at initialization |
| `DataKey::Version` | `Version` | Instance | `u32` | Current contract version |
| `DataKey::PreviousVersion` | `PreviousVersion` | Instance | `u32` | Version before the last upgrade |
| `DataKey::MigrationState` | `MigrationState` | Instance | `MigrationState` | Double-migration guard |
| `DataKey::UpgradeProposal(u64)` | `UpgradeProposal(id)` | Instance | `BytesN<32>` | Per-proposal wasm hash |
| `DataKey::ConfigSnapshot(u64)` | `ConfigSnapshot(id)` | Instance | `CoreConfigSnapshot` | Snapshotted configuration |
| `DataKey::SnapshotIndex` | `SnapshotIndex` | Instance | `Vec<u64>` | Ordered retained snapshot id list (≤ `CONFIG_SNAPSHOT_LIMIT`) |
| `DataKey::SnapshotCounter` | `SnapshotCounter` | Instance | `u64` | Monotone counter (never decrements) |
| `DataKey::ChainId` | `ChainId` | Instance | `String` | Cross-network protection |
| `DataKey::NetworkId` | `NetworkId` | Instance | `String` | Environment selector |
| `DataKey::ReadOnlyMode` | `ReadOnlyMode` | Instance | `bool` | Blocks state-mutating operations |
| `"op_count"` | (Symbol) | Persistent | `u64` | Monitoring operations counter |
| `"usr_count"` | (Symbol) | Persistent | `u64` | Monitoring unique users |
| `"err_count"` | (Symbol) | Persistent | `u64` | Monitoring error counter |
| `("perf_cnt", Symbol)` | Tuple | Persistent | `u64` | Hit count per-function |
| `("perf_time", Symbol)` | Tuple | Persistent | `u64` | Cumulative duration per-function |

## Config snapshot retention & growth bounds

### Why a hard limit exists

`create_config_snapshot` can be called repeatedly over a long-lived deployment.
Each call appends to `SnapshotIndex` and stores a `ConfigSnapshot(id)` entry.
Without a retention ceiling:

- `list_config_snapshots` CPU cost grows **O(N)** with retained count
- Instance storage grows unbounded
- A frequently snapshotted deployment can eventually fail listing with an out-of-budget error

Benchmarks in `src/test/state_snapshot_tests.rs` (`bench_list_and_compare_snapshot_growth`)
measure CPU instructions for listing and comparison as N scales from 1 → `CONFIG_SNAPSHOT_LIMIT`.
They confirm listing cost grows with N, while `compare_snapshots` stays O(1) (loads two ids only).

### Retention mechanism

| Mechanism | Behavior |
|-----------|----------|
| `CONFIG_SNAPSHOT_LIMIT` (= 20) | Hard ceiling on retained snapshots |
| Auto-rotate on create | When `create_config_snapshot` would exceed the limit, the oldest entry is removed (FIFO) |
| `prune_old_snapshots(keep_count)` | Admin can shrink the window further (keep newest `keep_count`, capped at the hard limit) |

`SnapshotCounter` is **never** decremented: pruned ids stay retired so ids remain unique forever.

### Tradeoff: storage/listing cost vs rollback history depth

| Choice | Benefit | Cost |
|--------|---------|------|
| Lower retention / aggressive prune | Bounded list CPU + smaller instance storage | Fewer historical rollback points |
| Higher retention (up to limit) | Deeper audit / restore history | Higher full-list CPU if not paginated |

**Operational guidance:** create a snapshot immediately before sensitive config changes so the known-good state stays inside the retention window. Prefer paginated `list_config_snapshots(offset, limit)` for indexers; use `list_config_snapshots_all` only for small-N / legacy callers.

### Pagination

`list_config_snapshots(offset, limit)` mirrors `view-facade::list_contracts`:

- `offset > total` → `ContractError::InvalidPagination`
- explicit `limit = 0` → `ContractError::InvalidPagination`
- `None, None` → all retained entries (safe empty `Ok([])` when none exist)
- `get_snapshot_count()` provides the total for page math

### Security notes

- `create_config_snapshot`, `prune_old_snapshots`, and `restore_config_snapshot` require admin auth and are blocked in read-only mode.
- Listing / get / compare are pure views (no auth).
- After pruning unrelated snapshots, retained ids remain fully usable for `get_config_snapshot` and `restore_config_snapshot`.
- Restoring a pruned id fails with `ContractError::SnapshotPruned`.

## Migration Steps
If modifying the schema:
1. Bump `STORAGE_SCHEMA_VERSION`.
2. Update this layout document.
3. Write `migrate` implementations that gracefully handle reading old variants and overwriting them with new variants.
4. Update `verify_storage_layout()` assertions to reflect the new requirements.
