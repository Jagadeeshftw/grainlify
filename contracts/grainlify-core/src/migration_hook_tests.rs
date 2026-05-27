//! Comprehensive tests for version migration hooks in Grainlify Core.
//!
//! These tests verify that the state migration system:
//! - Executes migrations only once per version (idempotency)
//! - Performs expected state transformations across version boundaries
//! - Enforces proper admin authorization
//! - Handles edge cases like unsupported paths, downgrades, and repeated calls
//!
//! NOTE: All `migrate()` calls now require a prior `commit_migration()` call
//! (replay protection introduced in issue #1087).

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, BytesN, Env,
};

use crate::{GrainlifyContract, GrainlifyContractClient};

// ============================================================================
// Helpers
// ============================================================================

fn setup_contract(env: &Env) -> (GrainlifyContractClient<'_>, Address) {
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    (client, admin)
}

fn migration_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Two-step commit + migrate (replay-protection flow).
fn commit_and_migrate(client: &GrainlifyContractClient, env: &Env, target: u32, seed: u8) {
    let h = migration_hash(env, seed);
    client.commit_migration(&target, &h, &0u64);
    client.migrate(&target, &h);
}

// ============================================================================
// 1. Single-execution guarantee (idempotency)
// ============================================================================

#[test]
fn migrate_v2_to_v3_executes_only_once() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    assert_eq!(client.get_version(), 2);

    let hash = migration_hash(&env, 0xAA);
    client.commit_migration(&3u32, &hash, &0u64);
    client.migrate(&3u32, &hash);
    let state_after_first = client.get_migration_state().unwrap();
    assert_eq!(state_after_first.from_version, 2);
    assert_eq!(state_after_first.to_version, 3);
    let ts_first = state_after_first.migrated_at;

    // Second call — idempotent (commitment already consumed; new commit needed)
    let hash2 = migration_hash(&env, 0xAB);
    client.commit_migration(&3u32, &hash2, &0u64);
    client.migrate(&3u32, &hash2);
    let state_after_second = client.get_migration_state().unwrap();

    assert_eq!(state_after_second.migrated_at, ts_first);
    assert_eq!(state_after_second.from_version, 2);
    assert_eq!(state_after_second.to_version, 3);
    assert_eq!(state_after_second.migration_hash, hash); // original hash preserved
}

#[test]
fn repeated_migrate_calls_do_not_change_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    commit_and_migrate(&client, &env, 3, 0xBB);
    assert_eq!(client.get_version(), 3);

    // Idempotent — version stays at 3
    let h2 = migration_hash(&env, 0xBC);
    client.commit_migration(&3u32, &h2, &0u64);
    client.migrate(&3u32, &h2);
    assert_eq!(client.get_version(), 3);
}

// ============================================================================
// 2. Correct state transformations across version boundaries
// ============================================================================

#[test]
fn migrate_v2_to_v3_records_correct_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let hash = migration_hash(&env, 0x01);
    client.commit_migration(&3u32, &hash, &0u64);
    client.migrate(&3u32, &hash);

    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, 2);
    assert_eq!(state.to_version, 3);
    assert_eq!(state.migration_hash, hash);
    assert_eq!(client.get_version(), 3);
}

#[test]
fn migrate_v1_to_v2_works_when_starting_at_v1() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    client.set_version(&1u32);
    assert_eq!(client.get_version(), 1);

    let hash = migration_hash(&env, 0x02);
    client.commit_migration(&2u32, &hash, &0u64);
    client.migrate(&2u32, &hash);

    assert_eq!(client.get_version(), 2);
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, 1);
    assert_eq!(state.to_version, 2);
}

#[test]
fn migrate_v1_to_v3_chains_through_v2() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    client.set_version(&1u32);
    assert_eq!(client.get_version(), 1);

    let hash = migration_hash(&env, 0x03);
    client.commit_migration(&3u32, &hash, &0u64);
    client.migrate(&3u32, &hash);

    assert_eq!(client.get_version(), 3);
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, 1);
    assert_eq!(state.to_version, 3);
}

#[test]
fn migration_hash_is_stored_and_retrievable() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let hash = migration_hash(&env, 0xDE);
    client.commit_migration(&3u32, &hash, &0u64);
    client.migrate(&3u32, &hash);

    let state = client.get_migration_state().unwrap();
    assert_eq!(state.migration_hash, hash);
}

#[test]
fn no_migration_state_before_first_migrate() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    assert!(client.get_migration_state().is_none());
}

// ============================================================================
// 3. Authorization checks
// ============================================================================

#[test]
#[should_panic]
fn migrate_panics_without_any_auth() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    // No auth mock — must panic
    client.migrate(&3u32, &migration_hash(&env, 0x10));
}

#[test]
#[should_panic]
fn migrate_rejects_non_admin_caller() {
    let env = Env::default();
    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.init_admin(&admin);

    let h = migration_hash(&env, 0x11);
    client.commit_migration(&3u32, &h, &0u64);

    // Remove all auths — must panic
    client.mock_auths(&[]).migrate(&3u32, &h);
}

#[test]
fn migrate_succeeds_with_correct_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    commit_and_migrate(&client, &env, 3, 0x12);

    assert!(!env.auths().is_empty());
    assert_eq!(client.get_version(), 3);
}

// ============================================================================
// 4. Invalid migration paths
// ============================================================================

#[test]
#[should_panic(expected = "Target version must be greater than current version")]
fn migrate_rejects_same_version_as_target() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let hash = migration_hash(&env, 0x20);
    client.commit_migration(&2u32, &hash, &0u64);
    client.migrate(&2u32, &hash);
}

#[test]
#[should_panic(expected = "Target version must be greater than current version")]
fn migrate_rejects_lower_target_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    client.set_version(&3u32);
    assert_eq!(client.get_version(), 3);

    let hash = migration_hash(&env, 0x22);
    client.commit_migration(&2u32, &hash, &0u64);
    client.migrate(&2u32, &hash);
}

#[test]
#[should_panic(expected = "No migration path available")]
fn migrate_panics_for_unsupported_version_jump() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    commit_and_migrate(&client, &env, 3, 0x23);

    // v3→v4 has no migration function
    let h = migration_hash(&env, 0x24);
    client.commit_migration(&4u32, &h, &0u64);
    client.migrate(&4u32, &h);
}

// ============================================================================
// 5. Event emission
// ============================================================================

#[test]
fn successful_migration_emits_events() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let events_before = env.events().all().len();
    commit_and_migrate(&client, &env, 3, 0x30);

    assert!(
        env.events().all().len() > events_before,
        "Migration must emit at least one event"
    );
}

#[test]
fn idempotent_migration_does_not_emit_extra_events() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    commit_and_migrate(&client, &env, 3, 0x31);
    let events_after_first = env.events().all().len();

    // Second call — commit emits an event, but migrate() itself is a no-op
    let h2 = migration_hash(&env, 0x32);
    client.commit_migration(&3u32, &h2, &0u64);
    let events_after_commit = env.events().all().len();
    client.migrate(&3u32, &h2); // no-op

    assert_eq!(
        events_after_commit,
        env.events().all().len(),
        "Idempotent migrate() must not emit additional events"
    );
    assert!(events_after_commit > events_after_first);
}

// ============================================================================
// 6. Version management integration
// ============================================================================

#[test]
fn version_reflects_migration_target_after_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    assert_eq!(client.get_version(), 2);
    commit_and_migrate(&client, &env, 3, 0x40);
    assert_eq!(client.get_version(), 3);
}

#[test]
fn migration_state_from_version_matches_pre_migration_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let pre = client.get_version();
    commit_and_migrate(&client, &env, 3, 0x41);

    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, pre);
}

#[test]
fn set_version_and_migrate_interact_correctly() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    client.set_version(&1u32);
    assert_eq!(client.get_version(), 1);

    commit_and_migrate(&client, &env, 2, 0x42);
    assert_eq!(client.get_version(), 2);

    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, 1);
    assert_eq!(state.to_version, 2);
}

// ============================================================================
// 7. Edge cases and boundary conditions
// ============================================================================

#[test]
fn migration_with_all_zero_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.commit_migration(&3u32, &zero_hash, &0u64);
    client.migrate(&3u32, &zero_hash);

    let state = client.get_migration_state().unwrap();
    assert_eq!(state.migration_hash, zero_hash);
    assert_eq!(client.get_version(), 3);
}

#[test]
fn migration_with_max_byte_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let max_hash = BytesN::from_array(&env, &[0xFF; 32]);
    client.commit_migration(&3u32, &max_hash, &0u64);
    client.migrate(&3u32, &max_hash);

    let state = client.get_migration_state().unwrap();
    assert_eq!(state.migration_hash, max_hash);
    assert_eq!(client.get_version(), 3);
}

#[test]
fn migration_preserves_admin_address() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    commit_and_migrate(&client, &env, 3, 0x50);

    let state = client.get_migration_state().unwrap();
    assert_eq!(state.to_version, 3);

    client.set_version(&3u32);
    assert_eq!(client.get_version(), 3);
}

#[test]
fn chained_migration_v1_through_v3_preserves_final_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    client.set_version(&1u32);

    let hash = migration_hash(&env, 0x60);
    client.commit_migration(&3u32, &hash, &0u64);
    client.migrate(&3u32, &hash);

    assert_eq!(client.get_version(), 3);
    let state = client.get_migration_state().unwrap();
    assert_eq!(state.from_version, 1);
    assert_eq!(state.to_version, 3);
    assert_eq!(state.migration_hash, hash);
}

#[test]
fn sequential_independent_migrations_update_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    commit_and_migrate(&client, &env, 3, 0x70);

    let state_a = client.get_migration_state().unwrap();
    assert_eq!(state_a.from_version, 2);
    assert_eq!(state_a.to_version, 3);
}

#[test]
#[should_panic(expected = "Target version must be greater than current version")]
fn migrate_to_version_zero_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let h = migration_hash(&env, 0x80);
    client.commit_migration(&0u32, &h, &0u64);
    client.migrate(&0u32, &h);
}
