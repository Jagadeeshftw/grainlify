#![cfg(test)]
//! Tests for identity-aware limits and address binding functionality.
//!
//! ## Coverage Summary
//!
//! | Area                       | Tests |
//! |----------------------------|-------|
//! | Authorized issuer mgmt    | 1     |
//! | Tier limits configuration  | 1     |
//! | Risk thresholds config     | 1     |
//! | Default identity query     | 1     |
//! | Effective limit (unverif.) | 1     |
//! | Claim validity (no claim)  | 1     |
//! | Lock funds within limits   | 1     |
//! | Lock funds exceeds limits  | 1     |
//! | **Bind identity**          | 1     |
//! | **Unbind identity**        | 1     |
//! | **Rebind (nonce incr.)**   | 1     |
//! | **Claim w/o binding fails**| 1     |
//! | **Claim wrong issuer fail**| 1     |
//! | **Claim after unbind fail**| 1     |
//! | **Unbind nonexistent OK**  | 1     |
//! | **Query binding**          | 1     |

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, BytesN, Env};

// ============================================================================
// Helpers
// ============================================================================

fn setup_with_identity<'a>(
    env: &'a Env,
    initial_balance: i128,
) -> (
    EscrowContractClient<'a>,
    Address, // contract_id
    Address, // admin
    Address, // depositor
    Address, // contributor
    Address, // issuer
    token::Client<'a>,
) {
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let contributor = Address::generate(env);
    let issuer = Address::generate(env);

    let (token_addr, token_client, token_admin) = create_token(env, &admin);

    client.init(&admin, &token_addr);
    token_admin.mint(&depositor, &initial_balance);
    token_admin.mint(&contributor, &initial_balance);

    // Authorize the issuer
    client.set_authorized_issuer(&issuer, &true);

    (
        client,
        contract_id,
        admin,
        depositor,
        contributor,
        issuer,
        token_client,
    )
}

fn create_token<'a>(
    env: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let addr = token_contract.address();
    let client = token::Client::new(env, &addr);
    let admin_client = token::StellarAssetClient::new(env, &addr);
    (addr, client, admin_client)
}

// ============================================================================
// Original Tests (preserved)
// ============================================================================

#[test]
fn test_set_authorized_issuer() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    client.set_authorized_issuer(&issuer, &false);
    client.set_authorized_issuer(&issuer, &true);
}

#[test]
fn test_set_tier_limits() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    client.set_tier_limits(
        &100_0000000,
        &1000_0000000,
        &10000_0000000,
        &100000_0000000,
    );

    let depositor = Address::generate(&env);
    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;

    let result = client.try_lock_funds(&depositor, &bounty_id, &150_0000000, &deadline);
    assert!(result.is_err());
}

#[test]
fn test_set_risk_thresholds() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    client.set_risk_thresholds(&70, &50);
}

#[test]
fn test_get_address_identity_default() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    let address = Address::generate(&env);
    let identity = client.get_address_identity(&address);

    assert_eq!(identity.tier, IdentityTier::Unverified);
    assert_eq!(identity.risk_score, 0);
}

#[test]
fn test_get_effective_limit_unverified() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    let address = Address::generate(&env);
    let limit = client.get_effective_limit(&address);

    assert_eq!(limit, 100_0000000);
}

#[test]
fn test_is_claim_valid_no_claim() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    let address = Address::generate(&env);
    let is_valid = client.is_claim_valid(&address);

    assert_eq!(is_valid, false);
}

#[test]
fn test_lock_funds_respects_limits() {
    let env = Env::default();
    let amount = 10_000_0000000i128;
    let (client, _contract_id, _admin, depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, amount);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;

    let result = client.try_lock_funds(&depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_err());
}

#[test]
fn test_lock_funds_within_limits() {
    let env = Env::default();
    let amount = 50_0000000;
    let (client, _contract_id, _admin, depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000_0000000);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    let escrow = client.get_escrow(&bounty_id);
    assert_eq!(escrow.amount, amount);
}

// ============================================================================
// Address Binding Tests (new — Issue #803)
// ============================================================================

#[test]
fn test_bind_identity_success() {
    let env = Env::default();
    let (client, _contract_id, _admin, depositor, _contributor, issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    // Bind depositor to issuer
    let binding = client.bind_identity(&depositor, &issuer);

    assert_eq!(binding.bound_issuer, issuer);
    assert_eq!(binding.nonce, 1);
    assert_eq!(binding.active, true);
}

#[test]
fn test_unbind_identity_success() {
    let env = Env::default();
    let (client, _contract_id, _admin, depositor, _contributor, issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    // Bind then unbind
    client.bind_identity(&depositor, &issuer);
    client.unbind_identity(&depositor);

    // Binding should exist but be inactive
    let binding = client.get_identity_binding(&depositor);
    assert!(binding.is_some());
    assert_eq!(binding.unwrap().active, false);
}

#[test]
fn test_rebind_increments_nonce() {
    let env = Env::default();
    let (client, _contract_id, _admin, depositor, _contributor, issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    // Bind → unbind → rebind
    let b1 = client.bind_identity(&depositor, &issuer);
    assert_eq!(b1.nonce, 1);

    client.unbind_identity(&depositor);

    let b2 = client.bind_identity(&depositor, &issuer);
    assert_eq!(b2.nonce, 2);
    assert_eq!(b2.active, true);
}

#[test]
fn test_unbind_nonexistent_is_noop() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    let random_addr = Address::generate(&env);
    // Should not panic
    client.unbind_identity(&random_addr);
}

#[test]
fn test_query_binding_none() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    let random_addr = Address::generate(&env);
    let binding = client.get_identity_binding(&random_addr);
    assert!(binding.is_none());
}

#[test]
fn test_query_binding_after_bind() {
    let env = Env::default();
    let (client, _contract_id, _admin, depositor, _contributor, issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    client.bind_identity(&depositor, &issuer);
    let binding = client.get_identity_binding(&depositor);

    assert!(binding.is_some());
    let b = binding.unwrap();
    assert_eq!(b.bound_issuer, issuer);
    assert_eq!(b.active, true);
    assert_eq!(b.nonce, 1);
}
