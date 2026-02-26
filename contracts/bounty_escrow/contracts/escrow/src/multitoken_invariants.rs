// ============================================================================
// Multi-Token Balance Invariants  (Issue #591)
//
// This module defines and enforces explicit invariants that govern the
// relationship between per-escrow balances and the contract's actual token
// ledger balance.  The goal is to prevent accidental mixing or mis-accounting
// when the contract handles multiple bounties backed by the same token.
//
// Invariants enforced:
//
// INV-1  (Per-Escrow Sanity)
//        For every escrow:
//          - amount > 0
//          - remaining_amount >= 0
//          - remaining_amount <= amount
//          - Released => remaining_amount == 0
//
// INV-2  (Aggregate-to-Ledger)
//        Sum of all `remaining_amount` across *active* escrows ==
//        actual token balance held by the contract.
//
// INV-3  (Fee Separation)
//        If a fee was collected, it was transferred out at the time of
//        collection and is NOT part of the escrow remaining amounts.
//        (The current contract transfers fees immediately, so this is
//        enforced structurally rather than via an accounting bucket.)
//
// INV-4  (Refund Consistency)
//        For every escrow, sum of refund_history amounts <=
//        original amount – remaining_amount.
//
// INV-5  (Index Completeness)
//        Every bounty_id in the EscrowIndex has a corresponding Escrow entry.
//
// ============================================================================

use crate::{DataKey, Escrow, EscrowStatus};
use soroban_sdk::{token, Address, Env, Vec};

/// Full result of a multi-token balance invariant check.
/// Returned by `check_all_invariants` so callers can inspect what failed.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct InvariantReport {
    /// True when ALL invariants pass.
    pub healthy: bool,
    /// Total remaining amount summed across all active escrows.
    pub sum_remaining: i128,
    /// Actual token balance of the contract.
    pub token_balance: i128,
    /// Number of escrows that failed per-escrow sanity checks (INV-1).
    pub per_escrow_failures: u32,
    /// Number of bounty IDs in the index with no backing Escrow (INV-5).
    pub orphaned_index_entries: u32,
    /// Number of escrows where refund history is inconsistent (INV-4).
    pub refund_inconsistencies: u32,
    /// Human-readable list of violations.
    pub violations: soroban_sdk::Vec<soroban_sdk::String>,
}

// ---------------------------------------------------------------------------
// INV-1  Per-Escrow Sanity (re-exports the existing check with more detail)
// ---------------------------------------------------------------------------

/// Check per-escrow sanity (INV-1).
/// Returns `true` if the escrow passes all per-escrow invariants.
pub(crate) fn check_escrow_sanity(escrow: &Escrow) -> bool {
    if escrow.amount <= 0 {
        return false;
    }
    if escrow.remaining_amount < 0 {
        return false;
    }
    if escrow.remaining_amount > escrow.amount {
        return false;
    }
    if escrow.status == EscrowStatus::Released && escrow.remaining_amount != 0 {
        return false;
    }
    if escrow.status == EscrowStatus::Refunded && escrow.remaining_amount != 0 {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// INV-4  Refund Consistency
// ---------------------------------------------------------------------------

/// Check that the sum of refund_history amounts is consistent with the
/// amount that has been consumed (amount – remaining_amount).
pub(crate) fn check_refund_consistency(escrow: &Escrow) -> bool {
    let mut total_refunded: i128 = 0;
    for record in escrow.refund_history.iter() {
        if record.amount <= 0 {
            return false; // refund records should always be positive
        }
        total_refunded += record.amount;
    }
    let consumed = escrow.amount - escrow.remaining_amount;
    // Total refunded must not exceed consumed amount.
    // It can be less because some consumption may be from releases, not refunds.
    total_refunded <= consumed
}

// ---------------------------------------------------------------------------
// INV-2  Aggregate-to-Ledger
// ---------------------------------------------------------------------------

/// Sum the remaining_amount of all active (Locked or PartiallyRefunded) escrows.
pub(crate) fn sum_active_escrow_balances(env: &Env) -> i128 {
    let index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap_or(Vec::new(env));

    let mut total: i128 = 0;
    for bounty_id in index.iter() {
        if let Some(escrow) = env
            .storage()
            .persistent()
            .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
        {
            // Only active escrows contribute to the contract balance.
            if escrow.status == EscrowStatus::Locked
                || escrow.status == EscrowStatus::PartiallyRefunded
            {
                total += escrow.remaining_amount;
            }
        }
    }
    total
}

/// Get the actual token balance held by the contract.
pub(crate) fn get_contract_token_balance(env: &Env) -> i128 {
    let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let client = token::Client::new(env, &token_addr);
    client.balance(&env.current_contract_address())
}

// ---------------------------------------------------------------------------
// INV-5  Index Completeness
// ---------------------------------------------------------------------------

/// Count how many bounty_ids in the global index have no corresponding Escrow.
pub(crate) fn count_orphaned_index_entries(env: &Env) -> u32 {
    let index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap_or(Vec::new(env));

    let mut orphans: u32 = 0;
    for bounty_id in index.iter() {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            orphans += 1;
        }
    }
    orphans
}

// ---------------------------------------------------------------------------
// Full Invariant Check
// ---------------------------------------------------------------------------

/// Run ALL invariant checks and return a comprehensive report.
///
/// This is intended to be called from:
/// - An on-chain `verify_all_invariants()` view function
/// - Tests that want to assert full system health
pub(crate) fn check_all_invariants(env: &Env) -> InvariantReport {
    let mut violations: Vec<soroban_sdk::String> = Vec::new(env);
    let mut per_escrow_failures: u32 = 0;
    let mut refund_inconsistencies: u32 = 0;

    let index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap_or(Vec::new(env));

    // INV-1 + INV-4: Check each escrow
    for bounty_id in index.iter() {
        if let Some(escrow) = env
            .storage()
            .persistent()
            .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
        {
            if !check_escrow_sanity(&escrow) {
                per_escrow_failures += 1;
                violations.push_back(soroban_sdk::String::from_str(
                    env,
                    "INV-1: Per-escrow sanity check failed",
                ));
            }
            if !check_refund_consistency(&escrow) {
                refund_inconsistencies += 1;
                violations.push_back(soroban_sdk::String::from_str(
                    env,
                    "INV-4: Refund history inconsistency",
                ));
            }
        }
    }

    // INV-2: Aggregate-to-Ledger
    let sum_remaining = sum_active_escrow_balances(env);
    let token_balance = get_contract_token_balance(env);

    if sum_remaining != token_balance {
        violations.push_back(soroban_sdk::String::from_str(
            env,
            "INV-2: Sum of remaining != contract balance",
        ));
    }

    // INV-5: Index Completeness
    let orphaned_index_entries = count_orphaned_index_entries(env);
    if orphaned_index_entries > 0 {
        violations.push_back(soroban_sdk::String::from_str(
            env,
            "INV-5: Orphaned index entries found",
        ));
    }

    let healthy = violations.is_empty();

    InvariantReport {
        healthy,
        sum_remaining,
        token_balance,
        per_escrow_failures,
        orphaned_index_entries,
        refund_inconsistencies,
        violations,
    }
}

/// Panic with a descriptive message if any invariant is violated.
/// Called from critical paths (lock, release, refund) after state mutation.
#[allow(dead_code)]
pub(crate) fn assert_all_invariants(env: &Env) {
    let report = check_all_invariants(env);
    if !report.healthy {
        panic!("Multi-token invariant violation detected");
    }
}

// ---------------------------------------------------------------------------
// Per-Operation Assertions
//
// Lightweight checks for hot paths — only assert the invariants that are
// most relevant to the specific operation.
// ---------------------------------------------------------------------------

/// Assert after a lock: aggregate balance must equal token balance.
pub(crate) fn assert_after_lock(env: &Env) {
    let sum = sum_active_escrow_balances(env);
    let actual = get_contract_token_balance(env);
    if sum != actual {
        panic!(
            "INV-2 violated after lock: escrow sum ({}) != balance ({})",
            sum, actual
        );
    }
}

/// Assert after a release/refund: aggregate balance must equal token balance.
pub(crate) fn assert_after_disbursement(env: &Env) {
    let sum = sum_active_escrow_balances(env);
    let actual = get_contract_token_balance(env);
    if sum != actual {
        panic!(
            "INV-2 violated after disbursement: escrow sum ({}) != balance ({})",
            sum, actual
        );
    }
}
