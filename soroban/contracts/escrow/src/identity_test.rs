#![cfg(test)]
//! Tests for identity-aware limits functionality

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env, BytesN};

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

#[test]
fn test_set_authorized_issuer() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    // Issuer should be authorized (set in setup)
    // We can't directly query this, but we can test by trying to submit a claim
    // For now, just verify the function doesn't panic
    client.set_authorized_issuer(&issuer, &false);
    client.set_authorized_issuer(&issuer, &true);
}

#[test]
fn test_set_tier_limits() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    // Set custom tier limits
    client.set_tier_limits(
        &100_0000000,   // unverified: 100 tokens
        &1000_0000000,  // basic: 1,000 tokens
        &10000_0000000, // verified: 10,000 tokens
        &100000_0000000, // premium: 100,000 tokens
    );

    // Verify limits are applied (test with actual transaction)
    let depositor = Address::generate(&env);
    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;

    // Unverified user should be limited to 100 tokens
    let result = client.try_lock_funds(&depositor, &bounty_id, &150_0000000, &deadline);
    assert!(result.is_err()); // Should fail due to limit
}

#[test]
fn test_set_risk_thresholds() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    // Set custom risk thresholds
    client.set_risk_thresholds(
        &70, // high risk threshold
        &50, // 50% multiplier
    );

    // Function should not panic
}

#[test]
fn test_get_address_identity_default() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    let address = Address::generate(&env);
    let identity = client.get_address_identity(&address);

    // Should return default unverified tier
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

    // Should return default unverified limit (100 tokens)
    assert_eq!(limit, 100_0000000);
}

#[test]
fn test_is_claim_valid_no_claim() {
    let env = Env::default();
    let (client, _contract_id, _admin, _depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, 10_000i128);

    let address = Address::generate(&env);
    let is_valid = client.is_claim_valid(&address);

    // Should return false for address with no claim
    assert_eq!(is_valid, false);
}

#[test]
fn test_lock_funds_respects_limits() {
    let env = Env::default();
    let amount = 10_000_0000000i128; // 10,000 tokens (exceeds unverified limit of 100)
    let (client, _contract_id, _admin, depositor, _contributor, _issuer, _token_client) =
        setup_with_identity(&env, amount);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;

    // Unverified user should be limited to 100 tokens (100_0000000 stroops)
    // Trying to lock 10,000 tokens should fail
    let result = client.try_lock_funds(&depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_err());
}

#[test]
fn test_lock_funds_within_limits() {
    let env = Env::default();
    let amount = 50_0000000; // 50 tokens, within unverified limit
    let (client, _contract_id, _admin, depositor, _contributor, _issuer, token_client) =
        setup_with_identity(&env, 10_000_0000000);

    let bounty_id = 1u64;
    let deadline = env.ledger().timestamp() + 1000;

    // Should succeed as it's within the unverified limit
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    let escrow = client.get_escrow(&bounty_id);
    assert_eq!(escrow.amount, amount);
}
