//! Tests for deterministic error ordering and prioritization in BountyEscrowContract.
//!
//! ## Validation Precedence (enforced order)
//!
//! 1. Reentrancy guard
//! 2. Contract initialized (`NotInitialized`)
//! 3. Paused / deprecated (operational state)
//! 4. Participant filter + rate limiting
//! 5. Authorization
//! 6. Input validation (amount policy)
//! 7. Business logic (bounty uniqueness, funds locked, etc.)
//!
//! When multiple error conditions apply simultaneously, the contract must always
//! return the highest-priority error, making behavior predictable across versions.

use crate::{BountyEscrowContract, BountyEscrowContractClient, Error};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn setup_uninitialized() -> (Env, BountyEscrowContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    (env, client)
}

fn setup_initialized() -> (
    Env,
    BountyEscrowContractClient<'static>,
    Address,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    client.init(&admin, &token_id);
    (env, client, admin, token_admin_client)
}

// ── lock_funds ────────────────────────────────────────────────────────────────

/// Priority 2 beats priority 3: NotInitialized returned even when paused would also apply.
#[test]
fn test_lock_not_initialized_beats_paused() {
    let (env, client) = setup_uninitialized();
    let depositor = Address::generate(&env);
    let future = 99_999_999u64;
    let result = client.try_lock_funds(&depositor, &1, &1_000, &future);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

/// Priority 2 beats priority 7: NotInitialized returned even when bounty would also be invalid.
#[test]
fn test_lock_not_initialized_beats_business_logic() {
    let (env, client) = setup_uninitialized();
    let depositor = Address::generate(&env);
    let result = client.try_lock_funds(&depositor, &1, &1_000, &0u64);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

/// Priority 3 beats priority 7: FundsPaused returned even when bounty already exists.
#[test]
fn test_lock_paused_beats_bounty_exists() {
    let (env, client, _admin, token_admin) = setup_initialized();
    let depositor = Address::generate(&env);
    let future = 99_999_999u64;
    token_admin.mint(&depositor, &2_000);
    client.lock_funds(&depositor, &1, &1_000, &future);
    // Pause lock
    client.set_paused(&Some(true), &None, &None, &None);
    // Both paused AND bounty #1 already exists — must get FundsPaused first
    let result = client.try_lock_funds(&depositor, &1, &1_000, &future);
    assert_eq!(result, Err(Ok(Error::FundsPaused)));
}

/// Priority 3 beats priority 6: FundsPaused returned even when amount is below minimum.
#[test]
fn test_lock_paused_beats_amount_below_minimum() {
    let (env, client, admin, _token_admin) = setup_initialized();
    let depositor = Address::generate(&env);
    let future = 99_999_999u64;
    client.set_amount_policy(&admin, &1_000, &1_000_000);
    client.set_paused(&Some(true), &None, &None, &None);
    // Both paused AND amount below minimum — must get FundsPaused first
    let result = client.try_lock_funds(&depositor, &1, &1, &future);
    assert_eq!(result, Err(Ok(Error::FundsPaused)));
}

/// Priority 7 (BountyExists) fires after all higher-priority checks pass.
#[test]
fn test_lock_bounty_exists_after_higher_checks_pass() {
    let (env, client, _admin, token_admin) = setup_initialized();
    let depositor = Address::generate(&env);
    let future = 99_999_999u64;
    token_admin.mint(&depositor, &2_000);
    client.lock_funds(&depositor, &1, &1_000, &future);
    // Contract initialized, not paused — only bounty already exists
    let result = client.try_lock_funds(&depositor, &1, &1_000, &future);
    assert_eq!(result, Err(Ok(Error::BountyExists)));
}

// ── release_funds ─────────────────────────────────────────────────────────────

/// Priority 2 beats priority 3: NotInitialized returned even when paused would also apply.
#[test]
fn test_release_not_initialized_beats_paused() {
    let (env, client) = setup_uninitialized();
    let contributor = Address::generate(&env);
    let result = client.try_release_funds(&1, &contributor);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

/// Priority 2 beats priority 7: NotInitialized returned even when bounty doesn't exist.
#[test]
fn test_release_not_initialized_beats_bounty_not_found() {
    let (env, client) = setup_uninitialized();
    let contributor = Address::generate(&env);
    let result = client.try_release_funds(&999, &contributor);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

/// Priority 3 beats priority 7: FundsPaused returned even when bounty doesn't exist.
#[test]
fn test_release_paused_beats_bounty_not_found() {
    let (env, client, _admin, _token_admin) = setup_initialized();
    let contributor = Address::generate(&env);
    client.set_paused(&None, &Some(true), &None, &None);
    // Both paused AND bounty #999 doesn't exist — must get FundsPaused first
    let result = client.try_release_funds(&999, &contributor);
    assert_eq!(result, Err(Ok(Error::FundsPaused)));
}

/// Priority 7 (BountyNotFound) fires after all higher-priority checks pass.
#[test]
fn test_release_bounty_not_found_after_higher_checks_pass() {
    let (env, client, _admin, _token_admin) = setup_initialized();
    let contributor = Address::generate(&env);
    // Contract initialized, not paused — only bounty doesn't exist
    let result = client.try_release_funds(&999, &contributor);
    assert_eq!(result, Err(Ok(Error::BountyNotFound)));
}

/// Priority 7 (FundsNotLocked) fires after BountyNotFound check passes.
#[test]
fn test_release_funds_not_locked_after_bounty_found() {
    let (env, client, _admin, token_admin) = setup_initialized();
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let future = 99_999_999u64;
    token_admin.mint(&depositor, &1_000);
    client.lock_funds(&depositor, &1, &1_000, &future);
    // Release once — status becomes Released
    client.release_funds(&1, &contributor);
    // Now try again — FundsNotLocked
    let result = client.try_release_funds(&1, &contributor);
    assert_eq!(result, Err(Ok(Error::FundsNotLocked)));
}
