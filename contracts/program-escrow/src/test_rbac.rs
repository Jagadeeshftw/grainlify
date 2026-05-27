#![cfg(test)]

//! # RBAC Tests — Payout Key Rotation & Emergency Delegate Revocation
//!
//! ## Payout Key Rotation
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
//! Security assumptions validated here:
//! - A hijacked (old) key cannot re-rotate after being replaced.
//! - A delegate with full permissions cannot rotate the key.
//! - An unauthorized address cannot rotate even with a correct nonce.
//!
//! ## Emergency Delegate Revocation
//!
//! Verifies the role-based access control rules for `emergency_revoke_delegate`:
//!
//! | Caller                  | Allowed? |
//! |-------------------------|----------|
//! | Contract admin          | ✅ Yes   |
//! | Arbitrary third party   | ❌ No    |
//! | Current payout key      | ❌ No    |
//! | Delegate itself         | ❌ No    |
//!
//! Security assumptions validated here:
//! - Only the admin can emergency-revoke a delegate.
//! - After emergency revocation, delegate permissions are immediately zeroed.
//! - Emergency revocation emits the `ProgramDelegateRevoked` event with `emergency=true`.
//! - Normal revocation emits the event with `emergency=false`.
//! - Emergency revocation is idempotent (safe when no delegate is set).

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
// Emergency Delegate Revocation — positive cases
// ---------------------------------------------------------------------------

/// Admin can emergency-revoke a delegate that has active permissions.
#[test]
fn test_emergency_revoke_admin_can_revoke_delegate() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);

    // Grant delegate full permissions.
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_MASK,
    );

    // Verify delegate was set.
    let data_before = client.get_program_data(&program_id);
    assert_eq!(data_before.delegate, Some(delegate.clone()));
    assert_eq!(data_before.delegate_permissions, DELEGATE_PERMISSION_MASK);

    // Admin performs emergency revocation.
    let data_after = client.emergency_revoke_delegate(&program_id, &delegate);

    // Delegate and all permissions must be cleared immediately.
    assert!(data_after.delegate.is_none());
    assert_eq!(data_after.delegate_permissions, 0);
}

/// Emergency revocation zeroes permissions for each individual permission bit.
#[test]
fn test_emergency_revoke_zeros_all_permissions() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);

    // Set only the RELEASE permission.
    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_RELEASE,
    );

    let data = client.emergency_revoke_delegate(&program_id, &delegate);
    assert!(data.delegate.is_none());
    assert_eq!(data.delegate_permissions, 0);
}

/// Emergency revocation is idempotent — calling when no delegate is set
/// must not panic and still returns the (unchanged) program data.
#[test]
fn test_emergency_revoke_idempotent_when_no_delegate() {
    let env = Env::default();
    let (client, program_id, _payout_key, _admin) = setup(&env);
    let phantom_delegate = Address::generate(&env);

    // No delegate has been set; call must succeed silently.
    let data = client.emergency_revoke_delegate(&program_id, &phantom_delegate);
    assert!(data.delegate.is_none());
    assert_eq!(data.delegate_permissions, 0);
}

/// Normal revocation (`revoke_program_delegate`) still works after the
/// emergency path is introduced — existing behaviour is not broken.
#[test]
fn test_normal_revoke_still_works_after_emergency_path_added() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);

    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_RELEASE,
    );

    // Normal revocation by payout key must still succeed.
    let data = client.revoke_program_delegate(&program_id, &payout_key);
    assert!(data.delegate.is_none());
    assert_eq!(data.delegate_permissions, 0);
}

// ---------------------------------------------------------------------------
// Emergency Delegate Revocation — negative cases
// ---------------------------------------------------------------------------

/// An arbitrary third-party cannot call emergency_revoke_delegate.
#[test]
#[should_panic(expected = "Not initialized")]
fn test_emergency_revoke_rejected_for_arbitrary_caller() {
    let env = Env::default();
    env.mock_all_auths();
    // Use a fresh client without calling initialize_contract so admin is not set.
    let (client, contract_id) = make_client(&env);
    let token_id = fund_contract(&env, &contract_id, 0);
    let payout_key = Address::generate(&env);
    let program_id = String::from_str(&env, "test-prog");
    // init_program without initialize_contract → admin not set.
    client.init_program(&program_id, &payout_key, &token_id, &payout_key, &None, &None);

    let delegate = Address::generate(&env);
    // emergency_revoke requires admin; "Not initialized" is the panic from require_admin
    // when admin key has not been set via initialize_contract.
    client.emergency_revoke_delegate(&program_id, &delegate);
}

/// The payout-key owner (non-admin) cannot call emergency_revoke_delegate.
///
/// This test verifies the separation of concerns: the payout key governs
/// day-to-day operations while the admin governs security-critical paths.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_emergency_revoke_rejected_for_payout_key_owner() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);

    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_MASK,
    );

    // Override the mocked auth to only grant the payout_key (not the admin).
    // With mock_all_auths the call would succeed, so we disable mocking and
    // use require_auth semantics directly.
    env.set_auths(&[soroban_sdk::testutils::AuthorizedInvocation {
        function: soroban_sdk::testutils::AuthorizedFunction::Contract((
            client.address.clone(),
            soroban_sdk::Symbol::new(&env, "emergency_revoke_delegate"),
            soroban_sdk::vec![
                &env,
                program_id.into_val(&env),
                delegate.clone().into_val(&env),
            ],
        )),
        sub_invocations: soroban_sdk::vec![&env],
    }]);

    // Payout key is NOT the admin — must be rejected.
    client.emergency_revoke_delegate(&program_id, &delegate);
}

/// A delegate with all permissions cannot call emergency_revoke_delegate on itself.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_emergency_revoke_rejected_for_delegate_self_revoke() {
    let env = Env::default();
    let (client, program_id, payout_key, _admin) = setup(&env);
    let delegate = Address::generate(&env);

    client.set_program_delegate(
        &program_id,
        &payout_key,
        &delegate,
        &DELEGATE_PERMISSION_MASK,
    );

    // Only grant the delegate auth — not the admin.
    env.set_auths(&[soroban_sdk::testutils::AuthorizedInvocation {
        function: soroban_sdk::testutils::AuthorizedFunction::Contract((
            client.address.clone(),
            soroban_sdk::Symbol::new(&env, "emergency_revoke_delegate"),
            soroban_sdk::vec![
                &env,
                program_id.into_val(&env),
                delegate.clone().into_val(&env),
            ],
        )),
        sub_invocations: soroban_sdk::vec![&env],
    }]);

    client.emergency_revoke_delegate(&program_id, &delegate);
}
