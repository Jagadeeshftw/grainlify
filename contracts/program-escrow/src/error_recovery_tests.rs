// contracts/program-escrow/src/error_recovery_tests.rs
//
// Consolidated tests for Error Recovery and Circuit Breaker behavior.

#![cfg(test)]

use soroban_sdk::testutils::Address as TestAddress;
use soroban_sdk::{contract, contractimpl, symbol_short, testutils::Ledger, Address, Env, String, vec};

use crate::error_recovery::*;
use crate::retry_executor::*;

#[contract]
pub struct CircuitBreakerTestContract;

#[contractimpl]
impl CircuitBreakerTestContract {}

fn setup_env() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register_contract(None, CircuitBreakerTestContract);
    (env, contract_id)
}

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
// 1. Error Classification Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_error_classification() {
    assert_eq!(classify_error(RecoveryError::NetworkTimeout), ErrorClass::Transient);
    assert_eq!(classify_error(RecoveryError::InsufficientFunds), ErrorClass::Permanent);
    assert_eq!(classify_error(RecoveryError::PartialBatchFailure), ErrorClass::Partial);
}

// ─────────────────────────────────────────────────────────
// 2. Retry Configuration Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_retry_config_presets() {
    let env = Env::default();
    let config = RetryConfig::default(&env);
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.backoff_multiplier, 2);
    
    let aggressive = RetryConfig::aggressive(&env);
    assert_eq!(aggressive.max_attempts, 5);
}

#[test]
fn test_backoff_calculation() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    
    let config = RetryConfig {
        max_attempts: 5,
        initial_delay_ms: 100,
        max_delay_ms: 1000,
        backoff_multiplier: 2,
        jitter_percent: 0, // No jitter for predictable tests
    };
    
    // Attempt 0: 100 * 2^0 = 100
    assert_eq!(calculate_backoff_delay(&config, 0, &env), 100);
    // Attempt 1: 100 * 2^1 = 200
    assert_eq!(calculate_backoff_delay(&config, 1, &env), 200);
    // Attempt 2: 100 * 2^2 = 400
    assert_eq!(calculate_backoff_delay(&config, 2, &env), 400);
}

// ─────────────────────────────────────────────────────────
// 3. Circuit Breaker State Transitions
// ─────────────────────────────────────────────────────────

#[test]
fn test_initial_state_is_closed() {
    let (env, contract_id) = setup_env();
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert!(check_and_allow(&env).is_ok());
    });
}

#[test]
fn test_circuit_opens_at_threshold() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    
    // 2 failures: still closed
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Closed);
    });
    
    // 3rd failure: opens
    simulate_failures(&env, &contract_id, 1);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));
    });
}

#[test]
fn test_success_resets_failure_count() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(get_failure_count(&env), 2);
        record_success(&env);
        assert_eq!(get_failure_count(&env), 0);
    });
}

#[test]
fn test_half_open_to_closed_on_success() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);
    
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        half_open_circuit(&env);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::Closed);
    });
}

#[test]
fn test_half_open_to_open_on_failure() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);
    
    env.as_contract(&contract_id, || {
        half_open_circuit(&env);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        
        let prog = String::from_str(&env, "Test");
        record_failure(&env, prog, symbol_short!("test"), ERR_TRANSFER_FAILED);
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

// ─────────────────────────────────────────────────────────
// 4. Admin and Reset Logic
// ─────────────────────────────────────────────────────────

#[test]
fn test_admin_reset_flow() {
    let (env, admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);
    
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::Closed);
    });
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_non_admin_cannot_reset() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    let impostor = Address::generate(&env);
    simulate_failures(&env, &contract_id, 3);
    
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &impostor);
    });
}

// ─────────────────────────────────────────────────────────
// 5. Retry Executor Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_execute_with_retry_success() {
    let (env, contract_id) = setup_env();
    let caller = Address::generate(&env);
    let config = RetryConfig::default(&env);
    let context = RetryContext::new(&env, symbol_short!("test"), caller, config);

    env.as_contract(&contract_id, || {
        let mut calls = 0;
        let result = execute_with_retry(&env, context, || {
            calls += 1;
            if calls < 2 {
                Err(RecoveryError::NetworkTimeout)
            } else {
                Ok(42)
            }
        });

        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls, 2);
    });
}

#[test]
fn test_execute_with_retry_failure_exhausted() {
    let (env, contract_id) = setup_env();
    let caller = Address::generate(&env);
    let config = RetryConfig {
        max_attempts: 2,
        ..RetryConfig::default(&env)
    };
    let context = RetryContext::new(&env, symbol_short!("test"), caller, config);

    env.as_contract(&contract_id, || {
        let result = execute_with_retry(&env, context, || {
            Err(RecoveryError::NetworkTimeout)
        });

        assert_eq!(result, Err(RecoveryError::MaxRetriesExceeded));
    });
}

#[test]
fn test_execute_with_retry_blocks_when_open() {
    let (env, _admin, contract_id) = setup_with_admin(1);
    simulate_failures(&env, &contract_id, 1);
    
    let caller = Address::generate(&env);
    let context = RetryContext::new(&env, symbol_short!("test"), caller, RetryConfig::default(&env));

    env.as_contract(&contract_id, || {
        let result = execute_with_retry(&env, context, || Ok(1));
        assert_eq!(result, Err(RecoveryError::CircuitBreakerOpen));
    });
}

// ─────────────────────────────────────────────────────────
// 6. Batch Operation Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_batch_partial_success() {
    let (env, contract_id) = setup_env();
    let recipients = vec![&env, Address::generate(&env), Address::generate(&env)];
    
    env.as_contract(&contract_id, || {
        let result = execute_batch_with_partial_success(&env, 2, symbol_short!("batch"), |i| {
            if i == 0 {
                Ok((recipients.get(i).unwrap(), 100))
            } else {
                Err(RecoveryError::NetworkTimeout)
            }
        });

        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 1);
        assert!(result.is_partial_success());
        assert_eq!(result.failed_indices.get(0).unwrap(), 1);
    });
}

#[test]
fn test_batch_all_failure() {
    let (env, contract_id) = setup_env();
    
    env.as_contract(&contract_id, || {
        let result = execute_batch_with_partial_success(&env, 2, symbol_short!("batch"), |_| {
            Err(RecoveryError::NetworkTimeout)
        });

        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 2);
        assert!(result.is_complete_failure());
    });
}
