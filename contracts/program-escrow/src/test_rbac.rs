#![cfg(test)]

//! # RBAC Tests — Payout Key Rotation and Draft Status Guards
//!
//! Verifies the role-based access control rules for `rotate_payout_key`:
//!
//! | Caller                  | Allowed? |
//! |-------------------------|----------|
//! | Current payout key      | ✅ Yes   |
//! | Contract admin          | ✅ Yes   |
//! | Arbitrary third party   | ❌ No    |
//! | Old key after rotation  | ❌ No    |
//! | Delegate                | ❌ No    |
//!
//! Also verifies Draft status guards for delegate and capability-token operations:
//! - set_program_delegate must reject programs in Draft status
//! - revoke_program_delegate must reject programs in Draft status  
//! - Delegate actions (via require_program_actor) must reject programs in Draft status
//!
//! Security assumptions validated here:
//! - A hijacked (old) key cannot re-rotate after being replaced.
//! - A delegate with full permissions cannot rotate the key.
//! - An unauthorized address cannot rotate even with a correct nonce.
//! - Delegate operations are blocked on programs in Draft status.

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

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

/// Set up a program with a distinct admin and payout key.
fn setup(
    env: &Env,
) -> (
    ProgramEscrowContractClient<'static>,
    String,   // program_id
    Address,  // payout_key
    Address,  // admin
) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let token_id = fund_contract(env, &contract_id, 0);
    let admin = Address::generate(env);
    let payout_key = Address::generate(env);
    let program_id = String::from_str(env, "rbac-prog");
    client.initialize_contract(&admin);
    client.init_program(&program_id, &payout_key, &token_id, &payout_key, &None, &None);
    (client, program_id, payout_key, admin)
}

// ---------------------------------------------------------------------------
// Positive cases
// ---------------------------------------------------------------------------

/// Current payout key is authorized to rotate.
#[test]
fn test_rbac_payout_key_can_rotate() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &payout_key, &new_key, &nonce);
    assert_eq!(data.authorized_payout_key, new_key);
}

/// Contract admin is authorized to rotate.
#[test]
fn test_rbac_admin_can_rotate() {
    let env = Env::default();
    let (client, program_id, _payout_key, admin) = setup(&env);
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    let data = client.rotate_payout_key(&program_id, &admin, &new_key, &nonce);
    assert_eq!(data.authorized_payout_key, new_key);
}

// ---------------------------------------------------------------------------
// Negative cases
// ---------------------------------------------------------------------------

/// An arbitrary third party cannot rotate the key.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_rbac_unauthorized_caller_rejected() {
    let env = Env::default();
    let (client, program_id, _payout_key, _admin) = setup(&env);
    let attacker = Address::generate(&env);
    let new_key = Address::generate(&env);
    let nonce = client.get_rotation_nonce(&program_id);
    client.rotate_payout_key(&program_id, &attacker, &new_key, &nonce);
}

/// After rotation the old key is immediately invalidated and cannot rotate again.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_rbac_old_key_cannot_rotate_after_replacement() {
    let env = Env::default();
    let (client, program_id, old_key, _admin) = setup(&env);
    let new_key = Address::generate(&env);
    let key3 = Address::generate(&env);

    // Successful rotation: old_key → new_key.
    let nonce0 = client.get_rotation_nonce(&program_id);
    client.rotate_payout_key(&program_id, &old_key, &new_key, &nonce0);

    // old_key is now invalid; attempting another rotation must fail.
    let nonce1 = client.get_rotation_nonce(&program_id);
    client.rotate_payout_key(&program_id, &old_key, &key3, &nonce1);
}

/// A delegate with all permissions cannot rotate the payout key.
///
/// Key rotation is a privileged operation reserved for the payout key itself
/// or the contract admin — delegates are explicitly excluded.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_rbac_delegate_cannot_rotate() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);
    let new_key = Address::generate(&env);

    // Grant delegate all permissions.
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &(DELEGATE_PERMISSION_RELEASE | DELEGATE_PERMISSION_REFUND | DELEGATE_PERMISSION_UPDATE_META),
    );

    let nonce = client.get_rotation_nonce(&program_id);
    // Delegate must not be able to rotate.
    client.rotate_payout_key(&program_id, &delegate, &new_key, &nonce);
}

/// Rotation on a non-existent program must panic.
#[test]
#[should_panic(expected = "Program not found")]
fn test_rbac_rotation_on_missing_program_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _contract_id) = make_client(&env);
    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    let ghost_id = String::from_str(&env, "ghost-prog");
    let caller = Address::generate(&env);
    let new_key = Address::generate(&env);
    client.rotate_payout_key(&ghost_id, &caller, &new_key, &0);
}

/// Wrong nonce is rejected even when caller is authorized.
#[test]
#[should_panic(expected = "Invalid nonce")]
fn test_rbac_wrong_nonce_rejected_for_authorized_caller() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let new_key = Address::generate(&env);
    // Supply nonce=99 when stored nonce is 0.
    client.rotate_payout_key(&program_id, &payout_key, &new_key, &99);
}

// ---------------------------------------------------------------------------
// Draft Status Guard Tests
// ---------------------------------------------------------------------------

/// set_program_delegate must reject programs in Draft status.
#[test]
#[should_panic(expected = "Cannot set delegate on program in Draft status")]
fn test_set_delegate_rejected_on_draft_program() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);
    
    // Program is in Draft status by default after init_program
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_RELEASE,
    );
}

/// revoke_program_delegate must reject programs in Draft status.
#[test]
#[should_panic(expected = "Cannot revoke delegate on program in Draft status")]
fn test_revoke_delegate_rejected_on_draft_program() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    
    // Program is in Draft status by default after init_program
    client.revoke_program_delegate(&program_id, &payout_key);
}

/// update_program_metadata by delegate must reject programs in Draft status.
#[test]
#[should_panic(expected = "Cannot perform delegate actions on program in Draft status")]
fn test_delegate_update_metadata_rejected_on_draft_program() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    
    // First publish the program to set a delegate
    client.publish_program();
    let delegate = Address::generate(&env);
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_UPDATE_META,
    );
    
    // Now create a new draft program to test delegate action rejection
    let draft_program_id = String::from_str(&env, "draft-prog");
    let token_id = fund_contract(&env, &client.address, 0);
    client.init_program(&draft_program_id, &payout_key, &token_id, &payout_key, &None, &None);
    
    // Try to update metadata on draft program as delegate - should fail
    let metadata = ProgramMetadata::empty(&env);
    client.update_program_metadata(&draft_program_id, &delegate, &metadata);
}

/// Delegate operations work after program is published.
#[test]
fn test_delegate_operations_work_after_publish() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    
    // Publish the program first
    client.publish_program();
    
    let delegate = Address::generate(&env);
    
    // Now delegate operations should work
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_UPDATE_META,
    );
    
    let program_data = client.get_program_info();
    assert_eq!(program_data.delegate, Some(delegate.clone()));
    assert_eq!(program_data.delegate_permissions, DELEGATE_PERMISSION_UPDATE_META);
    
    // Delegate should be able to update metadata
    let metadata = ProgramMetadata::empty(&env);
    client.update_program_metadata(&program_id, &delegate, &metadata);
    
    // Revoke should also work
    client.revoke_program_delegate(&program_id, &payout_key);
    let program_data = client.get_program_info();
    assert_eq!(program_data.delegate, None);
    assert_eq!(program_data.delegate_permissions, 0);
}
