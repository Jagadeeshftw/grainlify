#![allow(dead_code)]

use soroban_sdk::{panic_with_error, vec, Address, Env, IntoVal, Symbol, Val};

/// Default maximum gross-to-net multiplier for router quotes.
///
/// Expressed in basis points over `BASIS_POINTS` (i.e. `15_000` == 1.5x).
/// This bounds the acceptable return from `router.quote(token, net_amount)` so
/// a malicious or misconfigured router cannot drain the program by returning
/// an implausibly large gross amount.
pub const DEFAULT_MAX_FOT_MULTIPLIER_BPS: u32 = 15_000;

/// Maximum configurable multiplier, in basis points, allowed via `set_fot_router`.
///
/// This caps the bound at 10x the intended net amount. Any higher would let
/// a compromised admin set a multiplier large enough to defeat the sanity check.
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
/// # Panics
/// - If the router call fails or returns a non-positive amount
/// - If the router quote exceeds the configured maximum multiplier
/// - On arithmetic overflow during the bound or slippage calculations
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

    if gross <= 0 {
        panic!("FoT router returned non-positive amount");
    }

    // Reject implausibly large gross amounts before any further arithmetic.
    // This guards against a compromised router inflating the gross quote.
    let max_allowed = net_amount
        .checked_mul(router.max_fot_multiplier_bps as i128)
        .expect("FoT routing: bound calculation overflow")
        .checked_div(crate::BASIS_POINTS)
        .expect("FoT routing: bound calculation overflow");

    if gross > max_allowed {
        panic_with_error!(&env, &crate::errors::ContractError::FotRouterQuoteExceeded);
    }

    if router.slippage_bps == 0 {
        return gross;
    }

    // Apply slippage tolerance
    let multiplier = crate::BASIS_POINTS + router.slippage_bps as i128;
    let adjusted = gross
        .checked_mul(multiplier)
        .expect("FoT routing: slippage calculation overflow")
        .checked_div(crate::BASIS_POINTS)
        .expect("FoT routing: slippage calculation overflow");

    if adjusted <= 0 {
        panic!("FoT routing: adjusted amount is zero or negative");
    }

    adjusted
}
