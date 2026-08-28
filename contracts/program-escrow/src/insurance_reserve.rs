//! # Insurance Reserve Module
//!
//! This module specifies, enforces, and implements the solvency controls, accounting
//! invariants, and transition guards for the segregated on-chain insurance reserve
//! in `ProgramEscrowContract` (Issue #1723).
//!
//! ## Problem Statement
//!
//! The program escrow contract represents reserve behavior through fee carve-outs and
//! administrative withdrawals, but requires a single documented invariant connecting
//! reserve debits, credits, and failed payouts across all transaction paths.
//!
//! ## Design Decision: Solvency Invariant & Prohibition of Transient Negative Balances
//!
//! ### The Solvency Invariant Equation
//!
//! Let $R_t$ denote the on-chain insurance reserve balance stored under
//! `DataKey::InsuranceReserve` at transaction sequence $t \ge 0$.
//!
//! $$R_t = R_0 + \sum_{i=1}^t \text{Credits}_i - \sum_{j=1}^t \text{Debits}_j$$
//!
//! Where:
//! - **$\text{Credits}$** originate strictly from fee carve-out allocations during fund operations
//!   (`lock_program_funds`, `single_payout`, `batch_payout`). For every fee $F$ collected:
//!   $$\text{reserve\_share} = \left\lceil \frac{F \times \text{insurance\_reserve\_bps}}{\text{BASIS\_POINTS}} \right\rceil$$
//!   $$\text{recipient\_share} = F - \text{reserve\_share}$$
//!   with the conservation property: $\text{reserve\_share} + \text{recipient\_share} = F$.
//!
//! - **$\text{Debits}$** originate strictly from authorized administrative withdrawals
//!   (`withdraw_insurance_reserve`) or potential protocol claims coverage.
//!
//! - **Failed Payouts & Transaction Reversals**: Under Soroban's atomic execution model,
//!   any failed transaction (such as a payout fee shortfall, insufficient remaining balance,
//!   unauthorized caller, transfer failure, or cancelled operation) causes a complete state
//!   rollback:
//!   $$\Delta R = 0$$
//!   No phantom debits or credits are persisted if an operation fails.
//!
//! ### Strict Non-Negative Solvency Condition
//!
//! $$R_t \ge 0 \quad \forall t \ge 0$$
//!
//! ### Can the reserve be temporarily negative during a transaction?
//!
//! **DECISION: NO.** The reserve balance is **strictly non-negative** at all times.
//! It may **NEVER** be temporarily negative during any stage of transaction execution.
//!
//! - **Rationale**: Permitting transient negative balances introduces dangerous insolvency
//!   windows, reentrancy vulnerabilities, off-chain accounting desynchronization, and
//!   catastrophic risk if an intermediate call or cross-contract transfer fails mid-transaction.
//! - **Enforcement**: Every debit operation mandates a strict pre-condition check:
//!   $$\text{amount} \le \text{balance\_before}$$
//!   If $\text{amount} > \text{balance\_before}$, the call immediately panics with
//!   `ContractError::InsufficientInsuranceReserve` (error code 705) *before* any state
//!   mutation or token transfer can occur. An underfunded operation fails safely without
//!   altering storage or emitting withdrawal events.
//!
//! ## State Transitions Across Paths
//!
//! | Path | Trigger Operation | Reserve Impact ($\Delta R$) | Preconditions | Postconditions & Event Assertions |
//! |------|-------------------|-----------------------------|---------------|-----------------------------------|
//! | **Normal Payout** | `single_payout` / `batch_payout` | $+ \text{reserve\_share}$ | `fee_enabled == true`, `bps > 0` | Storage updated: $R_{t+1} = R_t + \text{reserve\_share}$. `FeeCollectedEvent` emitted. Conservation holds. |
//! | **Fee Shortfall / Underfunded** | Payout with 0 fee or debit $> R_t$ | $0$ | Fee is $0$ or debit $> R_t$ | Fails safely with error 705 (`InsufficientInsuranceReserve`) on over-withdrawal. No state mutation; no withdrawal event. |
//! | **Refund** | `cancel_claim` (refund path) | $0$ | Pending claim exists | Escrow balance restored; reserve storage strictly intact ($R_{t+1} = R_t$). Emits `ClmCncl`. |
//! | **Cancellation** | Program / claim cancellation | $0$ | Valid cancellation state | Unreleased funds returned; reserve storage strictly untouched ($R_{t+1} = R_t$). Zero reserve events. |
//! | **Repeated Failure** | Sequential failing calls | $0$ | Failing preconditions | Solvency preserved across all failures ($R_0 = R_1 = R_2$). No spurious events; subsequent valid operations execute cleanly. |

use crate::errors::ContractError;
use crate::DataKey;
use soroban_sdk::{symbol_short, Env, Symbol};

/// Basis points denominator (100% = 10,000 bps).
pub const BASIS_POINTS: i128 = 10_000;

/// Event symbol for insurance-reserve withdrawal audit events.
pub const INSURANCE_RESERVE_WITHDRAWN: Symbol = symbol_short!("InsRsvWd");

/// Splits `total_fee` into `(reserve_share, recipient_share)` using ceiling
/// division for the reserve so no sub-token dust is lost.
///
/// Invariant: `reserve_share + recipient_share == total_fee`.
pub fn split_fee_for_reserve(total_fee: i128, insurance_reserve_bps: u32) -> (i128, i128) {
    if insurance_reserve_bps == 0 || total_fee <= 0 {
        return (0, total_fee);
    }
    // Ceiling division: reserve_share = ceil(total_fee * bps / BASIS_POINTS)
    let bps = insurance_reserve_bps as i128;
    let numerator = total_fee
        .checked_mul(bps)
        .and_then(|n| n.checked_add(BASIS_POINTS - 1))
        .expect("Insurance reserve split overflow");
    let reserve_share = numerator / BASIS_POINTS;
    let recipient_share = total_fee - reserve_share;
    (reserve_share, recipient_share)
}

/// Accrues `amount` into the on-chain insurance reserve under `DataKey::InsuranceReserve`.
///
/// Ensures $R_{t+1} = R_t + \text{amount}$. Returns the updated reserve balance.
pub fn accrue_insurance_reserve(env: &Env, amount: i128) -> i128 {
    if amount <= 0 {
        return get_insurance_reserve_balance(env);
    }
    let current = get_insurance_reserve_balance(env);
    let next = current
        .checked_add(amount)
        .expect("Insurance reserve overflow");
    env.storage()
        .instance()
        .set(&DataKey::InsuranceReserve, &next);
    next
}

/// Reads the current insurance reserve balance in native token units.
///
/// Defaults to 0 if not yet initialized.
pub fn get_insurance_reserve_balance(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::InsuranceReserve)
        .unwrap_or(0)
}

/// Verifies that the insurance reserve strictly satisfies the non-negative
/// solvency invariant ($R \ge 0$).
///
/// Panics if the stored reserve is negative (which should be impossible under
/// enforced contract guards).
pub fn verify_reserve_solvency(env: &Env) -> i128 {
    let balance = get_insurance_reserve_balance(env);
    if balance < 0 {
        panic!("Solvency invariant violated: insurance reserve balance is negative");
    }
    balance
}

/// Debits `amount` from the insurance reserve after strictly validating solvency.
///
/// Enforces:
/// 1. `amount > 0` (non-positive debits rejected with `InvalidAmount`).
/// 2. `amount <= balance_before` (underfunded debits fail safely with `InsufficientInsuranceReserve`).
/// 3. Balance never transitions to negative, even transiently.
///
/// Returns `Ok((balance_before, balance_after))` on success.
pub fn debit_insurance_reserve(
    env: &Env,
    amount: i128,
) -> Result<(i128, i128), ContractError> {
    if amount <= 0 {
        return Err(ContractError::InvalidAmount);
    }
    let balance_before = get_insurance_reserve_balance(env);
    if amount > balance_before {
        return Err(ContractError::InsufficientInsuranceReserve);
    }
    let balance_after = balance_before - amount;
    env.storage()
        .instance()
        .set(&DataKey::InsuranceReserve, &balance_after);
    Ok((balance_before, balance_after))
}
