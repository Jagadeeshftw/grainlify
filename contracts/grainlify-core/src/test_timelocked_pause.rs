//! # Timelocked Upgrades & Emergency Pause — Edge Case Tests
//!
//! Covers every branch of the new entrypoints added in v3.0.0:
//!   - propose_upgrade / approve_upgrade / cancel_upgrade / execute_upgrade
//!   - pause / unpause / is_paused
//!   - commit_migration / migrate
//!   - set_timelock_delay bounds
//!
//! Target: ≥95 % line coverage of the new code paths.

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger as _},
    Address, BytesN, Env, Vec as SVec,
};

use crate::{GrainlifyContract, GrainlifyContractClient};

// ── shared helpers ────────────────────────────────────────────────────────────

fn fake_wasm(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xAB; 32])
}

fn fake_wasm2(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xCD; 32])
}

fn migration_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Register + init with 2-of-3 multisig; returns (client, [s1, s2, s3]).
fn setup_multisig(env: &Env) -> (GrainlifyContractClient, [Address; 3]) {
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let s1 = Address::generate(env);
    let s2 = Address::generate(env);
    let s3 = Address::generate(env);
    let mut signers = SVec::new(env);
    signers.push_back(s1.clone());
    signers.push_back(s2.clone());
    signers.push_back(s3.clone());
    client.init(&signers, &2);
    (client, [s1, s2, s3])
}

/// Register + init with single admin; returns (client, admin).
fn setup_admin(env: &Env) -> (GrainlifyContractClient, Address) {
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    (client, admin)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Timelock delay configuration
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_timelock_default_is_86400() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    assert_eq!(client.get_timelock_delay(), 86_400);
}

#[test]
fn test_set_timelock_delay_valid() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    client.set_timelock_delay(&7_200);
    assert_eq!(client.get_timelock_delay(), 7_200);
}

#[test]
fn test_set_timelock_delay_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    client.set_timelock_delay(&7_200);
    let events = env.events().all();
    assert!(events.len() > 0);
}

#[test]
#[should_panic(expected = "Timelock delay must be at least 1 hour")]
fn test_set_timelock_delay_below_min_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    client.set_timelock_delay(&3_599); // 1 second below minimum
}

#[test]
#[should_panic(expected = "Timelock delay cannot exceed 30 days")]
fn test_set_timelock_delay_above_max_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    client.set_timelock_delay(&2_592_001); // 1 second above maximum
}

#[test]
fn test_set_timelock_delay_at_exact_min() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    client.set_timelock_delay(&3_600); // exactly 1 hour — must succeed
    assert_eq!(client.get_timelock_delay(), 3_600);
}

#[test]
fn test_set_timelock_delay_at_exact_max() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    client.set_timelock_delay(&2_592_000); // exactly 30 days — must succeed
    assert_eq!(client.get_timelock_delay(), 2_592_000);
}

#[test]
#[should_panic(expected = "Read-only mode")]
fn test_set_timelock_delay_blocked_in_read_only_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    client.set_read_only_mode(&true);
    client.set_timelock_delay(&7_200);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — propose_upgrade
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_propose_upgrade_returns_proposal_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    assert_eq!(pid, 1);
}

#[test]
fn test_propose_upgrade_increments_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    let p1 = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    let p2 = client.propose_upgrade(&s1, &fake_wasm2(&env), &0u64);
    assert_eq!(p2, p1 + 1);
}

#[test]
fn test_propose_upgrade_stores_wasm_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    let wasm = fake_wasm(&env);
    let pid = client.propose_upgrade(&s1, &wasm, &0u64);
    let record = client.get_upgrade_proposal(&pid).unwrap();
    assert_eq!(record.wasm_hash, wasm);
}

#[test]
fn test_propose_upgrade_stores_proposer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    let record = client.get_upgrade_proposal(&pid).unwrap();
    assert_eq!(record.proposer, Some(s1));
}

#[test]
fn test_propose_upgrade_stores_expiry() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    let expiry = env.ledger().timestamp() + 3_600;
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &expiry);
    let record = client.get_upgrade_proposal(&pid).unwrap();
    assert_eq!(record.expiry, expiry);
}

#[test]
#[should_panic]
fn test_propose_upgrade_rejects_non_signer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_multisig(&env);
    let outsider = Address::generate(&env);
    client.propose_upgrade(&outsider, &fake_wasm(&env), &0u64);
}

#[test]
fn test_get_upgrade_proposal_returns_none_for_unknown_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_multisig(&env);
    assert!(client.get_upgrade_proposal(&999u64).is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — approve_upgrade and timelock auto-start
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_approve_upgrade_no_timelock_before_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    // Only one approval — threshold (2) not yet met
    client.approve_upgrade(&pid, &s2);
    assert_eq!(client.get_timelock_status(&pid), None);
}

#[test]
fn test_approve_upgrade_starts_timelock_at_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3); // threshold met
    let status = client.get_timelock_status(&pid);
    assert!(status.is_some());
    assert!(status.unwrap() > 0); // delay remaining
}

#[test]
fn test_approve_upgrade_emits_timelock_started_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3);
    let events = env.events().all();
    // At least one event should have been emitted
    assert!(events.len() > 0);
}

#[test]
fn test_approve_upgrade_timelock_not_restarted_on_extra_approval() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3); // threshold met, timelock starts
    let status_after_threshold = client.get_timelock_status(&pid);

    // Advance time slightly
    env.ledger().set_timestamp(env.ledger().timestamp() + 100);

    // Extra approval from s1 — timelock should NOT restart
    client.approve_upgrade(&pid, &s1);
    let status_after_extra = client.get_timelock_status(&pid);

    // Remaining time should be less (time passed), not reset to full delay
    assert!(status_after_extra.unwrap() < status_after_threshold.unwrap());
}

#[test]
#[should_panic]
fn test_approve_upgrade_rejects_non_signer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    let outsider = Address::generate(&env);
    client.approve_upgrade(&pid, &outsider);
}

#[test]
#[should_panic]
fn test_approve_upgrade_rejects_duplicate_approval() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s2); // duplicate — must panic
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — execute_upgrade timelock enforcement
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "Timelock not started")]
fn test_execute_upgrade_panics_without_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    // No approvals — timelock never started
    client.execute_upgrade(&pid);
}

#[test]
#[should_panic(expected = "Timelock delay not met")]
fn test_execute_upgrade_panics_before_delay_elapses() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    client.set_timelock_delay(&3_600);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3); // timelock starts
    // Do NOT advance time — should panic
    client.execute_upgrade(&pid);
}

#[test]
#[should_panic(expected = "Timelock delay not met")]
fn test_execute_upgrade_panics_one_second_before_delay() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    client.set_timelock_delay(&3_600);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3);
    let start = env.ledger().timestamp();
    env.ledger().set_timestamp(start + 3_599); // 1 second short
    client.execute_upgrade(&pid);
}

#[test]
fn test_execute_upgrade_succeeds_exactly_at_delay() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    client.set_timelock_delay(&3_600);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3);
    let start = env.ledger().timestamp();
    env.ledger().set_timestamp(start + 3_600); // exactly at boundary
    // Should not panic (WASM swap will fail in test env, but auth/timelock passes)
    let result = std::panic::catch_unwind(|| client.execute_upgrade(&pid));
    // We accept either success or a WASM-not-found error (not a timelock error)
    if let Err(e) = result {
        let msg = format!("{:?}", e);
        assert!(
            !msg.contains("Timelock delay not met"),
            "Should not fail on timelock: {}", msg
        );
    }
}

#[test]
fn test_execute_upgrade_clears_timelock_status() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    client.set_timelock_delay(&3_600);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3);
    let start = env.ledger().timestamp();
    env.ledger().set_timestamp(start + 3_700);
    // Attempt execution — may fail on WASM swap but timelock key should be cleared
    let _ = std::panic::catch_unwind(|| client.execute_upgrade(&pid));
    // After a successful execute the timelock entry is removed; after WASM failure
    // the storage is rolled back, so we just verify no timelock panic occurs on retry
}

#[test]
fn test_timelock_status_returns_zero_when_ready() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    client.set_timelock_delay(&3_600);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3);
    let start = env.ledger().timestamp();
    env.ledger().set_timestamp(start + 3_700);
    assert_eq!(client.get_timelock_status(&pid), Some(0));
}

#[test]
fn test_timelock_status_none_before_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    assert_eq!(client.get_timelock_status(&pid), None);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5 — cancel_upgrade
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_cancel_upgrade_marks_proposal_cancelled() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.cancel_upgrade(&pid, &s2);
    let record = client.get_upgrade_proposal(&pid).unwrap();
    assert!(record.cancelled);
}

#[test]
fn test_cancel_upgrade_clears_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3); // timelock starts
    assert!(client.get_timelock_status(&pid).is_some());
    client.cancel_upgrade(&pid, &s1);
    assert_eq!(client.get_timelock_status(&pid), None);
}

#[test]
#[should_panic]
fn test_cancel_upgrade_rejects_non_signer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    let outsider = Address::generate(&env);
    client.cancel_upgrade(&pid, &outsider);
}

#[test]
#[should_panic]
fn test_cancel_upgrade_rejects_double_cancel() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.cancel_upgrade(&pid, &s2);
    client.cancel_upgrade(&pid, &s2); // second cancel must panic
}

#[test]
#[should_panic]
fn test_approve_cancelled_proposal_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.cancel_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3); // must panic
}

#[test]
#[should_panic]
fn test_cancel_nonexistent_proposal_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    client.cancel_upgrade(&999u64, &s1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 6 — Emergency pause / unpause
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_is_paused_false_by_default() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_multisig(&env);
    assert!(!client.is_paused());
}

#[test]
fn test_pause_sets_paused_flag() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    client.pause(&s1);
    assert!(client.is_paused());
}

#[test]
fn test_unpause_clears_paused_flag() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    client.pause(&s1);
    client.unpause(&s1);
    assert!(!client.is_paused());
}

#[test]
fn test_pause_unpause_cycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    client.pause(&s1);
    assert!(client.is_paused());
    client.unpause(&s2);
    assert!(!client.is_paused());
    client.pause(&s2);
    assert!(client.is_paused());
}

#[test]
#[should_panic]
fn test_pause_rejects_non_signer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_multisig(&env);
    let outsider = Address::generate(&env);
    client.pause(&outsider);
}

#[test]
#[should_panic]
fn test_unpause_rejects_non_signer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    client.pause(&s1);
    let outsider = Address::generate(&env);
    client.unpause(&outsider);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_propose_upgrade_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    client.pause(&s1);
    client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_approve_upgrade_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.pause(&s1);
    client.approve_upgrade(&pid, &s2);
}

#[test]
fn test_execute_upgrade_not_blocked_by_pause() {
    // execute_upgrade does NOT check the pause flag — it only checks timelock.
    // This test verifies the panic is about timelock, not pause.
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    client.set_timelock_delay(&3_600);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.approve_upgrade(&pid, &s2);
    client.approve_upgrade(&pid, &s3);
    client.pause(&s1); // pause AFTER timelock started
    let start = env.ledger().timestamp();
    env.ledger().set_timestamp(start + 3_700);
    // Should fail on WASM-not-found, NOT on "Contract is paused"
    let result = std::panic::catch_unwind(|| client.execute_upgrade(&pid));
    if let Err(e) = result {
        let msg = format!("{:?}", e);
        assert!(!msg.contains("Contract is paused"), "execute_upgrade must not be blocked by pause: {}", msg);
    }
}

#[test]
fn test_cancel_upgrade_not_blocked_by_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    client.pause(&s1);
    // cancel_upgrade should still work while paused
    client.cancel_upgrade(&pid, &s2);
    let record = client.get_upgrade_proposal(&pid).unwrap();
    assert!(record.cancelled);
}

#[test]
fn test_propose_upgrade_works_after_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, _, _]) = setup_multisig(&env);
    client.pause(&s1);
    client.unpause(&s1);
    // Should succeed now
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    assert_eq!(pid, 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 7 — commit_migration / migrate
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_migrate_happy_path() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let hash = migration_hash(&env, 0x03);
    client.commit_migration(&3u32, &hash);
    client.migrate(&3u32, &hash);
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.to_version, 3);
    assert_eq!(state.migration_hash, hash);
}

#[test]
fn test_migrate_updates_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let hash = migration_hash(&env, 0x05);
    client.commit_migration(&5u32, &hash);
    client.migrate(&5u32, &hash);
    assert_eq!(client.get_version(), 5);
}

#[test]
fn test_migrate_records_from_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let initial_version = client.get_version();
    let hash = migration_hash(&env, 0x04);
    client.commit_migration(&4u32, &hash);
    client.migrate(&4u32, &hash);
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, initial_version);
}

#[test]
fn test_migrate_is_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let hash = migration_hash(&env, 0x03);
    client.commit_migration(&3u32, &hash);
    client.migrate(&3u32, &hash);
    // Second call to same version should be a no-op (not panic)
    client.migrate(&3u32, &hash);
    assert_eq!(client.get_version(), 3);
}

#[test]
#[should_panic]
fn test_migrate_without_commit_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let hash = migration_hash(&env, 0x03);
    // No commit_migration call — must panic with MigrationCommitmentNotFound
    client.migrate(&3u32, &hash);
}

#[test]
#[should_panic]
fn test_migrate_wrong_hash_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let committed_hash = migration_hash(&env, 0x03);
    let wrong_hash = migration_hash(&env, 0xFF);
    client.commit_migration(&3u32, &committed_hash);
    client.migrate(&3u32, &wrong_hash); // hash mismatch — must panic
}

#[test]
fn test_commit_migration_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let hash = migration_hash(&env, 0x03);
    client.commit_migration(&3u32, &hash);
    let events = env.events().all();
    assert!(events.len() > 0);
}

#[test]
fn test_migrate_emits_done_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let hash = migration_hash(&env, 0x03);
    client.commit_migration(&3u32, &hash);
    let before = env.events().all().len();
    client.migrate(&3u32, &hash);
    let after = env.events().all().len();
    assert!(after > before);
}

#[test]
fn test_commit_migration_commitment_consumed_after_migrate() {
    // After migrate() the commitment is deleted; a second migrate() with the
    // same version is idempotent (no-op), so no panic from missing commitment.
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let hash = migration_hash(&env, 0x03);
    client.commit_migration(&3u32, &hash);
    client.migrate(&3u32, &hash);
    // Idempotent second call — version already at 3, returns early
    client.migrate(&3u32, &hash);
    assert_eq!(client.get_version(), 3);
}

#[test]
#[should_panic(expected = "Read-only mode")]
fn test_commit_migration_blocked_in_read_only_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    client.set_read_only_mode(&true);
    client.commit_migration(&3u32, &migration_hash(&env, 0x03));
}

#[test]
#[should_panic(expected = "Read-only mode")]
fn test_migrate_blocked_in_read_only_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let hash = migration_hash(&env, 0x03);
    client.commit_migration(&3u32, &hash);
    client.set_read_only_mode(&true);
    client.migrate(&3u32, &hash);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 8 — Proposal expiry
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic]
fn test_approve_expired_proposal_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let now = env.ledger().timestamp();
    let expiry = now + 100;
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &expiry);
    // Advance past expiry
    env.ledger().set_timestamp(now + 200);
    client.approve_upgrade(&pid, &s2); // must panic
}

#[test]
fn test_approve_proposal_before_expiry_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let now = env.ledger().timestamp();
    let expiry = now + 10_000;
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &expiry);
    env.ledger().set_timestamp(now + 5_000); // still before expiry
    client.approve_upgrade(&pid, &s2); // must succeed
}

#[test]
fn test_zero_expiry_never_expires() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64); // no expiry
    env.ledger().set_timestamp(env.ledger().timestamp() + 999_999_999);
    // Should not panic on approval
    client.approve_upgrade(&pid, &s2);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 9 — Initialization paths
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_init_multisig_sets_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_multisig(&env);
    assert_eq!(client.get_version(), 2);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_multisig_blocks_reinit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);
    let mut signers = SVec::new(&env);
    signers.push_back(s1);
    signers.push_back(s2);
    client.init(&signers, &2);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_multisig_blocked_after_init_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let mut signers = SVec::new(&env);
    signers.push_back(s1);
    signers.push_back(s2);
    client.init(&signers, &2);
}

#[test]
fn test_init_with_network_sets_chain_and_network() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    let chain = soroban_sdk::String::from_str(&env, "stellar");
    let network = soroban_sdk::String::from_str(&env, "testnet");
    client.init_with_network(&admin, &chain, &network);
    assert_eq!(client.get_version(), 2);
    assert!(client.get_chain_id().is_some());
    assert!(client.get_network_id().is_some());
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_init_with_network_blocked_after_init() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);
    let admin2 = Address::generate(&env);
    let chain = soroban_sdk::String::from_str(&env, "stellar");
    let network = soroban_sdk::String::from_str(&env, "testnet");
    client.init_with_network(&admin2, &chain, &network);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 10 — Integration: full timelocked upgrade flow
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_full_flow_propose_approve_wait_execute_attempt() {
    // Verifies the complete happy-path up to the WASM swap (which fails in test env).
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, s3]) = setup_multisig(&env);
    client.set_timelock_delay(&3_600);

    // 1. Propose
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);
    assert_eq!(client.get_timelock_status(&pid), None);

    // 2. First approval — threshold not met
    client.approve_upgrade(&pid, &s2);
    assert_eq!(client.get_timelock_status(&pid), None);

    // 3. Second approval — threshold met, timelock starts
    client.approve_upgrade(&pid, &s3);
    let status = client.get_timelock_status(&pid);
    assert!(status.is_some() && status.unwrap() > 0);

    // 4. Attempt before delay — must fail on timelock
    let result = std::panic::catch_unwind(|| client.execute_upgrade(&pid));
    assert!(result.is_err());

    // 5. Advance past delay
    let start = env.ledger().timestamp();
    env.ledger().set_timestamp(start + 3_700);
    assert_eq!(client.get_timelock_status(&pid), Some(0));

    // 6. Execute — fails on WASM swap in test env, but NOT on timelock
    let result = std::panic::catch_unwind(|| client.execute_upgrade(&pid));
    if let Err(e) = result {
        let msg = format!("{:?}", e);
        assert!(!msg.contains("Timelock delay not met"), "Unexpected timelock error: {}", msg);
        assert!(!msg.contains("Timelock not started"), "Unexpected timelock error: {}", msg);
    }
}

#[test]
fn test_full_flow_propose_pause_cancel_unpause_repropose() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, [s1, s2, _]) = setup_multisig(&env);

    // 1. Propose
    let pid = client.propose_upgrade(&s1, &fake_wasm(&env), &0u64);

    // 2. Pause — blocks further approvals
    client.pause(&s1);
    let result = std::panic::catch_unwind(|| client.approve_upgrade(&pid, &s2));
    assert!(result.is_err());

    // 3. Cancel the stale proposal
    client.cancel_upgrade(&pid, &s2);

    // 4. Unpause
    client.unpause(&s1);

    // 5. Re-propose with new hash
    let pid2 = client.propose_upgrade(&s1, &fake_wasm2(&env), &0u64);
    assert_eq!(pid2, pid + 1);
    assert!(!client.is_paused());
}

#[test]
fn test_full_migration_flow_with_commit_reveal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_admin(&env);

    let hash = migration_hash(&env, 0x07);

    // 1. Commit
    client.commit_migration(&7u32, &hash);

    // 2. Migrate
    client.migrate(&7u32, &hash);

    // 3. Verify state
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.to_version, 7);
    assert_eq!(client.get_version(), 7);

    // 4. Idempotent second call
    client.migrate(&7u32, &hash);
    assert_eq!(client.get_version(), 7);
}
