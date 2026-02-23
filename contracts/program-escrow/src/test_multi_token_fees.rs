#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

struct Setup {
    env: Env,
    client: ProgramEscrowContractClient<'static>,
    admin: Address,
    treasury: Address,
    token_a: token::Client<'static>,
    token_a_admin: token::StellarAssetClient<'static>,
    token_b: token::Client<'static>,
    token_b_admin: token::StellarAssetClient<'static>,
    program_id: String,
}

impl Setup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        
        let token_a_id = env.register_stellar_asset_contract(Address::generate(&env));
        let token_a = token::Client::new(&env, &token_a_id);
        let token_a_admin = token::StellarAssetClient::new(&env, &token_a_id);

        let token_b_id = env.register_stellar_asset_contract(Address::generate(&env));
        let token_b = token::Client::new(&env, &token_b_id);
        let token_b_admin = token::StellarAssetClient::new(&env, &token_b_id);

        let program_id = String::from_str(&env, "fee-test-prog");
        
        // Initialize contract admin
        client.initialize_contract(&admin);
        
        // Initialize program
        client.init_program(&program_id, &admin, &token_a_id, &admin, &None);

        Self {
            env,
            client,
            admin,
            treasury,
            token_a,
            token_a_admin,
            token_b,
            token_b_admin,
            program_id,
        }
    }
}

#[test]
fn test_fee_same_token() {
    let setup = Setup::new();
    
    setup.client.update_fee_config(
        &Some(0),    // lock_fee: 0% (since lock doesn't transfer yet)
        &Some(1000),   // payout_fee: 10%
        &Some(setup.treasury.clone()),
        &None,         // use escrow token
        &Some(true),
    );

    let initial_amount = 100_000_i128;
    setup.token_a_admin.mint(&setup.client.address, &initial_amount);
    setup.client.lock_program_funds(&initial_amount);

    let recipient = Address::generate(&setup.env);
    let payout_amount = 50_000_i128;
    
    setup.client.single_payout(&recipient, &payout_amount);

    // 10% of 50,000 is 5,000
    assert_eq!(setup.token_a.balance(&setup.treasury), 5_000);
    assert_eq!(setup.token_a.balance(&recipient), 45_000);
    assert_eq!(setup.token_a.balance(&setup.client.address), 50_000);
}

#[test]
fn test_fee_different_token() {
    let setup = Setup::new();
    
    // Configure fee in Token B (5%)
    setup.client.update_fee_config(
        &Some(0),
        &Some(500), // 5%
        &Some(setup.treasury.clone()),
        &Some(setup.token_b.address.clone()),
        &Some(true),
    );

    let initial_amount = 100_000_i128;
    setup.token_a_admin.mint(&setup.client.address, &initial_amount);
    setup.client.lock_program_funds(&initial_amount);

    // Mint some Token B to the contract so it can pay the fee
    setup.token_b_admin.mint(&setup.client.address, &10_000);

    let recipient = Address::generate(&setup.env);
    let payout_amount = 40_000_i128; // Escrow token (Token A)
    
    setup.client.single_payout(&recipient, &payout_amount);

    // 5% of 40,000 is 2,000. This should be paid in Token B.
    assert_eq!(setup.token_b.balance(&setup.treasury), 2_000);
    // Recipient should get the FULL payout_amount in Token A (no deduction)
    assert_eq!(setup.token_a.balance(&recipient), 40_000);
    // Contract balance of Token A should remain 60,000 (100k - 40k)
    assert_eq!(setup.token_a.balance(&setup.client.address), 60_000);
    // Contract balance of Token B should be 8,000 (10k - 2k)
    assert_eq!(setup.token_b.balance(&setup.client.address), 8_000);
}

#[test]
fn test_batch_payout_fees() {
    let setup = Setup::new();
    
    setup.client.update_fee_config(
        &None,
        &Some(2000), // 20%
        &Some(setup.treasury.clone()),
        &None,
        &Some(true),
    );

    let initial_amount = 200_000_i128;
    setup.token_a_admin.mint(&setup.client.address, &initial_amount);
    setup.client.lock_program_funds(&initial_amount);

    let r1 = Address::generate(&setup.env);
    let r2 = Address::generate(&setup.env);
    
    let recipients = vec![&setup.env, r1.clone(), r2.clone()];
    let amounts = vec![&setup.env, 50_000, 100_000];
    
    setup.client.batch_payout(&recipients, &amounts);

    // 20% of 50,000 = 10,000
    // 20% of 100,000 = 20,000
    // Total fee = 30,000
    assert_eq!(setup.token_a.balance(&setup.treasury), 30_000);
    assert_eq!(setup.token_a.balance(&r1), 40_000);
    assert_eq!(setup.token_a.balance(&r2), 80_000);
    assert_eq!(setup.token_a.balance(&setup.client.address), 50_000);
}
#[test]
fn test_multi_token_isolation() {
    let setup = Setup::new();
    
    // Program 1 is already setup with Token A (global PROGRAM_DATA fallback)
    let prog1_id = setup.program_id.clone();
    
    // Program 2: Initialize with Token B
    let prog2_id = String::from_str(&setup.env, "prog-2");
    let authorized_key_2 = Address::generate(&setup.env);
    
    setup.client.initialize_program(
        &prog2_id,
        &authorized_key_2,
        &setup.token_b.address,
        &setup.admin,
        &None,
    );

    // Lock funds for both
    let amount_1 = 100_000_i128;
    let amount_2 = 200_000_i128;
    
    setup.token_a_admin.mint(&setup.client.address, &amount_1);
    setup.token_b_admin.mint(&setup.client.address, &amount_2);
    
    setup.client.lock_program_funds_v2(&prog1_id, &amount_1);
    setup.client.lock_program_funds_v2(&prog2_id, &amount_2);

    // Verify balances in stats
    let stats1 = setup.client.get_program_info_v2(&prog1_id);
    let stats2 = setup.client.get_program_info_v2(&prog2_id);
    
    assert_eq!(stats1.remaining_balance, amount_1);
    assert_eq!(stats2.remaining_balance, amount_2);
    assert_eq!(stats1.token_address, setup.token_a.address);
    assert_eq!(stats2.token_address, setup.token_b.address);

    // Payout from Program 1
    let r1 = Address::generate(&setup.env);
    setup.client.single_payout_v2(&prog1_id, &r1, &50_000);
    
    // Verify Program 1 balance decreased, Program 2 remained same
    let stats1_after = setup.client.get_program_info_v2(&prog1_id);
    let stats2_after = setup.client.get_program_info_v2(&prog2_id);
    
    assert_eq!(stats1_after.remaining_balance, 50_000);
    assert_eq!(stats2_after.remaining_balance, 200_000);
    
    assert_eq!(setup.token_a.balance(&r1), 50_000);
    assert_eq!(setup.token_b.balance(&r1), 0);
}
