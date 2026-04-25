#![cfg(test)]

use crate::MaintenanceModeChanged;
use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, String, Symbol, TryIntoVal,
};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = token_contract.address();
    token::Client::new(env, &token_address)
}

fn setup_program_with_admin<'a>(
    env: &Env,
) -> (
    ProgramEscrowContractClient<'a>,
    Address,
    Address,
    token::Client<'a>,
) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);

    client.mock_auths(&[]).initialize_contract(&admin);
    let payout_key = Address::generate(env);

    let token_admin = Address::generate(env);
    let token_client = create_token_contract(env, &token_admin);

    env.mock_all_auths();
    let program_id = String::from_str(env, "test-prog");
    client.init_program(
        &program_id,
        &payout_key,
        &token_client.address,
        &admin,
        &None,
        &None,
    );
    client.publish_program(&program_id);
    (client, admin, payout_key, token_client)
}

#[test]
fn test_maintenance_mode_toggles_and_blocks_lock() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, admin, _payout_key, _token) = setup_program_with_admin(&env);

    // Default maintenance mode is false
    assert_eq!(contract.is_maintenance_mode(), false);

    env.ledger().with_mut(|li| {
        li.timestamp = 420;
    });

    // Turn on maintenance mode
    contract.set_maintenance_mode(&true);
    assert_eq!(contract.is_maintenance_mode(), true);

    // Verify event
    let events = env.events().all();
    let emitted = events.iter().last().unwrap();
    let topics = emitted.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "MaintSt"));

    let data: MaintenanceModeChanged = emitted.2.try_into_val(&env).unwrap();
    assert_eq!(data.enabled, true);
    assert_eq!(data.admin, admin);
    assert_eq!(data.timestamp, 420);

    // Unset maintenance mode
    contract.set_maintenance_mode(&false);
    assert_eq!(contract.is_maintenance_mode(), false);
}

#[test]
#[should_panic(expected = "Funds Paused")]
fn test_lock_fails_in_maintenance_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, token) = setup_program_with_admin(&env);

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &1000);
    token.transfer(&depositor, &contract.address, &1000);

    contract.set_maintenance_mode(&true);

    // Should panic due to maintenance mode internally reusing `Funds Paused` via `check_paused`
    contract.lock_program_funds(&1000i128);
}

#[test]
fn test_release_and_refund_allowed_in_maintenance_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, token) = setup_program_with_admin(&env);

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &5000);
    token.transfer(&depositor, &contract.address, &5000);

    // Lock funds BEFORE maintenance mode
    contract.lock_program_funds(&5000i128);

    // Enable maintenance mode
    contract.set_maintenance_mode(&true);

    // Payout should succeed (not panicking)
    let recipient = Address::generate(&env);
    contract.single_payout(&recipient, &1000);

    assert_eq!(token.balance(&recipient), 1000);
}

/// Tests that emergency withdraw is allowed during maintenance mode
/// This is critical for admin to exit funds when contract is in emergency state
#[test]
fn test_emergency_withdraw_allowed_in_maintenance_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, _payout_key, token) = setup_program_with_admin(&env);

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &5000);
    token.transfer(&depositor, &contract.address, &5000);

    // Lock funds BEFORE maintenance mode
    contract.lock_program_funds(&5000i128);

    // Enable maintenance mode (also need to set lock_paused for emergency_withdraw)
    contract.set_maintenance_mode(&true);
    contract.set_paused(&Some(true), &None, &None, &None); // Set lock_paused to enable emergency withdraw

    // Emergency withdraw should succeed during maintenance mode
    let withdraw_recipient = Address::generate(&env);
    contract.emergency_withdraw(&withdraw_recipient);

    // Verify all funds were withdrawn
    assert_eq!(token.balance(&withdraw_recipient), 5000);
    assert_eq!(token.balance(&contract.address), 0);
}
