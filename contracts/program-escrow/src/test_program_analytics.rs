#![cfg(test)]

extern crate std;

use super::*;
use crate::test_support::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke}, token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val};

#[test]
fn test_analytics_initial_state() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.total_funds, 0);
    assert_eq!(stats.remaining_balance, 0);
    assert_eq!(stats.total_paid_out, 0);
    assert_eq!(stats.payout_count, 0);
    assert_eq!(stats.scheduled_count, 0);
    assert_eq!(stats.released_count, 0);
}

// Test: get_program_aggregate_stats reflects locked funds correctly
#[test]
fn test_analytics_after_lock_funds() {
    let env = Env::default();
    let locked_amount = 50_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, locked_amount);

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.total_funds, locked_amount);
    assert_eq!(stats.remaining_balance, locked_amount);
    assert_eq!(stats.total_paid_out, 0);
    assert_eq!(stats.payout_count, 0);
}

// Test: get_program_aggregate_stats reflects single payouts correctly
#[test]
fn test_analytics_after_single_payout() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let payout_amount = 25_000_0000000i128;

    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &payout_amount,
    &None
);

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.total_funds, initial_funds);
    assert_eq!(stats.remaining_balance, initial_funds - payout_amount);
    assert_eq!(stats.total_paid_out, payout_amount);
    assert_eq!(stats.payout_count, 1);
}

// Test: get_program_aggregate_stats reflects batch payouts correctly
#[test]
fn test_analytics_after_batch_payout() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    let recipients = vec![&env, r1.clone(), r2.clone(), r3.clone()];
    let amounts = vec![&env, 10_000_0000000, 20_000_0000000, 30_000_0000000];

    client.batch_payout(&recipients, &amounts,
    &None
);

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.total_funds, initial_funds);
    assert_eq!(stats.remaining_balance, 40_000_0000000i128);
    assert_eq!(stats.total_paid_out, 60_000_0000000i128);
    assert_eq!(stats.payout_count, 3);
}

// Test: aggregate stats after multiple operations
#[test]
fn test_analytics_multiple_operations() {
    let env = Env::default();
    let (client, _admin, _token, token_admin) = setup_program(&env, 0);
    token_admin.mint(&client.address, &30_000_0000000);

    // Lock funds in multiple calls
    client.lock_program_funds(&10_000_0000000);
    client.lock_program_funds(&15_000_0000000);
    client.lock_program_funds(&5_000_0000000);

    // Perform payouts
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.single_payout(&r1, &5_000_0000000,
    &None
);

    let recipients = vec![&env, r2.clone()];
    let amounts = vec![&env, 3_000_0000000];
    client.batch_payout(&recipients, &amounts,
    &None
);

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.total_funds, 30_000_0000000i128);
    assert_eq!(stats.remaining_balance, 22_000_0000000i128);
    assert_eq!(stats.total_paid_out, 8_000_0000000i128);
    assert_eq!(stats.payout_count, 2);
}

// Test: aggregate stats with release schedules
#[test]
fn test_analytics_with_schedules() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let future_timestamp = env.ledger().timestamp() + 1000;

    client.create_program_release_schedule(&recipient1, &20_000_0000000, &future_timestamp);
    client.create_program_release_schedule(&recipient2, &30_000_0000000, &(future_timestamp + 100));

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.scheduled_count, 2);
    assert_eq!(stats.released_count, 0);
}

// Test: aggregate stats after releasing schedules
#[test]
fn test_analytics_after_releasing_schedules() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let recipient = Address::generate(&env);
    let release_timestamp = env.ledger().timestamp() + 50;

    client.create_program_release_schedule(&recipient, &20_000_0000000, &release_timestamp);

    // Advance time and trigger releases
    env.ledger().set_timestamp(release_timestamp + 1);
    client.trigger_program_releases(&None);

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.scheduled_count, 0);
    assert_eq!(stats.released_count, 1);
    assert_eq!(stats.total_paid_out, 20_000_0000000i128);
    assert_eq!(stats.remaining_balance, 80_000_0000000i128);
}

// Test: remaining balance as a health metric
#[test]
fn test_health_remaining_balance() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let balance1 = client.get_remaining_balance();
    assert_eq!(balance1, initial_funds);

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &25_000_0000000,
    &None
);

    let balance2 = client.get_remaining_balance();
    assert_eq!(balance2, 75_000_0000000i128);
}

// Test: due schedules as a health indicator
#[test]
fn test_health_due_schedules() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let recipient = Address::generate(&env);
    let now = env.ledger().timestamp();

    client.create_program_release_schedule(&recipient, &10_000_0000000, &now);

    let recipient2 = Address::generate(&env);
    client.create_program_release_schedule(&recipient2, &15_000_0000000, &(now + 1000));

    let due = client.get_due_schedules();
    assert_eq!(due.len(), 1);
}

// Test: total scheduled amount calculation
#[test]
fn test_total_scheduled_amount() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let future_timestamp = env.ledger().timestamp() + 500;

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    client.create_program_release_schedule(&r1, &10_000_0000000, &future_timestamp);
    client.create_program_release_schedule(&r2, &20_000_0000000, &(future_timestamp + 100));
    client.create_program_release_schedule(&r3, &15_000_0000000, &(future_timestamp + 200));

    let total_scheduled = client.get_total_scheduled_amount();
    assert_eq!(total_scheduled, 45_000_0000000i128);
}

// Test: comprehensive analytics workflow
#[test]
fn test_comprehensive_analytics_workflow() {
    let env = Env::default();
    let (client, _admin, _token, token_admin) = setup_program(&env, 0);
    token_admin.mint(&client.address, &100_000_0000000);

    client.lock_program_funds(&50_000_0000000);
    client.lock_program_funds(&50_000_0000000);

    let r1 = Address::generate(&env);
    client.single_payout(&r1, &10_000_0000000,
    &None
);

    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    let recipients = vec![&env, r2.clone(), r3.clone()];
    let amounts = vec![&env, 15_000_0000000, 20_000_0000000];
    client.batch_payout(&recipients, &amounts,
    &None
);

    let future_timestamp = env.ledger().timestamp() + 100;
    let r4 = Address::generate(&env);
    client.create_program_release_schedule(&r4, &25_000_0000000, &future_timestamp);

    env.ledger().set_timestamp(future_timestamp + 1);
    client.trigger_program_releases(&None);

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.total_funds, 100_000_0000000i128);
    assert_eq!(stats.remaining_balance, 30_000_0000000i128);
    assert_eq!(stats.total_paid_out, 70_000_0000000i128);
    assert_eq!(stats.payout_count, 4);
    assert_eq!(stats.scheduled_count, 0);
    assert_eq!(stats.released_count, 1);
}

// Test: analytics partial release scenario
#[test]
fn test_analytics_partial_release_scenario() {
    let env = Env::default();
    let initial_funds = 50_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let future_timestamp = env.ledger().timestamp() + 50;

    for i in 0..3 {
        let recipient = Address::generate(&env);
        client.create_program_release_schedule(
            &recipient,
            &10_000_0000000,
            &(future_timestamp + (i as u64 * 10)),
        );
    }

    env.ledger().set_timestamp(future_timestamp + 15);
    client.trigger_program_releases(&None);

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.scheduled_count, 1);
    assert_eq!(stats.released_count, 2);
    assert_eq!(stats.total_paid_out, 20_000_0000000i128);
    assert_eq!(stats.remaining_balance, 30_000_0000000i128);

    env.ledger().set_timestamp(future_timestamp + 35);
    client.trigger_program_releases(&None);

    let stats_final = client.get_program_aggregate_stats();

    assert_eq!(stats_final.scheduled_count, 0);
    assert_eq!(stats_final.released_count, 3);
    assert_eq!(stats_final.total_paid_out, 30_000_0000000i128);
    assert_eq!(stats_final.remaining_balance, 20_000_0000000i128);
}

// Test: analytics query functions work correctly
#[test]
fn test_analytics_query_functions() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    // Create payouts to different recipients
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    client.single_payout(&r1, &10_000_0000000,
    &None
);
    client.single_payout(&r2, &20_000_0000000,
    &None
);
    client.single_payout(&r3, &15_000_0000000,
    &None
);

    // Query by recipient
    let payouts_r1 = client.get_payouts_by_recipient(&r1, &0, &10);
    assert_eq!(payouts_r1.len(), 1);
    assert_eq!(payouts_r1.get(0).unwrap().amount, 10_000_0000000);

    let payouts_r2 = client.get_payouts_by_recipient(&r2, &0, &10);
    assert_eq!(payouts_r2.len(), 1);
    assert_eq!(payouts_r2.get(0).unwrap().amount, 20_000_0000000);

    // Query by amount range
    let payouts_range = client.query_payouts_by_amount(&12_000_0000000, &18_000_0000000, &0, &10);
    assert_eq!(payouts_range.len(), 1);
    assert_eq!(payouts_range.get(0).unwrap().amount, 15_000_0000000);
}

// Test (#493): metrics reflect real operations â€” total operations, success counts
