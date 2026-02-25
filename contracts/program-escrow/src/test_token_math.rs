#![cfg(test)]

//! Tests for the `token_math` module: fee calculation with floor rounding,
//! amount splitting invariant, decimal scaling, and base-unit conversion.

use crate::token_math;

// ===========================================================================
// 1. calculate_fee — basic behaviour
// ===========================================================================

#[test]
fn fee_zero_rate_returns_zero() {
    assert_eq!(token_math::calculate_fee(1_000_000, 0), 0);
}

#[test]
fn fee_zero_amount_returns_zero() {
    assert_eq!(token_math::calculate_fee(0, 500), 0);
}

#[test]
fn fee_exact_division() {
    // 10_000 * 500 / 10_000 = 500
    assert_eq!(token_math::calculate_fee(10_000, 500), 500);
}

#[test]
fn fee_floor_rounds_down() {
    // 999 * 100 / 10_000 = 9.99 → floor = 9
    assert_eq!(token_math::calculate_fee(999, 100), 9);
}

#[test]
fn fee_one_basis_point() {
    // 10_000 * 1 / 10_000 = 1
    assert_eq!(token_math::calculate_fee(10_000, 1), 1);
}

#[test]
fn fee_max_rate() {
    // 1_000 * 5_000 / 10_000 = 500 (50%)
    assert_eq!(
        token_math::calculate_fee(1_000, token_math::MAX_FEE_RATE),
        500
    );
}

#[test]
fn fee_single_unit_amount() {
    // 1 * 100 / 10_000 = 0.01 → floor = 0
    assert_eq!(token_math::calculate_fee(1, 100), 0);
}

// ===========================================================================
// 2. calculate_fee — multiple decimal scenarios
// ===========================================================================

#[test]
fn fee_xlm_7_decimals() {
    // 100 XLM = 100_0000000 stroops, 2% fee (200 bp)
    let amount = 100_0000000_i128;
    let fee = token_math::calculate_fee(amount, 200);
    assert_eq!(fee, 2_0000000); // 2 XLM
}

#[test]
fn fee_usdc_6_decimals() {
    // 100 USDC = 100_000_000 (6 decimals), 2% fee (200 bp)
    let amount = 100_000_000_i128;
    let fee = token_math::calculate_fee(amount, 200);
    assert_eq!(fee, 2_000_000); // 2 USDC
}

#[test]
fn fee_low_decimal_token_2_decimals() {
    // 100 tokens = 10_000 (2 decimals), 3% fee (300 bp)
    let amount = 10_000_i128;
    let fee = token_math::calculate_fee(amount, 300);
    assert_eq!(fee, 300); // 3 tokens
}

#[test]
fn fee_small_amount_high_decimals_floors_correctly() {
    // 1 stroop (smallest XLM unit), 1% fee
    let fee = token_math::calculate_fee(1, 100);
    assert_eq!(fee, 0); // too small, floors to 0
}

// ===========================================================================
// 3. split_amount — invariant: fee + net == amount
// ===========================================================================

#[test]
fn split_invariant_exact() {
    let amount = 10_000_i128;
    let (fee, net) = token_math::split_amount(amount, 500);
    assert_eq!(fee + net, amount);
    assert_eq!(fee, 500);
    assert_eq!(net, 9_500);
}

#[test]
fn split_invariant_with_remainder() {
    // 999 * 100 / 10_000 = 9 (floor). net = 990.
    let amount = 999_i128;
    let (fee, net) = token_math::split_amount(amount, 100);
    assert_eq!(fee + net, amount);
    assert_eq!(fee, 9);
    assert_eq!(net, 990);
}

#[test]
fn split_invariant_zero_fee() {
    let amount = 5_000_i128;
    let (fee, net) = token_math::split_amount(amount, 0);
    assert_eq!(fee, 0);
    assert_eq!(net, amount);
    assert_eq!(fee + net, amount);
}

#[test]
fn split_invariant_max_rate() {
    let amount = 1_001_i128;
    let (fee, net) = token_math::split_amount(amount, token_math::MAX_FEE_RATE);
    assert_eq!(fee + net, amount);
    // 1001 * 5000 / 10000 = 500 (floor)
    assert_eq!(fee, 500);
    assert_eq!(net, 501);
}

#[test]
fn split_invariant_prime_amount() {
    let amount = 997_i128;
    let (fee, net) = token_math::split_amount(amount, 333);
    assert_eq!(fee + net, amount);
}

#[test]
fn split_invariant_large_amount() {
    let amount = 1_000_000_000_0000000_i128; // 1 billion XLM in stroops
    let (fee, net) = token_math::split_amount(amount, 250);
    assert_eq!(fee + net, amount);
}

// ===========================================================================
// 4. scale_amount — decimal conversion
// ===========================================================================

#[test]
fn scale_same_decimals_is_identity() {
    assert_eq!(token_math::scale_amount(12345, 7, 7), Some(12345));
}

#[test]
fn scale_up_6_to_7() {
    // 1_000_000 (6 dec) → 10_000_000 (7 dec)
    assert_eq!(token_math::scale_amount(1_000_000, 6, 7), Some(10_000_000));
}

#[test]
fn scale_down_7_to_6() {
    // 10_000_005 (7 dec) → 1_000_000 (6 dec), floor
    assert_eq!(token_math::scale_amount(10_000_005, 7, 6), Some(1_000_000));
}

#[test]
fn scale_down_floors() {
    // 19 (7 dec) → 1 (6 dec), floor of 1.9
    assert_eq!(token_math::scale_amount(19, 7, 6), Some(1));
}

#[test]
fn scale_down_sub_unit_floors_to_zero() {
    // 9 (7 dec) → 0 (6 dec), 0.9 floors to 0
    assert_eq!(token_math::scale_amount(9, 7, 6), Some(0));
}

#[test]
fn scale_large_gap() {
    // 1 (0 dec) → 10_000_000 (7 dec)
    assert_eq!(token_math::scale_amount(1, 0, 7), Some(10_000_000));
}

#[test]
fn scale_zero_amount() {
    assert_eq!(token_math::scale_amount(0, 6, 7), Some(0));
}

// ===========================================================================
// 5. to_base_units
// ===========================================================================

#[test]
fn to_base_units_xlm() {
    // 100 XLM → 100_0000000 stroops (7 decimals)
    assert_eq!(token_math::to_base_units(100, 7), Some(1_000_000_000));
}

#[test]
fn to_base_units_usdc() {
    // 50 USDC → 50_000_000 (6 decimals)
    assert_eq!(token_math::to_base_units(50, 6), Some(50_000_000));
}

#[test]
fn to_base_units_zero_decimals() {
    assert_eq!(token_math::to_base_units(42, 0), Some(42));
}

#[test]
fn to_base_units_zero_amount() {
    assert_eq!(token_math::to_base_units(0, 7), Some(0));
}

// ===========================================================================
// 6. Boundary / edge cases
// ===========================================================================

#[test]
fn fee_never_exceeds_amount() {
    // Even at max rate, fee ≤ amount
    for amount in [1_i128, 2, 3, 7, 99, 100, 999, 10_000, 1_000_000] {
        let fee = token_math::calculate_fee(amount, token_math::MAX_FEE_RATE);
        assert!(fee <= amount, "fee {} > amount {}", fee, amount);
    }
}

#[test]
fn split_net_never_negative() {
    for amount in [1_i128, 2, 3, 7, 99, 100, 999, 10_000] {
        let (fee, net) = token_math::split_amount(amount, token_math::MAX_FEE_RATE);
        assert!(net >= 0, "net {} negative for amount {}", net, amount);
        assert!(fee >= 0, "fee {} negative for amount {}", fee, amount);
    }
}

#[test]
fn fee_monotonic_with_amount() {
    let rate = 250_i128;
    let mut prev = 0_i128;
    for amount in (0..=10_000_i128).step_by(100) {
        let fee = token_math::calculate_fee(amount, rate);
        assert!(fee >= prev, "fee decreased at amount {}", amount);
        prev = fee;
    }
}
