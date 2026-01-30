#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String, token};

// ============================================================================
// Test Helpers
// ============================================================================

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let stellar_asset = e.register_stellar_asset_contract_v2(admin.clone());
    let token_address = stellar_asset.address();
    (
        token_address.clone(),
        token::Client::new(e, &token_address),
        token::StellarAssetClient::new(e, &token_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> ProgramEscrowClient<'a> {
    let contract_id = e.register(ProgramEscrow, ());
    ProgramEscrowClient::new(e, &contract_id)
}

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    recipient1: Address,
    recipient2: Address,
    token: token::Client<'a>,
    token_address: Address,
    token_admin: token::StellarAssetClient<'a>,
    escrow: ProgramEscrowClient<'a>,
    program_id: String,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let recipient1 = Address::generate(&env);
        let recipient2 = Address::generate(&env);

        let (token_address, token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);
        let program_id = String::from_str(&env, "hackathon-2024");

        // Initialize the program
        escrow.initialize(&program_id, &admin, &token_address);

        // Mint tokens to depositor
        token_admin.mint(&depositor, &1_000_000_000_000);

        // Transfer tokens to escrow contract for payouts
        token.transfer(&depositor, &escrow.address, &500_000_000_000);

        Self {
            env,
            admin,
            depositor,
            recipient1,
            recipient2,
            token,
            token_address,
            token_admin,
            escrow,
            program_id,
        }
    }

    fn new_without_init() -> (Env, ProgramEscrowClient<'a>) {
        let env = Env::default();
        env.mock_all_auths();
        let escrow = create_escrow_contract(&env);
        (env, escrow)
    }
}

// ============================================================================
// TESTS FOR initialize()
// ============================================================================

#[test]
fn test_initialize_success() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let escrow = create_escrow_contract(&env);
    let program_id = String::from_str(&env, "hackathon-2024-q1");

    let program_data = escrow.initialize(&program_id, &admin, &token);

    assert_eq!(program_data.program_id, program_id);
    assert_eq!(program_data.total_funds, 0);
    assert_eq!(program_data.remaining_bal, 0);
    assert_eq!(program_data.auth_key, admin);
    assert_eq!(program_data.token_addr, token);
    assert_eq!(program_data.payout_history.len(), 0);
    assert_eq!(program_data.whitelist.len(), 1);
}

#[test]
#[should_panic(expected = "Program already initialized")]
fn test_initialize_duplicate() {
    let setup = TestSetup::new();

    // Try to initialize again
    let token2 = Address::generate(&setup.env);
    setup.escrow.initialize(&setup.program_id, &setup.admin, &token2);
}

// ============================================================================
// TESTS FOR lock_funds()
// ============================================================================

#[test]
fn test_lock_funds_success() {
    let setup = TestSetup::new();
    let amount = 50_000_000_000i128;

    let program_data = setup.escrow.lock_funds(&amount, &setup.token_address);

    assert_eq!(program_data.total_funds, amount);
    assert_eq!(program_data.remaining_bal, amount);
}

#[test]
fn test_lock_funds_multiple_times() {
    let setup = TestSetup::new();

    // First lock
    let program_data = setup.escrow.lock_funds(&25_000_000_000, &setup.token_address);
    assert_eq!(program_data.total_funds, 25_000_000_000);
    assert_eq!(program_data.remaining_bal, 25_000_000_000);

    // Second lock
    let program_data = setup.escrow.lock_funds(&35_000_000_000, &setup.token_address);
    assert_eq!(program_data.total_funds, 60_000_000_000);
    assert_eq!(program_data.remaining_bal, 60_000_000_000);
}

#[test]
fn test_lock_funds_balance_tracking() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 100_000_000_000);

    setup.escrow.lock_funds(&50_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 150_000_000_000);
}

#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_lock_funds_zero_amount() {
    let setup = TestSetup::new();
    setup.escrow.lock_funds(&0, &setup.token_address);
}

#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_lock_funds_negative_amount() {
    let setup = TestSetup::new();
    setup.escrow.lock_funds(&-1_000_000_000, &setup.token_address);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_lock_funds_before_init() {
    let (env, escrow) = TestSetup::new_without_init();
    let token = Address::generate(&env);
    escrow.lock_funds(&10_000_000_000, &token);
}

#[test]
#[should_panic(expected = "Token not whitelisted")]
fn test_lock_funds_non_whitelisted_token() {
    let setup = TestSetup::new();
    let non_whitelisted_token = Address::generate(&setup.env);
    setup.escrow.lock_funds(&10_000_000_000, &non_whitelisted_token);
}

// ============================================================================
// TESTS FOR single_payout()
// ============================================================================

#[test]
fn test_single_payout_success() {
    let setup = TestSetup::new();
    let lock_amount = 50_000_000_000i128;
    let payout_amount = 10_000_000_000i128;

    setup.escrow.lock_funds(&lock_amount, &setup.token_address);

    let program_data = setup.escrow.single_payout(&setup.recipient1, &payout_amount, &setup.token_address);

    assert_eq!(program_data.remaining_bal, lock_amount - payout_amount);
    assert_eq!(program_data.payout_history.len(), 1);

    let payout = program_data.payout_history.get(0).unwrap();
    assert_eq!(payout.recipient, setup.recipient1);
    assert_eq!(payout.amount, payout_amount);
    assert_eq!(payout.token, setup.token_address);
}

#[test]
fn test_single_payout_multiple_recipients() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    // First payout
    let program_data = setup.escrow.single_payout(&setup.recipient1, &20_000_000_000, &setup.token_address);
    assert_eq!(program_data.remaining_bal, 80_000_000_000);
    assert_eq!(program_data.payout_history.len(), 1);

    // Second payout
    let program_data = setup.escrow.single_payout(&setup.recipient2, &25_000_000_000, &setup.token_address);
    assert_eq!(program_data.remaining_bal, 55_000_000_000);
    assert_eq!(program_data.payout_history.len(), 2);
}

#[test]
fn test_single_payout_balance_updates() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 100_000_000_000);

    setup.escrow.single_payout(&setup.recipient1, &40_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 60_000_000_000);
}

#[test]
#[should_panic(expected = "Insufficient token balance")]
fn test_single_payout_insufficient_balance() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&20_000_000_000, &setup.token_address);
    setup.escrow.single_payout(&setup.recipient1, &30_000_000_000, &setup.token_address);
}

#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_single_payout_zero_amount() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&50_000_000_000, &setup.token_address);
    setup.escrow.single_payout(&setup.recipient1, &0, &setup.token_address);
}

#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_single_payout_negative_amount() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&50_000_000_000, &setup.token_address);
    setup.escrow.single_payout(&setup.recipient1, &-10_000_000_000, &setup.token_address);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_single_payout_before_init() {
    let (env, escrow) = TestSetup::new_without_init();
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);
    escrow.single_payout(&recipient, &10_000_000_000, &token);
}

#[test]
#[should_panic(expected = "Token not whitelisted")]
fn test_single_payout_non_whitelisted_token() {
    let setup = TestSetup::new();
    let non_whitelisted_token = Address::generate(&setup.env);

    setup.escrow.lock_funds(&50_000_000_000, &setup.token_address);
    setup.escrow.single_payout(&setup.recipient1, &10_000_000_000, &non_whitelisted_token);
}

// ============================================================================
// TESTS FOR batch_payout()
// ============================================================================

#[test]
fn test_batch_payout_success() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    let recipients = vec![&setup.env, setup.recipient1.clone(), setup.recipient2.clone()];
    let amounts = vec![&setup.env, 10_000_000_000i128, 20_000_000_000i128];

    let program_data = setup.escrow.batch_payout(&recipients, &amounts, &setup.token_address);

    assert_eq!(program_data.remaining_bal, 70_000_000_000); // 100 - 10 - 20
    assert_eq!(program_data.payout_history.len(), 2);

    let payout1 = program_data.payout_history.get(0).unwrap();
    assert_eq!(payout1.recipient, setup.recipient1);
    assert_eq!(payout1.amount, 10_000_000_000);

    let payout2 = program_data.payout_history.get(1).unwrap();
    assert_eq!(payout2.recipient, setup.recipient2);
    assert_eq!(payout2.amount, 20_000_000_000);
}

#[test]
fn test_batch_payout_single_recipient() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&50_000_000_000, &setup.token_address);

    let recipients = vec![&setup.env, setup.recipient1.clone()];
    let amounts = vec![&setup.env, 25_000_000_000i128];

    let program_data = setup.escrow.batch_payout(&recipients, &amounts, &setup.token_address);

    assert_eq!(program_data.remaining_bal, 25_000_000_000);
    assert_eq!(program_data.payout_history.len(), 1);
}

#[test]
#[should_panic(expected = "Insufficient token balance")]
fn test_batch_payout_insufficient_balance() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&50_000_000_000, &setup.token_address);

    let recipients = vec![&setup.env, setup.recipient1.clone(), setup.recipient2.clone()];
    let amounts = vec![&setup.env, 30_000_000_000i128, 25_000_000_000i128]; // Total: 55 > 50

    setup.escrow.batch_payout(&recipients, &amounts, &setup.token_address);
}

#[test]
#[should_panic(expected = "Vectors must have the same length")]
fn test_batch_payout_mismatched_lengths() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    let recipients = vec![&setup.env, setup.recipient1.clone(), setup.recipient2.clone()];
    let amounts = vec![&setup.env, 10_000_000_000i128]; // Mismatched length

    setup.escrow.batch_payout(&recipients, &amounts, &setup.token_address);
}

#[test]
#[should_panic(expected = "Cannot process empty batch")]
fn test_batch_payout_empty_batch() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    let recipients: Vec<Address> = vec![&setup.env];
    let amounts: Vec<i128> = vec![&setup.env];

    setup.escrow.batch_payout(&recipients, &amounts, &setup.token_address);
}

#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn test_batch_payout_zero_amount() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    let recipients = vec![&setup.env, setup.recipient1.clone(), setup.recipient2.clone()];
    let amounts = vec![&setup.env, 10_000_000_000i128, 0i128]; // Zero amount

    setup.escrow.batch_payout(&recipients, &amounts, &setup.token_address);
}

#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn test_batch_payout_negative_amount() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    let recipients = vec![&setup.env, setup.recipient1.clone(), setup.recipient2.clone()];
    let amounts = vec![&setup.env, 10_000_000_000i128, -5_000_000_000i128]; // Negative

    setup.escrow.batch_payout(&recipients, &amounts, &setup.token_address);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_batch_payout_before_init() {
    let (env, escrow) = TestSetup::new_without_init();
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);
    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 10_000_000_000i128];

    escrow.batch_payout(&recipients, &amounts, &token);
}

#[test]
#[should_panic(expected = "Token not whitelisted")]
fn test_batch_payout_non_whitelisted_token() {
    let setup = TestSetup::new();
    let non_whitelisted_token = Address::generate(&setup.env);

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    let recipients = vec![&setup.env, setup.recipient1.clone()];
    let amounts = vec![&setup.env, 10_000_000_000i128];

    setup.escrow.batch_payout(&recipients, &amounts, &non_whitelisted_token);
}

// ============================================================================
// TESTS FOR VIEW FUNCTIONS
// ============================================================================

#[test]
fn test_get_info_success() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&75_000_000_000, &setup.token_address);

    let info = setup.escrow.get_info();

    assert_eq!(info.program_id, setup.program_id);
    assert_eq!(info.total_funds, 75_000_000_000);
    assert_eq!(info.remaining_bal, 75_000_000_000);
    assert_eq!(info.auth_key, setup.admin);
    assert_eq!(info.token_addr, setup.token_address);
}

#[test]
fn test_get_info_after_payouts() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    setup.escrow.single_payout(&setup.recipient1, &25_000_000_000, &setup.token_address);
    setup.escrow.single_payout(&setup.recipient2, &35_000_000_000, &setup.token_address);

    let info = setup.escrow.get_info();

    assert_eq!(info.total_funds, 100_000_000_000);
    assert_eq!(info.remaining_bal, 40_000_000_000); // 100 - 25 - 35
    assert_eq!(info.payout_history.len(), 2);
}

#[test]
fn test_get_remaining_balance_success() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&50_000_000_000, &setup.token_address);

    assert_eq!(setup.escrow.get_remaining_balance(), 50_000_000_000);
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_get_info_before_init() {
    let (_, escrow) = TestSetup::new_without_init();
    escrow.get_info();
}

#[test]
#[should_panic(expected = "Program not initialized")]
fn test_get_remaining_balance_before_init() {
    let (_, escrow) = TestSetup::new_without_init();
    escrow.get_remaining_balance();
}

// ============================================================================
// TESTS FOR TOKEN WHITELIST
// ============================================================================

#[test]
fn test_add_token_success() {
    let setup = TestSetup::new();
    let new_token = Address::generate(&setup.env);

    let program = setup.escrow.add_token(&new_token);

    assert_eq!(program.whitelist.len(), 2);
    assert!(setup.escrow.is_whitelisted(&new_token));
}

#[test]
fn test_remove_token_success() {
    let setup = TestSetup::new();
    let new_token = Address::generate(&setup.env);

    setup.escrow.add_token(&new_token);
    assert!(setup.escrow.is_whitelisted(&new_token));

    setup.escrow.remove_token(&new_token);
    assert!(!setup.escrow.is_whitelisted(&new_token));

    // Original token should still be whitelisted
    assert!(setup.escrow.is_whitelisted(&setup.token_address));
}

#[test]
#[should_panic(expected = "Token already whitelisted")]
fn test_add_duplicate_token() {
    let setup = TestSetup::new();
    // Token is already whitelisted from init
    setup.escrow.add_token(&setup.token_address);
}

#[test]
#[should_panic(expected = "Cannot remove default token")]
fn test_remove_default_token() {
    let setup = TestSetup::new();
    setup.escrow.remove_token(&setup.token_address);
}

#[test]
#[should_panic(expected = "Token not whitelisted")]
fn test_remove_non_whitelisted_token() {
    let setup = TestSetup::new();
    let non_whitelisted_token = Address::generate(&setup.env);
    setup.escrow.remove_token(&non_whitelisted_token);
}

#[test]
fn test_get_tokens() {
    let setup = TestSetup::new();

    let tokens = setup.escrow.get_tokens();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens.get(0).unwrap(), setup.token_address);

    let new_token = Address::generate(&setup.env);
    setup.escrow.add_token(&new_token);

    let tokens = setup.escrow.get_tokens();
    assert_eq!(tokens.len(), 2);
}

#[test]
fn test_is_whitelisted() {
    let setup = TestSetup::new();

    assert!(setup.escrow.is_whitelisted(&setup.token_address));

    let non_whitelisted = Address::generate(&setup.env);
    assert!(!setup.escrow.is_whitelisted(&non_whitelisted));
}

// ============================================================================
// TESTS FOR TOKEN BALANCE
// ============================================================================

#[test]
fn test_get_balance() {
    let setup = TestSetup::new();

    let balance = setup.escrow.get_balance(&setup.token_address);
    assert_eq!(balance.locked, 0);
    assert_eq!(balance.remaining, 0);

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    let balance = setup.escrow.get_balance(&setup.token_address);
    assert_eq!(balance.locked, 100_000_000_000);
    assert_eq!(balance.remaining, 100_000_000_000);
}

#[test]
fn test_get_balance_after_payout() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);
    setup.escrow.single_payout(&setup.recipient1, &30_000_000_000, &setup.token_address);

    let balance = setup.escrow.get_balance(&setup.token_address);
    assert_eq!(balance.locked, 100_000_000_000);
    assert_eq!(balance.remaining, 70_000_000_000);
}

#[test]
#[should_panic(expected = "Token not whitelisted")]
fn test_get_balance_non_whitelisted() {
    let setup = TestSetup::new();
    let non_whitelisted = Address::generate(&setup.env);
    setup.escrow.get_balance(&non_whitelisted);
}

#[test]
fn test_get_all_balances() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    let balances = setup.escrow.get_all_balances();
    assert_eq!(balances.len(), 1);

    let (token, balance) = balances.get(0).unwrap();
    assert_eq!(token, setup.token_address);
    assert_eq!(balance.locked, 100_000_000_000);
    assert_eq!(balance.remaining, 100_000_000_000);
}

// ============================================================================
// MULTI-TOKEN TESTS
// ============================================================================

struct MultiTokenSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    recipient: Address,
    token1: token::Client<'a>,
    token1_address: Address,
    token2: token::Client<'a>,
    token2_address: Address,
    escrow: ProgramEscrowClient<'a>,
}

impl<'a> MultiTokenSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let recipient = Address::generate(&env);

        let (token1_address, token1, token1_admin) = create_token_contract(&env, &admin);
        let (token2_address, token2, token2_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);
        let program_id = String::from_str(&env, "multi-token-program");

        // Initialize with token1
        escrow.initialize(&program_id, &admin, &token1_address);

        // Add token2 to whitelist
        escrow.add_token(&token2_address);

        // Mint and transfer tokens to contract
        token1_admin.mint(&depositor, &1_000_000_000_000);
        token2_admin.mint(&depositor, &1_000_000_000_000);
        token1.transfer(&depositor, &escrow.address, &500_000_000_000);
        token2.transfer(&depositor, &escrow.address, &500_000_000_000);

        Self {
            env,
            admin,
            depositor,
            recipient,
            token1,
            token1_address,
            token2,
            token2_address,
            escrow,
        }
    }
}

#[test]
fn test_multi_token_lock_funds() {
    let setup = MultiTokenSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token1_address);
    setup.escrow.lock_funds(&200_000_000_000, &setup.token2_address);

    let balance1 = setup.escrow.get_balance(&setup.token1_address);
    assert_eq!(balance1.locked, 100_000_000_000);
    assert_eq!(balance1.remaining, 100_000_000_000);

    let balance2 = setup.escrow.get_balance(&setup.token2_address);
    assert_eq!(balance2.locked, 200_000_000_000);
    assert_eq!(balance2.remaining, 200_000_000_000);

    // Total funds should be sum of both
    let info = setup.escrow.get_info();
    assert_eq!(info.total_funds, 300_000_000_000);
}

#[test]
fn test_multi_token_payout() {
    let setup = MultiTokenSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token1_address);
    setup.escrow.lock_funds(&200_000_000_000, &setup.token2_address);

    // Payout from token1
    setup.escrow.single_payout(&setup.recipient, &50_000_000_000, &setup.token1_address);

    let balance1 = setup.escrow.get_balance(&setup.token1_address);
    assert_eq!(balance1.remaining, 50_000_000_000);

    // Token2 balance should be unchanged
    let balance2 = setup.escrow.get_balance(&setup.token2_address);
    assert_eq!(balance2.remaining, 200_000_000_000);

    // Payout from token2
    setup.escrow.single_payout(&setup.recipient, &75_000_000_000, &setup.token2_address);

    let balance2 = setup.escrow.get_balance(&setup.token2_address);
    assert_eq!(balance2.remaining, 125_000_000_000);
}

#[test]
fn test_multi_token_batch_payout() {
    let setup = MultiTokenSetup::new();
    let recipient2 = Address::generate(&setup.env);

    setup.escrow.lock_funds(&100_000_000_000, &setup.token1_address);
    setup.escrow.lock_funds(&200_000_000_000, &setup.token2_address);

    let recipients = vec![&setup.env, setup.recipient.clone(), recipient2.clone()];
    let amounts = vec![&setup.env, 30_000_000_000i128, 40_000_000_000i128];

    setup.escrow.batch_payout(&recipients, &amounts, &setup.token2_address);

    // Token2 should be reduced
    let balance2 = setup.escrow.get_balance(&setup.token2_address);
    assert_eq!(balance2.remaining, 130_000_000_000); // 200 - 30 - 40

    // Token1 should be unchanged
    let balance1 = setup.escrow.get_balance(&setup.token1_address);
    assert_eq!(balance1.remaining, 100_000_000_000);
}

#[test]
fn test_multi_token_payout_history() {
    let setup = MultiTokenSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token1_address);
    setup.escrow.lock_funds(&200_000_000_000, &setup.token2_address);

    setup.escrow.single_payout(&setup.recipient, &50_000_000_000, &setup.token1_address);
    setup.escrow.single_payout(&setup.recipient, &75_000_000_000, &setup.token2_address);

    let info = setup.escrow.get_info();
    assert_eq!(info.payout_history.len(), 2);

    let payout1 = info.payout_history.get(0).unwrap();
    assert_eq!(payout1.token, setup.token1_address);
    assert_eq!(payout1.amount, 50_000_000_000);

    let payout2 = info.payout_history.get(1).unwrap();
    assert_eq!(payout2.token, setup.token2_address);
    assert_eq!(payout2.amount, 75_000_000_000);
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_complete_program_lifecycle() {
    let setup = TestSetup::new();

    // 1. Verify initial state
    let info = setup.escrow.get_info();
    assert_eq!(info.total_funds, 0);
    assert_eq!(info.remaining_bal, 0);

    // 2. Lock initial funds
    setup.escrow.lock_funds(&500_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 500_000_000_000);

    // 3. Single payouts
    setup.escrow.single_payout(&setup.recipient1, &50_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 450_000_000_000);

    setup.escrow.single_payout(&setup.recipient2, &75_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 375_000_000_000);

    // 4. Batch payout
    let recipient3 = Address::generate(&setup.env);
    let recipient4 = Address::generate(&setup.env);
    let recipients = vec![&setup.env, recipient3, recipient4];
    let amounts = vec![&setup.env, 100_000_000_000i128, 80_000_000_000i128];
    setup.escrow.batch_payout(&recipients, &amounts, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 195_000_000_000);

    // 5. Verify final state
    let final_info = setup.escrow.get_info();
    assert_eq!(final_info.total_funds, 500_000_000_000);
    assert_eq!(final_info.remaining_bal, 195_000_000_000);
    assert_eq!(final_info.payout_history.len(), 4);

    // 6. Lock additional funds
    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 295_000_000_000);

    let updated_info = setup.escrow.get_info();
    assert_eq!(updated_info.total_funds, 600_000_000_000);
}

#[test]
fn test_program_with_zero_final_balance() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    setup.escrow.single_payout(&setup.recipient1, &60_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 40_000_000_000);

    setup.escrow.single_payout(&setup.recipient2, &40_000_000_000, &setup.token_address);
    assert_eq!(setup.escrow.get_remaining_balance(), 0);

    let info = setup.escrow.get_info();
    assert_eq!(info.total_funds, 100_000_000_000);
    assert_eq!(info.remaining_bal, 0);
    assert_eq!(info.payout_history.len(), 2);
}

#[test]
fn test_payout_record_integrity() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&200_000_000_000, &setup.token_address);

    // Mix of single and batch payouts
    setup.escrow.single_payout(&setup.recipient1, &25_000_000_000, &setup.token_address);

    let recipients = vec![&setup.env, setup.recipient2.clone()];
    let amounts = vec![&setup.env, 35_000_000_000i128];
    setup.escrow.batch_payout(&recipients, &amounts, &setup.token_address);

    // Same recipient again
    setup.escrow.single_payout(&setup.recipient1, &15_000_000_000, &setup.token_address);

    let info = setup.escrow.get_info();
    assert_eq!(info.payout_history.len(), 3);
    assert_eq!(info.remaining_bal, 125_000_000_000); // 200 - 25 - 35 - 15

    // Verify all records
    let records = info.payout_history;
    assert_eq!(records.get(0).unwrap().recipient, setup.recipient1);
    assert_eq!(records.get(0).unwrap().amount, 25_000_000_000);

    assert_eq!(records.get(1).unwrap().recipient, setup.recipient2);
    assert_eq!(records.get(1).unwrap().amount, 35_000_000_000);

    assert_eq!(records.get(2).unwrap().recipient, setup.recipient1);
    assert_eq!(records.get(2).unwrap().amount, 15_000_000_000);
}

#[test]
fn test_timestamp_tracking() {
    let setup = TestSetup::new();

    setup.escrow.lock_funds(&100_000_000_000, &setup.token_address);

    // First payout
    setup.escrow.single_payout(&setup.recipient1, &25_000_000_000, &setup.token_address);
    let first_timestamp = setup.env.ledger().timestamp();

    // Advance time
    setup.env.ledger().set_timestamp(first_timestamp + 3600); // +1 hour

    // Second payout
    setup.escrow.single_payout(&setup.recipient2, &30_000_000_000, &setup.token_address);

    let info = setup.escrow.get_info();
    let payout1 = info.payout_history.get(0).unwrap();
    let payout2 = info.payout_history.get(1).unwrap();

    // Second payout should have later timestamp
    assert!(payout2.timestamp > payout1.timestamp);
}
