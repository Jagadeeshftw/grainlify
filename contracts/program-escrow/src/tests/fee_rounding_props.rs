//! # Property-Based Tests for `FeeConfig` Rounding Invariants
//!
//! This module contains the **property-based test suite** that audits the
//! integrity of [`crate::FeeConfig`] rounding arithmetic across the full
//! production input domain.  Whereas the existing deterministic test suite
//! (`src/test_token_math.rs`, `src/test_fee_on_transfer.rs`) covers dozens of
//! hand-picked boundary cases, the property-based suite below uses
//! [`proptest`] to exercise **at least 100 000 pseudorandom inputs** per
//! invariant, with automatic counterexample shrinking.
//!
//! ## Invariants under test
//!
//! 1. **No-dust-loss invariant** —
//!    `compute_fee(amount, fee_rate) + net_payout(amount, fee_rate) == amount`
//!    must hold for every legal `(amount, fee_rate)` pair, meaning the
//!    protocol never silently drops a single base unit of value.
//!
//! 2. **Monotonicity invariant** — for any fixed `amount > 0`,
//!    `compute_fee(amount, r1) <= compute_fee(amount, r2)` whenever
//!    `r1 <= r2`.  The fee function is non-decreasing in the rate.
//!
//! 3. **Idempotency invariant (purity)** —
//!    `compute_fee(a, r) == compute_fee(a, r)` and
//!    `net_payout(a, r) == net_payout(a, r)` — repeated evaluation of the
//!    pure math must produce the identical result with no hidden state.
//!
//! ## Why `i128` rather than `u128`?
//!
//! The contract's [`crate::FeeConfig`] stores basis-point rates in
//! `i128` (Soroban's signed integer type, see
//! [`crate::token_math::BASIS_POINTS`]).  The user-supplied spec referenced
//! `u128::MAX` as shorthand for "the largest representable amount", but
//! substituting `i128::MAX` keeps the property tests aligned with what the
//! production code actually accepts at the runtime boundary.
//!
//! ## Safe input envelope
//!
//! [`crate::token_math::calculate_fee`] uses `checked_mul` and panics on
//! overflow.  When the test runner's `RUST_BACKTRACE=1` is set, the panic
//! is a clean assertion failure, but to avoid wasting cycles on inputs that
//! are guaranteed to panic the property strategies are constrained so that
//! `amount * fee_rate / 10_000` is always within `i128::MAX`.
//!
//! Concretely: `safe_amount = 0..=(i128::MAX / (MAX_FEE_RATE + 1))`
//! guarantees `amount * MAX_FEE_RATE <= i128::MAX`.

#![cfg(test)]

use proptest::prelude::*;

use crate::token_math;

// =============================================================================
// Constants mirroring the production contract's published limits
// =============================================================================

/// `MAX_FEE_RATE` constant from `contracts/program-escrow/src/lib.rs`.
/// At the contract level this is capped at `1_000` (10 %).
///
/// (`crate::token_math::MAX_FEE_RATE` is `5_000`, a wider utility-module
///  bound, but the contract clamps caller-supplied fees to `1_000`.)
const CONTRACT_MAX_FEE_RATE: i128 = 1_000;

// =============================================================================
// Pure helpers — the functions whose invariants we test
// ============================================================================= //

/// Pure percentage-basis fee.
///
/// Equivalent to [`crate::token_math::calculate_fee`] but re-exported here
/// with a name matching the user-facing contract vocabulary so the
/// invariants read naturally.
///
/// @notice    Returns `floor(amount * fee_rate / BASIS_POINTS)`.
/// @dev       Panics on `amount * fee_rate` overflow; callers should stay
///            within the `safe_amount` envelope defined below.
/// @param     amount      — gross amount in token base units (`>= 0`).
/// @param     fee_rate    — basis-point rate in `0..=CONTRACT_MAX_FEE_RATE`.
/// @return    fee (floor-rounded) in token base units.
#[inline]
fn compute_fee(amount: i128, fee_rate: i128) -> i128 {
    token_math::calculate_fee(amount, fee_rate)
}

/// Pure net-payout after deduction of [`compute_fee`].
///
/// @notice    `net_payout = amount − compute_fee(amount, fee_rate)`.
/// @dev       Defined by composition with `compute_fee` rather than from
///            [`crate::token_math::split_amount`] directly so that the
///            round-trip property `compute_fee + net_payout == amount`
///            remains an **algebraic identity** and the more meaningful
///            `token_math::split_amount`-based audit runs as a separate
///            property (`prop_no_dust_loss_invariant_split_semantics`).
/// @param     amount      — gross amount in token base units.
/// @param     fee_rate    — basis-point rate applied to compute the fee.
/// @return    net payout  — the remainder after rounding the fee down.
#[inline]
fn net_payout(amount: i128, fee_rate: i128) -> i128 {
    amount
        .checked_sub(compute_fee(amount, fee_rate))
        .expect("no-dust-loss invariant violated: fee exceeds amount")
}

// =============================================================================
// Custom proptest strategies
// =============================================================================

/// Largest amount that does **not** overflow `amount * MAX_FEE_RATE`.
///
/// Because `fee = floor(amount * rate / 10_000)`, the intermediate product
/// `amount * MAX_FEE_RATE` must fit in [`i128`].  The smallest legal
/// boundary is `i128::MAX / (MAX_FEE_RATE + 1)`, giving one unit of headroom.
const SAFE_AMOUNT_MAX: i128 = i128::MAX / (CONTRACT_MAX_FEE_RATE + 1);

/// Strategy for any non-negative amount that will not overflow
/// `amount * CONTRACT_MAX_FEE_RATE`.
fn safe_amount() -> BoxedStrategy<i128> {
    (0_i128..=SAFE_AMOUNT_MAX).boxed()
}

/// Strategy for any rate in the production domain `0..=CONTRACT_MAX_FEE_RATE`.
fn safe_fee_rate() -> BoxedStrategy<i128> {
    (0_i128..=CONTRACT_MAX_FEE_RATE).boxed()
}

/// Common proptest configuration — 100 000 cases per invariant.
///
/// `100_000` matches the audit requirement (`at least 100,000 test
/// cases`).  Each property runs in its own `proptest! { … }` invocation
/// so the runner prints `cases passed: 100000 / 100000` per invariant
/// rather than aggregated.
fn proptest_config_100k() -> ProptestConfig {
    ProptestConfig {
        cases: 100_000,
        // The default fork behaviour is fine; the math is O(1) per input.
        ..ProptestConfig::default()
    }
}

// =============================================================================
// 1.  NO-DUST-LOSS INVARIANT
// =============================================================================
//
// IMPORTANT: this property is asserted against the **production helper**
// `token_math::split_amount` directly, rather than the local
// `compute_fee + net_payout` pair.  Otherwise the invariant
// `fee + (amount - fee) == amount` would be a structural tautology
// baked into the helper definition itself.

proptest! {
    #![proptest_config(proptest_config_100k())]

    /// **Property 1 — no-dust-loss invariant.**
    ///
    /// For every `(amount, fee_rate)` in the production domain, the
    /// `(fee, net)` pair returned by [`crate::token_math::split_amount`]
    /// must satisfy `fee + net == amount` **exactly**.  This is the
    /// non-tautological form of the no-dust-loss check: the test
    /// audits the production helper, not the helper's own algebra.
    ///
    /// @notice    Audits `golden-rule invariant #1` from `FeeConfig`.
    /// @dev       proptest will **shrink** any failing input to its
    ///            smallest reproducer, equivalent to documenting the
    ///            minimum failing input.
    ///            Equivalence to the user-spec wording: in production
    ///            `compute_fee(a, r)` is `token_math::calculate_fee(a, r)`
    ///            and `net_payout(a, r)` is `a - compute_fee(a, r)`;
    ///            therefore
    ///            `compute_fee(a, r) + net_payout(a, r) == a` holds
    ///            **iff** `split_amount(a, r).0 + split_amount(a, r).1 == a`,
    ///            which is precisely what this property asserts.
    #[test]
    fn prop_no_dust_loss_invariant_split_semantics(
        amount in safe_amount(),
        fee_rate in safe_fee_rate(),
    ) {
        // Drive the assertion off the production helper directly so
        // the test audits the actual contract code, not the
        // algebraic identity `a + (b - a) == b` of the test
        // helpers themselves.
        let (fee, net) = token_math::split_amount(amount, fee_rate);
        prop_assert_eq!(
            fee + net,
            amount,
            "no-dust-loss violated by token_math::split_amount: \
             fee({}) + net({}) != amount({})",
            fee, net, amount
        );
        // Belt-and-suspenders: neither component may be negative for
        // legal inputs.
        prop_assert!(fee >= 0,
            "fee went negative: fee={} for amount={}", fee, amount);
        prop_assert!(net >= 0,
            "net went negative: net={} for amount={}", net, amount);
        // And neither component may exceed the gross amount.
        prop_assert!(fee <= amount,
            "fee exceeded gross amount: fee={} > amount={}", fee, amount);
        prop_assert!(net <= amount,
            "net exceeded gross amount: net={} > amount={}", net, amount);
    }
}

// =============================================================================
// 2.  MONOTONICITY — fee does not decrease when the rate increases
// =============================================================================

proptest! {
    #![proptest_config(proptest_config_100k())]

    /// **Property 2 — monotonicity in `fee_rate`.**
    ///
    /// For any `amount > 0`, raising the rate must not lower the fee.
    /// Equal fees across adjacent rates are expected when the basis-point
    /// quotient floors (e.g. for `amount = 1, rate in 0..=1000`,
    /// `floor((1 * rate) / 10_000) == 0` for all `rate` in that range).
    /// The asserted inequality is therefore `fee_a <= fee_b`.
    ///
    /// @notice    Audits `golden-rule invariant #2` from `FeeConfig`.
    /// @dev       Strict monotonicity is *not* expected (floor rounding
    ///            ties at small amounts); the test asserts non-decreasing.
    #[test]
    fn prop_fee_monotonic_in_rate(
        amount in safe_amount(),
        rate_a  in safe_fee_rate(),
        rate_b  in safe_fee_rate(),
    ) {
        // Force rate_a <= rate_b without bias.
        let rate_lo = rate_a.min(rate_b);
        let rate_hi = rate_a.max(rate_b);

        let fee_lo = compute_fee(amount, rate_lo);
        let fee_hi = compute_fee(amount, rate_hi);

        prop_assert!(
            fee_lo <= fee_hi,
            "fee decreased with rate: fee(amount={}, r={})={} > \
             fee(amount={}, r={})={}",
            amount, rate_hi, fee_hi,
            amount, rate_lo, fee_lo
        );
    }
}

proptest! {
    #![proptest_config(proptest_config_100k())]

    /// **Property 2b — monotonicity in `amount`.**
    ///
    /// For any fixed `fee_rate > 0`, raising the amount must not lower
    /// the fee (a stronger sanity check on the floor-rounding scheme).
    #[test]
    fn prop_fee_monotonic_in_amount(
        rate     in 1_i128..=CONTRACT_MAX_FEE_RATE,
        amount_a in safe_amount(),
        amount_b in safe_amount(),
    ) {
        let amount_lo = amount_a.min(amount_b);
        let amount_hi = amount_a.max(amount_b);

        let fee_lo = compute_fee(amount_lo, rate);
        let fee_hi = compute_fee(amount_hi, rate);

        prop_assert!(
            fee_lo <= fee_hi,
            "fee decreased with amount: fee({}, r={})={} > \
             fee({}, r={})={}",
            amount_hi, rate, fee_hi,
            amount_lo, rate, fee_lo
        );
    }
}

// =============================================================================
// 3.  IDEMPOTENCY (determinism / purity) + INDEPENDENT-ORACLE CHECK
// =============================================================================
//
// **Why an oracle?**  Round-3 reviewer flagged that simply re-evaluating
// `compute_fee(a, r) == compute_fee(a, r)` cannot fail for a pure
// function — it is structurally true.  To make the "idempotency" /
// "purity" check meaningful at the algorithmic level we pair the
// production helper against an **independent oracle** that re-derives
// the floor-rounded fee from first principles:
//
// ```text
//     oracle(a, r) := floor((a * r) / 10_000)
// ```
//
// The oracle is implemented manually with integer arithmetic and is
// the textual specification of the on-chain rounding policy in
// `token_math`'s module-level docs.  Any divergence between
// `compute_fee` and the oracle indicates that the production helper
// no longer implements the documented rounding scheme.

/// Independent oracle for the floor-rounding fee.
///
/// @notice    Recomputes the percentage-based fee from first principles
///            using only integer arithmetic, with no call into the
///            production helper.
/// @dev       Returns `0` for `r = 0` — the short-circuit behaviour
///            documented in `token_math::calculate_fee`.
///            Uses `checked_mul` so a future widening of the safe-input
///            envelope fails fast here rather than silently wrapping.
///            The division by constant `10_000` cannot panic (positive
///            divisor) so it does not need to be `checked_div`.
/// @param     amount      — gross amount in token base units.
/// @param     fee_rate    — basis-point rate in `0..=CONTRACT_MAX_FEE_RATE`.
/// @return    fee (floor-rounded) in token base units.
#[inline]
fn floor_rounding_oracle(amount: i128, fee_rate: i128) -> i128 {
    if fee_rate == 0 {
        return 0;
    }
    // floor((a * r) / 10_000) using checked arithmetic:
    let numerator = amount
        .checked_mul(fee_rate)
        .expect("oracle envelope broken: amount * fee_rate overflows i128");
    numerator / 10_000
}

proptest! {
    #![proptest_config(proptest_config_100k())]

    /// **Property 3a — floor-rounding spec agreement.**
    ///
    /// The production helper `token_math::calculate_fee` must agree
    /// byte-for-byte with the floor-rounding specification re-derived
    /// from first principles in [`floor_rounding_oracle`].  This is the
    /// spec-based audit: the oracle's body text `floor((a * r) / 10_000)`
    /// IS the round-3 / round-4 documented contract, so a divergence
    /// here proves the production code no longer implements the
    /// spec.
    ///
    /// @notice    Audits `golden-rule invariant #3` from `FeeConfig`
    ///            (algorithmic correctness of the floor-rounding scheme).
    /// @dev       Failures here indicate that the production code has
    ///            diverged from the textual specification.  Use the
    ///            shrunk counterexample as the regression unit test.
    ///            The oracle uses `checked_mul` so it shares the
    ///            production helper's overflow-failure mode; both
    ///            surface envelope violations identically rather than
    ///            silently diverging.
    #[test]
    fn prop_compute_fee_matches_floor_rounding_spec(
        amount   in safe_amount(),
        fee_rate in safe_fee_rate(),
    ) {
        let fee_prod   = compute_fee(amount, fee_rate);
        let fee_oracle = floor_rounding_oracle(amount, fee_rate);
        prop_assert_eq!(
            fee_prod, fee_oracle,
            "token_math::calculate_fee diverged from the floor-rounding \
             spec: production={} oracle={} amount={} rate={}",
            fee_prod, fee_oracle, amount, fee_rate
        );
    }
}

proptest! {
    #![proptest_config(proptest_config_100k())]

    /// **Property 3b — round-trip composition.**
    ///
    /// The fee extracted from the *net payout* at the same rate cannot
    /// exceed the fee extracted from the gross amount.  This catches
    /// any future refactor that confuses gross/net arguments or breaks
    /// the floor-rounding scheme.
    ///
    /// @dev       `checked_sub` is used deliberately: a future change
    ///            that broke the no-dust-loss invariant would otherwise
    ///            silently wrap to a huge value, masking the root cause.
    ///            The `expect` message encodes the invariant for the
    ///            diagnostic output.
    #[test]
    fn prop_fee_split_roundtrip(
        amount   in safe_amount(),
        fee_rate in 1_i128..=CONTRACT_MAX_FEE_RATE,
    ) {
        let fee_first  = compute_fee(amount, fee_rate);
        let net_first  = amount
            .checked_sub(fee_first)
            .expect("no-dust-loss invariant violated in roundtrip");
        let fee_second = compute_fee(net_first, fee_rate);
        prop_assert!(fee_second <= fee_first,
            "second-pass fee exceeds first-pass: \
             first={} second={} amount={} rate={}",
            fee_first, fee_second, amount, fee_rate);
    }
}

proptest! {
    #![proptest_config(proptest_config_100k())]

    /// **Property 3c — `net_payout` pinned to `split_amount.1`.**
    ///
    /// `net_payout` is otherwise exercised only by the four
    /// deterministic `boundary_*` tests (~7 hand-picked inputs).  This
    /// property pins the helper to the second component of
    /// [`crate::token_math::split_amount`] across the same 100 000
    /// inputs as the rest of the suite, so a future refactor that
    /// silently changes how `net_payout` derives its value (or
    /// accidentally introduces a non-deterministic step) is caught
    /// on the next CI run.
    ///
    /// @notice    Sub-property **pinning** `net_payout` to the
    ///            production helper at full property-test coverage.
    /// @dev       Pins `net_payout(a, r) == split_amount(a, r).1` for
    ///            every `(a, r)` in the safe envelope.  By
    ///            construction both expressions are
    ///            `amount - token_math::calculate_fee(a, r)`, so the
    ///            property holds under the current implementation —
    ///            its purpose is regression-detection, not ground-truth
    ///            verification.  The reverse direction
    ///            (`compute_fee ↔ split_amount.0`) is covered
    ///            by `prop_compute_fee_matches_floor_rounding_spec`.
    #[test]
    fn prop_net_payout_pinned_to_split_amount(
        amount   in safe_amount(),
        fee_rate in safe_fee_rate(),
    ) {
        // Compute both sides once; the assertion message reuses
        // the cached values rather than re-deriving them via
        // another `net_payout` / `split_amount` call.
        let net_here = net_payout(amount, fee_rate);
        let (_, net_split) = token_math::split_amount(amount, fee_rate);
        prop_assert_eq!(net_here, net_split,
            "net_payout drifted from token_math::split_amount: \
             local={} production={} amount={} rate={}",
            net_here, net_split, amount, fee_rate);
    }
}

// =============================================================================
// 4.  ENVELOPE-TOP PROPERTY CHECK — bias sampling at the safe-input corner
// =============================================================================
//
// proptest explores the input domain by random sampling, but with very
// large domains (`i128::MAX / 1001` ≈ 1.7 × 10^35 values per axis),
// sampling 100 000 corner-pairs is statistically unlikely to hit the
// envelope top.  This property biases the sampler to `Just(amount)`
// for the envelope top while still varying the fee rate so we
// exercise the round-corners of the safe envelope at full coverage.

proptest! {
    #![proptest_config(proptest_config_100k())]

    /// **Property 4 — safe-envelope top holds across all rates.**
    ///
    /// Pins `amount = SAFE_AMOUNT_MAX` and varies `fee_rate` across
    /// the full `0..=CONTRACT_MAX_FEE_RATE` range, asserting the
    /// no-dust-loss, monotonicity, and oracle-agreement invariants
    /// at the safe-input envelope top.
    ///
    /// @notice    Companion to `boundary_safe_envelope_top_holds_invariant`,
    ///            but with **100 000 fee_rate samples** instead of
    ///            one.
    /// @dev       `Just(SAFE_AMOUNT_MAX)` is a degenerate strategy
    ///            that always returns the fixed envelope top; only
    ///            `fee_rate` is varied.  Sampling-orthogonal to the
    ///            other 100 000-case properties.
    #[test]
    fn prop_at_safe_envelope_top(
        fee_rate in safe_fee_rate(),
    ) {
        let amount = SAFE_AMOUNT_MAX;
        let (fee, net) = token_math::split_amount(amount, fee_rate);
        prop_assert_eq!(fee + net, amount,
            "no-dust-loss violated at envelope top: fee({}) + net({}) != amount({})",
            fee, net, amount);
        prop_assert_eq!(fee, floor_rounding_oracle(amount, fee_rate),
            "oracle mismatch at envelope top: production={} oracle={} rate={}",
            fee, floor_rounding_oracle(amount, fee_rate), fee_rate);
        prop_assert_eq!(net, net_payout(amount, fee_rate),
            "net_payout diverged from split_amount at envelope top: \
             local={} production={} rate={}",
            net, net_payout(amount, fee_rate), fee_rate);
    }
}

// =============================================================================
// 5.  BOUNDARY CASE HANDLERS (deterministic, independent of proptest)
// =============================================================================

/// `compute_fee(0, anything) == 0` and `0 + 0 == 0`.
#[test]
fn boundary_amount_zero_charges_no_fee() {
    for rate in 0..=CONTRACT_MAX_FEE_RATE {
        assert_eq!(compute_fee(0, rate), 0, "rate={}", rate);
        assert_eq!(net_payout(0, rate), 0, "rate={}", rate);
        assert_eq!(
            compute_fee(0, rate) + net_payout(0, rate),
            0,
            "rate={}", rate
        );
    }
}

/// `compute_fee(anything, 0) == 0` and `fee + net == anything`.
#[test]
fn boundary_rate_zero_charges_no_fee() {
    for amount in [0_i128, 1, 7, 100, 10_000, 1_000_000, SAFE_AMOUNT_MAX] {
        assert_eq!(compute_fee(amount, 0), 0, "amount={}", amount);
        assert_eq!(net_payout(amount, 0), amount, "amount={}", amount);
    }
}

/// `compute_fee(1, MAX_RATE) == 0` because floor rounds `0.1` to zero.
#[test]
fn boundary_one_amount_at_max_rate_rounds_to_zero() {
    // NOTE: This is a deliberate, documented consequence of the
    // floor-rounding policy.  The protocol never charges more than
    // `floor(gross * rate / 10_000)` and the smallest representable
    // amount at any contract-allowed rate is therefore zero dust.
    assert_eq!(compute_fee(1, CONTRACT_MAX_FEE_RATE), 0);
    assert_eq!(net_payout(1, CONTRACT_MAX_FEE_RATE), 1);
    assert_eq!(
        compute_fee(1, CONTRACT_MAX_FEE_RATE)
            + net_payout(1, CONTRACT_MAX_FEE_RATE),
        1
    );
}

/// At the very top of the safe envelope the no-dust-loss invariant
/// still holds and the fee itself is a fully-formed `floor(...)` result.
#[test]
fn boundary_safe_envelope_top_holds_invariant() {
    let amount = SAFE_AMOUNT_MAX;
    let rate   = CONTRACT_MAX_FEE_RATE;
    let fee    = compute_fee(amount, rate);
    let (fee_split, net_split) = token_math::split_amount(amount, rate);
    assert!(fee > 0, "fee at the envelope top must be positive: {}", fee);
    assert!(fee <= amount,
        "fee never exceeds amount: fee={} amount={}", fee, amount);
    assert_eq!(fee, fee_split,
        "fee-from-calculate_fee matches fee-from-split_amount at envelope top");
    assert_eq!(fee + net_split, amount,
        "no-dust-loss at envelope top");
}
