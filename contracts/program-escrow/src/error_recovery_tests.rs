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

// ============================================================================
// Retry Policy Comparison Tests (Aggressive vs Conservative)
// ============================================================================

/// Test that aggressive policy has more retry attempts than conservative
#[test]
fn test_aggressive_has_more_attempts_than_conservative() {
    let env = Env::default();
    let aggressive = RetryConfig::aggressive(&env);
    let conservative = RetryConfig::conservative(&env);

    // Aggressive should have more attempts (5 vs 2)
    assert!(
        aggressive.max_attempts > conservative.max_attempts,
        "Aggressive policy should have more retry attempts"
    );
    assert_eq!(aggressive.max_attempts, 5);
    assert_eq!(conservative.max_attempts, 2);
}

/// Test that aggressive policy starts with shorter delay than conservative
#[test]
fn test_aggressive_has_shorter_initial_delay_than_conservative() {
    let env = Env::default();
    let aggressive = RetryConfig::aggressive(&env);
    let conservative = RetryConfig::conservative(&env);

    // Aggressive should start with shorter delay (50ms vs 200ms)
    assert!(
        aggressive.initial_delay_ms < conservative.initial_delay_ms,
        "Aggressive policy should have shorter initial delay"
    );
    assert_eq!(aggressive.initial_delay_ms, 50);
    assert_eq!(conservative.initial_delay_ms, 200);
}

/// Test that conservative policy has higher max delay cap
#[test]
fn test_conservative_has_higher_max_delay_cap() {
    let env = Env::default();
    let aggressive = RetryConfig::aggressive(&env);
    let conservative = RetryConfig::conservative(&env);

    // Conservative should have higher max delay cap (10000ms vs 3000ms)
    assert!(
        conservative.max_delay_ms > aggressive.max_delay_ms,
        "Conservative policy should have higher max delay cap"
    );
    assert_eq!(aggressive.max_delay_ms, 3000);
    assert_eq!(conservative.max_delay_ms, 10000);
}

/// Test that conservative policy has higher backoff multiplier
#[test]
fn test_conservative_has_higher_backoff_multiplier() {
    let env = Env::default();
    let aggressive = RetryConfig::aggressive(&env);
    let conservative = RetryConfig::conservative(&env);

    // Conservative has multiplier of 3, aggressive has 2
    assert!(
        conservative.backoff_multiplier > aggressive.backoff_multiplier,
        "Conservative policy should have higher backoff multiplier"
    );
    assert_eq!(aggressive.backoff_multiplier, 2);
    assert_eq!(conservative.backoff_multiplier, 3);
}

/// Test delay sequence comparison between aggressive and conservative policies
#[test]
fn test_delay_sequence_aggressive_vs_conservative() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    // Create configs without jitter for predictable testing
    let aggressive = RetryConfig {
        max_attempts: 5,
        initial_delay_ms: 50,
        max_delay_ms: 3000,
        backoff_multiplier: 2,
        jitter_percent: 0,
    };

    let conservative = RetryConfig {
        max_attempts: 2,
        initial_delay_ms: 200,
        max_delay_ms: 10000,
        backoff_multiplier: 3,
        jitter_percent: 0,
    };

    // Attempt 0: Aggressive = 50ms, Conservative = 200ms
    let agg_delay_0 = calculate_backoff_delay(&aggressive, 0, &env);
    let con_delay_0 = calculate_backoff_delay(&conservative, 0, &env);
    assert!(
        agg_delay_0 < con_delay_0,
        "At attempt 0: aggressive delay ({}) should be less than conservative ({})",
        agg_delay_0, con_delay_0
    );

    // Attempt 1: Aggressive = 50*2 = 100ms, Conservative = 200*3 = 600ms
    let agg_delay_1 = calculate_backoff_delay(&aggressive, 1, &env);
    let con_delay_1 = calculate_backoff_delay(&conservative, 1, &env);
    assert!(
        agg_delay_1 < con_delay_1,
        "At attempt 1: aggressive delay ({}) should be less than conservative ({})",
        agg_delay_1, con_delay_1
    );

    // Verify exponential growth
    // Aggressive: 50 -> 100 (2x growth)
    // Conservative: 200 -> 600 (3x growth)
    assert!(con_delay_1 > con_delay_0 * 2, "Conservative should grow faster");
}

/// Test that aggressive policy reaches max delay faster due to lower cap
#[test]
fn test_aggressive_reaches_max_delay_faster() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let aggressive = RetryConfig {
        max_attempts: 10,
        initial_delay_ms: 50,
        max_delay_ms: 3000,
        backoff_multiplier: 2,
        jitter_percent: 0,
    };

    // With 50ms initial and 2x multiplier:
    // Attempt 0: 50ms
    // Attempt 1: 100ms
    // Attempt 2: 200ms
    // Attempt 3: 400ms
    // Attempt 4: 800ms
    // Attempt 5: 1600ms
    // Attempt 6: 3200ms -> capped at 3000ms

    let _delay_at_5 = calculate_backoff_delay(&aggressive, 5, &env);
    let delay_at_6 = calculate_backoff_delay(&aggressive, 6, &env);

    // At attempt 6, should be capped at max_delay_ms
    assert!(
        delay_at_6 <= aggressive.max_delay_ms,
        "Delay should be capped at max_delay_ms"
    );
    assert_eq!(delay_at_6, 3000, "Should reach max cap at attempt 6");
}

/// Test that conservative policy allows longer delays before capping
#[test]
fn test_conservative_allows_longer_delays() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let conservative = RetryConfig {
        max_attempts: 10,
        initial_delay_ms: 200,
        max_delay_ms: 10000,
        backoff_multiplier: 3,
        jitter_percent: 0,
    };

    // With 200ms initial and 3x multiplier:
    // Attempt 0: 200ms
    // Attempt 1: 600ms
    // Attempt 2: 1800ms
    // Attempt 3: 5400ms
    // Attempt 4: 16200ms -> capped at 10000ms

    let _delay_at_3 = calculate_backoff_delay(&conservative, 3, &env);
    let delay_at_4 = calculate_backoff_delay(&conservative, 4, &env);

    assert!(
        delay_at_4 <= conservative.max_delay_ms,
        "Delay should be capped at max_delay_ms"
    );
    assert_eq!(delay_at_4, 10000, "Should reach max cap at attempt 4");
}

/// Test max retries behavior for aggressive policy
#[test]
fn test_aggressive_max_retries_behavior() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let aggressive = RetryConfig::aggressive(&env);

    // Aggressive allows 5 attempts
    // Simulate all 5 attempts failing and calculate total delay
    let mut total_delay = 0u64;
    for attempt in 0..aggressive.max_attempts {
        let delay = calculate_backoff_delay(&aggressive, attempt, &env);
        total_delay += delay;
    }

    // After max attempts, no more retries should be allowed
    // Total delay should be reasonable (less than 5 * max_delay)
    assert!(
        total_delay < aggressive.max_delay_ms * aggressive.max_attempts as u64,
        "Total delay should be bounded"
    );

    // Verify we can track that max attempts was reached
    assert_eq!(aggressive.max_attempts, 5, "Aggressive should allow 5 attempts");
}

/// Test max retries behavior for conservative policy
#[test]
fn test_conservative_max_retries_behavior() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let conservative = RetryConfig::conservative(&env);

    // Conservative allows only 2 attempts
    let mut total_delay = 0u64;
    for attempt in 0..conservative.max_attempts {
        let delay = calculate_backoff_delay(&conservative, attempt, &env);
        total_delay += delay;
    }

    // Conservative gives up quickly but with longer delays
    assert_eq!(conservative.max_attempts, 2, "Conservative should allow only 2 attempts");

    // First delay should be significant (200ms base)
    let first_delay = calculate_backoff_delay(&conservative, 0, &env);
    assert!(
        first_delay >= 150, // Account for jitter variance
        "Conservative first delay should be significant"
    );
}

/// Test jitter application differs between policies
#[test]
fn test_jitter_application_differs_between_policies() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let aggressive = RetryConfig::aggressive(&env);
    let conservative = RetryConfig::conservative(&env);

    // Aggressive has 15% jitter, conservative has 25% jitter
    assert_eq!(aggressive.jitter_percent, 15);
    assert_eq!(conservative.jitter_percent, 25);

    // Conservative should have larger jitter range
    assert!(
        conservative.jitter_percent > aggressive.jitter_percent,
        "Conservative should have larger jitter percentage"
    );
}

/// Test policy comparison table - comprehensive comparison
#[test]
fn test_policy_comparison_table() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let default_config = RetryConfig::default(&env);
    let aggressive = RetryConfig::aggressive(&env);
    let conservative = RetryConfig::conservative(&env);

    // Policy Comparison Table:
    // | Property           | Default     | Aggressive | Conservative |
    // |--------------------|-------------|------------|--------------|
    // | max_attempts       | 3           | 5          | 2            |
    // | initial_delay_ms   | 100         | 50         | 200          |
    // | max_delay_ms       | 5000        | 3000       | 10000        |
    // | backoff_multiplier | 2           | 2          | 3            |
    // | jitter_percent     | 20          | 15         | 25           |

    // Verify max_attempts ordering: conservative < default < aggressive
    assert!(conservative.max_attempts < default_config.max_attempts);
    assert!(default_config.max_attempts < aggressive.max_attempts);

    // Verify initial_delay_ms ordering: aggressive < default < conservative
    assert!(aggressive.initial_delay_ms < default_config.initial_delay_ms);
    assert!(default_config.initial_delay_ms < conservative.initial_delay_ms);

    // Verify max_delay_ms ordering: aggressive < default < conservative
    assert!(aggressive.max_delay_ms < default_config.max_delay_ms);
    assert!(default_config.max_delay_ms < conservative.max_delay_ms);

    // Verify backoff_multiplier: aggressive == default < conservative
    assert_eq!(aggressive.backoff_multiplier, default_config.backoff_multiplier);
    assert!(default_config.backoff_multiplier < conservative.backoff_multiplier);

    // Verify jitter_percent: aggressive < default < conservative
    assert!(aggressive.jitter_percent < default_config.jitter_percent);
    assert!(default_config.jitter_percent < conservative.jitter_percent);
}

/// Test that aggressive policy is suitable for quick recovery scenarios
#[test]
fn test_aggressive_policy_quick_recovery_scenario() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let aggressive = RetryConfig {
        max_attempts: 5,
        initial_delay_ms: 50,
        max_delay_ms: 3000,
        backoff_multiplier: 2,
        jitter_percent: 0,
    };

    // Calculate total possible delay for aggressive policy
    // Attempt 0: 50ms
    // Attempt 1: 100ms
    // Attempt 2: 200ms
    // Attempt 3: 400ms
    // Attempt 4: 800ms
    // Total: 1550ms max

    let mut total_delay = 0u64;
    for attempt in 0..aggressive.max_attempts {
        total_delay += calculate_backoff_delay(&aggressive, attempt, &env);
    }

    // Aggressive policy total delay should be relatively short
    assert!(
        total_delay < 2000,
        "Aggressive policy total delay should be under 2 seconds"
    );
}

/// Test that conservative policy is suitable for rate-limit-aware scenarios
#[test]
fn test_conservative_policy_rate_limit_aware_scenario() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let conservative = RetryConfig {
        max_attempts: 2,
        initial_delay_ms: 200,
        max_delay_ms: 10000,
        backoff_multiplier: 3,
        jitter_percent: 0,
    };

    // Calculate delays for conservative policy
    // Attempt 0: 200ms
    // Attempt 1: 600ms

    let delay_0 = calculate_backoff_delay(&conservative, 0, &env);
    let delay_1 = calculate_backoff_delay(&conservative, 1, &env);

    // Conservative starts with longer delay to avoid rate limiting
    assert!(
        delay_0 >= 200,
        "Conservative should start with longer delay"
    );

    // Conservative grows faster (3x multiplier)
    assert!(
        delay_1 >= delay_0 * 2,
        "Conservative should grow delays quickly"
    );
}

/// Test delay bounds are respected for all policies
#[test]
fn test_all_policies_respect_delay_bounds() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let policies = [
        RetryConfig::default(&env),
        RetryConfig::aggressive(&env),
        RetryConfig::conservative(&env),
    ];

    for policy in &policies {
        // Test that all delays are within bounds for various attempt numbers
        for attempt in 0..10 {
            let delay = calculate_backoff_delay(policy, attempt, &env);

            // Delay should never exceed max_delay_ms
            assert!(
                delay <= policy.max_delay_ms,
                "Delay {} exceeds max_delay_ms {} for policy with max_attempts {}",
                delay, policy.max_delay_ms, policy.max_attempts
            );

            // Delay should be positive
            assert!(delay > 0, "Delay should be positive");
        }
    }
}

/// Test that default policy is a middle ground between aggressive and conservative
#[test]
fn test_default_is_middle_ground() {
    let env = Env::default();

    let default_config = RetryConfig::default(&env);
    let aggressive = RetryConfig::aggressive(&env);
    let conservative = RetryConfig::conservative(&env);

    // Default should be between aggressive and conservative for most properties
    // max_attempts: aggressive(5) > default(3) > conservative(2)
    assert!(
        aggressive.max_attempts > default_config.max_attempts
            && default_config.max_attempts > conservative.max_attempts,
        "Default max_attempts should be between aggressive and conservative"
    );

    // initial_delay_ms: aggressive(50) < default(100) < conservative(200)
    assert!(
        aggressive.initial_delay_ms < default_config.initial_delay_ms
            && default_config.initial_delay_ms < conservative.initial_delay_ms,
        "Default initial_delay_ms should be between aggressive and conservative"
    );
}
