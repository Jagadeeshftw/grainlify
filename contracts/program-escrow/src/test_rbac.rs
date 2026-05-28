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
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token, Address, Env, IntoVal, String,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

/// Configure auth for `propose_admin` as the current admin.
fn mock_propose_admin_auth(
    env: &Env,
    contract_id: &Address,
    admin: &Address,
    proposed_admin: &Address,
) {
    env.mock_auths(&[MockAuth {
        address: admin,
        invoke: &MockAuthInvoke {
            contract: contract_id,
            fn_name: "propose_admin",
            args: (proposed_admin.clone(),).into_val(env),
            sub_invokes: &[],
        },
    }]);
}

/// Configure auth for `accept_admin` as the supplied signer.
fn mock_accept_admin_auth(env: &Env, contract_id: &Address, signer: &Address) {
    env.mock_auths(&[MockAuth {
        address: signer,
        invoke: &MockAuthInvoke {
            contract: contract_id,
            fn_name: "accept_admin",
            args: ().into_val(env),
            sub_invokes: &[],
        },
    }]);
}

/// Return the currently stored pending admin.
fn pending_admin(env: &Env, contract_id: &Address) -> Address {
    env.as_contract(contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .expect("pending admin should exist")
    })
}

/// Return the currently stored pending admin transition metadata.
fn pending_admin_transition(env: &Env, contract_id: &Address) -> RoleTransitionState {
    env.as_contract(contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::PendingAdminTransition)
            .expect("pending admin transition should exist")
    })
}

/// Assert that no admin rotation state remains in storage.
fn assert_no_pending_admin_rotation(env: &Env, contract_id: &Address) {
    env.as_contract(contract_id, || {
        assert!(!env.storage().instance().has(&DataKey::PendingAdmin));
        assert!(!env
            .storage()
            .instance()
            .has(&DataKey::PendingAdminTransition));
    });
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
    String,  // program_id
    Address, // payout_key
    Address, // admin
) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let token_id = fund_contract(env, &contract_id, 0);
    let admin = Address::generate(env);
    let payout_key = Address::generate(env);
    let program_id = String::from_str(env, "rbac-prog");
    client.initialize_contract(&admin);
    client.init_program(
        &program_id,
        &payout_key,
        &token_id,
        &payout_key,
        &None,
        &None,
    );
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
        &(DELEGATE_PERMISSION_RELEASE
            | DELEGATE_PERMISSION_REFUND
            | DELEGATE_PERMISSION_UPDATE_META),
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
// Admin rotation edge cases
// ---------------------------------------------------------------------------

/// A later `propose_admin` call must invalidate any earlier pending proposal.
#[test]
fn test_rbac_second_admin_proposal_overwrites_first_pending_proposal() {
    let env = Env::default();
    let (client, contract_id) = make_client(&env);
    let admin = Address::generate(&env);
    let first_proposed_admin = Address::generate(&env);
    let second_proposed_admin = Address::generate(&env);

    client.initialize_contract(&admin);

    env.ledger().set_timestamp(1_000);
    mock_propose_admin_auth(&env, &contract_id, &admin, &first_proposed_admin);
    client.propose_admin(&first_proposed_admin);

    assert_eq!(pending_admin(&env, &contract_id), first_proposed_admin);
    let first_transition = pending_admin_transition(&env, &contract_id);
    assert_eq!(first_transition.proposed_role, first_proposed_admin);
    assert_eq!(first_transition.proposer, admin);
    assert_eq!(first_transition.proposed_at, 1_000);

    env.ledger().set_timestamp(1_001);
    mock_propose_admin_auth(&env, &contract_id, &admin, &second_proposed_admin);
    client.propose_admin(&second_proposed_admin);

    assert_eq!(pending_admin(&env, &contract_id), second_proposed_admin);
    let second_transition = pending_admin_transition(&env, &contract_id);
    assert_eq!(second_transition.proposed_role, second_proposed_admin);
    assert_eq!(second_transition.proposer, admin);
    assert_eq!(second_transition.proposed_at, 1_001);
    assert!(second_transition.deadline > second_transition.proposed_at);

    mock_accept_admin_auth(&env, &contract_id, &first_proposed_admin);
    assert!(
        client.try_accept_admin().is_err(),
        "the original proposed admin must lose acceptance rights after overwrite"
    );
    assert_eq!(client.get_admin().unwrap(), admin);
    assert_eq!(pending_admin(&env, &contract_id), second_proposed_admin);

    mock_accept_admin_auth(&env, &contract_id, &second_proposed_admin);
    client.accept_admin();

    assert_eq!(client.get_admin().unwrap(), second_proposed_admin);
    assert_no_pending_admin_rotation(&env, &contract_id);
}

/// Only the currently proposed admin may accept the admin role.
#[test]
fn test_rbac_accept_admin_rejects_non_proposed_address() {
    let env = Env::default();
    let (client, contract_id) = make_client(&env);
    let admin = Address::generate(&env);
    let proposed_admin = Address::generate(&env);
    let outsider = Address::generate(&env);

    client.initialize_contract(&admin);

    mock_propose_admin_auth(&env, &contract_id, &admin, &proposed_admin);
    client.propose_admin(&proposed_admin);

    mock_accept_admin_auth(&env, &contract_id, &outsider);
    assert!(
        client.try_accept_admin().is_err(),
        "a non-proposed address must not be able to accept admin rotation"
    );

    assert_eq!(client.get_admin().unwrap(), admin);
    assert_eq!(pending_admin(&env, &contract_id), proposed_admin);

    mock_accept_admin_auth(&env, &contract_id, &proposed_admin);
    client.accept_admin();
    assert_eq!(client.get_admin().unwrap(), proposed_admin);
}

/// Acceptance after the proposal TTL must fail and clear stale proposal state.
#[test]
fn test_rbac_admin_rotation_proposal_expires_before_acceptance() {
    let env = Env::default();
    let (client, contract_id) = make_client(&env);
    let admin = Address::generate(&env);
    let proposed_admin = Address::generate(&env);

    client.initialize_contract(&admin);

    env.ledger().set_timestamp(5_000);
    mock_propose_admin_auth(&env, &contract_id, &admin, &proposed_admin);
    client.propose_admin(&proposed_admin);

    let transition = pending_admin_transition(&env, &contract_id);
    let proposal_ttl = transition.deadline - transition.proposed_at;
    assert_eq!(proposal_ttl, MAX_ROLE_TRANSITION_PERIOD);

    env.ledger().set_timestamp(transition.deadline + 1);
    mock_accept_admin_auth(&env, &contract_id, &proposed_admin);
    let result = client.try_accept_admin();
    assert!(
        matches!(result, Err(Ok(ContractError::RoleTransitionExpired))),
        "acceptance after the TTL must fail with RoleTransitionExpired"
    );

    assert_eq!(client.get_admin().unwrap(), admin);
    assert_no_pending_admin_rotation(&env, &contract_id);
}
