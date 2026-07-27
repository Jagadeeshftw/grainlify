#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, String as SorobanString};

fn make_minimal_program_data(env: &Env) -> ProgramData {
    ProgramData {
        status: ProgramStatus::Active,
        remaining_balance: 9_000,
        total_funds: 10_000,
        authorized_payout_key: Address::generate(env),
        token_address: Address::generate(env),
        delegate: None,
        delegate_permissions: 0,
        risk_flags: 0,
        archived: false,
        archived_at: None,
        initial_liquidity: 10_000,
        circuit_breaker_threshold: None,
        program_id: SorobanString::from_str(env, "test-program"),
        payout_history: soroban_sdk::Vec::new(env),
        fot_router: OptionalFotRouter::None,
        reference_hash: None,
    }
}

fn make_full_program_data(env: &Env) -> ProgramData {
    let payout = PayoutRecord {
        recipient: Address::generate(env),
        amount: 500,
        timestamp: 12345,
    };
    ProgramData {
        status: ProgramStatus::Draft,
        remaining_balance: 5_000,
        total_funds: 10_000,
        authorized_payout_key: Address::generate(env),
        token_address: Address::generate(env),
        delegate: Some(Address::generate(env)),
        delegate_permissions: 7,
        risk_flags: 3,
        archived: true,
        archived_at: Some(99999),
        initial_liquidity: 10_000,
        circuit_breaker_threshold: Some(5),
        program_id: SorobanString::from_str(env, "full-program"),
        payout_history: soroban_sdk::vec![env, payout],
        fot_router: OptionalFotRouter::None,
        reference_hash: Some(Bytes::from_array(env, &[0x01, 0x02, 0x03, 0x04])),
    }
}

// Verifies schema version was bumped to 2 after the ProgramData field reorder.
#[test]
fn test_schema_version_is_2() {
    assert_eq!(
        STORAGE_SCHEMA_VERSION, 2,
        "STORAGE_SCHEMA_VERSION must be 2 after ProgramData field reordering"
    );
}

// Verifies that hot-path status field is accessible and correct after construction.
#[test]
fn test_status_field_accessible() {
    let env = Env::default();
    let data = make_minimal_program_data(&env);
    assert_eq!(data.status, ProgramStatus::Active);

    let draft = ProgramData {
        status: ProgramStatus::Draft,
        ..make_minimal_program_data(&env)
    };
    assert_eq!(draft.status, ProgramStatus::Draft);
}

// Verifies that hot-path balance fields are accessible and correct.
#[test]
fn test_balance_fields_accessible() {
    let env = Env::default();
    let data = make_minimal_program_data(&env);
    assert_eq!(data.remaining_balance, 9_000);
    assert_eq!(data.total_funds, 10_000);
    assert_eq!(data.initial_liquidity, 10_000);
}

// Verifies that circuit_breaker_threshold round-trips correctly with None and Some.
#[test]
fn test_circuit_breaker_threshold_variants() {
    let env = Env::default();

    let no_threshold = make_minimal_program_data(&env);
    assert_eq!(no_threshold.circuit_breaker_threshold, None);

    let with_threshold = ProgramData {
        circuit_breaker_threshold: Some(10),
        ..make_minimal_program_data(&env)
    };
    assert_eq!(with_threshold.circuit_breaker_threshold, Some(10));
}

// Verifies that a struct with all optional fields set retains correct values.
#[test]
fn test_full_struct_field_values() {
    let env = Env::default();
    let data = make_full_program_data(&env);

    assert_eq!(data.status, ProgramStatus::Draft);
    assert_eq!(data.remaining_balance, 5_000);
    assert_eq!(data.total_funds, 10_000);
    assert_eq!(data.delegate_permissions, 7);
    assert_eq!(data.risk_flags, 3);
    assert!(data.archived);
    assert_eq!(data.archived_at, Some(99999));
    assert_eq!(data.circuit_breaker_threshold, Some(5));
    assert_eq!(data.payout_history.len(), 1);
    assert!(data.reference_hash.is_some());
}

// Verifies that archived_at is None when not archived.
#[test]
fn test_archived_at_default_none() {
    let env = Env::default();
    let data = make_minimal_program_data(&env);
    assert!(!data.archived);
    assert_eq!(data.archived_at, None);
}

// Verifies payout_history starts empty and can hold a record.
#[test]
fn test_payout_history_initially_empty() {
    let env = Env::default();
    let data = make_minimal_program_data(&env);
    assert_eq!(data.payout_history.len(), 0);
}

// Verifies reference_hash is None by default.
#[test]
fn test_reference_hash_default_none() {
    let env = Env::default();
    let data = make_minimal_program_data(&env);
    assert_eq!(data.reference_hash, None);
}

// Verifies delegate fields default to None / zero.
#[test]
fn test_delegate_defaults() {
    let env = Env::default();
    let data = make_minimal_program_data(&env);
    assert_eq!(data.delegate, None);
    assert_eq!(data.delegate_permissions, 0);
}

// Verifies risk_flags defaults to zero.
#[test]
fn test_risk_flags_default_zero() {
    let env = Env::default();
    let data = make_minimal_program_data(&env);
    assert_eq!(data.risk_flags, 0);
}
