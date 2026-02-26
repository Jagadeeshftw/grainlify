#![cfg(test)]

use crate::{BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus};
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
fn test_e2e_upgrade_with_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _escrow_id) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &100_000);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1_000;
    escrow_client.lock_funds(&depositor, &bounty_id, &10_000, &deadline);

    escrow_client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    let paused_lock = escrow_client.try_lock_funds(&depositor, &2, &1_000, &deadline);
    assert_eq!(paused_lock, Err(Ok(Error::FundsPaused)));

    let paused_release = escrow_client.try_release_funds(&bounty_id, &contributor);
    assert_eq!(paused_release, Err(Ok(Error::FundsPaused)));

    escrow_client.set_paused(&Some(false), &Some(false), &Some(false), &None);
    escrow_client.release_funds(&bounty_id, &contributor);

    let escrow = escrow_client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_e2e_upgrade_with_pause_preserves_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, escrow_id) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &100_000);

    let deadline = env.ledger().timestamp() + 1_000;
    escrow_client.lock_funds(&depositor, &1, &20_000, &deadline);
    escrow_client.lock_funds(&depositor, &2, &30_000, &deadline);

    let balance_before = token_client.balance(&escrow_id);
    assert_eq!(balance_before, 50_000);

    escrow_client.set_paused(&Some(true), &Some(true), &Some(true), &None);
    let balance_during_pause = token_client.balance(&escrow_id);
    assert_eq!(balance_during_pause, balance_before);

    escrow_client.set_paused(&Some(false), &Some(false), &Some(false), &None);
    let balance_after = token_client.balance(&escrow_id);
    assert_eq!(balance_after, balance_before);
}

#[test]
fn test_e2e_upgrade_with_pause_emergency_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let target = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, escrow_id) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &50_000);

    let deadline = env.ledger().timestamp() + 1_000;
    escrow_client.lock_funds(&depositor, &7, &50_000, &deadline);

    let not_paused = escrow_client.try_emergency_withdraw(&target);
    assert_eq!(not_paused, Err(Ok(Error::NotPaused)));

    escrow_client.set_paused(&Some(true), &None, &None, &None);
    escrow_client.emergency_withdraw(&target);

    assert_eq!(token_client.balance(&escrow_id), 0);
    assert_eq!(token_client.balance(&target), 50_000);
}
