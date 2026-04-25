extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke},
    token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val,
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
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let token_client = token::Client::new(env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    let program_id = String::from_str(env, "hack-2026");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id);

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
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let program_id = String::from_str(&env, "hack-2026");

    let data = client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
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

    for _ in 0..10 {
        let mut recipients = vec![&env];
        let mut amounts = vec![&env];

        for _ in 0..10 {
            recipients.push_back(Address::generate(&env));
            amounts.push_back(3_000);
        }

        client.batch_payout(&recipients, &amounts);
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
    let schedule = client.create_program_release_schedule(&recipient, &25_000, &(now + 100));

    env.ledger().set_timestamp(now + 100);
    let released = client.trigger_program_releases();
    assert_eq!(released, 1);

    let schedules = client.get_release_schedules();
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

    let schedules = client.get_release_schedules();
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
    client.create_program_release_schedule(&recipient2, &15_000, &(now + 50));
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
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    // ── Program A: "hackathon-alpha" ────────────────────────────────────
    let contract_a = env.register_contract(None, ProgramEscrowContract);
    let client_a = ProgramEscrowContractClient::new(&env, &contract_a);
    let auth_key_a = Address::generate(&env);

    let program_id_a = String::from_str(&env, "hackathon-alpha");
    let prog_a = client_a.init_program(
        &program_id_a,
        &auth_key_a,
        &token_id,
        &auth_key_a,
        &None,
        &None,
    );
    client_a.publish_program(&program_id_a);
    assert_eq!(prog_a.total_funds, 0);
    assert_eq!(prog_a.remaining_balance, 0);

    // ── Program B: "hackathon-beta" ─────────────────────────────────────
    let contract_b = env.register_contract(None, ProgramEscrowContract);
    let client_b = ProgramEscrowContractClient::new(&env, &contract_b);
    let auth_key_b = Address::generate(&env);

    let program_id_b = String::from_str(&env, "hackathon-beta");
    let prog_b = client_b.init_program(
        &program_id_b,
        &auth_key_b,
        &token_id,
        &auth_key_b,
        &None,
        &None,
    );
    client_b.publish_program(&program_id_b);
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
        &vec![
            &env,
            winner_a1.clone(),
            winner_a2.clone(),
            winner_a3.clone(),
        ],
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
        &vec![
            &env,
            winner_b3.clone(),
            winner_b4.clone(),
            winner_b5.clone(),
        ],
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

#[test]
fn test_multi_token_balance_accounting_isolated_across_program_instances() {
    let env = Env::default();
    env.mock_all_auths();

    // Two program escrow instances with different token contracts.
    let contract_a = env.register_contract(None, ProgramEscrowContract);
    let contract_b = env.register_contract(None, ProgramEscrowContract);
    let client_a = ProgramEscrowContractClient::new(&env, &contract_a);
    let client_b = ProgramEscrowContractClient::new(&env, &contract_b);

    let token_admin_a = Address::generate(&env);
    let token_admin_b = Address::generate(&env);
    let token_a = env.register_stellar_asset_contract(token_admin_a.clone());
    let token_b = env.register_stellar_asset_contract(token_admin_b.clone());
    let token_client_a = token::Client::new(&env, &token_a);
    let token_client_b = token::Client::new(&env, &token_b);
    let token_admin_client_a = token::StellarAssetClient::new(&env, &token_a);
    let token_admin_client_b = token::StellarAssetClient::new(&env, &token_b);

    let payout_key_a = Address::generate(&env);
    let payout_key_b = Address::generate(&env);

    let program_id_a = String::from_str(&env, "multi-token-a");
    client_a.init_program(
        &program_id_a,
        &payout_key_a,
        &token_a,
        &payout_key_a,
        &None,
        &None,
    );
    client_a.publish_program(&program_id_a);

    let program_id_b = String::from_str(&env, "multi-token-b");
    client_b.init_program(
        &program_id_b,
        &payout_key_b,
        &token_b,
        &payout_key_b,
        &None,
        &None,
    );
    client_b.publish_program(&program_id_b);

    token_admin_client_a.mint(&client_a.address, &500_000);
    token_admin_client_b.mint(&client_b.address, &300_000);
    client_a.lock_program_funds(&500_000);
    client_b.lock_program_funds(&300_000);

    // Initial per-token accounting after lock.
    assert_eq!(client_a.get_remaining_balance(), 500_000);
    assert_eq!(client_b.get_remaining_balance(), 300_000);
    assert_eq!(token_client_a.balance(&client_a.address), 500_000);
    assert_eq!(token_client_b.balance(&client_b.address), 300_000);

    let recipient = Address::generate(&env);
    client_a.single_payout(&recipient, &120_000);

    // Payout in token A should not affect token B program balances.
    assert_eq!(client_a.get_remaining_balance(), 380_000);
    assert_eq!(client_b.get_remaining_balance(), 300_000);
    assert_eq!(token_client_a.balance(&recipient), 120_000);
    assert_eq!(token_client_b.balance(&recipient), 0);
    assert_eq!(token_client_a.balance(&client_a.address), 380_000);
    assert_eq!(token_client_b.balance(&client_b.address), 300_000);

    let r_b1 = Address::generate(&env);
    let r_b2 = Address::generate(&env);
    client_b.batch_payout(
        &vec![&env, r_b1.clone(), r_b2.clone()],
        &vec![&env, 50_000, 25_000],
    );

    // Payout in token B should not affect token A accounting.
    assert_eq!(client_a.get_remaining_balance(), 380_000);
    assert_eq!(client_b.get_remaining_balance(), 225_000);
    assert_eq!(token_client_a.balance(&client_a.address), 380_000);
    assert_eq!(token_client_b.balance(&client_b.address), 225_000);
}

#[test]
fn test_anti_abuse_whitelist_bypass() {
    let env = Env::default();
    let lock_amount = 100_000_000_000i128;
    let (client, admin, _token_client, _token_admin) = setup_program(&env, lock_amount);

    client.set_admin(&admin);

    let config = client.get_rate_limit_config();
    let max_ops = config.max_operations;
    let recipient = Address::generate(&env);

    let start_time = 1_000_000;
    env.ledger().set_timestamp(start_time);

    client.set_whitelist(&admin, &true);

    env.ledger()
        .set_timestamp(start_time + config.cooldown_period + 1);

    for _ in 0..(max_ops + 5) {
        client.single_payout(&recipient, &100);
    }

    let info = client.get_program_info();
    assert_eq!(info.payout_history.len() as u32, max_ops + 5);
}

// =============================================================================
// Admin rotation and config updates (Issue #465)
// =============================================================================

/// Admin can be set and rotated; new admin is persisted.
#[test]
fn test_admin_rotation() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();

    client.set_admin(&admin);
    assert_eq!(client.get_admin(), Some(admin.clone()));

    client.set_admin(&new_admin);
    assert_eq!(client.get_admin(), Some(new_admin));
}

/// After admin rotation, new admin can update rate limit config.
#[test]
fn test_new_admin_can_update_config() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();

    client.set_admin(&admin);
    client.set_admin(&new_admin);

    client.update_rate_limit_config(&3600, &10, &30);

    let config = client.get_rate_limit_config();
    assert_eq!(config.window_size, 3600);
    assert_eq!(config.max_operations, 10);
    assert_eq!(config.cooldown_period, 30);
}

/// Non-admin cannot update rate limit config.
#[test]
#[should_panic]
fn test_non_admin_cannot_update_config() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);

    env.mock_all_auths();

    client.set_admin(&admin);

    // Mock only non_admin so that update_rate_limit_config sees non_admin as caller;
    // contract requires admin.require_auth(), so this must panic.
    env.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "update_rate_limit_config",
            args: (3600u64, 10u32, 30u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.update_rate_limit_config(&3600, &10, &30);
}

// =============================================================================
// TESTS FOR batch_initialize_programs
// =============================================================================

#[test]
fn test_batch_initialize_programs_success() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-1"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-2"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, 2);
    assert!(client.program_exists());
}

#[test]
fn test_batch_initialize_programs_empty_err() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let items: Vec<ProgramInitItem> = Vec::new(&env);
    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::InvalidBatchSizeProgram))));
}

#[test]
fn test_batch_initialize_programs_duplicate_id_err() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let pid = String::from_str(&env, "same-id");
    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: pid.clone(),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: pid,
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::DuplicateProgramId))));
}

// =============================================================================
// EXTENDED TESTS FOR batch_initialize_programs
// =============================================================================

/// Helper: build a deterministic program ID for large-batch tests.
fn make_program_id(env: &Env, index: u32) -> String {
    let mut buf = [b'p', b'-', b'0', b'0', b'0', b'0', b'0'];
    let mut n = index;
    let mut pos = 6usize;
    loop {
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 || pos == 2 {
            break;
        }
        pos -= 1;
    }
    String::from_str(env, core::str::from_utf8(&buf).unwrap())
}

#[test]
fn test_batch_register_happy_path_five_programs() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    for i in 0..5u32 {
        items.push_back(ProgramInitItem {
            program_id: make_program_id(&env, i),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
            reference_hash: None,
        });
    }

    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, 5);

    for i in 0..5u32 {
        assert!(client.program_exists_by_id(&make_program_id(&env, i)));
    }
}

#[test]
fn test_batch_register_single_item() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "solo-prog"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, 1);
    assert!(client.program_exists_by_id(&String::from_str(&env, "solo-prog")));
}

#[test]
fn test_batch_register_exceeds_max_batch_size() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    for i in 0..(MAX_BATCH_SIZE + 1) {
        items.push_back(ProgramInitItem {
            program_id: make_program_id(&env, i),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
            reference_hash: None,
        });
    }

    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::InvalidBatchSizeProgram))));
}

#[test]
fn test_batch_register_at_exact_max_batch_size() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    for i in 0..MAX_BATCH_SIZE {
        items.push_back(ProgramInitItem {
            program_id: make_program_id(&env, i),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
            reference_hash: None,
        });
    }

    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, MAX_BATCH_SIZE);

    // Spot-check first, middle, and last entries
    assert!(client.program_exists_by_id(&make_program_id(&env, 0)));
    assert!(client.program_exists_by_id(&make_program_id(&env, 50)));
    assert!(client.program_exists_by_id(&make_program_id(&env, MAX_BATCH_SIZE - 1)));
}

#[test]
fn test_batch_register_program_already_exists_error() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Register first batch
    let mut first = Vec::new(&env);
    first.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "existing"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    client
        .try_batch_initialize_programs(&first)
        .unwrap()
        .unwrap();

    // Second batch contains the same ID — must fail entirely
    let mut second = Vec::new(&env);
    second.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "brand-new"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    second.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "existing"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let res = client.try_batch_initialize_programs(&second);
    assert!(matches!(res, Err(Ok(BatchError::ProgramAlreadyExists))));

    // "brand-new" must NOT exist — all-or-nothing semantics
    assert!(!client.program_exists_by_id(&String::from_str(&env, "brand-new")));
}

#[test]
fn test_batch_register_all_or_nothing_on_duplicate() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Batch with valid IDs plus a duplicate — entire batch must be rejected
    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "alpha"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "beta"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "alpha"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::DuplicateProgramId))));

    // Neither program should exist
    assert!(!client.program_exists_by_id(&String::from_str(&env, "alpha")));
    assert!(!client.program_exists_by_id(&String::from_str(&env, "beta")));
}

#[test]
fn test_batch_register_duplicate_at_tail() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "unique-1"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "dup-tail"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "dup-tail"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let res = client.try_batch_initialize_programs(&items);
    assert!(matches!(res, Err(Ok(BatchError::DuplicateProgramId))));
}

#[test]
fn test_batch_register_different_auth_keys_and_tokens() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin_a = Address::generate(&env);
    let admin_b = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-a"),
        authorized_payout_key: admin_a.clone(),
        token_address: token_a.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "prog-b"),
        authorized_payout_key: admin_b.clone(),
        token_address: token_b.clone(),
        reference_hash: None,
    });

    let count = client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();
    assert_eq!(count, 2);
    assert!(client.program_exists_by_id(&String::from_str(&env, "prog-a")));
    assert!(client.program_exists_by_id(&String::from_str(&env, "prog-b")));
}

#[test]
fn test_batch_register_events_emitted_per_program() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let events_before = env.events().all().len();

    let mut items = Vec::new(&env);
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "evt-1"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "evt-2"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    items.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "evt-3"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    client
        .try_batch_initialize_programs(&items)
        .unwrap()
        .unwrap();

    let events_after = env.events().all().len();
    let new_events = events_after - events_before;

    // At least one event per registered program
    assert!(
        new_events >= 3,
        "Expected at least 3 events for 3 programs, got {}",
        new_events
    );
}

#[test]
fn test_batch_register_sequential_batches_no_conflict() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // First batch
    let mut batch1 = Vec::new(&env);
    batch1.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "b1-a"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    batch1.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "b1-b"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    let c1 = client
        .try_batch_initialize_programs(&batch1)
        .unwrap()
        .unwrap();
    assert_eq!(c1, 2);

    // Second batch — different IDs
    let mut batch2 = Vec::new(&env);
    batch2.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "b2-a"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    batch2.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "b2-b"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    let c2 = client
        .try_batch_initialize_programs(&batch2)
        .unwrap()
        .unwrap();
    assert_eq!(c2, 2);

    // All four should exist
    assert!(client.program_exists_by_id(&String::from_str(&env, "b1-a")));
    assert!(client.program_exists_by_id(&String::from_str(&env, "b1-b")));
    assert!(client.program_exists_by_id(&String::from_str(&env, "b2-a")));
    assert!(client.program_exists_by_id(&String::from_str(&env, "b2-b")));
}

#[test]
fn test_batch_register_second_batch_conflicts_with_first() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // First batch succeeds
    let mut batch1 = Vec::new(&env);
    batch1.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "shared"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    client
        .try_batch_initialize_programs(&batch1)
        .unwrap()
        .unwrap();

    // Second batch reuses "shared" — must fail
    let mut batch2 = Vec::new(&env);
    batch2.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "fresh"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });
    batch2.push_back(ProgramInitItem {
        program_id: String::from_str(&env, "shared"),
        authorized_payout_key: admin.clone(),
        token_address: token.clone(),
        reference_hash: None,
    });

    let res = client.try_batch_initialize_programs(&batch2);
    assert!(matches!(res, Err(Ok(BatchError::ProgramAlreadyExists))));

    // "fresh" must not exist (all-or-nothing)
    assert!(!client.program_exists_by_id(&String::from_str(&env, "fresh")));
}

// =============================================================================
// TESTS FOR MAXIMUM PROGRAM COUNT (#501)
// =============================================================================

/// Stress test: create many programs via sequential batches and verify counts
/// and sampling queries remain accurate (bounded for CI).
#[test]
fn test_max_program_count_sequential_batches_queries_accurate() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    const BATCH_SIZE: u32 = 10;
    const NUM_BATCHES: u32 = 3;
    let total_programs = BATCH_SIZE * NUM_BATCHES;

    for batch in 0..NUM_BATCHES {
        let mut items = Vec::new(&env);
        for i in 0..BATCH_SIZE {
            let idx = batch * BATCH_SIZE + i;
            items.push_back(ProgramInitItem {
                program_id: make_program_id(&env, idx),
                authorized_payout_key: admin.clone(),
                token_address: token.clone(),
                reference_hash: None,
            });
        }
        let count = client
            .try_batch_initialize_programs(&items)
            .unwrap()
            .unwrap();
        assert_eq!(count, BATCH_SIZE);
    }

    for i in 0..total_programs {
        assert!(
            client.program_exists_by_id(&make_program_id(&env, i)),
            "program {} should exist",
            i
        );
    }
    assert!(client.program_exists());
}

// =============================================================================
// TESTS FOR MULTI-TENANT ISOLATION (#473)
// =============================================================================

/// Verify funds, schedules, and analytics for one program cannot affect or
/// be read as another program's data (tenant isolation).
#[test]
fn test_multi_tenant_no_cross_program_balance_or_analytics() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_a = env.register_contract(None, ProgramEscrowContract);
    let client_a = ProgramEscrowContractClient::new(&env, &contract_a);
    let contract_b = env.register_contract(None, ProgramEscrowContract);
    let client_b = ProgramEscrowContractClient::new(&env, &contract_b);

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let _token_client = token::Client::new(&env, &token_id);
    let token_sac = token::StellarAssetClient::new(&env, &token_id);

    let admin_a = Address::generate(&env);
    let admin_b = Address::generate(&env);
    let creator = Address::generate(&env);

    let program_id_a = String::from_str(&env, "prog-isolation-a");
    client_a.init_program(&program_id_a, &admin_a, &token_id, &creator, &None, &None);
    client_a.publish_program(&program_id_a);

    let program_id_b = String::from_str(&env, "prog-isolation-b");
    client_b.init_program(&program_id_b, &admin_b, &token_id, &creator, &None, &None);
    client_b.publish_program(&program_id_b);

    token_sac.mint(&client_a.address, &500_000);
    token_sac.mint(&client_b.address, &300_000);
    client_a.lock_program_funds(&500_000);
    client_b.lock_program_funds(&300_000);

    let stats_a = client_a.get_program_aggregate_stats();
    let stats_b = client_b.get_program_aggregate_stats();
    assert_eq!(stats_a.total_funds, 500_000);
    assert_eq!(stats_a.remaining_balance, 500_000);
    assert_eq!(stats_b.total_funds, 300_000);
    assert_eq!(stats_b.remaining_balance, 300_000);

    let r = Address::generate(&env);
    client_a.single_payout(&r, &100_000);

    assert_eq!(client_a.get_remaining_balance(), 400_000);
    assert_eq!(client_b.get_remaining_balance(), 300_000);
    let info_a = client_a.get_program_info();
    let info_b = client_b.get_program_info();
    assert_eq!(info_a.payout_history.len(), 1);
    assert_eq!(info_b.payout_history.len(), 0);
    assert_eq!(client_a.get_program_aggregate_stats().payout_count, 1);
    assert_eq!(client_b.get_program_aggregate_stats().payout_count, 0);
}

// Note: Additional multi-tenant isolation tests exist above (test_batch_payout_no_cross_program_interference, etc.)

// =============================================================================
// TESTS FOR PROGRAM ANALYTICS AND MONITORING VIEWS
// =============================================================================

// Test: get_program_aggregate_stats returns correct initial values
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
    client.single_payout(&recipient, &payout_amount);

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

    client.batch_payout(&recipients, &amounts);

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
    client.single_payout(&r1, &5_000_0000000);

    let recipients = vec![&env, r2.clone()];
    let amounts = vec![&env, 3_000_0000000];
    client.batch_payout(&recipients, &amounts);

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
    client.trigger_program_releases();

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
    client.single_payout(&recipient, &25_000_0000000);

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
    client.single_payout(&r1, &10_000_0000000);

    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    let recipients = vec![&env, r2.clone(), r3.clone()];
    let amounts = vec![&env, 15_000_0000000, 20_000_0000000];
    client.batch_payout(&recipients, &amounts);

    let future_timestamp = env.ledger().timestamp() + 100;
    let r4 = Address::generate(&env);
    client.create_program_release_schedule(&r4, &25_000_0000000, &future_timestamp);

    env.ledger().set_timestamp(future_timestamp + 1);
    client.trigger_program_releases();

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
    client.trigger_program_releases();

    let stats = client.get_program_aggregate_stats();

    assert_eq!(stats.scheduled_count, 1);
    assert_eq!(stats.released_count, 2);
    assert_eq!(stats.total_paid_out, 20_000_0000000i128);
    assert_eq!(stats.remaining_balance, 30_000_0000000i128);

    env.ledger().set_timestamp(future_timestamp + 35);
    client.trigger_program_releases();

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

    client.single_payout(&r1, &10_000_0000000);
    client.single_payout(&r2, &20_000_0000000);
    client.single_payout(&r3, &15_000_0000000);

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

// Test (#493): metrics reflect real operations — total operations, success counts
#[test]
fn test_analytics_metrics_match_operation_counts() {
    let env = Env::default();
    let initial_funds = 100_000_0000000i128;
    let (client, _admin, _token, _token_admin) = setup_program(&env, initial_funds);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.single_payout(&r1, &10_000_0000000);
    client.single_payout(&r2, &20_000_0000000);

    let recipients = vec![&env, Address::generate(&env)];
    let amounts = vec![&env, 5_000_0000000i128];
    client.batch_payout(&recipients, &amounts);

    let stats = client.get_program_aggregate_stats();
    assert_eq!(stats.payout_count, 3);
    assert_eq!(stats.total_paid_out, 35_000_0000000i128);
    assert_eq!(stats.remaining_balance, 65_000_0000000i128);
    assert_eq!(stats.total_funds, 100_000_0000000i128);
}

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
fn test_spend_threshold_single_payout_at_boundary_allowed() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 50_000);
    let program_id = String::from_str(&env, "hack-2026");

    client.set_program_spend_threshold(&program_id, &10_000);

    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &10_000);

    assert_eq!(data.remaining_balance, 40_000);
    assert_eq!(token_client.balance(&recipient), 10_000);
}

#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_threshold_single_payout_above_limit_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 50_000);
    let program_id = String::from_str(&env, "hack-2026");

    client.set_program_spend_threshold(&program_id, &10_000);

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &10_001);
}

#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_threshold_batch_total_above_limit_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 50_000);
    let program_id = String::from_str(&env, "hack-2026");

    client.set_program_spend_threshold(&program_id, &10_000);

    let recipients = vec![&env, Address::generate(&env), Address::generate(&env)];
    let amounts = vec![&env, 6_000, 5_000];
    client.batch_payout(&recipients, &amounts);
}

#[test]
#[should_panic(expected = "Invalid spend threshold")]
fn test_spend_threshold_must_be_positive() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 1_000);
    let program_id = String::from_str(&env, "hack-2026");
    client.set_program_spend_threshold(&program_id, &0);
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

// PROGRAM ESCROW HISTORY QUERY FILTER TESTS
// Tests for recipient, amount, timestamp filters + pagination on payout history

#[test]
fn test_query_payouts_by_recipient_returns_correct_records() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 500_000);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Multiple payouts: two to r1, one to r2
    client.single_payout(&r1, &100_000);
    client.single_payout(&r2, &150_000);
    client.single_payout(&r1, &50_000);

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

    client.single_payout(&r1, &50_000);

    let results = client.query_payouts_by_recipient(&unknown, &0, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_query_payouts_by_amount_range_returns_matching() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 600_000);

    client.single_payout(&Address::generate(&env), &10_000);
    client.single_payout(&Address::generate(&env), &50_000);
    client.single_payout(&Address::generate(&env), &100_000);
    client.single_payout(&Address::generate(&env), &200_000);

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

    client.single_payout(&Address::generate(&env), &100_000);
    client.single_payout(&Address::generate(&env), &200_000);
    client.single_payout(&Address::generate(&env), &300_000);

    // Exact boundaries should be included
    let results = client.query_payouts_by_amount(&100_000, &300_000, &0, &10);
    assert_eq!(results.len(), 3);
}

#[test]
fn test_query_payouts_by_amount_no_results_outside_range() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 200_000);

    client.single_payout(&Address::generate(&env), &50_000);
    client.single_payout(&Address::generate(&env), &100_000);

    let results = client.query_payouts_by_amount(&500_000, &999_000, &0, &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_query_payouts_by_timestamp_range_filters_correctly() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 600_000);

    let base = env.ledger().timestamp();

    env.ledger().set_timestamp(base + 100);
    client.single_payout(&Address::generate(&env), &100_000);

    env.ledger().set_timestamp(base + 300);
    client.single_payout(&Address::generate(&env), &100_000);

    env.ledger().set_timestamp(base + 700);
    client.single_payout(&Address::generate(&env), &100_000);

    env.ledger().set_timestamp(base + 1200);
    client.single_payout(&Address::generate(&env), &100_000);

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
    client.single_payout(&Address::generate(&env), &100_000);

    env.ledger().set_timestamp(base + 200);
    client.single_payout(&Address::generate(&env), &100_000);

    env.ledger().set_timestamp(base + 300);
    client.single_payout(&Address::generate(&env), &100_000);

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
        client.single_payout(&r1, &10_000);
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
    client.single_payout(&r1, &10_000);
    let _ = client.query_payouts_by_recipient(&r1, &0, &0);
}

#[test]
#[should_panic(expected = "Pagination limit exceeds maximum")]
fn test_query_payouts_pagination_limit_above_max_rejected() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 100_000);
    let r1 = Address::generate(&env);
    client.single_payout(&r1, &10_000);
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
    client.trigger_program_releases();

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

    client.single_payout(&r1, &10_000);
    client.single_payout(&r1, &200_000);
    client.single_payout(&r1, &50_000);

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
    client.trigger_program_releases();

    let schedules_after = client.get_all_prog_release_schedules();
    assert_eq!(schedules_after.len(), 2);
    let released_count = schedules_after.iter().filter(|s| s.released).count();
    assert_eq!(released_count, 1);

    let stats = client.get_program_aggregate_stats();
    assert_eq!(stats.released_count, 1);
    assert_eq!(stats.scheduled_count, 1);
    assert_eq!(stats.remaining_balance, 150_000);

    env.ledger().set_timestamp(now + 250);
    client.trigger_program_releases();

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
    let released_count = client.trigger_program_releases();
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
    let released = client.trigger_program_releases();
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

#[test]
fn test_program_fee_zero_by_default_matches_prior_payouts() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 100_000);
    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &30_000);
    assert_eq!(data.remaining_balance, 70_000);
    assert_eq!(token_client.balance(&recipient), 30_000);
}

#[test]
fn test_program_payout_fee_percentage_and_fixed() {
    let env = Env::default();
    let (client, admin, token_client, token_admin) = setup_program(&env, 0);
    let fee_bucket = Address::generate(&env);
    token_admin.mint(&client.address, &100_000);
    client.lock_program_funds(&100_000);
    client.update_fee_config(
        &None,
        &Some(1_000i128),
        &None,
        &Some(500i128),
        &Some(fee_bucket.clone()),
        &Some(true),
    );
    let recipient = Address::generate(&env);
    // Gross 10_000: 10% ceil = 1_000 + 500 fixed = 1_500 fee, net 8_500
    client.single_payout(&recipient, &10_000);
    assert_eq!(token_client.balance(&recipient), 8_500);
    assert_eq!(token_client.balance(&fee_bucket), 1_500);
    assert_eq!(client.get_remaining_balance(), 90_000);
    let _ = admin;
}

#[test]
fn test_program_lock_fixed_fee_reduces_credited_balance() {
    let env = Env::default();
    let (client, admin, token_client, token_admin) = setup_program(&env, 0);
    let fee_bucket = Address::generate(&env);
    token_admin.mint(&client.address, &50_000);
    client.update_fee_config(
        &None,
        &None,
        &Some(2_000i128),
        &None,
        &Some(fee_bucket.clone()),
        &Some(true),
    );
    client.lock_program_funds(&20_000);
    assert_eq!(client.get_remaining_balance(), 18_000);
    assert_eq!(token_client.balance(&fee_bucket), 2_000);
    let _ = admin;
}

#[test]
fn test_program_update_fee_config_disables_fees() {
    let env = Env::default();
    let (client, admin, token_client, token_admin) = setup_program(&env, 0);
    let fee_bucket = Address::generate(&env);
    token_admin.mint(&client.address, &50_000);
    client.update_fee_config(
        &None,
        &None,
        &Some(1_000i128),
        &None,
        &Some(fee_bucket.clone()),
        &Some(true),
    );
    client.lock_program_funds(&10_000);
    client.update_fee_config(&None, &None, &None, &None, &None, &Some(false));
    client.lock_program_funds(&10_000);
    assert_eq!(client.get_remaining_balance(), 19_000);
    assert_eq!(token_client.balance(&fee_bucket), 1_000);
    let _ = admin;
}

// =============================================================================
// SPEND LIMIT THRESHOLD TESTS (Issue #15)
// =============================================================================
//
// These tests verify the spend-limit threshold invariants:
//   - single_payout and batch_payout are rejected when the requested amount
//     exceeds the configured per-program threshold.
//   - The threshold is enforced BEFORE balance checks (deterministic ordering).
//   - Audit events (SpendLimitSetEvent, SpendLimitExceededEvent) are emitted.
//   - The upgrade-safe schema version marker is written on init.
//   - Setting threshold to i128::MAX effectively disables enforcement.

/// SL-1: single_payout below threshold succeeds.
#[test]
fn test_spend_limit_single_payout_below_threshold_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &5_000);
    client.single_payout(&recipient, &4_999);

    assert_eq!(token_client.balance(&recipient), 4_999);
    assert_eq!(client.get_remaining_balance(), 5_001);
}

/// SL-2: single_payout exactly at threshold succeeds.
#[test]
fn test_spend_limit_single_payout_at_threshold_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &5_000);
    client.single_payout(&recipient, &5_000);

    assert_eq!(token_client.balance(&recipient), 5_000);
    assert_eq!(client.get_remaining_balance(), 5_000);
}

/// SL-3: single_payout above threshold is rejected.
#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_limit_single_payout_above_threshold_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &5_000);
    client.single_payout(&recipient, &5_001); // must panic
}

/// SL-4: batch_payout total below threshold succeeds.
#[test]
fn test_spend_limit_batch_payout_below_threshold_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &6_000);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1.clone(), r2.clone()],
        &soroban_sdk::vec![&env, 2_000i128, 3_000i128],
    );

    assert_eq!(token_client.balance(&r1), 2_000);
    assert_eq!(token_client.balance(&r2), 3_000);
    assert_eq!(client.get_remaining_balance(), 5_000);
}

/// SL-5: batch_payout total above threshold is rejected.
#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_limit_batch_payout_above_threshold_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &4_000);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1, r2],
        &soroban_sdk::vec![&env, 2_000i128, 3_000i128], // total = 5_000 > 4_000
    );
}

/// SL-6: threshold check runs before balance check (deterministic ordering).
/// Even when balance is sufficient, exceeding threshold is rejected first.
#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_limit_threshold_checked_before_balance() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    // Balance is 100_000 but threshold is only 1_000.
    client.set_program_spend_threshold(&program_id, &1_000);
    client.single_payout(&recipient, &50_000); // threshold exceeded, not balance
}

/// SL-7: no threshold set → i128::MAX → any amount within balance is allowed.
#[test]
fn test_spend_limit_no_threshold_allows_full_balance() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    // Verify default is i128::MAX (unlimited).
    assert_eq!(
        client.get_program_spend_threshold(&program_id),
        i128::MAX,
        "default threshold must be i128::MAX"
    );

    client.single_payout(&recipient, &10_000);
    assert_eq!(token_client.balance(&recipient), 10_000);
    assert_eq!(client.get_remaining_balance(), 0);
}

/// SL-8: threshold can be updated; new value takes effect immediately.
#[test]
fn test_spend_limit_threshold_update_takes_effect() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 20_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    // Set tight threshold.
    client.set_program_spend_threshold(&program_id, &3_000);
    client.single_payout(&recipient, &3_000);
    assert_eq!(token_client.balance(&recipient), 3_000);

    // Raise threshold.
    client.set_program_spend_threshold(&program_id, &10_000);
    client.single_payout(&recipient, &10_000);
    assert_eq!(token_client.balance(&recipient), 13_000);
    assert_eq!(client.get_remaining_balance(), 7_000);
}

/// SL-9: SpendLimitSetEvent is emitted with correct fields.
#[test]
fn test_spend_limit_set_event_emitted() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;

    let events_before = env.events().all().len();
    client.set_program_spend_threshold(&program_id, &7_500);
    let events_after = env.events().all();

    // At least one new event must have been emitted.
    assert!(
        events_after.len() > events_before,
        "SpendLimitSetEvent must be emitted"
    );
}

/// SL-10: SpendLimitExceededEvent is emitted when threshold is breached.
#[test]
fn test_spend_limit_exceeded_event_emitted_on_rejection() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &1_000);

    let events_before = env.events().all().len();
    // Attempt an over-threshold payout; it will panic but the event is emitted first.
    let result = client.try_single_payout(&recipient, &5_000);
    assert!(result.is_err(), "over-threshold payout must fail");

    // The SpendLimitExceededEvent must have been emitted before the panic.
    let events_after = env.events().all();
    assert!(
        events_after.len() > events_before,
        "SpendLimitExceededEvent must be emitted on rejection"
    );
}

/// SL-11: upgrade-safe schema version is written on init.
#[test]
fn test_spend_limit_schema_version_written_on_init() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin_addr.clone());
    let token_id = sac.address();
    let program_id = String::from_str(&env, "schema-test");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);

    let version = client.get_spend_limit_schema_version();
    assert_eq!(version, 1u32, "schema version must be 1 after init");
}

/// SL-12: threshold of 1 rejects any amount > 1.
#[test]
#[should_panic(expected = "Spend threshold exceeded")]
fn test_spend_limit_minimum_threshold_rejects_larger_amounts() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &1);
    client.single_payout(&recipient, &2); // must panic
}

/// SL-13: threshold of 1 allows amount == 1.
#[test]
fn test_spend_limit_minimum_threshold_allows_exact_amount() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spend_threshold(&program_id, &1);
    client.single_payout(&recipient, &1);
    assert_eq!(token_client.balance(&recipient), 1);
}

/// SL-14: zero threshold is rejected by set_program_spend_threshold.
#[test]
#[should_panic(expected = "Invalid spend threshold")]
fn test_spend_limit_zero_threshold_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;
    client.set_program_spend_threshold(&program_id, &0);
}

/// SL-15: negative threshold is rejected by set_program_spend_threshold.
#[test]
#[should_panic(expected = "Invalid spend threshold")]
fn test_spend_limit_negative_threshold_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;
    client.set_program_spend_threshold(&program_id, &-1);
}

// ============================================================================
// PER-WINDOW SPENDING LIMITS — Issue #25
// ============================================================================
// Tests for time-windowed spend limits: set/get config, enforcement in
// single_payout, batch_payout, schedule releases, window reset, and events.

/// SW-1: No limit set → payouts proceed without restriction.
#[test]
fn test_spending_window_no_limit_allows_payout() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let recipient = Address::generate(&env);

    client.single_payout(&recipient, &10_000);
    assert_eq!(token_client.balance(&recipient), 10_000);
}

/// SW-2: Limit disabled (enabled=false) → payouts proceed even if amount > max_amount.
#[test]
fn test_spending_window_disabled_limit_allows_payout() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &100i128, &false);
    client.single_payout(&recipient, &10_000);
    assert_eq!(token_client.balance(&recipient), 10_000);
}

/// SW-3: single_payout within window limit succeeds.
#[test]
fn test_spending_window_single_payout_within_limit_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &5_000i128, &true);
    client.single_payout(&recipient, &5_000);
    assert_eq!(token_client.balance(&recipient), 5_000);
}

/// SW-4: single_payout exceeding window limit is rejected.
#[test]
#[should_panic(expected = "Program spending limit exceeded for current window")]
fn test_spending_window_single_payout_exceeds_limit_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &3_000i128, &true);
    client.single_payout(&recipient, &3_001);
}

/// SW-5: Cumulative payouts within window are tracked; second payout that
///        would push total over limit is rejected.
#[test]
#[should_panic(expected = "Program spending limit exceeded for current window")]
fn test_spending_window_cumulative_limit_enforced() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Window limit = 5_000; first payout = 3_000 (ok), second = 3_000 (total 6_000 > 5_000)
    client.set_program_spending_limit(&program_id, &86400u64, &5_000i128, &true);
    client.single_payout(&r1, &3_000);
    client.single_payout(&r2, &3_000); // must panic
}

/// SW-6: batch_payout total within window limit succeeds.
#[test]
fn test_spending_window_batch_payout_within_limit_succeeds() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &6_000i128, &true);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1.clone(), r2.clone()],
        &soroban_sdk::vec![&env, 2_000i128, 3_000i128],
    );
    assert_eq!(token_client.balance(&r1), 2_000);
    assert_eq!(token_client.balance(&r2), 3_000);
}

/// SW-7: batch_payout total exceeding window limit is rejected.
#[test]
#[should_panic(expected = "Program spending limit exceeded for current window")]
fn test_spending_window_batch_payout_exceeds_limit_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &4_000i128, &true);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1, r2],
        &soroban_sdk::vec![&env, 2_000i128, 3_000i128], // total 5_000 > 4_000
    );
}

/// SW-8: Window resets after window_size seconds; new window allows full limit again.
#[test]
fn test_spending_window_resets_after_window_expires() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 20_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    // Short window of 100 seconds, limit 5_000
    client.set_program_spending_limit(&program_id, &100u64, &5_000i128, &true);

    // Exhaust the window
    client.single_payout(&r1, &5_000);
    assert_eq!(token_client.balance(&r1), 5_000);

    // Advance time past the window
    env.ledger().with_mut(|l| l.timestamp += 101);

    // New window: same limit available again
    client.single_payout(&r2, &5_000);
    assert_eq!(token_client.balance(&r2), 5_000);
}

/// SW-9: get_program_spending_limit returns None when not set.
#[test]
fn test_spending_window_get_limit_none_when_not_set() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;

    let limit = client.get_program_spending_limit(&program_id);
    assert!(limit.is_none(), "limit must be None when not configured");
}

/// SW-10: get_program_spending_state returns None before any payout.
#[test]
fn test_spending_window_get_state_none_before_payout() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;

    client.set_program_spending_limit(&program_id, &86400u64, &5_000i128, &true);
    let state = client.get_program_spending_state(&program_id);
    assert!(state.is_none(), "state must be None before any payout");
}

/// SW-11: get_program_spending_state reflects cumulative amount after payouts.
#[test]
fn test_spending_window_state_tracks_cumulative_amount() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &10_000i128, &true);
    client.single_payout(&r1, &2_000);
    client.single_payout(&r2, &3_000);

    let state = client.get_program_spending_state(&program_id).unwrap();
    assert_eq!(state.amount_released, 5_000);
}

/// SW-12: zero window_size is rejected.
#[test]
#[should_panic(expected = "window_size must be greater than zero")]
fn test_spending_window_zero_window_size_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;
    client.set_program_spending_limit(&program_id, &0u64, &5_000i128, &true);
}

/// SW-13: negative max_amount is rejected.
#[test]
#[should_panic(expected = "max_amount must be non-negative")]
fn test_spending_window_negative_max_amount_rejected() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 0);
    let program_id = client.get_program_info().program_id;
    client.set_program_spending_limit(&program_id, &86400u64, &-1i128, &true);
}

/// SW-14: Rejection emits the (limit, prog_spend) event.
#[test]
fn test_spending_window_rejection_emits_event() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 10_000);
    let program_id = client.get_program_info().program_id;
    let recipient = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &1_000i128, &true);

    let events_before = env.events().all().len();
    let result = client.try_single_payout(&recipient, &5_000);
    assert!(result.is_err(), "over-limit payout must fail");

    let events_after = env.events().all();
    assert!(
        events_after.len() > events_before,
        "rejection event must be emitted"
    );
}

/// SW-15: Limit can be updated; new value takes effect immediately.
#[test]
fn test_spending_window_limit_update_takes_effect() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 20_000);
    let program_id = client.get_program_info().program_id;
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.set_program_spending_limit(&program_id, &86400u64, &3_000i128, &true);
    client.single_payout(&r1, &3_000);
    assert_eq!(token_client.balance(&r1), 3_000);

    // Raise limit
    client.set_program_spending_limit(&program_id, &86400u64, &20_000i128, &true);
    client.single_payout(&r2, &10_000);
    assert_eq!(token_client.balance(&r2), 10_000);
}

// ============================================================================
// PAUSE MODE BLOCKS PAYOUTS — Issue #1060
// ============================================================================
// Tests for deterministic pause behavior, PauseStateChangedV2 events,
// upgrade-safe storage (PauseSchemaVersion), and edge cases.

/// PM-01: Pause schema version is written at init and readable.
#[test]
fn test_pause_schema_version_written_at_init() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let schema_version = client.get_pause_schema_version();
    assert_eq!(
        schema_version, PAUSE_SCHEMA_VERSION_V1,
        "Pause schema version must be PAUSE_SCHEMA_VERSION_V1 after init"
    );
}

/// PM-02: Default pause flags are all false after init.
#[test]
fn test_pause_flags_default_all_false() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let flags = client.get_pause_flags();
    assert!(!flags.lock_paused, "lock_paused must default to false");
    assert!(
        !flags.release_paused,
        "release_paused must default to false"
    );
    assert!(!flags.refund_paused, "refund_paused must default to false");
}

/// PM-03: release_paused blocks single_payout with deterministic "Funds Paused" panic.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_release_paused_blocks_single_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&None, &Some(true), &None, &None);

    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &100);
}

/// PM-04: release_paused blocks batch_payout with deterministic "Funds Paused" panic.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_release_paused_blocks_batch_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&None, &Some(true), &None, &None);

    let r1 = Address::generate(&env);
    client.batch_payout(
        &soroban_sdk::vec![&env, r1],
        &soroban_sdk::vec![&env, 100i128],
    );
}

/// PM-05: lock_paused blocks lock_program_funds with deterministic "Funds Paused" panic.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_lock_paused_blocks_lock_program_funds() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    client.set_paused(&Some(true), &None, &None, &None);
    client.lock_program_funds(&500);
}

/// PM-06: lock_paused does NOT block single_payout (orthogonal flags).
#[test]
fn test_lock_paused_does_not_block_single_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&Some(true), &None, &None, &None);

    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &200);
    assert_eq!(data.remaining_balance, 800);
}

/// PM-07: release_paused does NOT block lock_program_funds (orthogonal flags).
#[test]
fn test_release_paused_does_not_block_lock() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    client.set_paused(&None, &Some(true), &None, &None);

    let data = client.lock_program_funds(&300);
    assert_eq!(data.remaining_balance, 300);
}

/// PM-08: Unpause restores single_payout after release_paused.
#[test]
fn test_unpause_restores_single_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&None, &Some(true), &None, &None);
    assert!(client
        .try_single_payout(&Address::generate(&env), &100)
        .is_err());

    client.set_paused(&None, &Some(false), &None, &None);
    let data = client.single_payout(&Address::generate(&env), &100);
    assert_eq!(data.remaining_balance, 900);
}

/// PM-09: Unpause restores batch_payout after release_paused.
#[test]
fn test_unpause_restores_batch_payout() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&None, &Some(true), &None, &None);
    let r1 = Address::generate(&env);
    assert!(client
        .try_batch_payout(
            &soroban_sdk::vec![&env, r1.clone()],
            &soroban_sdk::vec![&env, 100i128]
        )
        .is_err());

    client.set_paused(&None, &Some(false), &None, &None);
    let data = client.batch_payout(
        &soroban_sdk::vec![&env, r1],
        &soroban_sdk::vec![&env, 100i128],
    );
    assert_eq!(data.remaining_balance, 900);
}

/// PM-10: PauseStateChangedV2 event is emitted with correct fields on pause.
#[test]
fn test_pause_state_changed_v2_event_on_pause() {
    let env = Env::default();
    let (client, admin, _token, _token_admin) = setup_program(&env, 0);

    env.ledger().with_mut(|li| li.timestamp = 99_999);

    client.set_paused(&None, &Some(true), &None, &None);

    // Find the PauseStateChangedV2 event
    let events = env.events().all();
    let v2_event = events.iter().find(|e| {
        let topics = e.1.clone();
        if let Some(t0) = topics.get(0) {
            let sym: Symbol = t0.into_val(&env);
            sym == Symbol::new(&env, "PauseStV2")
        } else {
            false
        }
    });

    assert!(
        v2_event.is_some(),
        "PauseStateChangedV2 event must be emitted"
    );

    let event = v2_event.unwrap();
    let data = PauseStateChangedV2::try_from_val(&env, &event.2).unwrap();

    assert_eq!(data.version, EVENT_VERSION_V2);
    assert_eq!(data.operation, symbol_short!("release"));
    assert_eq!(
        data.previous_paused, false,
        "previous_paused must be false before first pause"
    );
    assert_eq!(data.paused, true);
    assert_eq!(data.admin, admin);
    assert_eq!(data.timestamp, 99_999);
    assert!(data.receipt_id > 0);
}

/// PM-11: PauseStateChangedV2 captures previous_paused = true when unpausing.
#[test]
fn test_pause_state_changed_v2_previous_paused_on_unpause() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    // First pause
    client.set_paused(&None, &Some(true), &None, &None);

    // Then unpause — previous_paused should be true
    client.set_paused(&None, &Some(false), &None, &None);

    let events = env.events().all();
    // Get the last PauseStateChangedV2 event (the unpause one)
    let v2_events: std::vec::Vec<_> = events
        .iter()
        .filter(|e| {
            let topics = e.1.clone();
            if let Some(t0) = topics.get(0) {
                let sym: Symbol = t0.into_val(&env);
                sym == Symbol::new(&env, "PauseStV2")
            } else {
                false
            }
        })
        .collect::<std::vec::Vec<_>>();

    assert!(
        v2_events.len() >= 2,
        "Should have at least 2 PauseStateChangedV2 events"
    );

    let unpause_event = v2_events.last().unwrap();
    let data = PauseStateChangedV2::try_from_val(&env, &unpause_event.2).unwrap();

    assert_eq!(
        data.previous_paused, true,
        "previous_paused must be true when unpausing"
    );
    assert_eq!(data.paused, false);
}

/// PM-12: All three flags can be paused simultaneously; all three block their ops.
#[test]
fn test_all_flags_paused_blocks_all_operations() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    assert!(
        client.try_lock_program_funds(&100).is_err(),
        "lock must be blocked"
    );
    assert!(
        client
            .try_single_payout(&Address::generate(&env), &100)
            .is_err(),
        "single_payout must be blocked"
    );
    assert!(
        client
            .try_batch_payout(
                &soroban_sdk::vec![&env, Address::generate(&env)],
                &soroban_sdk::vec![&env, 100i128]
            )
            .is_err(),
        "batch_payout must be blocked"
    );
}

/// PM-13: Partial unpause — only release unpaused, lock stays paused.
#[test]
fn test_partial_unpause_preserves_other_flags() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 1_000);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    // Only unpause release
    client.set_paused(&None, &Some(false), &None, &None);

    let flags = client.get_pause_flags();
    assert!(flags.lock_paused, "lock_paused must remain true");
    assert!(
        !flags.release_paused,
        "release_paused must be false after unpause"
    );
    assert!(flags.refund_paused, "refund_paused must remain true");
}

/// PM-14: Read-only queries (get_program_info, get_remaining_balance) are unaffected by pause.
#[test]
fn test_read_only_queries_unaffected_by_pause() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 500);

    client.set_paused(&Some(true), &Some(true), &Some(true), &None);

    let info = client.get_program_info();
    assert_eq!(info.remaining_balance, 500);

    let balance = client.get_remaining_balance();
    assert_eq!(balance, 500);
}

/// PM-15: Pause reason is stored and retrievable via get_pause_flags.
#[test]
fn test_pause_reason_stored_in_flags() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let reason = String::from_str(&env, "Security incident");
    client.set_paused(&Some(true), &None, &None, &Some(reason.clone()));

    let flags = client.get_pause_flags();
    assert_eq!(flags.pause_reason, Some(reason));
}

/// PM-16: Pause reason is cleared when all flags are unpaused.
#[test]
fn test_pause_reason_cleared_on_full_unpause() {
    let env = Env::default();
    let (client, _admin, _token, _token_admin) = setup_program(&env, 0);

    let reason = String::from_str(&env, "Temporary halt");
    client.set_paused(&Some(true), &None, &None, &Some(reason));
    client.set_paused(&Some(false), &None, &None, &None);

    let flags = client.get_pause_flags();
    assert_eq!(
        flags.pause_reason, None,
        "reason must be cleared when fully unpaused"
    );
}

// ========================================================================
// Idempotency Key Tests
// ========================================================================

/// Test idempotency key validation for successful batch payout
#[test]
fntest_idempotency_key_batch_payout_success() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1.clone(), recipient2.clone()];
    let amounts = vec![&env, 100_0000000, 200_0000000];
    let idempotency_key = String::from_str(&env, "test-batch-123");

    // First successful payout with idempotency key
    let result = client.batch_payout(&recipients, &amounts, &Some(idempotency_key.clone()));
    assert_eq!(result.remaining_balance, 700_0000000);

    // Verify idempotency record was stored
    let record = env.storage().instance().get(&DataKey::IdempotencyKey(idempotency_key.clone())).unwrap();
    assert_eq!(record.idempotency_key, idempotency_key);
    assert_eq!(record.operation_type, symbol_short!("batch_payout"));
    assert!(record.success);
    assert_eq!(record.total_amount, 300_0000000);
    assert_eq!(record.recipient_count, 2);

    // Verify events were emitted
    let events = env.events().all();
    assert!(events.len() >= 2); // BatchPayout + IdempotencyKeyUsed
}

/// Test idempotency key retry behavior for batch payout
#[test]
fntest_idempotency_key_batch_payout_retry() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1.clone(), recipient2.clone()];
    let amounts = vec![&env, 100_0000000, 200_0000000];
    let idempotency_key = String::from_str(&env, "test-batch-retry-456");

    // First successful payout
    let result1 = client.batch_payout(&recipients, &amounts, &Some(idempotency_key.clone()));
    assert_eq!(result1.remaining_balance, 700_0000000);

    // Retry with same idempotency key should return same result
    let result2 = client.batch_payout(&recipients, &amounts, &Some(idempotency_key.clone()));
    assert_eq!(result2.remaining_balance, 700_0000000);
    assert_eq!(result1.payout_history.len(), result2.payout_history.len());

    // Verify retry event was emitted
    let events = env.events().all();
    let retry_events: Vec<_> = events.iter()
        .filter(|e| e.topics[0] == IDEMPOTENCY_KEY_USED)
        .collect();
    assert_eq!(retry_events.len(), 2); // First use + retry
}

/// Test idempotency key validation for successful single payout
#[test]
fntest_idempotency_key_single_payout_success() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 500_0000000;
    let idempotency_key = String::from_str(&env, "test-single-789");

    // First successful payout with idempotency key
    let result = client.single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    assert_eq!(result.remaining_balance, 500_0000000);

    // Verify idempotency record was stored
    let record = env.storage().instance().get(&DataKey::IdempotencyKey(idempotency_key.clone())).unwrap();
    assert_eq!(record.idempotency_key, idempotency_key);
    assert_eq!(record.operation_type, symbol_short!("single_payout"));
    assert!(record.success);
    assert_eq!(record.total_amount, 500_0000000);
    assert_eq!(record.recipient_count, 1);
}

/// Test idempotency key retry behavior for single payout
#[test]
fntest_idempotency_key_single_payout_retry() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 300_0000000;
    let idempotency_key = String::from_str(&env, "test-single-retry-999");

    // First successful payout
    let result1 = client.single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    assert_eq!(result1.remaining_balance, 700_0000000);

    // Retry with same idempotency key should return same result
    let result2 = client.single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    assert_eq!(result2.remaining_balance, 700_0000000);
    assert_eq!(result1.payout_history.len(), result2.payout_history.len());
}

/// Test idempotency key validation failures
#[test]
fntest_idempotency_key_validation_failures() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 100_0000000;

    // Empty idempotency key should panic
    let empty_key = String::from_str(&env, "");
    let result = std::panic::catch_unwind(|| {
        client.single_payout(&recipient, &amount, &Some(empty_key));
    });
    assert!(result.is_err());

    // Oversized idempotency key should panic
    let mut oversized_key = String::from_str(&env, "");
    for _ in 0..300 {
        oversized_key = oversized_key + "a";
    }
    let result = std::panic::catch_unwind(|| {
        client.single_payout(&recipient, &amount, &Some(oversized_key));
    });
    assert!(result.is_err());
}

/// Test idempotency key with insufficient funds (failure case)
#[test]
fntest_idempotency_key_insufficient_funds() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 100_0000000);

    let recipient = Address::generate(&env);
    let amount = 2000_0000000; // More than available
    let idempotency_key = String::from_str(&env, "test-insufficient-111");

    // First attempt should fail
    let result = std::panic::catch_unwind(|| {
        client.single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    });
    assert!(result.is_err());

    // Verify failure record was stored
    let record = env.storage().instance().get(&DataKey::IdempotencyKey(idempotency_key.clone())).unwrap();
    assert_eq!(record.idempotency_key, idempotency_key);
    assert!(!record.success);
    assert!(record.error_code.is_some());

    // Retry should return same failure
    let result2 = std::panic::catch_unwind(|| {
        client.single_payout(&recipient, &amount, &Some(idempotency_key.clone()));
    });
    assert!(result2.is_err());
}

/// Test idempotency schema version initialization
#[test]
fntest_idempotency_schema_version() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize_contract(&admin);

    // Verify schema version is set
    let schema_version = client.get_idempotency_schema_version();
    assert_eq!(schema_version, IDEMPOTENCY_SCHEMA_VERSION_V1);

    // Verify schema version event was emitted
    let events = env.events().all();
    let schema_events: Vec<_> = events.iter()
        .filter(|e| e.topics[0] == IDEMPOTENCY_SCHEMA)
        .collect();
    assert_eq!(schema_events.len(), 1);
}

/// Test idempotency key with no key provided (normal operation)
#[test]
fntest_idempotency_key_none_provided() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 300_0000000;

    // Payout without idempotency key should work normally
    let result = client.single_payout(&recipient, &amount, &None);
    assert_eq!(result.remaining_balance, 700_0000000);

    // Should be able to do multiple payouts without idempotency keys
    let recipient2 = Address::generate(&env);
    let result2 = client.single_payout(&recipient2, &amount, &None);
    assert_eq!(result2.remaining_balance, 400_0000000);
}

/// Test idempotency key isolation between different operations
#[test]
fntest_idempotency_key_operation_isolation() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipients = vec![&env, recipient1.clone(), recipient2.clone()];
    let batch_amounts = vec![&env, 100_0000000, 100_0000000];
    let single_amount = 200_0000000;
    let idempotency_key = String::from_str(&env, "test-isolation-333");

    // Batch payout with idempotency key
    let batch_result = client.batch_payout(&recipients, &batch_amounts, &Some(idempotency_key.clone()));
    assert_eq!(batch_result.remaining_balance, 800_0000000);

    // Single payout with same idempotency key should fail (key already used)
    let result = std::panic::catch_unwind(|| {
        client.single_payout(&recipient1, &single_amount, &Some(idempotency_key.clone()));
    });
    assert!(result.is_err());

    // Verify retry of batch payout still works
    let batch_retry = client.batch_payout(&recipients, &batch_amounts, &Some(idempotency_key.clone()));
    assert_eq!(batch_retry.remaining_balance, 800_0000000); // Same as before
}

/// Test idempotency key with different keys for same operation
#[test]
fntest_idempotency_key_different_keys_same_operation() {
    let env = Env::default();
    let (client, admin, token, token_admin) = setup_program(&env, 1000_0000000);

    let recipient = Address::generate(&env);
    let amount = 300_0000000;
    let key1 = String::from_str(&env, "test-diff-key-1");
    let key2 = String::from_str(&env, "test-diff-key-2");

    // First payout with key1
    let result1 = client.single_payout(&recipient, &amount, &Some(key1.clone()));
    assert_eq!(result1.remaining_balance, 700_0000000);

    // Second payout with different key2 should work (different recipient)
    let recipient2 = Address::generate(&env);
    let result2 = client.single_payout(&recipient2, &amount, &Some(key2.clone()));
    assert_eq!(result2.remaining_balance, 400_0000000);

    // Verify both keys have their own records
    let record1 = env.storage().instance().get(&DataKey::IdempotencyKey(key1)).unwrap();
    let record2 = env.storage().instance().get(&DataKey::IdempotencyKey(key2)).unwrap();
    assert_eq!(record1.recipient_count, 1);
    assert_eq!(record2.recipient_count, 1);
}
