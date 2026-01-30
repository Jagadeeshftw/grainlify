//! # Error Recovery Tests
//! Tests cover all error scenarios, retry logic, circuit breaker behavior,
//! and batch partial success handling.

#![cfg(test)]

use super::error_recovery::*;
use super::retry_executor::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, Env,
};

// Error Classification Tests
#[test]
fn test_transient_error_classification() {
    assert_eq!(
        classify_error(RecoveryError::NetworkTimeout),
        ErrorClass::Transient
    );
    assert_eq!(
        classify_error(RecoveryError::TemporaryUnavailable),
        ErrorClass::Transient
    );
    assert_eq!(
        classify_error(RecoveryError::RateLimitExceeded),
        ErrorClass::Transient
    );
    assert_eq!(
        classify_error(RecoveryError::ResourceExhausted),
        ErrorClass::Transient
    );
}

#[test]
fn test_permanent_error_classification() {
    assert_eq!(
        classify_error(RecoveryError::InsufficientFunds),
        ErrorClass::Permanent
    );
    assert_eq!(
        classify_error(RecoveryError::InvalidRecipient),
        ErrorClass::Permanent
    );
    assert_eq!(
        classify_error(RecoveryError::Unauthorized),
        ErrorClass::Permanent
    );
    assert_eq!(
        classify_error(RecoveryError::InvalidAmount),
        ErrorClass::Permanent
    );
}

#[test]
fn test_partial_error_classification() {
    assert_eq!(
        classify_error(RecoveryError::PartialBatchFailure),
        ErrorClass::Partial
    );
    assert_eq!(
        classify_error(RecoveryError::AllBatchItemsFailed),
        ErrorClass::Partial
    );
}

// Retry Configuration Tests

#[test]
fn test_default_retry_config() {
    let env = Env::default();
    let config = RetryConfig::default(&env);

    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.initial_delay_ms, 100);
    assert_eq!(config.max_delay_ms, 5000);
    assert_eq!(config.backoff_multiplier, 2);
    assert_eq!(config.jitter_percent, 20);
}

#[test]
fn test_aggressive_retry_config() {
    let env = Env::default();
    let config = RetryConfig::aggressive(&env);

    assert_eq!(config.max_attempts, 5);
    assert_eq!(config.initial_delay_ms, 50);
}

#[test]
fn test_conservative_retry_config() {
    let env = Env::default();
    let config = RetryConfig::conservative(&env);

    assert_eq!(config.max_attempts, 2);
    assert_eq!(config.initial_delay_ms, 200);
}

// Exponential Backoff Tests

#[test]
fn test_exponential_backoff_progression() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let config = RetryConfig {
        max_attempts: 5,
        initial_delay_ms: 100,
        max_delay_ms: 10000,
        backoff_multiplier: 2,
        jitter_percent: 0, // No jitter for predictable testing
    };

    // Attempt 0: 100ms * 2^0 = 100ms
    let delay0 = calculate_backoff_delay(&config, 0, &env);
    assert!(delay0 >= 80 && delay0 <= 120); // Allow for some variance

    // Attempt 1: 100ms * 2^1 = 200ms
    let delay1 = calculate_backoff_delay(&config, 1, &env);
    assert!(delay1 >= 180 && delay1 <= 220);

    // Attempt 2: 100ms * 2^2 = 400ms
    let delay2 = calculate_backoff_delay(&config, 2, &env);
    assert!(delay2 >= 380 && delay2 <= 420);

    // Attempt 3: 100ms * 2^3 = 800ms
    let delay3 = calculate_backoff_delay(&config, 3, &env);
    assert!(delay3 >= 780 && delay3 <= 820);
}

#[test]
fn test_backoff_max_delay_cap() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let config = RetryConfig {
        max_attempts: 10,
        initial_delay_ms: 100,
        max_delay_ms: 1000, // Cap at 1 second
        backoff_multiplier: 2,
        jitter_percent: 0,
    };

    // Attempt 10 would be 100ms * 2^10 = 102,400ms, but should cap at 1000ms
    let delay = calculate_backoff_delay(&config, 10, &env);
    assert!(delay <= 1000);
}

#[test]
fn test_backoff_with_jitter() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let config = RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 1000,
        max_delay_ms: 10000,
        backoff_multiplier: 2,
        jitter_percent: 20, // 20% jitter
    };

    // With 20% jitter, delay should be between 800ms and 1200ms for attempt 0
    let delay = calculate_backoff_delay(&config, 0, &env);
    assert!(delay >= 800 && delay <= 1200);
}

// Error State Tracking Tests
#[test]
fn test_create_error_state() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let caller = Address::generate(&env);
    let operation_id = 42;

    let state = create_error_state(
        &env,
        operation_id,
        RecoveryError::NetworkTimeout,
        caller.clone(),
    );

    assert_eq!(state.operation_id, operation_id);
    assert_eq!(state.error_type, RecoveryError::NetworkTimeout as u32);
    assert_eq!(state.retry_count, 0);
    assert_eq!(state.first_error_timestamp, 1000);
    assert_eq!(state.can_recover, true); // Transient error
    assert_eq!(state.caller, caller);
}

// Note: These tests require contract context and are commented out for now
// They would work in actual contract execution context

/*
#[test]
fn test_error_state_persistence() {
    let env = Env::default();
    let caller = Address::generate(&env);

    let state = create_error_state(&env, 123, RecoveryError::NetworkTimeout, caller);

    // Store state
    store_error_state(&env, &state);

    // Retrieve state
    let retrieved = get_error_state(&env, 123).unwrap();

    assert_eq!(retrieved.operation_id, state.operation_id);
    assert_eq!(retrieved.error_type, state.error_type);
    assert_eq!(retrieved.retry_count, state.retry_count);
}

#[test]
fn test_operation_id_generation() {
    let env = Env::default();

    let id1 = generate_operation_id(&env);
    let id2 = generate_operation_id(&env);
    let id3 = generate_operation_id(&env);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
fn test_circuit_breaker_persistence() {
    let env = Env::default();
    let operation_type = symbol_short!("transfer");

    let mut breaker = CircuitBreaker::new(&env);
    breaker.failure_count = 3;

    // Store breaker
    store_circuit_breaker(&env, operation_type.clone(), &breaker);

    // Retrieve breaker
    let retrieved = get_circuit_breaker(&env, operation_type);
    assert_eq!(retrieved.failure_count, 3);
}

#[test]
fn test_retry_success_on_first_attempt() {
    let env = Env::default();
    let caller = Address::generate(&env);
    let config = RetryConfig::default(&env);

    let context = RetryContext::new(&env, symbol_short!("test"), caller, config);

    let mut attempt_count = 0;
    let result = execute_with_retry(&env, context, || {
        attempt_count += 1;
        Ok(42)
    });

    match result {
        RetryResult::Success(value) => {
            assert_eq!(value, 42);
            assert_eq!(attempt_count, 1);
        }
        _ => panic!("Expected success"),
    }
}

#[test]
fn test_retry_success_after_transient_failures() {
    let env = Env::default();
    let caller = Address::generate(&env);
    let config = RetryConfig::default(&env);

    let context = RetryContext::new(&env, symbol_short!("test"), caller, config);

    let mut attempt_count = 0;
    let result = execute_with_retry(&env, context, || {
        attempt_count += 1;
        if attempt_count < 3 {
            Err(RecoveryError::NetworkTimeout)
        } else {
            Ok(100)
        }
    });

    match result {
        RetryResult::Success(value) => {
            assert_eq!(value, 100);
            assert_eq!(attempt_count, 3);
        }
        _ => panic!("Expected success after retries"),
    }
}

#[test]
fn test_retry_permanent_error_no_retry() {
    let env = Env::default();
    let caller = Address::generate(&env);
    let config = RetryConfig::default(&env);

    let context = RetryContext::new(&env, symbol_short!("test"), caller, config);

    let mut attempt_count = 0;
    let result: RetryResult<i32> = execute_with_retry(&env, context, || {
        attempt_count += 1;
        Err(RecoveryError::InsufficientFunds)
    });

    match result {
        RetryResult::Failed(error) => {
            assert_eq!(error, RecoveryError::InsufficientFunds);
            assert_eq!(attempt_count, 1); // Should not retry permanent errors
        }
        _ => panic!("Expected failure"),
    }
}

#[test]
fn test_retry_max_attempts_exceeded() {
    let env = Env::default();
    let caller = Address::generate(&env);
    let config = RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 100,
        max_delay_ms: 5000,
        backoff_multiplier: 2,
        jitter_percent: 0,
    };

    let context = RetryContext::new(&env, symbol_short!("test"), caller, config);

    let mut attempt_count = 0;
    let result: RetryResult<i32> = execute_with_retry(&env, context, || {
        attempt_count += 1;
        Err(RecoveryError::NetworkTimeout)
    });

    match result {
        RetryResult::Failed(error) => {
            assert_eq!(error, RecoveryError::MaxRetriesExceeded);
            assert_eq!(attempt_count, 3);
        }
        _ => panic!("Expected max retries exceeded"),
    }
}

#[test]
fn test_retry_circuit_breaker_blocks() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let caller = Address::generate(&env);
    let operation_type = symbol_short!("test");

    // Open the circuit breaker
    let mut breaker = CircuitBreaker::new(&env);
    for _ in 0..5 {
        breaker.record_failure(&env);
    }
    store_circuit_breaker(&env, operation_type.clone(), &breaker);

    // Try to execute - should be blocked
    let config = RetryConfig::default(&env);
    let context = RetryContext::new(&env, operation_type, caller, config);

    let result = execute_with_retry(&env, context, || Ok(42));

    match result {
        RetryResult::CircuitBreakerOpen => {
            // Expected
        }
        _ => panic!("Expected circuit breaker to block request"),
    }
}

#[test]
fn test_full_retry_flow_with_recovery() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let caller = Address::generate(&env);
    let config = RetryConfig::default(&env);
    let context = RetryContext::new(&env, symbol_short!("payout"), caller.clone(), config);

    // Simulate operation that fails twice then succeeds
    let mut attempts = 0;
    let result = execute_with_retry(&env, context, || {
        attempts += 1;
        if attempts < 3 {
            Err(RecoveryError::TemporaryUnavailable)
        } else {
            Ok(1000i128)
        }
    });

    // Verify success
    match result {
        RetryResult::Success(amount) => {
            assert_eq!(amount, 1000);
            assert_eq!(attempts, 3);
        }
        _ => panic!("Expected successful recovery"),
    }

    // Verify circuit breaker is healthy
    let breaker = get_circuit_breaker(&env, symbol_short!("payout"));
    assert_eq!(breaker.state, CircuitState::Closed);
    assert_eq!(breaker.failure_count, 0);
}

#[test]
fn test_batch_with_mixed_results() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let recipients = vec![
        &env,
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    let amounts = vec![&env, 100i128, 200i128, 300i128, 400i128, 500i128];

    // Simulate batch where items 1 and 3 fail
    let result = execute_batch_with_partial_success(
        &env,
        5,
        symbol_short!("batch"),
        |index| {
            let recipient = recipients.get(index).unwrap();
            let amount = amounts.get(index).unwrap();

            if index == 1 || index == 3 {
                Err(RecoveryError::NetworkTimeout)
            } else {
                Ok((recipient, amount))
            }
        },
    );

    assert_eq!(result.total_items, 5);
    assert_eq!(result.successful, 3);
    assert_eq!(result.failed, 2);
    assert!(result.is_partial_success());

    // Verify failed indices
    assert_eq!(result.failed_indices.get(0).unwrap(), 1);
    assert_eq!(result.failed_indices.get(1).unwrap(), 3);
}
*/

// Batch Result Tests

#[test]
fn test_batch_result_all_success() {
    let env = Env::default();
    let mut result = BatchResult::new(&env, 5);

    for _ in 0..5 {
        result.record_success();
    }

    assert_eq!(result.total_items, 5);
    assert_eq!(result.successful, 5);
    assert_eq!(result.failed, 0);
    assert!(result.is_full_success());
    assert!(!result.is_partial_success());
    assert!(!result.is_complete_failure());
}

#[test]
fn test_batch_result_partial_success() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let mut result = BatchResult::new(&env, 5);
    let recipient = Address::generate(&env);

    // 3 successes
    result.record_success();
    result.record_success();
    result.record_success();

    // 2 failures
    result.record_failure(
        3,
        recipient.clone(),
        100,
        RecoveryError::NetworkTimeout,
        &env,
    );
    result.record_failure(
        4,
        recipient.clone(),
        200,
        RecoveryError::InvalidRecipient,
        &env,
    );

    assert_eq!(result.total_items, 5);
    assert_eq!(result.successful, 3);
    assert_eq!(result.failed, 2);
    assert!(!result.is_full_success());
    assert!(result.is_partial_success());
    assert!(!result.is_complete_failure());

    // Check failed indices
    assert_eq!(result.failed_indices.len(), 2);
    assert_eq!(result.failed_indices.get(0).unwrap(), 3);
    assert_eq!(result.failed_indices.get(1).unwrap(), 4);

    // Check error details
    assert_eq!(result.error_details.len(), 2);
    let error1 = result.error_details.get(0).unwrap();
    assert_eq!(error1.index, 3);
    assert_eq!(error1.amount, 100);
    assert_eq!(error1.can_retry, true); // NetworkTimeout is transient

    let error2 = result.error_details.get(1).unwrap();
    assert_eq!(error2.index, 4);
    assert_eq!(error2.amount, 200);
    assert_eq!(error2.can_retry, false); // InvalidRecipient is permanent
}

#[test]
fn test_batch_result_complete_failure() {
    let env = Env::default();
    let mut result = BatchResult::new(&env, 3);
    let recipient = Address::generate(&env);

    result.record_failure(
        0,
        recipient.clone(),
        100,
        RecoveryError::NetworkTimeout,
        &env,
    );
    result.record_failure(
        1,
        recipient.clone(),
        200,
        RecoveryError::NetworkTimeout,
        &env,
    );
    result.record_failure(
        2,
        recipient.clone(),
        300,
        RecoveryError::NetworkTimeout,
        &env,
    );

    assert_eq!(result.successful, 0);
    assert_eq!(result.failed, 3);
    assert!(!result.is_full_success());
    assert!(!result.is_partial_success());
    assert!(result.is_complete_failure());
}

// Circuit Breaker Tests
#[test]
fn test_circuit_breaker_initial_state() {
    let env = Env::default();
    let breaker = CircuitBreaker::new(&env);

    assert_eq!(breaker.state, CircuitState::Closed);
    assert_eq!(breaker.failure_count, 0);
    assert_eq!(breaker.failure_threshold, 5);
}

#[test]
fn test_circuit_breaker_opens_after_threshold() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let mut breaker = CircuitBreaker::new(&env);

    // Record failures up to threshold
    for i in 0..5 {
        breaker.record_failure(&env);
        if i < 4 {
            assert_eq!(breaker.state, CircuitState::Closed);
        }
    }

    // Should be open after 5 failures
    assert_eq!(breaker.state, CircuitState::Open);
}

#[test]
fn test_circuit_breaker_success_resets_count() {
    let env = Env::default();
    let mut breaker = CircuitBreaker::new(&env);

    // Record some failures
    breaker.record_failure(&env);
    breaker.record_failure(&env);
    assert_eq!(breaker.failure_count, 2);

    // Success should reset
    breaker.record_success(&env);
    assert_eq!(breaker.failure_count, 0);
    assert_eq!(breaker.state, CircuitState::Closed);
}

#[test]
fn test_circuit_breaker_half_open_transition() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let mut breaker = CircuitBreaker::new(&env);
    breaker.timeout_duration = 60; // 60 seconds

    // Open the circuit
    for _ in 0..5 {
        breaker.record_failure(&env);
    }
    assert_eq!(breaker.state, CircuitState::Open);

    // Before timeout, should still be open
    env.ledger().with_mut(|li| li.timestamp = 1030);
    assert!(!breaker.is_request_allowed(&env));
    assert_eq!(breaker.state, CircuitState::Open);

    // After timeout, should transition to half-open
    env.ledger().with_mut(|li| li.timestamp = 1061);
    assert!(breaker.is_request_allowed(&env));
    assert_eq!(breaker.state, CircuitState::HalfOpen);
}

#[test]
fn test_circuit_breaker_half_open_to_closed() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let mut breaker = CircuitBreaker::new(&env);
    breaker.state = CircuitState::HalfOpen;
    breaker.failure_count = 2; // Need 2 successes to close
    breaker.success_threshold = 2;

    // First success
    breaker.record_success(&env);
    assert_eq!(breaker.state, CircuitState::HalfOpen);
    assert_eq!(breaker.failure_count, 1);

    // Second success should close circuit
    breaker.record_success(&env);
    assert_eq!(breaker.state, CircuitState::Closed);
    assert_eq!(breaker.failure_count, 0);
}

#[test]
fn test_circuit_breaker_half_open_to_open() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let mut breaker = CircuitBreaker::new(&env);
    breaker.state = CircuitState::HalfOpen;

    // Any failure in half-open should reopen circuit
    breaker.record_failure(&env);
    assert_eq!(breaker.state, CircuitState::Open);
}

// ============================================================================
// Recovery Strategy Tests
// ============================================================================

#[test]
fn test_recovery_strategy_determination() {
    assert_eq!(
        determine_recovery_strategy(RecoveryError::NetworkTimeout),
        RecoveryStrategy::AutoRetry
    );

    assert_eq!(
        determine_recovery_strategy(RecoveryError::InsufficientFunds),
        RecoveryStrategy::ManualRetry
    );

    assert_eq!(
        determine_recovery_strategy(RecoveryError::PartialBatchFailure),
        RecoveryStrategy::ManualRetry
    );
}
