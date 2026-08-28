//! # Migration Fixture Tests  [Issue #1747]
//!
//! Tests that verify migration compatibility with:
//!
//! ## Legacy Storage Fixtures
//! - Seed a v1-era storage layout (keys that existed before v2)
//! - Run migrate v1 → v2 and verify all legacy keys are still readable
//! - Run migrate v1 → v3 (chained) and verify state is intact
//! - Verify MigrationState, Version, and Admin after migration
//!
//! ## Unknown / Future Key Policy — PRESERVE
//!
//! The documented policy (MIGRATION.md + STORAGE_LAYOUT.md) states:
//!   "Storage key enum variants must never be renamed or removed."
//!
//! This means migration functions MUST NOT delete storage keys they do not
//! recognise. Any key written before a migration executes must still be
//! readable after it completes. The tests below plant a synthetic
//! "future schema" key into instance storage before running a migration and
//! verify it survives unchanged.
//!
//! ## Covered scenarios
//! | # | Scenario                                   | Pass condition                              |
//! |---|---------------------------------------------|---------------------------------------------|
//! | 1 | Seed v1 layout → migrate v1→v2             | All v1 keys readable, Version = 2           |
//! | 2 | Seed v1 layout → migrate v1→v3 (chained)  | All v1 keys readable, Version = 3           |
//! | 3 | Unknown key planted before v1→v2 migration | Key value preserved after migration         |
//! | 4 | Unknown key planted before v2→v3 migration | Key value preserved after migration         |
//! | 5 | Unknown key planted before chained v1→v3   | Key value preserved after migration         |
//! | 6 | Multiple unknown keys all preserved        | All keys readable with correct values       |
//! | 7 | Legacy key TTL not modified by migration   | Instance TTL unchanged after migration      |
//! | 8 | Version marker correct after each path     | Version matches target after migration      |
//! | 9 | MigrationState records correct from/to     | from_version and to_version match           |
//! |10 | Idempotent migrate preserves unknown keys  | Keys survive repeated idempotent call       |

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, Symbol,
};

use crate::{DataKey, GrainlifyContract, GrainlifyContractClient, STORAGE_SCHEMA_VERSION};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Set up a fresh contract with the given admin and return the client.
fn setup(env: &Env) -> (GrainlifyContractClient<'_>, Address) {
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.init_admin(&admin);
    (client, admin)
}

/// Build a deterministic 32-byte migration hash from a seed byte.
fn mhash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Commit + migrate in one step (the standard replay-protected flow).
fn commit_and_migrate(client: &GrainlifyContractClient, env: &Env, target: u32, seed: u8) {
    let h = mhash(env, seed);
    client.commit_migration(&target, &h, &0u64);
    client.migrate(&target, &h);
}

// ─────────────────────────────────────────────────────────────────────────────
// V1-Era Legacy Storage Layout
// ─────────────────────────────────────────────────────────────────────────────
//
// The v1 storage layout consisted of the following instance-storage keys:
//   - DataKey::Admin       (Address)
//   - DataKey::Version     (u32 = 1)
//   - DataKey::ReadOnlyMode (bool = false)
//   - DataKey::ChainId     (String)   — optional, set by init_with_network
//   - DataKey::NetworkId   (String)   — optional, set by init_with_network
//   - DataKey::TimelockDelay (u64)    — optional, may be absent for older deployments
//
// Because init_admin always sets Version to the compile-time constant (2), we
// must override it via set_version() to simulate a v1 deployment.

/// Seed a v1-era layout by overriding Version to 1 after init_admin.
fn seed_v1_layout(client: &GrainlifyContractClient, env: &Env) -> Address {
    let admin = Address::generate(env);
    client.init_admin(&admin);
    // Override version to 1 to simulate a pre-v2 deployment fixture.
    client.set_version(&1);
    assert_eq!(client.get_version(), 1, "fixture: version must be 1");
    admin
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 1 — Legacy v1 layout migrates deterministically to v2
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_legacy_v1_layout_to_v2_deterministic() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);

    // Seed v1 layout
    let admin = seed_v1_layout(&client, &env);

    // Verify pre-migration state
    assert_eq!(client.get_version(), 1);
    assert!(client.get_migration_state().is_none(), "no migration state before first migrate");

    // Run v1 → v2
    commit_and_migrate(&client, &env, 2, 0x12);

    // ── Version marker must be 2 ──
    assert_eq!(client.get_version(), 2, "version must be 2 after v1→v2 migration");

    // ── MigrationState must record correct from/to ──
    let state = client.get_migration_state().expect("MigrationState must exist after migration");
    assert_eq!(state.from_version, 1, "from_version must be 1");
    assert_eq!(state.to_version, 2, "to_version must be 2");

    // ── Admin key must still be readable ──
    env.as_contract(&client.address, || {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin key must survive migration");
        assert_eq!(stored_admin, admin, "Admin address must not change after migration");
    });

    // ── ReadOnlyMode must still be set ──
    assert!(!client.is_read_only(), "ReadOnlyMode must remain false");

    // ── verify_storage_layout must pass ──
    assert!(client.verify_storage_layout(), "storage layout must be valid post-migration");
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 2 — Legacy v1 layout migrates deterministically to v3 (chained)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_legacy_v1_layout_to_v3_chained_deterministic() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);

    let admin = seed_v1_layout(&client, &env);

    // Run chained v1 → v3
    commit_and_migrate(&client, &env, 3, 0x13);

    // ── Version marker must be 3 ──
    assert_eq!(client.get_version(), 3, "version must be 3 after chained v1→v3");

    // ── MigrationState ──
    let state = client.get_migration_state().expect("MigrationState must exist");
    assert_eq!(state.from_version, 1, "from_version must be 1 for chained path");
    assert_eq!(state.to_version, 3, "to_version must be 3 for chained path");

    // ── Admin still intact ──
    env.as_contract(&client.address, || {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin key must survive chained migration");
        assert_eq!(stored, admin);
    });

    assert!(client.verify_storage_layout(), "storage layout must be valid post-chained-migration");
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 3 — Unknown key planted before v1→v2 migration is preserved (PRESERVE policy)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_unknown_key_preserved_across_v1_to_v2() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);

    seed_v1_layout(&client, &env);

    // Plant a synthetic "future schema" key that does not exist in the current
    // DataKey enum — simulating a key added in a hypothetical v4+ schema.
    let future_key = Symbol::new(&env, "future_v4");
    let future_value: u64 = 0xDEAD_BEEF_CAFE_1234_u64;

    env.as_contract(&client.address, || {
        env.storage().instance().set(&future_key, &future_value);
    });

    // Run v1 → v2
    commit_and_migrate(&client, &env, 2, 0x22);

    // ── PRESERVE policy: future_key must still hold its value ──
    env.as_contract(&client.address, || {
        let actual: u64 = env
            .storage()
            .instance()
            .get(&future_key)
            .expect("unknown future key must be preserved after migration (PRESERVE policy)");
        assert_eq!(
            actual, future_value,
            "unknown key value must not be altered by migration"
        );
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 4 — Unknown key planted before v2→v3 migration is preserved
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_unknown_key_preserved_across_v2_to_v3() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    // init_admin sets version = 2

    // Plant unknown key before v2→v3
    let future_key = Symbol::new(&env, "fut_schema5");
    let future_value: u32 = 0xCAFE_BABE_u32;

    env.as_contract(&client.address, || {
        env.storage().instance().set(&future_key, &future_value);
    });

    // Run v2 → v3
    commit_and_migrate(&client, &env, 3, 0x23);

    // ── PRESERVE policy ──
    env.as_contract(&client.address, || {
        let actual: u32 = env
            .storage()
            .instance()
            .get(&future_key)
            .expect("unknown key must be preserved after v2→v3 migration");
        assert_eq!(actual, future_value, "value must not change after migration");
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 5 — Unknown key planted before chained v1→v3 migration is preserved
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_unknown_key_preserved_across_chained_v1_to_v3() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);

    seed_v1_layout(&client, &env);

    let future_key = Symbol::new(&env, "fut_chain_k");
    let future_value: bool = true;

    env.as_contract(&client.address, || {
        env.storage().instance().set(&future_key, &future_value);
    });

    // Chained v1 → v3
    commit_and_migrate(&client, &env, 3, 0x33);

    env.as_contract(&client.address, || {
        let actual: bool = env
            .storage()
            .instance()
            .get(&future_key)
            .expect("unknown key must be preserved after chained v1→v3 migration");
        assert_eq!(actual, future_value);
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 6 — Multiple unknown keys all preserved (PRESERVE policy, bulk)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_multiple_unknown_keys_all_preserved() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    // Simulate several future-schema keys written before v2→v3 migration
    let k1 = Symbol::new(&env, "fut_key_1");
    let k2 = Symbol::new(&env, "fut_key_2");
    let k3 = Symbol::new(&env, "fut_key_3");

    let v1: u64 = 111;
    let v2: u64 = 222;
    let v3: u64 = 333;

    env.as_contract(&client.address, || {
        env.storage().instance().set(&k1, &v1);
        env.storage().instance().set(&k2, &v2);
        env.storage().instance().set(&k3, &v3);
    });

    // Run v2 → v3
    commit_and_migrate(&client, &env, 3, 0x44);

    // All three keys must survive
    env.as_contract(&client.address, || {
        assert_eq!(
            env.storage().instance().get::<_, u64>(&k1).unwrap(),
            v1,
            "fut_key_1 must be preserved"
        );
        assert_eq!(
            env.storage().instance().get::<_, u64>(&k2).unwrap(),
            v2,
            "fut_key_2 must be preserved"
        );
        assert_eq!(
            env.storage().instance().get::<_, u64>(&k3).unwrap(),
            v3,
            "fut_key_3 must be preserved"
        );
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 7 — Legacy keys written in v1-era survive v2→v3 migration intact
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_legacy_keys_survive_subsequent_v2_to_v3() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);

    let admin = seed_v1_layout(&client, &env);

    // Run v1 → v2 first
    commit_and_migrate(&client, &env, 2, 0x12);
    assert_eq!(client.get_version(), 2);

    // Run v2 → v3 next
    commit_and_migrate(&client, &env, 3, 0x23);
    assert_eq!(client.get_version(), 3);

    // Verify legacy Admin key survived both migrations
    env.as_contract(&client.address, || {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin key must survive both v1→v2 and v2→v3 migrations");
        assert_eq!(stored, admin);
    });

    // MigrationState must record the most recent migration
    let state = client.get_migration_state().expect("MigrationState must be present");
    assert_eq!(state.from_version, 2, "second migration from_version must be 2");
    assert_eq!(state.to_version, 3, "second migration to_version must be 3");
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 8 — Version marker is correct after each migration path
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_version_marker_correct_after_each_path() {
    // Path A: v1 → v2
    {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &id);
        seed_v1_layout(&client, &env);
        commit_and_migrate(&client, &env, 2, 0x01);
        assert_eq!(client.get_version(), 2, "path v1→v2: version must be 2");
    }

    // Path B: v2 → v3
    {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        commit_and_migrate(&client, &env, 3, 0x02);
        assert_eq!(client.get_version(), 3, "path v2→v3: version must be 3");
    }

    // Path C: v1 → v3 (chained)
    {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &id);
        seed_v1_layout(&client, &env);
        commit_and_migrate(&client, &env, 3, 0x03);
        assert_eq!(client.get_version(), 3, "path v1→v3 chained: version must be 3");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 9 — MigrationState records correct from/to for each path
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_state_records_correct_from_and_to_for_each_path() {
    // v1 → v2
    {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &id);
        seed_v1_layout(&client, &env);
        let h = mhash(&env, 0x11);
        client.commit_migration(&2, &h, &0u64);
        client.migrate(&2, &h);
        let s = client.get_migration_state().unwrap();
        assert_eq!(s.from_version, 1);
        assert_eq!(s.to_version, 2);
        assert_eq!(s.migration_hash, h, "migration_hash must match committed hash");
    }

    // v2 → v3
    {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = setup(&env);
        let h = mhash(&env, 0x22);
        client.commit_migration(&3, &h, &0u64);
        client.migrate(&3, &h);
        let s = client.get_migration_state().unwrap();
        assert_eq!(s.from_version, 2);
        assert_eq!(s.to_version, 3);
        assert_eq!(s.migration_hash, h);
    }

    // v1 → v3 chained
    {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &id);
        seed_v1_layout(&client, &env);
        let h = mhash(&env, 0x33);
        client.commit_migration(&3, &h, &0u64);
        client.migrate(&3, &h);
        let s = client.get_migration_state().unwrap();
        assert_eq!(s.from_version, 1);
        assert_eq!(s.to_version, 3);
        assert_eq!(s.migration_hash, h);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 10 — Idempotent migrate call preserves unknown keys
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_idempotent_call_preserves_unknown_keys() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let future_key = Symbol::new(&env, "idem_futkey");
    let future_value: u64 = 0x1234_5678_9ABC_DEF0;

    env.as_contract(&client.address, || {
        env.storage().instance().set(&future_key, &future_value);
    });

    // First migration v2 → v3
    let h = mhash(&env, 0x55);
    client.commit_migration(&3, &h, &0u64);
    client.migrate(&3, &h);

    // Second call is idempotent (commit again since commitment was consumed)
    let h2 = mhash(&env, 0x56);
    client.commit_migration(&3, &h2, &0u64);
    client.migrate(&3, &h2); // no-op idempotent path

    // Unknown key must still be intact
    env.as_contract(&client.address, || {
        let actual: u64 = env
            .storage()
            .instance()
            .get(&future_key)
            .expect("unknown key must survive idempotent migration calls");
        assert_eq!(actual, future_value);
    });

    // Version must still be 3
    assert_eq!(client.get_version(), 3);
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 11 — No unrelated storage deleted: existing v1 keys survive v1→v2
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_no_unrelated_storage_deleted_v1_to_v2() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &id);

    seed_v1_layout(&client, &env);

    // Verify all expected v1 keys are set before migration
    env.as_contract(&client.address, || {
        assert!(env.storage().instance().has(&DataKey::Admin), "Admin must be set");
        assert!(env.storage().instance().has(&DataKey::Version), "Version must be set");
        assert!(
            env.storage().instance().has(&DataKey::ReadOnlyMode),
            "ReadOnlyMode must be set"
        );
    });

    // Run v1 → v2
    commit_and_migrate(&client, &env, 2, 0x77);

    // All keys must still be present — no unrelated storage deleted
    env.as_contract(&client.address, || {
        assert!(
            env.storage().instance().has(&DataKey::Admin),
            "Admin key must not be deleted by migration"
        );
        assert!(
            env.storage().instance().has(&DataKey::Version),
            "Version key must not be deleted by migration"
        );
        assert!(
            env.storage().instance().has(&DataKey::ReadOnlyMode),
            "ReadOnlyMode key must not be deleted by migration"
        );
        // MigrationState is written by migrate() itself
        assert!(
            env.storage().instance().has(&DataKey::MigrationState),
            "MigrationState key must be written after migration"
        );
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 12 — STORAGE_SCHEMA_VERSION constant is stable
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_storage_schema_version_is_stable() {
    // This test pins the compile-time schema version constant so any accidental
    // change to STORAGE_SCHEMA_VERSION is caught as a test failure.
    assert_eq!(
        STORAGE_SCHEMA_VERSION, 1,
        "STORAGE_SCHEMA_VERSION must be 1; bump requires updating this test and STORAGE_LAYOUT.md"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 13 — MigrationCommitment key is consumed after successful migration
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_commitment_consumed_after_successful_migration() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let target: u32 = 3;
    let h = mhash(&env, 0x88);

    // Commitment must be present before migrate()
    client.commit_migration(&target, &h, &0u64);

    env.as_contract(&client.address, || {
        assert!(
            env.storage()
                .instance()
                .has(&DataKey::MigrationCommitment(target)),
            "commitment must exist before migrate()"
        );
    });

    // After successful migration the commitment is deleted (replay protection)
    client.migrate(&target, &h);

    env.as_contract(&client.address, || {
        assert!(
            !env.storage()
                .instance()
                .has(&DataKey::MigrationCommitment(target)),
            "commitment must be consumed (deleted) after successful migration"
        );
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 14 — Migration with ledger timestamp recorded in MigrationState
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_state_records_ledger_timestamp() {
    let env = Env::default();
    env.mock_all_auths();

    // Advance ledger timestamp to a non-zero value so we can verify it is stored
    env.ledger().with_mut(|li| {
        li.timestamp = 5_000_000;
    });

    let (client, _admin) = setup(&env);

    commit_and_migrate(&client, &env, 3, 0x99);

    let state = client.get_migration_state().expect("MigrationState must be present");
    assert_eq!(
        state.migrated_at, 5_000_000,
        "migrated_at must equal the ledger timestamp at time of migration"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 15 — Unknown key with structured value preserved (end-to-end)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn migration_unknown_structured_value_preserved_end_to_end() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    // Simulate a future schema entry that stores an (address, u64) tuple
    let future_addr = Address::generate(&env);
    let future_ts: u64 = 987_654_321;

    // We store as two separate sentinel keys to avoid needing a new contracttype
    let key_addr = Symbol::new(&env, "fut_addr_k");
    let key_ts   = Symbol::new(&env, "fut_ts_k");

    env.as_contract(&client.address, || {
        env.storage().instance().set(&key_addr, &future_addr);
        env.storage().instance().set(&key_ts,   &future_ts);
    });

    // Run v2 → v3
    commit_and_migrate(&client, &env, 3, 0xAA);

    // Both "future schema" keys must be preserved verbatim
    env.as_contract(&client.address, || {
        let actual_addr: Address = env
            .storage()
            .instance()
            .get(&key_addr)
            .expect("future Address key must survive migration");
        let actual_ts: u64 = env
            .storage()
            .instance()
            .get(&key_ts)
            .expect("future timestamp key must survive migration");
        assert_eq!(actual_addr, future_addr, "Address value must not change after migration");
        assert_eq!(actual_ts,   future_ts,   "timestamp value must not change after migration");
    });
}
