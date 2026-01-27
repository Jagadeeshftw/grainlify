#![cfg(test)]

use soroban_sdk::{ vec;
    testutils::{Address as _},
    token, Address, Env
};
use soroban_sdk::testutils::Events;


use crate::{BountyEscrowContract, BountyEscrowContractClient};
use soroban_sdk::testutils::Events;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn create_test_env() -> (Env, BountyEscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    (env, client, contract_id)
}

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

#[test]
fn test_init_event() {
    let (env, client, _contract_id) = create_test_env();
    let _employee = Address::generate(&env);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let _depositor = Address::generate(&env);
    let _bounty_id = 1;

    env.mock_all_auths();

    // Initialize
    client.init(&admin.clone(), &token.clone());

    // Get all events emitted
    let events = env.events().all();

    // Verify the event was emitted (1 init event + 2 monitoring events)
    assert_eq!(events.len(), 3);
}

#[test]
fn test_lock_fund() {
    let (env, client, _contract_id) = create_test_env();
    let _employee = Address::generate(&env);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 1000;
    let deadline = 10;

    env.mock_all_auths();

    // Setup token
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    // Initialize
    client.init(&admin.clone(), &token.clone());

    token_admin_client.mint(&depositor, &amount);

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    // Get all events emitted
    let events = env.events().all();

    // Verify the event was emitted (5 original events + 4 monitoring events from init & lock_funds)
    assert_eq!(events.len(), 9);
}

#[test]
fn test_release_fund() {
    let (env, client, _contract_id) = create_test_env();

    let admin = Address::generate(&env);
    // let token = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 1000;
    let deadline = 10;

    env.mock_all_auths();

    // Setup token
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    // Initialize
    client.init(&admin.clone(), &token.clone());

    token_admin_client.mint(&depositor, &amount);

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);

    client.release_funds(&bounty_id, &contributor);

    // Get all events emitted
    let events = env.events().all();

    // Verify the event was emitted (7 original events + 6 monitoring events from init, lock_funds & release_funds)
    assert_eq!(events.len(), 13);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_lock_fund_invalid_amount() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 0; // Invalid amount
    let deadline = 100;

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, _token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin.clone(), &token.clone());

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_lock_fund_invalid_deadline() {
    let (env, client, _contract_id) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 1;
    let amount = 1000;
    let deadline = 0; // Past deadline (default timestamp is 0, so 0 <= 0)

    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin.clone(), &token.clone());
    token_admin_client.mint(&depositor, &amount);

    client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
}
