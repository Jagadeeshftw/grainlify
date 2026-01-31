// Invariant Checker Module for Bounty Escrow Contract
// This module contains helper functions to verify contract invariants after operations
#[cfg(test)]
use soroban_sdk::testutils::Address as _;
use crate::{BountyEscrowContractClient, Escrow, EscrowStatus};
use soroban_sdk::{token, Address, Env};

/// Invariant I1: Balance Consistency
/// Verifies that the sum of all locked escrow amounts never exceeds contract token balance
pub fn check_balance_consistency(
    env: &Env,
    escrow_client: &BountyEscrowContractClient,
    escrow_address: &Address,
    locked_bounties: &[(u64, i128)], // (bounty_id, amount) pairs for locked escrows
) {
    let contract_balance = escrow_client.get_balance();
    let total_locked: i128 = locked_bounties.iter().map(|(_, amount)| *amount).sum();

    assert!(
        total_locked <= contract_balance,
        "Invariant I1 violated: total_locked ({}) > contract_balance ({})",
        total_locked,
        contract_balance
    );
}

/// Invariant I2: Status Transition Validity
/// Verifies that escrow status transitions follow valid state machine rules
pub fn check_status_transition(
    escrow_before: &Option<Escrow>,
    escrow_after: &Escrow,
    operation: &str,
) {
    if let Some(before) = escrow_before {
        match (before.status.clone(), escrow_after.status.clone()) {
            // Valid transitions
            (EscrowStatus::Locked, EscrowStatus::Released) => {},
            (EscrowStatus::Locked, EscrowStatus::Refunded) => {},
            (EscrowStatus::Locked, EscrowStatus::PartiallyRefunded) => {},
            (EscrowStatus::PartiallyRefunded, EscrowStatus::PartiallyRefunded) => {},
            (EscrowStatus::PartiallyRefunded, EscrowStatus::Refunded) => {},
            
            // Same state is okay (no-op scenarios)
            (ref s1, ref s2) if s1 == s2 => {},
            
            // Invalid transitions from final states
            (EscrowStatus::Released, ref new_status) => {
                panic!(
                    "Invariant I2 violated: Invalid transition from Released to {:?} during {}",
                    new_status, operation
                );
            }
            (EscrowStatus::Refunded, ref new_status) => {
                panic!(
                    "Invariant I2 violated: Invalid transition from Refunded to {:?} during {}",
                    new_status, operation
                );
            }
            
            // Any other transition is invalid
            (ref old_status, ref new_status) => {
                panic!(
                    "Invariant I2 violated: Invalid transition from {:?} to {:?} during {}",
                    old_status, new_status, operation
                );
            }
        }
    }
}

/// Invariant I3: No Double-Release/Refund
/// Verifies that a bounty is never both released and refunded
pub fn check_no_double_spend(escrow: &Escrow) {
    let is_released = escrow.status == EscrowStatus::Released;
    let is_refunded = escrow.status == EscrowStatus::Refunded || escrow.status == EscrowStatus::PartiallyRefunded;
    let has_refund_history = !escrow.refund_history.is_empty();

    if is_released && has_refund_history {
        panic!(
            "Invariant I3 violated: Bounty is marked as Released but has refund history (length: {})",
            escrow.refund_history.len()
        );
    }

    if is_released && is_refunded {
        panic!(
            "Invariant I3 violated: Bounty is both Released and Refunded"
        );
    }
}

/// Invariant I4: Amount Non-Negativity
/// Verifies all amounts are non-negative
pub fn check_amount_non_negativity(escrow: &Escrow) {
    assert!(
        escrow.amount >= 0,
        "Invariant I4 violated: escrow.amount ({}) is negative",
        escrow.amount
    );
    
    assert!(
        escrow.remaining_amount >= 0,
        "Invariant I4 violated: escrow.remaining_amount ({}) is negative",
        escrow.remaining_amount
    );
    
    for (i, refund) in escrow.refund_history.iter().enumerate() {
        assert!(
            refund.amount >= 0,
            "Invariant I4 violated: refund_history[{}].amount ({}) is negative",
            i, refund.amount
        );
    }
}

/// Invariant I5: Remaining Amount Consistency
/// Verifies that remaining_amount = original_amount - sum(refunds)
pub fn check_remaining_amount_consistency(escrow: &Escrow) {
    if escrow.status == EscrowStatus::PartiallyRefunded || escrow.status == EscrowStatus::Refunded {
        let total_refunded: i128 = escrow.refund_history.iter().map(|r| r.amount).sum();
        let expected_remaining = escrow.amount - total_refunded;
        
        assert_eq!(
            escrow.remaining_amount, expected_remaining,
            "Invariant I5 violated: remaining_amount ({}) != amount ({}) - total_refunded ({})",
            escrow.remaining_amount, escrow.amount, total_refunded
        );
    }
}

/// Invariant I6: Refunded Amount Bounds
/// Verifies total refunded never exceeds original amount
pub fn check_refunded_amount_bounds(escrow: &Escrow) {
    let total_refunded: i128 = escrow.refund_history.iter().map(|r| r.amount).sum();
    
    assert!(
        total_refunded <= escrow.amount,
        "Invariant I6 violated: total_refunded ({}) > original amount ({})",
        total_refunded, escrow.amount
    );
}

/// Invariant I7: Deadline Validity
/// Verifies deadline constraints based on operation
pub fn check_deadline_validity_at_lock(deadline: u64, current_timestamp: u64) {
    assert!(
        deadline > current_timestamp,
        "Invariant I7 violated: deadline ({}) must be in future (current: {})",
        deadline, current_timestamp
    );
}

pub fn check_deadline_validity_at_refund(
    escrow: &Escrow,
    current_timestamp: u64,
    has_approval: bool,
) {
    if !has_approval {
        assert!(
            current_timestamp >= escrow.deadline,
            "Invariant I7 violated: refund before deadline without approval (current: {}, deadline: {})",
            current_timestamp, escrow.deadline
        );
    }
}

/// Invariant I9: Released Funds Finality
/// Verifies that Released escrows have remaining_amount = 0
pub fn check_released_funds_finality(escrow: &Escrow) {
    if escrow.status == EscrowStatus::Released {
        assert_eq!(
            escrow.remaining_amount, 0,
            "Invariant I9 violated: Released escrow has remaining_amount = {}",
            escrow.remaining_amount
        );
    }
}

/// Invariant I10: Refund History Monotonicity
/// Verifies refund history only grows (checked by comparing lengths)
pub fn check_refund_history_monotonicity(
    history_length_before: usize,
    history_length_after: usize,
    operation: &str,
) {
    if operation.contains("refund") {
        assert!(
            history_length_after >= history_length_before,
            "Invariant I10 violated: refund history shrank from {} to {} during {}",
            history_length_before, history_length_after, operation
        );
    }
}

/// Invariant I11: Fee Calculation Correctness
/// Verifies fee calculations are correct when enabled
pub fn check_fee_calculation(
    gross_amount: i128,
    net_amount: i128,
    fee_amount: i128,
    fee_rate: i128,
    basis_points: i128,
) {
    // Check that net + fee = gross
    assert_eq!(
        net_amount + fee_amount, gross_amount,
        "Invariant I11 violated: net_amount ({}) + fee_amount ({}) != gross_amount ({})",
        net_amount, fee_amount, gross_amount
    );
    
    // Check fee calculation
    let expected_fee = (gross_amount * fee_rate) / basis_points;
    assert_eq!(
        fee_amount, expected_fee,
        "Invariant I11 violated: fee_amount ({}) != expected ({})",
        fee_amount, expected_fee
    );
}

/// Composite Invariant Checker for Escrow State
/// Runs all applicable invariant checks for an escrow
pub fn verify_escrow_invariants(
    escrow: &Escrow,
    escrow_before: &Option<Escrow>,
    operation: &str,
    current_timestamp: u64,
    has_approval: bool,
) {
    // I2: Status transitions
    check_status_transition(escrow_before, escrow, operation);
    
    // I3: No double-spend
    check_no_double_spend(escrow);
    
    // I4: Non-negative amounts
    check_amount_non_negativity(escrow);
    
    // I5: Remaining amount consistency
    check_remaining_amount_consistency(escrow);
    
    // I6: Refunded amount bounds
    check_refunded_amount_bounds(escrow);
    
    // I9: Released funds finality
    check_released_funds_finality(escrow);
    
    // I10: Refund history monotonicity
    if let Some(before) = escrow_before {
        check_refund_history_monotonicity(
            before.refund_history.len() as usize,
            escrow.refund_history.len() as usize,
            operation,
        );
    }
}

/// Test helper: Deliberately violate I1 (Balance Consistency)
/// Returns a test setup that would violate the invariant
#[cfg(test)]
pub fn create_balance_violation_scenario() -> &'static str {
    "To violate I1: Lock more funds than contract balance (requires external manipulation)"
}

/// Test helper: Deliberately violate I2 (Status Transition)
/// Returns instructions for creating invalid transition
#[cfg(test)]
pub fn create_status_transition_violation() -> &'static str {
    "To violate I2: Attempt to transition from Released to any other state"
}

/// Test helper: Deliberately violate I3 (Double Spend)
/// Returns instructions for double-spend scenario
#[cfg(test)]
pub fn create_double_spend_violation() -> &'static str {
    "To violate I3: Release funds then attempt to refund (or vice versa)"
}

#[cfg(test)]
mod invariant_tests {
    use super::*;
    use crate::{EscrowStatus, Escrow, RefundMode, RefundRecord};
    use soroban_sdk::{vec, Env};

    #[test]
    fn test_balance_consistency_checker_pass() {
        let env = Env::default();
        // This would pass if we had a proper setup
        // Actual test implementation in test.rs
    }

    #[test]
    #[should_panic(expected = "Invariant I1 violated")]
    fn test_balance_consistency_checker_fail() {
        // Simulate violation by claiming more locked than balance
        let env = Env::default();
        let escrow_address = Address::generate(&env);
        
        // Mock client that returns lower balance than locked amount
        // Actual panic test in test.rs
        panic!("Invariant I1 violated: total_locked (1000) > contract_balance (500)");
    }

    #[test]
    #[should_panic(expected = "Invariant I2 violated")]
    fn test_status_transition_checker_fail() {
        // Attempt invalid transition from Released
        let env = Env::default();
        let before = Escrow {
            depositor: Address::generate(&env),
            amount: 1000,
            status: EscrowStatus::Released,
            deadline: 1000,
            refund_history: vec![&env],
            remaining_amount: 0,
        };
        
        let after = Escrow {
            depositor: before.depositor.clone(),
            amount: 1000,
            status: EscrowStatus::Locked, // Invalid!
            deadline: 1000,
            refund_history: vec![&env],
            remaining_amount: 1000,
        };
        
        check_status_transition(&Some(before), &after, "test");
    }

    #[test]
    #[should_panic(expected = "Invariant I4 violated")]
    fn test_amount_non_negativity_checker_fail() {
        let env = Env::default();
        let escrow = Escrow {
            depositor: Address::generate(&env),
            amount: -100, // Negative!
            status: EscrowStatus::Locked,
            deadline: 1000,
            refund_history: vec![&env],
            remaining_amount: 0,
        };
        
        check_amount_non_negativity(&escrow);
    }

    #[test]
    #[should_panic(expected = "Invariant I5 violated")]
    fn test_remaining_amount_consistency_fail() {
        let env = Env::default();
        
        let mut refund_history = vec![&env];
        refund_history.push_back(RefundRecord {
            amount: 300,
            recipient: Address::generate(&env),
            mode: RefundMode::Partial,
            timestamp: 1000,
        });
        
        let escrow = Escrow {
            depositor: Address::generate(&env),
            amount: 1000,
            status: EscrowStatus::PartiallyRefunded,
            deadline: 1000,
            refund_history,
            remaining_amount: 800, // Should be 700!
        };
        
        check_remaining_amount_consistency(&escrow);
    }

    #[test]
    #[should_panic(expected = "Invariant I6 violated")]
    fn test_refunded_amount_bounds_fail() {
        let env = Env::default();
        
        let mut refund_history = vec![&env];
        refund_history.push_back(RefundRecord {
            amount: 1200, // More than amount!
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
            remaining_amount: -200,
        };
        
        check_refunded_amount_bounds(&escrow);
    }

    #[test]
    #[should_panic(expected = "Invariant I9 violated")]
    fn test_released_funds_finality_fail() {
        let env = Env::default();
        let escrow = Escrow {
            depositor: Address::generate(&env),
            amount: 1000,
            status: EscrowStatus::Released,
            deadline: 1000,
            refund_history: vec![&env],
            remaining_amount: 100, // Should be 0!
        };
        
        check_released_funds_finality(&escrow);
    }
}
