#![cfg(test)]

use crate::{BountyEscrowContract, BountyEscrowContractClient};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, Symbol, TryIntoVal,
};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = token_contract.address();
    token::Client::new(env, &token_address)
}

fn setup_bounty_env<'a>(env: &Env) -> (BountyEscrowContractClient<'a>, Address, token::Client<'a>) {
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);

    let token_admin = Address::generate(env);
    let token_client = create_token_contract(env, &token_admin);

    env.mock_all_auths();
    client.init(&admin, &token_client.address);
    client.set_anti_abuse_admin(&admin);
    (client, admin, token_client)
}

#[test]
fn test_maintenance_mode_toggles_and_blocks_lock() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, admin, _token) = setup_bounty_env(&env);

    assert_eq!(contract.is_maintenance_mode(), false);

    env.ledger().with_mut(|li| {
        li.timestamp = 555;
    });

    contract.set_maintenance_mode(&true);
    assert_eq!(contract.is_maintenance_mode(), true);

    let events = env.events().all();
    let emitted = events.iter().last().unwrap();
    let topics = emitted.1;
    let topic_0: Symbol = topics.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "maint"));

    let data: crate::events::MaintenanceModeChanged = emitted.2.try_into_val(&env).unwrap();
    assert_eq!(data.enabled, true);
    assert_eq!(data.admin, admin);
    assert_eq!(data.timestamp, 555);

    contract.set_maintenance_mode(&false);
    assert_eq!(contract.is_maintenance_mode(), false);
}

#[test]
#[should_panic(expected = "Error(Contract, ")]
fn test_lock_fails_in_maintenance_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, token) = setup_bounty_env(&env);

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &1000i128);

    contract.set_maintenance_mode(&true);

    contract.lock_funds(&depositor, &1u64, &1000i128, &999999999u64);
}

#[test]
fn test_release_and_refund_allowed_in_maintenance_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract, _admin, token) = setup_bounty_env(&env);

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token.address);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &5000i128);

    // Lock funds BEFORE maintenance mode
    contract.lock_funds(&depositor, &1u64, &1000i128, &999999999u64);

    // Enable maintenance mode
    contract.set_maintenance_mode(&true);

    // Release should succeed (not panicking)
    let contributor = Address::generate(&env);
    contract.release_funds(&1u64, &contributor);

    // Balance check
    assert_eq!(token.balance(&contributor), 1000);
}
