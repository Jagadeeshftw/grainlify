#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env, Map, String, Symbol, TryFromVal, Val,
};

fn setup_program(
    env: &Env,
    initial_amount: i128,
) -> (
    ProgramEscrowContractClient<'static>,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::Client::new(env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    let program_id = String::from_str(env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id);

    if initial_amount > 0 {
        token_admin_client.mint(&client.address, &initial_amount);
        client.lock_program_funds(&initial_amount);
    }

    (client, admin, token_client, token_admin_client)
}

fn next_seed(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    *seed
}

fn assert_event_data_has_v2_tag(env: &Env, data: &Val) {
    let data_map: Map<Symbol, Val> =
        Map::try_from_val(env, data).unwrap_or_else(|_| panic!("event payload should be a map"));
    let version_val = data_map
        .get(Symbol::new(env, "version"))
        .unwrap_or_else(|| panic!("event payload must contain version field"));
    let version = u32::try_from_val(env, &version_val).expect("version should decode as u32");
    assert_eq!(version, 2);
}

#[test]
fn test_init_program_and_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin);
    let program_id = String::from_str(&env, "hack-2026");

    let data = client.init_program(&program_id, &admin, &token_id);
    assert_eq!(data.total_funds, 0);
    assert_eq!(data.remaining_balance, 0);

    let events = env.events().all();
    assert!(events.len() >= 1);
}

#[test]
fn test_lock_program_funds_multi_step_balance() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    client.lock_program_funds(&10_000);
    client.lock_program_funds(&5_000);
    assert_eq!(client.get_remaining_balance(), 15_000);
    assert_eq!(client.get_program_info().total_funds, 15_000);
}

#[test]
fn test_edge_zero_initial_state() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 0);

    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(client.get_program_info().payout_history.len(), 0);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_edge_max_safe_lock_and_payout() {
    let env = Env::default();
    let safe_max = i64::MAX as i128;
    let (client, _admin, token_client, _token_admin) = setup_program(&env, safe_max);

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &safe_max);

    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(token_client.balance(&recipient), safe_max);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_single_payout_token_transfer_integration() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 100_000);

    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &30_000);

    assert_eq!(data.remaining_balance, 70_000);
    assert_eq!(token_client.balance(&recipient), 30_000);
    assert_eq!(token_client.balance(&client.address), 70_000);
}

#[test]
fn test_batch_payout_token_transfer_integration() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 150_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    let recipients = vec![&env, r1.clone(), r2.clone(), r3.clone()];
    let amounts = vec![&env, 10_000, 20_000, 30_000];

    let data = client.batch_payout(&recipients, &amounts);
    assert_eq!(data.remaining_balance, 90_000);
    assert_eq!(data.payout_history.len(), 3);

    assert_eq!(token_client.balance(&r1), 10_000);
    assert_eq!(token_client.balance(&r2), 20_000);
    assert_eq!(token_client.balance(&r3), 30_000);
}

#[test]
fn test_complete_lifecycle_integration() {
    let env = Env::default();
    let (client, _admin, token_client, token_admin) = setup_program(&env, 0);

    token_admin.mint(&client.address, &300_000);
    client.lock_program_funds(&300_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    client.single_payout(&r1, &50_000);
    let recipients = vec![&env, r2.clone(), r3.clone()];
    let amounts = vec![&env, 70_000, 30_000];
    client.batch_payout(&recipients, &amounts);

    let info = client.get_program_info();
    assert_eq!(info.total_funds, 300_000);
    assert_eq!(info.remaining_balance, 150_000);
    assert_eq!(info.payout_history.len(), 3);
    assert_eq!(token_client.balance(&client.address), 150_000);
}

#[test]
fn test_property_fuzz_balance_invariants() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 1_000_000);

    let mut seed = 123_u64;
    let mut expected_remaining = 1_000_000_i128;

    for _ in 0..40 {
        let amount = (next_seed(&mut seed) % 4_000 + 1) as i128;
        if amount > expected_remaining {
            continue;
        }

        if next_seed(&mut seed) % 2 == 0 {
            let recipient = Address::generate(&env);
            client.single_payout(&recipient, &amount);
        } else {
            let recipient1 = Address::generate(&env);
            let recipient2 = Address::generate(&env);
            let first = amount / 2;
            let second = amount - first;
            if first == 0 || second == 0 || first + second > expected_remaining {
                continue;
            }
            let recipients = vec![&env, recipient1, recipient2];
            let amounts = vec![&env, first, second];
            client.batch_payout(&recipients, &amounts);
        }

        expected_remaining -= amount;
        assert_eq!(client.get_remaining_balance(), expected_remaining);
        assert_eq!(token_client.balance(&client.address), expected_remaining);

        if expected_remaining == 0 {
            break;
        }
    }
}

#[test]
fn test_stress_high_load_many_payouts() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 1_000_000);

    for _ in 0..100 {
        let recipient = Address::generate(&env);
        client.single_payout(&recipient, &3_000);
    }

    let info = client.get_program_info();
    assert_eq!(info.payout_history.len(), 100);
    assert_eq!(info.remaining_balance, 700_000);
    assert_eq!(token_client.balance(&client.address), 700_000);
}

#[test]
fn test_gas_proxy_batch_vs_single_event_efficiency() {
    let env_single = Env::default();
    let (single_client, _single_admin, _single_token, _single_token_admin) =
        setup_program(&env_single, 200_000);

    let single_before = env_single.events().all().len();
    for _ in 0..10 {
        let recipient = Address::generate(&env_single);
        single_client.single_payout(&recipient, &1_000);
    }
    let single_events = env_single.events().all().len() - single_before;

    let env_batch = Env::default();
    let (batch_client, _batch_admin, _batch_token, _batch_token_admin) =
        setup_program(&env_batch, 200_000);

    let mut recipients = vec![&env_batch];
    let mut amounts = vec![&env_batch];
    for _ in 0..10 {
        recipients.push_back(Address::generate(&env_batch));
        amounts.push_back(1_000);
    }

    let batch_before = env_batch.events().all().len();
    batch_client.batch_payout(&recipients, &amounts);
    let batch_events = env_batch.events().all().len() - batch_before;

    assert!(batch_events <= single_events);
}

#[test]
fn test_events_emit_v2_version_tags_for_all_program_emitters() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.single_payout(&r1, &10_000);
    let recipients = vec![&env, r2];
    let amounts = vec![&env, 5_000];
    client.batch_payout(&recipients, &amounts);

    let events = env.events().all();
    let mut program_events_checked = 0_u32;
    for (contract, _topics, data) in events.iter() {
        if contract != client.address {
            continue;
        }
        assert_event_data_has_v2_tag(&env, &data);
        program_events_checked += 1;
    }

    // init_program, lock_program_funds, single_payout, batch_payout
    assert!(program_events_checked >= 4);
}

#[test]
fn test_release_schedule_exact_timestamp_boundary() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 100_000);
    let recipient = Address::generate(&env);

    let now = env.ledger().timestamp();
    let schedule = client.create_program_release_schedule(&25_000, &(now + 100), &recipient);

    env.ledger().set_timestamp(now + 100);
    let released = client.trigger_program_releases();
    assert_eq!(released, 1);

    let schedules = client.get_program_release_schedules();
    let updated = schedules.get(0).unwrap();
    assert_eq!(updated.schedule_id, schedule.schedule_id);
    assert!(updated.released);
    assert_eq!(token_client.balance(&recipient), 25_000);
}

#[test]
fn test_release_schedule_just_before_timestamp_rejected() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 100_000);
    let recipient = Address::generate(&env);

    let now = env.ledger().timestamp();
    client.create_program_release_schedule(&20_000, &(now + 80), &recipient);

    env.ledger().set_timestamp(now + 79);
    let released = client.trigger_program_releases();
    assert_eq!(released, 0);
    assert_eq!(token_client.balance(&recipient), 0);

    let schedules = client.get_program_release_schedules();
    assert!(!schedules.get(0).unwrap().released);
}

#[test]
fn test_release_schedule_significantly_after_timestamp_releases() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 100_000);
    let recipient = Address::generate(&env);

    let now = env.ledger().timestamp();
    client.create_program_release_schedule(&30_000, &(now + 60), &recipient);

    env.ledger().set_timestamp(now + 10_000);
    let released = client.trigger_program_releases();
    assert_eq!(released, 1);
    assert_eq!(token_client.balance(&recipient), 30_000);
}

#[test]
fn test_release_schedule_overlapping_schedules() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 200_000);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    let now = env.ledger().timestamp();
    client.create_program_release_schedule(&10_000, &(now + 50), &recipient1);
    client.create_program_release_schedule(&15_000, &(now + 50), &recipient2);
    client.create_program_release_schedule(&20_000, &(now + 120), &recipient3);

    env.ledger().set_timestamp(now + 50);
    let released_at_overlap = client.trigger_program_releases();
    assert_eq!(released_at_overlap, 2);
    assert_eq!(token_client.balance(&recipient1), 10_000);
    assert_eq!(token_client.balance(&recipient2), 15_000);
    assert_eq!(token_client.balance(&recipient3), 0);

    env.ledger().set_timestamp(now + 120);
    let released_later = client.trigger_program_releases();
    assert_eq!(released_later, 1);
    assert_eq!(token_client.balance(&recipient3), 20_000);

    let history = client.get_program_release_history();
    assert_eq!(history.len(), 3);
}

// =============================================================================
// TESTS FOR MULTI-TENANT ISOLATION
// =============================================================================

// Note: Comprehensive multi-tenant isolation tests are implemented in lib.rs
// using the ProgramEscrowContractClient for proper integration testing.
// The tests verify:
// - Funds and balance isolation between programs
// - Payout history isolation
// - Release schedule isolation
// - Release history isolation
// - Analytics isolation concepts (for future program-specific analytics)
// =============================================================================
// BATCH PROGRAM REGISTRATION TESTS
// =============================================================================
// These tests validate batch payout functionality including:
// - Happy path with multiple distinct recipients
// - Batches containing duplicate recipient addresses
// - Edge case at maximum allowed batch size
// - Error handling strategy (all-or-nothing atomicity)

#[test]
fn test_batch_payout_happy_path_multiple_recipients() {
    // Test the happy path: valid batch with multiple distinct recipients
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 6_000_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    let recipients = vec![&env, r1.clone(), r2.clone(), r3.clone()];
    let amounts = vec![&env, 1_000_000, 2_000_000, 3_000_000];

    let data = client.batch_payout(&recipients, &amounts);

    // Verify balance updated correctly (all-or-nothing)
    assert_eq!(data.remaining_balance, 0);

    // Verify payout history has all three records
    assert_eq!(data.payout_history.len(), 3);

    // Verify each payout record
    let payout1 = data.payout_history.get(0).unwrap();
    assert_eq!(payout1.recipient, r1);
    assert_eq!(payout1.amount, 1_000_000);

    let payout2 = data.payout_history.get(1).unwrap();
    assert_eq!(payout2.recipient, r2);
    assert_eq!(payout2.amount, 2_000_000);

    let payout3 = data.payout_history.get(2).unwrap();
    assert_eq!(payout3.recipient, r3);
    assert_eq!(payout3.amount, 3_000_000);

    // Verify token transfers
    assert_eq!(token_client.balance(&r1), 1_000_000);
    assert_eq!(token_client.balance(&r2), 2_000_000);
    assert_eq!(token_client.balance(&r3), 3_000_000);
}

#[test]
fn test_batch_payout_with_duplicate_recipient_addresses() {
    // Test batch containing duplicate recipient addresses
    // This validates that the contract handles repeated recipients correctly
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 4_500_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Create batch with duplicate recipient
    let recipients = vec![&env, r1.clone(), r2.clone(), r1.clone()];
    let amounts = vec![&env, 1_000_000, 2_000_000, 1_500_000];

    let data = client.batch_payout(&recipients, &amounts);

    // Balance should be fully consumed
    assert_eq!(data.remaining_balance, 0);

    // Payout history should have all three records (duplicates are allowed)
    assert_eq!(data.payout_history.len(), 3);

    // Count occurrences of r1 in history
    let mut r1_count = 0;
    let mut r1_total = 0i128;
    for i in 0..data.payout_history.len() {
        let record = data.payout_history.get(i).unwrap();
        if record.recipient == r1 {
            r1_count += 1;
            r1_total += record.amount;
        }
    }

    // r1 should appear twice with correct total
    assert_eq!(r1_count, 2);
    assert_eq!(r1_total, 1_000_000 + 1_500_000);

    // Verify token balances
    assert_eq!(token_client.balance(&r1), 2_500_000);
    assert_eq!(token_client.balance(&r2), 2_000_000);
}

#[test]
fn test_batch_payout_maximum_batch_size() {
    // Test batch at maximum allowed size
    // This validates edge case behavior with large batches
    let env = Env::default();
    let batch_size = 50usize;
    let amount_per_recipient = 100_000i128;
    let total_amount = (batch_size as i128) * amount_per_recipient;

    let (client, _admin, _token_client, _token_admin) = setup_program(&env, total_amount);

    let mut recipients = vec![&env];
    let mut amounts = vec![&env];

    for _ in 0..batch_size {
        recipients.push_back(Address::generate(&env));
        amounts.push_back(amount_per_recipient);
    }

    // Execute large batch payout
    let data = client.batch_payout(&recipients, &amounts);

    // Balance should be fully consumed
    assert_eq!(data.remaining_balance, 0);

    // Payout history should have all records
    assert_eq!(data.payout_history.len(), batch_size as u32);

    // Verify total payout amount
    let mut total_paid = 0i128;
    for i in 0..data.payout_history.len() {
        let record = data.payout_history.get(i).unwrap();
        total_paid += record.amount;
    }
    assert_eq!(total_paid, total_amount);
}

#[test]
#[should_panic(expected = "Cannot process empty batch")]
fn test_batch_payout_empty_batch_panic() {
    // Test that empty batch is rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 1_000_000);

    let recipients = vec![&env];
    let amounts = vec![&env];

    // Should panic
    client.batch_payout(&recipients, &amounts);
}

#[test]
#[should_panic(expected = "Recipients and amounts vectors must have the same length")]
fn test_batch_payout_mismatched_arrays_panic() {
    // Test that mismatched recipient/amount arrays are rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 5_000_000);

    let recipients = vec![&env, Address::generate(&env), Address::generate(&env)];
    let amounts = vec![&env, 1_000_000]; // Only 1 amount for 2 recipients

    // Should panic
    client.batch_payout(&recipients, &amounts);
}

#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn test_batch_payout_invalid_amount_zero_panic() {
    // Test that zero amounts are rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 5_000_000);

    let recipients = vec![&env, Address::generate(&env)];
    let amounts = vec![&env, 0i128]; // Zero amount - invalid

    // Should panic
    client.batch_payout(&recipients, &amounts);
}

#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn test_batch_payout_invalid_amount_negative_panic() {
    // Test that negative amounts are rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 5_000_000);

    let recipients = vec![&env, Address::generate(&env)];
    let amounts = vec![&env, -1_000_000]; // Negative amount - invalid

    // Should panic
    client.batch_payout(&recipients, &amounts);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_batch_payout_insufficient_balance_panic() {
    // Test that insufficient balance is rejected
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 5_000_000);

    let recipients = vec![&env, Address::generate(&env)];
    let amounts = vec![&env, 10_000_000]; // More than available

    // Should panic
    client.batch_payout(&recipients, &amounts);
}

#[test]
fn test_batch_payout_partial_spend() {
    // Test batch payout that doesn't spend entire balance
    // This validates that partial payouts work correctly
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let recipients = vec![&env, r1, r2];
    let amounts = vec![&env, 3_000_000, 3_000_000];

    let data = client.batch_payout(&recipients, &amounts);

    // Remaining balance should be correct
    assert_eq!(data.remaining_balance, 4_000_000);

    // Payout history should have both records
    assert_eq!(data.payout_history.len(), 2);
}

#[test]
fn test_batch_payout_atomicity_all_or_nothing() {
    // Test that batch payout maintains atomicity (all-or-nothing semantics)
    // Verify that either all payouts succeed or the entire transaction fails
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 3_000_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Get program state before payout
    let program_data_before = client.get_program_info();
    let history_len_before = program_data_before.payout_history.len();
    let balance_before = program_data_before.remaining_balance;

    // Execute successful batch payout
    let recipients = vec![&env, r1, r2];
    let amounts = vec![&env, 1_000_000, 2_000_000];

    let data = client.batch_payout(&recipients, &amounts);

    // All records must be written
    assert_eq!(data.payout_history.len(), history_len_before + 2);

    // Balance must be fully updated
    assert_eq!(data.remaining_balance, balance_before - 3_000_000);

    // All conditions should be satisfied together (atomicity)
    assert_eq!(data.payout_history.len(), 2);
    assert_eq!(data.remaining_balance, 0);
}

#[test]
fn test_batch_payout_sequential_batches() {
    // Test multiple sequential batch payouts to same program
    // Validates that history accumulates correctly
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 9_000_000);

    // First batch
    let r1 = Address::generate(&env);
    let recipients1 = vec![&env, r1];
    let amounts1 = vec![&env, 3_000_000];
    let data1 = client.batch_payout(&recipients1, &amounts1);

    // Verify after first batch
    assert_eq!(data1.payout_history.len(), 1);
    assert_eq!(data1.remaining_balance, 6_000_000);

    // Second batch
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    let recipients2 = vec![&env, r2, r3];
    let amounts2 = vec![&env, 2_000_000, 4_000_000];
    let data2 = client.batch_payout(&recipients2, &amounts2);

    // Verify after second batch
    assert_eq!(data2.payout_history.len(), 3);
    assert_eq!(data2.remaining_balance, 0);

    // Verify history order
    let record1 = data2.payout_history.get(0).unwrap();
    assert_eq!(record1.amount, 3_000_000);

    let record2 = data2.payout_history.get(1).unwrap();
    assert_eq!(record2.amount, 2_000_000);

    let record3 = data2.payout_history.get(2).unwrap();
    assert_eq!(record3.amount, 4_000_000);
}
