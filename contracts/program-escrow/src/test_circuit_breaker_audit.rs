#[cfg(test)]
mod test {
    use crate::error_recovery::{self, CircuitBreakerKey, CircuitState};
    use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
    use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String};

    fn setup_test(env: &Env) -> (ProgramEscrowContractClient, Address) {
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize_contract(&admin);
        client.set_circuit_admin(&admin, &None);
        (client, admin)
    }

    #[test]
    fn test_circuit_healthy_state_passes_verification() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        env.as_contract(&client.address, || {

        // Initially Closed and healthy
        assert!(error_recovery::verify_circuit_invariants(&env));
        });
    }

    #[test]
    fn test_circuit_tamper_open_without_timestamp() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        env.as_contract(&client.address, || {

        // TAMPER: Force state to Open but leave opened_at as 0
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::State, &CircuitState::Open);
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::OpenedAt, &0u64);

        // Verify that verification detects the inconsistency
        assert!(!error_recovery::verify_circuit_invariants(&env), "Should fail when Open state has no timestamp");
        });
    }

    #[test]
    fn test_circuit_tamper_closed_with_threshold_exceeded() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        env.as_contract(&client.address, || {

        // TAMPER: Force failure_count to 10 (threshold is 3) but keep state Closed
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::FailureCount, &10u32);
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::State, &CircuitState::Closed);

        // Verify that verification detects the inconsistency
        assert!(!error_recovery::verify_circuit_invariants(&env), "Should fail when Closed state exceeds failure threshold");
        });
    }

    #[test]
    fn test_circuit_tamper_half_open_with_success_exceeded() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        env.as_contract(&client.address, || {

        // TAMPER: Force success_count to 5 (threshold is 1) but keep state HalfOpen
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::State, &CircuitState::HalfOpen);
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::SuccessCount, &5u32);

        // Verify that verification detects the inconsistency
        assert!(!error_recovery::verify_circuit_invariants(&env), "Should fail when HalfOpen state exceeds success threshold");
        });
    }

    #[test]
    fn test_circuit_blocking_when_open() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        
        // Set a non-zero timestamp so OpenedAt is valid (invariant check fails if 0)
        env.ledger().set_timestamp(100);

        env.as_contract(&client.address, || {
            // Open the circuit properly
            error_recovery::open_circuit(&env);
            assert!(error_recovery::verify_circuit_invariants(&env));

            // Verify check_and_allow rejects
            assert!(error_recovery::check_and_allow(&env).is_err());
        });
    }
}
