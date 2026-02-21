#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env, String,
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
fn test_release_schedule_exact_timestamp_boundary() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 100_000);
    let recipient = Address::generate(&env);

    let now = env.ledger().timestamp();
    let schedule = client.create_program_release_schedule(&recipient, &25_000, &(now + 100));

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
    client.create_program_release_schedule(&recipient, &20_000, &(now + 80));

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
    client.create_program_release_schedule(&recipient, &30_000, &(now + 60));

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
    client.create_program_release_schedule(&recipient1, &10_000, &(now + 50));
    client.create_program_release_schedule(&recipient2, &15_000, &(now + 50)); // overlapping timestamp
    client.create_program_release_schedule(&recipient3, &20_000, &(now + 120));

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

// ---------------------------------------------------------------------------
// Full program lifecycle integration test with batch payouts across two
// independent program-escrow instances.
// ---------------------------------------------------------------------------
#[test]
fn test_full_lifecycle_multi_program_batch_payouts() {
    let env = Env::default();
    env.mock_all_auths();

    // ── Shared token setup ──────────────────────────────────────────────
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::Client::new(&env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    // ── Program A: "hackathon-alpha" ────────────────────────────────────
    let contract_a = env.register_contract(None, ProgramEscrowContract);
    let client_a = ProgramEscrowContractClient::new(&env, &contract_a);
    let auth_key_a = Address::generate(&env);

    let prog_a = client_a.init_program(
        &String::from_str(&env, "hackathon-alpha"),
        &auth_key_a,
        &token_id,
    );
    assert_eq!(prog_a.total_funds, 0);
    assert_eq!(prog_a.remaining_balance, 0);

    // ── Program B: "hackathon-beta" ─────────────────────────────────────
    let contract_b = env.register_contract(None, ProgramEscrowContract);
    let client_b = ProgramEscrowContractClient::new(&env, &contract_b);
    let auth_key_b = Address::generate(&env);

    let prog_b = client_b.init_program(
        &String::from_str(&env, "hackathon-beta"),
        &auth_key_b,
        &token_id,
    );
    assert_eq!(prog_b.total_funds, 0);

    // ── Phase 1: Lock funds in multiple steps ───────────────────────────
    // Program A receives 500_000 in two tranches
    token_admin_client.mint(&client_a.address, &300_000);
    client_a.lock_program_funds(&300_000);
    assert_eq!(client_a.get_remaining_balance(), 300_000);

    token_admin_client.mint(&client_a.address, &200_000);
    client_a.lock_program_funds(&200_000);
    assert_eq!(client_a.get_remaining_balance(), 500_000);
    assert_eq!(client_a.get_program_info().total_funds, 500_000);

    // Program B receives 400_000 in three tranches
    token_admin_client.mint(&client_b.address, &150_000);
    client_b.lock_program_funds(&150_000);

    token_admin_client.mint(&client_b.address, &150_000);
    client_b.lock_program_funds(&150_000);

    token_admin_client.mint(&client_b.address, &100_000);
    client_b.lock_program_funds(&100_000);
    assert_eq!(client_b.get_remaining_balance(), 400_000);
    assert_eq!(client_b.get_program_info().total_funds, 400_000);

    // ── Phase 2: First round of batch payouts ───────────────────────────
    let winner_a1 = Address::generate(&env);
    let winner_a2 = Address::generate(&env);
    let winner_a3 = Address::generate(&env);

    // Program A — batch payout round 1: 3 winners
    let data_a1 = client_a.batch_payout(
        &vec![&env, winner_a1.clone(), winner_a2.clone(), winner_a3.clone()],
        &vec![&env, 100_000, 75_000, 50_000],
    );
    assert_eq!(data_a1.remaining_balance, 275_000);
    assert_eq!(data_a1.payout_history.len(), 3);
    assert_eq!(token_client.balance(&winner_a1), 100_000);
    assert_eq!(token_client.balance(&winner_a2), 75_000);
    assert_eq!(token_client.balance(&winner_a3), 50_000);

    let winner_b1 = Address::generate(&env);
    let winner_b2 = Address::generate(&env);

    // Program B — batch payout round 1: 2 winners
    let data_b1 = client_b.batch_payout(
        &vec![&env, winner_b1.clone(), winner_b2.clone()],
        &vec![&env, 120_000, 80_000],
    );
    assert_eq!(data_b1.remaining_balance, 200_000);
    assert_eq!(data_b1.payout_history.len(), 2);
    assert_eq!(token_client.balance(&winner_b1), 120_000);
    assert_eq!(token_client.balance(&winner_b2), 80_000);

    // ── Phase 3: Second round of batch payouts ──────────────────────────
    let winner_a4 = Address::generate(&env);
    let winner_a5 = Address::generate(&env);

    // Program A — batch payout round 2: 2 more winners
    let data_a2 = client_a.batch_payout(
        &vec![&env, winner_a4.clone(), winner_a5.clone()],
        &vec![&env, 125_000, 50_000],
    );
    assert_eq!(data_a2.remaining_balance, 100_000);
    assert_eq!(data_a2.payout_history.len(), 5);
    assert_eq!(token_client.balance(&winner_a4), 125_000);
    assert_eq!(token_client.balance(&winner_a5), 50_000);

    let winner_b3 = Address::generate(&env);
    let winner_b4 = Address::generate(&env);
    let winner_b5 = Address::generate(&env);

    // Program B — batch payout round 2: 3 more winners
    let data_b2 = client_b.batch_payout(
        &vec![&env, winner_b3.clone(), winner_b4.clone(), winner_b5.clone()],
        &vec![&env, 60_000, 40_000, 30_000],
    );
    assert_eq!(data_b2.remaining_balance, 70_000);
    assert_eq!(data_b2.payout_history.len(), 5);
    assert_eq!(token_client.balance(&winner_b3), 60_000);
    assert_eq!(token_client.balance(&winner_b4), 40_000);
    assert_eq!(token_client.balance(&winner_b5), 30_000);

    // ── Phase 4: Final balance verification ─────────────────────────────
    // Program A: 500_000 locked − (100k + 75k + 50k + 125k + 50k) = 100_000
    assert_eq!(client_a.get_remaining_balance(), 100_000);
    assert_eq!(token_client.balance(&client_a.address), 100_000);

    let info_a = client_a.get_program_info();
    assert_eq!(info_a.total_funds, 500_000);
    assert_eq!(info_a.remaining_balance, 100_000);
    assert_eq!(info_a.payout_history.len(), 5);

    // Program B: 400_000 locked − (120k + 80k + 60k + 40k + 30k) = 70_000
    assert_eq!(client_b.get_remaining_balance(), 70_000);
    assert_eq!(token_client.balance(&client_b.address), 70_000);

    let info_b = client_b.get_program_info();
    assert_eq!(info_b.total_funds, 400_000);
    assert_eq!(info_b.remaining_balance, 70_000);
    assert_eq!(info_b.payout_history.len(), 5);

    // ── Phase 5: Aggregate stats verification ───────────────────────────
    let stats_a = client_a.get_program_aggregate_stats();
    assert_eq!(stats_a.total_funds, 500_000);
    assert_eq!(stats_a.remaining_balance, 100_000);
    assert_eq!(stats_a.total_paid_out, 400_000);
    assert_eq!(stats_a.payout_count, 5);

    let stats_b = client_b.get_program_aggregate_stats();
    assert_eq!(stats_b.total_funds, 400_000);
    assert_eq!(stats_b.remaining_balance, 70_000);
    assert_eq!(stats_b.total_paid_out, 330_000);
    assert_eq!(stats_b.payout_count, 5);

    // ── Phase 6: Cross-program isolation check ──────────────────────────
    // Verify programs don't interfere with each other's on-chain balances
    let total_distributed = (500_000 - 100_000) + (400_000 - 70_000);
    assert_eq!(total_distributed, 730_000);
    assert_eq!(
        token_client.balance(&client_a.address) + token_client.balance(&client_b.address),
        170_000
    );

    // ── Phase 7: Event emission verification ────────────────────────────
    let all_events = env.events().all();

    // At minimum we expect: 2 PrgInit + 5 FndsLock + 4 BatchPay = 11 contract events
    // (plus token transfer events emitted by the SAC)
    assert!(
        all_events.len() >= 11,
        "Expected at least 11 contract events, got {}",
        all_events.len()
    );
}
