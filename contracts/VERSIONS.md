# Contract Versions

| Version | Description | Deployed At | WASM Hash |
|---------|-------------|-------------|-----------|
| 1       | Initial deployment with basic upgrade capability. | TBD | TBD |
| 2       | Added state migration system with migration hooks, version validation, and data transformation. | TBD | TBD |

## Migration Compatibility Matrix

| From Version | To Version | Migration Required | Migration Function | Breaking Changes |
|--------------|-----------|-------------------|-------------------|------------------|
| 1 | 2 | Yes | `migrate_v1_to_v2()` | No - backward compatible |
| 2 | 3 | Yes | `migrate_v2_to_v3()` | TBD |

## Migration Process

### Overview
The state migration system allows safe contract upgrades while maintaining data compatibility. Migrations are:
- **Idempotent**: Can be run multiple times safely
- **Tracked**: Migration state is recorded to prevent double migration
- **Auditable**: All migrations emit events for audit trail
- **Versioned**: Each migration path is version-specific

### Migration Workflow

1. **Upgrade WASM**: Call `upgrade(new_wasm_hash)` to update contract code
2. **Run Migration**: Call `migrate(target_version, migration_hash)` to migrate state
3. **Verify**: Check migration state and events to confirm success

### Example Migration

```rust
// 1. Upgrade contract WASM
contract.upgrade(&env, &new_wasm_hash);

// 2. Migrate state from v1 to v2
let migration_hash = BytesN::from_array(&env, &[...]);
contract.migrate(&env, &2, &migration_hash);

// 3. Verify migration
let migration_state = contract.get_migration_state(&env);
assert_eq!(migration_state.unwrap().to_version, 2);
```

### Migration Functions

#### `migrate_v1_to_v2()`
- **Purpose**: Migrate from version 1 to version 2
- **Changes**: Adds migration state tracking
- **Data Transformation**: No data structure changes (backward compatible)
- **Status**: Implemented

#### `migrate_v2_to_v3()`
- **Purpose**: Migrate from version 2 to version 3
- **Changes**: TBD
- **Data Transformation**: TBD
- **Status**: Placeholder for future implementation

### Migration State Tracking

The contract tracks migration state to prevent:
- Double migration
- Migration rollback issues
- State corruption

Migration state includes:
- `from_version`: Version migrated from
- `to_version`: Version migrated to
- `migrated_at`: Timestamp of migration
- `migration_hash`: Hash for verification

### Rollback Support

The contract stores the previous version before upgrade to enable potential rollback:
- Previous version is stored in `PreviousVersion` key
- Can be retrieved via `get_previous_version()`
- Rollback would require upgrading back to previous WASM and handling state compatibility

### Best Practices

1. **Test First**: Always test migrations on testnet before mainnet
2. **Verify State**: Check migration state after completion
3. **Monitor Events**: Watch for migration events in your indexing system
4. **Document Changes**: Document all data structure changes between versions
5. **Backup**: Keep previous WASM hash for emergency rollback

### Migration Events

All migrations emit events with:
- `from_version`: Source version
- `to_version`: Target version
- `timestamp`: Migration timestamp
- `migration_hash`: Verification hash
- `success`: Migration success status
- `error_message`: Error details if failed

### Version Compatibility

- **v1 → v2**: Fully compatible, no breaking changes
- **v2 → v3**: TBD (to be determined when v3 is released)
# Contract Versions and Compatibility Matrix

This document defines semantic versioning (MAJOR.MINOR.PATCH) for all Grainlify contracts, tracks breaking changes, and documents migration and compatibility expectations across versions.

Contracts covered:
- grainlify-core
- program-escrow
- bounty-escrow (placeholder until stabilized)

## Versioning Policy

- MAJOR: Incompatible API/storage changes and/or required migration
- MINOR: Backward-compatible features or optional fields
- PATCH: Backward-compatible bug fixes or docs/tests only

All contracts expose both a numeric version for on-chain checks and a semantic string for off-chain tooling. Numeric encoding policy: major*10_000 + minor*100 + patch. Example: 1.2.3 => 10203.

---

## grainlify-core

Current: 1.0.0 (numeric 10000)
Planned next: 2.0.0 (numeric 20000)

| SemVer | Numeric | Date | Description | Breaking |
|--------|---------|------|-------------|----------|
| 1.0.0  | 10000   | TBA  | Initial release with admin + multisig upgrade hooks and version tracking | No |
| 1.1.0  | 10100   | TBA  | Add migration state/events and PreviousVersion, no storage schema changes | No |
| 2.0.0  | 20000   | TBA  | Introduce explicit migration API and compatibility checks, require migrate() call | Yes |

### Compatibility Matrix (grainlify-core)

| From | To | Migration | Function | Notes |
|------|----|-----------|----------|-------|
| 1.0.x | 1.1.x | No | N/A | Fully compatible; features optional |
| 1.x | 2.0.0 | Yes | migrate_v1_to_v2 | State journal introduced; emits events |
| 2.0.x | 2.1.x | No | N/A | Backward compatible feature additions |

### Migration Guide

- 1.x -> 2.0.0
  - Deploy new WASM, then call migrate(target=20000, hash)
  - Verify via get_migration_state(); ensure to_version == 20000
  - Update off-chain indexers to listen to (migration) events

Breaking changes: require explicit migrate() before using new features that rely on migrated state.

---

## program-escrow

Current: 1.0.0 (numeric 10000)

| SemVer | Numeric | Date | Description | Breaking |
|--------|---------|------|-------------|----------|
| 1.0.0  | 10000   | TBA  | Initial public release of program escrow | No |
| 1.0.1  | 10001   | TBA  | Documentation/events clarifications; no storage changes | No |

### Compatibility Matrix (program-escrow)

| From | To | Migration | Function | Notes |
|------|----|-----------|----------|-------|
| 1.0.x | 1.0.y | No | N/A | Patch only |

### Migration Guide

- 1.0.0 -> 1.0.1
  - No on-chain migration required; upgrade WASM only

---

## Global Migration Process

1. Upgrade contract WASM using upgrade(new_wasm_hash)
2. If migration is required, call migrate(target_numeric_version, migration_hash)
3. Verify with get_version(), get_migration_state()
4. Update clients to enforce minimal compatible version

### Example (grainlify-core)

```rust
// Upgrade + migrate
auth_admin.require_auth();
contract.upgrade(&env, &new_wasm_hash);
let hash = BytesN::from_array(&env, &[0u8;32]);
contract.migrate(&env, &20000, &hash);
assert_eq!(contract.get_version(&env), 20000);
```

### Events and Tracking

- migration event: (from_version, to_version, timestamp, migration_hash, success)
- monitoring metrics emitted for upgrade/migrate

### Version Checks (client guidance)

- Off-chain SDKs should enforce minimal version using numeric encoding
- On-chain functions may guard behavior with require_min_version(min_semver_numeric)

---

## Breaking Changes Log

- 2.0.0 (core): Require explicit migration; introduce MigrationState recording as hard requirement for post-2.x features.

## Notes

- WASM hashes should be recorded post-deploy in this document under the appropriate version row when known.
