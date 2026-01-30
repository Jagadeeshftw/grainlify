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
