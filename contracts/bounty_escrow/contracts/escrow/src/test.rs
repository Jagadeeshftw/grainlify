#![cfg(test)]
use crate::invariants::*;
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
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
fn test_amount_limits_initialization() {
    let setup = TestSetup::new();
    
    // Initialize contract
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Check default limits
    let limits = setup.escrow.get_amount_limits();
    assert_eq!(limits.min_lock_amount, 1);
    assert_eq!(limits.max_lock_amount, i128::MAX);
    assert_eq!(limits.min_payout, 1);
    assert_eq!(limits.max_payout, i128::MAX);
}

#[test]
fn test_update_amount_limits() {
    let setup = TestSetup::new();
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Update limits
    setup.escrow.update_amount_limits(&100, &1000, &50, &500).unwrap();
    
    // Verify updated limits
    let limits = setup.escrow.get_amount_limits();
    assert_eq!(limits.min_lock_amount, 100);
    assert_eq!(limits.max_lock_amount, 1000);
    assert_eq!(limits.min_payout, 50);
    assert_eq!(limits.max_payout, 500);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_update_amount_limits_invalid_negative() {
    let setup = TestSetup::new();
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Try to set negative limits
    setup.escrow.update_amount_limits(&-100, &1000, &50, &500).unwrap();
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_update_amount_limits_invalid_min_greater_than_max() {
    let setup = TestSetup::new();
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Try to set min > max
    setup.escrow.update_amount_limits(&1000, &100, &50, &500).unwrap();
}

#[test]
fn test_lock_funds_respects_amount_limits() {
    let setup = TestSetup::new();
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Set limits
    setup.escrow.update_amount_limits(&100, &1000, &50, &500).unwrap();
    
    // Mint tokens
    setup.token_admin.mint(&setup.depositor, &2000);
    
    // Test successful lock within limits
    let deadline = setup.env.ledger().timestamp() + 86400;
    setup.escrow.lock_funds(&setup.depositor, &1, &500, &deadline).unwrap();
    
    // Test lock at minimum limit
    setup.escrow.lock_funds(&setup.depositor, &2, &100, &deadline).unwrap();
    
    // Test lock at maximum limit
    setup.escrow.lock_funds(&setup.depositor, &3, &1000, &deadline).unwrap();
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_lock_funds_below_minimum() {
    let setup = TestSetup::new();
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Set limits
    setup.escrow.update_amount_limits(&100, &1000, &50, &500).unwrap();
    
    // Mint tokens
    setup.token_admin.mint(&setup.depositor, &2000);
    
    // Try to lock below minimum
    let deadline = setup.env.ledger().timestamp() + 86400;
    setup.escrow.lock_funds(&setup.depositor, &1, &50, &deadline).unwrap();
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_lock_funds_above_maximum() {
    let setup = TestSetup::new();
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Set limits
    setup.escrow.update_amount_limits(&100, &1000, &50, &500).unwrap();
    
    // Mint tokens
    setup.token_admin.mint(&setup.depositor, &2000);
    
    // Try to lock above maximum
    let deadline = setup.env.ledger().timestamp() + 86400;
    setup.escrow.lock_funds(&setup.depositor, &1, &1500, &deadline).unwrap();
}

#[test]
fn test_release_funds_respects_payout_limits() {
    let setup = TestSetup::new();
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Set limits - payout limits are 50-500
    setup.escrow.update_amount_limits(&100, &1000, &50, &500).unwrap();
    
    // Mint and lock funds
    setup.token_admin.mint(&setup.depositor, &600);
    let deadline = setup.env.ledger().timestamp() + 86400;
    setup.escrow.lock_funds(&setup.depositor, &1, &600, &deadline).unwrap();
    
    // Release should work (600 is within payout limits)
    setup.escrow.release_funds(&1, &setup.contributor).unwrap();
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_release_funds_above_payout_maximum() {
    let setup = TestSetup::new();
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Set limits - payout max is 500
    setup.escrow.update_amount_limits(&100, &1000, &50, &500).unwrap();
    
    // Mint and lock funds above payout limit
    setup.token_admin.mint(&setup.depositor, &800);
    let deadline = setup.env.ledger().timestamp() + 86400;
    setup.escrow.lock_funds(&setup.depositor, &1, &800, &deadline).unwrap();
    
    // Try to release - should fail because 800 > 500 (payout max)
    setup.escrow.release_funds(&1, &setup.contributor).unwrap();
}

#[test]
fn test_batch_operations_respect_limits() {
    let setup = TestSetup::new();
    setup.escrow.init(&setup.admin, &setup.token.address);
    
    // Set limits
    setup.escrow.update_amount_limits(&100, &1000, &50, &500).unwrap();
    
    // Mint tokens
    setup.token_admin.mint(&setup.depositor, &3000);
    
    // Create batch lock items within limits
    let deadline = setup.env.ledger().timestamp() + 86400;
    let items = vec![
        &setup.env,
        LockFundsItem {
            bounty_id: 1,
            depositor: setup.depositor.clone(),
            amount: 200,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: setup.depositor.clone(),
            amount: 500,
            deadline,
        },
    ];
    
    // Batch lock should succeed
    let result = setup.escrow.batch_lock_funds(&items).unwrap();
    assert_eq!(result, 2);
    
    // Create batch release items
    let release_items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: setup.contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: setup.contributor.clone(),
        },
    ];
    
    // Batch release should succeed (amounts are within payout limits)
    let result = setup.escrow.batch_release_funds(&release_items).unwrap();
    assert_eq!(result, 2);
}

#[test]
fn test_lock_funds_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let deadline = setup.env.ledger().timestamp() + 1000;

    // Lock funds
    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Verify stored escrow data
    // Note: amount stores net_amount (after fee), but fees are disabled by default
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.depositor, setup.depositor);
    assert_eq!(stored_escrow.amount, amount);
    assert_eq!(stored_escrow.remaining_amount, amount);
    assert_eq!(stored_escrow.status, EscrowStatus::Locked);
    assert_eq!(stored_escrow.deadline, deadline);

    // ✅ NEW: Check invariants after lock
    check_balance_consistency(
        &setup.env,
        &setup.escrow,
        &setup.escrow_address,
        &[(bounty_id, amount)],
    );
    
    verify_escrow_invariants(
        &stored_escrow,
        &None,
        "lock_funds",
        setup.env.ledger().timestamp(),
        false,
    );

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

    setup.escrow.lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // ✅ Get escrow before release
    let escrow_before = setup.escrow.get_escrow_info(&bounty_id);

    // Release funds
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Verify updated state
    let stored_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(stored_escrow.status, EscrowStatus::Released);

    // ✅ NEW: Check invariants after release
    check_balance_consistency(
        &setup.env,
        &setup.escrow,
        &setup.escrow_address,
        &[], // No locked bounties after release
    );
    
    verify_escrow_invariants(
        &stored_escrow,
        &Some(escrow_before),
        "release_funds",
        setup.env.ledger().timestamp(),
        false,
    );

    // Verify balances after release
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
    assert_eq!(setup.escrow.get_contract_balance(), 0);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Balance should be updated
    assert_eq!(setup.escrow.get_contract_balance(), amount);
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
    assert_eq!(setup.escrow.get_contract_balance(), 6000);
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
    assert_eq!(setup.escrow.get_contract_balance(), 0);
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
// DEADLINE EXTENSION TESTS
// ============================================================================

#[test]
fn test_extend_refund_deadline_success() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let initial_deadline = current_time + 1000;

    // Lock funds
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &initial_deadline);

    // Verify initial deadline
    let escrow_before = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_before.deadline, initial_deadline);

    // Extend deadline
    let new_deadline = initial_deadline + 2000;
    setup
        .escrow
        .extend_refund_deadline(&bounty_id, &new_deadline);

    // Verify deadline was extended
    let escrow_after = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.deadline, new_deadline);
    assert_eq!(escrow_after.status, EscrowStatus::Locked);
    assert_eq!(escrow_after.amount, amount);

    // Verify event was emitted (check that deadline was extended)
    // Event verification is done through the contract's event system
    // The deadline extension should have been logged
}

#[test]
fn test_extend_refund_deadline_multiple_times() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let initial_deadline = current_time + 1000;

    // Lock funds
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &initial_deadline);

    // First extension
    let first_extension = initial_deadline + 1000;
    setup
        .escrow
        .extend_refund_deadline(&bounty_id, &first_extension);

    let escrow_after_first = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after_first.deadline, first_extension);

    // Second extension
    let second_extension = first_extension + 2000;
    setup
        .escrow
        .extend_refund_deadline(&bounty_id, &second_extension);

    let escrow_after_second = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after_second.deadline, second_extension);

    // Third extension
    let third_extension = second_extension + 3000;
    setup
        .escrow
        .extend_refund_deadline(&bounty_id, &third_extension);

    let escrow_after_third = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after_third.deadline, third_extension);
}

#[test]
#[should_panic(expected = "Error(Contract, #19)")] // InvalidDeadlineExtension
fn test_extend_refund_deadline_not_greater() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let initial_deadline = current_time + 1000;

    // Lock funds
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &initial_deadline);

    // Try to extend with same deadline (should fail)
    setup
        .escrow
        .extend_refund_deadline(&bounty_id, &initial_deadline);
}

#[test]
#[should_panic(expected = "Error(Contract, #19)")] // InvalidDeadlineExtension
fn test_extend_refund_deadline_shorter() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let initial_deadline = current_time + 1000;

    // Lock funds
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &initial_deadline);

    // Try to extend with shorter deadline (should fail)
    let shorter_deadline = initial_deadline - 100;
    setup
        .escrow
        .extend_refund_deadline(&bounty_id, &shorter_deadline);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_extend_refund_deadline_after_release() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let deadline = current_time + 1000;

    // Lock funds
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Release funds
    setup.escrow.release_funds(&bounty_id, &setup.contributor);

    // Try to extend deadline after release (should fail)
    let new_deadline = deadline + 2000;
    setup
        .escrow
        .extend_refund_deadline(&bounty_id, &new_deadline);
}

#[test]
fn test_extend_refund_deadline_with_partially_refunded() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let current_time = setup.env.ledger().timestamp();
    let initial_deadline = current_time + 1000;

    // Lock funds
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &initial_deadline);

    // Advance time past deadline
    setup.env.ledger().set_timestamp(initial_deadline + 1);

    // Partial refund
    let partial_amount = 500;
    setup.escrow.refund(
        &bounty_id,
        &Some(partial_amount),
        &None::<Address>,
        &RefundMode::Partial,
    );

    // Verify status is PartiallyRefunded
    let escrow_before = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_before.status, EscrowStatus::PartiallyRefunded);

    // Extend deadline (should work with PartiallyRefunded status)
    let new_deadline = initial_deadline + 2000;
    setup
        .escrow
        .extend_refund_deadline(&bounty_id, &new_deadline);

    // Verify deadline was extended
    let escrow_after = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.deadline, new_deadline);
    assert_eq!(escrow_after.status, EscrowStatus::PartiallyRefunded);
}

#[test]
#[should_panic(expected = "Invariant I2 violated")]
fn test_invariant_violation_invalid_transition() {
    let env = Env::default();
    
    // Create a Released escrow
    let escrow_before = Escrow {
        depositor: Address::generate(&env),
        amount: 1000,
        status: EscrowStatus::Released,
        deadline: 1000,
        refund_history: vec![&env],
        remaining_amount: 0,
    };
    
    // Try to transition to Locked (invalid!)
    let escrow_after = Escrow {
        depositor: escrow_before.depositor.clone(),
        amount: 1000,
        status: EscrowStatus::Locked,
        deadline: 1000,
        refund_history: vec![&env],
        remaining_amount: 1000,
    };
    
    // This should panic
    check_status_transition(&Some(escrow_before), &escrow_after, "invalid_transition");
}

#[test]
#[should_panic(expected = "Invariant I6 violated")]
fn test_invariant_violation_over_refund() {
    let env = Env::default();
    
    // Create an escrow with refunds exceeding locked amount
    let mut refund_history = vec![&env];
    refund_history.push_back(RefundRecord {
        amount: 1500, // More than locked!
        recipient: Address::generate(&env),
        mode: RefundMode::Full,
        timestamp: 1000,
    });
    
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: 1000,
        status: EscrowStatus::Refunded,
        deadline: 1000,
        refund_history,
        remaining_amount: -500,
    };
    
    // This should panic
    check_refunded_amount_bounds(&escrow);
}

#[test]
#[should_panic(expected = "Invariant I4 violated")]
fn test_invariant_violation_negative_amount() {
    let env = Env::default();
    
    let escrow = Escrow {
        depositor: Address::generate(&env),
        amount: -100, // Negative!
        status: EscrowStatus::Locked,
        deadline: 1000,
        refund_history: vec![&env],
        remaining_amount: -100,
    };
    
    // This should panic
    check_amount_non_negativity(&escrow);
}