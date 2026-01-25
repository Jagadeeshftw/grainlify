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
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);

        // Mint tokens to depositor
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }
}

#[test]
fn test_lock_funds_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock funds
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Verify stored escrow data
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.depositor, setup.depositor);
    assert_eq!(stored_escrow.amount, amount);
    assert_eq!(stored_escrow.status, EscrowStatus::Locked);
    assert_eq!(stored_escrow.deadline, deadline);

    // Verify contract balance
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // BountyExists
fn test_lock_funds_duplicate() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Try to lock again with same bounty_id
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

#[test]
#[should_panic] // Token transfer fail
fn test_lock_funds_negative_amount() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = -100;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

#[test]
fn test_get_escrow_info() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.deadline, deadline);
    assert_eq!(escrow.depositor, setup.depositor);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_release_funds_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Verify initial balances
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
    assert_eq!(setup.token.balance(&setup.contributor), 0);

    // Release funds
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Verify updated state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Released);

    // Verify balances after release
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_release_funds_already_released() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Try to release again
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_release_funds_not_found() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

#[test]
fn test_refund_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Advance time past deadline
    setup.env.ledger().set_timestamp(deadline + 1);

    // Initial value
    let initial_depositor_balance = setup.token.balance(&setup.depositor);

    // Refund
    setup.escrow.refund(&bounty_id);

    // Verify state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Refunded);

    // Verify balances
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_depositor_balance + amount
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // DeadlineNotPassed
fn test_refund_too_early() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Attempt refund before deadline
    setup.escrow.refund(&bounty_id);
}

#[test]
fn test_get_balance() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 500;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Initial balance should be 0
    assert_eq!(setup.escrow.get_balance(), 0);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Balance should be updated
    assert_eq!(setup.escrow.get_balance(), amount);
}
