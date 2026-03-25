// contracts/program-escrow/src/error_recovery_tests.rs

#![cfg(test)]

use soroban_sdk::testutils::Address as TestAddress;
use soroban_sdk::{contract, contractimpl, symbol_short, testutils::Ledger, Address, Env, String};

use crate::error_recovery::{
    check_and_allow, close_circuit, execute_with_retry, get_circuit_admin, get_config,
    get_error_log, get_failure_count, get_state, get_status, get_success_count, half_open_circuit,
    open_circuit, record_failure, record_success, reset_circuit_breaker, set_circuit_admin,
    set_config, CircuitBreakerConfig, CircuitState, RetryConfig, ERR_CIRCUIT_OPEN,
    ERR_TRANSFER_FAILED,
};

// ─────────────────────────────────────────────────────────
// Dummy contract to provide a valid contract context
// ─────────────────────────────────────────────────────────

#[contract]
pub struct CircuitBreakerTestContract;

#[contractimpl]
impl CircuitBreakerTestContract {}

// ─────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────

/// Create a standard test environment with a registered contract and timestamp set to 1000.
fn setup_env() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register_contract(None, CircuitBreakerTestContract);
    (env, contract_id)
}

/// Create a fresh Env, register an admin, and configure the circuit breaker.
/// Returns (env, admin_address, contract_id).
fn setup_with_admin(failure_threshold: u32) -> (Env, Address, Address) {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold,
                success_threshold: 1,
                max_error_log: 5,
            },
        );
    });

    (env, admin, contract_id)
}

/// Simulate `n` consecutive failures against the circuit breaker.
fn simulate_failures(env: &Env, contract_id: &Address, n: u32) {
    let prog = String::from_str(env, "TestProg");
    let op = symbol_short!("op");
    env.as_contract(contract_id, || {
        for _ in 0..n {
            record_failure(env, prog.clone(), op.clone(), ERR_TRANSFER_FAILED);
        }
    });
}

// ─────────────────────────────────────────────────────────
// 1. Initial state
// ─────────────────────────────────────────────────────────

#[test]
fn test_initial_state_is_closed() {
    let (env, contract_id) = setup_env();
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
        assert_eq!(get_success_count(&env), 0);
    });
}

#[test]
fn test_check_and_allow_passes_when_closed() {
    let (env, contract_id) = setup_env();
    env.as_contract(&contract_id, || {
        assert!(check_and_allow(&env).is_ok());
    });
}

// ─────────────────────────────────────────────────────────
// 2. Failures below threshold do not open circuit
// ─────────────────────────────────────────────────────────

#[test]
fn test_single_failure_does_not_open_circuit() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 1);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 1);
        assert!(check_and_allow(&env).is_ok());
    });
}

#[test]
fn test_failures_below_threshold_keep_circuit_closed() {
    let (env, _admin, contract_id) = setup_with_admin(5);
    simulate_failures(&env, &contract_id, 4);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 4);
        assert!(check_and_allow(&env).is_ok());
    });
}

// ─────────────────────────────────────────────────────────
// 3. Failures at threshold open the circuit
// ─────────────────────────────────────────────────────────

#[test]
fn test_circuit_opens_at_threshold() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(get_failure_count(&env), 3);
    });
}

#[test]
fn test_circuit_opens_exactly_at_threshold_not_before() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(
            get_state(&env),
            CircuitState::Closed,
            "Should be Closed after 2 failures"
        );
    });
    simulate_failures(&env, &contract_id, 1);
    env.as_contract(&contract_id, || {
        assert_eq!(
            get_state(&env),
            CircuitState::Open,
            "Should be Open after 3rd failure"
        );
    });
}

#[test]
fn test_opened_at_timestamp_recorded() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    env.ledger().set_timestamp(5000);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        let status = get_status(&env);
        assert_eq!(status.state, CircuitState::Open);
        assert_eq!(status.opened_at, 5000);
    });
}

// ─────────────────────────────────────────────────────────
// 4. Circuit stays Open — all operations rejected
// ─────────────────────────────────────────────────────────

#[test]
fn test_circuit_open_rejects_operations() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        let result = check_and_allow(&env);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ERR_CIRCUIT_OPEN);
    });
}

#[test]
fn test_circuit_stays_open_across_multiple_check_attempts() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        for _ in 0..10 {
            assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));
        }
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(get_failure_count(&env), 2);
    });
}

#[test]
fn test_additional_failures_after_open_do_not_change_state() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        record_failure(&env, prog.clone(), op.clone(), ERR_TRANSFER_FAILED);
        record_failure(&env, prog, op, ERR_TRANSFER_FAILED);
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

#[test]
fn test_success_record_while_open_is_ignored() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

// ─────────────────────────────────────────────────────────
// 5. Admin reset: Open → HalfOpen
// ─────────────────────────────────────────────────────────

#[test]
fn test_reset_open_to_half_open() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
    });
}

#[test]
fn test_half_open_allows_one_operation_through() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert!(check_and_allow(&env).is_ok());
    });
}

#[test]
fn test_success_count_reset_on_half_open() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_success_count(&env), 0);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
    });
}

// ─────────────────────────────────────────────────────────
// 6. Success in HalfOpen closes the circuit
// ─────────────────────────────────────────────────────────

#[test]
fn test_success_in_half_open_closes_circuit() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
    });
}

#[test]
fn test_circuit_closed_fully_operational_after_half_open_recovery() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        record_success(&env);
        assert!(check_and_allow(&env).is_ok());
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
    });
}

#[test]
fn test_multi_success_threshold_half_open() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 2,
                success_threshold: 3,
                max_error_log: 10,
            },
        );
    });
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        record_success(&env);
        assert_eq!(
            get_state(&env),
            CircuitState::HalfOpen,
            "Still HalfOpen after 1 success"
        );
        record_success(&env);
        assert_eq!(
            get_state(&env),
            CircuitState::HalfOpen,
            "Still HalfOpen after 2 successes"
        );
        record_success(&env);
        assert_eq!(
            get_state(&env),
            CircuitState::Closed,
            "Closed after 3 successes"
        );
    });
}

// ─────────────────────────────────────────────────────────
// 7. Failure in HalfOpen re-opens circuit
// ─────────────────────────────────────────────────────────

#[test]
fn test_failure_in_half_open_reopens_circuit() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        let prog = String::from_str(&env, "TestProg");
        record_failure(&env, prog, symbol_short!("op"), ERR_TRANSFER_FAILED);
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

#[test]
fn test_reopen_after_half_open_failure_rejects_immediately() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        let prog = String::from_str(&env, "TestProg");
        record_failure(&env, prog, symbol_short!("op"), ERR_TRANSFER_FAILED);
        assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));
    });
}

#[test]
fn test_half_open_can_be_reset_again_after_reopen() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        let prog = String::from_str(&env, "TestProg");
        record_failure(&env, prog, symbol_short!("op"), ERR_TRANSFER_FAILED);
        assert_eq!(get_state(&env), CircuitState::Open);
    });
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::Closed);
    });
}

// ─────────────────────────────────────────────────────────
// 8. Hard reset: HalfOpen / Closed → Closed
// ─────────────────────────────────────────────────────────

#[test]
fn test_reset_half_open_goes_to_closed() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin); // Open → HalfOpen
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
    });
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin); // HalfOpen → Closed
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
    });
}

#[test]
fn test_reset_from_closed_stays_closed() {
    let (env, admin, contract_id) = setup_with_admin(3);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::Closed);
    });
}

// ─────────────────────────────────────────────────────────
// 9. Error log population and cap
// ─────────────────────────────────────────────────────────

#[test]
fn test_error_log_populated_on_failure() {
    let (env, _admin, contract_id) = setup_with_admin(10);
    env.as_contract(&contract_id, || {
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        record_failure(&env, prog, op, ERR_TRANSFER_FAILED);
        let log = get_error_log(&env);
        assert_eq!(log.len(), 1);
        let entry = log.get(0).unwrap();
        assert_eq!(entry.error_code, ERR_TRANSFER_FAILED);
        assert_eq!(entry.failure_count_at_time, 1);
    });
}

#[test]
fn test_error_log_capped_at_max() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 100,
                success_threshold: 1,
                max_error_log: 3,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        for _ in 0..7 {
            record_failure(&env, prog.clone(), op.clone(), ERR_TRANSFER_FAILED);
        }
        let log = get_error_log(&env);
        assert_eq!(log.len(), 3, "Log should be capped at max_error_log=3");
    });
}

#[test]
fn test_error_log_contains_latest_errors_when_capped() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 100,
                success_threshold: 1,
                max_error_log: 2,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        for _ in 0..5 {
            record_failure(&env, prog.clone(), op.clone(), ERR_TRANSFER_FAILED);
        }
        let log = get_error_log(&env);
        assert_eq!(log.len(), 2);
        let last = log.get(1).unwrap();
        assert_eq!(last.failure_count_at_time, 5);
    });
}

// ─────────────────────────────────────────────────────────
// 10. Retry integration: exhaustion opens circuit
// ─────────────────────────────────────────────────────────

#[test]
fn test_retry_exhaustion_opens_circuit() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 3,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig {
            max_attempts: 3,
            initial_backoff: 0,
            backoff_multiplier: 1,
            max_backoff: 0,
        };
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 3);
        assert_eq!(result.final_error, ERR_TRANSFER_FAILED);
        assert_eq!(result.total_delay, 0);
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

#[test]
fn test_retry_circuit_open_stops_immediately() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig {
            max_attempts: 5,
            initial_backoff: 0,
            backoff_multiplier: 1,
            max_backoff: 0,
        };
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Ok(()));
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 0);
        assert_eq!(result.final_error, ERR_CIRCUIT_OPEN);
        assert_eq!(result.total_delay, 0);
    });
}

// ─────────────────────────────────────────────────────────
// 11. Retry success resets failure streak
// ─────────────────────────────────────────────────────────

#[test]
fn test_retry_success_on_second_attempt_resets_failures() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 5,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig {
            max_attempts: 3,
            initial_backoff: 0,
            backoff_multiplier: 1,
            max_backoff: 0,
        };
        let mut call_count = 0u32;
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || {
            call_count += 1;
            if call_count < 2 {
                Err(ERR_TRANSFER_FAILED)
            } else {
                Ok(())
            }
        });
        assert!(result.succeeded);
        assert_eq!(result.attempts, 2);
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
    });
}

// ─────────────────────────────────────────────────────────
// 12. Unauthorized reset is rejected
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_unauthorized_reset_panics() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    let impostor = Address::generate(&env);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &impostor);
    });
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_reset_with_no_admin_set_panics() {
    let (env, contract_id) = setup_env();
    let random = Address::generate(&env);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &random);
    });
}

// ─────────────────────────────────────────────────────────
// 13. Config changes take effect
// ─────────────────────────────────────────────────────────

#[test]
fn test_config_change_threshold_takes_effect() {
    let (env, _admin, contract_id) = setup_with_admin(10);
    simulate_failures(&env, &contract_id, 5);
    env.as_contract(&contract_id, || {
        assert_eq!(
            get_state(&env),
            CircuitState::Closed,
            "Should still be Closed with threshold=10"
        );
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 5,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        record_failure(&env, prog, symbol_short!("op"), ERR_TRANSFER_FAILED);
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

#[test]
fn test_get_config_returns_set_values() {
    let (env, contract_id) = setup_env();
    env.as_contract(&contract_id, || {
        let cfg = CircuitBreakerConfig {
            failure_threshold: 7,
            success_threshold: 2,
            max_error_log: 15,
        };
        set_config(&env, cfg);
        let stored = get_config(&env);
        assert_eq!(stored.failure_threshold, 7);
        assert_eq!(stored.success_threshold, 2);
        assert_eq!(stored.max_error_log, 15);
    });
}

// ─────────────────────────────────────────────────────────
// 14. Full state machine walkthrough
// ─────────────────────────────────────────────────────────

#[test]
fn test_full_circuit_breaker_lifecycle() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 3,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
    });

    env.as_contract(&contract_id, || {
        // Phase 1: Normal operation
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert!(check_and_allow(&env).is_ok());
        record_success(&env);
        assert_eq!(get_failure_count(&env), 0);
    });

    simulate_failures(&env, &contract_id, 2);

    env.as_contract(&contract_id, || {
        // Phase 2: Partial failures
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 2);
        assert!(check_and_allow(&env).is_ok());
    });

    simulate_failures(&env, &contract_id, 1);

    env.as_contract(&contract_id, || {
        // Phase 3: Threshold hit
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));

        // Phase 4: Admin resets (first reset — own frame for require_auth)
        env.ledger().set_timestamp(2000);
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        assert!(check_and_allow(&env).is_ok());
    });

    env.as_contract(&contract_id, || {
        // Phase 5: Failure in HalfOpen
        let prog = String::from_str(&env, "TestProg");
        record_failure(&env, prog.clone(), symbol_short!("op"), ERR_TRANSFER_FAILED);
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));
    });

    env.as_contract(&contract_id, || {
        // Phase 6: Admin resets again (second reset — own frame for require_auth)
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);

        // Phase 7: Success closes
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
        assert!(check_and_allow(&env).is_ok());

        // Phase 8: Error log has entries
        let log = get_error_log(&env);
        assert!(
            log.len() > 0,
            "Error log should contain entries from failures"
        );
    });
}

// ─────────────────────────────────────────────────────────
// 14b. Circuit stays open until reset (no auto-recovery)
// ─────────────────────────────────────────────────────────

#[test]
fn test_circuit_stays_open_until_admin_reset() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        // Advance time — circuit does not auto-close
        env.ledger().set_timestamp(100_000);
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));
        // Multiple check_and_allow calls still reject
        for _ in 0..20 {
            assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));
        }
    });
}

#[test]
fn test_reset_circuit_breaker_state_checks_open_to_half_open() {
    let (env, admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);
    env.as_contract(&contract_id, || {
        let status_before = get_status(&env);
        assert_eq!(status_before.state, CircuitState::Open);
        assert!(status_before.failure_count >= 3);

        reset_circuit_breaker(&env, &admin);

        let status_after = get_status(&env);
        assert_eq!(status_after.state, CircuitState::HalfOpen);
        assert_eq!(status_after.success_count, 0);
    });
}

#[test]
fn test_reset_circuit_breaker_state_checks_successful_recovery_to_closed() {
    let (env, admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        record_success(&env);
        let status = get_status(&env);
        assert_eq!(status.state, CircuitState::Closed);
        assert_eq!(status.failure_count, 0);
        assert_eq!(status.opened_at, 0);
    });
}

// ─────────────────────────────────────────────────────────
// 15. Status snapshot is accurate
// ─────────────────────────────────────────────────────────

#[test]
fn test_status_snapshot_reflects_state() {
    let (env, admin, contract_id) = setup_with_admin(3);
    env.ledger().set_timestamp(9999);
    simulate_failures(&env, &contract_id, 3);
    env.as_contract(&contract_id, || {
        let status = get_status(&env);
        assert_eq!(status.state, CircuitState::Open);
        assert_eq!(status.failure_count, 3);
        assert_eq!(status.opened_at, 9999);
        assert_eq!(status.failure_threshold, 3);

        reset_circuit_breaker(&env, &admin);
        let status2 = get_status(&env);
        assert_eq!(status2.state, CircuitState::HalfOpen);
        assert_eq!(status2.success_count, 0);

        record_success(&env);
        let status3 = get_status(&env);
        assert_eq!(status3.state, CircuitState::Closed);
        assert_eq!(status3.failure_count, 0);
    });
}

// ─────────────────────────────────────────────────────────
// 16. Direct open/close/half_open functions
// ─────────────────────────────────────────────────────────

#[test]
fn test_direct_open_circuit() {
    let (env, contract_id) = setup_env();
    env.as_contract(&contract_id, || {
        open_circuit(&env);
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));
    });
}

#[test]
fn test_direct_close_circuit_resets_counters() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        close_circuit(&env);
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
        assert_eq!(get_success_count(&env), 0);
        assert!(check_and_allow(&env).is_ok());
    });
}

#[test]
fn test_direct_half_open_circuit() {
    let (env, _admin, contract_id) = setup_with_admin(2);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        half_open_circuit(&env);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        assert_eq!(get_success_count(&env), 0);
        assert!(check_and_allow(&env).is_ok());
    });
}

// ─────────────────────────────────────────────────────────
// 17. Admin management
// ─────────────────────────────────────────────────────────

#[test]
fn test_set_and_get_circuit_admin() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        assert_eq!(get_circuit_admin(&env), Some(admin));
    });
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_change_admin() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    let impostor = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_circuit_admin(&env, impostor.clone(), Some(impostor));
    });
}

#[test]
fn test_admin_can_update_admin() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_circuit_admin(&env, new_admin.clone(), Some(admin));
        assert_eq!(get_circuit_admin(&env), Some(new_admin));
    });
}

// ─────────────────────────────────────────────────────────
// 18. Closed → success never opens circuit
// ─────────────────────────────────────────────────────────

#[test]
fn test_many_successes_in_closed_state_never_open() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    env.as_contract(&contract_id, || {
        for _ in 0..100 {
            record_success(&env);
        }
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
    });
}

#[test]
fn test_interleaved_failures_and_successes_do_not_open_if_never_hit_threshold() {
    let (env, _admin, contract_id) = setup_with_admin(5);
    env.as_contract(&contract_id, || {
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");

        record_failure(&env, prog.clone(), op.clone(), ERR_TRANSFER_FAILED);
        assert_eq!(get_failure_count(&env), 1);

        record_success(&env);
        assert_eq!(get_failure_count(&env), 0);

        record_failure(&env, prog.clone(), op.clone(), ERR_TRANSFER_FAILED);
        assert_eq!(get_failure_count(&env), 1);

        record_success(&env);
        assert_eq!(get_failure_count(&env), 0);

        assert_eq!(get_state(&env), CircuitState::Closed);
    });
}

// ─────────────────────────────────────────────────────────
// 19. Retry policy presets and backoff computation
// ─────────────────────────────────────────────────────────

#[test]
fn test_retry_config_default_preset() {
    let config = RetryConfig::default();
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.initial_backoff, 0);
    assert_eq!(config.backoff_multiplier, 1);
    assert_eq!(config.max_backoff, 0);
}

#[test]
fn test_retry_config_aggressive_preset() {
    let config = RetryConfig::aggressive();
    assert_eq!(config.max_attempts, 5);
    assert_eq!(config.initial_backoff, 1);
    assert_eq!(config.backoff_multiplier, 1);
    assert_eq!(config.max_backoff, 5);
}

#[test]
fn test_retry_config_conservative_preset() {
    let config = RetryConfig::conservative();
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.initial_backoff, 10);
    assert_eq!(config.backoff_multiplier, 2);
    assert_eq!(config.max_backoff, 100);
}

#[test]
fn test_retry_config_exponential_preset() {
    let config = RetryConfig::exponential();
    assert_eq!(config.max_attempts, 4);
    assert_eq!(config.initial_backoff, 5);
    assert_eq!(config.backoff_multiplier, 3);
    assert_eq!(config.max_backoff, 200);
}

// ─────────────────────────────────────────────────────────
// 20. Backoff delay computation tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_compute_backoff_no_delay() {
    let config = RetryConfig::default();
    assert_eq!(config.compute_backoff(0), 0);
    assert_eq!(config.compute_backoff(1), 0);
    assert_eq!(config.compute_backoff(5), 0);
}

#[test]
fn test_compute_backoff_constant_delay() {
    let config = RetryConfig::aggressive();
    // Constant backoff: multiplier = 1
    assert_eq!(config.compute_backoff(0), 1); // 1 * 1^0 = 1
    assert_eq!(config.compute_backoff(1), 1); // 1 * 1^1 = 1
    assert_eq!(config.compute_backoff(2), 1); // 1 * 1^2 = 1
    assert_eq!(config.compute_backoff(3), 1); // 1 * 1^3 = 1
}

#[test]
fn test_compute_backoff_exponential_growth() {
    let config = RetryConfig::conservative();
    // Exponential backoff: initial=10, multiplier=2
    assert_eq!(config.compute_backoff(0), 10); // 10 * 2^0 = 10
    assert_eq!(config.compute_backoff(1), 20); // 10 * 2^1 = 20
    assert_eq!(config.compute_backoff(2), 40); // 10 * 2^2 = 40
    assert_eq!(config.compute_backoff(3), 80); // 10 * 2^3 = 80
}

#[test]
fn test_compute_backoff_capped_at_max() {
    let config = RetryConfig::conservative();
    // Should cap at max_backoff = 100
    assert_eq!(config.compute_backoff(4), 100); // 10 * 2^4 = 160, capped to 100
    assert_eq!(config.compute_backoff(5), 100); // 10 * 2^5 = 320, capped to 100
}

#[test]
fn test_compute_backoff_exponential_preset() {
    let config = RetryConfig::exponential();
    // Exponential backoff: initial=5, multiplier=3
    assert_eq!(config.compute_backoff(0), 5); // 5 * 3^0 = 5
    assert_eq!(config.compute_backoff(1), 15); // 5 * 3^1 = 15
    assert_eq!(config.compute_backoff(2), 45); // 5 * 3^2 = 45
    assert_eq!(config.compute_backoff(3), 135); // 5 * 3^3 = 135
    assert_eq!(config.compute_backoff(4), 200); // 5 * 3^4 = 405, capped to 200
}

// ─────────────────────────────────────────────────────────
// 21. Aggressive policy behavior tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_aggressive_policy_max_attempts() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::aggressive();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        assert!(!result.succeeded);
        assert_eq!(
            result.attempts, 5,
            "Aggressive policy should attempt 5 times"
        );
        assert_eq!(result.final_error, ERR_TRANSFER_FAILED);
    });
}

#[test]
fn test_aggressive_policy_minimal_backoff() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::aggressive();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        // Aggressive: constant backoff of 1, 4 retries (attempts 2-5)
        // Total delay = 1 + 1 + 1 + 1 = 4
        assert_eq!(
            result.total_delay, 4,
            "Aggressive policy should have minimal total delay"
        );
    });
}

#[test]
fn test_aggressive_policy_succeeds_on_last_attempt() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::aggressive();
        let mut call_count = 0u32;
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || {
            call_count += 1;
            if call_count < 5 {
                Err(ERR_TRANSFER_FAILED)
            } else {
                Ok(())
            }
        });
        assert!(result.succeeded);
        assert_eq!(result.attempts, 5);
        assert_eq!(result.final_error, 0);
    });
}

// ─────────────────────────────────────────────────────────
// 22. Conservative policy behavior tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_conservative_policy_max_attempts() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::conservative();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        assert!(!result.succeeded);
        assert_eq!(
            result.attempts, 3,
            "Conservative policy should attempt 3 times"
        );
        assert_eq!(result.final_error, ERR_TRANSFER_FAILED);
    });
}

#[test]
fn test_conservative_policy_exponential_backoff() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::conservative();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        // Conservative: exponential backoff 10, 20 (2 retries for attempts 2-3)
        // Total delay = 10 + 20 = 30
        assert_eq!(
            result.total_delay, 30,
            "Conservative policy should have exponential backoff"
        );
    });
}

#[test]
fn test_conservative_policy_succeeds_early() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::conservative();
        let mut call_count = 0u32;
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || {
            call_count += 1;
            if call_count < 2 {
                Err(ERR_TRANSFER_FAILED)
            } else {
                Ok(())
            }
        });
        assert!(result.succeeded);
        assert_eq!(result.attempts, 2);
        // Only one retry, so delay = 10
        assert_eq!(result.total_delay, 10);
    });
}

// ─────────────────────────────────────────────────────────
// 23. Exponential policy behavior tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_exponential_policy_max_attempts() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::exponential();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        assert!(!result.succeeded);
        assert_eq!(
            result.attempts, 4,
            "Exponential policy should attempt 4 times"
        );
    });
}

#[test]
fn test_exponential_policy_strong_backoff() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::exponential();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        // Exponential: 5, 15, 45 (3 retries for attempts 2-4)
        // Total delay = 5 + 15 + 45 = 65
        assert_eq!(
            result.total_delay, 65,
            "Exponential policy should have strong backoff growth"
        );
    });
}

// ─────────────────────────────────────────────────────────
// 24. Policy comparison tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_policy_comparison_attempt_counts() {
    let aggressive = RetryConfig::aggressive();
    let conservative = RetryConfig::conservative();
    let exponential = RetryConfig::exponential();

    assert_eq!(aggressive.max_attempts, 5, "Aggressive: 5 attempts");
    assert_eq!(conservative.max_attempts, 3, "Conservative: 3 attempts");
    assert_eq!(exponential.max_attempts, 4, "Exponential: 4 attempts");

    assert!(aggressive.max_attempts > conservative.max_attempts);
    assert!(aggressive.max_attempts > exponential.max_attempts);
}

#[test]
fn test_policy_comparison_backoff_sequences() {
    let aggressive = RetryConfig::aggressive();
    let conservative = RetryConfig::conservative();
    let exponential = RetryConfig::exponential();

    // Compare backoff sequences for first 3 retries
    // Aggressive: constant 1
    assert_eq!(aggressive.compute_backoff(0), 1);
    assert_eq!(aggressive.compute_backoff(1), 1);
    assert_eq!(aggressive.compute_backoff(2), 1);

    // Conservative: exponential 10, 20, 40
    assert_eq!(conservative.compute_backoff(0), 10);
    assert_eq!(conservative.compute_backoff(1), 20);
    assert_eq!(conservative.compute_backoff(2), 40);

    // Exponential: strong growth 5, 15, 45
    assert_eq!(exponential.compute_backoff(0), 5);
    assert_eq!(exponential.compute_backoff(1), 15);
    assert_eq!(exponential.compute_backoff(2), 45);

    // Verify aggressive has minimal delays
    assert!(aggressive.compute_backoff(0) < conservative.compute_backoff(0));
    assert!(aggressive.compute_backoff(1) < conservative.compute_backoff(1));
    assert!(aggressive.compute_backoff(2) < conservative.compute_backoff(2));
}

#[test]
fn test_policy_comparison_total_delays() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 20,
                success_threshold: 1,
                max_error_log: 20,
            },
        );
    });

    // Test aggressive policy
    let aggressive_delay = env.as_contract(&contract_id, || {
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op_agg");
        let retry_cfg = RetryConfig::aggressive();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        result.total_delay
    });

    // Reset circuit for next test
    env.as_contract(&contract_id, || {
        close_circuit(&env);
    });

    // Test conservative policy
    let conservative_delay = env.as_contract(&contract_id, || {
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op_con");
        let retry_cfg = RetryConfig::conservative();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        result.total_delay
    });

    // Aggressive should have much lower total delay
    assert!(
        aggressive_delay < conservative_delay,
        "Aggressive delay ({}) should be less than conservative delay ({})",
        aggressive_delay,
        conservative_delay
    );
}

// ─────────────────────────────────────────────────────────
// 25. Max retry reached behavior
// ─────────────────────────────────────────────────────────

#[test]
fn test_aggressive_policy_max_retries_reached() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::aggressive();
        let mut call_count = 0u32;
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || {
            call_count += 1;
            Err(ERR_TRANSFER_FAILED)
        });
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 5);
        assert_eq!(
            call_count, 5,
            "Should have called operation exactly max_attempts times"
        );
        assert_eq!(result.final_error, ERR_TRANSFER_FAILED);
    });
}

#[test]
fn test_conservative_policy_max_retries_reached() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::conservative();
        let mut call_count = 0u32;
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || {
            call_count += 1;
            Err(ERR_TRANSFER_FAILED)
        });
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 3);
        assert_eq!(
            call_count, 3,
            "Should have called operation exactly max_attempts times"
        );
    });
}

// ─────────────────────────────────────────────────────────
// 26. Custom retry policy tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_custom_retry_policy_no_backoff() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig {
            max_attempts: 7,
            initial_backoff: 0,
            backoff_multiplier: 1,
            max_backoff: 0,
        };
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        assert_eq!(result.attempts, 7);
        assert_eq!(result.total_delay, 0);
    });
}

#[test]
fn test_custom_retry_policy_high_multiplier() {
    let config = RetryConfig {
        max_attempts: 5,
        initial_backoff: 2,
        backoff_multiplier: 5,
        max_backoff: 1000,
    };
    // Verify exponential growth with high multiplier
    assert_eq!(config.compute_backoff(0), 2); // 2 * 5^0 = 2
    assert_eq!(config.compute_backoff(1), 10); // 2 * 5^1 = 10
    assert_eq!(config.compute_backoff(2), 50); // 2 * 5^2 = 50
    assert_eq!(config.compute_backoff(3), 250); // 2 * 5^3 = 250
    assert_eq!(config.compute_backoff(4), 1000); // 2 * 5^4 = 1250, capped to 1000
}

#[test]
fn test_custom_retry_policy_max_backoff_cap() {
    let config = RetryConfig {
        max_attempts: 10,
        initial_backoff: 100,
        backoff_multiplier: 2,
        max_backoff: 150,
    };
    // Should cap at max_backoff
    assert_eq!(config.compute_backoff(0), 100); // 100 * 2^0 = 100
    assert_eq!(config.compute_backoff(1), 150); // 100 * 2^1 = 200, capped to 150
    assert_eq!(config.compute_backoff(2), 150); // 100 * 2^2 = 400, capped to 150
    assert_eq!(config.compute_backoff(5), 150); // Always capped
}

// ─────────────────────────────────────────────────────────
// 27. Retry policy interaction with circuit breaker
// ─────────────────────────────────────────────────────────

#[test]
fn test_aggressive_policy_opens_circuit_on_exhaustion() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 5,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::aggressive();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 5);
        // Circuit should be open after 5 failures
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(get_failure_count(&env), 5);
    });
}

#[test]
fn test_conservative_policy_opens_circuit_on_exhaustion() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 3,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::conservative();
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 3);
        // Circuit should be open after 3 failures
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

#[test]
fn test_policy_stops_on_circuit_open_mid_retry() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 2,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig::aggressive(); // 5 attempts
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        // Should stop at 2 attempts when circuit opens
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 2, "Should stop when circuit opens");
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

// ─────────────────────────────────────────────────────────
// 28. Edge cases and boundary conditions
// ─────────────────────────────────────────────────────────

#[test]
fn test_single_attempt_no_retry() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 10,
                success_threshold: 1,
                max_error_log: 10,
            },
        );
        let prog = String::from_str(&env, "TestProg");
        let op = symbol_short!("op");
        let retry_cfg = RetryConfig {
            max_attempts: 1,
            initial_backoff: 10,
            backoff_multiplier: 2,
            max_backoff: 100,
        };
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 1);
        assert_eq!(result.total_delay, 0, "No delay on single attempt");
    });
}

#[test]
fn test_zero_initial_backoff_with_multiplier() {
    let config = RetryConfig {
        max_attempts: 5,
        initial_backoff: 0,
        backoff_multiplier: 10,
        max_backoff: 1000,
    };
    // All delays should be 0 when initial_backoff is 0
    assert_eq!(config.compute_backoff(0), 0);
    assert_eq!(config.compute_backoff(1), 0);
    assert_eq!(config.compute_backoff(10), 0);
}

// ─────────────────────────────────────────────────────────
// 29. Batch Recovery Configuration Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_batch_recovery_config_default() {
    let config = crate::error_recovery::BatchRecoveryConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.auto_retry, false);
    assert_eq!(config.max_batch_size, 100);
    assert_eq!(config.rollback_enabled, true);
    assert_eq!(config.recovery_timeout_secs, 86400);
}

#[test]
fn test_batch_recovery_config_validation() {
    // Valid config
    let valid = crate::error_recovery::BatchRecoveryConfig::default();
    assert!(valid.validate().is_ok());

    // Invalid: max_retries too high
    let invalid_retries = crate::error_recovery::BatchRecoveryConfig {
        max_retries: 15,
        ..crate::error_recovery::BatchRecoveryConfig::default()
    };
    assert!(invalid_retries.validate().is_err());

    // Invalid: batch size 0
    let invalid_batch_zero = crate::error_recovery::BatchRecoveryConfig {
        max_batch_size: 0,
        ..crate::error_recovery::BatchRecoveryConfig::default()
    };
    assert!(invalid_batch_zero.validate().is_err());

    // Invalid: batch size too large
    let invalid_batch_large = crate::error_recovery::BatchRecoveryConfig {
        max_batch_size: 600,
        ..crate::error_recovery::BatchRecoveryConfig::default()
    };
    assert!(invalid_batch_large.validate().is_err());

    // Invalid: timeout too short
    let invalid_timeout_short = crate::error_recovery::BatchRecoveryConfig {
        recovery_timeout_secs: 100,
        ..crate::error_recovery::BatchRecoveryConfig::default()
    };
    assert!(invalid_timeout_short.validate().is_err());

    // Invalid: timeout too long
    let invalid_timeout_long = crate::error_recovery::BatchRecoveryConfig {
        recovery_timeout_secs: 700000,
        ..crate::error_recovery::BatchRecoveryConfig::default()
    };
    assert!(invalid_timeout_long.validate().is_err());
}

#[test]
fn test_set_and_get_batch_recovery_config() {
    let (env, contract_id) = setup_env();
    let config = crate::error_recovery::BatchRecoveryConfig {
        max_retries: 5,
        auto_retry: true,
        max_batch_size: 50,
        rollback_enabled: false,
        recovery_timeout_secs: 172800,
    };

    env.as_contract(&contract_id, || {
        let result = crate::error_recovery::set_batch_recovery_config(&env, config.clone());
        assert!(result.is_ok());

        let stored = crate::error_recovery::get_batch_recovery_config(&env);
        assert_eq!(stored.max_retries, 5);
        assert_eq!(stored.auto_retry, true);
        assert_eq!(stored.max_batch_size, 50);
        assert_eq!(stored.rollback_enabled, false);
        assert_eq!(stored.recovery_timeout_secs, 172800);
    });
}

// ─────────────────────────────────────────────────────────
// 30. Batch State Storage Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_store_batch_state_basic() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient1.clone(), recipient2.clone()];
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128];

        let result = crate::error_recovery::store_batch_state(
            &env,
            program_id.clone(),
            recipients,
            amounts,
            10000i128,
            authorized.clone(),
        );

        assert!(result.is_ok());
        let batch_id = result.unwrap();
        assert!(batch_id > 0);

        let state = crate::error_recovery::get_batch_state(&env, batch_id);
        assert!(state.is_some());

        let state = state.unwrap();
        assert_eq!(state.batch_id, batch_id);
        assert_eq!(state.program_id, program_id);
        assert_eq!(state.original_balance, 10000);
        assert_eq!(state.total_amount, 3000);
        assert_eq!(state.successful_amount, 0);
        assert_eq!(state.item_count, 2);
        assert_eq!(state.pending_recovery, false);
        assert_eq!(state.authorized_key, authorized);
    });
}

#[test]
fn test_store_batch_state_exceeds_max_size() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Set a small max batch size
        let config = crate::error_recovery::BatchRecoveryConfig {
            max_batch_size: 2,
            ..crate::error_recovery::BatchRecoveryConfig::default()
        };
        let _ = crate::error_recovery::set_batch_recovery_config(&env, config);

        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![
            &env,
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env)
        ];
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128, 3000i128];

        let result = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::error_recovery::ERR_BATCH_SIZE_EXCEEDED);
    });
}

#[test]
fn test_get_batch_state_not_found() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        let state = crate::error_recovery::get_batch_state(&env, 99999);
        assert!(state.is_none());
    });
}

#[test]
fn test_clear_batch_state() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Verify it exists
        assert!(crate::error_recovery::get_batch_state(&env, batch_id).is_some());

        // Clear it
        crate::error_recovery::clear_batch_state(&env, batch_id);

        // Verify it's gone
        assert!(crate::error_recovery::get_batch_state(&env, batch_id).is_none());
    });
}

// ─────────────────────────────────────────────────────────
// 31. Batch Item Status Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_mark_item_success() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient.clone()];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        let result = crate::error_recovery::mark_item_success(&env, batch_id, 0);
        assert!(result.is_ok());

        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        assert_eq!(state.successful_amount, 1000);

        let item = state.items.get(0).unwrap();
        assert_eq!(item.status, crate::error_recovery::BatchItemStatus::Success);
    });
}

#[test]
fn test_mark_item_failed() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        let result = crate::error_recovery::mark_item_failed(&env, batch_id, 0, 500);
        assert!(result.is_ok());

        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        assert_eq!(state.pending_recovery, true);

        let item = state.items.get(0).unwrap();
        assert_eq!(item.status, crate::error_recovery::BatchItemStatus::Failed);
        assert_eq!(item.error_code, 500);
    });
}

#[test]
fn test_mark_item_rolled_back() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // First mark as success
        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();

        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        assert_eq!(state.successful_amount, 1000);

        // Then roll back
        crate::error_recovery::mark_item_rolled_back(&env, batch_id, 0).unwrap();

        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        assert_eq!(state.successful_amount, 0);

        let item = state.items.get(0).unwrap();
        assert_eq!(item.status, crate::error_recovery::BatchItemStatus::RolledBack);
    });
}

#[test]
fn test_mark_item_invalid_index() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Try to mark an item with invalid index
        let result = crate::error_recovery::mark_item_success(&env, batch_id, 99);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::error_recovery::ERR_BATCH_NOT_FOUND);
    });
}

// ─────────────────────────────────────────────────────────
// 32. Partial Failure Recovery Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_get_failed_items() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient1, recipient2, recipient3];
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128, 3000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Mark first as success, second as failed, third as pending
        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
        crate::error_recovery::mark_item_failed(&env, batch_id, 1, 500).unwrap();
        // Item 2 remains pending

        let failed = crate::error_recovery::get_failed_items(&env, batch_id).unwrap();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed.get(0).unwrap(), 1);
    });
}

#[test]
fn test_get_successful_items() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient1, recipient2, recipient3];
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128, 3000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Mark first as success, second as failed, third as success
        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
        crate::error_recovery::mark_item_failed(&env, batch_id, 1, 500).unwrap();
        crate::error_recovery::mark_item_success(&env, batch_id, 2).unwrap();

        let successful = crate::error_recovery::get_successful_items(&env, batch_id).unwrap();
        assert_eq!(successful.len(), 2);
        assert_eq!(successful.get(0).unwrap(), 0);
        assert_eq!(successful.get(1).unwrap(), 2);
    });
}

#[test]
fn test_partial_batch_failure_tracking() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipients = soroban_sdk::vec![
        &env,
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128, 3000i128, 4000i128, 5000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            20000i128,
            authorized,
        ).unwrap();

        // Simulate partial failure: items 0, 2, 4 succeed; 1, 3 fail
        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
        crate::error_recovery::mark_item_failed(&env, batch_id, 1, 100).unwrap();
        crate::error_recovery::mark_item_success(&env, batch_id, 2).unwrap();
        crate::error_recovery::mark_item_failed(&env, batch_id, 3, 200).unwrap();
        crate::error_recovery::mark_item_success(&env, batch_id, 4).unwrap();

        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();

        // Verify amounts
        assert_eq!(state.successful_amount, 1000 + 3000 + 5000); // 9000
        assert_eq!(state.total_amount, 15000);
        assert_eq!(state.pending_recovery, true);

        // Verify failed items count
        let failed = crate::error_recovery::get_failed_items(&env, batch_id).unwrap();
        assert_eq!(failed.len(), 2);

        // Verify successful items count
        let successful = crate::error_recovery::get_successful_items(&env, batch_id).unwrap();
        assert_eq!(successful.len(), 3);
    });
}

// ─────────────────────────────────────────────────────────
// 33. Rollback Mechanism Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_calculate_rollback_amount() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient1, recipient2, recipient3];
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128, 3000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Only mark first two as success
        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
        crate::error_recovery::mark_item_success(&env, batch_id, 1).unwrap();
        // Third item fails
        crate::error_recovery::mark_item_failed(&env, batch_id, 2, 500).unwrap();

        let rollback_amount = crate::error_recovery::calculate_rollback_amount(&env, batch_id).unwrap();
        assert_eq!(rollback_amount, 3000); // 1000 + 2000
    });
}

#[test]
fn test_calculate_rollback_no_successful_items() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Only failure, no success
        crate::error_recovery::mark_item_failed(&env, batch_id, 0, 500).unwrap();

        let rollback_amount = crate::error_recovery::calculate_rollback_amount(&env, batch_id).unwrap();
        assert_eq!(rollback_amount, 0);
    });
}

#[test]
fn test_prepare_rollback_success() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient1.clone(), recipient2.clone()];
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized.clone(),
        ).unwrap();

        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
        crate::error_recovery::mark_item_success(&env, batch_id, 1).unwrap();

        let result = crate::error_recovery::prepare_rollback(&env, batch_id, &authorized);
        assert!(result.is_ok());

        let rollback = result.unwrap();
        assert_eq!(rollback.batch_id, batch_id);
        assert_eq!(rollback.items_rolled_back, 2);
        assert_eq!(rollback.amount_returned, 3000);
        assert_eq!(rollback.affected_recipients.len(), 2);
    });
}

#[test]
fn test_prepare_rollback_unauthorized() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let impostor = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();

        let result = crate::error_recovery::prepare_rollback(&env, batch_id, &impostor);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::error_recovery::ERR_UNAUTHORIZED_RECOVERY);
    });
}

#[test]
fn test_prepare_rollback_no_successful_items() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized.clone(),
        ).unwrap();

        // Only failure, no success
        crate::error_recovery::mark_item_failed(&env, batch_id, 0, 500).unwrap();

        let result = crate::error_recovery::prepare_rollback(&env, batch_id, &authorized);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::error_recovery::ERR_NO_SUCCESSFUL_ITEMS);
    });
}

#[test]
fn test_rollback_disabled() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Disable rollback
        let config = crate::error_recovery::BatchRecoveryConfig {
            rollback_enabled: false,
            ..crate::error_recovery::BatchRecoveryConfig::default()
        };
        let _ = crate::error_recovery::set_batch_recovery_config(&env, config);

        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized.clone(),
        ).unwrap();

        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();

        let result = crate::error_recovery::calculate_rollback_amount(&env, batch_id);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::error_recovery::ERR_ROLLBACK_DISABLED);
    });
}

// ─────────────────────────────────────────────────────────
// 34. Batch Integrity Verification Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_verify_batch_integrity_valid() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient1, recipient2, recipient3];
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128, 3000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Mark items with different statuses
        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
        crate::error_recovery::mark_item_failed(&env, batch_id, 1, 500).unwrap();
        // Item 2 remains pending

        // Integrity should pass
        assert!(crate::error_recovery::verify_batch_integrity(&env, batch_id));
    });
}

#[test]
fn test_verify_batch_integrity_all_success() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient1, recipient2];
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
        crate::error_recovery::mark_item_success(&env, batch_id, 1).unwrap();

        assert!(crate::error_recovery::verify_batch_integrity(&env, batch_id));
    });
}

#[test]
fn test_verify_batch_integrity_nonexistent_batch() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        assert!(!crate::error_recovery::verify_batch_integrity(&env, 99999));
    });
}

// ─────────────────────────────────────────────────────────
// 35. Recovery Expiration Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_is_recovery_expired_not_expired() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Should not be expired immediately after creation
        assert!(!crate::error_recovery::is_recovery_expired(&env, batch_id));
    });
}

#[test]
fn test_is_recovery_expired_after_timeout() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Set a very short timeout
        let config = crate::error_recovery::BatchRecoveryConfig {
            recovery_timeout_secs: 3600, // 1 hour
            ..crate::error_recovery::BatchRecoveryConfig::default()
        };
        let _ = crate::error_recovery::set_batch_recovery_config(&env, config);

        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Advance time past timeout
        env.ledger().set_timestamp(5000); // Well past 1000 + 3600

        assert!(crate::error_recovery::is_recovery_expired(&env, batch_id));
    });
}

#[test]
fn test_is_recovery_expired_nonexistent_batch() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        // Non-existent batches are considered expired
        assert!(crate::error_recovery::is_recovery_expired(&env, 99999));
    });
}

// ─────────────────────────────────────────────────────────
// 36. Cancel Batch Recovery Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_cancel_batch_recovery_success() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized.clone(),
        ).unwrap();

        crate::error_recovery::mark_item_failed(&env, batch_id, 0, 500).unwrap();

        let result = crate::error_recovery::cancel_batch_recovery(&env, batch_id, &authorized);
        assert!(result.is_ok());

        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        assert_eq!(state.pending_recovery, false);
        assert!(state.completed_at.is_some());
    });
}

#[test]
fn test_cancel_batch_recovery_unauthorized() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let impostor = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        let result = crate::error_recovery::cancel_batch_recovery(&env, batch_id, &impostor);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::error_recovery::ERR_UNAUTHORIZED_RECOVERY);
    });
}

#[test]
fn test_cancel_batch_recovery_not_found() {
    let (env, contract_id) = setup_env();
    let caller = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let result = crate::error_recovery::cancel_batch_recovery(&env, 99999, &caller);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::error_recovery::ERR_BATCH_NOT_FOUND);
    });
}

// ─────────────────────────────────────────────────────────
// 37. Pending Recoveries List Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_get_pending_recoveries_empty() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        let pending = crate::error_recovery::get_pending_recoveries(&env);
        assert_eq!(pending.len(), 0);
    });
}

#[test]
fn test_get_pending_recoveries_multiple() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient.clone()];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id1 = crate::error_recovery::store_batch_state(
            &env,
            program_id.clone(),
            recipients.clone(),
            amounts.clone(),
            10000i128,
            authorized.clone(),
        ).unwrap();

        let batch_id2 = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        let pending = crate::error_recovery::get_pending_recoveries(&env);
        assert_eq!(pending.len(), 2);

        // Clear one batch
        crate::error_recovery::clear_batch_state(&env, batch_id1);

        let pending = crate::error_recovery::get_pending_recoveries(&env);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending.get(0).unwrap(), batch_id2);
    });
}

// ─────────────────────────────────────────────────────────
// 38. Finalize Batch Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_finalize_batch_success() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient1, recipient2];
        let amounts = soroban_sdk::vec![&env, 1000i128, 2000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Mark all as success
        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
        crate::error_recovery::mark_item_success(&env, batch_id, 1).unwrap();

        let result = crate::error_recovery::finalize_batch(&env, batch_id);
        assert!(result.is_ok());

        // Batch should be cleared after finalization
        let state = crate::error_recovery::get_batch_state(&env, batch_id);
        assert!(state.is_none());

        // Should be in history
        let history = crate::error_recovery::get_recovery_history(&env);
        assert!(history.len() > 0);
    });
}

#[test]
fn test_finalize_batch_not_found() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        let result = crate::error_recovery::finalize_batch(&env, 99999);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::error_recovery::ERR_BATCH_NOT_FOUND);
    });
}

// ─────────────────────────────────────────────────────────
// 39. Batch Recovery Invariants Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_verify_batch_recovery_invariants_empty() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        assert!(crate::error_recovery::verify_batch_recovery_invariants(&env));
    });
}

#[test]
fn test_verify_batch_recovery_invariants_valid() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();

        assert!(crate::error_recovery::verify_batch_recovery_invariants(&env));
    });
}

// ─────────────────────────────────────────────────────────
// 40. Edge Cases and Boundary Conditions
// ─────────────────────────────────────────────────────────

#[test]
fn test_single_item_batch() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        assert_eq!(state.item_count, 1);
        assert_eq!(state.total_amount, 1000);

        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();

        let failed = crate::error_recovery::get_failed_items(&env, batch_id).unwrap();
        assert_eq!(failed.len(), 0);

        let successful = crate::error_recovery::get_successful_items(&env, batch_id).unwrap();
        assert_eq!(successful.len(), 1);
    });
}

#[test]
fn test_large_batch_amounts() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient1, recipient2];

        // Large amounts near i128::MAX / 2
        let large_amount = 1_000_000_000_000_000_000i128;
        let amounts = soroban_sdk::vec![&env, large_amount, large_amount];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            large_amount * 3,
            authorized,
        ).unwrap();

        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        assert_eq!(state.total_amount, large_amount * 2);
    });
}

#[test]
fn test_batch_item_status_transitions() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        // Check initial status
        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        let item = state.items.get(0).unwrap();
        assert_eq!(item.status, crate::error_recovery::BatchItemStatus::Pending);

        // Transition to success
        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        let item = state.items.get(0).unwrap();
        assert_eq!(item.status, crate::error_recovery::BatchItemStatus::Success);

        // Transition to rolled back
        crate::error_recovery::mark_item_rolled_back(&env, batch_id, 0).unwrap();
        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        let item = state.items.get(0).unwrap();
        assert_eq!(item.status, crate::error_recovery::BatchItemStatus::RolledBack);
    });
}

#[test]
fn test_multiple_batches_isolation() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");

        // Create first batch
        let batch_id1 = crate::error_recovery::store_batch_state(
            &env,
            program_id.clone(),
            soroban_sdk::vec![&env, recipient1.clone()],
            soroban_sdk::vec![&env, 1000i128],
            10000i128,
            authorized.clone(),
        ).unwrap();

        // Create second batch
        let batch_id2 = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            soroban_sdk::vec![&env, recipient2],
            soroban_sdk::vec![&env, 2000i128],
            10000i128,
            authorized,
        ).unwrap();

        // Modify first batch
        crate::error_recovery::mark_item_success(&env, batch_id1, 0).unwrap();

        // Modify second batch
        crate::error_recovery::mark_item_failed(&env, batch_id2, 0, 500).unwrap();

        // Verify isolation
        let state1 = crate::error_recovery::get_batch_state(&env, batch_id1).unwrap();
        let state2 = crate::error_recovery::get_batch_state(&env, batch_id2).unwrap();

        assert_eq!(state1.successful_amount, 1000);
        assert_eq!(state1.pending_recovery, false);

        assert_eq!(state2.successful_amount, 0);
        assert_eq!(state2.pending_recovery, true);
    });
}

// ─────────────────────────────────────────────────────────
// 41. Recovery History Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_recovery_history_archives_batches() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();

        // Initially history is empty
        let history = crate::error_recovery::get_recovery_history(&env);
        assert_eq!(history.len(), 0);

        // Finalize adds to history
        crate::error_recovery::finalize_batch(&env, batch_id).unwrap();

        let history = crate::error_recovery::get_recovery_history(&env);
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().batch_id, batch_id);
    });
}

#[test]
fn test_recovery_history_max_size() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        // Create and finalize more than MAX_HISTORY batches
        for _ in 0..55 {
            let batch_id = crate::error_recovery::store_batch_state(
                &env,
                program_id.clone(),
                recipients.clone(),
                amounts.clone(),
                10000i128,
                authorized.clone(),
            ).unwrap();

            crate::error_recovery::mark_item_success(&env, batch_id, 0).unwrap();
            crate::error_recovery::finalize_batch(&env, batch_id).unwrap();
        }

        // History should be capped at MAX_HISTORY (50)
        let history = crate::error_recovery::get_recovery_history(&env);
        assert_eq!(history.len(), 50);
    });
}

// ─────────────────────────────────────────────────────────
// 42. Increment Retry Count Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_increment_retry_count_success() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        crate::error_recovery::mark_item_failed(&env, batch_id, 0, 500).unwrap();

        // Increment retry count
        let result = crate::error_recovery::increment_retry_count(&env, batch_id, 0);
        assert!(result.is_ok());

        let state = crate::error_recovery::get_batch_state(&env, batch_id).unwrap();
        let item = state.items.get(0).unwrap();
        assert_eq!(item.retry_count, 1);
        assert_eq!(item.status, crate::error_recovery::BatchItemStatus::Pending);
    });
}

#[test]
fn test_increment_retry_count_exceeds_max() {
    let (env, contract_id) = setup_env();
    let authorized = Address::generate(&env);
    let recipient = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Set low max retries
        let config = crate::error_recovery::BatchRecoveryConfig {
            max_retries: 2,
            ..crate::error_recovery::BatchRecoveryConfig::default()
        };
        let _ = crate::error_recovery::set_batch_recovery_config(&env, config);

        let program_id = String::from_str(&env, "TestProgram");
        let recipients = soroban_sdk::vec![&env, recipient];
        let amounts = soroban_sdk::vec![&env, 1000i128];

        let batch_id = crate::error_recovery::store_batch_state(
            &env,
            program_id,
            recipients,
            amounts,
            10000i128,
            authorized,
        ).unwrap();

        crate::error_recovery::mark_item_failed(&env, batch_id, 0, 500).unwrap();

        // Increment retry count twice (should succeed)
        crate::error_recovery::increment_retry_count(&env, batch_id, 0).unwrap();
        crate::error_recovery::increment_retry_count(&env, batch_id, 0).unwrap();

        // Third increment should fail
        let result = crate::error_recovery::increment_retry_count(&env, batch_id, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::error_recovery::ERR_BATCH_NOT_RECOVERABLE);
    });
}
