#![cfg(test)]
use crate::{BountyEscrowContract, BountyEscrowContractClient};
use soroban_sdk::{
    testutils::{Address as _},
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

#[test]
fn test_fee_same_token() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    let (token_addr, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    // Initialize contract
    client.init(&admin, &token_addr);

    // Configure fees: 5% lock, 10% release (500 and 1000 basis points)
    client.update_fee_config(
        &Some(500),    // lock_fee_rate: 5%
        &Some(1000),   // release_fee_rate: 10%
        &Some(treasury.clone()),
        &None,         // fee_token: None (use escrow token)
        &Some(true),   // fee_enabled: true
    );

    // Mint tokens to depositor
    token_admin_client.mint(&depositor, &20_000);
    
    // Lock funds: 10,000 tokens
    // Lock fee should be 10,000 * 5% = 500
    // Total cost for depositor: 10,500
    let bounty_id = 1;
    let amount = 10_000;
    client.lock_funds(&depositor, &bounty_id, &amount, &(env.ledger().timestamp() + 100));

    // Check balances after lock
    assert_eq!(token_client.balance(&depositor), 20_000 - 10_000 - 500);
    assert_eq!(token_client.balance(&treasury), 500);
    assert_eq!(token_client.balance(&client.address), 10_000);

    // Release funds
    // Release fee should be 10,000 * 10% = 1,000
    // Contributor receives: 10,000 - 1,000 = 9,000
    client.release_funds(&bounty_id, &contributor);

    // Check balances after release
    assert_eq!(token_client.balance(&contributor), 9_000);
    assert_eq!(token_client.balance(&treasury), 500 + 1_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_fee_different_token() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    // Create Escrow Token (Token A)
    let (token_a_addr, token_a_client, token_a_admin_client) = create_token_contract(&env, &token_admin);
    // Create Fee Token (Token B)
    let (token_b_addr, token_b_client, token_b_admin_client) = create_token_contract(&env, &token_admin);

    // Initialize contract with Token A
    client.init(&admin, &token_a_addr);

    // Configure fees: 5% lock in Token B, 10% release (always in escrow token for now)
    client.update_fee_config(
        &Some(500),    // lock_fee_rate: 5%
        &Some(1000),   // release_fee_rate: 10%
        &Some(treasury.clone()),
        &Some(token_b_addr.clone()), // fee_token: Token B
        &Some(true),   // fee_enabled: true
    );

    // Mint tokens
    token_a_admin_client.mint(&depositor, &20_000);
    token_b_admin_client.mint(&depositor, &5_000);
    
    // Lock funds: 10,000 Token A
    // Lock fee should be 10,000 * 5% = 500 Token B
    let bounty_id = 1;
    let amount = 10_000;
    client.lock_funds(&depositor, &bounty_id, &amount, &(env.ledger().timestamp() + 100));

    // Check balances after lock
    assert_eq!(token_a_client.balance(&depositor), 10_000);
    assert_eq!(token_b_client.balance(&depositor), 4_500);
    assert_eq!(token_b_client.balance(&treasury), 500);
    assert_eq!(token_a_client.balance(&client.address), 10_000);

    // Release funds
    // Release fee should be 10,000 * 10% = 1,000 Token A (deducted from escrow)
    client.release_funds(&bounty_id, &contributor);

    // Check balances after release
    assert_eq!(token_a_client.balance(&contributor), 9_000);
    assert_eq!(token_a_client.balance(&treasury), 1_000); // Treasury now has 1000 Token A
    assert_eq!(token_b_client.balance(&treasury), 500);   // Treasury still has 500 Token B
    assert_eq!(token_a_client.balance(&client.address), 0);
}

#[test]
fn test_fee_disabled() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    let (token_addr, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    // Initialize contract
    client.init(&admin, &token_addr);

    // Configure fees: but keep them disabled
    client.update_fee_config(
        &Some(500),
        &Some(1000),
        &Some(treasury.clone()),
        &None,
        &Some(false), // fee_enabled: false
    );

    token_admin_client.mint(&depositor, &20_000);
    
    let bounty_id = 1;
    let amount = 10_000;
    client.lock_funds(&depositor, &bounty_id, &amount, &(env.ledger().timestamp() + 100));

    // Check balances: no fees should be taken
    assert_eq!(token_client.balance(&depositor), 10_000);
    assert_eq!(token_client.balance(&treasury), 0);

    client.release_funds(&bounty_id, &contributor);

    assert_eq!(token_client.balance(&contributor), 10_000);
    assert_eq!(token_client.balance(&treasury), 0);
}

#[test]
fn test_cross_token_leakage_batch() {
    let (env, client, _contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    
    let (token_addr, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token_addr);
    client.update_fee_config(
        &Some(100), // 1% lock fee
        &Some(200), // 2% release fee
        &Some(treasury.clone()),
        &None,
        &Some(true),
    );

    let dep1 = Address::generate(&env);
    let dep2 = Address::generate(&env);
    token_admin_client.mint(&dep1, &10_000);
    token_admin_client.mint(&dep2, &10_000);

    use crate::LockFundsItem;
    let items = soroban_sdk::vec![&env, 
        LockFundsItem {
            bounty_id: 1,
            depositor: dep1.clone(),
            amount: 1_000,
            deadline: env.ledger().timestamp() + 100,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: dep2.clone(),
            amount: 2_000,
            deadline: env.ledger().timestamp() + 100,
        }
    ];

    client.batch_lock_funds(&items);

    // Check balances
    // dep1: 10,000 - 1,000 - 10 (1% of 1000) = 8,990
    // dep2: 10,000 - 2_000 - 20 (1% of 2000) = 7,980
    // treasury: 10 + 20 = 30
    assert_eq!(token_client.balance(&dep1), 8_990);
    assert_eq!(token_client.balance(&dep2), 7_980);
    assert_eq!(token_client.balance(&treasury), 30);
    assert_eq!(token_client.balance(&client.address), 3_000);

    let cont1 = Address::generate(&env);
    let cont2 = Address::generate(&env);
    
    use crate::ReleaseFundsItem;
    let rel_items = soroban_sdk::vec![&env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: cont1.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: cont2.clone(),
        }
    ];

    client.batch_release_funds(&rel_items);

    // Release fees
    // cont1: 1,000 - 20 (2% of 1000) = 980
    // cont2: 2,000 - 40 (2% of 2000) = 1,960
    // treasury: 30 + 20 + 40 = 90
    assert_eq!(token_client.balance(&cont1), 980);
    assert_eq!(token_client.balance(&cont2), 1_960);
    assert_eq!(token_client.balance(&treasury), 90);
    assert_eq!(token_client.balance(&client.address), 0);
}
