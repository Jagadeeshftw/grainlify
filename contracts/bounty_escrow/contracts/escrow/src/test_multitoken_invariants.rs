//! Tests for multi-token balance invariants (Issue #591).
//!
//! These tests verify that:
//! - INV-1: Per-escrow sanity checks catch inconsistent state
//! - INV-2: Sum of active escrow balances always equals contract token balance
//! - INV-4: Refund history is consistent with consumed amounts
//! - INV-5: Index completeness (no orphaned entries)
//!
//! Each test also verifies that `verify_all_invariants()` returns the expected
//! boolean, ensuring the on-chain view function is useful for monitoring.

#![cfg(test)]

use super::*;
use crate::multitoken_invariants;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_token<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let addr = contract.address();
    (
        token::Client::new(e, &addr),
        token::StellarAssetClient::new(e, &addr),
    )
}

fn create_escrow<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &id)
}

struct InvSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> InvSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token(&env, &admin);
        let escrow = create_escrow(&env);
        escrow.init(&admin, &token.address);

        token_admin.mint(&depositor, &100_000_000);

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

// ===========================================================================
// INV-1: Per-Escrow Sanity
// ===========================================================================

#[test]
fn test_inv1_healthy_escrow_passes() {
    let escrow = Escrow {
        depositor: Address::generate(&Env::default()),
        amount: 1_000,
        remaining_amount: 1_000,
        status: EscrowStatus::Locked,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
    };
    assert!(multitoken_invariants::check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_negative_amount_fails() {
    let escrow = Escrow {
        depositor: Address::generate(&Env::default()),
        amount: -1,
        remaining_amount: 0,
        status: EscrowStatus::Locked,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
    };
    assert!(!multitoken_invariants::check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_remaining_exceeds_amount_fails() {
    let escrow = Escrow {
        depositor: Address::generate(&Env::default()),
        amount: 500,
        remaining_amount: 600,
        status: EscrowStatus::Locked,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
    };
    assert!(!multitoken_invariants::check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_released_with_nonzero_remaining_fails() {
    let escrow = Escrow {
        depositor: Address::generate(&Env::default()),
        amount: 1_000,
        remaining_amount: 100,
        status: EscrowStatus::Released,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
    };
    assert!(!multitoken_invariants::check_escrow_sanity(&escrow));
}

#[test]
fn test_inv1_refunded_with_nonzero_remaining_fails() {
    let escrow = Escrow {
        depositor: Address::generate(&Env::default()),
        amount: 1_000,
        remaining_amount: 100,
        status: EscrowStatus::Refunded,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
    };
    assert!(!multitoken_invariants::check_escrow_sanity(&escrow));
}

// ===========================================================================
// INV-2: Aggregate-to-Ledger (sum remaining == contract balance)
// ===========================================================================

#[test]
fn test_inv2_single_lock_invariant_holds() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);

    // verify_all_invariants calls check_all_invariants internally
    assert!(s.escrow.verify_all_invariants());

    // Double-check manually
    s.env.as_contract(&s.escrow.address, || {
        let sum = multitoken_invariants::sum_active_escrow_balances(&s.env);
        let balance = multitoken_invariants::get_contract_token_balance(&s.env);
        assert_eq!(sum, balance);
        assert_eq!(sum, 1_000);
    });
}

#[test]
fn test_inv2_multiple_locks_invariant_holds() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.escrow.lock_funds(&s.depositor, &2_u64, &2_000, &deadline);
    s.escrow.lock_funds(&s.depositor, &3_u64, &3_000, &deadline);

    assert!(s.escrow.verify_all_invariants());

    s.env.as_contract(&s.escrow.address, || {
        let sum = multitoken_invariants::sum_active_escrow_balances(&s.env);
        let balance = multitoken_invariants::get_contract_token_balance(&s.env);
        assert_eq!(sum, 6_000);
        assert_eq!(sum, balance);
    });
}

#[test]
fn test_inv2_lock_then_release_invariant_holds() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.escrow.lock_funds(&s.depositor, &2_u64, &2_000, &deadline);

    // Release bounty 1
    s.escrow.release_funds(&1_u64, &s.contributor);

    assert!(s.escrow.verify_all_invariants());

    s.env.as_contract(&s.escrow.address, || {
        let sum = multitoken_invariants::sum_active_escrow_balances(&s.env);
        let balance = multitoken_invariants::get_contract_token_balance(&s.env);
        // Only bounty 2 is still active
        assert_eq!(sum, 2_000);
        assert_eq!(sum, balance);
    });
}

#[test]
fn test_inv2_lock_then_refund_invariant_holds() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.escrow.lock_funds(&s.depositor, &2_u64, &2_000, &deadline);

    // Advance past deadline
    s.env.ledger().set_timestamp(deadline + 1);

    // Refund bounty 1
    s.escrow.refund(&1_u64);

    assert!(s.escrow.verify_all_invariants());

    s.env.as_contract(&s.escrow.address, || {
        let sum = multitoken_invariants::sum_active_escrow_balances(&s.env);
        let balance = multitoken_invariants::get_contract_token_balance(&s.env);
        assert_eq!(sum, 2_000);
        assert_eq!(sum, balance);
    });
}

#[test]
fn test_inv2_all_released_contract_empty() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &500, &deadline);
    s.escrow.lock_funds(&s.depositor, &2_u64, &300, &deadline);

    s.escrow.release_funds(&1_u64, &s.contributor);
    s.escrow.release_funds(&2_u64, &s.contributor);

    assert!(s.escrow.verify_all_invariants());

    s.env.as_contract(&s.escrow.address, || {
        let sum = multitoken_invariants::sum_active_escrow_balances(&s.env);
        let balance = multitoken_invariants::get_contract_token_balance(&s.env);
        assert_eq!(sum, 0);
        assert_eq!(balance, 0);
    });
}

#[test]
fn test_inv2_partial_release_invariant_holds() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow
        .lock_funds(&s.depositor, &1_u64, &10_000, &deadline);

    // Partial release
    s.escrow.partial_release(&1_u64, &s.contributor, &3_000);

    assert!(s.escrow.verify_all_invariants());

    s.env.as_contract(&s.escrow.address, || {
        let sum = multitoken_invariants::sum_active_escrow_balances(&s.env);
        let balance = multitoken_invariants::get_contract_token_balance(&s.env);
        assert_eq!(sum, 7_000);
        assert_eq!(sum, balance);
    });
}

// ===========================================================================
// INV-4: Refund Consistency
// ===========================================================================

#[test]
fn test_inv4_refund_after_deadline_consistent() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 1_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &5_000, &deadline);

    s.env.ledger().set_timestamp(deadline + 1);
    s.escrow.refund(&1_u64);

    assert!(s.escrow.verify_all_invariants());

    // Check refund consistency directly
    s.env.as_contract(&s.escrow.address, || {
        let escrow: Escrow = s
            .env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(1))
            .unwrap();
        assert!(multitoken_invariants::check_refund_consistency(&escrow));
        assert_eq!(escrow.refund_history.len(), 1);
    });
}

#[test]
fn test_inv4_no_refund_history_is_consistent() {
    let escrow = Escrow {
        depositor: Address::generate(&Env::default()),
        amount: 1_000,
        remaining_amount: 1_000,
        status: EscrowStatus::Locked,
        deadline: 999,
        refund_history: soroban_sdk::Vec::new(&Env::default()),
    };
    assert!(multitoken_invariants::check_refund_consistency(&escrow));
}

// ===========================================================================
// INV-5: Index Completeness
// ===========================================================================

#[test]
fn test_inv5_no_orphans_after_normal_flow() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.escrow.lock_funds(&s.depositor, &2_u64, &2_000, &deadline);

    s.env.as_contract(&s.escrow.address, || {
        let orphans = multitoken_invariants::count_orphaned_index_entries(&s.env);
        assert_eq!(orphans, 0);
    });
}

#[test]
fn test_inv5_tampered_index_detects_orphan() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);

    // Tamper: add a fake bounty ID to the index without a backing escrow
    s.env.as_contract(&s.escrow.address, || {
        let mut index: soroban_sdk::Vec<u64> = s
            .env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(soroban_sdk::Vec::new(&s.env));
        index.push_back(999_u64); // no Escrow(999) exists
        s.env
            .storage()
            .persistent()
            .set(&DataKey::EscrowIndex, &index);
    });

    s.env.as_contract(&s.escrow.address, || {
        let orphans = multitoken_invariants::count_orphaned_index_entries(&s.env);
        assert_eq!(orphans, 1);

        let report = multitoken_invariants::check_all_invariants(&s.env);
        assert!(!report.healthy);
        assert_eq!(report.orphaned_index_entries, 1);
    });
}

// ===========================================================================
// Full Invariant Report
// ===========================================================================

#[test]
fn test_full_invariant_report_healthy() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.escrow.lock_funds(&s.depositor, &2_u64, &2_000, &deadline);

    s.env.as_contract(&s.escrow.address, || {
        let report = multitoken_invariants::check_all_invariants(&s.env);
        assert!(report.healthy);
        assert_eq!(report.sum_remaining, 3_000);
        assert_eq!(report.token_balance, 3_000);
        assert_eq!(report.per_escrow_failures, 0);
        assert_eq!(report.orphaned_index_entries, 0);
        assert_eq!(report.refund_inconsistencies, 0);
        assert_eq!(report.violations.len(), 0);
    });
}

#[test]
fn test_full_invariant_report_after_mixed_operations() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 1_000;

    // Lock 3 bounties
    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);
    s.escrow.lock_funds(&s.depositor, &2_u64, &2_000, &deadline);
    s.escrow.lock_funds(&s.depositor, &3_u64, &3_000, &deadline);

    // Release bounty 1
    s.escrow.release_funds(&1_u64, &s.contributor);

    // Refund bounty 3 after deadline
    s.env.ledger().set_timestamp(deadline + 1);
    s.escrow.refund(&3_u64);

    // verify_all_invariants should still hold
    assert!(s.escrow.verify_all_invariants());

    s.env.as_contract(&s.escrow.address, || {
        let report = multitoken_invariants::check_all_invariants(&s.env);
        assert!(report.healthy);
        // Only bounty 2 remains active
        assert_eq!(report.sum_remaining, 2_000);
        assert_eq!(report.token_balance, 2_000);
    });
}

#[test]
fn test_verify_all_invariants_not_initialized_returns_false() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &id);

    // Contract not initialized yet
    assert!(!client.verify_all_invariants());
}

#[test]
fn test_tampered_balance_detected_by_invariant() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 5_000;

    s.escrow.lock_funds(&s.depositor, &1_u64, &1_000, &deadline);

    // Tamper: modify the escrow remaining_amount without moving tokens
    s.env.as_contract(&s.escrow.address, || {
        let mut escrow: Escrow = s
            .env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(1))
            .unwrap();
        escrow.remaining_amount = 5_000; // Inflated — doesn't match actual balance
        s.env
            .storage()
            .persistent()
            .set(&DataKey::Escrow(1_u64), &escrow);

        let report = multitoken_invariants::check_all_invariants(&s.env);
        assert!(!report.healthy);
        // INV-1 should fail (remaining > amount)
        assert!(report.per_escrow_failures > 0);
        // INV-2 should fail (sum != balance)
        assert_ne!(report.sum_remaining, report.token_balance);
    });
}

// ===========================================================================
// Lifecycle Integration Tests (INV-2 maintained across full lifecycle)
// ===========================================================================

#[test]
fn test_invariant_maintained_through_full_lifecycle() {
    let s = InvSetup::new();
    let deadline = s.env.ledger().timestamp() + 2_000;

    // Phase 1: Multiple locks
    for i in 1_u64..=5 {
        s.escrow
            .lock_funds(&s.depositor, &i, &(i as i128 * 1_000), &deadline);
    }
    assert!(s.escrow.verify_all_invariants());

    // Phase 2: Release some
    s.escrow.release_funds(&1_u64, &s.contributor);
    s.escrow.release_funds(&3_u64, &s.contributor);
    assert!(s.escrow.verify_all_invariants());

    // Phase 3: Partial release
    s.escrow.partial_release(&5_u64, &s.contributor, &2_000);
    assert!(s.escrow.verify_all_invariants());

    // Phase 4: Refund after deadline
    s.env.ledger().set_timestamp(deadline + 1);
    s.escrow.refund(&2_u64);
    assert!(s.escrow.verify_all_invariants());

    // Final check: only bounties 4 and 5 (partially) should be active
    s.env.as_contract(&s.escrow.address, || {
        let report = multitoken_invariants::check_all_invariants(&s.env);
        assert!(report.healthy);
        // bounty 4: 4_000, bounty 5: 5_000 - 2_000 = 3_000 => 7_000
        assert_eq!(report.sum_remaining, 7_000);
        assert_eq!(report.token_balance, 7_000);
    });
}
