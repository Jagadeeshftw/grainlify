
#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events},
    Address, BytesN, Env, IntoVal, Symbol,
};

// ── Import the contract under test (v1 = "current" / old version) ──────────
use crate::{GrainlifyCore, GrainlifyCoreClient};

mod new_contract {
    soroban_sdk::contractimport!(
        file = "../target/wasm32v1-none/release/grainlify_core_new.wasm"
    );
}

// ── Helper: upload the v2 WASM and return its hash ─────────────────────────
fn upload_new_wasm(env: &Env) -> BytesN<32> {
    env.deployer()
        .upload_contract_wasm(new_contract::WASM)
}

// ── Helper: upload the original (v1) WASM and return its hash ──────────────
fn upload_old_wasm(env: &Env) -> BytesN<32> {
    env.deployer()
        .upload_contract_wasm(crate::WASM) // exposed via contractimport in lib.rs test module
}

// ===========================================================================
//  TEST 1 — Basic upgrade then rollback restores version number
// ===========================================================================

#[test]
fn test_upgrade_then_rollback_restores_version() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    // Deploy the v1 (old) contract
    let contract_id = env.register(GrainlifyCore, (&admin,));
    let client = GrainlifyCoreClient::new(&env, &contract_id);

    // Assert we start at version 1
    assert_eq!(client.version(), 1u32, "Initial version should be 1");

    // Upload both WASMs so their hashes are on the simulated ledger
    let old_wasm_hash = upload_old_wasm(&env);
    let new_wasm_hash = upload_new_wasm(&env);

    // ── Upgrade to v2 ───────────────────────────────────────────────────────
    client.upgrade(&new_wasm_hash);

    // Now interact via the new contract client to confirm v2 is live
    let new_client = new_contract::Client::new(&env, &contract_id);
    assert_eq!(new_client.version(), 2u32, "After upgrade version should be 2");

    // ── Rollback: upgrade back to the original WASM hash ────────────────────
    new_client.upgrade(&old_wasm_hash);

    // Re-bind to original client type
    let rolled_back_client = GrainlifyCoreClient::new(&env, &contract_id);
    assert_eq!(
        rolled_back_client.version(),
        1u32,
        "After rollback version should be restored to 1"
    );
}

// ===========================================================================
//  TEST 2 — State is preserved across upgrade and rollback
// ===========================================================================

#[test]
fn test_state_preserved_after_upgrade_and_rollback() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(GrainlifyCore, (&admin,));
    let client = GrainlifyCoreClient::new(&env, &contract_id);

    // Seed some state: lock a program escrow of 1_000 tokens
    let initial_balance: i128 = 1_000;
    client.lock_escrow(&initial_balance);

    // Confirm state before upgrade
    assert_eq!(
        client.get_escrow_balance(),
        initial_balance,
        "Escrow balance should equal initial locked amount"
    );

    let old_wasm_hash = upload_old_wasm(&env);
    let new_wasm_hash = upload_new_wasm(&env);

    // Upgrade to v2
    client.upgrade(&new_wasm_hash);
    let new_client = new_contract::Client::new(&env, &contract_id);

    // State must still be readable from v2
    assert_eq!(
        new_client.get_escrow_balance(),
        initial_balance,
        "Escrow balance must survive upgrade to v2"
    );

    // Perform an operation on v2 (e.g. add more funds)
    let additional: i128 = 500;
    new_client.lock_escrow(&additional);

    // Rollback to v1
    new_client.upgrade(&old_wasm_hash);
    let restored_client = GrainlifyCoreClient::new(&env, &contract_id);

    // Both deposits must still be reflected
    assert_eq!(
        restored_client.get_escrow_balance(),
        initial_balance + additional,
        "Cumulative escrow balance must be intact after rollback"
    );
}

// ===========================================================================
//  TEST 3 — Re-using a previously uploaded WASM hash (no re-upload needed)
// ===========================================================================

#[test]
fn test_previous_wasm_hash_can_be_reused_without_reuploading() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(GrainlifyCore, (&admin,));
    let client = GrainlifyCoreClient::new(&env, &contract_id);

    // Upload both WASMs once at the start
    let old_wasm_hash: BytesN<32> = upload_old_wasm(&env);
    let new_wasm_hash: BytesN<32> = upload_new_wasm(&env);

    // Upgrade to v2
    client.upgrade(&new_wasm_hash);
    let new_client = new_contract::Client::new(&env, &contract_id);
    assert_eq!(new_client.version(), 2u32);

    new_client.upgrade(&old_wasm_hash);

    let restored = GrainlifyCoreClient::new(&env, &contract_id);
    assert_eq!(
        restored.version(),
        1u32,
        "Rollback using cached WASM hash should succeed and restore v1"
    );
}

// ===========================================================================
//  TEST 4 — Upgrade emits the correct executable_update system event
// ===========================================================================

#[test]
fn test_upgrade_and_rollback_emit_executable_update_events() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(GrainlifyCore, (&admin,));
    let client = GrainlifyCoreClient::new(&env, &contract_id);

    let old_wasm_hash = upload_old_wasm(&env);
    let new_wasm_hash = upload_new_wasm(&env);

    // ── Forward upgrade ─────────────────────────────────────────────────────
    client.upgrade(&new_wasm_hash);

    let events_after_upgrade = env.events().all();
    // At least one event must have been emitted
    assert!(
        !events_after_upgrade.is_empty(),
        "Upgrade should emit at least one system event"
    );

    // ── Rollback ────────────────────────────────────────────────────────────
    let new_client = new_contract::Client::new(&env, &contract_id);
    new_client.upgrade(&old_wasm_hash);

    let events_after_rollback = env.events().all();
    assert!(
        events_after_rollback.len() > events_after_upgrade.len(),
        "Rollback should emit additional system events"
    );
}

// ===========================================================================
//  TEST 5 — Only admin can perform an upgrade or rollback
// ===========================================================================

#[test]
#[should_panic]
fn test_non_admin_cannot_upgrade_or_rollback() {
    let env = Env::default();
    // Do NOT mock auths — real auth checks will run

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    let contract_id = env.register(GrainlifyCore, (&admin,));
    let client = GrainlifyCoreClient::new(&env, &contract_id);

    let new_wasm_hash = upload_new_wasm(&env);

    // Attempt upgrade from a non-admin address — should panic / be rejected
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &attacker,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "upgrade",
            args: (&new_wasm_hash,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.upgrade(&new_wasm_hash); // must panic
}

// ===========================================================================
//  TEST 6 — Multiple upgrade/rollback cycles preserve state integrity
// ===========================================================================

#[test]
fn test_multiple_upgrade_rollback_cycles_preserve_state() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(GrainlifyCore, (&admin,));
    let client = GrainlifyCoreClient::new(&env, &contract_id);

    let old_wasm_hash = upload_old_wasm(&env);
    let new_wasm_hash = upload_new_wasm(&env);

    // Lock some initial escrow
    client.lock_escrow(&2_000i128);

    for cycle in 0..3u32 {
        // Upgrade → v2
        GrainlifyCoreClient::new(&env, &contract_id).upgrade(&new_wasm_hash);
        let nc = new_contract::Client::new(&env, &contract_id);
        assert_eq!(nc.version(), 2u32, "Cycle {cycle}: version after upgrade should be 2");

        // Rollback → v1
        nc.upgrade(&old_wasm_hash);
        let rc = GrainlifyCoreClient::new(&env, &contract_id);
        assert_eq!(rc.version(), 1u32, "Cycle {cycle}: version after rollback should be 1");
        assert_eq!(
            rc.get_escrow_balance(),
            2_000i128,
            "Cycle {cycle}: escrow balance must be unchanged"
        );
    }
}

// ===========================================================================
//  TEST 7 — Rollback does not call the old constructor again
// ===========================================================================

#[test]
fn test_rollback_does_not_reinvoke_constructor() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(GrainlifyCore, (&admin,));
    let client = GrainlifyCoreClient::new(&env, &contract_id);

    let old_wasm_hash = upload_old_wasm(&env);
    let new_wasm_hash = upload_new_wasm(&env);

    // Upgrade to v2
    client.upgrade(&new_wasm_hash);

    // Rollback to v1
    let new_client = new_contract::Client::new(&env, &contract_id);
    new_client.upgrade(&old_wasm_hash);

    // Admin must still be the original admin
    let restored_client = GrainlifyCoreClient::new(&env, &contract_id);
    assert_eq!(
        restored_client.get_admin(),
        admin,
        "Admin must remain unchanged after rollback — constructor was NOT called again"
    );
}