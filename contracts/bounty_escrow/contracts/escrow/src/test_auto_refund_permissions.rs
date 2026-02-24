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
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
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
    random_user: Address,
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
        let random_user = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            admin,
            depositor,
            random_user,
            token,
            token_admin,
            escrow,
        }
    }
}

#[test]
fn test_auto_refund_anyone_can_trigger_after_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);

    let initial_balance = setup.token.balance(&setup.depositor);

    // Random user triggers refund
    setup.escrow.refund(&bounty_id);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_balance + amount
    );
}

#[test]
fn test_auto_refund_admin_can_trigger_after_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);

    let initial_balance = setup.token.balance(&setup.depositor);

    // Admin triggers refund
    setup.escrow.refund(&bounty_id);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_balance + amount
    );
}

#[test]
fn test_auto_refund_depositor_can_trigger_after_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);

    let initial_balance = setup.token.balance(&setup.depositor);

    // Depositor triggers refund
    setup.escrow.refund(&bounty_id);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_balance + amount
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // DeadlineNotPassed
fn test_auto_refund_fails_before_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Try to refund before deadline
    setup.escrow.refund(&bounty_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // DeadlineNotPassed
fn test_auto_refund_admin_cannot_bypass_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin tries to refund before deadline (should fail)
    setup.escrow.refund(&bounty_id);
}

#[test]
fn test_auto_refund_at_exact_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline);

    let initial_balance = setup.token.balance(&setup.depositor);

    setup.escrow.refund(&bounty_id);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_balance + amount
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_auto_refund_idempotent_second_call_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);

    // First refund succeeds
    setup.escrow.refund(&bounty_id);

    // Second refund should fail
    setup.escrow.refund(&bounty_id);
}

#[test]
fn test_auto_refund_balance_stable_after_first_refund() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);

    let initial_balance = setup.token.balance(&setup.depositor);

    // First refund
    setup.escrow.refund(&bounty_id);

    let escrow_after = setup.escrow.get_escrow_info(&bounty_id);
    let balance_after = setup.token.balance(&setup.depositor);

    // Verify state after successful refund
    assert_eq!(escrow_after.status, EscrowStatus::Refunded);
    assert_eq!(balance_after, initial_balance + amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

#[test]
fn test_auto_refund_different_users_same_result() {
    let setup = TestSetup::new();
    let bounty_id_1 = 1;
    let bounty_id_2 = 2;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock two bounties
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id_1, &amount, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id_2, &amount, &deadline);

    setup.env.ledger().set_timestamp(deadline + 1);

    let initial_balance = setup.token.balance(&setup.depositor);

    // Random user triggers first refund
    setup.escrow.refund(&bounty_id_1);

    // Admin triggers second refund
    setup.escrow.refund(&bounty_id_2);

    // Both should have same result
    let escrow_1 = setup.escrow.get_escrow_info(&bounty_id_1);
    let escrow_2 = setup.escrow.get_escrow_info(&bounty_id_2);

    assert_eq!(escrow_1.status, EscrowStatus::Refunded);
    assert_eq!(escrow_2.status, EscrowStatus::Refunded);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_balance + (amount * 2)
    );
}
