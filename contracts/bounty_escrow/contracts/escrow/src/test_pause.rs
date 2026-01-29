#![cfg(test)]

use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

use crate::{BountyEscrowContract, BountyEscrowContractClient};

fn create_test_env() -> (
    Env,
    BountyEscrowContractClient<'static>,
    Address,
    Address,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths(); // Mock all authorizations for testing

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    // Create admin and token
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_addr.address());

    // Initialize contract
    client.init(&admin, &token_addr.address());

    (env, client, admin, token_addr.address(), token_client)
}

#[test]
fn test_pause_functionality() {
    let (env, client, admin, _token_addr, token_client) = create_test_env();

    // Initially not paused
    assert!(!client.is_paused());

    // Pause the contract
    client.pause(&Some(String::from_str(&env, "Security issue")));

    // Should be paused now
    assert!(client.is_paused());

    // Try to lock funds while paused - should fail
    let depositor = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = env.ledger().timestamp() + 1000;

    // This should fail with ContractPaused error
    let result = client.try_lock_funds(&depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_err());

    // Unpause the contract
    client.unpause(&Some(String::from_str(&env, "Issue resolved")));

    // Should not be paused anymore
    assert!(!client.is_paused());

    // Mint tokens to depositor and lock funds
    token_client.mint(&depositor, &amount);
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
}

#[test]
fn test_emergency_withdraw() {
    let (env, client, admin, _token_addr, token_client) = create_test_env();

    // Lock some funds first
    let depositor = Address::generate(&env);
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = env.ledger().timestamp() + 1000;

    // Mint tokens and lock funds
    token_client.mint(&depositor, &amount);
    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Pause and emergency withdraw
    client.pause(&Some(String::from_str(&env, "Emergency")));
    assert!(client.is_paused());

    // Emergency withdraw to admin
    client.emergency_withdraw(
        &admin,
        &amount,
        &String::from_str(&env, "Emergency withdrawal"),
    );

    // Contract should still be paused
    assert!(client.is_paused());
}
