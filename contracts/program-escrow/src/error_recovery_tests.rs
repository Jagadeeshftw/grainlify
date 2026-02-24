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

// #[test]
// fn test_half_open_can_be_reset_again_after_reopen() {
//     let (env, admin, contract_id) = setup_with_admin(2);
//     simulate_failures(&env, &contract_id, 2);
//     env.as_contract(&contract_id, || {
//         reset_circuit_breaker(&env, &admin);
//         let prog = String::from_str(&env, "TestProg");
//         record_failure(&env, prog, symbol_short!("op"), ERR_TRANSFER_FAILED);
//         assert_eq!(get_state(&env), CircuitState::Open);
//         reset_circuit_breaker(&env, &admin);
//         assert_eq!(get_state(&env), CircuitState::HalfOpen);
//         record_success(&env);
//         assert_eq!(get_state(&env), CircuitState::Closed);
//     });
// }

// ─────────────────────────────────────────────────────────
// 8. Hard reset: HalfOpen / Closed → Closed
// ─────────────────────────────────────────────────────────

// #[test]
// fn test_reset_half_open_goes_to_closed() {
//     let (env, admin, contract_id) = setup_with_admin(2);
//     simulate_failures(&env, &contract_id, 2);
//     env.as_contract(&contract_id, || {
//         reset_circuit_breaker(&env, &admin); // Open → HalfOpen
//         reset_circuit_breaker(&env, &admin); // HalfOpen → Closed
//         assert_eq!(get_state(&env), CircuitState::Closed);
//         assert_eq!(get_failure_count(&env), 0);
//     });
// }

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
        let retry_cfg = RetryConfig { max_attempts: 3 };
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Err(ERR_TRANSFER_FAILED));
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 3);
        assert_eq!(result.final_error, ERR_TRANSFER_FAILED);
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
        let retry_cfg = RetryConfig { max_attempts: 5 };
        let result = execute_with_retry(&env, &retry_cfg, prog, op, || Ok(()));
        assert!(!result.succeeded);
        assert_eq!(result.attempts, 0);
        assert_eq!(result.final_error, ERR_CIRCUIT_OPEN);
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
        let retry_cfg = RetryConfig { max_attempts: 3 };
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

// #[test]
// fn test_full_circuit_breaker_lifecycle() {
//     let (env, contract_id) = setup_env();
//     let admin = Address::generate(&env);
//     env.as_contract(&contract_id, || {
//         set_circuit_admin(&env, admin.clone(), None);
//         set_config(
//             &env,
//             CircuitBreakerConfig {
//                 failure_threshold: 3,
//                 success_threshold: 1,
//                 max_error_log: 10,
//             },
//         );
//     });

//     env.as_contract(&contract_id, || {
//         // Phase 1: Normal operation
//         assert_eq!(get_state(&env), CircuitState::Closed);
//         assert!(check_and_allow(&env).is_ok());
//         record_success(&env);
//         assert_eq!(get_failure_count(&env), 0);
//     });

//     simulate_failures(&env, &contract_id, 2);

//     env.as_contract(&contract_id, || {
//         // Phase 2: Partial failures
//         assert_eq!(get_state(&env), CircuitState::Closed);
//         assert_eq!(get_failure_count(&env), 2);
//         assert!(check_and_allow(&env).is_ok());
//     });

//     simulate_failures(&env, &contract_id, 1);

//     env.as_contract(&contract_id, || {
//         // Phase 3: Threshold hit
//         assert_eq!(get_state(&env), CircuitState::Open);
//         assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));

//         // Phase 4: Admin resets
//         env.ledger().set_timestamp(2000);
//         reset_circuit_breaker(&env, &admin);
//         assert_eq!(get_state(&env), CircuitState::HalfOpen);
//         assert!(check_and_allow(&env).is_ok());

//         // Phase 5: Failure in HalfOpen
//         let prog = String::from_str(&env, "TestProg");
//         record_failure(&env, prog.clone(), symbol_short!("op"), ERR_TRANSFER_FAILED);
//         assert_eq!(get_state(&env), CircuitState::Open);
//         assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));

//         // Phase 6: Admin resets again
//         reset_circuit_breaker(&env, &admin);
//         assert_eq!(get_state(&env), CircuitState::HalfOpen);

//         // Phase 7: Success closes
//         record_success(&env);
//         assert_eq!(get_state(&env), CircuitState::Closed);
//         assert_eq!(get_failure_count(&env), 0);
//         assert!(check_and_allow(&env).is_ok());

//         // Phase 8: Error log has entries
//         let log = get_error_log(&env);
//         assert!(log.len() > 0, "Error log should contain entries from failures");
//     });
// }

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
