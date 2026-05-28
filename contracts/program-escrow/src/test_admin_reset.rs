#![cfg(test)]

use soroban_sdk::{Env, Address, String};

use crate::error_recovery::{CircuitBreakerKey, CircuitState};
use crate::ProgramEscrowContract;
use crate::DataKey;

#[test]
fn test_admin_reset_closes_open_circuit() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProgramEscrowContract);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "TestProg");

    // Set contract admin and open circuit
    env.as_contract(&contract_id, || {
        env.storage().instance().set(&DataKey::Admin, &admin);
    });

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::State, &CircuitState::Open);
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::FailureCount, &3u32);
    });

    // Call reset_circuit_breaker as admin
    env.as_contract(&contract_id, || {
        let res = ProgramEscrowContract::reset_circuit_breaker(env.clone(), program_id.clone());
        assert!(res.is_ok());

        // Verify circuit closed and counters cleared
        let state: CircuitState = env
            .storage()
            .persistent()
            .get(&CircuitBreakerKey::State)
            .unwrap();
        assert_eq!(state, CircuitState::Closed);
        let failures: u32 = env
            .storage()
            .persistent()
            .get(&CircuitBreakerKey::FailureCount)
            .unwrap_or(0);
        assert_eq!(failures, 0);
    });
}

#[test]
#[should_panic]
fn test_non_admin_cannot_reset_panics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);

    let admin = Address::generate(&env);
    let program_id = String::from_str(&env, "TestProg");

    // Set contract admin but do not mock auth — should panic on require_auth
    env.as_contract(&contract_id, || {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::State, &CircuitState::Open);
    });

    env.as_contract(&contract_id, || {
        // This should panic because admin.require_auth() will fail
        let _ = ProgramEscrowContract::reset_circuit_breaker(env.clone(), program_id.clone());
    });
}
