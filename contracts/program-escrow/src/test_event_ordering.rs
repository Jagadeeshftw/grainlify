extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    token, vec, Address, Env, IntoVal, String, Symbol, TryIntoVal, Val,
};

fn setup(
    env: &Env,
    initial_amount: i128,
) -> (
    ProgramEscrowContractClient<'static>,
    Address,
    token::Client<'static>,
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

    let program_id = String::from_str(env, "event-order-test");
    client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
    client.publish_program(&program_id, &admin);

    if initial_amount > 0 {
        token_admin_client.mint(&client.address, &initial_amount);
        client.lock_program_funds(&initial_amount);
    }

    (client, admin, token_client)
}

fn topic_at(env: &Env, event: &(Address, soroban_sdk::Vec<Val>, Val), idx: u32) -> Symbol {
    event.1.get(idx).unwrap().into_val(env)
}

fn events_since(
    env: &Env,
    before: u32,
    contract_id: &Address,
) -> soroban_sdk::Vec<(Address, soroban_sdk::Vec<Val>, Val)> {
    let all = env.events().all();
    let mut result = soroban_sdk::Vec::new(env);
    for i in before..all.len() {
        let ev = all.get(i).unwrap();
        if ev.0 == *contract_id {
            result.push_back(ev);
        }
    }
    result
}

fn topics_from(env: &Env, events: &soroban_sdk::Vec<(Address, soroban_sdk::Vec<Val>, Val)>) -> std::vec::Vec<Symbol> {
    let mut out = std::vec::Vec::new();
    for e in events.iter() {
        out.push(topic_at(env, &e, 0));
    }
    out
}

#[test]
fn test_single_then_batch_payout_event_order() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 10_000);

    let before = env.events().all().len();

    let r1 = Address::generate(&env);
    client.single_payout(&r1, &1_000, &None);

    let r2 = Address::generate(&env);
    let recipients = vec![&env, r2.clone()];
    let amounts = vec![&env, 2_000_i128];
    client.batch_payout(&recipients, &amounts);

    let new_events = events_since(&env, before, &client.address);
    let topics = topics_from(&env, &new_events);

    assert_eq!(topics.len(), 2, "expected 2 payout events, got {}", topics.len());
    assert_eq!(topics[0], Symbol::new(&env, "Payout"), "first event should be Payout");
    assert_eq!(topics[1], Symbol::new(&env, "BatchPay"), "second event should be BatchPay");
}

#[test]
fn test_batch_then_single_payout_event_order() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 10_000);

    let before = env.events().all().len();

    let r1 = Address::generate(&env);
    let recipients = vec![&env, r1.clone()];
    let amounts = vec![&env, 1_500_i128];
    client.batch_payout(&recipients, &amounts);

    let r2 = Address::generate(&env);
    client.single_payout(&r2, &500, &None);

    let new_events = events_since(&env, before, &client.address);
    let topics = topics_from(&env, &new_events);

    assert_eq!(topics.len(), 2, "expected 2 payout events, got {}", topics.len());
    assert_eq!(topics[0], Symbol::new(&env, "BatchPay"), "first event should be BatchPay");
    assert_eq!(topics[1], Symbol::new(&env, "Payout"), "second event should be Payout");
}

#[test]
fn test_interleaved_single_batch_single_batch() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 20_000);

    let before = env.events().all().len();

    let r1 = Address::generate(&env);
    client.single_payout(&r1, &500, &None);

    let r2 = Address::generate(&env);
    client.batch_payout(
        &vec![&env, r2],
        &vec![&env, 1_000_i128],
    );

    let r3 = Address::generate(&env);
    client.single_payout(&r3, &1_500, &None);

    let r4 = Address::generate(&env);
    client.batch_payout(
        &vec![&env, r4],
        &vec![&env, 2_000_i128],
    );

    let new_events = events_since(&env, before, &client.address);
    let topics = topics_from(&env, &new_events);

    assert_eq!(topics.len(), 4, "expected 4 payout events, got {}", topics.len());
    assert_eq!(topics[0], Symbol::new(&env, "Payout"));
    assert_eq!(topics[1], Symbol::new(&env, "BatchPay"));
    assert_eq!(topics[2], Symbol::new(&env, "Payout"));
    assert_eq!(topics[3], Symbol::new(&env, "BatchPay"));
}

#[test]
fn test_pause_between_payouts_preserves_order() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 20_000);

    let before = env.events().all().len();

    let r1 = Address::generate(&env);
    client.single_payout(&r1, &1_000, &None);

    client.set_paused(&Some(true), &None, &None, &None, &None);

    let r2 = Address::generate(&env);
    client.batch_payout(
        &vec![&env, r2],
        &vec![&env, 2_000_i128],
    );

    client.set_paused(&Some(false), &None, &None, &None, &None);

    let new_events = events_since(&env, before, &client.address);
    let topics = topics_from(&env, &new_events);

    assert_eq!(topics.len(), 6, "expected 6 events (2 payouts + 4 pause), got {}", topics.len());
    assert_eq!(topics[0], Symbol::new(&env, "Payout"), "1st: Payout");
    assert_eq!(topics[1], Symbol::new(&env, "PauseSt"), "2nd: PauseSt (v1)");
    assert_eq!(topics[2], Symbol::new(&env, "PauseStV2"), "3rd: PauseStV2 (v2)");
    assert_eq!(topics[3], Symbol::new(&env, "BatchPay"), "4th: BatchPay");
    assert_eq!(topics[4], Symbol::new(&env, "PauseSt"), "5th: PauseSt (v1)");
    assert_eq!(topics[5], Symbol::new(&env, "PauseStV2"), "6th: PauseStV2 (v2)");
}

#[test]
fn test_batch_single_interleaved_with_multi_mode_pause() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 30_000);

    let before = env.events().all().len();

    let r1 = Address::generate(&env);
    client.batch_payout(
        &vec![&env, r1],
        &vec![&env, 1_000_i128],
    );

    client.set_paused(&Some(true), &Some(true), &None, &None, &None);

    client.set_paused(&None, &Some(false), &None, &None, &None);

    let r2 = Address::generate(&env);
    client.single_payout(&r2, &2_000, &None);

    let new_events = events_since(&env, before, &client.address);
    let topics = topics_from(&env, &new_events);

    assert_eq!(topics.len(), 8, "expected 8 events, got {}", topics.len());
    assert_eq!(topics[0], Symbol::new(&env, "BatchPay"), "1st: BatchPay");
    assert_eq!(topics[1], Symbol::new(&env, "PauseSt"), "2nd: lock PauseSt");
    assert_eq!(topics[2], Symbol::new(&env, "PauseStV2"), "3rd: lock PauseStV2");
    assert_eq!(topics[3], Symbol::new(&env, "PauseSt"), "4th: release PauseSt");
    assert_eq!(topics[4], Symbol::new(&env, "PauseStV2"), "5th: release PauseStV2");
    assert_eq!(topics[5], Symbol::new(&env, "PauseSt"), "6th: release-unpause PauseSt");
    assert_eq!(topics[6], Symbol::new(&env, "PauseStV2"), "7th: release-unpause PauseStV2");
    assert_eq!(topics[7], Symbol::new(&env, "Payout"), "8th: Payout");
}

#[test]
fn test_event_ordering_determinism_across_sequential_calls() {
    let mut all_orders: std::vec::Vec<std::vec::Vec<Symbol>> = std::vec::Vec::new();

    for _ in 0..2 {
        let env = Env::default();
        let (client, _admin, _token) = setup(&env, 50_000);

        let before = env.events().all().len();

        let r1 = Address::generate(&env);
        client.single_payout(&r1, &500, &None);

        client.set_paused(&Some(true), &None, &None, &None, &None);

        let r2 = Address::generate(&env);
        client.batch_payout(
            &vec![&env, r2],
            &vec![&env, 1_000_i128],
        );

        client.set_paused(&Some(false), &None, &None, &None, &None);

        let r3 = Address::generate(&env);
        client.single_payout(&r3, &1_500, &None);

        let new_events = events_since(&env, before, &client.address);
        let topics = topics_from(&env, &new_events);

        all_orders.push(topics);
    }

    assert_eq!(
        all_orders[0], all_orders[1],
        "event topic order must be deterministic across runs"
    );

    assert!(all_orders[0].len() > 1, "must have multiple events to compare");
}

#[test]
fn test_all_events_have_v2_version_tag() {
    let env = Env::default();
    let (client, _admin, _token) = setup(&env, 20_000);

    let before = env.events().all().len();

    let r1 = Address::generate(&env);
    client.single_payout(&r1, &1_000, &None);

    let r2 = Address::generate(&env);
    client.batch_payout(
        &vec![&env, r2],
        &vec![&env, 2_000_i128],
    );

    client.set_paused(&Some(true), &None, &None, &None, &None);

    let new_events = events_since(&env, before, &client.address);

    let mut payout_count: u32 = 0;
    let mut batch_count: u32 = 0;

    for e in new_events.iter() {
        let sym = topic_at(&env, &e, 0);
        if sym == Symbol::new(&env, "Payout") {
            payout_count += 1;
            let data: PayoutEvent = e.2.try_into_val(&env).unwrap();
            assert_eq!(data.version, 2, "PayoutEvent must have version 2");
        } else if sym == Symbol::new(&env, "BatchPay") {
            batch_count += 1;
            let data: BatchPayoutEvent = e.2.try_into_val(&env).unwrap();
            assert_eq!(data.version, 2, "BatchPayoutEvent must have version 2");
        }
    }

    assert_eq!(payout_count, 1, "expected 1 Payout event");
    assert_eq!(batch_count, 1, "expected 1 BatchPay event");
}
