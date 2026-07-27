//! Reputation scoring helpers for program-escrow.
//!
//! The on-chain [`ProgramReputation`](crate::ProgramReputation) snapshot combines
//! schedule completion and **value-weighted** payout fulfillment. Raw payout
//! counts are exposed for analytics but do not drive `overall_score_bps`.
//!
//! # Dust payout gaming
//!
//! Attackers may spam minimum-size `single_payout` calls to inflate `total_payouts`
//! cheaply. Value-based fulfillment prevents dust from raising `overall_score_bps`
//! when meaningful funds remain locked. Activity that counts toward
//! `qualified_payout_count` requires amounts at or above
//! [`REPUTATION_MIN_QUALIFYING_PAYOUT_AMOUNT`].

/// Smallest payout amount (token base units) treated as a full reputation activity event.
pub const REPUTATION_MIN_QUALIFYING_PAYOUT_AMOUNT: i128 = 1_000;

/// Reference payout size used in tests and documentation for “typical” prize amounts.
pub const REPUTATION_TYPICAL_PAYOUT_AMOUNT: i128 = 10_000;

/// Minimum viable positive payout accepted by `single_payout` validation.
pub const REPUTATION_DUST_PAYOUT_AMOUNT: i128 = 1;

const BPS_SCALE: u32 = 10_000;
const COMPLETION_WEIGHT_PERCENT: u64 = 60;
const PAYOUT_FULFILLMENT_WEIGHT_PERCENT: u64 = 40;

/// Sum payout amounts from history (saturating).
pub fn total_funds_distributed_from_amounts<I>(amounts: I) -> i128
where
    I: Iterator<Item = i128>,
{
    amounts.fold(0i128, |acc, amount| acc.saturating_add(amount))
}

/// Count payouts whose amount meets the qualifying floor for activity metrics.
pub fn count_qualified_payouts<I>(amounts: I) -> u32
where
    I: Iterator<Item = i128>,
{
    amounts
        .filter(|amount| *amount >= REPUTATION_MIN_QUALIFYING_PAYOUT_AMOUNT)
        .count() as u32
}

/// Completion rate in basis points; defaults to perfect when no schedules exist.
pub fn completion_rate_bps(completed_releases: u32, total_scheduled: u32) -> u32 {
    if total_scheduled == 0 {
        BPS_SCALE
    } else {
        let rate = (completed_releases as u64)
            .saturating_mul(BPS_SCALE as u64)
            .saturating_div(total_scheduled as u64);
        (rate.min(BPS_SCALE as u64)) as u32
    }
}

/// Distributed / locked ratio in basis points; defaults to perfect when nothing is locked.
pub fn payout_fulfillment_rate_bps(total_funds_distributed: i128, total_funds_locked: i128) -> u32 {
    if total_funds_locked == 0 {
        BPS_SCALE
    } else {
        let rate = total_funds_distributed
            .saturating_mul(BPS_SCALE as i128)
            .saturating_div(total_funds_locked);
        (rate.min(BPS_SCALE as i128)) as u32
    }
}

/// Weighted overall reputation score; any overdue schedule forces zero.
pub fn overall_score_bps(
    completion_rate_bps: u32,
    payout_fulfillment_rate_bps: u32,
    overdue_releases: u32,
) -> u32 {
    if overdue_releases > 0 {
        0
    } else {
        let weighted = (completion_rate_bps as u64)
            .saturating_mul(COMPLETION_WEIGHT_PERCENT)
            .saturating_add(
                (payout_fulfillment_rate_bps as u64).saturating_mul(PAYOUT_FULFILLMENT_WEIGHT_PERCENT),
            )
            .saturating_div(100);
        (weighted.min(BPS_SCALE as u64)) as u32
    }
}

/// Pure benchmark helper: overall scores after `payout_count` dust vs typical payouts
/// against the same locked pool (`payout_count * typical_amount`).
pub fn benchmark_overall_scores_dust_vs_typical(payout_count: u32) -> (u32, u32) {
    let locked = (payout_count as i128).saturating_mul(REPUTATION_TYPICAL_PAYOUT_AMOUNT);
    let dust_distributed = (payout_count as i128).saturating_mul(REPUTATION_DUST_PAYOUT_AMOUNT);
    let typical_distributed = locked;

    let dust_fulfillment = payout_fulfillment_rate_bps(dust_distributed, locked);
    let typical_fulfillment = payout_fulfillment_rate_bps(typical_distributed, locked);

    let dust_overall = overall_score_bps(BPS_SCALE, dust_fulfillment, 0);
    let typical_overall = overall_score_bps(BPS_SCALE, typical_fulfillment, 0);
    (dust_overall, typical_overall)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn qualified_payout_count_ignores_dust() {
        let amounts = [1i128, 2, 999, 1_000, 10_000];
        assert_eq!(count_qualified_payouts(amounts.into_iter()), 2);
    }

    #[test]
    fn benchmark_favors_typical_over_dust() {
        let (dust, typical) = benchmark_overall_scores_dust_vs_typical(50);
        assert!(typical > dust);
        assert!(dust < 7_000);
        assert_eq!(typical, BPS_SCALE);
    }
}
