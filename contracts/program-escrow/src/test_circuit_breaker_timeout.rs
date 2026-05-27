/// Circuit Breaker Timeout Tests — Issue #1254
///
/// Tests the automatic timeout handling for circuit breaker state transitions:
/// - Open → HalfOpen after recovery_window elapsed
/// - HalfOpen → Closed after successful probe operation
/// - Proper timestamp tracking and event emission

#[cfg(test)]
mod test {
    use crate::error_recovery::{self, CircuitBreakerConfig, CircuitBreakerKey, CircuitState};
    use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Events, Ledger},
        token, vec, Address, Env, String, Symbol, TryFromVal,
    };

    struct TimeoutTestSetup<'a> {
        env: Env,
        client: ProgramEscrowContractClient<'a>,
        admin: Address,
        token_client: token::Client<'a>,
    }

    fn setup_with_timeout_config(
        initial_balance: i128,
        recovery_window: u64,
    ) -> TimeoutTestSetup<'static> {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = sac.address();
        let token_client = token::Client::new(&env, &token_id);
        let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

        client.initialize_contract(&admin);
        client.set_circuit_admin(&admin, &None);

        // Configure circuit breaker with custom recovery window
        client.configure_circuit_breaker(&admin, &3u32, &1u32, &10u32, &recovery_window);

        let program_id = String::from_str(&env, "timeout-test");
        client.init_program(&program_id, &admin, &token_id, &admin, &None, &None);
        client.publish_program(&program_id);

        if initial_balance > 0 {
            token_admin_client.mint(&contract_id, &initial_balance);
            client.lock_program_funds(&initial_balance);
        }

        TimeoutTestSetup {
            env,
            client,
            admin,
            token_client,
        }
    }

    fn open_circuit_at_time(setup: &TimeoutTestSetup, timestamp: u64) {
        setup.env.ledger().set_timestamp(timestamp);
        setup.env.as_contract(&setup.client.address, || {
            error_recovery::open_circuit(&setup.env);
        });
    }

    fn get_circuit_state(setup: &TimeoutTestSetup) -> CircuitState {
        setup.env.as_contract(&setup.client.address, || {
            error_recovery::get_state(&setup.env)
        })
    }

    fn get_opened_at(setup: &TimeoutTestSetup) -> u64 {
        setup.env.as_contract(&setup.client.address, || {
            setup
                .env
                .storage()
                .persistent()
                .get(&CircuitBreakerKey::OpenedAt)
                .unwrap_or(0)
        })
    }

    /// Circuit automatically transitions from Open to HalfOpen after recovery_window.
    #[test]
    fn test_automatic_open_to_halfopen_after_recovery_window() {
        let setup = setup_with_timeout_config(1000, 300); // 5 minute recovery window

        // Open circuit at time 1000
        open_circuit_at_time(&setup, 1000);
        assert_eq!(get_circuit_state(&setup), CircuitState::Open);
        assert_eq!(get_opened_at(&setup), 1000);

        // Before recovery window - should still be Open
        setup.env.ledger().set_timestamp(1200); // 200 seconds later
        let winner = Address::generate(&setup.env);
        let result = setup.client.try_single_payout(&winner, &100i128, &None);
        assert!(result.is_err(), "Payout should fail - circuit still Open");
        assert_eq!(get_circuit_state(&setup), CircuitState::Open);

        // After recovery window - should transition to HalfOpen on next operation
        setup.env.ledger().set_timestamp(1350); // 350 seconds later (past 300s window)
        let result = setup.client.try_single_payout(&winner, &100i128, &None);
        assert!(
            result.is_ok(),
            "Payout should succeed - circuit now HalfOpen"
        );
        assert_eq!(get_circuit_state(&setup), CircuitState::HalfOpen);
    }

    /// Successful operation in HalfOpen automatically closes the circuit.
    #[test]
    fn test_successful_probe_closes_circuit() {
        let setup = setup_with_timeout_config(1000, 300);

        // Open circuit and wait for timeout transition
        open_circuit_at_time(&setup, 1000);
        setup.env.ledger().set_timestamp(1350); // Past recovery window

        let winner = Address::generate(&setup.env);

        // First operation transitions to HalfOpen
        let result = setup.client.try_single_payout(&winner, &100i128, &None);
        assert!(
            result.is_ok(),
            "First payout should succeed and transition to HalfOpen"
        );
        assert_eq!(get_circuit_state(&setup), CircuitState::HalfOpen);

        // Second successful operation should close the circuit
        let result = setup.client.try_single_payout(&winner, &100i128, &None);
        assert!(result.is_ok(), "Second payout should succeed");
        assert_eq!(get_circuit_state(&setup), CircuitState::Closed);
    }

    /// Zero recovery window should immediately allow HalfOpen transition.
    #[test]
    fn test_zero_recovery_window() {
        let setup = setup_with_timeout_config(1000, 0); // Zero recovery window

        open_circuit_at_time(&setup, 1000);
        assert_eq!(get_circuit_state(&setup), CircuitState::Open);

        // Any operation should immediately transition to HalfOpen
        let winner = Address::generate(&setup.env);
        let result = setup.client.try_single_payout(&winner, &100i128, &None);
        assert!(
            result.is_ok(),
            "Should succeed immediately with zero recovery window"
        );
        assert_eq!(get_circuit_state(&setup), CircuitState::HalfOpen);
    }

    /// Status endpoint reflects timeout configuration correctly.
    #[test]
    fn test_status_reflects_recovery_window() {
        let setup = setup_with_timeout_config(1000, 500);

        let status = setup.client.get_circuit_breaker_status();
        assert_eq!(
            status.recovery_window, 500,
            "Status should reflect configured recovery window"
        );
        assert_eq!(status.state, CircuitState::Closed);

        open_circuit_at_time(&setup, 1000);
        let status = setup.client.get_circuit_breaker_status();
        assert_eq!(status.state, CircuitState::Open);
        assert_eq!(status.opened_at, 1000);
        assert_eq!(status.recovery_window, 500);
    }
}
