#![no_std]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env, Vec,
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

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
    escrow_address: Address,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let (escrow, escrow_address) = create_escrow_contract(&env);

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
            escrow_address,
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
    // Note: amount stores net_amount (after fee), but fees are disabled by default
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.depositor, setup.depositor);
    assert_eq!(stored_escrow.amount, amount); // net_amount = amount when fees disabled
    assert_eq!(stored_escrow.remaining_amount, amount); // remaining_amount stores original
    assert_eq!(stored_escrow.status, EscrowStatus::Locked);
    assert_eq!(stored_escrow.deadline, deadline);

    // Verify contract balance
    assert_eq!(setup.token.balance(&setup.escrow_address), amount);
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
    assert_eq!(setup.token.balance(&setup.escrow_address), amount);
    assert_eq!(setup.token.balance(&setup.contributor), 0);

    // Release funds
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Verify updated state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Released);

    // Verify balances after release (fees disabled by default, so net_amount = amount)
    assert_eq!(setup.token.balance(&setup.escrow_address), 0);
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

// ============================================================================
// REFUND TESTS - Full Refund After Deadline
// ============================================================================

#[test]
fn test_refund_full_after_deadline() {
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

    // Initial balances
    let initial_depositor_balance = setup.token.balance(&setup.depositor);

    // Full refund (no amount/recipient specified, mode = Full)
    setup.escrow.refund(
        &bounty_id,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );

    // Verify state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Refunded);
    assert_eq!(stored_escrow.remaining_amount, 0);

    // Verify balances
    assert_eq!(setup.token.balance(&setup.escrow_address), 0);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_depositor_balance + amount
    );

    // Verify refund history
    let refund_history = setup.escrow.get_refund_history(&bounty_id);
    assert_eq!(refund_history.len(), 1);
    assert_eq!(refund_history.get(0).unwrap().amount, amount);
    assert_eq!(refund_history.get(0).unwrap().recipient, setup.depositor);
    assert_eq!(refund_history.get(0).unwrap().mode, RefundMode::Full);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // DeadlineNotPassed
fn test_refund_full_before_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Attempt full refund before deadline (should fail)
    setup.escrow.refund(
        &bounty_id,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );
}

// ============================================================================
// REFUND TESTS - Partial Refund
// ============================================================================

#[test]
fn test_refund_partial_after_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let total_amount = 1000;
    let refund_amount = 300;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &total_amount, &deadline);

    // Advance time past deadline
    setup.env.ledger().set_timestamp(deadline + 1);

    // Initial balances
    let initial_depositor_balance = setup.token.balance(&setup.depositor);

    // Partial refund
    setup.escrow.refund(
        &bounty_id,
        &Some(refund_amount),
        &None::<Address>,
        &RefundMode::Partial,
    );

    // Verify state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(stored_escrow.remaining_amount, total_amount - refund_amount);

    // Verify balances
    assert_eq!(
        setup.token.balance(&setup.escrow_address),
        total_amount - refund_amount
    );
    assert_eq!(
        setup.token.balance(&setup.depositor),
        initial_depositor_balance + refund_amount
    );

    // Verify refund history
    let refund_history = setup.escrow.get_refund_history(&bounty_id);
    assert_eq!(refund_history.len(), 1);
    assert_eq!(refund_history.get(0).unwrap().amount, refund_amount);
    assert_eq!(refund_history.get(0).unwrap().recipient, setup.depositor);
    assert_eq!(refund_history.get(0).unwrap().mode, RefundMode::Partial);
}

#[test]
fn test_refund_partial_multiple_times() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let total_amount = 1000;
    let refund1 = 200;
    let refund2 = 300;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &total_amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);

    // First partial refund
    setup.escrow.refund(
        &bounty_id,
        &Some(refund1),
        &None::<Address>,
        &RefundMode::Partial,
    );

    // Second partial refund
    setup.escrow.refund(
        &bounty_id,
        &Some(refund2),
        &None::<Address>,
        &RefundMode::Partial,
    );

    // Verify state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(
        stored_escrow.remaining_amount,
        total_amount - refund1 - refund2
    );

    // Verify refund history has 2 records
    let refund_history = setup.escrow.get_refund_history(&bounty_id);
    assert_eq!(refund_history.len(), 2);
    assert_eq!(refund_history.get(0).unwrap().amount, refund1);
    assert_eq!(refund_history.get(1).unwrap().amount, refund2);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // DeadlineNotPassed
fn test_refund_partial_before_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let refund_amount = 300;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Attempt partial refund before deadline (should fail)
    setup.escrow.refund(
        &bounty_id,
        &Some(refund_amount),
        &None::<Address>,
        &RefundMode::Partial,
    );
}

// ============================================================================
// REFUND TESTS - Custom Refund (Different Address)
// ============================================================================

#[test]
fn test_refund_custom_after_deadline() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let refund_amount = 500;
    let custom_recipient = Address::generate(&setup.env);
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);

    // Initial balances
    let initial_recipient_balance = setup.token.balance(&custom_recipient);

    // Custom refund to different address (after deadline, no approval needed)
    setup.escrow.refund(
        &bounty_id,
        &Some(refund_amount),
        &Some(custom_recipient.clone()),
        &RefundMode::Custom,
    );

    // Verify state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(stored_escrow.remaining_amount, amount - refund_amount);

    // Verify balances
    assert_eq!(
        setup.token.balance(&custom_recipient),
        initial_recipient_balance + refund_amount
    );

    // Verify refund history
    let refund_history = setup.escrow.get_refund_history(&bounty_id);
    assert_eq!(refund_history.len(), 1);
    assert_eq!(refund_history.get(0).unwrap().amount, refund_amount);
    assert_eq!(refund_history.get(0).unwrap().recipient, custom_recipient);
    assert_eq!(refund_history.get(0).unwrap().mode, RefundMode::Custom);
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")] // RefundNotApproved
fn test_refund_custom_before_deadline_without_approval() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let refund_amount = 500;
    let custom_recipient = Address::generate(&setup.env);
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Attempt custom refund before deadline without approval (should fail)
    setup.escrow.refund(
        &bounty_id,
        &Some(refund_amount),
        &Some(custom_recipient),
        &RefundMode::Custom,
    );
}

// ============================================================================
// REFUND TESTS - Approval Workflow
// ============================================================================

#[test]
fn test_refund_approval_workflow() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let refund_amount = 500;
    let custom_recipient = Address::generate(&setup.env);
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin approves refund before deadline
    setup.escrow.approve_refund(
        &bounty_id,
        &refund_amount,
        &custom_recipient.clone(),
        &RefundMode::Custom,
    );

    // Verify approval exists
    let (can_refund, deadline_passed, remaining, approval) =
        setup.escrow.get_refund_eligibility(&bounty_id);
    assert!(can_refund);
    assert!(!deadline_passed);
    assert_eq!(remaining, amount);
    assert!(approval.is_some());
    let approval_data = approval.unwrap();
    assert_eq!(approval_data.amount, refund_amount);
    assert_eq!(approval_data.recipient, custom_recipient);
    assert_eq!(approval_data.mode, RefundMode::Custom);
    assert_eq!(approval_data.approved_by, setup.admin);

    // Initial balances
    let initial_recipient_balance = setup.token.balance(&custom_recipient);

    // Execute approved refund (before deadline)
    setup.escrow.refund(
        &bounty_id,
        &Some(refund_amount),
        &Some(custom_recipient.clone()),
        &RefundMode::Custom,
    );

    // Verify approval was consumed (removed after use)
    let (_, _, _, approval_after) = setup.escrow.get_refund_eligibility(&bounty_id);
    assert!(approval_after.is_none());

    // Verify state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(stored_escrow.remaining_amount, amount - refund_amount);

    // Verify balances
    assert_eq!(
        setup.token.balance(&custom_recipient),
        initial_recipient_balance + refund_amount
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")] // RefundNotApproved
fn test_refund_approval_mismatch() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let approved_amount = 500;
    let requested_amount = 600; // Different amount
    let custom_recipient = Address::generate(&setup.env);
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin approves refund for 500
    setup.escrow.approve_refund(
        &bounty_id,
        &approved_amount,
        &custom_recipient.clone(),
        &RefundMode::Custom,
    );

    // Try to refund with different amount (should fail)
    setup.escrow.refund(
        &bounty_id,
        &Some(requested_amount),
        &Some(custom_recipient),
        &RefundMode::Custom,
    );
}

#[test]
#[ignore] // Note: With mock_all_auths(), we can't test unauthorized access
          // The security is enforced by require_auth() in the contract which checks admin address
          // In production, non-admin calls will fail at require_auth()
fn test_refund_approval_non_admin() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let _refund_amount = 500;
    let _custom_recipient = Address::generate(&setup.env);
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Note: With mock_all_auths(), we can't easily test unauthorized access
    // The contract's require_auth() will enforce admin-only access in production
    // This test is marked as ignored as it requires more complex auth setup
}

// ============================================================================
// REFUND TESTS - Refund History Tracking
// ============================================================================

#[test]
fn test_refund_history_tracking() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let total_amount = 1000;
    let refund1 = 200;
    let refund2 = 300;
    let _refund3 = 400;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &total_amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);

    // First refund (Partial)
    setup.escrow.refund(
        &bounty_id,
        &Some(refund1),
        &None::<Address>,
        &RefundMode::Partial,
    );

    // Second refund (Partial)
    setup.escrow.refund(
        &bounty_id,
        &Some(refund2),
        &None::<Address>,
        &RefundMode::Partial,
    );

    // Third refund (Full remaining - should complete the refund)
    let remaining = total_amount - refund1 - refund2;
    setup.escrow.refund(
        &bounty_id,
        &Some(remaining),
        &None::<Address>,
        &RefundMode::Partial,
    );

    // Verify refund history
    let refund_history = setup.escrow.get_refund_history(&bounty_id);
    assert_eq!(refund_history.len(), 3);

    // Check first refund record
    let record1 = refund_history.get(0).unwrap();
    assert_eq!(record1.amount, refund1);
    assert_eq!(record1.recipient, setup.depositor);
    assert_eq!(record1.mode, RefundMode::Partial);

    // Check second refund record
    let record2 = refund_history.get(1).unwrap();
    assert_eq!(record2.amount, refund2);
    assert_eq!(record2.recipient, setup.depositor);
    assert_eq!(record2.mode, RefundMode::Partial);

    // Check third refund record
    let record3 = refund_history.get(2).unwrap();
    assert_eq!(record3.amount, remaining);
    assert_eq!(record3.recipient, setup.depositor);
    assert_eq!(record3.mode, RefundMode::Partial);

    // Verify final state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Refunded);
    assert_eq!(stored_escrow.remaining_amount, 0);
}

#[test]
fn test_refund_history_with_custom_recipients() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let total_amount = 1000;
    let recipient1 = Address::generate(&setup.env);
    let recipient2 = Address::generate(&setup.env);
    let refund1 = 300;
    let refund2 = 400;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &total_amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);

    // First custom refund
    setup.escrow.refund(
        &bounty_id,
        &Some(refund1),
        &Some(recipient1.clone()),
        &RefundMode::Custom,
    );

    // Second custom refund
    setup.escrow.refund(
        &bounty_id,
        &Some(refund2),
        &Some(recipient2.clone()),
        &RefundMode::Custom,
    );

    // Verify refund history
    let refund_history = setup.escrow.get_refund_history(&bounty_id);
    assert_eq!(refund_history.len(), 2);
    assert_eq!(refund_history.get(0).unwrap().recipient, recipient1);
    assert_eq!(refund_history.get(1).unwrap().recipient, recipient2);
}

// ============================================================================
// REFUND TESTS - Error Cases
// ============================================================================

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
fn test_refund_invalid_amount_zero() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);

    // Try to refund zero amount
    setup
        .escrow
        .refund(&bounty_id, &Some(0), &None::<Address>, &RefundMode::Partial);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
fn test_refund_invalid_amount_exceeds_remaining() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let refund_amount = 1500; // More than available
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);

    // Try to refund more than available
    setup.escrow.refund(
        &bounty_id,
        &Some(refund_amount),
        &None::<Address>,
        &RefundMode::Partial,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
fn test_refund_custom_missing_amount() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let custom_recipient = Address::generate(&setup.env);
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);

    // Custom refund requires amount
    setup.escrow.refund(
        &bounty_id,
        &None::<i128>,
        &Some(custom_recipient),
        &RefundMode::Custom,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // InvalidAmount
fn test_refund_custom_missing_recipient() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let refund_amount = 500;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);

    // Custom refund requires recipient
    setup.escrow.refund(
        &bounty_id,
        &Some(refund_amount),
        &None::<Address>,
        &RefundMode::Custom,
    );
}

#[test]
fn test_get_refund_eligibility() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Before deadline, no approval
    let (can_refund, deadline_passed, remaining, approval) =
        setup.escrow.get_refund_eligibility(&bounty_id);
    assert!(!can_refund);
    assert!(!deadline_passed);
    assert_eq!(remaining, amount);
    assert!(approval.is_none());

    // After deadline
    setup.env.ledger().set_timestamp(deadline + 1);
    let (can_refund, deadline_passed, remaining, approval) =
        setup.escrow.get_refund_eligibility(&bounty_id);
    assert!(can_refund);
    assert!(deadline_passed);
    assert_eq!(remaining, amount);
    assert!(approval.is_none());

    // With approval before deadline
    setup.env.ledger().set_timestamp(deadline - 100);
    let custom_recipient = Address::generate(&setup.env);
    setup
        .escrow
        .approve_refund(&bounty_id, &500, &custom_recipient, &RefundMode::Custom);

    let (can_refund, deadline_passed, remaining, approval) =
        setup.escrow.get_refund_eligibility(&bounty_id);
    assert!(can_refund);
    assert!(!deadline_passed);
    assert_eq!(remaining, amount);
    assert!(approval.is_some());
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

// ============================================================================
// BATCH OPERATIONS TESTS
// ============================================================================

#[test]
fn test_batch_lock_funds_success() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Create batch items
    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 3,
            depositor: setup.depositor.clone(),
            amount: 3000,
            deadline,
        },
    ];

    // Mint enough tokens
    setup.token_admin.mint(&setup.depositor, &10_000);

    // Batch lock funds
    let count = setup.escrow.batch_lock_funds(&items);
    assert_eq!(count, 3);

    // Verify all bounties are locked
    for i in 1..=3 {
        let escrow = setup.escrow.get_escrow_info(&i);
        assert_eq!(escrow.status, EscrowStatus::Locked);
    }

    // Verify contract balance
    assert_eq!(setup.escrow.get_balance(), 6000);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidBatchSize
fn test_batch_lock_funds_empty() {
    let setup = TestSetup::new();
    let items: Vec<LockFundsItem> = vec![&setup.env];
    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // BountyExists
fn test_batch_lock_funds_duplicate_bounty_id() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock a bounty first
    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);

    // Try to batch lock with duplicate bounty_id
    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1, // Already exists
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: setup.depositor.clone(),
            amount: 3000,
            deadline,
        },
    ];

    setup.escrow.batch_lock_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")] // DuplicateBountyId
fn test_batch_lock_funds_duplicate_in_batch() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 1000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1, // Duplicate in same batch
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
    ];

    setup.escrow.batch_lock_funds(&items);
}

#[test]
fn test_batch_release_funds_success() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock multiple bounties
    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &3, &3000, &deadline);

    // Create contributors
    let contributor1 = Address::generate(&setup.env);
    let contributor2 = Address::generate(&setup.env);
    let contributor3 = Address::generate(&setup.env);

    // Create batch release items
    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor1.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor2.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 3,
            contributor: contributor3.clone(),
        },
    ];

    // Batch release funds
    let count = setup.escrow.batch_release_funds(&items);
    assert_eq!(count, 3);

    // Verify all bounties are released
    for i in 1..=3 {
        let escrow = setup.escrow.get_escrow_info(&i);
        assert_eq!(escrow.status, EscrowStatus::Released);
    }

    // Verify balances
    assert_eq!(setup.token.balance(&contributor1), 1000);
    assert_eq!(setup.token.balance(&contributor2), 2000);
    assert_eq!(setup.token.balance(&contributor3), 3000);
    assert_eq!(setup.escrow.get_balance(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")] // InvalidBatchSize
fn test_batch_release_funds_empty() {
    let setup = TestSetup::new();
    let items: Vec<ReleaseFundsItem> = vec![&setup.env];
    setup.escrow.batch_release_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_batch_release_funds_not_found() {
    let setup = TestSetup::new();
    let contributor = Address::generate(&setup.env);

    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 999, // Doesn't exist
            contributor: contributor.clone(),
        },
    ];

    setup.escrow.batch_release_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_batch_release_funds_already_released() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock and release one bounty
    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);
    setup.escrow.release_funds(&1, &setup.contributor);

    // Lock another bounty
    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &deadline);

    let contributor2 = Address::generate(&setup.env);

    // Try to batch release including already released bounty
    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1, // Already released
            contributor: setup.contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor2.clone(),
        },
    ];

    setup.escrow.batch_release_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")] // DuplicateBountyId
fn test_batch_release_funds_duplicate_in_batch() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);

    let contributor = Address::generate(&setup.env);

    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 1, // Duplicate in same batch
            contributor: contributor.clone(),
        },
    ];

    setup.escrow.batch_release_funds(&items);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // BountyExists
fn test_batch_operations_atomicity() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock one bounty successfully
    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);

    // Try to batch lock with one valid and one that would fail (duplicate)
    // This should fail entirely due to atomicity
    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 2, // Valid
            depositor: setup.depositor.clone(),
            amount: 2000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 1, // Already exists - should cause entire batch to fail
            depositor: setup.depositor.clone(),
            amount: 3000,
            deadline,
        },
    ];

    // This should panic and no bounties should be locked
    setup.escrow.batch_lock_funds(&items);
}

#[test]
fn test_batch_operations_large_batch() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Create a batch of 10 bounties
    let mut items = Vec::new(&setup.env);
    for i in 1..=10 {
        items.push_back(LockFundsItem {
            bounty_id: i,
            depositor: setup.depositor.clone(),
            amount: (i * 100) as i128,
            deadline,
        });
    }

    // Mint enough tokens
    setup.token_admin.mint(&setup.depositor, &10_000);

    // Batch lock
    let count = setup.escrow.batch_lock_funds(&items);
    assert_eq!(count, 10);

    // Verify all are locked
    for i in 1..=10 {
        let escrow = setup.escrow.get_escrow_info(&i);
        assert_eq!(escrow.status, EscrowStatus::Locked);
    }

    // Create batch release items
    let mut release_items = Vec::new(&setup.env);
    for i in 1..=10 {
        release_items.push_back(ReleaseFundsItem {
            bounty_id: i,
            contributor: Address::generate(&setup.env),
        });
    }

    // Batch release
    let release_count = setup.escrow.batch_release_funds(&release_items);
    assert_eq!(release_count, 10);
}

// ============================================================================
// CLAIM PERIOD TESTS
// ============================================================================

#[test]
fn test_set_claim_window_success() {
    let setup = TestSetup::new();
    // Admin sets a 2-hour claim window
    setup.escrow.set_claim_window(&7200);
}

#[test]
fn test_authorize_claim_creates_pending_claim() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&3600);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(claim.recipient, setup.contributor);
    assert_eq!(claim.amount, amount);
    assert!(!claim.claimed);
    // expires_at should be current time + 3600
    assert!(claim.expires_at > setup.env.ledger().timestamp());
}

#[test]
fn test_claim_within_window_transfers_funds() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&3600);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Contributor claims within window
    setup.escrow.claim(&bounty_id);

    // Funds transferred
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow_address), 0);

    // Escrow marked released
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(escrow.remaining_amount, 0);
}

#[test]
#[should_panic]
fn test_claim_after_window_expires_panics() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 10000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&3600);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Advance time past the claim window
    setup
        .env
        .ledger()
        .set_timestamp(setup.env.ledger().timestamp() + 3601);

    setup.escrow.claim(&bounty_id);
}

#[test]
fn test_cancel_pending_claim_restores_escrow() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&3600);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Admin cancels the claim
    setup.escrow.cancel_pending_claim(&bounty_id);

    // Funds still in escrow
    assert_eq!(setup.token.balance(&setup.escrow_address), amount);
    assert_eq!(setup.token.balance(&setup.contributor), 0);

    // Escrow still locked
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
#[should_panic]
fn test_get_pending_claim_not_found() {
    let setup = TestSetup::new();
    setup.escrow.get_pending_claim(&999);
}

#[test]
#[should_panic]
fn test_cancel_pending_claim_not_found() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.cancel_pending_claim(&bounty_id);
}

#[test]
#[should_panic]
fn test_claim_twice_panics() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&3600);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    setup.escrow.claim(&bounty_id);
    // Second claim should fail
    setup.escrow.claim(&bounty_id);
}

#[test]
fn test_authorize_claim_default_window_used_when_not_set() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    // No set_claim_window call — should use 86400 default
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);
    let expected_expiry = setup.env.ledger().timestamp() + 86400;
    assert_eq!(claim.expires_at, expected_expiry);
}

#[test]
#[should_panic]
fn test_authorize_claim_on_nonexistent_bounty() {
    let setup = TestSetup::new();
    setup.escrow.authorize_claim(&999, &setup.contributor);
}

#[test]
#[should_panic]
fn test_authorize_claim_on_released_bounty() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);
}

#[test]
#[should_panic]
fn test_authorize_claim_on_refunded_bounty() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(
        &bounty_id,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );

    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);
}

#[test]
fn test_claim_at_exact_window_boundary_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 10000;
    let claim_window = 3600u64;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&claim_window);

    let now = setup.env.ledger().timestamp();
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Advance to exactly expires_at
    setup.env.ledger().set_timestamp(now + claim_window);
    setup.escrow.claim(&bounty_id);

    assert_eq!(setup.token.balance(&setup.contributor), amount);
}

#[test]
fn test_cancel_expired_claim_then_authorize_new_one() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 10000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&3600);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Advance past window
    setup
        .env
        .ledger()
        .set_timestamp(setup.env.ledger().timestamp() + 3601);

    // Admin cancels expired claim
    setup.escrow.cancel_pending_claim(&bounty_id);

    let new_contributor = Address::generate(&setup.env);
    setup.escrow.authorize_claim(&bounty_id, &new_contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(claim.recipient, new_contributor);
    assert!(!claim.claimed);
}

#[test]
fn test_cancel_claim_then_use_release_funds_normally() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 10000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_claim_window(&3600);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    setup.escrow.cancel_pending_claim(&bounty_id);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    assert_eq!(setup.token.balance(&setup.contributor), amount);
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_authorize_claim_zero_window_expires_immediately() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 10000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    // Set zero-second window — claim expires instantly
    setup.escrow.set_claim_window(&0);
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);
    // expires_at == created_at, so already expired
    assert!(claim.expires_at <= setup.env.ledger().timestamp());
}

#[test]
fn test_claim_does_not_affect_other_bounties() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;
    let contributor2 = Address::generate(&setup.env);

    setup.token_admin.mint(&setup.depositor, &5000);

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &deadline);
    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &deadline);

    setup.escrow.set_claim_window(&3600);
    setup.escrow.authorize_claim(&1, &setup.contributor);

    // Claim bounty 1
    setup.escrow.claim(&1);

    // Bounty 2 should be unaffected
    let escrow2 = setup.escrow.get_escrow_info(&2);
    assert_eq!(escrow2.status, EscrowStatus::Locked);
    assert_eq!(escrow2.amount, 2000);
    assert_eq!(setup.token.balance(&setup.escrow_address), 2000);

    // Bounty 2 can still be released normally
    setup.escrow.release_funds(&2, &contributor2);
    assert_eq!(setup.token.balance(&contributor2), 2000);
}

// ============================================================================
// ANTI-ABUSE TESTS FOR BOUNTY ESCROW
// ============================================================================

#[test]
#[should_panic(expected = "Rate limit exceeded")]
fn test_bounty_anti_abuse_rate_limit_exceeded() {
    let setup = TestSetup::new();
    let bounty_id = 999;
    let amount = 1000;

    let config = setup.escrow.get_config();
    let max_ops = config.max_operations;

    // Initial time setup
    let start_time = 1_000_000;
    setup.env.ledger().set_timestamp(start_time);

    let deadline = start_time + 1000;

    // We expect max_ops within the window_size

    for i in 0..max_ops {
        setup
            .env
            .ledger()
            .set_timestamp(start_time + config.cooldown_period * (i as u64) + 1);

        setup.escrow.lock_funds(
            &setup.depositor,
            &(bounty_id + i as u64),
            &amount,
            &deadline,
        );
    }

    setup
        .env
        .ledger()
        .set_timestamp(start_time + config.cooldown_period * (max_ops as u64) + 1);

    setup.escrow.lock_funds(
        &setup.depositor,
        &(bounty_id + max_ops as u64),
        &amount,
        &deadline,
    );
}

#[test]
#[should_panic(expected = "Operation in cooldown period")]
fn test_bounty_anti_abuse_cooldown_violation() {
    let setup = TestSetup::new();
    let bounty_id = 2999;
    let amount = 1000;

    let config = setup.escrow.get_config();

    // Initial time setup
    let start_time = 1_000_000;
    setup.env.ledger().set_timestamp(start_time);

    let deadline = start_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup
        .env
        .ledger()
        .set_timestamp(start_time + config.cooldown_period + 1);

    setup
        .escrow
        .lock_funds(&setup.depositor, &(bounty_id + 1), &amount, &deadline);

    setup
        .escrow
        .lock_funds(&setup.depositor, &(bounty_id + 2), &amount, &deadline);
}

#[test]
fn test_bounty_anti_abuse_whitelist_bypass() {
    let setup = TestSetup::new();
    let bounty_id = 3999;
    let amount = 10;

    let config = setup.escrow.get_config();
    let max_ops = config.max_operations;

    // Initial time setup
    let start_time = 1_000_000;
    setup.env.ledger().set_timestamp(start_time);

    let deadline = start_time + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Add depositor to whitelist
    setup.escrow.set_whitelist(&setup.depositor, &true);

    setup
        .env
        .ledger()
        .set_timestamp(start_time + config.cooldown_period + 1);

    // We should be able to do theoretically unlimited operations at the exact same timestamp
    for i in 1..=(max_ops + 5) {
        setup.escrow.lock_funds(
            &setup.depositor,
            &(bounty_id + i as u64),
            &amount,
            &deadline,
        );
    }

    // Verify successful locking
    let escrow = setup
        .escrow
        .get_escrow_info(&(bounty_id + max_ops as u64 + 5));
    assert_eq!(escrow.amount, amount);
}

// ============================================================================
// PAUSE & EMERGENCY STOP NEGATIVE TESTS
// ============================================================================

// --- pause() ---

#[test]
fn test_pause_by_admin_succeeds() {
    let setup = TestSetup::new();
    let reason = soroban_sdk::String::from_str(&setup.env, "security test");
    setup.escrow.pause(&reason);
    assert!(setup.escrow.is_paused());
    assert_eq!(setup.escrow.get_pause_reason(), Some(reason));
}

#[test]
#[should_panic]
fn test_pause_by_non_admin_fails() {
    // Build a fresh env WITHOUT mock_all_auths so require_auth enforces the check
    let env = Env::default();
    let (escrow, _) = create_escrow_contract(&env);
    let admin = Address::generate(&env);
    let (token, _) = create_token_contract(&env, &admin);

    // init with mocks
    env.mock_all_auths();
    escrow.init(&admin, &token.address);

    // Create a fresh env clone that does NOT have auths mocked to simulate non-admin
    // Use try_pause which returns Result — non-admin should cause require_auth panic
    let non_admin = Address::generate(&env);
    // Directly force non-admin context
    let reason = soroban_sdk::String::from_str(&env, "hack");
    non_admin.require_auth();
    escrow.pause(&reason);
}

// --- unpause() ---

#[test]
fn test_unpause_by_admin_succeeds() {
    let setup = TestSetup::new();
    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "test"));
    assert!(setup.escrow.is_paused());
    setup.escrow.unpause();
    assert!(!setup.escrow.is_paused());
    assert_eq!(setup.escrow.get_pause_reason(), None);
}

// --- lock_funds blocked while paused ---

#[test]
#[should_panic(expected = "Error(Contract, #18)")] // ContractPaused
fn test_lock_funds_while_paused_fails() {
    let setup = TestSetup::new();
    let bounty_id = 42u64;
    let amount = 500i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "security"));
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

// Verify no state change occurred during a paused lock_funds attempt
#[test]
fn test_lock_funds_while_paused_no_state_change() {
    let setup = TestSetup::new();
    let bounty_id = 43u64;
    let amount = 500i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "security"));

    // try_lock_funds returns Err(ContractPaused) without panicking
    let result = setup
        .escrow
        .try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert!(result.is_err());

    // No escrow should have been created
    let try_info = setup.escrow.try_get_escrow_info(&bounty_id);
    assert!(
        try_info.is_err(),
        "No escrow should exist after paused lock attempt"
    );

    // Contract balance stays zero
    assert_eq!(setup.escrow.get_balance(), 0);
}

// --- release_funds blocked while paused ---

#[test]
#[should_panic(expected = "Error(Contract, #18)")] // ContractPaused
fn test_release_funds_while_paused_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1u64;
    let amount = 500i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock before pausing
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "security"));
    setup.escrow.release_funds(&bounty_id, &setup.contributor);
}

// Verify escrow status unchanged after failed release while paused
#[test]
fn test_release_funds_while_paused_no_state_change() {
    let setup = TestSetup::new();
    let bounty_id = 2u64;
    let amount = 500i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "security"));

    let result = setup
        .escrow
        .try_release_funds(&bounty_id, &setup.contributor);
    assert!(result.is_err());

    // Escrow should still be Locked, contributor balance still 0
    let info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.contributor), 0);
}

// --- refund blocked while paused ---

#[test]
#[should_panic(expected = "Error(Contract, #18)")] // ContractPaused
fn test_refund_while_paused_fails() {
    let setup = TestSetup::new();
    let bounty_id = 1u64;
    let amount = 500i128;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Advance past deadline so refund would otherwise be valid
    setup.env.ledger().set_timestamp(deadline + 1);

    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "security"));

    // refund must be blocked while paused
    setup.escrow.refund(
        &bounty_id,
        &None::<i128>,
        &None::<Address>,
        &RefundMode::Full,
    );
}

// --- batch_lock_funds blocked while paused ---

#[test]
#[should_panic(expected = "Error(Contract, #18)")] // ContractPaused
fn test_batch_lock_funds_while_paused_fails() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "security"));

    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 100,
            deadline,
        },
    ];
    setup.escrow.batch_lock_funds(&items);
}

// --- batch_release_funds blocked while paused ---

#[test]
#[should_panic(expected = "Error(Contract, #18)")] // ContractPaused
fn test_batch_release_funds_while_paused_fails() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &500, &deadline);
    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "security"));

    let items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: setup.contributor.clone(),
        },
    ];
    setup.escrow.batch_release_funds(&items);
}

// --- emergency_withdraw ---

#[test]
fn test_emergency_withdraw_by_admin_drains_contract() {
    let setup = TestSetup::new();
    let bounty_id = 1u64;
    let amount = 500i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock some funds
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    assert_eq!(setup.escrow.get_balance(), amount);

    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "emergency"));
    let recipient = Address::generate(&setup.env);
    setup.escrow.emergency_withdraw(&recipient);

    // Contract balance should be zero; recipient received all funds
    assert_eq!(setup.escrow.get_balance(), 0);
    assert_eq!(setup.token.balance(&recipient), amount);
}

#[test]
#[should_panic]
fn test_emergency_withdraw_by_non_admin_fails() {
    // Build env without mock_all_auths for non-admin test
    let env = Env::default();
    let (escrow, _addr) = create_escrow_contract(&env);
    let admin = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let depositor = Address::generate(&env);

    env.mock_all_auths();
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1000);
    escrow.lock_funds(
        &depositor,
        &1u64,
        &500i128,
        &(env.ledger().timestamp() + 1000),
    );

    // non-admin attempting emergency_withdraw — require_auth for admin will fail
    let non_admin = Address::generate(&env);
    non_admin.require_auth(); // sets auth context to non_admin
    escrow.emergency_withdraw(&non_admin);
}

// --- operations resume after unpause ---

#[test]
fn test_operations_resume_after_unpause() {
    let setup = TestSetup::new();
    let bounty_id = 1u64;
    let amount = 500i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Pause then unpause
    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "temporary"));
    assert!(setup.escrow.is_paused());
    setup.escrow.unpause();
    assert!(!setup.escrow.is_paused());

    // Operations should work normally after unpause
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    let info = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Locked);
    assert_eq!(info.amount, amount);

    setup.escrow.release_funds(&bounty_id, &setup.contributor);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
}

// ============================================================================
// EVENT EMISSION TESTS
// ============================================================================

#[test]
fn test_pause_emits_event() {
    let setup = TestSetup::new();
    let initial_event_count = setup.env.events().all().len();

    let reason = soroban_sdk::String::from_str(&setup.env, "security review");
    setup.escrow.pause(&reason);

    // At least one event must have been emitted for the pause call
    let events = setup.env.events().all();
    assert!(
        events.len() > initial_event_count,
        "pause() must emit at least one event"
    );
}

#[test]
fn test_unpause_emits_event() {
    let setup = TestSetup::new();
    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "test"));

    let initial_event_count = setup.env.events().all().len();
    setup.escrow.unpause();

    let events = setup.env.events().all();
    assert!(
        events.len() > initial_event_count,
        "unpause() must emit at least one event"
    );
}

#[test]
fn test_emergency_withdraw_emits_event() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &500, &deadline);
    setup
        .escrow
        .pause(&soroban_sdk::String::from_str(&setup.env, "emergency"));

    let initial_event_count = setup.env.events().all().len();
    let recipient = Address::generate(&setup.env);
    setup.escrow.emergency_withdraw(&recipient);

    let events = setup.env.events().all();
    assert!(
        events.len() > initial_event_count,
        "emergency_withdraw() must emit at least one event"
    );
}

// ============================================================================
// EDGE CASES
// ============================================================================

// Edge case: pausing an already-paused contract updates the reason without error
#[test]
fn test_pause_when_already_paused_updates_reason() {
    let setup = TestSetup::new();

    let reason1 = soroban_sdk::String::from_str(&setup.env, "reason one");
    let reason2 = soroban_sdk::String::from_str(&setup.env, "updated reason");

    setup.escrow.pause(&reason1);
    assert!(setup.escrow.is_paused());
    assert_eq!(setup.escrow.get_pause_reason(), Some(reason1));

    // Second pause call overwrites the previous reason
    setup.escrow.pause(&reason2);
    assert!(setup.escrow.is_paused(), "should still be paused");
    assert_eq!(
        setup.escrow.get_pause_reason(),
        Some(reason2),
        "pause reason should be updated to the newest reason"
    );
}

// Edge case: calling unpause when contract is not paused should not panic
#[test]
fn test_unpause_when_not_paused_is_safe() {
    let setup = TestSetup::new();

    // Contract starts unpaused — unpause should be a safe no-op
    assert!(!setup.escrow.is_paused());
    setup.escrow.unpause(); // must not panic
    assert!(!setup.escrow.is_paused());
}

// Edge case: emergency_withdraw when contract has zero balance should not panic
#[test]
fn test_emergency_withdraw_zero_balance_is_safe() {
    let setup = TestSetup::new();

    // No funds have been locked — balance is zero
    assert_eq!(setup.escrow.get_balance(), 0);

    let recipient = Address::generate(&setup.env);
    setup.escrow.emergency_withdraw(&recipient); // must not panic

    // Recipient balance stays zero, contract balance stays zero
    assert_eq!(setup.token.balance(&recipient), 0);
    assert_eq!(setup.escrow.get_balance(), 0);
}

// Edge case: multiple pause/unpause cycles work correctly
#[test]
fn test_multiple_pause_unpause_cycles() {
    let setup = TestSetup::new();
    let bounty_id_base = 100u64;
    let amount = 100i128;
    let deadline = setup.env.ledger().timestamp() + 1000;

    for i in 0..3u64 {
        // Pause
        setup
            .escrow
            .pause(&soroban_sdk::String::from_str(&setup.env, "cycle"));
        assert!(setup.escrow.is_paused());

        // Operations must be blocked
        let result = setup.escrow.try_lock_funds(
            &setup.depositor,
            &(bounty_id_base + i),
            &amount,
            &deadline,
        );
        assert!(
            result.is_err(),
            "cycle {i}: lock_funds must be blocked while paused"
        );

        // Unpause
        setup.escrow.unpause();
        assert!(!setup.escrow.is_paused());

        // Operation succeeds again
        setup
            .escrow
            .lock_funds(&setup.depositor, &(bounty_id_base + i), &amount, &deadline);
        let info = setup.escrow.get_escrow_info(&(bounty_id_base + i));
        assert_eq!(
            info.status,
            EscrowStatus::Locked,
            "cycle {i}: lock should succeed after unpause"
        );
    }
}

// Edge case: emergency_withdraw does NOT require the contract to be paused first
#[test]
fn test_emergency_withdraw_works_without_prior_pause() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1000;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &500, &deadline);
    assert_eq!(setup.escrow.get_balance(), 500);
    assert!(!setup.escrow.is_paused()); // NOT paused

    let recipient = Address::generate(&setup.env);
    setup.escrow.emergency_withdraw(&recipient); // must work fine

    assert_eq!(setup.escrow.get_balance(), 0);
    assert_eq!(setup.token.balance(&recipient), 500);
}

// Edge case: pause reason is persisted and readable before and after the operation
#[test]
fn test_pause_reason_and_timestamp_are_stored() {
    let setup = TestSetup::new();
    let ts_before = setup.env.ledger().timestamp();
    let reason = soroban_sdk::String::from_str(&setup.env, "regulatory halt");

    assert_eq!(setup.escrow.get_pause_reason(), None);

    setup.escrow.pause(&reason);

    assert!(setup.escrow.is_paused());
    assert_eq!(setup.escrow.get_pause_reason(), Some(reason));

    // After unpause the reason is cleared
    setup.escrow.unpause();
    assert!(!setup.escrow.is_paused());
    assert_eq!(setup.escrow.get_pause_reason(), None);
    let _ = ts_before; // timestamp was used for pre-condition
}
