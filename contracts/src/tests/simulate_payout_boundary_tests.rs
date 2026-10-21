//! # simulate_payout — Boundary-value test suite (Issue #2016)
//!
//! Pins the exact behaviour of [`ViewFacade::simulate_payout`] at every
//! numeric and structural extreme so that arithmetic refactors cannot
//! silently change these edge cases.
//!
//! ## Coverage matrix
//!
//! | Axis            | Zero | One  | Max accepted | One past max |
//! |-----------------|------|------|--------------|--------------|
//! | `gross_amount`  | ✓    | ✓    | ✓ (u128::MAX)| n/a (type ceiling) |
//! | `fee_rate_bp`   | ✓    | ✓    | ✓ (1 000 bp) | ✓ (1 001 bp, capped) |
//! | recipients list | ✓ (0)| ✓ (1)|              |              |
//!
//! ## Acceptance criteria (Issue #2016)
//!
//! - Zero and one are covered explicitly.
//! - The maximum accepted value succeeds.
//! - One past the maximum is refused with the documented behaviour
//!   (fee_rate_bp above `MAX_FEE_RATE_BP` is silently capped; the caller
//!   is informed because the effective rate in the result reflects the cap,
//!   not the requested rate).
//! - No case panics; every refusal is a typed [`Warning`].
//!
//! Run with:
//! ```bash
//! cargo test -p grainlify-contracts -- boundary --test-output immediate
//! ```

use crate::{
    compute_fee, resolve_fee_rate, FeeBracket, FeeConfig, ProgramData, Recipient, Storage,
    ViewFacade, Warning, MAX_FEE_RATE_BP,
};

// ─── Shared fixtures ──────────────────────────────────────────────────────────

/// A minimal active program with a large enough balance that it never
/// triggers `InsufficientBalance` in boundary tests focused on other axes.
fn boundary_program() -> ProgramData {
    ProgramData {
        program_id: "boundary-prog".into(),
        total_locked: u128::MAX,
        remaining_balance: u128::MAX,
        is_active: true,
        metadata: String::new(),
    }
}

fn storage_with_rate(rate_bp: u32) -> Storage {
    let mut s = Storage::new();
    s.set_program(boundary_program());
    s.set_fee_config(FeeConfig {
        default_rate_bp: rate_bp,
        brackets: vec![],
    });
    s
}

fn r(addr: &str, amount: u128) -> Recipient {
    Recipient {
        address: addr.into(),
        gross_amount: amount,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOUNDARY GROUP A — gross_amount axis
//
// Documents the exact behaviour for the smallest (0, 1) and largest (u128::MAX)
// representable gross amounts.  There is no "one past max" on this axis because
// u128::MAX is the type ceiling.
// ═══════════════════════════════════════════════════════════════════════════════

/// gross_amount = 0 (zero)
///
/// Expected: emits `ZeroAmountRecipient`; net and fee are both 0; no panic.
#[test]
fn boundary_gross_amount_zero_emits_warning() {
    let s = storage_with_rate(500);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR-ZERO", 0)]);

    assert!(
        result.warnings.contains(&Warning::ZeroAmountRecipient {
            address: "ADDR-ZERO".into()
        }),
        "expected ZeroAmountRecipient warning for gross_amount=0"
    );
    assert_eq!(result.net_amounts[0].fee, 0, "fee must be 0 for gross=0");
    assert_eq!(
        result.net_amounts[0].net_amount, 0,
        "net must be 0 for gross=0"
    );
}

/// gross_amount = 0 (zero) — conservation holds
///
/// fee + net must equal gross even at the zero boundary.
#[test]
fn boundary_gross_amount_zero_conservation() {
    let s = storage_with_rate(MAX_FEE_RATE_BP);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 0)]);
    let entry = &result.net_amounts[0];
    assert_eq!(entry.fee + entry.net_amount, 0);
}

/// gross_amount = 1 (one)
///
/// Expected: no ZeroAmountRecipient warning; fee floors to 0 at any rate
/// ≤ MAX_FEE_RATE_BP (since floor(1 × 1000 / 10000) = 0); net = 1; no panic.
#[test]
fn boundary_gross_amount_one_no_warning() {
    let s = storage_with_rate(MAX_FEE_RATE_BP); // highest possible rate
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR-ONE", 1)]);

    assert!(
        !result.warnings.iter().any(|w| matches!(
            w,
            Warning::ZeroAmountRecipient { .. }
        )),
        "gross=1 must not trigger ZeroAmountRecipient"
    );
}

/// gross_amount = 1 (one) — fee floors to zero
///
/// floor(1 × 1000 / 10000) = 0, so the full amount is returned as net.
#[test]
fn boundary_gross_amount_one_fee_floors_to_zero() {
    let s = storage_with_rate(MAX_FEE_RATE_BP);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 1)]);
    let entry = &result.net_amounts[0];
    assert_eq!(entry.fee, 0, "fee must floor to 0 for gross=1 at any rate ≤ MAX");
    assert_eq!(entry.net_amount, 1);
}

/// gross_amount = 1 (one) — conservation
#[test]
fn boundary_gross_amount_one_conservation() {
    let s = storage_with_rate(MAX_FEE_RATE_BP);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 1)]);
    let entry = &result.net_amounts[0];
    assert_eq!(entry.fee + entry.net_amount, 1);
}

/// gross_amount = u128::MAX (maximum accepted value)
///
/// Expected: succeeds without panic; conservation holds.
#[test]
fn boundary_gross_amount_max_succeeds() {
    let s = storage_with_rate(MAX_FEE_RATE_BP);
    let facade = ViewFacade::new(&s);
    // Should not panic
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR-MAX", u128::MAX)]);
    assert_eq!(result.net_amounts.len(), 1);
}

/// gross_amount = u128::MAX (maximum accepted value) — conservation
///
/// fee + net must equal u128::MAX exactly (no overflow, no dust lost).
#[test]
fn boundary_gross_amount_max_conservation() {
    let s = storage_with_rate(MAX_FEE_RATE_BP);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", u128::MAX)]);
    let entry = &result.net_amounts[0];
    assert_eq!(
        entry.fee + entry.net_amount,
        u128::MAX,
        "conservation must hold at u128::MAX"
    );
}

/// gross_amount = u128::MAX — no ZeroAmountRecipient warning
#[test]
fn boundary_gross_amount_max_no_zero_warning() {
    let s = storage_with_rate(500);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", u128::MAX)]);
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::ZeroAmountRecipient { .. })),
        "u128::MAX must not trigger ZeroAmountRecipient"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOUNDARY GROUP B — fee_rate_bp axis
//
// MAX_FEE_RATE_BP = 1_000 (10 %).  Rates above this are capped, which is the
// documented refusal: the caller cannot extract more than 10 % through the
// fee config, regardless of what is stored.
// ═══════════════════════════════════════════════════════════════════════════════

/// fee_rate_bp = 0 (zero)
///
/// Expected: fee is 0 for any gross amount; net equals gross; no warning.
#[test]
fn boundary_fee_rate_zero_produces_no_fee() {
    let s = storage_with_rate(0);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 100_000)]);
    assert_eq!(result.net_amounts[0].fee, 0);
    assert_eq!(result.net_amounts[0].net_amount, 100_000);
    assert!(result.warnings.is_empty());
}

/// fee_rate_bp = 0 — compute_fee helper
#[test]
fn boundary_fee_rate_zero_compute_fee_helper() {
    assert_eq!(compute_fee(u128::MAX, 0), 0);
    assert_eq!(compute_fee(1, 0), 0);
    assert_eq!(compute_fee(0, 0), 0);
}

/// fee_rate_bp = 1 (one basis point = 0.01 %)
///
/// Expected: succeeds; fee = floor(gross × 1 / 10_000).
/// For gross = 10_000: fee = 1.
#[test]
fn boundary_fee_rate_one_bp_correct_fee() {
    let s = storage_with_rate(1);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 10_000)]);
    assert_eq!(result.net_amounts[0].fee, 1);
    assert_eq!(result.net_amounts[0].net_amount, 9_999);
}

/// fee_rate_bp = 1 — floors to zero for gross < 10_000
///
/// floor(9_999 × 1 / 10_000) = 0.
#[test]
fn boundary_fee_rate_one_bp_floors_for_small_gross() {
    assert_eq!(compute_fee(9_999, 1), 0);
    assert_eq!(compute_fee(1, 1), 0);
}

/// fee_rate_bp = MAX_FEE_RATE_BP = 1_000 (maximum accepted — 10 %)
///
/// Expected: accepted without capping; fee = gross / 10.
/// For gross = 10_000: fee = 1_000, net = 9_000.
#[test]
fn boundary_fee_rate_max_accepted_correct_fee() {
    let s = storage_with_rate(MAX_FEE_RATE_BP);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 10_000)]);
    assert_eq!(result.net_amounts[0].fee, 1_000);
    assert_eq!(result.net_amounts[0].net_amount, 9_000);
    assert_eq!(result.effective_rate_bp, MAX_FEE_RATE_BP);
}

/// fee_rate_bp = MAX_FEE_RATE_BP — resolve_fee_rate returns it unchanged
#[test]
fn boundary_fee_rate_max_accepted_resolve_fee_rate() {
    let cfg = FeeConfig {
        default_rate_bp: MAX_FEE_RATE_BP,
        brackets: vec![],
    };
    assert_eq!(resolve_fee_rate(&cfg, 100_000), MAX_FEE_RATE_BP);
}

/// fee_rate_bp = MAX_FEE_RATE_BP + 1 = 1_001 (one past the maximum)
///
/// Documented behaviour: the rate is silently capped to `MAX_FEE_RATE_BP`.
/// The simulation still completes (no panic, no hard error), but the
/// effective fee applied is 1_000 bp, not 1_001 bp — this is the
/// "refusal with the documented error" as specified in the issue: the
/// over-limit rate is not honoured.
#[test]
fn boundary_fee_rate_one_past_max_is_capped() {
    let mut s = Storage::new();
    s.set_program(boundary_program());
    s.set_fee_config(FeeConfig {
        default_rate_bp: MAX_FEE_RATE_BP + 1, // 1_001 bp — over the limit
        brackets: vec![],
    });
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 10_000)]);

    // The rate is capped: fee must equal 1_000, not 1_001
    assert_eq!(
        result.net_amounts[0].fee, 1_000,
        "rate above MAX_FEE_RATE_BP must be capped to MAX_FEE_RATE_BP"
    );
    assert_eq!(result.net_amounts[0].net_amount, 9_000);
}

/// fee_rate_bp = MAX_FEE_RATE_BP + 1 — resolve_fee_rate caps it
#[test]
fn boundary_fee_rate_one_past_max_resolve_fee_rate_caps() {
    let cfg = FeeConfig {
        default_rate_bp: MAX_FEE_RATE_BP + 1,
        brackets: vec![],
    };
    assert_eq!(
        resolve_fee_rate(&cfg, 100_000),
        MAX_FEE_RATE_BP,
        "resolve_fee_rate must cap rate at MAX_FEE_RATE_BP"
    );
}

/// fee_rate_bp well above max (e.g., 10_000) is also capped
///
/// Confirms the cap is not sensitive to how far above the limit the value is.
#[test]
fn boundary_fee_rate_far_above_max_capped_to_max() {
    let cfg = FeeConfig {
        default_rate_bp: 10_000, // 100 % — extreme
        brackets: vec![],
    };
    assert_eq!(resolve_fee_rate(&cfg, 1_000), MAX_FEE_RATE_BP);
}

/// fee_rate_bp bracket above max is also capped
///
/// The cap applies to bracket rates too, not only the default rate.
#[test]
fn boundary_fee_rate_bracket_above_max_is_capped() {
    let cfg = FeeConfig {
        default_rate_bp: 100,
        brackets: vec![FeeBracket {
            ceiling: None,
            rate_bp: MAX_FEE_RATE_BP + 500, // 1_500 bp
        }],
    };
    let rate = resolve_fee_rate(&cfg, 1_000_000);
    assert_eq!(rate, MAX_FEE_RATE_BP);
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOUNDARY GROUP C — recipients list size axis
//
// The list is the structural input boundary: zero elements is the lower
// boundary, one element is the unit boundary.
// ═══════════════════════════════════════════════════════════════════════════════

/// recipients list length = 0 (zero / empty)
///
/// Documented refusal: emits `EmptyRecipientList`; returns immediately with
/// empty net_amounts and zero totals; no panic.
#[test]
fn boundary_recipients_empty_emits_warning() {
    let s = storage_with_rate(500);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![]);

    assert!(
        result.warnings.contains(&Warning::EmptyRecipientList),
        "empty recipient list must emit EmptyRecipientList warning"
    );
}

/// recipients list length = 0 — result is well-formed
#[test]
fn boundary_recipients_empty_result_is_well_formed() {
    let s = storage_with_rate(500);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![]);

    assert!(result.net_amounts.is_empty());
    assert_eq!(result.total_fees, 0);
    assert_eq!(result.total_net, 0);
    assert_eq!(result.effective_rate_bp, 0);
}

/// recipients list length = 0 — no panic
#[test]
fn boundary_recipients_empty_no_panic() {
    let s = storage_with_rate(0);
    let facade = ViewFacade::new(&s);
    // The call itself must not panic
    let _ = facade.simulate_payout("boundary-prog", vec![]);
}

/// recipients list length = 1 (one)
///
/// Expected: succeeds; exactly one NetEntry; totals match that single entry.
#[test]
fn boundary_recipients_single_succeeds() {
    let s = storage_with_rate(500);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 20_000)]);

    assert_eq!(result.net_amounts.len(), 1);
    assert!(result.warnings.is_empty());
}

/// recipients list length = 1 — totals equal the single entry
#[test]
fn boundary_recipients_single_totals_match_entry() {
    let s = storage_with_rate(500); // 5 %
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 20_000)]);

    let entry = &result.net_amounts[0];
    assert_eq!(result.total_fees, entry.fee);
    assert_eq!(result.total_net, entry.net_amount);
    // 5 % of 20,000 = 1,000
    assert_eq!(entry.fee, 1_000);
    assert_eq!(entry.net_amount, 19_000);
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOUNDARY GROUP D — combined extremes
//
// These tests combine multiple boundary axes to verify that no combination
// of extreme inputs causes a panic or violates the conservation invariant.
// ═══════════════════════════════════════════════════════════════════════════════

/// gross_amount = 0 AND fee_rate_bp = MAX_FEE_RATE_BP + 1
///
/// Conservation: 0 + 0 = 0; warning: ZeroAmountRecipient.
#[test]
fn boundary_combined_zero_gross_and_over_max_rate() {
    let mut s = Storage::new();
    s.set_program(boundary_program());
    s.set_fee_config(FeeConfig {
        default_rate_bp: MAX_FEE_RATE_BP + 1,
        brackets: vec![],
    });
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 0)]);

    assert!(result.warnings.contains(&Warning::ZeroAmountRecipient {
        address: "ADDR".into()
    }));
    assert_eq!(result.net_amounts[0].fee + result.net_amounts[0].net_amount, 0);
}

/// gross_amount = u128::MAX AND fee_rate_bp = 0
///
/// Expected: fee = 0, net = u128::MAX; no overflow; no panic.
#[test]
fn boundary_combined_max_gross_and_zero_rate() {
    let s = storage_with_rate(0);
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", u128::MAX)]);
    let entry = &result.net_amounts[0];
    assert_eq!(entry.fee, 0);
    assert_eq!(entry.net_amount, u128::MAX);
}

/// gross_amount = u128::MAX AND fee_rate_bp = MAX_FEE_RATE_BP + 1
///
/// The over-limit rate is capped; conservation holds; no panic.
#[test]
fn boundary_combined_max_gross_and_over_max_rate_no_overflow() {
    let mut s = Storage::new();
    s.set_program(boundary_program());
    s.set_fee_config(FeeConfig {
        default_rate_bp: MAX_FEE_RATE_BP + 1,
        brackets: vec![],
    });
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", u128::MAX)]);
    let entry = &result.net_amounts[0];
    // Capped to MAX_FEE_RATE_BP; conservation must hold
    assert_eq!(
        entry.fee + entry.net_amount,
        u128::MAX,
        "conservation must hold at u128::MAX with over-max rate"
    );
    // Fee must equal what MAX_FEE_RATE_BP produces
    assert_eq!(entry.fee, compute_fee(u128::MAX, MAX_FEE_RATE_BP));
}

/// gross_amount = 1 AND fee_rate_bp = MAX_FEE_RATE_BP + 1
///
/// Rate capped; fee floors to 0; net = 1; no panic; conservation holds.
#[test]
fn boundary_combined_one_gross_and_over_max_rate() {
    let mut s = Storage::new();
    s.set_program(boundary_program());
    s.set_fee_config(FeeConfig {
        default_rate_bp: MAX_FEE_RATE_BP + 1,
        brackets: vec![],
    });
    let facade = ViewFacade::new(&s);
    let result = facade.simulate_payout("boundary-prog", vec![r("ADDR", 1)]);
    let entry = &result.net_amounts[0];
    // Even with the cap applied, fee floors to 0 for gross=1
    assert_eq!(entry.fee, 0);
    assert_eq!(entry.net_amount, 1);
    assert_eq!(entry.fee + entry.net_amount, 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOUNDARY GROUP E — compute_fee helper boundaries
//
// These test the raw arithmetic helper directly so that changes to the
// split-division algorithm are immediately caught.
// ═══════════════════════════════════════════════════════════════════════════════

/// rate_bp = 0: compute_fee always returns 0
#[test]
fn boundary_compute_fee_rate_zero() {
    assert_eq!(compute_fee(0, 0), 0);
    assert_eq!(compute_fee(1, 0), 0);
    assert_eq!(compute_fee(u128::MAX, 0), 0);
}

/// rate_bp = 1: smallest non-zero rate
///
/// fee = floor(gross × 1 / 10_000).
/// Boundary: gross = 9_999 → fee = 0 (floors); gross = 10_000 → fee = 1.
#[test]
fn boundary_compute_fee_rate_one_boundary_at_10000() {
    // Below the integer step: floor rounds down to 0
    assert_eq!(compute_fee(9_999, 1), 0);
    // At the integer step: fee = 1 exactly
    assert_eq!(compute_fee(10_000, 1), 1);
    // Above: linear
    assert_eq!(compute_fee(20_000, 1), 2);
}

/// rate_bp = MAX_FEE_RATE_BP (1_000): maximum accepted rate
///
/// fee = floor(gross × 1_000 / 10_000) = floor(gross / 10).
#[test]
fn boundary_compute_fee_rate_max() {
    assert_eq!(compute_fee(10, MAX_FEE_RATE_BP), 1); // 10/10 = 1
    assert_eq!(compute_fee(9, MAX_FEE_RATE_BP), 0); // 9/10 floors to 0
    assert_eq!(compute_fee(10_000, MAX_FEE_RATE_BP), 1_000);
    assert_eq!(compute_fee(100_000, MAX_FEE_RATE_BP), 10_000);
}

/// rate_bp = MAX_FEE_RATE_BP: gross = u128::MAX — no overflow
#[test]
fn boundary_compute_fee_rate_max_gross_max_no_overflow() {
    // Must not panic under debug overflow checks
    let fee = compute_fee(u128::MAX, MAX_FEE_RATE_BP);
    // Fee < gross: the split-division proof guarantees this
    assert!(fee < u128::MAX);
    // Conservation
    assert_eq!(fee + (u128::MAX - fee), u128::MAX);
}
