//! Property tests for threshold-monitor window boundaries.
//!
//! `proptest` shrinks a failing timestamp/outcome sequence to the smallest
//! reproducer, which makes boundary regressions straightforward to diagnose.

use proptest::prelude::*;
use soroban_sdk::Env;

use crate::threshold_monitor::{self, ThresholdConfig};

const WINDOW_SECS: u64 = 10;
const FAILURE_THRESHOLD: u32 = 3;

fn boundary_offsets() -> impl Strategy<Value = Vec<(bool, u8)>> {
    prop::collection::vec((any::<bool>(), prop_oneof![Just(9), Just(10), Just(11)]), 0..24)
}

fn expected_breach(events: &[(bool, u8)]) -> bool {
    let mut window_start = 0_u64;
    let mut failures = 0_u32;

    for &(failed, offset) in events {
        let timestamp = offset as u64;
        if timestamp >= window_start + WINDOW_SECS {
            window_start = timestamp;
            failures = 0;
        }
        if failed {
            failures += 1;
        }
    }

    failures >= FAILURE_THRESHOLD
}

proptest! {
    /// Events at `end - 1` remain in the old window; events at `end` begin a
    /// new one. The breaker result must exactly match the current-window model.
    #[test]
    fn prop_boundary_sequences_open_only_when_current_window_exceeds_threshold(
        mut events in boundary_offsets(),
    ) {
        events.sort_by_key(|(_, offset)| *offset);

        let env = Env::default();
        env.ledger().with_mut(|ledger| ledger.timestamp = 0);
        threshold_monitor::init_threshold_monitor(&env);
        let mut config = ThresholdConfig::default();
        config.set_time_window_secs(WINDOW_SECS);
        config.set_failure_rate_threshold(FAILURE_THRESHOLD);
        threshold_monitor::set_threshold_config(&env, config).unwrap();

        for &(failed, offset) in &events {
            env.ledger().with_mut(|ledger| ledger.timestamp = offset as u64);
            if failed {
                threshold_monitor::record_operation_failure(&env);
            } else {
                threshold_monitor::record_operation_success(&env);
            }
        }

        let final_timestamp = events.last().map(|(_, offset)| *offset as u64).unwrap_or(0);
        env.ledger().with_mut(|ledger| ledger.timestamp = final_timestamp);
        prop_assert_eq!(
            threshold_monitor::check_thresholds(&env).is_err(),
            expected_breach(&events),
            "boundary sequence did not match the window-membership model: {:?}",
            events,
        );
    }
}
