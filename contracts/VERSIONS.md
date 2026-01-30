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
