#![cfg(test)]
use crate::{
    BountyEscrowContract, BountyEscrowContractClient, Error as ContractError, EscrowStatus,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn create_test_env() -> (Env, BountyEscrowContractClient<'static>) {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    (env, client)
}

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token_client = token::Client::new(e, &token_id.address());
    let token_admin_client = token::StellarAssetClient::new(e, &token_id.address());
    (token_id.address(), token_client, token_admin_client)
}

#[test]
fn test_lock_funds_deadline_none() {
    let (env, client) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 901;
    let amount = 1000;

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    env.mock_all_auths();
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &amount);

    // Create bounty with None deadline
    client.lock_funds(&depositor, &bounty_id, &amount, &None);

    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.deadline, None);
    assert_eq!(escrow.status, EscrowStatus::Locked);

    // Try to refund, since there is no deadline, it should fail
    let refund_res = client.try_refund(&bounty_id);
    assert_eq!(refund_res, Err(Ok(ContractError::DeadlineNotPassed)));
}

#[test]
fn test_lock_funds_deadline_zero() {
    let (env, client) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 902;
    let amount = 1000;

    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    env.mock_all_auths();
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &amount);

    // Create bounty with 0 deadline
    client.lock_funds(&depositor, &bounty_id, &amount, &Some(0));

    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.deadline, Some(0));
    assert_eq!(escrow.status, EscrowStatus::Locked);

    // Refund should be allowed immediately
    client.refund(&bounty_id);

    let refunded_escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(refunded_escrow.status, EscrowStatus::Refunded);
    assert_eq!(token_client.balance(&depositor), amount);
}

#[test]
fn test_lock_funds_deadline_future() {
    let (env, client) = create_test_env();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let bounty_id = 903;
    let amount = 1000;

    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    env.mock_all_auths();
    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &amount);

    let now = env.ledger().timestamp();
    let deadline = now + 100;

    // Create bounty with Some(ts) deadline
    client.lock_funds(&depositor, &bounty_id, &amount, &Some(deadline));

    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.deadline, Some(deadline));

    // Try refund early
    let refund_res = client.try_refund(&bounty_id);
    assert_eq!(refund_res, Err(Ok(ContractError::DeadlineNotPassed)));

    // Advance time and refund
    env.ledger().set_timestamp(deadline + 1);
    client.refund(&bounty_id);

    let refunded_escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(refunded_escrow.status, EscrowStatus::Refunded);
}
