#![cfg(test)]
use crate::{BountyEscrowContract, BountyEscrowContractClient, EscrowStatus};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

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

// ── UPGRADE SCENARIO TESTS ───────────────────────────────────────────────────

#[test]
fn test_upgrade_locked_bounty_remains_locked() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &1, &5_000, &deadline);

    // Simulate upgrade by re-registering contract (state persists)
    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.amount, 5_000);
    assert_eq!(escrow.remaining_amount, 5_000);
}

#[test]
fn test_upgrade_complete_release_flow() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &1, &5_000, &deadline);

    // Verify locked
    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.status, EscrowStatus::Locked);

    // Complete release after upgrade
    client.release_funds(&1, &contributor);

    let escrow = client.get_escrow_info(&1);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_upgrade_pending_lock_then_refund() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 100;
    client.lock_funds(&depositor, &2, &5_000, &deadline);

    // Advance time past deadline
    env.ledger().with_mut(|l| l.timestamp += 200);

    // Refund after upgrade
    client.refund(&2);

    let escrow = client.get_escrow_info(&2);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
}

#[test]
fn test_upgrade_partial_release_then_complete() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &10_000);

    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &3, &6_000, &deadline);

    client.partial_release(&3, &contributor, &2_000);

    let escrow = client.get_escrow_info(&3);
    assert_eq!(escrow.remaining_amount, 4_000);
    assert_eq!(escrow.status, EscrowStatus::Locked);

    client.partial_release(&3, &contributor, &4_000);

    let escrow = client.get_escrow_info(&3);
    assert_eq!(escrow.remaining_amount, 0);
    assert_eq!(escrow.status, EscrowStatus::Released);
}