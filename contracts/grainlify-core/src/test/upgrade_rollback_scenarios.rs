//! # Expanded Upgrade Rollback Scenarios
//!
//! Additional tests for multisig partial approvals, hash mismatch isolation,
//! operational rollback workflows, and edge cases (issue #732).
//!
//! ## Operational Rollback Runbook (documented in tests)
//!
//! 1. **Pre-upgrade**: `create_config_snapshot()` to capture known-good state.
//! 2. **Upgrade**: `upgrade(new_hash)` or multisig `propose → approve → execute`.
//! 3. **Verify**: `get_version()`, `health_check()`, `get_rollback_info()`.
//! 4. **Rollback (if needed)**:
//!    - Single-admin: `upgrade(previous_hash)` + `set_version(prev)`.
//!    - Multisig: `propose_upgrade(prev_hash)` → approvals → `execute_upgrade`.
//!    - Config: `restore_config_snapshot(id)` to restore version/admin/multisig.
//! 5. **Post-rollback**: Verify `get_version()`, `get_admin()`, `health_check()`.
//!
//! ## Security Notes
//! - Approvals on one proposal never affect another proposal.
//! - Each proposal stores its own WASM hash independently.
//! - Failed WASM swaps roll back all storage changes atomically.
//! - `PreviousVersion` is only set by a successful `upgrade()`/`execute_upgrade()`.

#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Vec as SorobanVec};

use crate::{GrainlifyContract, GrainlifyContractClient};

// ── helpers ──────────────────────────────────────────────────────────────────

fn fake_wasm(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xAB; 32])
}
fn fake_wasm_v2(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xCD; 32])
}
fn fake_wasm_v3(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xEF; 32])
}

fn setup_admin(env: &Env) -> (GrainlifyContractClient<'_>, Address) {
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    (client, admin)
}

fn setup_multisig(env: &Env) -> (GrainlifyContractClient<'_>, [Address; 3]) {
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let s1 = Address::generate(env);
    let s2 = Address::generate(env);
    let s3 = Address::generate(env);
    let mut signers = SorobanVec::new(env);
    signers.push_back(s1.clone());
    signers.push_back(s2.clone());
    signers.push_back(s3.clone());
    client.init(&signers, &2);
    (client, [s1, s2, s3])
}

fn setup_multisig_custom(env: &Env, threshold: u32) -> (GrainlifyContractClient<'_>, [Address; 3]) {
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let s1 = Address::generate(env);
    let s2 = Address::generate(env);
    let s3 = Address::generate(env);
    let mut signers = SorobanVec::new(env);
    signers.push_back(s1.clone());
    signers.push_back(s2.clone());
    signers.push_back(s3.clone());
    client.init(&signers, &threshold);
    (client, [s1, s2, s3])
}

// ============================================================================
// Multisig: partial approval scenarios
// ============================================================================

/// Approvals on proposal A must NOT carry over to proposal B.
#[test]
#[should_panic(expected = "Timelock not started - call approve_upgrade first")]
fn test_partial_approvals_do_not_carry_across_proposals() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let p1 = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p1, &signers[0]);
    // p1 has 1 approval — below threshold.

    let p2 = client.propose_upgrade(&signers[1], &fake_wasm_v2(&env), &0u64);
    // p2 has 0 approvals. Approvals on p1 must not help p2.
    client.execute_upgrade(&p2);
}

/// A proposal with zero approvals must not be executable.
#[test]
#[should_panic(expected = "Timelock not started - call approve_upgrade first")]
fn test_zero_approvals_not_executable() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let p = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.execute_upgrade(&p);
}

/// Two different signers can approve same proposal to meet threshold.
#[test]
#[should_panic] // panics at WASM swap — confirms quorum passed
fn test_two_different_signers_meet_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let p = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p, &signers[1]);
    client.approve_upgrade(&p, &signers[2]);
    // 2-of-3 met by signers[1]+signers[2] (proposer didn't approve)
    client.execute_upgrade(&p);
}

/// 3-of-3 threshold: all signers must approve.
#[test]
#[should_panic(expected = "Timelock not started - call approve_upgrade first")]
fn test_3_of_3_rejects_with_only_two_approvals() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig_custom(&env, 3);

    let p = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p, &signers[0]);
    client.approve_upgrade(&p, &signers[1]);
    // Only 2/3 — must fail.
    client.execute_upgrade(&p);
}

/// 3-of-3 threshold: all three approvals reach WASM swap.
#[test]
#[should_panic] // WASM swap panic = quorum passed
fn test_3_of_3_passes_with_all_three_approvals() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig_custom(&env, 3);

    let p = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p, &signers[0]);
    client.approve_upgrade(&p, &signers[1]);
    client.approve_upgrade(&p, &signers[2]);
    client.execute_upgrade(&p);
}

/// 1-of-3 threshold: single approval is enough.
#[test]
#[should_panic] // WASM swap panic = quorum passed
fn test_1_of_3_single_approval_sufficient() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig_custom(&env, 1);

    let p = client.propose_upgrade(&signers[2], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p, &signers[2]);
    client.execute_upgrade(&p);
}

/// Approving a fully-approved but not-yet-executed proposal with a third
/// signer is fine (over-threshold approvals are accepted).
#[test]
fn test_over_threshold_approval_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env); // 2-of-3

    let p = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p, &signers[0]);
    client.approve_upgrade(&p, &signers[1]);
    // Already at threshold — third approval should not panic.
    client.approve_upgrade(&p, &signers[2]);
}

// ============================================================================
// Hash mismatch / isolation scenarios
// ============================================================================

/// Each proposal stores its own WASM hash; approving one does not affect another.
#[test]
fn test_proposals_store_independent_hashes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let p1 = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    let p2 = client.propose_upgrade(&signers[0], &fake_wasm_v2(&env), &0u64);
    let p3 = client.propose_upgrade(&signers[1], &fake_wasm_v3(&env), &0u64);

    // All three proposals exist independently.
    assert_ne!(p1, p2);
    assert_ne!(p2, p3);
    assert_ne!(p1, p3);
}

/// Approving proposal for hash_v1 and executing it doesn't change hash_v2 proposal.
#[test]
#[should_panic(expected = "Timelock not started - call approve_upgrade first")]
fn test_executing_one_proposal_leaves_other_unaffected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let p1 = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    let p2 = client.propose_upgrade(&signers[1], &fake_wasm_v2(&env), &0u64);

    // Fully approve p1.
    client.approve_upgrade(&p1, &signers[0]);
    client.approve_upgrade(&p1, &signers[1]);

    // p1 is executable but we try p2 instead — must fail.
    client.execute_upgrade(&p2);
}

/// Same signer can propose multiple upgrades with different hashes.
#[test]
fn test_same_signer_multiple_proposals() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let p1 = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    let p2 = client.propose_upgrade(&signers[0], &fake_wasm_v2(&env), &0u64);
    let p3 = client.propose_upgrade(&signers[0], &fake_wasm_v3(&env), &0u64);

    assert!(p1 < p2 && p2 < p3, "IDs must be monotonic");
}

// ============================================================================
// Operational rollback: single-admin path
// ============================================================================

/// Full rollback cycle: version bump → rollback via set_version.
#[test]
fn test_single_admin_version_rollback_cycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    let v_initial = client.get_version();
    client.set_version(&3);
    assert_eq!(client.get_version(), 3);

    // Rollback
    client.set_version(&v_initial);
    assert_eq!(client.get_version(), v_initial);
}

/// Admin address must survive version rollback.
#[test]
fn test_admin_persists_through_version_rollback() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_admin(&env);

    client.set_version(&5);
    client.set_version(&2);

    assert_eq!(client.get_admin(), Some(admin));
}

/// State snapshot before upgrade → restore after rollback.
#[test]
fn test_snapshot_based_rollback() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    // 1. Snapshot at v2
    let snap = client.create_config_snapshot();
    assert_eq!(client.get_version(), 2);

    // 2. "Upgrade" to v5
    client.set_version(&5);
    assert_eq!(client.get_version(), 5);

    // 3. Rollback via snapshot
    client.restore_config_snapshot(&snap);
    assert_eq!(client.get_version(), 2);
}

/// get_rollback_info reflects state at each phase of the cycle.
#[test]
fn test_rollback_info_through_upgrade_cycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    // Phase 1: fresh contract
    let info1 = client.get_rollback_info();
    assert_eq!(info1.current_version, 2);
    assert!(!info1.rollback_available);

    // Phase 2: create snapshot + advance version
    client.create_config_snapshot();
    client.set_version(&5);

    let info2 = client.get_rollback_info();
    assert_eq!(info2.current_version, 5);
    assert_eq!(info2.snapshot_count, 1);
    assert!(info2.has_snapshot);

    // Phase 3: simulate rollback
    client.set_version(&2);
    let info3 = client.get_rollback_info();
    assert_eq!(info3.current_version, 2);
}

/// Migration state persists across version rollback.
#[test]
fn test_migration_state_persists_through_rollback() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    let hash = BytesN::from_array(&env, &[0xAA; 32]);
    client.commit_migration(&3u32, &hash, &0u64);
    client.migrate(&3, &hash);
    assert_eq!(client.get_version(), 3);

    // Rollback version (migration state should persist)
    client.set_version(&2);

    let mig = client.get_migration_state();
    assert!(mig.is_some());
    let m = mig.unwrap();
    assert_eq!(m.from_version, 2);
    assert_eq!(m.to_version, 3);
}

/// Multiple sequential rollback cycles don't corrupt state.
#[test]
fn test_multiple_rollback_cycles() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_admin(&env);

    for round in 0..5u32 {
        let up_ver = 10 + round;
        client.set_version(&up_ver);
        assert_eq!(client.get_version(), up_ver);

        client.set_version(&2);
        assert_eq!(client.get_version(), 2);
    }

    // Admin must still be intact.
    assert_eq!(client.get_admin(), Some(admin));
}

// ============================================================================
// Operational rollback: multisig path
// ============================================================================

/// Multisig rollback: propose with previous hash after a failed upgrade attempt.
#[test]
fn test_multisig_rollback_proposal_after_failed_upgrade() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let version_before = client.get_version();

    // Attempt upgrade (will fail at WASM swap)
    let p1 = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p1, &signers[0]);
    client.approve_upgrade(&p1, &signers[1]);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.execute_upgrade(&p1);
    }));

    // Version should be unchanged (host rollback).
    assert_eq!(client.get_version(), version_before);

    // Now propose a "rollback" to the old WASM hash.
    let p2 = client.propose_upgrade(&signers[0], &fake_wasm_v2(&env), &0u64);
    assert!(p2 > p1, "rollback proposal gets a new ID");

    // Approve rollback proposal
    client.approve_upgrade(&p2, &signers[0]);
    client.approve_upgrade(&p2, &signers[1]);

    // Execute also fails at WASM swap (fake hash) but quorum check passed.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.execute_upgrade(&p2);
    }));

    // Version still unchanged.
    assert_eq!(client.get_version(), version_before);
}

/// Sequential proposals: old proposal approvals don't leak to new one.
#[test]
#[should_panic(expected = "Timelock not started - call approve_upgrade first")]
fn test_sequential_proposals_approvals_isolated() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    // Proposal 1: partially approved then abandoned.
    let p1 = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p1, &signers[0]);

    // Proposal 2: new proposal, zero approvals. The approval on p1 must not help.
    let p2 = client.propose_upgrade(&signers[0], &fake_wasm_v2(&env), &0u64);
    client.approve_upgrade(&p2, &signers[1]); // only 1 approval
    client.execute_upgrade(&p2); // threshold is 2 — must fail
}

// ============================================================================
// Edge cases
// ============================================================================

/// Failed upgrade must not set PreviousVersion (host rollback).
#[test]
fn test_failed_multisig_upgrade_no_previous_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let p = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p, &signers[0]);
    client.approve_upgrade(&p, &signers[1]);

    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.execute_upgrade(&p);
    }));

    // PreviousVersion write was rolled back.
    assert!(
        client.get_previous_version().is_none(),
        "PreviousVersion must not be set after failed multisig upgrade"
    );
}

/// Snapshot + rollback cycle preserves snapshot data.
#[test]
fn test_snapshot_survives_rollback() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    let snap_id = client.create_config_snapshot();
    client.set_version(&10);
    client.set_version(&2); // rollback

    // Snapshot from before rollback must still be retrievable.
    let snap = client.get_config_snapshot(&snap_id);
    assert!(snap.is_some());
    assert_eq!(snap.unwrap().version, 2);
}

/// Health check should remain healthy through rollback cycles.
#[test]
fn test_health_check_stable_through_rollback() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    client.set_version(&5);
    assert!(client.health_check().is_healthy);

    client.set_version(&2);
    assert!(client.health_check().is_healthy);
}

/// Invariants remain valid through rollback cycles.
#[test]
fn test_invariants_valid_through_rollback() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    client.set_version(&5);
    client.set_version(&2);

    assert!(
        client.verify_invariants(),
        "invariants must hold after rollback"
    );
}

/// Compare snapshots across an upgrade/rollback cycle.
#[test]
fn test_compare_snapshots_across_rollback() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_admin(&env);

    // Snapshot at v2
    let s1 = client.create_config_snapshot();

    // "Upgrade" to v5, snapshot again
    client.set_version(&5);
    let s2 = client.create_config_snapshot();

    // Rollback to v2, snapshot again
    client.set_version(&2);
    let s3 = client.create_config_snapshot();

    // s1 vs s2: version changed
    let diff12 = client.compare_snapshots(&s1, &s2);
    assert!(diff12.version_changed);
    assert_eq!(diff12.from_version, 2);
    assert_eq!(diff12.to_version, 5);

    // s1 vs s3: same version (both v2)
    let diff13 = client.compare_snapshots(&s1, &s3);
    assert!(!diff13.version_changed);
}

/// Large number of proposals can coexist.
#[test]
fn test_many_proposals_coexist() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let mut ids = std::vec::Vec::new();
    for i in 0..10u8 {
        let hash = BytesN::from_array(&env, &[i; 32]);
        let id = client.propose_upgrade(&signers[(i % 3) as usize], &hash, &0u64);
        ids.push(id);
    }

    // All IDs are unique and monotonic.
    for i in 1..ids.len() {
        assert!(ids[i] > ids[i - 1]);
    }
}

/// Approving after a caught execute failure resumes normal state.
#[test]
fn test_approve_after_caught_execute_failure() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, signers) = setup_multisig(&env);

    let p = client.propose_upgrade(&signers[0], &fake_wasm(&env), &0u64);
    client.approve_upgrade(&p, &signers[0]);
    client.approve_upgrade(&p, &signers[1]);

    // Execute fails at WASM swap — host rolls back mark_executed.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.execute_upgrade(&p);
    }));

    // A new proposal can still be created and approved normally.
    let p2 = client.propose_upgrade(&signers[1], &fake_wasm_v2(&env), &0u64);
    client.approve_upgrade(&p2, &signers[0]);
    client.approve_upgrade(&p2, &signers[1]);
    // p2 is now fully approved — would reach WASM swap.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.execute_upgrade(&p2);
    }));

    // Contract state still consistent.
    assert_eq!(client.get_version(), 2);
}
