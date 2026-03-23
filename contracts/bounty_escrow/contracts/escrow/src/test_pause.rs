#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> (BountyEscrowContractClient<'a>, Address) {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(e, &contract_id);
    (client, contract_id)
}

#[test]
fn test_granular_pause_lock() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _escrow_address) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);

    token_admin_client.mint(&depositor, &1000);

    let bounty_id_1: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    escrow_client.lock_funds(&depositor, &bounty_id_1, &100, &Some(deadline));

    escrow_client.set_paused(&Some(true), &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);

    let bounty_id_2: u64 = 2;
    let res = escrow_client.try_lock_funds(&depositor, &bounty_id_2, &100, &Some(deadline));
    assert!(res.is_err());

    escrow_client.set_paused(&Some(false), &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);

    escrow_client.lock_funds(&depositor, &bounty_id_2, &100, &Some(deadline));
}

#[test]
fn test_granular_pause_release() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    escrow_client.lock_funds(&depositor, &bounty_id, &100, &Some(deadline));

    escrow_client.set_paused(&None, &Some(true), &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.release_paused, true);

    let res = escrow_client.try_release_funds(&bounty_id, &contributor);
    assert!(res.is_err());

    escrow_client.set_paused(&None, &Some(false), &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.release_paused, false);

    escrow_client.release_funds(&bounty_id, &contributor);
}

#[test]
fn test_granular_pause_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;

    escrow_client.lock_funds(&depositor, &bounty_id, &100, &Some(deadline));

    env.ledger().set_timestamp(deadline + 1);

    escrow_client.set_paused(&None, &None, &Some(true));
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.refund_paused, true);

    let res = escrow_client.try_refund(&bounty_id);
    assert!(res.is_err());

    escrow_client.set_paused(&None, &None, &Some(false));
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.refund_paused, false);

    escrow_client.refund(&bounty_id);
}

#[test]
fn test_mixed_pause_states() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    escrow_client.set_paused(&Some(true), &Some(true), &Some(false));
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, true);
    assert_eq!(flags.refund_paused, false);

    escrow_client.set_paused(&None, &Some(false), &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
}
