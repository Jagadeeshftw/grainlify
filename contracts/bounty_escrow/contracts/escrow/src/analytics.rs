//! Contract operation analytics and aggregate statistics.

use soroban_sdk::Env;
use crate::{monitoring, AggregateStats, DataKey, Escrow, EscrowStatus};

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


pub fn health_check(env: Env) -> monitoring::HealthStatus {
    monitoring::health_check(&env)
}



pub fn get_analytics(env: Env) -> monitoring::Analytics {
    monitoring::get_analytics(&env)
}



pub fn get_state_snapshot(env: Env) -> monitoring::StateSnapshot {
    monitoring::get_state_snapshot(&env)
}



/// Aggregate totals across all escrows grouped by status.
///
/// Iterates the full `EscrowIndex` in a single pass, grouping counts and
/// amounts by terminal/active state. Escrows in `PartiallyRefunded` are
/// considered **active** (still held by the contract) and contribute to
/// the `_locked_` bucket using their current `remaining_amount`. Escrows
/// that have reached a terminal state (`Released` / `Refunded`) contribute
/// their original `amount` to their respective buckets. Draft escrows are
/// skipped entirely so operators can stage bounties without skewing totals.
///
/// # Complexity
/// O(n) on the number of escrows. Intended for periodic operator queries
/// and monitoring; not in hot transaction paths. Worst-case cost for a
/// 60-escrow index is validated in the gas CI thresholds suite.
///
/// # Return fields
/// - `total_locked` — funds still held in Locked or PartiallyRefunded escrows.
/// - `total_released` — original `amount` sum of all Released escrows.
/// - `total_refunded` — original `amount` sum of all Refunded escrows.
/// - The matching `count_*` fields give escrow cardinality per bucket.
pub fn get_aggregate_stats(env: Env) -> AggregateStats {
    let index: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowIndex)
        .unwrap_or(Vec::new(&env));

    let mut total_locked: i128 = 0;
    let mut total_released: i128 = 0;
    let mut total_refunded: i128 = 0;
    let mut count_locked: u32 = 0;
    let mut count_released: u32 = 0;
    let mut count_refunded: u32 = 0;

    for i in 0..index.len() {
        let bounty_id = index.get_unchecked(i);
        let escrow: Escrow = match env.storage().persistent().get(&DataKey::Escrow(bounty_id)) {
            Some(e) => e,
            None => continue,
        };
        match escrow.status {
            EscrowStatus::Locked => {
                total_locked = total_locked.checked_add(escrow.remaining_amount).unwrap();
                count_locked = count_locked.saturating_add(1);
            }
            EscrowStatus::PartiallyRefunded => {
                total_locked = total_locked.checked_add(escrow.remaining_amount).unwrap();
                count_locked = count_locked.saturating_add(1);
            }
            EscrowStatus::Released => {
                total_released = total_released.checked_add(escrow.amount).unwrap();
                count_released = count_released.saturating_add(1);
            }
            EscrowStatus::Refunded => {
                total_refunded = total_refunded.checked_add(escrow.amount).unwrap();
                count_refunded = count_refunded.saturating_add(1);
            }
            EscrowStatus::Draft => {
                // Drafts are staged but not funded; they do not contribute
                // to any aggregate bucket so totals remain reflective of
                // actual funds held/processed.
            }
        }
    }

    AggregateStats {
        total_locked,
        total_released,
        total_refunded,
        count_locked,
        count_released,
        count_refunded,
    }
}

