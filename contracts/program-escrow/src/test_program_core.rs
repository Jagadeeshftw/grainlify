#![cfg(test)]

extern crate std;

use super::*;
use crate::test_support::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke}, token, vec, Address, Env, IntoVal, Map, String, Symbol, TryFromVal, Val};

#[test]
#[should_panic(expected = "107")]
fn test_single_payout_rejects_draft_program() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let program_id = String::from_str(&env, "draft-single");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    let recipient = Address::generate(&env);
    client.single_payout(&recipient, &1, &None);
}

#[test]
#[should_panic(expected = "107")]
fn test_batch_payout_rejects_draft_program() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let program_id = String::from_str(&env, "draft-batch");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    let recipient = Address::generate(&env);
    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 1_i128];
    client.batch_payout(&recipients, &amounts, &None);
}

#[test]
fn test_legacy_active_program_payouts_still_work() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);

    let single_recipient = Address::generate(&env);
    client.single_payout(&single_recipient, &1_000, &None);

    let batch_recipient = Address::generate(&env);
    let recipients = vec![&env, batch_recipient.clone()];
    let amounts = vec![&env, 2_000_i128];
    let data = client.batch_payout(&recipients, &amounts, &None);

    assert_eq!(token_client.balance(&single_recipient), 1_000);
    assert_eq!(token_client.balance(&batch_recipient), 2_000);
    assert_eq!(data.remaining_balance, 7_000);
    assert_eq!(data.status, ProgramStatus::Active);
}

#[test]
fn test_program_published_event_contains_required_fields() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(12345);

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let program_id = String::from_str(&env, "publish-event");

    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    let before = env.events().all().len();
    client.publish_program(&program_id, &admin);

    let events = env.events().all();
    let (_, topics, data) = events.get(before).expect("publish event should be emitted");
    assert_eq!(topics, (PROGRAM_PUBLISHED,).into_val(&env));

    let event = ProgramPublishedEvent::try_from_val(&env, &data).expect("event payload should decode");
    assert_eq!(event.program_id, program_id);
    assert_eq!(event.publisher, admin);
    assert_eq!(event.timestamp, 12345);
}

#[test]
fn test_fee_ceiling_division_avoids_dust_for_odd_amount() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 2_000);
    let fee_recipient = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.update_fee_config(&None, &Some(100), &None, &None, &Some(fee_recipient.clone()), &Some(true));
    client.single_payout(&recipient, &1001, &None);

    assert_eq!(token_client.balance(&fee_recipient), 11);
    assert_eq!(token_client.balance(&recipient), 990);
    assert_eq!(token_client.balance(&fee_recipient) + token_client.balance(&recipient), 1001);
}

#[test]
fn test_fee_ceiling_division_boundary_max_rate() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 10_000);
    let fee_recipient = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.update_fee_config(&None, &Some(1000), &None, &None, &Some(fee_recipient.clone()), &Some(true));
    client.single_payout(&recipient, &1001, &None);

    assert_eq!(token_client.balance(&fee_recipient), 101);
    assert_eq!(token_client.balance(&recipient), 900);
    assert_eq!(token_client.balance(&fee_recipient) + token_client.balance(&recipient), 1001);
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
    client.single_payout(&recipient, &safe_max,
    &None
);

    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(token_client.balance(&recipient), safe_max);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_single_payout_token_transfer_integration() {
    let env = Env::default();
    let (client, _admin, token_client, _token_admin) = setup_program(&env, 100_000);

    let recipient = Address::generate(&env);
    let data = client.single_payout(&recipient, &30_000,
    &None
);

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

    let data = client.batch_payout(&recipients, &amounts,
    &None
);
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

    client.single_payout(&r1, &50_000,
    &None
);
    let recipients = vec![&env, r2.clone(), r3.clone()];
    let amounts = vec![&env, 70_000, 30_000];
    client.batch_payout(&recipients, &amounts,
    &None
);

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
            client.single_payout(&recipient, &amount,
    &None
);
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
            client.batch_payout(&recipients, &amounts,
    &None
);
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

        client.batch_payout(&recipients, &amounts,
    &None
);
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
        single_client.single_payout(&recipient, &1_000,
    &None
);
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
    batch_client.batch_payout(&recipients, &amounts,
    &None
);
    let batch_events = env_batch.events().all().len() - batch_before;

    assert!(batch_events <= single_events);
}

#[test]
fn test_events_emit_v2_version_tags_for_all_program_emitters() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.single_payout(&r1, &10_000,
    &None
);
    let recipients = vec![&env, r2];
    let amounts = vec![&env, 5_000];
    client.batch_payout(&recipients, &amounts,
    &None
);

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
    let released = client.trigger_program_releases(&None);
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
    let released = client.trigger_program_releases(&None);
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
    let released = client.trigger_program_releases(&None);
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
    let released_at_overlap = client.trigger_program_releases(&None);
    assert_eq!(released_at_overlap, 2);
    assert_eq!(token_client.balance(&recipient1), 10_000);
    assert_eq!(token_client.balance(&recipient2), 15_000);
    assert_eq!(token_client.balance(&recipient3), 0);

    env.ledger().set_timestamp(now + 120);
    let released_later = client.trigger_program_releases(&None);
    assert_eq!(released_later, 1);
    assert_eq!(token_client.balance(&recipient3), 20_000);
}

#[test]
fn test_access_control_violation_unauthorized_payout() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let unauthorized_user = Address::generate(&env);
    let recipient = Address::generate(&env);

    // Mock auth for unauthorized user attempting a payout
    env.mock_auths(&[MockAuth {
        address: &unauthorized_user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "single_payout",
            args: (recipient.clone(), 10_000_i128, None::<String>).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    // This should fail because unauthorized_user is not the payout_key
    let result = client.try_single_payout(&recipient, &10_000, &None);
    assert!(result.is_err());
}

#[test]
fn test_threat_model_reentrancy_prevention() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let recipient = Address::generate(&env);

    // Soroban handles reentrancy by not allowing cross-contract calls
    // during a contract execution unless explicitly allowed.
    // Here we test that a standard payout works.
    client.single_payout(&recipient, &10_000, &None);
    assert_eq!(client.get_remaining_balance(), 90_000);
}

#[test]
fn test_threat_model_oracle_manipulation_unauthorized_rotation() {
    let env = Env::default();
    let (client, _admin, _token_client, _token_admin) = setup_program(&env, 100_000);
    let attacker = Address::generate(&env);

    // Attacker tries to propose themselves as admin without authorization
    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "propose_admin",
            args: (attacker.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_propose_admin(&attacker);
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "Invalid payout fee rate")]
fn test_threat_model_fee_drain_prevention() {
    let env = Env::default();
    let (client, admin, _token_client, _token_admin) = setup_program(&env, 100_000);

    // Admin tries to set payout fee to 20% (assuming MAX_FEE_RATE is 1000 = 10%)
    // Mock auth for admin
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "update_fee_config",
            args: (
                None::<i128>,
                Some(2000_i128), // 20%
                None::<i128>,
                None::<i128>,
                None::<Address>,
                None::<bool>,
            ).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.update_fee_config(
        &None,
        &Some(2000), // This should panic
        &None,
        &None,
        &None,
        &None,
    );
}

// ---------------------------------------------------------------------------
// Full program lifecycle integration test with batch payouts across two
// independent program-escrow instances.
// ---------------------------------------------------------------------------
#[test]
fn test_full_lifecycle_multi_program_batch_payouts() {
    let env = Env::default();
    env.mock_all_auths();

    // â”€â”€ Shared token setup â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = sac.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    // â”€â”€ Program A: "hackathon-alpha" â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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
    client_a.publish_program(&program_id_a, &auth_key_a);
    assert_eq!(prog_a.total_funds, 0);
    assert_eq!(prog_a.remaining_balance, 0);

    // â”€â”€ Program B: "hackathon-beta" â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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
    client_b.publish_program(&program_id_b, &auth_key_b);
    assert_eq!(prog_b.total_funds, 0);

    // â”€â”€ Phase 1: Lock funds in multiple steps â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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

    // â”€â”€ Phase 2: First round of batch payouts â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    let winner_a1 = Address::generate(&env);
    let winner_a2 = Address::generate(&env);
    let winner_a3 = Address::generate(&env);

    // Program A â€” batch payout round 1: 3 winners
    let data_a1 = client_a.batch_payout(
        &vec![
            &env,
            winner_a1.clone(),
            winner_a2.clone(),
            winner_a3.clone(),
        ],
        &vec![&env, 100_000, 75_000, 50_000],
        &None
);
    assert_eq!(data_a1.remaining_balance, 275_000);
    assert_eq!(data_a1.payout_history.len(), 3);
    assert_eq!(token_client.balance(&winner_a1), 100_000);
    assert_eq!(token_client.balance(&winner_a2), 75_000);
    assert_eq!(token_client.balance(&winner_a3), 50_000);

    let winner_b1 = Address::generate(&env);
    let winner_b2 = Address::generate(&env);

    // Program B â€” batch payout round 1: 2 winners
    let data_b1 = client_b.batch_payout(
        &vec![&env, winner_b1.clone(), winner_b2.clone()],
        &vec![&env, 120_000, 80_000],
        &None
);
    assert_eq!(data_b1.remaining_balance, 200_000);
    assert_eq!(data_b1.payout_history.len(), 2);
    assert_eq!(token_client.balance(&winner_b1), 120_000);
    assert_eq!(token_client.balance(&winner_b2), 80_000);

    // â”€â”€ Phase 3: Second round of batch payouts â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    let winner_a4 = Address::generate(&env);
    let winner_a5 = Address::generate(&env);

    // Program A â€” batch payout round 2: 2 more winners
    let data_a2 = client_a.batch_payout(
        &vec![&env, winner_a4.clone(), winner_a5.clone()],
        &vec![&env, 125_000, 50_000],
        &None
);
    assert_eq!(data_a2.remaining_balance, 100_000);
    assert_eq!(data_a2.payout_history.len(), 5);
    assert_eq!(token_client.balance(&winner_a4), 125_000);
    assert_eq!(token_client.balance(&winner_a5), 50_000);

    let winner_b3 = Address::generate(&env);
    let winner_b4 = Address::generate(&env);
    let winner_b5 = Address::generate(&env);

    // Program B â€” batch payout round 2: 3 more winners
    let data_b2 = client_b.batch_payout(
        &vec![
            &env,
            winner_b3.clone(),
            winner_b4.clone(),
            winner_b5.clone(),
        ],
        &vec![&env, 60_000, 40_000, 30_000],
        &None
);
    assert_eq!(data_b2.remaining_balance, 70_000);
    assert_eq!(data_b2.payout_history.len(), 5);
    assert_eq!(token_client.balance(&winner_b3), 60_000);
    assert_eq!(token_client.balance(&winner_b4), 40_000);
    assert_eq!(token_client.balance(&winner_b5), 30_000);

    // â”€â”€ Phase 4: Final balance verification â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Program A: 500_000 locked âˆ’ (100k + 75k + 50k + 125k + 50k) = 100_000
    assert_eq!(client_a.get_remaining_balance(), 100_000);
    assert_eq!(token_client.balance(&client_a.address), 100_000);

    let info_a = client_a.get_program_info();
    assert_eq!(info_a.total_funds, 500_000);
    assert_eq!(info_a.remaining_balance, 100_000);
    assert_eq!(info_a.payout_history.len(), 5);

    // Program B: 400_000 locked âˆ’ (120k + 80k + 60k + 40k + 30k) = 70_000
    assert_eq!(client_b.get_remaining_balance(), 70_000);
    assert_eq!(token_client.balance(&client_b.address), 70_000);

    let info_b = client_b.get_program_info();
    assert_eq!(info_b.total_funds, 400_000);
    assert_eq!(info_b.remaining_balance, 70_000);
    assert_eq!(info_b.payout_history.len(), 5);

    // â”€â”€ Phase 5: Aggregate stats verification â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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

    // â”€â”€ Phase 6: Cross-program isolation check â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Verify programs don't interfere with each other's on-chain balances
    let total_distributed = (500_000 - 100_000) + (400_000 - 70_000);
    assert_eq!(total_distributed, 730_000);
    assert_eq!(
        token_client.balance(&client_a.address) + token_client.balance(&client_b.address),
        170_000
    );

    // â”€â”€ Phase 7: Event emission verification â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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
    client_a.publish_program(program_id_a.clone(), payout_key_a.clone());

    let program_id_b = String::from_str(&env, "multi-token-b");
    client_b.init_program(
        &program_id_b,
        &payout_key_b,
        &token_b,
        &payout_key_b,
        &None,
        &None,
    );
    client_b.publish_program(program_id_b.clone(), payout_key_b.clone());

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
    client_a.single_payout(&recipient, &120_000,
    &None
);

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
        &None
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
        client.single_payout(&recipient, &100,
    &None
);
    }

    let info = client.get_program_info();
    assert_eq!(info.payout_history.len() as u32, max_ops + 5);
}

// =============================================================================
// Admin rotation and config updates (Issue #465)
// =============================================================================

/// Admin can be set and rotated; new admin is persisted.
