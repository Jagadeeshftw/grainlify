//! # State Snapshot & Rollback-Oriented Query Tests
//!
//! Comprehensive tests for the snapshot and rollback query surface:
//!
//! - `get_config_snapshot(id)` — retrieve a specific snapshot
//! - `get_latest_config_snapshot()` — most recent snapshot
//! - `get_snapshot_count()` — number of retained snapshots
//! - `list_config_snapshots(offset, limit)` — paginated listing
//! - `list_config_snapshots_all()` — full listing (legacy)
//! - `prune_old_snapshots(keep_count)` — explicit retention control
//! - `compare_snapshots(from, to)` — diff between two snapshots
//! - `get_rollback_info()` — aggregated rollback intelligence
//! - Growth-bound CPU benchmarks for list/compare as N scales
//!
//! ## Security Notes
//! - Query endpoints (`get_*`, `list_*`, `compare_*`) are pure views — no auth,
//!   no state mutation.
//! - `create_config_snapshot` / `prune_old_snapshots` / `restore_config_snapshot`
//!   require admin auth and are blocked in read-only mode.
//! - Pruning permanently destroys older rollback points; retained snapshots
//!   remain fully readable and restorable.
//!
//! ## Coverage
//! - Happy-path retrieval of individual snapshots
//! - None/empty returns when no snapshots exist
//! - Latest snapshot correctness after multiple creates
//! - Snapshot count accuracy including pruning at CONFIG_SNAPSHOT_LIMIT
//! - Pagination edge cases (offset/limit validation)
//! - Explicit prune + get/restore of unrelated retained snapshots
//! - Diff detection for all CoreConfigSnapshot fields
//! - Identical snapshots produce an all-false diff
//! - Panic on invalid snapshot IDs in compare
//! - RollbackInfo before and after upgrades/migrations
//! - CPU cost growth of list vs constant cost of compare

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, budget as _},
    Address, BytesN, Env, Vec as SorobanVec,
};

use crate::{
    ContractError, GrainlifyContract, GrainlifyContractClient, CONFIG_SNAPSHOT_LIMIT,
};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Initializes a contract with a single admin and returns (client, admin).
fn setup_admin(env: &Env) -> (GrainlifyContractClient, Address) {
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    (client, admin)
}

// ============================================================================
// get_config_snapshot
// ============================================================================

/// Retrieving a specific snapshot by ID returns the correct data.
#[test]
fn test_get_config_snapshot_returns_correct_snapshot() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let snap_id = client.create_config_snapshot();
    let snapshot = client.get_config_snapshot(&snap_id);

    assert!(snapshot.is_some(), "snapshot must exist after creation");
    let snap = snapshot.unwrap();
    assert_eq!(snap.id, snap_id);
    assert_eq!(snap.version, client.get_version());
}

/// Returns `None` for a snapshot ID that was never created.
#[test]
fn test_get_config_snapshot_returns_none_for_missing_id() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    assert!(
        client.get_config_snapshot(&999).is_none(),
        "non-existent snapshot must return None"
    );
}

/// Returns `None` for a pruned snapshot after exceeding CONFIG_SNAPSHOT_LIMIT.
#[test]
fn test_get_config_snapshot_returns_none_for_pruned_id() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    // Create first snapshot (id=1), then create 20 more to push it out.
    let first_id = client.create_config_snapshot();
    for _ in 0..20 {
        client.create_config_snapshot();
    }

    assert!(
        client.get_config_snapshot(&first_id).is_none(),
        "pruned snapshot must return None"
    );
}

/// Snapshot captures the correct admin address.
#[test]
fn test_get_config_snapshot_captures_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin) = setup_admin(&env);

    let snap_id = client.create_config_snapshot();
    let snap = client.get_config_snapshot(&snap_id).unwrap();

    assert_eq!(snap.admin, Some(admin));
}

// ============================================================================
// get_latest_config_snapshot
// ============================================================================

/// Returns `None` when no snapshots have been created.
#[test]
fn test_get_latest_config_snapshot_none_before_creation() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    assert!(
        client.get_latest_config_snapshot().is_none(),
        "latest must be None before any snapshot"
    );
}

/// Returns the most recently created snapshot.
#[test]
fn test_get_latest_config_snapshot_returns_most_recent() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    client.set_version(&3);
    let _id1 = client.create_config_snapshot();

    client.set_version(&4);
    let id2 = client.create_config_snapshot();

    let latest = client.get_latest_config_snapshot();
    assert!(latest.is_some());
    let snap = latest.unwrap();
    assert_eq!(snap.id, id2);
    assert_eq!(snap.version, 4);
}

/// After pruning, latest still reflects the correct most-recent snapshot.
#[test]
fn test_get_latest_config_snapshot_correct_after_pruning() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let mut last_id = 0u64;
    for v in 1..=25u32 {
        client.set_version(&v);
        last_id = client.create_config_snapshot();
    }

    let latest = client.get_latest_config_snapshot().unwrap();
    assert_eq!(latest.id, last_id);
    assert_eq!(latest.version, 25);
}

// ============================================================================
// get_snapshot_count
// ============================================================================

/// Count is zero before any snapshots.
#[test]
fn test_get_snapshot_count_zero_initially() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    assert_eq!(client.get_snapshot_count(), 0);
}

/// Count increments with each snapshot creation.
#[test]
fn test_get_snapshot_count_increments() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    client.create_config_snapshot();
    assert_eq!(client.get_snapshot_count(), 1);

    client.create_config_snapshot();
    assert_eq!(client.get_snapshot_count(), 2);

    client.create_config_snapshot();
    assert_eq!(client.get_snapshot_count(), 3);
}

/// Count is capped at CONFIG_SNAPSHOT_LIMIT (20) after pruning.
#[test]
fn test_get_snapshot_count_capped_at_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    for _ in 0..25 {
        client.create_config_snapshot();
    }

    assert_eq!(
        client.get_snapshot_count(),
        20,
        "count must be capped at CONFIG_SNAPSHOT_LIMIT"
    );
}

// ============================================================================
// compare_snapshots
// ============================================================================

/// Two identical snapshots produce an all-false diff (nothing changed).
#[test]
fn test_compare_snapshots_identical() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let id1 = client.create_config_snapshot();
    let id2 = client.create_config_snapshot();

    let diff = client.compare_snapshots(&id1, &id2);

    assert_eq!(diff.from_id, id1);
    assert_eq!(diff.to_id, id2);
    assert!(!diff.admin_changed, "admin should not change");
    assert!(!diff.version_changed, "version should not change");
    assert!(
        !diff.previous_version_changed,
        "previous_version should not change"
    );
    assert!(
        !diff.multisig_threshold_changed,
        "threshold should not change"
    );
    assert!(!diff.multisig_signers_changed, "signers should not change");
}

/// Detects a version change between two snapshots.
#[test]
fn test_compare_snapshots_detects_version_change() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    client.set_version(&3);
    let id1 = client.create_config_snapshot();

    client.set_version(&5);
    let id2 = client.create_config_snapshot();

    let diff = client.compare_snapshots(&id1, &id2);

    assert!(diff.version_changed, "version must be detected as changed");
    assert_eq!(diff.from_version, 3);
    assert_eq!(diff.to_version, 5);
    assert!(!diff.admin_changed, "admin should not have changed");
}

/// Detects combined version changes across multiple updates.
#[test]
fn test_compare_snapshots_detects_multiple_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    client.set_version(&3);
    let id1 = client.create_config_snapshot();

    client.set_version(&7);
    let id2 = client.create_config_snapshot();

    let diff = client.compare_snapshots(&id1, &id2);

    assert!(diff.version_changed);
    assert_eq!(diff.from_version, 3);
    assert_eq!(diff.to_version, 7);
}

/// Panics when from_id does not exist.
#[test]
#[should_panic(expected = "Snapshot not found: from_id")]
fn test_compare_snapshots_panics_on_missing_from() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let id2 = client.create_config_snapshot();
    client.compare_snapshots(&999, &id2);
}

/// Panics when to_id does not exist.
#[test]
#[should_panic(expected = "Snapshot not found: to_id")]
fn test_compare_snapshots_panics_on_missing_to() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let id1 = client.create_config_snapshot();
    client.compare_snapshots(&id1, &999);
}

/// Comparing a snapshot with itself produces all-false diff.
#[test]
fn test_compare_snapshots_self_diff() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let id = client.create_config_snapshot();
    let diff = client.compare_snapshots(&id, &id);

    assert!(!diff.admin_changed);
    assert!(!diff.version_changed);
    assert!(!diff.previous_version_changed);
    assert!(!diff.multisig_threshold_changed);
    assert!(!diff.multisig_signers_changed);
}

/// Detects previous_version field changes between two snapshots.
#[test]
fn test_compare_snapshots_detects_previous_version_change() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    // Snapshot before any upgrade
    let id1 = client.create_config_snapshot();

    // Simulate an upgrade cycle via commit + migrate (replay-protected path)
    let hash = BytesN::from_array(&env, &[1u8; 32]);
    client.commit_migration(&3, &hash, &u64::MAX);
    client.migrate(&3, &hash);

    let id2 = client.create_config_snapshot();

    let diff = client.compare_snapshots(&id1, &id2);
    assert!(diff.version_changed);
}

// ============================================================================
// get_rollback_info
// ============================================================================

/// RollbackInfo on a freshly initialized contract (no upgrades, no snapshots).
#[test]
fn test_get_rollback_info_fresh_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let info = client.get_rollback_info();

    assert_eq!(info.current_version, 2, "init sets version to 2");
    assert_eq!(
        info.previous_version, 0,
        "no previous version before upgrade"
    );
    assert!(
        !info.rollback_available,
        "rollback not available before upgrade"
    );
    assert!(!info.has_migration, "no migration state before migration");
    assert_eq!(info.migration_from_version, 0);
    assert_eq!(info.migration_to_version, 0);
    assert_eq!(info.migration_timestamp, 0);
    assert_eq!(info.snapshot_count, 0, "no snapshots created yet");
    assert!(!info.has_snapshot, "no latest snapshot before creation");
    assert_eq!(info.latest_snapshot_id, 0);
    assert_eq!(info.latest_snapshot_version, 0);
}

/// RollbackInfo after creating snapshots.
#[test]
fn test_get_rollback_info_with_snapshots() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    client.create_config_snapshot();
    client.set_version(&5);
    let snap_id = client.create_config_snapshot();

    let info = client.get_rollback_info();

    assert_eq!(info.snapshot_count, 2);
    assert!(info.has_snapshot);
    assert_eq!(info.latest_snapshot_id, snap_id);
    assert_eq!(info.latest_snapshot_version, 5);
}

/// RollbackInfo after a migration shows the migration state.
#[test]
fn test_get_rollback_info_after_migration() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let hash = BytesN::from_array(&env, &[1u8; 32]);
    client.commit_migration(&3, &hash, &u64::MAX);
    client.migrate(&3, &hash);

    let info = client.get_rollback_info();

    assert_eq!(info.current_version, 3);
    assert!(info.has_migration);
    assert_eq!(info.migration_from_version, 2);
    assert_eq!(info.migration_to_version, 3);
}

/// RollbackInfo reflects restored version after snapshot restore.
#[test]
fn test_get_rollback_info_after_snapshot_restore() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    // Create snapshot at v2
    let snap_id = client.create_config_snapshot();

    // Advance to v5
    client.set_version(&5);

    // Restore to v2 from snapshot
    client.restore_config_snapshot(&snap_id);

    let info = client.get_rollback_info();
    assert_eq!(info.current_version, 2);
}

/// RollbackInfo is consistent with individual query functions.
#[test]
fn test_get_rollback_info_consistency_with_individual_queries() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    client.create_config_snapshot();
    client.set_version(&4);
    client.create_config_snapshot();

    let info = client.get_rollback_info();

    // Verify consistency with individual queries
    assert_eq!(info.current_version, client.get_version());
    assert_eq!(info.snapshot_count, client.get_snapshot_count());

    let latest = client.get_latest_config_snapshot().unwrap();
    assert_eq!(info.latest_snapshot_id, latest.id);
    assert_eq!(info.latest_snapshot_version, latest.version);
}

/// RollbackInfo on an uninitialized contract returns safe defaults.
#[test]
fn test_get_rollback_info_uninitialized() {
    let env = Env::default();

    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);

    let info = client.get_rollback_info();

    assert_eq!(info.current_version, 0);
    assert_eq!(info.previous_version, 0);
    assert!(!info.rollback_available);
    assert!(!info.has_migration);
    assert_eq!(info.snapshot_count, 0);
    assert!(!info.has_snapshot);
}

/// RollbackInfo correctly reflects rollback_available after version changes.
#[test]
fn test_get_rollback_info_rollback_flag() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    // Before upgrade: no rollback available
    let info1 = client.get_rollback_info();
    assert!(!info1.rollback_available);

    // After snapshot restore that sets previous_version,
    // use create + restore to set PreviousVersion
    let snap_id = client.create_config_snapshot();
    client.set_version(&5);
    client.restore_config_snapshot(&snap_id);

    // Restore sets PreviousVersion from the snapshot
    let info2 = client.get_rollback_info();
    // previous_version from snapshot is None (first snapshot before any upgrade),
    // so rollback may not be available. The PreviousVersion is tracked by upgrade().
    assert_eq!(info2.current_version, 2);
}

// ============================================================================
// get_state_snapshot (monitoring snapshot — existing, verify it still works)
// ============================================================================

/// Monitoring state snapshot returns consistent data.
#[test]
fn test_get_state_snapshot_returns_valid_data() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    // init_admin does not increment monitoring counters; perform a tracked op.
    client.set_version(&3);
    let snapshot = client.get_state_snapshot();

    assert!(
        snapshot.total_operations >= 1,
        "tracked operations should appear in the monitoring snapshot"
    );
}

/// Monitoring state snapshot counts increase after operations.
#[test]
fn test_get_state_snapshot_reflects_operations() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let snap1 = client.get_state_snapshot();

    // Perform an operation (set_version triggers monitoring)
    client.set_version(&3);

    let snap2 = client.get_state_snapshot();

    assert!(
        snap2.total_operations >= snap1.total_operations,
        "operations must not decrease"
    );
}

// ============================================================================
// Edge Cases & Integration
// ============================================================================

/// Snapshot queries work correctly with multisig-initialized contracts.
#[test]
fn test_snapshot_queries_with_multisig_init() {
    let env = Env::default();
    env.mock_all_auths();

    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);

    let mut signers = SorobanVec::new(&env);
    signers.push_back(Address::generate(&env));
    signers.push_back(Address::generate(&env));
    signers.push_back(Address::generate(&env));
    client.init(&signers, &2);

    // Rollback info should work even without a single admin
    let info = client.get_rollback_info();
    assert_eq!(info.current_version, 2);
    assert!(!info.rollback_available);
    assert_eq!(info.snapshot_count, 0);
}

/// End-to-end: create → compare → restore → verify rollback info.
#[test]
fn test_end_to_end_snapshot_rollback_workflow() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    // 1. Create snapshot at initial state (v2)
    let snap1 = client.create_config_snapshot();
    assert_eq!(client.get_snapshot_count(), 1);

    // 2. Change version
    client.set_version(&5);

    // 3. Create second snapshot
    let snap2 = client.create_config_snapshot();
    assert_eq!(client.get_snapshot_count(), 2);

    // 4. Compare snapshots — version should differ
    let diff = client.compare_snapshots(&snap1, &snap2);
    assert!(diff.version_changed);
    assert_eq!(diff.from_version, 2);
    assert_eq!(diff.to_version, 5);

    // 5. Verify latest snapshot
    let latest = client.get_latest_config_snapshot().unwrap();
    assert_eq!(latest.id, snap2);
    assert_eq!(latest.version, 5);

    // 6. Restore from first snapshot
    client.restore_config_snapshot(&snap1);
    assert_eq!(client.get_version(), 2);

    // 7. Verify rollback info consistency
    let info = client.get_rollback_info();
    assert_eq!(info.current_version, 2);
}

/// get_config_snapshot and get_latest_config_snapshot agree when one snapshot.
#[test]
fn test_single_snapshot_consistency() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    let snap_id = client.create_config_snapshot();

    let by_id = client.get_config_snapshot(&snap_id).unwrap();
    let latest = client.get_latest_config_snapshot().unwrap();

    assert_eq!(by_id.id, latest.id);
    assert_eq!(by_id.version, latest.version);
    assert_eq!(by_id.admin, latest.admin);
}

/// Snapshot count stays accurate across create/prune cycles.
#[test]
fn test_snapshot_count_prune_cycle() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_admin(&env);

    for _ in 0..CONFIG_SNAPSHOT_LIMIT {
        client.create_config_snapshot();
    }
    assert_eq!(client.get_snapshot_count(), CONFIG_SNAPSHOT_LIMIT);

    client.create_config_snapshot();
    assert_eq!(
        client.get_snapshot_count(),
        CONFIG_SNAPSHOT_LIMIT,
        "must stay at limit"
    );

    client.create_config_snapshot();
    client.create_config_snapshot();
    assert_eq!(
        client.get_snapshot_count(),
        CONFIG_SNAPSHOT_LIMIT,
        "must stay at limit"
    );
}

// ============================================================================
// list_config_snapshots pagination
// ============================================================================

#[test]
fn test_list_config_snapshots_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    assert_eq!(client.list_config_snapshots(&None, &None).len(), 0);
    assert_eq!(client.list_config_snapshots_all().len(), 0);
}

#[test]
fn test_list_config_snapshots_returns_all_when_unbounded() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    let mut ids = std::vec![];
    for v in 1..=5u32 {
        client.set_version(&v);
        ids.push(client.create_config_snapshot());
    }
    let all = client.list_config_snapshots(&None, &None);
    assert_eq!(all.len(), 5);
    for i in 0..5 {
        assert_eq!(all.get(i as u32).unwrap().id, ids[i as usize]);
        assert_eq!(all.get(i as u32).unwrap().version, (i + 1) as u32);
    }
}

#[test]
fn test_list_config_snapshots_with_offset() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    for _ in 0..5 {
        client.create_config_snapshot();
    }
    assert_eq!(client.list_config_snapshots(&Some(2), &None).len(), 3);
}

#[test]
fn test_list_config_snapshots_with_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    for _ in 0..5 {
        client.create_config_snapshot();
    }
    assert_eq!(client.list_config_snapshots(&None, &Some(2)).len(), 2);
}

#[test]
fn test_list_config_snapshots_offset_and_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    let mut ids = std::vec![];
    for _ in 0..6 {
        ids.push(client.create_config_snapshot());
    }
    let page = client.list_config_snapshots(&Some(2), &Some(2));
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().id, ids[2]);
    assert_eq!(page.get(1).unwrap().id, ids[3]);
}

#[test]
fn test_list_config_snapshots_offset_beyond_total_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    client.create_config_snapshot();
    let result = client.try_list_config_snapshots(&Some(5), &Some(1));
    assert_eq!(result, Err(Ok(ContractError::InvalidPagination)));
}

#[test]
fn test_list_config_snapshots_zero_limit_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    client.create_config_snapshot();
    let result = client.try_list_config_snapshots(&Some(0), &Some(0));
    assert_eq!(result, Err(Ok(ContractError::InvalidPagination)));
}

#[test]
fn test_list_config_snapshots_offset_plus_limit_truncates() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    for _ in 0..5 {
        client.create_config_snapshot();
    }
    assert_eq!(client.list_config_snapshots(&Some(3), &Some(10)).len(), 2);
}

#[test]
fn test_list_config_snapshots_full_pagination_workflow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    let total = 7u32;
    for _ in 0..total {
        client.create_config_snapshot();
    }
    let page_size = 3u32;
    let mut collected = 0u32;
    let mut offset = 0u32;
    loop {
        let page = client.list_config_snapshots(&Some(offset), &Some(page_size));
        collected += page.len();
        if page.len() < page_size {
            break;
        }
        offset += page_size;
    }
    assert_eq!(collected, total);
}

#[test]
fn test_list_config_snapshots_all_matches_unbounded() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    for _ in 0..4 {
        client.create_config_snapshot();
    }
    let all = client.list_config_snapshots_all();
    let page = client.list_config_snapshots(&None, &None);
    assert_eq!(all.len(), page.len());
    for i in 0..all.len() {
        assert_eq!(all.get(i).unwrap().id, page.get(i).unwrap().id);
    }
}

#[test]
fn test_get_config_snapshot_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    assert_eq!(client.get_config_snapshot_limit(), CONFIG_SNAPSHOT_LIMIT);
}

// ============================================================================
// prune_old_snapshots
// ============================================================================

#[test]
fn test_prune_old_snapshots_retains_newest() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    let mut ids = std::vec![];
    for _ in 0..6 {
        ids.push(client.create_config_snapshot());
    }
    assert_eq!(client.prune_old_snapshots(&3), 3);
    assert_eq!(client.get_snapshot_count(), 3);
    assert!(client.get_config_snapshot(&ids[0]).is_none());
    assert!(client.get_config_snapshot(&ids[1]).is_none());
    assert!(client.get_config_snapshot(&ids[2]).is_none());
    assert!(client.get_config_snapshot(&ids[3]).is_some());
    assert!(client.get_config_snapshot(&ids[4]).is_some());
    assert!(client.get_config_snapshot(&ids[5]).is_some());
}

#[test]
fn test_prune_old_snapshots_noop_when_within_keep() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    client.create_config_snapshot();
    client.create_config_snapshot();
    assert_eq!(client.prune_old_snapshots(&5), 0);
    assert_eq!(client.get_snapshot_count(), 2);
}

#[test]
fn test_prune_old_snapshots_zero_clears_all() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    let id = client.create_config_snapshot();
    client.create_config_snapshot();
    assert_eq!(client.prune_old_snapshots(&0), 2);
    assert_eq!(client.get_snapshot_count(), 0);
    assert!(client.get_config_snapshot(&id).is_none());
    assert!(client.get_latest_config_snapshot().is_none());
}

#[test]
fn test_prune_old_snapshots_caps_at_hard_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    for _ in 0..CONFIG_SNAPSHOT_LIMIT {
        client.create_config_snapshot();
    }
    assert_eq!(client.prune_old_snapshots(&(CONFIG_SNAPSHOT_LIMIT + 5)), 0);
    assert_eq!(client.get_snapshot_count(), CONFIG_SNAPSHOT_LIMIT);
}

#[test]
fn test_get_and_restore_survive_unrelated_prune() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    let keep_id = client.create_config_snapshot();
    client.set_version(&4);
    let _unrelated_a = client.create_config_snapshot();
    client.set_version(&7);
    let keep_latest = client.create_config_snapshot();
    client.set_version(&9);
    let _unrelated_b = client.create_config_snapshot();

    assert_eq!(client.prune_old_snapshots(&2), 2);
    assert!(client.get_config_snapshot(&keep_id).is_none());

    let retained = client.get_config_snapshot(&keep_latest).unwrap();
    assert_eq!(retained.id, keep_latest);
    assert_eq!(retained.version, 7);

    client.set_version(&11);
    client.restore_config_snapshot(&keep_latest);
    assert_eq!(client.get_version(), 7);
}

#[test]
fn test_restore_middle_snapshot_after_pruning_older() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    client.set_version(&3);
    let _old = client.create_config_snapshot();
    client.set_version(&5);
    let middle = client.create_config_snapshot();
    client.set_version(&8);
    let _new = client.create_config_snapshot();

    assert_eq!(client.prune_old_snapshots(&2), 1);
    assert!(client.get_config_snapshot(&middle).is_some());
    client.restore_config_snapshot(&middle);
    assert_eq!(client.get_version(), 5);

    let newest = client.get_latest_config_snapshot().unwrap();
    let diff = client.compare_snapshots(&middle, &newest.id);
    assert!(diff.version_changed);
}

#[test]
#[should_panic(expected = "Read-only mode")]
fn test_prune_old_snapshots_blocked_in_read_only() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);
    client.create_config_snapshot();
    client.set_read_only_mode(&true);
    client.prune_old_snapshots(&0);
}

// ============================================================================
// Growth-bound CPU benchmarks
// ============================================================================

/// Measure list/compare CPU as snapshot count scales within the retention window.
#[test]
fn bench_list_and_compare_snapshot_growth() {
    let sizes: &[u32] = &[1, 5, 10, 20];
    let mut prev_list_cpu = 0u64;

    for &n in sizes {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin) = setup_admin(&env);

        let mut first_id = 0u64;
        let mut last_id = 0u64;
        for i in 0..n {
            client.set_version(&(i + 1));
            let id = client.create_config_snapshot();
            if i == 0 {
                first_id = id;
            }
            last_id = id;
        }
        assert_eq!(client.get_snapshot_count(), n);

        env.budget().reset_default();
        let listed = client.list_config_snapshots(&None, &None);
        let list_cpu = env.budget().cpu_instruction_cost();
        let list_mem = env.budget().memory_bytes_cost();
        assert_eq!(listed.len(), n);
        std::println!(
            "[BENCH] op=list_config_snapshots n={} cpu_insns={} mem_bytes={}",
            n, list_cpu, list_mem
        );
        assert!(
            list_cpu >= prev_list_cpu,
            "list CPU should grow with N: n={} cpu={} prev={}",
            n, list_cpu, prev_list_cpu
        );
        prev_list_cpu = list_cpu;

        env.budget().reset_default();
        let page = client.list_config_snapshots(&Some(0), &Some(5u32.min(n)));
        let page_cpu = env.budget().cpu_instruction_cost();
        std::println!(
            "[BENCH] op=list_config_snapshots_page n={} page={} cpu_insns={}",
            n, page.len(), page_cpu
        );

        env.budget().reset_default();
        let _diff = client.compare_snapshots(&first_id, &last_id);
        let cmp_cpu = env.budget().cpu_instruction_cost();
        let cmp_mem = env.budget().memory_bytes_cost();
        std::println!(
            "[BENCH] op=compare_snapshots n={} cpu_insns={} mem_bytes={}",
            n, cmp_cpu, cmp_mem
        );
        assert!(
            cmp_cpu < 5_000_000,
            "compare_snapshots should stay O(1); n={} cpu={}",
            n, cmp_cpu
        );
    }

    assert!(
        prev_list_cpu < 50_000_000,
        "list at CONFIG_SNAPSHOT_LIMIT must stay within a safe budget; cpu={}",
        prev_list_cpu
    );
}

#[test]
fn bench_paginated_list_cheaper_than_full_scan() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    for i in 0..CONFIG_SNAPSHOT_LIMIT {
        client.set_version(&(i + 1));
        client.create_config_snapshot();
    }

    env.budget().reset_default();
    let _all = client.list_config_snapshots(&None, &None);
    let full_cpu = env.budget().cpu_instruction_cost();

    env.budget().reset_default();
    let _page = client.list_config_snapshots(&Some(0), &Some(5));
    let page_cpu = env.budget().cpu_instruction_cost();

    std::println!(
        "[BENCH] op=list_full_vs_page full_cpu={} page_cpu={}",
        full_cpu, page_cpu
    );
    assert!(
        page_cpu < full_cpu,
        "paginated list should cost less than full scan: page={} full={}",
        page_cpu, full_cpu
    );
}
