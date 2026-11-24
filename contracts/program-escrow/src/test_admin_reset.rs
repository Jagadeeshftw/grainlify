//! Circuit breaker admin reset (`reset_circuit_breaker`).
//!
//! Wired up for #2089. The suite was previously an orphaned file: it existed in
//! `src/` but no `mod` declared it, so its assertions never ran.
//!
//! Updated for the current entrypoint: `reset_circuit_breaker(env, caller)` takes
//! an authorised `Address` (the registered circuit admin) rather than a program id,
//! and a reset from `Open` moves the circuit to `HalfOpen` (the next successful
//! operation closes it). Both are asserted below.

#![cfg(test)]

use crate::error_recovery::{CircuitBreakerKey, CircuitState};
use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

/// Initialise a contract, register `admin` as the circuit admin and force the
/// circuit into `Open` with a non-zero failure count.
fn setup_open_circuit<'a>(env: &Env) -> (ProgramEscrowContractClient<'a>, Address) {
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);

    client.initialize_contract(&admin);
    client.set_circuit_admin(&admin, &None);

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::State, &CircuitState::Open);
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::FailureCount, &3u32);
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::OpenedAt, &1_000u64);
    });

    (client, admin)
}

#[test]
fn test_admin_reset_reopens_circuit_as_half_open() {
    let env = Env::default();
    let (client, admin) = setup_open_circuit(&env);

    client.reset_circuit_breaker(&admin);

    let status = client.get_circuit_breaker_status();
    assert_eq!(
        status.state,
        CircuitState::HalfOpen,
        "an admin reset from Open must move the circuit to HalfOpen"
    );
    // `half_open_circuit` deliberately keeps the failure counter so the probe
    // window still reflects how badly the circuit degraded; it is cleared when the
    // circuit closes.
    assert_eq!(
        status.failure_count, 3,
        "failure count is preserved while the circuit waits for a probe"
    );
    assert_eq!(
        status.success_count, 0,
        "reset must clear the success counter"
    );
}

#[test]
#[should_panic]
fn test_non_admin_cannot_reset_panics() {
    let env = Env::default();
    let (client, _admin) = setup_open_circuit(&env);
    let attacker = Address::generate(&env);

    // `caller` is authenticated by `require_auth` (mocked here), but the caller
    // identity check against the registered circuit admin must still refuse.
    client.reset_circuit_breaker(&attacker);
}
