//! Token decimal scaling and fee rounding helpers.
//!
//! ## Rounding Policy
//!
//! All fee calculations use **floor (round-down)** rounding. This means the
//! protocol never overcharges — any remainder from basis-point division stays
//! with the payer rather than being collected as fee. The invariant
//! `fee + net == gross` holds for every split.
//!
//! ## Token Decimals
//!
//! Stellar tokens can have different decimal places (e.g. 7 for XLM/stroops,
//! 6 for USDC). The helpers here convert between decimal scales using floor
//! rounding when scaling down (higher → lower precision).

/// Basis-point denominator (1 bp = 0.01%).
pub const BASIS_POINTS: i128 = 10_000;

/// Maximum allowed fee rate in basis points (50%).
pub const MAX_FEE_RATE: i128 = 5_000;

/// Calculate fee using floor rounding.
///
/// `fee = floor(amount * fee_rate / BASIS_POINTS)`
///
/// Returns 0 when `fee_rate` is 0 or on overflow.
pub fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
    if fee_rate == 0 {
        return 0;
    }
    amount
        .checked_mul(fee_rate)
        .and_then(|x| x.checked_div(BASIS_POINTS))
        .unwrap_or(0)
}

/// Split `amount` into `(fee, net)` where `fee + net == amount`.
///
/// Fee is floored; any remainder from division stays in `net`.
pub fn split_amount(amount: i128, fee_rate: i128) -> (i128, i128) {
    let fee = calculate_fee(amount, fee_rate);
    (fee, amount - fee)
}

/// Scale `amount` from `from_decimals` to `to_decimals`.
///
/// Uses floor rounding when scaling down. Returns `None` on overflow.
pub fn scale_amount(amount: i128, from_decimals: u32, to_decimals: u32) -> Option<i128> {
    if from_decimals == to_decimals {
        return Some(amount);
    }
    if to_decimals > from_decimals {
        let factor = 10_i128.checked_pow(to_decimals - from_decimals)?;
        amount.checked_mul(factor)
    } else {
        let factor = 10_i128.checked_pow(from_decimals - to_decimals)?;
        Some(amount / factor)
    }
}

/// Convert a human-readable amount to the token's smallest unit.
///
/// E.g. `to_base_units(100, 7)` → `1_000_000_000` (100 XLM in stroops).
/// Returns `None` on overflow.
pub fn to_base_units(amount: i128, decimals: u32) -> Option<i128> {
    let factor = 10_i128.checked_pow(decimals)?;
    amount.checked_mul(factor)
}
