#[cfg(test)]
mod test {
    use crate::error_recovery::{self, CircuitBreakerKey, CircuitState};
    use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger},
        symbol_short, vec, Address, Env, String,
    };

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

            // TAMPER: Force state to Open but leave opened_at as 0
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::State, &CircuitState::Open);
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::OpenedAt, &0u64);

            // Verify that verification detects the inconsistency
            assert!(
                !error_recovery::verify_circuit_invariants(&env),
                "Should fail when Open state has no timestamp"
            );
            // Verify that verification detects the inconsistency
            assert!(
                !error_recovery::verify_circuit_invariants(&env),
                "Should fail when Open state has no timestamp"
            );
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

            // TAMPER: Force failure_count to 10 (threshold is 3) but keep state Closed
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::FailureCount, &10u32);
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::State, &CircuitState::Closed);

            // Verify that verification detects the inconsistency
            assert!(
                !error_recovery::verify_circuit_invariants(&env),
                "Should fail when Closed state exceeds failure threshold"
            );
            // Verify that verification detects the inconsistency
            assert!(
                !error_recovery::verify_circuit_invariants(&env),
                "Should fail when Closed state exceeds failure threshold"
            );
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

            // TAMPER: Force success_count to 5 (threshold is 1) but keep state HalfOpen
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::State, &CircuitState::HalfOpen);
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::SuccessCount, &5u32);

            // Verify that verification detects the inconsistency
            assert!(
                !error_recovery::verify_circuit_invariants(&env),
                "Should fail when HalfOpen state exceeds success threshold"
            );
            // Verify that verification detects the inconsistency
            assert!(
                !error_recovery::verify_circuit_invariants(&env),
                "Should fail when HalfOpen state exceeds success threshold"
            );
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

    #[test]
    fn test_audit_circuit_admin_change() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        let new_admin = Address::generate(&env);

        client.set_circuit_admin(&new_admin, &Some(admin.clone()));

        let circuit_events = env.events().all();
        let ev = circuit_events.get(circuit_events.len() - 1).unwrap();
        assert_eq!(ev.0, client.address);
        assert_eq!(ev.1, (symbol_short!("circuit"), symbol_short!("cb_adm")));
    }

    #[test]
    fn test_audit_circuit_manual_reset() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        env.ledger().set_timestamp(100);

        // First open it
        env.as_contract(&client.address, || {
            error_recovery::open_circuit(&env);
        });

        // Now reset via client
        client.reset_circuit_breaker(&admin);

        let circuit_events = env.events().all();
        // Check for cb_reset event
        let mut found = false;
        for ev in circuit_events.iter() {
            if ev.1 == (symbol_short!("circuit"), symbol_short!("cb_reset")) {
                found = true;
                break;
            }
        }
        assert!(found, "cb_reset event not found");
    }

    #[test]
    fn test_audit_circuit_config_update() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);

        client.configure_circuit_breaker(&admin, &5u32, &2u32, &10u32);

        let circuit_events = env.events().all();
        let ev = circuit_events.get(circuit_events.len() - 1).unwrap();
        assert_eq!(ev.1, (symbol_short!("circuit"), symbol_short!("cb_cfg")));
    }

    #[test]
    fn test_audit_rate_limit_config_update() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);

        client.update_rate_limit_config(&3600u64, &50u32, &120u64);

        let events = env.events().all();
        let ev = events.get(events.len() - 1).unwrap();
        assert_eq!(ev.1, (symbol_short!("rate_lim"), symbol_short!("update")));
    }

    #[test]
    fn test_payout_respects_circuit_breaker() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup_test(&env);

        let user = Address::generate(&env);
        let token_addr = Address::generate(&env); // Mock token
        
        // Initialize program
        client.init_program(
            &String::from_str(&env, "prog1"),
            &admin,
            &token_addr,
            &admin,
            &Some(1000i128),
            &None,
        );

        // Open circuit
        env.as_contract(&client.address, || {
            error_recovery::open_circuit(&env);
        });

        // Try single payout - should panic
        let result = client.try_single_payout(&user, &100i128);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_payout_respects_circuit_breaker() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup_test(&env);

        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let token_addr = Address::generate(&env);
        
        client.init_program(
            &String::from_str(&env, "prog1"),
            &admin,
            &token_addr,
            &admin,
            &Some(1000i128),
            &None,
        );

        // Open circuit
        env.as_contract(&client.address, || {
            error_recovery::open_circuit(&env);
        });

        // Try batch payout - should panic
        let result = client.try_batch_payout(&vec![&env, user1, user2], &vec![&env, 50i128, 50i128]);
        assert!(result.is_err());
    }
}
