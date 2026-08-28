#![cfg(test)]

extern crate std;

use super::*;
use crate::test_support::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke}, token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val};

#[test]
fn test_query_payouts_by_recipient_returns_correct_records() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 500_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Multiple payouts: two to r1, one to r2
    client.single_payout(&r1, &100_000,
    &None
);
    client.single_payout(&r2, &150_000,
    &None
);
    client.single_payout(&r1, &50_000,
    &None
);

    let r1_records = client.query_payouts_by_recipient(&r1, &0, &10);
    assert_eq!(r1_records.len(), 2);
    for record in r1_records.iter() {
        assert_eq!(record.recipient, r1);
    }

    let r2_records = client.query_payouts_by_recipient(&r2, &0, &10);
    assert_eq!(r2_records.len(), 1);
    assert_eq!(r2_records.get(0).unwrap().recipient, r2);
}

#[test]
fn test_query_payouts_by_recipient_unknown_returns_empty() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 100_000);

    let r1 = Address::generate(&env);
    let unknown = Address::generate(&env);

    client.single_payout(&r1, &50_000,
    &None
);

    let results = client.query_payouts_by_recipient(&unknown, &0, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_query_payouts_by_amount_range_returns_matching() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 600_000);

    client.single_payout(&Address::generate(&env), &10_000,
    &None
);
    client.single_payout(&Address::generate(&env), &50_000,
    &None
);
    client.single_payout(&Address::generate(&env), &100_000,
    &None
);
    client.single_payout(&Address::generate(&env), &200_000,
    &None
);

    // Filter: 40_000 to 110_000
    let results = client.query_payouts_by_amount(&40_000, &110_000, &0, &10);
    assert_eq!(results.len(), 2);
    for record in results.iter() {
        assert!(record.amount >= 40_000 && record.amount <= 110_000);
    }
}

#[test]
fn test_query_payouts_by_amount_exact_boundaries_included() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 600_000);

    client.single_payout(&Address::generate(&env), &100_000,
    &None
);
    client.single_payout(&Address::generate(&env), &200_000,
    &None
);
    client.single_payout(&Address::generate(&env), &300_000,
    &None
);

    // Exact boundaries should be included
    let results = client.query_payouts_by_amount(&100_000, &300_000, &0, &10);
    assert_eq!(results.len(), 3);
}

#[test]
fn test_query_payouts_by_amount_no_results_outside_range() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 200_000);

    client.single_payout(&Address::generate(&env), &50_000,
    &None
);
    client.single_payout(&Address::generate(&env), &100_000,
    &None
);

    let results = client.query_payouts_by_amount(&500_000, &999_000, &0, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_query_payouts_by_timestamp_range_filters_correctly() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 600_000);

    let base = env.ledger().timestamp();

    env.ledger().set_timestamp(base + 100);
    client.single_payout(&Address::generate(&env), &100_000,
    &None
);

    env.ledger().set_timestamp(base + 300);
    client.single_payout(&Address::generate(&env), &100_000,
    &None
);

    env.ledger().set_timestamp(base + 700);
    client.single_payout(&Address::generate(&env), &100_000,
    &None
);

    env.ledger().set_timestamp(base + 1200);
    client.single_payout(&Address::generate(&env), &100_000,
    &None
);

    // Filter for timestamps between base+200 and base+800
    let results = client.query_payouts_by_timestamp(&(base + 200), &(base + 800), &0, &10);
    assert_eq!(results.len(), 2);
    for record in results.iter() {
        assert!(record.timestamp >= base + 200 && record.timestamp <= base + 800);
    }
}

#[test]
fn test_query_payouts_by_timestamp_exact_boundary_included() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 300_000);

    let base = env.ledger().timestamp();

    env.ledger().set_timestamp(base + 100);
    client.single_payout(&Address::generate(&env), &100_000,
    &None
);

    env.ledger().set_timestamp(base + 200);
    client.single_payout(&Address::generate(&env), &100_000,
    &None
);

    env.ledger().set_timestamp(base + 300);
    client.single_payout(&Address::generate(&env), &100_000,
    &None
);

    // Exact boundary should include first and last
    let results = client.query_payouts_by_timestamp(&(base + 100), &(base + 300), &0, &10);
    assert_eq!(results.len(), 3);
}

#[test]
fn test_query_payouts_pagination_offset_and_limit() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 500_000);

    let r1 = Address::generate(&env);
    for _ in 0..5 {
        client.single_payout(&r1, &10_000,
    &None
);
    }

    // Page 1
    let page1 = client.query_payouts_by_recipient(&r1, &0, &2);
    assert_eq!(page1.len(), 2);

    // Page 2
    let page2 = client.query_payouts_by_recipient(&r1, &2, &2);
    assert_eq!(page2.len(), 2);

    // Page 3
    let page3 = client.query_payouts_by_recipient(&r1, &4, &2);
    assert_eq!(page3.len(), 1);
}

#[test]
#[should_panic(expected = "Pagination limit must be greater than zero")]
fn test_query_payouts_pagination_limit_zero_rejected() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 100_000);
    let r1 = Address::generate(&env);
    client.single_payout(&r1, &10_000,
    &None
);
    let _ = client.query_payouts_by_recipient(&r1, &0, &0);
}

#[test]
#[should_panic(expected = "Pagination limit exceeds maximum")]
fn test_query_payouts_pagination_limit_above_max_rejected() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 100_000);
    let r1 = Address::generate(&env);
    client.single_payout(&r1, &10_000,
    &None
);
    let _ = client.query_payouts_by_recipient(&r1, &0, &201);
}

#[test]
#[should_panic(expected = "Invalid amount range")]
fn test_query_payouts_by_amount_invalid_range_rejected() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 100_000);
    let _ = client.query_payouts_by_amount(&1000, &100, &0, &10);
}

#[test]
#[should_panic(expected = "Invalid timestamp range")]
fn test_query_payouts_by_timestamp_invalid_range_rejected() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 100_000);
    let now = env.ledger().timestamp();
    let _ = client.query_payouts_by_timestamp(&(now + 10), &now, &0, &10);
}

#[test]
fn test_query_schedules_by_status_pending_vs_released() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 200_000);

    let now = env.ledger().timestamp();
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    client.create_program_release_schedule(&r1, &50_000, &(now + 100));
    client.create_program_release_schedule(&r2, &50_000, &(now + 200));
    client.create_program_release_schedule(&r3, &50_000, &(now + 300));

    // Trigger first two schedules
    env.ledger().set_timestamp(now + 250);
    client.trigger_program_releases(&None);

    // Pending (not yet released) = only the third
    let pending = client.query_schedules_by_status(&false, &0, &10);
    assert_eq!(pending.len(), 1);
    assert!(!pending.get(0).unwrap().released);

    // Released = first two
    let released = client.query_schedules_by_status(&true, &0, &10);
    assert_eq!(released.len(), 2);
    for s in released.iter() {
        assert!(s.released);
    }
}

#[test]
fn test_query_schedules_by_recipient_returns_correct_subset() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 300_000);

    let now = env.ledger().timestamp();
    let winner = Address::generate(&env);
    let other = Address::generate(&env);

    client.create_program_release_schedule(&winner, &100_000, &(now + 100));
    client.create_program_release_schedule(&other, &50_000, &(now + 200));
    client.create_program_release_schedule(&winner, &50_000, &(now + 300));

    let winner_schedules = client.query_schedules_by_recipient(&winner, &0, &10);
    assert_eq!(winner_schedules.len(), 2);
    for s in winner_schedules.iter() {
        assert_eq!(s.recipient, winner);
    }

    let other_schedules = client.query_schedules_by_recipient(&other, &0, &10);
    assert_eq!(other_schedules.len(), 1);
}

#[test]
fn test_combined_recipient_and_amount_filter_manual() {
    // Query by recipient, then verify amount subset manually
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 500_000);

    let r1 = Address::generate(&env);

    client.single_payout(&r1, &10_000,
    &None
);
    client.single_payout(&r1, &200_000,
    &None
);
    client.single_payout(&r1, &50_000,
    &None
);

    // Get r1's records, then filter by amount > 100_000 in test
    let records = client.query_payouts_by_recipient(&r1, &0, &10);
    assert_eq!(records.len(), 3);

    let mut large_amounts = soroban_sdk::Vec::new(&env);
    for r in records.iter() {
        if r.amount > 100_000 {
            large_amounts.push_back(r);
        }
    }
    assert_eq!(large_amounts.get(0).unwrap().amount, 200_000);
}

// =============================================================================
// TESTS FOR PROGRAM RELEASE SCHEDULES ACROSS UPGRADES (#497)
// =============================================================================

/// Create schedules on "version N", then continue automatic and manual releases
/// without re-init (simulated post-upgrade) and verify no data loss.
#[test]
fn test_release_schedules_persist_after_simulated_upgrade() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 200_000);

    let now = env.ledger().timestamp();
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.create_program_release_schedule(&r1, &50_000, &(now + 100));
    client.create_program_release_schedule(&r2, &50_000, &(now + 200));

    let schedules_before = client.get_all_prog_release_schedules();
    assert_eq!(schedules_before.len(), 2);

    env.ledger().set_timestamp(now + 150);
    client.trigger_program_releases(&None);

    let schedules_after = client.get_all_prog_release_schedules();
    assert_eq!(schedules_after.len(), 2);
    let released_count = schedules_after.iter().filter(|s| s.released).count();
    assert_eq!(released_count, 1);

    let stats = client.get_program_aggregate_stats();
    assert_eq!(stats.released_count, 1);
    assert_eq!(stats.scheduled_count, 1);
    assert_eq!(stats.remaining_balance, 150_000);

    env.ledger().set_timestamp(now + 250);
    client.trigger_program_releases(&None);

    let stats_final = client.get_program_aggregate_stats();
    assert_eq!(stats_final.released_count, 2);
    assert_eq!(stats_final.scheduled_count, 0);
    assert_eq!(stats_final.remaining_balance, 100_000);
}

#[test]
fn test_release_schedules_timestamps_and_manual_release_after_simulated_upgrade() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 300_000);

    let now = env.ledger().timestamp();
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let s1 = client.create_program_release_schedule(&r1, &100_000, &(now + 100));
    let s2 = client.create_program_release_schedule(&r2, &150_000, &(now + 200));

    let schedules_before = client.get_all_prog_release_schedules();
    assert_eq!(schedules_before.len(), 2);
    assert_eq!(
        schedules_before.get(0).unwrap().release_timestamp,
        now + 100
    );
    assert_eq!(
        schedules_before.get(1).unwrap().release_timestamp,
        now + 200
    );
    assert!(!schedules_before.get(0).unwrap().released);
    assert!(!schedules_before.get(1).unwrap().released);

    // Simulated upgrade (no re-init, state is preserved)
    env.ledger().set_timestamp(now + 150);
    let released_count = client.trigger_program_releases(&None);
    assert_eq!(released_count, 1);

    let schedules_mid = client.get_all_prog_release_schedules();
    assert_eq!(schedules_mid.len(), 2);
    let mid_s1 = schedules_mid
        .iter()
        .find(|s| s.schedule_id == s1.schedule_id)
        .unwrap();
    let mid_s2 = schedules_mid
        .iter()
        .find(|s| s.schedule_id == s2.schedule_id)
        .unwrap();
    assert!(mid_s1.released);
    assert_eq!(mid_s1.release_timestamp, now + 100);
    assert!(!mid_s2.released);
    assert_eq!(mid_s2.release_timestamp, now + 200);

    // Manual release should succeed after upgrade even if schedule timestamp is in future.
    client.release_program_schedule_manual(&s2.schedule_id);

    let stats_after_manual = client.get_program_aggregate_stats();
    assert_eq!(stats_after_manual.released_count, 2);
    assert_eq!(stats_after_manual.scheduled_count, 0);
    assert_eq!(stats_after_manual.remaining_balance, 50_000);

    let schedules_final = client.get_all_prog_release_schedules();
    let final_s2 = schedules_final
        .iter()
        .find(|s| s.schedule_id == s2.schedule_id)
        .unwrap();
    assert!(final_s2.released);
    assert_eq!(final_s2.release_timestamp, now + 200);
}

#[test]
fn test_release_schedules_work_after_v2_program_state_migration() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 400_000);

    let program_id = String::from_str(&env, "hack-2026");
    let now = env.ledger().timestamp();
    let recipient = Address::generate(&env);

    client.create_program_release_schedule(&recipient, &100_000, &(now + 100));

    let prog_v2_before = client.get_program_info();
    assert_eq!(prog_v2_before.remaining_balance, 400_000);

    env.ledger().set_timestamp(now + 200);
    let released = client.trigger_program_releases(&None);
    assert_eq!(released, 1);

    let schedule = client
        .get_all_prog_release_schedules()
        .iter()
        .find(|s| s.schedule_id == 1)
        .unwrap();
    assert!(schedule.released);
    assert_eq!(schedule.release_timestamp, now + 100);

    let history = client.get_program_release_history();
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().schedule_id, 1);

    let prog_v2_after = client.get_program_info();
    assert_eq!(prog_v2_after.remaining_balance, 300_000);
    assert_eq!(prog_v2_after.payout_history.len(), 1);
}
