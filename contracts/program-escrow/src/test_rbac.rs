#![cfg(test)]

//! # RBAC Tests — Payout Key Rotation
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

// =========================================================================
// ISSUE #1272: 24h time-lock delay for admin/controller rotation acceptance
// =========================================================================

/// Helper: set up a contract with admin and a program with payout_key.
fn setup_with_program(
    env: &Env,
) -> (
    ProgramEscrowContractClient<'static>,
    Address, // admin
    Address, // payout_key (controller)
    String,  // program_id
) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let token_id = fund_contract(env, &contract_id, 0);
    let admin = Address::generate(env);
    let payout_key = Address::generate(env);
    let program_id = String::from_str(env, "timelock-prog");
    client.initialize_contract(&admin);
    client.init_program(&program_id, &payout_key, &token_id, &payout_key, &None, &None);
    (client, admin, payout_key, program_id)
}

// --- Admin rotation timelock ---

/// accept_admin fails immediately after propose_admin (timelock not elapsed).
#[test]
fn test_accept_admin_before_timelock_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000_000);

    let (client, _admin, _payout_key, _program_id) = setup_with_program(&env);
    let new_admin = Address::generate(&env);

    client.propose_admin(&new_admin);

    // Advance time by less than 24h (e.g., 1 hour)
    env.ledger().with_mut(|li| li.timestamp = 1_000_000 + 3_600);

    let result = client.try_accept_admin();
    assert_eq!(
        result,
        Err(Ok(ContractError::RotationTimelockActive)),
        "accept_admin must fail before 24h timelock expires"
    );
}

/// accept_admin succeeds exactly at the 24h boundary.
#[test]
fn test_accept_admin_after_timelock_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000_000);

    let (client, _admin, _payout_key, _program_id) = setup_with_program(&env);
    let new_admin = Address::generate(&env);

    client.propose_admin(&new_admin);

    // Advance time by exactly 24h
    env.ledger().with_mut(|li| li.timestamp = 1_000_000 + ROTATION_TIMELOCK_DELAY);

    // Should succeed
    client.accept_admin();
}

/// accept_admin succeeds after more than 24h.
#[test]
fn test_accept_admin_well_after_timelock_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000_000);

    let (client, _admin, _payout_key, _program_id) = setup_with_program(&env);
    let new_admin = Address::generate(&env);

    client.propose_admin(&new_admin);

    // Advance time by 48h
    env.ledger().with_mut(|li| li.timestamp = 1_000_000 + 2 * ROTATION_TIMELOCK_DELAY);

    client.accept_admin();
}

/// Admin can cancel a pending rotation within the timelock window.
#[test]
fn test_admin_can_cancel_rotation_within_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000_000);

    let (client, _admin, _payout_key, _program_id) = setup_with_program(&env);
    let new_admin = Address::generate(&env);

    client.propose_admin(&new_admin);

    // Cancel within the timelock window (1 hour after proposal)
    env.ledger().with_mut(|li| li.timestamp = 1_000_000 + 3_600);
    client.cancel_admin_rotation();

    // After cancellation, accept_admin must fail with NoAdminRotationInProgress
    let result = client.try_accept_admin();
    assert_eq!(
        result,
        Err(Ok(ContractError::NoAdminRotationInProgress)),
        "accept_admin must fail after cancellation"
    );
}

/// After cancellation, a new proposal can be made.
#[test]
fn test_new_proposal_allowed_after_cancellation() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000_000);

    let (client, _admin, _payout_key, _program_id) = setup_with_program(&env);
    let new_admin = Address::generate(&env);
    let another_admin = Address::generate(&env);

    client.propose_admin(&new_admin);
    client.cancel_admin_rotation();

    // Should be able to propose again
    client.propose_admin(&another_admin);

    // Advance past timelock and accept
    env.ledger().with_mut(|li| li.timestamp = 1_000_000 + ROTATION_TIMELOCK_DELAY + 1);
    client.accept_admin();
}

// --- Controller rotation timelock ---

/// accept_controller fails immediately after propose_controller (timelock not elapsed).
#[test]
fn test_accept_controller_before_timelock_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 2_000_000);

    let (client, _admin, payout_key, program_id) = setup_with_program(&env);
    let new_controller = Address::generate(&env);

    client.propose_controller(&program_id, &payout_key, &new_controller);

    // Advance time by less than 24h
    env.ledger().with_mut(|li| li.timestamp = 2_000_000 + 3_600);

    let result = client.try_accept_controller(&program_id);
    assert_eq!(
        result,
        Err(Ok(ContractError::RotationTimelockActive)),
        "accept_controller must fail before 24h timelock expires"
    );
}

/// accept_controller succeeds exactly at the 24h boundary.
#[test]
fn test_accept_controller_after_timelock_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 2_000_000);

    let (client, _admin, payout_key, program_id) = setup_with_program(&env);
    let new_controller = Address::generate(&env);

    client.propose_controller(&program_id, &payout_key, &new_controller);

    // Advance time by exactly 24h
    env.ledger().with_mut(|li| li.timestamp = 2_000_000 + ROTATION_TIMELOCK_DELAY);

    let data = client.accept_controller(&program_id);
    assert_eq!(data.authorized_payout_key, new_controller);
}

/// Admin can cancel a pending controller rotation within the timelock window.
#[test]
fn test_admin_can_cancel_controller_rotation_within_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 2_000_000);

    let (client, admin, payout_key, program_id) = setup_with_program(&env);
    let new_controller = Address::generate(&env);

    client.propose_controller(&program_id, &payout_key, &new_controller);

    // Cancel within the timelock window
    env.ledger().with_mut(|li| li.timestamp = 2_000_000 + 3_600);
    client.cancel_controller_rotation(&program_id, &admin);

    // After cancellation, accept_controller must fail
    let result = client.try_accept_controller(&program_id);
    assert_eq!(
        result,
        Err(Ok(ContractError::NoControllerRotationInProgress)),
        "accept_controller must fail after cancellation"
    );
}

/// accept_admin with no pending proposal returns NoAdminRotationInProgress.
#[test]
fn test_accept_admin_no_pending_proposal_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _payout_key, _program_id) = setup_with_program(&env);

    let result = client.try_accept_admin();
    assert_eq!(
        result,
        Err(Ok(ContractError::NoAdminRotationInProgress)),
    );
}

/// accept_controller with no pending proposal returns NoControllerRotationInProgress.
#[test]
fn test_accept_controller_no_pending_proposal_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _payout_key, program_id) = setup_with_program(&env);

    let result = client.try_accept_controller(&program_id);
    assert_eq!(
        result,
        Err(Ok(ContractError::NoControllerRotationInProgress)),
    );
}
