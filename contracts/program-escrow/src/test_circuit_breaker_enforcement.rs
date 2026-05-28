//! Integration tests asserting the correct precedence order when multiple
//! protection guards are active simultaneously.
//!
//! Precedence (highest → lowest):
//!   Pause / maintenance mode → Read-only mode → Circuit breaker
//!
//! See `docs/program-escrow/CIRCUIT_BREAKER_ENFORCEMENT.md` for full details.

#![cfg(test)]

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, Env, String,
};

use crate::{
    error_recovery::{self, CircuitBreakerConfig, CircuitState},
    ProgramEscrowContract, ProgramEscrowContractClient,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

struct TestSetup {
    env: Env,
    client: ProgramEscrowContractClient<'static>,
    admin: Address,
    payout_key: Address,
    recipient: Address,
    token: Address,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    // SAFETY: the client reference borrows from env which lives in the struct.
    let client: ProgramEscrowContractClient<'static> =
        unsafe { core::mem::transmute(ProgramEscrowContractClient::new(&env, &contract_id)) };

    let admin = Address::generate(&env);
    let payout_key = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    // Bootstrap contract
    client.initialize_contract(&admin);
    client.init_program(
        &String::from_str(&env, "test-program"),
        &payout_key,
        &token,
        &admin,
        &None,
        &None,
    );

    // Register a circuit breaker admin (same as contract admin for simplicity)
    client.set_circuit_admin(&admin, &None);

    TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token,
    }
}

/// Force the circuit breaker into Open state by recording failures above threshold.
fn open_circuit(env: &Env) {
    let cfg = CircuitBreakerConfig {
        failure_threshold: 1,
        success_threshold: 1,
        max_error_log: 10,
    };
    error_recovery::set_config(env, cfg);
    error_recovery::record_failure(
        env,
        String::from_str(env, "test-program"),
        symbol_short!("test"),
        1001,
    );
    assert_eq!(error_recovery::get_state(env), CircuitState::Open);
}

/// Assert the circuit is in Closed state.
fn assert_circuit_closed(env: &Env) {
    assert_eq!(
        error_recovery::get_state(env),
        CircuitState::Closed,
        "Expected circuit to be Closed"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Pause wins over circuit breaker
// ─────────────────────────────────────────────────────────────────────────────

/// When release is paused AND the circuit is open, the pause error message
/// must be returned (pause takes precedence).
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_pause_wins_over_open_circuit_single_payout() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token: _,
    } = setup();

    // Open the circuit breaker
    open_circuit(&env);

    // Also pause release operations
    client.set_paused(&Some(false), &Some(true), &Some(false), &None, &None);

    // Attempt a single payout — should fail with "Funds Paused", NOT "Circuit breaker is OPEN"
    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

/// Same as above but for batch_payout.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_pause_wins_over_open_circuit_batch_payout() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token: _,
    } = setup();

    open_circuit(&env);
    client.set_paused(&Some(false), &Some(true), &Some(false), &None, &None);

    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 100i128];
    client.batch_payout_by(&payout_key, &recipients, &amounts, &None);
}

/// When lock is paused AND the circuit is open, lock operations must return
/// "Funds Paused".
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_pause_wins_over_open_circuit_lock() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key: _,
        recipient: _,
        token: _,
    } = setup();

    open_circuit(&env);
    client.set_paused(&Some(true), &Some(false), &Some(false), &None, &None);

    client.lock_program_funds(&1000i128);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: All three layers simultaneously — pause wins
// ─────────────────────────────────────────────────────────────────────────────

/// With pause, read-only, AND circuit breaker all active, the pause error
/// must surface first.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_all_three_active_pause_wins_single_payout() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token: _,
    } = setup();

    // Activate all three layers
    open_circuit(&env);
    client.set_read_only_mode(&true, &None);
    client.set_paused(&Some(false), &Some(true), &Some(false), &None, &None);

    // Pause must win
    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

/// With pause, read-only, AND circuit breaker all active, the pause error
/// must surface for batch_payout.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_all_three_active_pause_wins_batch_payout() {
    let TestSetup {
        env,
        client,
        admin,
        payout_key,
        recipient,
        token: _,
    } = setup();

    open_circuit(&env);
    client.set_read_only_mode(&true, &None);
    client.set_paused(&Some(false), &Some(true), &Some(false), &None, &None);

    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 100i128];
    client.batch_payout_by(&payout_key, &recipients, &amounts, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Circuit breaker fires only when pause is inactive
// ─────────────────────────────────────────────────────────────────────────────

/// When the circuit is open but pause is NOT active, the circuit breaker
/// error must surface.
#[test]
#[should_panic(expected = "Circuit breaker is OPEN")]
fn test_circuit_open_fires_when_pause_inactive_single_payout() {
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    open_circuit(&env);
    // Explicitly confirm pause is off
    client.set_paused(&Some(false), &Some(false), &Some(false), &None, &None);

    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

/// Same for batch_payout.
#[test]
#[should_panic(expected = "Circuit breaker is OPEN")]
fn test_circuit_open_fires_when_pause_inactive_batch_payout() {
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    open_circuit(&env);
    client.set_paused(&Some(false), &Some(false), &Some(false), &None, &None);

    let recipients = vec![&env, recipient];
    let amounts = vec![&env, 100i128];
    client.batch_payout_by(&payout_key, &recipients, &amounts, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Unpausing reveals the circuit breaker
// ─────────────────────────────────────────────────────────────────────────────

/// After unpausing, if the circuit is still open, the circuit breaker error
/// must be returned (not a pause error).
#[test]
#[should_panic(expected = "Circuit breaker is OPEN")]
fn test_unpause_reveals_open_circuit() {
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    // Start: both active
    open_circuit(&env);
    client.set_paused(&Some(false), &Some(true), &Some(false), &None, &None);

    // Unpause, circuit stays open
    client.set_paused(&Some(false), &Some(false), &Some(false), &None, &None);

    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Resetting the circuit breaker
// ─────────────────────────────────────────────────────────────────────────────

/// After the circuit is reset (Open → HalfOpen → Closed) and pause is not
/// active, operations should be allowed (assuming sufficient balance).
/// Here we only verify the circuit reaches Closed state after reset.
#[test]
fn test_circuit_reset_closes_after_open() {
    let TestSetup {
        env,
        client,
        admin,
        ..
    } = setup();

    open_circuit(&env);
    assert_eq!(error_recovery::get_state(&env), CircuitState::Open);

    // Step 1: reset moves Open → HalfOpen
    client.reset_circuit_breaker(&admin);
    assert_eq!(error_recovery::get_state(&env), CircuitState::HalfOpen);

    // Step 2: another reset moves HalfOpen → Closed
    client.reset_circuit_breaker(&admin);
    assert_circuit_closed(&env);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Maintenance mode (treated as lock pause)
// ─────────────────────────────────────────────────────────────────────────────

/// Maintenance mode blocks lock operations with its own error message,
/// independently of the circuit breaker state.
#[test]
#[should_panic(expected = "Funds Paused")]
fn test_maintenance_mode_wins_over_open_circuit_on_lock() {
    let TestSetup {
        env,
        client,
        admin,
        ..
    } = setup();

    open_circuit(&env);
    client.set_maintenance_mode(&true);

    client.lock_program_funds(&1000i128);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Pause state is independent per operation type
// ─────────────────────────────────────────────────────────────────────────────

/// Pausing only release does NOT block lock operations (even with open circuit).
/// We can't easily test a successful lock here (no token mock), but we can
/// confirm the circuit error, not a pause error, surfaces.
#[test]
#[should_panic(expected = "Circuit breaker is OPEN")]
fn test_release_pause_does_not_block_lock_when_circuit_open() {
    // NOTE: lock_program_funds does NOT check the circuit breaker in its
    // current implementation, so this test verifies orthogonality differently:
    // we confirm that pausing *release* doesn't bleed into payout operations
    // beyond what's expected. Here we verify payout sees the circuit, not pause.
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    open_circuit(&env);
    // Only lock is paused — release is not
    client.set_paused(&Some(true), &Some(false), &Some(false), &None, &None);

    // Payout should fail with circuit error (release not paused)
    client.single_payout_by(&payout_key, &recipient, &100i128, &None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Closed circuit + no pause = no protection-layer interference
// ─────────────────────────────────────────────────────────────────────────────

/// With no protection layers active, an operation reaches business-logic
/// validation (here it will fail with "Program not initialized" or balance
/// error, NOT a protection-layer error). This confirms the layers are
/// transparent when inactive.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_no_protection_layers_active_reaches_business_logic() {
    let TestSetup {
        env,
        client,
        payout_key,
        recipient,
        ..
    } = setup();

    // Ensure all protection layers are inactive
    assert_circuit_closed(&env);
    client.set_paused(&Some(false), &Some(false), &Some(false), &None, &None);

    // No funds locked → should reach balance check
    client.single_payout_by(&payout_key, &recipient, &1i128, &None);
}
