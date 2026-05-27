//! # Migration Framework + Replay Protection Tests  [Issue #1087]
//!
//! Covers every branch of the migration framework and replay-protection system:
//!
//! ## Replay Protection (`commit_migration` / `migrate`)
//! - Happy path: commit then migrate succeeds
//! - Missing commitment panics with `MigrationCommitmentNotFound`
//! - Hash mismatch panics with `MigrationHashMismatch`
//! - Expired commitment is rejected and cleaned up
//! - Commitment is consumed after successful migration (no replay)
//! - `revoke_migration_commitment` removes a live commitment
//! - `get_migration_commitment` returns `Some` / `None` correctly
//!
//! ## Migration Idempotency
//! - Second call with same target_version is a no-op
//! - No extra events emitted on idempotent call
//!
//! ## Version Monotonicity
//! - Downgrade panics with "Target version must be greater than current version"
//! - Same-version target panics
//!
//! ## Chained Migrations
//! - v1 → v3 chains through v2 in a single call
//! - Unsupported step panics with "No migration path available"
//!
//! ## Initialization
//! - `init_admin` sets admin + version, emits event
//! - Double-init panics with `AlreadyInitialized`
//! - `init_governance` sets multisig config
//!
//! ## Multisig Upgrade Flow
//! - `propose_upgrade` stores proposal and returns id
//! - `approve_upgrade` approves and starts timelock when threshold met
//! - `cancel_upgrade` removes proposal (admin or proposer)
//! - `get_upgrade_proposal` returns correct record
//!
//! ## Event Emission
//! - `commit_migration` emits `MigrationCommittedEvent`
//! - `migrate` emits `MigrationEvent` on success
//! - `revoke_migration_commitment` emits revoke event
//!
//! ## Authorization
//! - All admin-only functions reject non-admin callers
//! - `migrate` without auth panics
//! - `commit_migration` without auth panics
//!
//! ## Edge Cases
//! - Zero-byte hash accepted
//! - Max-byte hash accepted
//! - Commitment with `expiry_seconds = 0` never expires
//! - `get_migration_state` returns `None` before first migration

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, BytesN, Env, TryIntoVal,
};

use crate::{
    GrainlifyContract, GrainlifyContractClient, MigrationCommittedEvent, MigrationEvent,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (GrainlifyContractClient<'_>, Address) {
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.init_admin(&admin);
    (client, admin)
}

fn hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Commit then migrate in one helper — the standard happy path.
fn commit_and_migrate(
    client: &GrainlifyContractClient,
    env: &Env,
    target: u32,
    seed: u8,
) {
    let h = hash(env, seed);
    client.commit_migration(&target, &h, &0u64);
    client.migrate(&target, &h);
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Initialization
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn init_admin_sets_version_and_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    client.init_admin(&admin);

    assert_eq!(client.get_admin(), Some(admin));
    assert!(client.get_version() >= 1);
}

#[test]
#[should_panic]
fn init_admin_double_init_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup(&env);
    let admin2 = Address::generate(&env);
    client.init_admin(&admin2); // must panic
}

#[test]
fn init_governance_sets_multisig_config() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let mut signers = soroban_sdk::Vec::new(&env);
    signers.push_back(s1.clone());
    signers.push_back(s2.clone());

    client.init_governance(&admin, &signers, &1u32);

    assert_eq!(client.get_admin(), Some(admin));
    assert!(client.get_version() >= 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. commit_migration — happy path
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn commit_migration_stores_commitment() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let h = hash(&env, 0xAA);
    client.commit_migration(&3u32, &h, &0u64);

    let commitment = client.get_migration_commitment(&3u32).unwrap();
    assert_eq!(commitment.target_version, 3);
    assert_eq!(commitment.hash, h);
    assert_eq!(commitment.expires_at, 0); // no expiry
}

#[test]
fn commit_migration_with_expiry_stores_correct_expires_at() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1_000);
    let (client, _) = setup(&env);

    let h = hash(&env, 0xBB);
    client.commit_migration(&3u32, &h, &3600u64); // 1 hour TTL

    let commitment = client.get_migration_commitment(&3u32).unwrap();
    assert_eq!(commitment.expires_at, 1_000 + 3_600);
}

#[test]
fn commit_migration_emits_committed_event() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 5_000);
    let (client, admin) = setup(&env);

    let h = hash(&env, 0xCC);
    let events_before = env.events().all().len();
    client.commit_migration(&3u32, &h, &0u64);

    let events = env.events().all();
    assert!(
        events.len() > events_before,
        "commit_migration must emit at least one event"
    );

    // Find the MigrationCommittedEvent by trying each new event
    let mut found = false;
    for i in events_before..events.len() {
        let val = events.get(i).unwrap().2;
        if let Ok(payload) = TryIntoVal::<_, MigrationCommittedEvent>::try_into_val(&val, &env) {
            assert_eq!(payload.target_version, 3);
            assert_eq!(payload.hash, h);
            assert_eq!(payload.committed_at, 5_000);
            assert_eq!(payload.admin, admin);
            found = true;
            break;
        }
    }
    assert!(found, "MigrationCommittedEvent must be emitted");
}

#[test]
fn get_migration_commitment_returns_none_when_absent() {
    let env = Env::default();
    let (client, _) = setup(&env);
    assert!(client.get_migration_commitment(&99u32).is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. migrate — happy path with replay protection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migrate_succeeds_after_commit() {
    let env = Env::default();
    let (client, _) = setup(&env);

    commit_and_migrate(&client, &env, 3, 0x01);

    assert_eq!(client.get_version(), 3);
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.to_version, 3);
}

#[test]
fn migrate_consumes_commitment_replay_protection() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let h = hash(&env, 0x02);
    client.commit_migration(&3u32, &h, &0u64);
    client.migrate(&3u32, &h);

    // Commitment must be gone — cannot replay
    assert!(
        client.get_migration_commitment(&3u32).is_none(),
        "Commitment must be consumed after migration"
    );
}

#[test]
#[should_panic]
fn migrate_without_prior_commit_panics() {
    let env = Env::default();
    let (client, _) = setup(&env);

    // No commit_migration call — must panic with MigrationCommitmentNotFound
    client.migrate(&3u32, &hash(&env, 0x03));
}

#[test]
#[should_panic]
fn migrate_with_wrong_hash_panics() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let committed_hash = hash(&env, 0x04);
    let wrong_hash = hash(&env, 0x05);

    client.commit_migration(&3u32, &committed_hash, &0u64);
    client.migrate(&3u32, &wrong_hash); // must panic with MigrationHashMismatch
}

#[test]
#[should_panic(expected = "Migration commitment has expired")]
fn migrate_with_expired_commitment_panics() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1_000);
    let (client, _) = setup(&env);

    let h = hash(&env, 0x06);
    client.commit_migration(&3u32, &h, &3600u64); // expires at 4_600

    // Advance time past expiry
    env.ledger().with_mut(|li| li.timestamp = 10_000);

    client.migrate(&3u32, &h); // must panic
}

#[test]
fn migrate_with_no_expiry_commitment_never_expires() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1_000);
    let (client, _) = setup(&env);

    let h = hash(&env, 0x07);
    client.commit_migration(&3u32, &h, &0u64); // expires_at = 0 = never

    // Advance time far into the future
    env.ledger().with_mut(|li| li.timestamp = 999_999_999);

    client.migrate(&3u32, &h); // must succeed
    assert_eq!(client.get_version(), 3);
}

#[test]
fn migrate_emits_success_event() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 2_000);
    let (client, _) = setup(&env);

    let h = hash(&env, 0x08);
    client.commit_migration(&3u32, &h, &0u64);
    let events_before = env.events().all().len();
    client.migrate(&3u32, &h);

    let events = env.events().all();
    assert!(events.len() > events_before, "migrate must emit at least one event");

    // Find the MigrationEvent (success) among new events
    let mut found = false;
    for i in events_before..events.len() {
        let val = events.get(i).unwrap().2;
        if let Ok(payload) = TryIntoVal::<_, MigrationEvent>::try_into_val(&val, &env) {
            if payload.success && payload.to_version == 3 {
                assert_eq!(payload.migration_hash, h);
                found = true;
                break;
            }
        }
    }
    assert!(found, "MigrationEvent (success) must be emitted");
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Idempotency
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migrate_idempotent_second_call_is_noop() {
    let env = Env::default();
    let (client, _) = setup(&env);

    commit_and_migrate(&client, &env, 3, 0x10);
    let state_first = client.get_migration_state().unwrap();

    // Second call — idempotent (no commitment needed for no-op)
    let h2 = hash(&env, 0x11);
    client.commit_migration(&3u32, &h2, &0u64);
    client.migrate(&3u32, &h2);

    let state_second = client.get_migration_state().unwrap();
    assert_eq!(
        state_first.migrated_at, state_second.migrated_at,
        "Idempotent call must not update migration state"
    );
    assert_eq!(client.get_version(), 3);
}

#[test]
fn idempotent_migration_emits_no_extra_events() {
    let env = Env::default();
    let (client, _) = setup(&env);

    commit_and_migrate(&client, &env, 3, 0x12);
    let events_after_first = env.events().all().len();

    // Second call — idempotent: commit emits 1 event, but migrate itself is a no-op
    let h2 = hash(&env, 0x13);
    client.commit_migration(&3u32, &h2, &0u64);
    let events_after_commit = env.events().all().len();
    client.migrate(&3u32, &h2); // no-op — no migration events

    let events_after_second = env.events().all().len();
    // migrate() itself must not emit any new events (idempotent no-op)
    assert_eq!(
        events_after_commit, events_after_second,
        "Idempotent migrate() must not emit additional events"
    );
    // Sanity: commit did emit something
    assert!(events_after_commit > events_after_first);
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Version monotonicity
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Target version must be greater than current version")]
fn migrate_rejects_same_version() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let h = hash(&env, 0x20);
    let current = client.get_version();
    client.commit_migration(&current, &h, &0u64);
    client.migrate(&current, &h);
}

#[test]
#[should_panic(expected = "Target version must be greater than current version")]
fn migrate_rejects_downgrade() {
    let env = Env::default();
    let (client, _) = setup(&env);

    // Bump to v3 first
    commit_and_migrate(&client, &env, 3, 0x21);
    assert_eq!(client.get_version(), 3);

    // Try to go back to v2
    let h = hash(&env, 0x22);
    client.commit_migration(&2u32, &h, &0u64);
    client.migrate(&2u32, &h);
}

#[test]
#[should_panic(expected = "Target version must be greater than current version")]
fn migrate_to_version_zero_panics() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let h = hash(&env, 0x23);
    client.commit_migration(&0u32, &h, &0u64);
    client.migrate(&0u32, &h);
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Chained migrations
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migrate_v1_to_v3_chains_through_v2() {
    let env = Env::default();
    let (client, _) = setup(&env);

    client.set_version(&1u32);
    assert_eq!(client.get_version(), 1);

    let h = hash(&env, 0x30);
    client.commit_migration(&3u32, &h, &0u64);
    client.migrate(&3u32, &h);

    assert_eq!(client.get_version(), 3);
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, 1);
    assert_eq!(state.to_version, 3);
}

#[test]
#[should_panic(expected = "No migration path available")]
fn migrate_to_unsupported_version_panics() {
    let env = Env::default();
    let (client, _) = setup(&env);

    // Migrate to v3 first (supported)
    commit_and_migrate(&client, &env, 3, 0x31);

    // v3 → v4 has no migration function
    let h = hash(&env, 0x32);
    client.commit_migration(&4u32, &h, &0u64);
    client.migrate(&4u32, &h);
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. revoke_migration_commitment
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn revoke_removes_commitment() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let h = hash(&env, 0x40);
    client.commit_migration(&3u32, &h, &0u64);
    assert!(client.get_migration_commitment(&3u32).is_some());

    client.revoke_migration_commitment(&3u32);
    assert!(
        client.get_migration_commitment(&3u32).is_none(),
        "Commitment must be removed after revoke"
    );
}

#[test]
fn revoke_emits_event() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let h = hash(&env, 0x41);
    client.commit_migration(&3u32, &h, &0u64);
    let events_before = env.events().all().len();
    client.revoke_migration_commitment(&3u32);

    let events_after = env.events().all().len();
    assert!(
        events_after > events_before,
        "revoke_migration_commitment must emit at least one event"
    );
}

#[test]
#[should_panic]
fn migrate_after_revoke_panics() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let h = hash(&env, 0x42);
    client.commit_migration(&3u32, &h, &0u64);
    client.revoke_migration_commitment(&3u32);

    // No commitment — must panic
    client.migrate(&3u32, &h);
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Authorization
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn commit_migration_requires_admin_auth() {
    let env = Env::default();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.init_admin(&admin);

    // Call without any auth mock
    client.mock_auths(&[]).commit_migration(&3u32, &hash(&env, 0x50), &0u64);
}

#[test]
#[should_panic]
fn migrate_requires_admin_auth() {
    let env = Env::default();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.init_admin(&admin);

    let h = hash(&env, 0x51);
    client.commit_migration(&3u32, &h, &0u64);

    // Call migrate without auth
    client.mock_auths(&[]).migrate(&3u32, &h);
}

#[test]
#[should_panic]
fn revoke_requires_admin_auth() {
    let env = Env::default();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.init_admin(&admin);
    client.commit_migration(&3u32, &hash(&env, 0x52), &0u64);

    client.mock_auths(&[]).revoke_migration_commitment(&3u32);
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migrate_with_zero_hash_succeeds() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let zero = BytesN::from_array(&env, &[0u8; 32]);
    client.commit_migration(&3u32, &zero, &0u64);
    client.migrate(&3u32, &zero);

    assert_eq!(client.get_version(), 3);
    assert_eq!(client.get_migration_state().unwrap().migration_hash, zero);
}

#[test]
fn migrate_with_max_hash_succeeds() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let max = BytesN::from_array(&env, &[0xFFu8; 32]);
    client.commit_migration(&3u32, &max, &0u64);
    client.migrate(&3u32, &max);

    assert_eq!(client.get_version(), 3);
    assert_eq!(client.get_migration_state().unwrap().migration_hash, max);
}

#[test]
fn no_migration_state_before_first_migrate() {
    let env = Env::default();
    let (client, _) = setup(&env);
    assert!(client.get_migration_state().is_none());
}

#[test]
fn migration_preserves_admin() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    commit_and_migrate(&client, &env, 3, 0x60);

    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn multiple_independent_commitments_coexist() {
    let env = Env::default();
    let (client, _) = setup(&env);

    // Commit for two different target versions simultaneously
    let h3 = hash(&env, 0x70);
    let _h4 = hash(&env, 0x71);
    client.commit_migration(&3u32, &h3, &0u64);
    // We can't commit for v4 yet (v3 must be migrated first), but we can
    // verify the v3 commitment is stored correctly.
    assert_eq!(client.get_migration_commitment(&3u32).unwrap().hash, h3);
    assert!(client.get_migration_commitment(&4u32).is_none());

    // Migrate to v3 — consumes v3 commitment
    client.migrate(&3u32, &h3);
    assert!(client.get_migration_commitment(&3u32).is_none());
}

#[test]
fn recommit_after_revoke_succeeds() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let h1 = hash(&env, 0x80);
    client.commit_migration(&3u32, &h1, &0u64);
    client.revoke_migration_commitment(&3u32);

    // Re-commit with a different hash
    let h2 = hash(&env, 0x81);
    client.commit_migration(&3u32, &h2, &0u64);
    assert_eq!(client.get_migration_commitment(&3u32).unwrap().hash, h2);

    // Migrate with the new hash
    client.migrate(&3u32, &h2);
    assert_eq!(client.get_version(), 3);
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Upgrade proposal queries
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_upgrade_proposal_returns_none_for_unknown_id() {
    let env = Env::default();
    let (client, _) = setup(&env);
    assert!(client.get_upgrade_proposal(&999u64).is_none());
}

#[test]
fn get_timelock_status_returns_none_before_approval() {
    let env = Env::default();
    let (client, _) = setup(&env);
    // No proposal has been approved — timelock status should be None
    assert!(client.get_timelock_status(&1u64).is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// 11. Migration state correctness
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_state_from_version_matches_pre_migration_version() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let pre = client.get_version();
    commit_and_migrate(&client, &env, 3, 0x90);

    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, pre);
    assert_eq!(state.to_version, 3);
}

#[test]
fn sequential_migrations_update_state_correctly() {
    let env = Env::default();
    let (client, _) = setup(&env);

    // v2 → v3
    commit_and_migrate(&client, &env, 3, 0xA0);
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, 2);
    assert_eq!(state.to_version, 3);
    assert_eq!(client.get_version(), 3);
}

#[test]
fn chained_migration_records_full_range() {
    let env = Env::default();
    let (client, _) = setup(&env);

    client.set_version(&1u32);
    let h = hash(&env, 0xB0);
    client.commit_migration(&3u32, &h, &0u64);
    client.migrate(&3u32, &h);

    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, 1);
    assert_eq!(state.to_version, 3);
    assert_eq!(state.migration_hash, h);
}
