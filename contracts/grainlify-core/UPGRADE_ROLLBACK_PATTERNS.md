# Upgrade and Rollback Patterns for Grainlify Core

This document describes the upgrade and rollback patterns implemented in the Grainlify Core contract, including the authorization matrix and executable regression tests.

## Authorization Decision

Upgrades and rollbacks use the same authority: the configured multisig signer
threshold followed by the configured timelock. The legacy single-admin
`upgrade` entrypoint authenticates the stored admin but then rejects; it cannot
change WASM. A rollback is a normal multisig proposal whose hash points to a
previous deployment. No emergency role can bypass the timelock.

| State | Signer action | Non-signer action | WASM change |
|-------|---------------|-------------------|-------------|
| Pending | Approve or cancel | Rejected | No |
| Executable | Threshold is met | Cannot alter proposal | Only after timelock |
| Expired | Rejected | Rejected | No |
| Cancelled | Rejected | Rejected | No |

The matrix is tested in `src/test/upgrade_authorization_matrix.rs`. Rejection
tests assert unchanged proposal data, version, previous version, and timelock.

## Overview

The Grainlify Core contract supports secure WASM upgrades with the ability to rollback to previous versions. This is critical for:

- Bug fixes in production
- Feature rollouts
- Emergency rollbacks
- State migration between versions

## Architecture

### WASM Hash Management

When you compile a Soroban contract, you can upload the WASM to the Stellar network and receive a 32-byte hash. This hash can be reused multiple times without re-uploading the WASM.

```rust
// Upload WASM once
let wasm_hash_v1 = env.deployer().upload_contract_wasm(WASM_V1);

// Use the hash multiple times
contract.upgrade(&wasm_hash_v1);  // First time
// ... later ...
contract.upgrade(&wasm_hash_v1);  // Rollback using same hash
```

### Version Tracking

The contract tracks:

1. **Current Version** (`DataKey::Version`) - The active version number
2. **Previous Version** (`DataKey::PreviousVersion`) - The version before the last upgrade
3. **Migration State** (`DataKey::MigrationState`) - State migration metadata

### Instance Storage Persistence

All instance storage persists across WASM upgrades:

- Admin address
- Version numbers
- Migration state
- Any custom contract data

## Upgrade Patterns

### Pattern 1: Multisig Upgrade

```rust
// Initialize a 2-of-3 multisig
let signers = vec![signer1, signer2, signer3];
client.init(&signers, &2);

// Upload new WASM or reuse a previously deployed hash
let new_wasm_hash = upload_new_version_wasm(&env);

// Propose, approve, wait for the timelock, then execute
let proposal_id = client.propose_upgrade(&signer1, &new_wasm_hash, &expiry);
client.approve_upgrade(&proposal_id, &signer1);
client.approve_upgrade(&proposal_id, &signer2);
client.execute_upgrade(&proposal_id);

// Review the persisted proposal metadata before execution.
let proposal = client.get_upgrade_proposal(&proposal_id).unwrap();
assert_eq!(proposal.proposal_id, proposal_id);
assert_eq!(proposal.proposer, Some(signer1.clone()));
assert_eq!(proposal.wasm_hash, new_wasm_hash);

// Collect approvals
client.approve_upgrade(&proposal_id, &signer1);
client.approve_upgrade(&proposal_id, &signer2);

```

### Pattern 2: Upgrade with Migration

```rust
// Run migration after the multisig upgrade executes

// Run migration
let migration_hash = BytesN::from_array(&env, &[...]);
client.migrate(&3, &migration_hash);

// Migration is idempotent - safe to call multiple times
client.migrate(&3, &migration_hash);  // No-op if already migrated
```

## Rollback Patterns

### Pattern 1: Rollback through the same multisig timelock

Keep the previous WASM hash for emergency rollback:

```rust
// Before upgrade, save current hash
let current_wasm_hash = get_current_wasm_hash();  // Store this safely

// If issues are detected, propose the previous hash and wait for the same timelock
let rollback = client.propose_upgrade(&signer1, &current_wasm_hash, &expiry);
client.approve_upgrade(&rollback, &signer1);
client.approve_upgrade(&rollback, &signer2);
client.execute_upgrade(&rollback);

// Optionally restore version number
let prev_version = client.get_previous_version().unwrap();
client.set_version(&prev_version);
```

### Pattern 2: Rollback proposal details

```rust
// Propose rollback to previous WASM
let rollback_proposal = client.propose_upgrade(&signer1, &previous_wasm_hash, &expiry);

// Proposal ids are monotonic and remain stable across the full lifecycle.
let rollback_state = client.get_upgrade_proposal(&rollback_proposal).unwrap();
assert_eq!(rollback_state.proposer, Some(signer1.clone()));
assert_eq!(rollback_state.wasm_hash, previous_wasm_hash);

// Use the normal threshold approvals; rollback has no emergency bypass
client.approve_upgrade(&rollback_proposal, &signer1);
client.approve_upgrade(&rollback_proposal, &signer2);

// Execute rollback
client.execute_upgrade(&rollback_proposal);
```

### Pattern 3: Rollback with State Preservation

```rust
// Migration state persists across rollbacks
client.migrate(&3, &migration_hash);

// Rollback through a multisig proposal; migrated state remains in storage
let rollback = client.propose_upgrade(&signer1, &old_wasm_hash, &expiry);
client.approve_upgrade(&rollback, &signer1);
client.approve_upgrade(&rollback, &signer2);
client.execute_upgrade(&rollback);

// Migration state is still accessible
let state = client.get_migration_state();
assert_eq!(state.unwrap().to_version, 3);
```

## Test Coverage

### Implemented Tests

1. **test_wasm_upload_returns_valid_hash** ✅
   - Verifies WASM upload produces valid 32-byte hash

2. **test_wasm_hash_reuse_without_reuploading** ✅
   - Confirms same WASM produces same hash
   - Tests hash caching behavior

3. **test_multisig_upgrade_proposal** ✅
   - Tests multisig proposal creation
   - Verifies approval threshold
   - Confirms upgrade execution

4. **test_multisig_rollback_proposal** ✅
   - Tests rollback via multisig
   - Verifies multiple upgrade/rollback cycles

### Additional Test Scenarios

The following scenarios are documented and should be tested in integration environments:

5. **Previous Version Tracking**
   - Verifies `get_previous_version()` returns correct value after upgrade

6. **Version Persistence**
   - Confirms version numbers persist across WASM upgrades

7. **Admin Persistence**
   - Ensures admin can still operate after upgrade

8. **Migration State Persistence**
   - Verifies migration state survives upgrades and rollbacks

9. **Event Emission**
   - Confirms upgrade operations emit appropriate events

10. **Multiple Sequential Upgrades**
    - Tests stability across many upgrade cycles

11. **Upgrade with Version Update**
    - Tests coordinated WASM and version number updates

12. **Instance Storage Preservation**
    - Comprehensive test of all storage persistence

## Best Practices

### Before Upgrade

1. **Test on Testnet**

   ```bash
   # Deploy to testnet
   stellar contract deploy --wasm new_version.wasm --network testnet

   # Test thoroughly
   stellar contract invoke --id CONTRACT_ID --network testnet -- upgrade --new_wasm_hash HASH
   ```

2. **Save Current WASM Hash**

   ```rust
   // Store this in a secure location
   let rollback_hash = current_wasm_hash;
   ```

3. **Document Changes**
   - List breaking changes
   - Document migration requirements
   - Note rollback procedures

### During Upgrade

1. **Use Multisig for Production**
   - Require multiple approvals
   - Implement timelock if needed

2. **Monitor Events**
   - Watch for upgrade events
   - Verify success before proceeding

3. **Verify State**
   ```rust
   // After upgrade
   assert_eq!(client.get_version(), expected_version);
   let state = client.get_migration_state();
   assert!(state.is_some());
   ```

### After Upgrade

1. **Run Health Checks**

   ```rust
   let health = client.health_check();
   assert!(health.is_healthy);
   ```

2. **Monitor Analytics**

   ```rust
   let analytics = client.get_analytics();
   // Check error rates, operation counts
   ```

### Monitoring Field Meanings

`health_check()` returns a bounded, read-only status object for operators:

- `is_healthy`: `true` only when the contract's configuration and monitoring invariants hold.
- `last_operation`: Ledger timestamp of the last tracked operation. Returns `0` before any tracked operation exists.
- `total_operations`: Total tracked operations, including tracked failures.
- `contract_version`: Semantic version string derived from the contract's stored version. Returns `0.0.0` before initialization.

`get_analytics()` returns bounded aggregate counters for dashboards:

- `operation_count`: Total tracked operations.
- `unique_users`: Distinct tracked callers retained in the bounded monitoring index. This count saturates once the tracked-user cap is reached, so it remains storage-safe on-chain.
- `error_count`: Total tracked failed operations.
- `error_rate`: Failure rate in basis points where `10000 == 100%` and `250 == 2.5%`.

These views are intentionally panic-free on empty state so monitoring systems can poll a deployed-but-not-yet-initialized contract safely.

3. **Keep Rollback Ready**
   - Maintain previous WASM hash for 24-48 hours
   - Have rollback procedure documented
   - Test rollback in staging

## Emergency Procedures

### Immediate Rollback

If critical issues are detected:

```bash
# 1. Get previous WASM hash (from deployment records)
PREV_HASH="abc123..."

# 2. Execute rollback
stellar contract invoke \
  --id CONTRACT_ID \
  --source ADMIN_KEY \
  -- upgrade \
  --new_wasm_hash $PREV_HASH

# 3. Verify rollback
stellar contract invoke \
  --id CONTRACT_ID \
  -- get_version
```

### Multisig Emergency Rollback

```bash
# 1. Propose rollback
stellar contract invoke \
  --id CONTRACT_ID \
  --source SIGNER1_KEY \
  -- propose_upgrade \
  --proposer SIGNER1_ADDR \
  --wasm_hash $PREV_HASH

# 2. Fast-track approvals
stellar contract invoke --id CONTRACT_ID --source SIGNER1_KEY \
   -- approve_upgrade --proposal_id "$PROPOSAL_ID" --signer SIGNER1_ADDR

stellar contract invoke --id CONTRACT_ID --source SIGNER2_KEY \
   -- approve_upgrade --proposal_id "$PROPOSAL_ID" --signer SIGNER2_ADDR

# 3. Execute
stellar contract invoke --id CONTRACT_ID \
   -- execute_upgrade --proposal_id "$PROPOSAL_ID"
```

## Version History

- **v1.0.0**: Initial release
- **v2.0.0**: Added migration system, multisig support
- **v2.1.0**: Enhanced monitoring and analytics

## References

- [Soroban Upgrade Documentation](https://soroban.stellar.org/docs/learn/smart-contract-internals/contract-lifecycle)
- [Contract Deployment Guide](../GRAINLIFY_BUILD.md)
- [Migration System](../VERSIONS.md)
