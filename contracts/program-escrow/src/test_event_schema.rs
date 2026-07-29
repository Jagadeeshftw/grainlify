//! # Test Event Schema & Cross-Contract Event Correlation
//!
//! Comprehensive tests verifying event schema compatibility, deterministic correlation ID generation,
//! and cross-contract event correlation assertions across `grainlify-core` and `program-escrow`.

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, String, Symbol, TryFromVal, Val, Vec,
};

use grainlify_core::{generate_correlation_id, CorrelationId, UpgradeEvent, EVENT_SCHEMA_VERSION};
use crate::{BatchPayoutEvent, PayoutEvent, ReleaseScheduledEvent, ScheduleReleasedEvent, EVENT_VERSION_V2};

#[test]
fn test_deterministic_correlation_id_consistency() {
    let env = Env::default();
    let initiator = Address::generate(&env);
    let nonce = 12345u64;
    let domain = symbol_short!("payout");

    let corr_id_1 = generate_correlation_id(&env, &initiator, nonce, Some(&domain));
    let corr_id_2 = generate_correlation_id(&env, &initiator, nonce, Some(&domain));

    assert_eq!(corr_id_1, corr_id_2, "Deterministic correlation IDs for identical parameters must match");
}

#[test]
fn test_cross_contract_event_correlation_sequence() {
    let env = Env::default();
    let initiator = Address::generate(&env);
    let recipient = Address::generate(&env);
    let nonce = 99u64;
    let domain = symbol_short!("upg_pay");

    // 1. Generate a shared correlation ID for the combined multi-contract operation
    let shared_correlation_id = generate_correlation_id(&env, &initiator, nonce, Some(&domain));

    // 2. Emit an event from grainlify-core (e.g., UpgradeEvent with correlation_id)
    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[7u8; 32]);
    let core_event = UpgradeEvent {
        new_wasm_hash: wasm_hash,
        previous_version: 1,
        timestamp: 1000,
        event_version: EVENT_SCHEMA_VERSION,
        correlation_id: Some(shared_correlation_id.clone()),
    };

    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("wasm")),
        core_event.clone(),
    );

    // 3. Emit an event from program-escrow (e.g., PayoutEvent with identical correlation_id)
    let escrow_event = PayoutEvent {
        version: EVENT_VERSION_V2,
        program_id: String::from_str(&env, "HACK_2026"),
        recipient: recipient.clone(),
        amount: 50_000,
        remaining_balance: 150_000,
        correlation_id: Some(shared_correlation_id.clone()),
    };

    env.events().publish(
        (symbol_short!("Payout"),),
        escrow_event.clone(),
    );

    // 4. Inspect published events and verify the shared correlation ID is present and consistent on both sides
    let published_events = env.events().all();
    assert!(published_events.len() >= 2, "Expected at least 2 emitted events");

    let first_raw_event = published_events.get(0).unwrap();
    let second_raw_event = published_events.get(1).unwrap();

    let decoded_core: UpgradeEvent = UpgradeEvent::try_from_val(&env, &first_raw_event.2)
        .expect("Failed to deserialize UpgradeEvent");
    let decoded_escrow: PayoutEvent = PayoutEvent::try_from_val(&env, &second_raw_event.2)
        .expect("Failed to deserialize PayoutEvent");

    assert_eq!(
        decoded_core.correlation_id,
        Some(shared_correlation_id.clone()),
        "Core upgrade event must contain the shared correlation ID"
    );
    assert_eq!(
        decoded_escrow.correlation_id,
        Some(shared_correlation_id.clone()),
        "Escrow payout event must contain the identical shared correlation ID"
    );
    assert_eq!(
        decoded_core.correlation_id,
        decoded_escrow.correlation_id,
        "Correlation IDs across multi-contract event sequence must match exactly"
    );
}

#[test]
fn test_event_schema_backwards_compatibility_without_correlation_id() {
    let env = Env::default();
    let recipient = Address::generate(&env);

    // Legacy / un-correlated event (correlation_id = None)
    let payout = PayoutEvent {
        version: EVENT_VERSION_V2,
        program_id: String::from_str(&env, "TEST_PROG"),
        recipient: recipient.clone(),
        amount: 1_000,
        remaining_balance: 9_000,
        correlation_id: None,
    };

    env.events().publish((symbol_short!("Payout"),), payout.clone());

    let published = env.events().all();
    let raw = published.get(0).unwrap();
    let decoded: PayoutEvent = PayoutEvent::try_from_val(&env, &raw.2)
        .expect("Event missing correlation_id must deserialize cleanly with None");

    assert_eq!(decoded.correlation_id, None);
    assert_eq!(decoded.amount, 1_000);
}

#[test]
fn test_batch_payout_and_schedule_events_with_correlation_id() {
    let env = Env::default();
    let initiator = Address::generate(&env);
    let recipient = Address::generate(&env);

    let corr_id = generate_correlation_id(&env, &initiator, 50, Some(&symbol_short!("batch")));

    let batch_ev = BatchPayoutEvent {
        version: EVENT_VERSION_V2,
        program_id: String::from_str(&env, "PROG_BATCH"),
        recipient_count: 5,
        total_amount: 100_000,
        remaining_balance: 400_000,
        idempotency_key: Some(String::from_str(&env, "KEY_123")),
        correlation_id: Some(corr_id.clone()),
    };

    let release_ev = ReleaseScheduledEvent {
        version: EVENT_VERSION_V2,
        program_id: String::from_str(&env, "PROG_BATCH"),
        schedule_id: 10,
        recipient: recipient.clone(),
        amount: 20_000,
        release_timestamp: 2000,
        correlation_id: Some(corr_id.clone()),
    };

    let schedule_released_ev = ScheduleReleasedEvent {
        version: EVENT_VERSION_V2,
        program_id: String::from_str(&env, "PROG_BATCH"),
        schedule_id: 10,
        recipient: recipient.clone(),
        amount: 20_000,
        released_at: 2005,
        released_by: initiator.clone(),
        correlation_id: Some(corr_id.clone()),
    };

    env.events().publish((symbol_short!("BatchPay"),), batch_ev.clone());
    env.events().publish((symbol_short!("RelSch"),), release_ev.clone());
    env.events().publish((symbol_short!("SchRel"),), schedule_released_ev.clone());

    let all_events = env.events().all();
    assert_eq!(all_events.len(), 3);

    let decoded_batch: BatchPayoutEvent = BatchPayoutEvent::try_from_val(&env, &all_events.get(0).unwrap().2).unwrap();
    let decoded_rel: ReleaseScheduledEvent = ReleaseScheduledEvent::try_from_val(&env, &all_events.get(1).unwrap().2).unwrap();
    let decoded_sch_rel: ScheduleReleasedEvent = ScheduleReleasedEvent::try_from_val(&env, &all_events.get(2).unwrap().2).unwrap();

    assert_eq!(decoded_batch.correlation_id, Some(corr_id.clone()));
    assert_eq!(decoded_rel.correlation_id, Some(corr_id.clone()));
    assert_eq!(decoded_sch_rel.correlation_id, Some(corr_id.clone()));
}
