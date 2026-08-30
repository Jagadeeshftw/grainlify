//! # Fee-on-Transfer (FoT) Routing — Liability-Invariant Design
//!
//! This module implements the router gross-up used when the escorted token is a
//! fee-on-transfer (deflationary) asset, and records the *liability invariant*
//! that every supported fee scenario must preserve. See
//! `test_fot_routing.rs` for the end-to-end proof across deposit, payout and
//! refund.
//!
//! ## Design decision (issue #1721): who bears each fee
//!
//! Fee-on-transfer tokens deduct a fee on every transfer. The escrow cannot opt
//! out, so the accounting model must decide who bears that fee in each direction
//! and how the contract discovers the amount it actually received:
//!
//! | Direction | Fee borne by | How the received amount is discovered |
//! |---|---|---|
//! | **Deposit** (funder → escrow) | The funder | The escrow credits liability only up to the value it actually holds. Because the inbound transfer happens *before* the `lock_*` call, the contract cannot observe a balance delta inside the call; instead it enforces the supported boundary — a lock is only credited if the on-chain balance covers it (see `lock_program_funds_v2`'s FoT shortfall guard), i.e. the funder locks the **received** amount. A shortfall is rejected, never silently credited, so `remaining_balance` can never exceed the on-chain balance. |
//! | **Payout** (escrow → beneficiary) | The escrow pool (`remaining_balance`) | The beneficiary must always receive the intended net. `apply_fot_router` grosses the transfer up by the router's `quote`, the escrow debits the **full gross** from `remaining_balance`, and the FoT fee (`gross − net`) is burned by the token. Liability and on-chain balance drop by the same gross, so the invariant holds. |
//! | **Refund** (escrow → funder) | The escrow pool, same as payout | A return of unspent liability is structurally a payout (e.g. `single_payout` to the funder), so the same gross-up applies. The funder receives (`≥`) its outstanding liability, and again liability and balance drop by the same gross. |
//!
//! ## Liability invariant
//!
//! At every ledger state transition the escrow must never owe more than it holds:
//!
//! ```text
//! remaining_balance + insurance_reserve == on_chain_token_balance
//! ```
//!
//! (`insurance_reserve` is zero unless protocol fees carve a reserve aside.) The
//! FoT fee that is burned by the token is charged — via the gross-up — against
//! the pool *before* the outgoing transfer, so it is never financed by a future
//! promise (`remaining_balance`). The invariant is asserted in
//! `test_fot_routing.rs::test_end_to_end_liability_invariant_deposit_payout_refund`
//! after every leg.
//!
//! ## Impossible fee configurations
//!
//! A fee rate ≥ 100% (≥ 10_000 bps) cannot be routed: the gross-up divisor
//! `10_000 − fee` is zero or negative, and a router that cannot quote such a
//! configuration signals it by returning a non-positive quote. `apply_fot_router`
//! turns every such impossible configuration into the typed error
//! [`crate::errors::ContractError::FotRoutingFailed`] rather than a generic
//! panic. Bound violations on an otherwise-valid quote keep the dedicated
//! [`crate::errors::ContractError::FotRouterQuoteExceeded`] error.

// Module-level allow: FoT routing helpers are part of the fee-on-transfer feature
// surface. Not all entrypoints are wired yet; a single module-level annotation
// keeps the rationale auditable instead of scattering per-function allows.
#![allow(dead_code)]

use soroban_sdk::{panic_with_error, vec, Address, Env, IntoVal, Symbol, Val};

/// Default maximum gross-to-net multiplier for router quotes.
///
/// Expressed in basis points over `BASIS_POINTS` (i.e. `15_000` == 1.5x).
/// This bounds the acceptable return from `router.quote(token, net_amount)` so
/// a malicious or misconfigured router cannot drain the program by returning
/// an implausibly large gross amount.
#[allow(dead_code)] // Intended for external integration and future configuration commands
pub const DEFAULT_MAX_FOT_MULTIPLIER_BPS: u32 = 15_000;

/// Maximum configurable multiplier, in basis points, allowed via `set_fot_router`.
///
/// This caps the bound at 10x the intended net amount. Any higher would let
/// a compromised admin set a multiplier large enough to defeat the sanity check.
#[allow(dead_code)] // Intended for external integration and future configuration commands
pub const MAX_FOT_MULTIPLIER_BPS: u32 = 100_000;

/// Apply FoT routing to compute the gross transfer amount.
///
/// If `fot_router` is `None`, returns `net_amount` unchanged (no routing).
/// Otherwise calls `router_contract.quote(token, net_amount)` and applies
/// the configured slippage tolerance.
///
/// The router contract must expose a `quote(Address, i128) -> i128` function
/// that returns the gross amount needed to deliver `net_amount` to the
/// recipient after fee-on-transfer deductions.
///
/// # Upper-bound sanity check
///
/// The raw `gross` returned by the router is bounded by
/// `net_amount * max_fot_multiplier_bps / BASIS_POINTS`. This prevents a
/// malicious or misconfigured router from returning an arbitrarily large
/// gross amount, which could otherwise be used to drain the program's
/// `remaining_balance` under the guise of fee-on-transfer compensation.
/// If the router quote exceeds the configured bound, the function aborts
/// with [`crate::errors::ContractError::FotRouterQuoteExceeded`].
///
/// Slippage is applied as:
/// `adjusted = gross * (BASIS_POINTS + slippage_bps) / BASIS_POINTS`
///
/// # Typed errors (issue #1721)
///
/// Impossible fee configurations — a non-positive router quote, arithmetic
/// overflow while computing the bound or the slippage-adjusted gross, or a
/// slippage adjustment that collapses to zero/negative — abort with the typed
/// [`crate::errors::ContractError::FotRoutingFailed`] instead of a generic
/// `panic!`. This is what makes "impossible fee configurations return a typed
/// error" enforceable by callers (see `test_fot_routing.rs`).
///
/// # Panics
/// - With [`crate::errors::ContractError::FotRoutingFailed`] if the router
///   returns a non-positive amount or the bound/slippage arithmetic overflows
/// - With [`crate::errors::ContractError::FotRouterQuoteExceeded`] if the
///   router quote exceeds the configured maximum multiplier
pub fn apply_fot_router(
    env: &Env,
    token_address: &Address,
    net_amount: i128,
    fot_router: &crate::OptionalFotRouter,
) -> i128 {
    let router = match fot_router {
        crate::OptionalFotRouter::Some(r) => r,
        crate::OptionalFotRouter::None => return net_amount,
    };

    if net_amount <= 0 {
        return net_amount;
    }

    // Build args as Vec<Val> for the cross-contract call
    let token_val: Val = token_address.clone().into_val(env);
    let amount_val: Val = net_amount.into_val(env);
    let args = vec![&env, token_val, amount_val];

    let gross: i128 = env.invoke_contract(
        &router.router_contract,
        &Symbol::new(env, "quote"),
        args,
    );

    // A non-positive gross means the configured fee cannot be routed (e.g. a
    // fee >= 100% makes the gross-up divisor non-positive). Reject with the
    // typed error rather than a generic panic so callers can enforce the
    // "impossible fee configurations return a typed error" contract.
    if gross <= 0 {
        panic_with_error!(env, &crate::errors::ContractError::FotRoutingFailed);
    }

    // Reject implausibly large gross amounts before any further arithmetic.
    // This guards against a compromised router inflating the gross quote.
    let max_allowed = match net_amount
        .checked_mul(router.max_fot_multiplier_bps as i128)
        .and_then(|n| n.checked_div(crate::BASIS_POINTS))
    {
        Some(v) => v,
        None => panic_with_error!(env, &crate::errors::ContractError::FotRoutingFailed),
    };

    if gross > max_allowed {
        panic_with_error!(&env, &crate::errors::ContractError::FotRouterQuoteExceeded);
    }

    if router.slippage_bps == 0 {
        return gross;
    }

    // Apply slippage tolerance
    let multiplier = crate::BASIS_POINTS + router.slippage_bps as i128;
    let adjusted = match gross.checked_mul(multiplier).and_then(|n| n.checked_div(crate::BASIS_POINTS))
    {
        Some(v) => v,
        None => panic_with_error!(env, &crate::errors::ContractError::FotRoutingFailed),
    };

    if adjusted <= 0 {
        panic_with_error!(env, &crate::errors::ContractError::FotRoutingFailed);
    }

    adjusted
}