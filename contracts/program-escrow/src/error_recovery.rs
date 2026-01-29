#![allow(dead_code)]

use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RecoveryError {
    // Transient errors (can retry)
    NetworkTimeout = 100,
    TemporaryUnavailable = 101,
    RateLimitExceeded = 102,
    ResourceExhausted = 103,

    // Permanent errors (cannot retry)
    InsufficientFunds = 200,
    InvalidRecipient = 201,
    Unauthorized = 202,
    InvalidAmount = 203,
    ProgramNotFound = 204,

    // Batch operation errors
    PartialBatchFailure = 300,
    AllBatchItemsFailed = 301,
    BatchSizeMismatch = 302,

    // Recovery state errors
    MaxRetriesExceeded = 400,
    RecoveryInProgress = 401,
    CircuitBreakerOpen = 402,
    InvalidRetryConfig = 403,
}

/// Error classification for retry decision making
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorClass {
    Transient, // Can retry
    Permanent, // Cannot retry
    Partial,   // Batch with mixed results
}

/// Classifies an error to determine if it can be retried
pub fn classify_error(error: RecoveryError) -> ErrorClass {
    match error {
        RecoveryError::NetworkTimeout
        | RecoveryError::TemporaryUnavailable
        | RecoveryError::RateLimitExceeded
        | RecoveryError::ResourceExhausted => ErrorClass::Transient,

        RecoveryError::InsufficientFunds
        | RecoveryError::InvalidRecipient
        | RecoveryError::Unauthorized
        | RecoveryError::InvalidAmount
        | RecoveryError::ProgramNotFound => ErrorClass::Permanent,

        RecoveryError::PartialBatchFailure
        | RecoveryError::AllBatchItemsFailed
        | RecoveryError::BatchSizeMismatch => ErrorClass::Partial,

        RecoveryError::MaxRetriesExceeded
        | RecoveryError::RecoveryInProgress
        | RecoveryError::CircuitBreakerOpen
        | RecoveryError::InvalidRetryConfig => ErrorClass::Permanent,
    }
}

// Retry Configuration
/// Configuration for retry behavior with exponential backoff
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryConfig {
    pub max_attempts: u32,       // Maximum retry attempts (e.g., 3-5)
    pub initial_delay_ms: u64,   // Initial delay in milliseconds (e.g., 100ms)
    pub max_delay_ms: u64,       // Maximum delay cap (e.g., 5000ms)
    pub backoff_multiplier: u32, // Multiplier for exponential backoff (e.g., 2)
    pub jitter_percent: u32,     // Jitter percentage (0-100) to prevent thundering herd
}

impl RetryConfig {
    pub fn default(_env: &Env) -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2,
            jitter_percent: 20,
        }
    }

    /// Creates an aggressive retry configuration for critical operations
    pub fn aggressive(_env: &Env) -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 50,
            max_delay_ms: 3000,
            backoff_multiplier: 2,
            jitter_percent: 15,
        }
    }

    /// Creates a conservative retry configuration
    pub fn conservative(_env: &Env) -> Self {
        Self {
            max_attempts: 2,
            initial_delay_ms: 200,
            max_delay_ms: 10000,
            backoff_multiplier: 3,
            jitter_percent: 25,
        }
    }
}

pub fn calculate_backoff_delay(config: &RetryConfig, attempt: u32, env: &Env) -> u64 {
    // Calculate base delay with exponential backoff
    let multiplier_power = config.backoff_multiplier.pow(attempt);
    let base_delay = config
        .initial_delay_ms
        .saturating_mul(multiplier_power as u64);

    // Cap at max delay
    let capped_delay = base_delay.min(config.max_delay_ms);

    // Apply jitter to prevent thundering herd
    // Jitter range: delay * (1 - jitter%) to delay * (1 + jitter%)
    let jitter_range = (capped_delay * config.jitter_percent as u64) / 100;

    // Use timestamp as pseudo-random seed for jitter
    if jitter_range > 0 {
        let timestamp = env.ledger().timestamp();
        let jitter_offset = (timestamp % (jitter_range * 2)).saturating_sub(jitter_range);
        capped_delay.saturating_add(jitter_offset)
    } else {
        capped_delay
    }
}

// Error State Tracking
/// Persistent error state for monitoring and recovery
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorState {
    pub operation_id: u64,          // Unique operation identifier
    pub error_type: u32,            // RecoveryError as u32
    pub retry_count: u32,           // Number of retry attempts made
    pub last_retry_timestamp: u64,  // Timestamp of last retry
    pub first_error_timestamp: u64, // Timestamp of first error
    pub can_recover: bool,          // Whether recovery is possible
    pub error_message: Symbol,      // Short error description
    pub caller: Address,            // Address that triggered operation
}

/// Storage key for error states
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorStateKey {
    State(u64),       // operation_id -> ErrorState
    OperationCounter, // Global counter for operation IDs
}

/// Creates a new error state
pub fn create_error_state(
    env: &Env,
    operation_id: u64,
    error: RecoveryError,
    caller: Address,
) -> ErrorState {
    let error_class = classify_error(error);
    let can_recover = matches!(error_class, ErrorClass::Transient);

    ErrorState {
        operation_id,
        error_type: error as u32,
        retry_count: 0,
        last_retry_timestamp: env.ledger().timestamp(),
        first_error_timestamp: env.ledger().timestamp(),
        can_recover,
        error_message: symbol_short!("err"),
        caller,
    }
}

/// Stores error state in persistent storage
pub fn store_error_state(env: &Env, state: &ErrorState) {
    let key = ErrorStateKey::State(state.operation_id);
    env.storage().persistent().set(&key, state);

    // Extend TTL for 7 days (approx 120960 ledgers at 5s per ledger)
    env.storage().persistent().extend_ttl(&key, 120960, 120960);
}

/// Retrieves error state from storage
pub fn get_error_state(env: &Env, operation_id: u64) -> Option<ErrorState> {
    let key = ErrorStateKey::State(operation_id);
    env.storage().persistent().get(&key)
}

/// Generates a new unique operation ID
pub fn generate_operation_id(env: &Env) -> u64 {
    let key = ErrorStateKey::OperationCounter;
    let counter: u64 = env.storage().persistent().get(&key).unwrap_or(0);
    let new_counter = counter.saturating_add(1);
    env.storage().persistent().set(&key, &new_counter);
    new_counter
}

// Batch Operation Results
/// Result of a batch operation with partial success tracking
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchResult {
    pub total_items: u32,                   // Total items in batch
    pub successful: u32,                    // Number of successful items
    pub failed: u32,                        // Number of failed items
    pub failed_indices: Vec<u32>,           // Indices of failed items
    pub error_details: Vec<BatchItemError>, // Detailed error info
}

/// Error details for a single batch item
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchItemError {
    pub index: u32,         // Index in original batch
    pub recipient: Address, // Recipient address
    pub amount: i128,       // Amount that failed
    pub error_code: u32,    // RecoveryError as u32
    pub can_retry: bool,    // Whether this item can be retried
    pub timestamp: u64,     // When error occurred
}

impl BatchResult {
    /// Creates a new empty batch result
    pub fn new(env: &Env, total_items: u32) -> Self {
        Self {
            total_items,
            successful: 0,
            failed: 0,
            failed_indices: Vec::new(env),
            error_details: Vec::new(env),
        }
    }

    /// Records a successful item
    pub fn record_success(&mut self) {
        self.successful = self.successful.saturating_add(1);
    }

    /// Records a failed item
    pub fn record_failure(
        &mut self,
        index: u32,
        recipient: Address,
        amount: i128,
        error: RecoveryError,
        env: &Env,
    ) {
        self.failed = self.failed.saturating_add(1);
        self.failed_indices.push_back(index);

        let error_class = classify_error(error);
        let can_retry = matches!(error_class, ErrorClass::Transient);

        let error_detail = BatchItemError {
            index,
            recipient,
            amount,
            error_code: error as u32,
            can_retry,
            timestamp: env.ledger().timestamp(),
        };

        self.error_details.push_back(error_detail);
    }

    /// Checks if batch was fully successful
    pub fn is_full_success(&self) -> bool {
        self.failed == 0
    }

    /// Checks if batch was partial success
    pub fn is_partial_success(&self) -> bool {
        self.successful > 0 && self.failed > 0
    }

    /// Checks if batch completely failed
    pub fn is_complete_failure(&self) -> bool {
        self.successful == 0 && self.failed > 0
    }
}

// Circuit Breaker Pattern
/// Circuit breaker state
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CircuitState {
    Closed = 0,   // Normal operation, requests allowed
    Open = 1,     // Blocking requests due to failures
    HalfOpen = 2, // Testing if service recovered
}

/// Circuit breaker configuration and state
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub failure_count: u32,
    pub failure_threshold: u32, // Failures before opening circuit
    pub success_threshold: u32, // Successes in half-open to close
    pub timeout_duration: u64,  // Seconds before trying half-open
    pub last_failure_time: u64, // Timestamp of last failure
    pub last_state_change: u64, // Timestamp of last state change
}

impl CircuitBreaker {
    /// Creates a new circuit breaker with default settings
    pub fn new(env: &Env) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold: 5, // Open after 5 failures
            success_threshold: 2, // Close after 2 successes in half-open
            timeout_duration: 60, // Try recovery after 60 seconds
            last_failure_time: 0,
            last_state_change: env.ledger().timestamp(),
        }
    }

    /// Records a successful operation
    pub fn record_success(&mut self, env: &Env) {
        match self.state {
            CircuitState::HalfOpen => {
                // In half-open, successes move toward closed
                self.failure_count = self.failure_count.saturating_sub(1);
                if self.failure_count == 0 {
                    self.state = CircuitState::Closed;
                    self.last_state_change = env.ledger().timestamp();
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count = 0;
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset if it does
                self.failure_count = 0;
            }
        }
    }

    /// Records a failed operation
    pub fn record_failure(&mut self, env: &Env) {
        let now = env.ledger().timestamp();
        self.last_failure_time = now;
        self.failure_count = self.failure_count.saturating_add(1);

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    self.last_state_change = now;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open reopens circuit
                self.state = CircuitState::Open;
                self.last_state_change = now;
            }
            CircuitState::Open => {
                // Already open, just update timestamp
            }
        }
    }

    /// Checks if operation is allowed
    pub fn is_request_allowed(&mut self, env: &Env) -> bool {
        let now = env.ledger().timestamp();

        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has passed
                let time_since_open = now.saturating_sub(self.last_state_change);
                if time_since_open >= self.timeout_duration {
                    // Try half-open state
                    self.state = CircuitState::HalfOpen;
                    self.failure_count = self.success_threshold; // Need this many successes
                    self.last_state_change = now;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
}

/// Storage key for circuit breaker
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitBreakerKey {
    State(Symbol), // operation_type -> CircuitBreaker
}

/// Gets circuit breaker for an operation type
pub fn get_circuit_breaker(env: &Env, operation_type: Symbol) -> CircuitBreaker {
    let key = CircuitBreakerKey::State(operation_type);
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| CircuitBreaker::new(env))
}

/// Stores circuit breaker state
pub fn store_circuit_breaker(env: &Env, operation_type: Symbol, breaker: &CircuitBreaker) {
    let key = CircuitBreakerKey::State(operation_type);
    env.storage().persistent().set(&key, breaker);

    // Extend TTL for 1 day
    env.storage().persistent().extend_ttl(&key, 17280, 17280);
}

// Event Emission for Monitoring
/// Event topics for error recovery
pub const ERROR_OCCURRED: Symbol = symbol_short!("err_occur");
pub const RETRY_ATTEMPTED: Symbol = symbol_short!("retry");
pub const RECOVERY_SUCCESS: Symbol = symbol_short!("recovered");
pub const BATCH_PARTIAL: Symbol = symbol_short!("batch_prt");
pub const CIRCUIT_OPENED: Symbol = symbol_short!("circ_open");
pub const CIRCUIT_CLOSED: Symbol = symbol_short!("circ_cls");

/// Emits error occurrence event
pub fn emit_error_event(env: &Env, operation_id: u64, error: RecoveryError, caller: Address) {
    env.events().publish(
        (ERROR_OCCURRED, operation_id),
        (error as u32, caller, env.ledger().timestamp()),
    );
}

/// Emits retry attempt event
pub fn emit_retry_event(env: &Env, operation_id: u64, attempt: u32, delay_ms: u64) {
    env.events().publish(
        (RETRY_ATTEMPTED, operation_id),
        (attempt, delay_ms, env.ledger().timestamp()),
    );
}

/// Emits recovery success event
pub fn emit_recovery_success_event(env: &Env, operation_id: u64, total_attempts: u32) {
    env.events().publish(
        (RECOVERY_SUCCESS, operation_id),
        (total_attempts, env.ledger().timestamp()),
    );
}

/// Emits batch partial success event
pub fn emit_batch_partial_event(env: &Env, batch_result: &BatchResult) {
    env.events().publish(
        (BATCH_PARTIAL,),
        (
            batch_result.total_items,
            batch_result.successful,
            batch_result.failed,
            env.ledger().timestamp(),
        ),
    );
}

/// Emits circuit breaker state change event
pub fn emit_circuit_event(env: &Env, operation_type: Symbol, new_state: CircuitState) {
    let topic = match new_state {
        CircuitState::Open => CIRCUIT_OPENED,
        CircuitState::Closed => CIRCUIT_CLOSED,
        CircuitState::HalfOpen => symbol_short!("circ_half"),
    };

    env.events().publish(
        (topic, operation_type),
        (new_state as u32, env.ledger().timestamp()),
    );
}

// Recovery Strategy
/// Strategy for recovering from errors
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RecoveryStrategy {
    AutoRetry,   // Automatic retry with exponential backoff
    ManualRetry, // Requires manual intervention
    Skip,        // Skip and continue
    Abort,       // Abort entire operation
}

/// Determines recovery strategy based on error type
pub fn determine_recovery_strategy(error: RecoveryError) -> RecoveryStrategy {
    match classify_error(error) {
        ErrorClass::Transient => RecoveryStrategy::AutoRetry,
        ErrorClass::Permanent => RecoveryStrategy::ManualRetry,
        ErrorClass::Partial => RecoveryStrategy::ManualRetry,
    }
}
