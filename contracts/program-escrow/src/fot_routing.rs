#![allow(dead_code)]

use soroban_sdk::{vec, Address, Env, IntoVal, Symbol, Val};

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
/// Slippage is applied as:
/// `adjusted = gross * (BASIS_POINTS + slippage_bps) / BASIS_POINTS`
///
/// # Panics
/// - If the router call fails or returns a non-positive amount
/// - On arithmetic overflow during slippage adjustment
pub fn apply_fot_router(
    env: &Env,
    token_address: &Address,
    net_amount: i128,
    fot_router: &Option<crate::FotRouter>,
) -> i128 {
    let router = match fot_router {
        Some(r) => r,
        None => return net_amount,
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
