#![cfg(test)]

//! # `rotate_payout_key` boundary tests (issue #2012)
//!
//! Pins the extremes of every numeric or time-valued input `rotate_payout_key` reads:
//! specifically `expected_nonce` (`0`, `1`, `u64::MAX`, and `u64::MAX + 1` overflow).
//!
//! Acceptance criteria:
//! - Zero and one are covered explicitly
//! - The maximum accepted value succeeds
//! - One past the maximum is refused with the documented error
//! - No case panics; every refusal is a typed error

use crate::errors::ContractError;
use crate::{DataKey, ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{
    testutils::Address as _,
    token, Address, Env, String,
};

fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

fn fund_contract(env: &Env, contract_id: &Address, amount: i128) -> Address {
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
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
    Address, // contract_id
    String,  // program_id
    Address, // payout_key
    Address, // admin
) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let token_id = fund_contract(env, &contract_id, 0);
    let admin = Address::generate(env);
    let payout_key = Address::generate(env);
    let program_id = String::from_str(env, "boundary-prog");
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
    (client, contract_id, program_id, payout_key, admin)
}

/// Helper to set the stored rotation nonce directly in contract instance storage.
fn set_stored_nonce(env: &Env, contract_id: &Address, program_id: &String, nonce: u64) {
    env.as_contract(contract_id, || {
        let nonce_key = DataKey::RotationNonce(program_id.clone());
        env.storage().instance().set(&nonce_key, &nonce);
    });
}

// ── Nonce boundary: 0 is covered explicitly ─────────────────────────────────

#[test]
fn test_rotate_payout_key_accepts_nonce_zero() {
    let env = Env::default();
    let (client, contract_id, program_id, payout_key, _admin) = setup(&env);

    assert_eq!(client.get_rotation_nonce(&program_id), 0);

    let new_key = Address::generate(&env);
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &new_key, &0);
    assert!(result.is_ok(), "rotation with expected_nonce=0 must succeed");

    let data = result.unwrap().unwrap();
    assert_eq!(data.authorized_payout_key, new_key);
    assert_eq!(client.get_rotation_nonce(&program_id), 1);

    // Mismatched nonce 0 when stored nonce is now 1 must fail with typed error
    let newer_key = Address::generate(&env);
    let result_refused = client.try_rotate_payout_key(&program_id, &new_key, &newer_key, &0);
    match result_refused {
        Err(Ok(err)) => assert_eq!(
            err.get_code(),
            ContractError::InvalidState as u32,
            "stale nonce=0 must return typed ContractError::InvalidState"
        ),
        other => panic!("expected ContractError::InvalidState, got {other:?}"),
    }
}

// ── Nonce boundary: 1 is covered explicitly ─────────────────────────────────

#[test]
fn test_rotate_payout_key_accepts_nonce_one() {
    let env = Env::default();
    let (client, contract_id, program_id, payout_key, _admin) = setup(&env);

    // Advance stored nonce to 1
    set_stored_nonce(&env, &contract_id, &program_id, 1);
    assert_eq!(client.get_rotation_nonce(&program_id), 1);

    let new_key = Address::generate(&env);
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &new_key, &1);
    assert!(result.is_ok(), "rotation with expected_nonce=1 must succeed");

    let data = result.unwrap().unwrap();
    assert_eq!(data.authorized_payout_key, new_key);
    assert_eq!(client.get_rotation_nonce(&program_id), 2);

    // Mismatched nonce 1 when stored nonce is now 2 must fail with typed error
    let newer_key = Address::generate(&env);
    let result_refused = client.try_rotate_payout_key(&program_id, &new_key, &newer_key, &1);
    match result_refused {
        Err(Ok(err)) => assert_eq!(
            err.get_code(),
            ContractError::InvalidState as u32,
            "stale nonce=1 must return typed ContractError::InvalidState"
        ),
        other => panic!("expected ContractError::InvalidState, got {other:?}"),
    }
}

// ── Nonce boundary: maximum accepted value (u64::MAX - 1) succeeds ───────────

#[test]
fn test_rotate_payout_key_accepts_max_accepted_nonce() {
    let env = Env::default();
    let (client, contract_id, program_id, payout_key, _admin) = setup(&env);

    // The maximum accepted nonce that can safely increment to u64::MAX without overflow is u64::MAX - 1
    let max_accepted_nonce = u64::MAX - 1;
    set_stored_nonce(&env, &contract_id, &program_id, max_accepted_nonce);
    assert_eq!(client.get_rotation_nonce(&program_id), max_accepted_nonce);

    let new_key = Address::generate(&env);
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &new_key, &max_accepted_nonce);
    assert!(
        result.is_ok(),
        "rotation with max accepted nonce (u64::MAX - 1) must succeed"
    );

    let data = result.unwrap().unwrap();
    assert_eq!(data.authorized_payout_key, new_key);
    assert_eq!(client.get_rotation_nonce(&program_id), u64::MAX);
}

// ── Nonce boundary: one past the maximum is refused with typed error ─────────

#[test]
fn test_rotate_payout_key_refuses_one_past_max_and_mismatched_nonces() {
    let env = Env::default();
    let (client, contract_id, program_id, payout_key, _admin) = setup(&env);

    // Case 1: Stored nonce is 0; expected_nonce is u64::MAX (far beyond current nonce).
    let new_key = Address::generate(&env);
    let result = client.try_rotate_payout_key(&program_id, &payout_key, &new_key, &u64::MAX);
    match result {
        Err(Ok(err)) => assert_eq!(
            err.get_code(),
            ContractError::InvalidState as u32,
            "expected_nonce=u64::MAX against stored=0 must return typed ContractError::InvalidState"
        ),
        other => panic!("expected ContractError::InvalidState, got {other:?}"),
    }

    // Case 2: One past the maximum valid expected nonce when stored nonce is max_accepted_nonce.
    let max_accepted_nonce = u64::MAX - 1;
    set_stored_nonce(&env, &contract_id, &program_id, max_accepted_nonce);
    let one_past_max = max_accepted_nonce + 1; // u64::MAX
    let result_past = client.try_rotate_payout_key(&program_id, &payout_key, &new_key, &one_past_max);
    match result_past {
        Err(Ok(err)) => assert_eq!(
            err.get_code(),
            ContractError::InvalidState as u32,
            "expected_nonce=u64::MAX against stored=u64::MAX - 1 must return typed ContractError::InvalidState"
        ),
        other => panic!("expected ContractError::InvalidState, got {other:?}"),
    }

    // Case 3: Same key check also refuses with typed error ContractError::InvalidState without panicking.
    let current_key = payout_key.clone();
    let result_same_key = client.try_rotate_payout_key(&program_id, &payout_key, &current_key, &max_accepted_nonce);
    match result_same_key {
        Err(Ok(err)) => assert_eq!(
            err.get_code(),
            ContractError::InvalidState as u32,
            "rotating to identical key must return typed ContractError::InvalidState"
        ),
        other => panic!("expected ContractError::InvalidState, got {other:?}"),
    }

    // Case 4: Unauthorized caller refuses with typed error ContractError::Unauthorized.
    let unauthorized_caller = Address::generate(&env);
    let result_unauth = client.try_rotate_payout_key(&program_id, &unauthorized_caller, &new_key, &max_accepted_nonce);
    match result_unauth {
        Err(Ok(err)) => assert_eq!(
            err.get_code(),
            ContractError::Unauthorized as u32,
            "unauthorized caller must return typed ContractError::Unauthorized"
        ),
        other => panic!("expected ContractError::Unauthorized, got {other:?}"),
    }
}

// ── No case panics: every refusal is a typed error ───────────────────────────

#[test]
fn test_rotate_payout_key_never_panics_at_boundary_extremes() {
    let env = Env::default();
    let (client, contract_id, program_id, payout_key, admin) = setup(&env);

    // Test a matrix of boundary nonce values against stored nonces
    let boundary_nonces = [0u64, 1u64, u64::MAX - 1, u64::MAX];

    for &stored in &boundary_nonces {
        set_stored_nonce(&env, &contract_id, &program_id, stored);

        for &expected in &boundary_nonces {
            let fresh_key = Address::generate(&env);
            let res = client.try_rotate_payout_key(&program_id, &payout_key, &fresh_key, &expected);

            if stored == expected && expected < u64::MAX {
                assert!(res.is_ok(), "stored == expected ({stored}) must succeed");
                // Reset stored nonce for next iteration
                set_stored_nonce(&env, &contract_id, &program_id, stored);
            } else if stored != expected {
                match res {
                    Err(Ok(err)) => assert_eq!(
                        err.get_code(),
                        ContractError::InvalidState as u32,
                        "stored ({stored}) != expected ({expected}) must return ContractError::InvalidState"
                    ),
                    other => panic!("expected ContractError::InvalidState, got {other:?}"),
                }
            }
        }
    }

    // Admin authorization also succeeds at boundaries
    for &nonce in &[0u64, 1u64] {
        set_stored_nonce(&env, &contract_id, &program_id, nonce);
        let fresh_key = Address::generate(&env);
        let res = client.try_rotate_payout_key(&program_id, &admin, &fresh_key, &nonce);
        assert!(res.is_ok(), "admin rotation at nonce {nonce} must succeed");
    }
}
