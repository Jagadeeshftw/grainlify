#![cfg(test)]

//! # Lifecycle State Tests for `rotate_payout_key`
//!
//! This suite tests `rotate_payout_key` behavior across all program lifecycle states.
//!
//! ## States Covered
//!
//! - **Draft**: Program created but not yet published
//! - **Active**: Program published and operational
//! - **Archived**: Program archived (archived = true)
//!
//! ## Test Strategy
//!
//! Each test:
//! 1. Creates a program in a specific state
//! 2. Attempts to rotate the payout key
//! 3. Asserts the outcome (success or typed error)
//! 4. For terminal states, verifies the record is not mutated on refusal

use soroban_sdk::{
    testutils::Address as _,
    token, Address, Env, String,
};

use crate::{
    ProgramEscrowContract, ProgramEscrowContractClient, ProgramStatus,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

fn fund_contract(env: &Env, contract_id: &Address, amount: i128) -> Address {
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let sac = token::StellarAssetClient::new(env, &token_id);
    if amount > 0 {
        sac.mint(contract_id, &amount);
    }
    token_id
}

/// Setup a program in Draft state (not published)
fn setup_draft_program(env: &Env) -> (ProgramEscrowContractClient<'static>, String, Address, Address) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let token_id = fund_contract(env, &contract_id, 0);
    let admin = Address::generate(env);
    let payout_key = Address::generate(env);
    let program_id = String::from_str(env, "draft-prog");
    client.initialize_contract(&admin);
    client.init_program(
        &program_id,
        &payout_key,
        &token_id,
        &payout_key,
        &None,
        &None,
    );
    // Do NOT publish - stays in Draft state
    (client, program_id, payout_key, admin)
}

/// Setup a program in Active state (published)
fn setup_active_program(env: &Env) -> (ProgramEscrowContractClient<'static>, String, Address, Address) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let token_id = fund_contract(env, &contract_id, 0);
    let admin = Address::generate(env);
    let payout_key = Address::generate(env);
    let program_id = String::from_str(env, "active-prog");
    client.initialize_contract(&admin);
    client.init_program(
        &program_id,
        &payout_key,
        &token_id,
        &payout_key,
        &None,
        &None,
    );
    client.publish_program(&program_id, &admin);
    (client, program_id, payout_key, admin)
}

/// Setup a program in Archived state
fn setup_archived_program(env: &Env) -> (ProgramEscrowContractClient<'static>, String, Address, Address) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let token_id = fund_contract(env, &contract_id, 0);
    let admin = Address::generate(env);
    let payout_key = Address::generate(env);
    let program_id = String::from_str(env, "archived-prog");
    client.initialize_contract(&admin);
    client.init_program(
        &program_id,
        &payout_key,
        &token_id,
        &payout_key,
        &None,
        &None,
    );
    client.publish_program(&program_id, &admin);
    client.archive_program(&program_id);
    (client, program_id, payout_key, admin)
}

// ---------------------------------------------------------------------------
// Draft State Tests
// ---------------------------------------------------------------------------

/// `rotate_payout_key` should succeed from Draft state when called by the payout key.
#[test]
fn test_rotate_payout_key_from_draft_state_with_payout_key() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup_draft_program(&env);

    // Verify program is in Draft state
    let program_data = client.get_program_info_v2(&program_id);
    assert_eq!(program_data.status, ProgramStatus::Draft);

    // Attempt rotation with payout key
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &payout_key, &new_key, &nonce);

    // Verify the key was rotated
    assert_eq!(data.authorized_payout_key, new_key);
}

/// `rotate_payout_key` should succeed from Draft state when called by admin.
#[test]
fn test_rotate_payout_key_from_draft_state_with_admin() {
    let env = Env::default();
    let (client, program_id, _payout_key, admin) = setup_draft_program(&env);

    // Verify program is in Draft state
    let program_data = client.get_program_info_v2(&program_id);
    assert_eq!(program_data.status, ProgramStatus::Draft);

    // Attempt rotation with admin
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &admin, &new_key, &nonce);

    // Verify the key was rotated
    assert_eq!(data.authorized_payout_key, new_key);
}

/// `rotate_payout_key` should fail from Draft state with unauthorized caller.
#[test]
fn test_rotate_payout_key_from_draft_state_unauthorized() {
    let env = Env::default();
    let (client, program_id, _payout_key, _admin) = setup_draft_program(&env);

    // Verify program is in Draft state
    let program_data = client.get_program_info_v2(&program_id);
    assert_eq!(program_data.status, ProgramStatus::Draft);

    // Store original key
    let original_key = program_data.authorized_payout_key.clone();

    // Attempt rotation with unauthorized caller
    let attacker = Address::generate(&env);
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let result = client.try_rotate_payout_key(&program_id, &attacker, &new_key, &nonce);

    // Should fail
    assert!(result.is_err());

    // Verify the key was NOT rotated
    let updated_data = client.get_program_info_v2(&program_id);
    assert_eq!(updated_data.authorized_payout_key, original_key);
}

// ---------------------------------------------------------------------------
// Active State Tests
// ---------------------------------------------------------------------------

/// `rotate_payout_key` should succeed from Active state when called by the payout key.
#[test]
fn test_rotate_payout_key_from_active_state_with_payout_key() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup_active_program(&env);

    // Verify program is in Active state
    let program_data = client.get_program_info_v2(&program_id);
    assert_eq!(program_data.status, ProgramStatus::Active);

    // Attempt rotation with payout key
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &payout_key, &new_key, &nonce);

    // Verify the key was rotated
    assert_eq!(data.authorized_payout_key, new_key);
}

/// `rotate_payout_key` should succeed from Active state when called by admin.
#[test]
fn test_rotate_payout_key_from_active_state_with_admin() {
    let env = Env::default();
    let (client, program_id, _payout_key, admin) = setup_active_program(&env);

    // Verify program is in Active state
    let program_data = client.get_program_info_v2(&program_id);
    assert_eq!(program_data.status, ProgramStatus::Active);

    // Attempt rotation with admin
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &admin, &new_key, &nonce);

    // Verify the key was rotated
    assert_eq!(data.authorized_payout_key, new_key);
}

/// `rotate_payout_key` should fail from Active state with unauthorized caller.
#[test]
fn test_rotate_payout_key_from_active_state_unauthorized() {
    let env = Env::default();
    let (client, program_id, _payout_key, _admin) = setup_active_program(&env);

    // Verify program is in Active state
    let program_data = client.get_program_info_v2(&program_id);
    assert_eq!(program_data.status, ProgramStatus::Active);

    // Store original key
    let original_key = program_data.authorized_payout_key.clone();

    // Attempt rotation with unauthorized caller
    let attacker = Address::generate(&env);
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let result = client.try_rotate_payout_key(&program_id, &attacker, &new_key, &nonce);

    // Should fail
    assert!(result.is_err());

    // Verify the key was NOT rotated
    let updated_data = client.get_program_info_v2(&program_id);
    assert_eq!(updated_data.authorized_payout_key, original_key);
}

// ---------------------------------------------------------------------------
// Archived State Tests
// ---------------------------------------------------------------------------

/// `rotate_payout_key` should succeed from Archived state when called by the payout key.
///
/// Note: The current implementation does NOT block rotation on archived programs.
/// This test documents the current behavior. If the implementation changes to
/// reject rotation on archived programs, this test should be updated to expect
/// a ProgramArchived error.
#[test]
fn test_rotate_payout_key_from_archived_state_with_payout_key() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup_archived_program(&env);

    // Verify program is archived
    let program_data = client.get_program_info_v2(&program_id);
    assert!(program_data.archived);
    assert_eq!(program_data.status, ProgramStatus::Active); // Archived programs are still Active

    // Store the original key for comparison
    let original_key = program_data.authorized_payout_key.clone();

    // Attempt rotation with payout key
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &payout_key, &new_key, &nonce);

    // Current behavior: succeeds (not blocked by archived flag)
    assert_eq!(data.authorized_payout_key, new_key);
    assert_ne!(data.authorized_payout_key, original_key);
}

/// `rotate_payout_key` should succeed from Archived state when called by admin.
///
/// Note: The current implementation does NOT block rotation on archived programs.
/// This test documents the current behavior. If the implementation changes to
/// reject rotation on archived programs, this test should be updated to expect
/// a ProgramArchived error.
#[test]
fn test_rotate_payout_key_from_archived_state_with_admin() {
    let env = Env::default();
    let (client, program_id, _payout_key, admin) = setup_archived_program(&env);

    // Verify program is archived
    let program_data = client.get_program_info_v2(&program_id);
    assert!(program_data.archived);
    assert_eq!(program_data.status, ProgramStatus::Active); // Archived programs are still Active

    // Store the original key for comparison
    let original_key = program_data.authorized_payout_key.clone();

    // Attempt rotation with admin
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &admin, &new_key, &nonce);

    // Current behavior: succeeds (not blocked by archived flag)
    assert_eq!(data.authorized_payout_key, new_key);
    assert_ne!(data.authorized_payout_key, original_key);
}

/// `rotate_payout_key` should fail from Archived state with unauthorized caller.
#[test]
fn test_rotate_payout_key_from_archived_state_unauthorized() {
    let env = Env::default();
    let (client, program_id, _payout_key, _admin) = setup_archived_program(&env);

    // Verify program is archived
    let program_data = client.get_program_info_v2(&program_id);
    assert!(program_data.archived);

    // Store original key
    let original_key = program_data.authorized_payout_key.clone();

    // Attempt rotation with unauthorized caller
    let attacker = Address::generate(&env);
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let result = client.try_rotate_payout_key(&program_id, &attacker, &new_key, &nonce);

    // Should fail
    assert!(result.is_err());

    // Verify the key was NOT rotated (archived is a terminal-like state)
    let updated_data = client.get_program_info_v2(&program_id);
    assert_eq!(updated_data.authorized_payout_key, original_key);
}

// ---------------------------------------------------------------------------
// Edge Case: Wrong Nonce (All States)
// ---------------------------------------------------------------------------

/// Wrong nonce should be rejected from Draft state.
#[test]
fn test_rotate_payout_key_wrong_nonce_from_draft() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup_draft_program(&env);

    let new_key = Address::generate(&env);
    // Supply wrong nonce
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &new_key, &999);

    // Should fail
    assert!(result.is_err());
}

/// Wrong nonce should be rejected from Active state.
#[test]
fn test_rotate_payout_key_wrong_nonce_from_active() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup_active_program(&env);

    let new_key = Address::generate(&env);
    // Supply wrong nonce
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &new_key, &999);

    // Should fail
    assert!(result.is_err());
}

/// Wrong nonce should be rejected from Archived state.
#[test]
fn test_rotate_payout_key_wrong_nonce_from_archived() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup_archived_program(&env);

    let new_key = Address::generate(&env);
    // Supply wrong nonce
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &new_key, &999);

    // Should fail
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Edge Case: Self-Rotation (All States)
// ---------------------------------------------------------------------------

/// Self-rotation should be rejected from Draft state.
#[test]
fn test_rotate_payout_key_self_rotation_from_draft() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup_draft_program(&env);

    let nonce = client.get_rotation_nonce(&program_id);
    // Attempt to rotate to the same key
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &payout_key, &nonce);

    // Should fail
    assert!(result.is_err());
}

/// Self-rotation should be rejected from Active state.
#[test]
fn test_rotate_payout_key_self_rotation_from_active() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup_active_program(&env);

    let nonce = client.get_rotation_nonce(&program_id);
    // Attempt to rotate to the same key
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &payout_key, &nonce);

    // Should fail
    assert!(result.is_err());
}

/// Self-rotation should be rejected from Archived state.
#[test]
fn test_rotate_payout_key_self_rotation_from_archived() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup_archived_program(&env);

    let nonce = client.get_rotation_nonce(&program_id);
    // Attempt to rotate to the same key
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &payout_key, &nonce);

    // Should fail
    assert!(result.is_err());
}
