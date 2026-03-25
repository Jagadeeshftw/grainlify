# Grainlify Core Migration Checklist

A step-by-step guide for safely performing version upgrades with state migration in Grainlify Core.

---

## Pre-Migration Planning (Before any code changes)

- [ ] **Define migration scope**: Identify what state changes are needed
  - What data structures change (rename, extend, or deprecate)?
  - What new storage keys are needed?
  - Are there backward compatibility concerns?

- [ ] **Review storage keys**: Confirm no existing DataKey variants will be renamed or removed
  - Verify all current DataKey enum variants in lib.rs
  - Plan new keys for v3+ using new enum variants
  - Document the rationale for any key additions

- [ ] **Identify affected storage**:
  - What instance storage keys does this affect?
  - Are there indexes or secondary lookups that need updates?
  - Will this impact multisig configuration (if applicable)?

- [ ] **Write migration tests first** (TDD):
  - Tests should verify old data can be read
  - Tests should verify transformed data is correct
  - Tests should verify migration state is recorded
  - Include tests with realistic data samples

---

## Development Phase (Writing and testing the migration)

### 1. Implement Migration Function

- [ ] **Add migrate_vN_to_vN+1() function** in lib.rs
  - Place in "Migration Functions" section
  - Add comprehensive doc comments (///)
  - Include security notes about storage key stability
  - Explain data transformation with concrete examples

- [ ] **Handle data transformation**:

  ```rust
  fn migrate_v1_to_v2(env: &Env) {
      // 1. Read old format
      // let old_data: OldType = env.storage().instance()
      //     .get(&DataKey::OldKey)
      //     .unwrap_or_default();

      // 2. Transform
      // let new_data = transform(old_data);

      // 3. Write new format
      // env.storage().instance().set(&DataKey::NewKey, &new_data);

      // 4. Cleanup (optional)
      // env.storage().instance().remove(&DataKey::OldKey);
  }
  ```

- [ ] **Preserve storage keys**:
  - ✅ SAFE: Create new DataKey enum variants
  - ✅ SAFE: Add new storage keys
  - ❌ UNSAFE: Rename DataKey variants
  - ❌ UNSAFE: Remove DataKey variants
  - ❌ UNSAFE: Change enum variant indices

### 2. Update Version Numbers

- [ ] **Increment VERSION constant** in lib.rs

  ```rust
  #[cfg(feature = "contract")]
  const VERSION: u32 = 3;  // was 2
  ```

- [ ] **Update version in Cargo.toml** (optional, for releases)
  ```toml
  [package]
  version = "0.2.0"  # was 0.1.0
  ```

### 3. Write Comprehensive Tests

- [ ] **Unit tests in migration_hook_tests.rs**:
  - [ ] Test idempotency (calling migrate() twice returns same result)
  - [ ] Test authorization (only admin can call)
  - [ ] Test version progression (from_version, to_version correct)
  - [ ] Test state preservation (admin address unchanged)
  - [ ] Test chained migrations (v1→v2→v3)

- [ ] **Integration tests in e2e_upgrade_migration_tests.rs**:
  - [ ] Test complete lifecycle (init → migrate → verify)
  - [ ] Test with realistic state (create snapshots, multisig configs)
  - [ ] Test event emission (audit trail recorded)
  - [ ] Test post-migration interactions (can execute new operations)

- [ ] **Edge case tests**:
  - [ ] Large data sets (near storage limits)
  - [ ] Concurrent operations (if applicable)
  - [ ] Invalid data handling (corrupt state recovery)
  - [ ] Rollback scenarios (if supported)

### 4. Verify Coverage Target

- [ ] **Minimum 95% coverage** for new/modified code
  - Run: `cargo tarpaulin --lib -p grainlify-core`
  - Mark unavoidable uncovered paths (e.g., panic!s, unreachable!s)
  - Document coverage gaps with comments

---

## Testnet Validation Phase

### 1. Build and Deploy

- [ ] **Compile new WASM**:

  ```bash
  cargo build --release --target wasm32-unknown-unknown -p grainlify-core
  ```

- [ ] **Upload WASM to testnet**:

  ```bash
  stellar contract install \
    --wasm target/wasm32-unknown-unknown/release/grainlify_core.wasm
  # Get hash: abc123...
  ```

- [ ] **Verify WASM hash** matches expected output
  - Compare against build artifacts
  - Ensure deterministic build (reproducible)

### 2. Execute Upgrade

- [ ] **Call upgrade() with new WASM hash**:

  ```bash
  stellar contract invoke \
    --id <contract-id> \
    --source <admin-key> \
    -- upgrade \
    --new_wasm_hash <hash>
  ```

- [ ] **Verify contract is still callable** (no panics)
  - Call get_version() → returns old version (not yet migrated)
  - Call other functions → should work with old WASM

### 3. Execute Migration

- [ ] **Call migrate() with target version**:

  ```bash
  stellar contract invoke \
    --id <contract-id> \
    --source <admin-key> \
    -- migrate \
    --target_version 2 \
    --migration_hash <hash>
  ```

- [ ] **Verify migration state** is recorded:
  - Call get_migration_state() → should show from_version, to_version, timestamp
  - Call get_version() → should return target_version

- [ ] **Verify state was preserved**:
  - Admin address unchanged
  - Multisig config intact (if applicable)
  - Custom data correctly transformed

### 4. Test New Functionality

- [ ] **Execute operations with new WASM**:
  - Test new features
  - Verify backward compatibility
  - Check event emissions

- [ ] **Verify idempotency** (safe to retry):
  - Call migrate() again with same parameters → should be no-op
  - Call migrate() with different parameters → should be rejected
  - Check migration state unchanged

### 5. Monitor for Issues

- [ ] **Check event logs** for errors:
  - Look for migration events with success=false
  - Verify error_message field for diagnostics

- [ ] **Verify storage integrity**:
  - Confirm no storage data loss
  - Verify all DataKey accesses work (no broken references)
  - Check storage size hasn't grown unexpectedly

---

## Mainnet Deployment (When ready for production)

### 1. Final Review

- [ ] **Security audit** of migration logic
  - No unsafe storage access
  - All state transitions verified
  - Authorization checks in place
  - No data loss possible

- [ ] **Performance validation** on testnet
  - Measure gas usage of upgrade + migrate
  - Confirm within expected bounds
  - No timeouts or resource exhaustion

- [ ] **Communication**:
  - [ ] Notify all integrators of pending upgrade
  - [ ] Publish upgrade timeline
  - [ ] Document breaking changes (if any)
  - [ ] Provide rollback procedure (if applicable)

### 2. Deployment Execution

- [ ] **Staged deployment** (if supported):
  - [ ] Deploy to canary contract first
  - [ ] Monitor for 24-48 hours
  - [ ] Deploy to mainnet

- [ ] **Execute upgrade** during low-activity window
  - Upload WASM
  - Call upgrade()
  - Verify WASM hash

- [ ] **Execute migration** immediately after
  - Call migrate()
  - Verify state recorded
  - Confirm new WASM operational

### 3. Post-Deployment Verification

- [ ] **Health checks**:
  - Get version → should return new version
  - Get migration state → should show successful migration
  - Call critical functions → should work correctly

- [ ] **Monitoring**:
  - Watch logs for errors
  - Monitor storage growth
  - Track transaction success rates

- [ ] **Communication**:
  - Announce successful upgrade
  - Share migration timestamp and hash
  - Provide rollback instructions (if needed)

---

## Rollback Procedure (If migration fails)

- [ ] **Identify failure**:
  - Check migration event logs for error details
  - Verify storage by calling get_version(), get_migration_state()

- [ ] **Recovery steps** (if revertible):
  - [ ] Upload previous WASM version
  - [ ] Call upgrade() with old WASM hash
  - [ ] Verify state integrity
  - [ ] Investigate root cause

- [ ] **Post-incident analysis**:
  - [ ] Document what went wrong
  - [ ] Identify prevention measures
  - [ ] Update tests to prevent recurrence
  - [ ] Share learnings with team

---

## Documentation Updates

After successful migration:

- [ ] **Update VERSIONS.md**:
  - Add new version entry
  - List breaking changes
  - Note migration path

- [ ] **Update UPGRADE_ROLLBACK_PATTERNS.md**:
  - Add examples of new migration pattern
  - Document edge cases encountered

- [ ] **Add to CHANGELOG.md** (if using semantic versioning):
  - Version number
  - Migration required: YES
  - Breaking changes (if any)
  - Upgrade procedure

- [ ] **Update architecture docs**:
  - Describe new data structures
  - Explain migration rationale
  - Document storage changes

---

## Storage Key Stability Verification

Before releasing, verify NO breaking changes to storage:

- [ ] DataKey::Admin - ✅ Stable (unchanged)
- [ ] DataKey::Version - ✅ Stable (unchanged)
- [ ] DataKey::MigrationState - ✅ Stable (unchanged)
- [ ] DataKey::PreviousVersion - ✅ Stable (unchanged)
- [ ] Any other keys - ✅ Stable (if unchanged)
- [ ] No DataKey variants removed - ✅ Verified
- [ ] No DataKey variants renamed - ✅ Verified

---

## Example: Migrating from v2 to v3

### Scenario

You're adding a new `FeatureFlag` configuration and need to initialize it during migration.

### Implementation

1. **Add new storage key** (lib.rs):

   ```rust
   enum DataKey {
       // ... existing keys ...
       FeatureFlag,  // NEW
   }
   ```

2. **Write migration function** (lib.rs):

   ```rust
   fn migrate_v2_to_v3(env: &Env) {
       // Initialize new feature flag with v2-compatible default
       env.storage().instance().set(&DataKey::FeatureFlag, &false);
   }
   ```

3. **Write tests**:

   ```rust
   #[test]
   fn migrate_v2_to_v3_initializes_feature_flag() {
       let env = Env::default();
       env.mock_all_auths();
       let (client, _) = setup_contract(&env);

       client.migrate(&3, &migration_hash(&env, 0x03));

       // New function in v3 to read feature flag
       assert!(!client.is_feature_x_enabled());
   }
   ```

4. **Update VERSION**:

   ```rust
   const VERSION: u32 = 3;  // was 2
   ```

5. **Deploy and migrate** (per checklist above)

---

## Key Principles

1. **Storage Key Stability**: Never rename or remove DataKey variants
2. **Idempotency**: Migration must be safe to call multiple times
3. **Audit Trail**: All migrations emit MigrationEvent for off-chain indexing
4. **Admin Only**: Only admin can trigger migrations
5. **No Data Loss**: Preserve all existing data during transformation
6. **Forward Compatibility**: New WASM should handle old storage gracefully
7. **Testnet First**: Always test on testnet before mainnet
8. **Documentation**: Document all breaking changes clearly

---

## Support and Issues

If you encounter migration issues:

1. **Check test results**: Review migration_hook_tests.rs and e2e_upgrade_migration_tests.rs
2. **Inspect storage keys**: Verify no DataKey variants were modified
3. **Check admin auth**: Ensure admin address is set and correct
4. **Review events**: Look at MigrationEvent logs for error details
5. **Ask for help**: Post in issues with migration state details

---

_Last updated: 2026-03-24_
_Migration tests: migration_hook_tests.rs, e2e_upgrade_migration_tests.rs_
