// contracts/program-escrow/src/error_recovery.rs
//
// Error Recovery & Circuit Breaker Module
//
// Implements a three-state circuit breaker pattern for protecting the escrow
// contract from cascading failures, integrated with a comprehensive retry
// and error classification system.

#![allow(dead_code)]

use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol, Vec};
use grainlify_time::{self, Timestamp, Duration, TimestampExt};

// ─────────────────────────────────────────────────────────
// Error Types and Classification
// ─────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────
// Circuit Breaker State and Configuration
// ─────────────────────────────────────────────────────────

/// The three states of the circuit breaker.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitState {
    /// Normal operation — requests pass through.
    Closed,
    /// Too many failures — all requests are rejected immediately.
    Open,
    /// Admin has initiated a reset — next success will close the circuit.
    HalfOpen,
}

/// Persistent storage keys for circuit breaker data (Upstream granular keys).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitBreakerKey {
    /// Current circuit state (CircuitState)
    State,
    /// Number of consecutive failures since last reset
    FailureCount,
    /// Timestamp of the last recorded failure
    LastFailureTimestamp,
    /// Timestamp when the circuit was opened
    OpenedAt,
    /// Number of successful operations since last failure
    SuccessCount,
    /// Admin address allowed to reset the circuit
    Admin,
    /// Configuration (threshold, etc.)
    Config,
    /// Operation-level error log (last N errors)
    ErrorLog,
}

/// Configuration for the circuit breaker.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures required to open the circuit.
    pub failure_threshold: u32,
    /// Number of consecutive successes in HalfOpen to close the circuit.
    pub success_threshold: u32,
    /// Maximum number of error log entries to retain.
    pub max_error_log: u32,
}

impl CircuitBreakerConfig {
    pub fn default() -> Self {
        CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 1,
            max_error_log: 10,
        }
    }
}

/// A single error log entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorEntry {
    pub operation: Symbol,
    pub program_id: String,
    pub error_code: u32,
    pub timestamp: Timestamp,
    pub failure_count_at_time: u32,
}

/// Snapshot of the circuit breaker's current status.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerStatus {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_timestamp: Timestamp,
    pub opened_at: Timestamp,
    pub failure_threshold: u32,
    pub success_threshold: u32,
}

// ─────────────────────────────────────────────────────────
// Error codes (u32 — no_std compatible)
// ─────────────────────────────────────────────────────────

/// Circuit is open; operation rejected without attempting.
pub const ERR_CIRCUIT_OPEN: u32 = 1001;
/// Token transfer failed (transient).
pub const ERR_TRANSFER_FAILED: u32 = 1002;
/// Insufficient contract balance.
pub const ERR_INSUFFICIENT_BALANCE: u32 = 1003;
/// Operation succeeded — for logging.
pub const ERR_NONE: u32 = 0;

// ─────────────────────────────────────────────────────────
// Core circuit breaker functions
// ─────────────────────────────────────────────────────────

pub fn get_config(env: &Env) -> CircuitBreakerConfig {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::Config)
        .unwrap_or(CircuitBreakerConfig::default())
}

pub fn set_config(env: &Env, config: CircuitBreakerConfig) {
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::Config, &config);
}

pub fn get_state(env: &Env) -> CircuitState {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::State)
        .unwrap_or(CircuitState::Closed)
}

pub fn get_failure_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::FailureCount)
        .unwrap_or(0)
}

pub fn get_success_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::SuccessCount)
        .unwrap_or(0)
}

pub fn get_status(env: &Env) -> CircuitBreakerStatus {
    let config = get_config(env);
    CircuitBreakerStatus {
        state: get_state(env),
        failure_count: get_failure_count(env),
        success_count: get_success_count(env),
        last_failure_timestamp: env
            .storage()
            .persistent()
            .get(&CircuitBreakerKey::LastFailureTimestamp)
            .unwrap_or(0),
        opened_at: env
            .storage()
            .persistent()
            .get(&CircuitBreakerKey::OpenedAt)
            .unwrap_or(0),
        failure_threshold: config.failure_threshold,
        success_threshold: config.success_threshold,
    }
}

pub fn check_and_allow(env: &Env) -> Result<(), u32> {
    match get_state(env) {
        CircuitState::Open => {
            emit_circuit_event(env, symbol_short!("cb_reject"), get_failure_count(env));
            Err(ERR_CIRCUIT_OPEN)
        }
        CircuitState::Closed | CircuitState::HalfOpen => Ok(()),
    }
}

pub fn check_and_allow_with_thresholds(env: &Env) -> Result<(), u32> {
    check_and_allow(env)?;
    
    if let Err(breach) = crate::threshold_monitor::check_thresholds(env) {
        open_circuit(env);
        crate::threshold_monitor::emit_threshold_breach_event(env, &breach);
        crate::threshold_monitor::apply_cooldown(env);
        
        let mut metrics = crate::threshold_monitor::get_current_metrics(env);
        metrics.breach_count += 1;
        env.storage()
            .persistent()
            .set(&crate::threshold_monitor::ThresholdKey::CurrentMetrics, &metrics);
        
        return Err(crate::threshold_monitor::ERR_THRESHOLD_BREACHED);
    }
    
    Ok(())
}

pub fn record_success(env: &Env) {
    let state = get_state(env);
    match state {
        CircuitState::Closed => {
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::FailureCount, &0u32);
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::SuccessCount, &0u32);
        }
        CircuitState::HalfOpen => {
            let config = get_config(env);
            let successes = get_success_count(env) + 1;
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::SuccessCount, &successes);

            if successes >= config.success_threshold {
                close_circuit(env);
            }
        }
        CircuitState::Open => {}
    }
}

pub fn record_failure(
    env: &Env,
    program_id: String,
    operation: Symbol,
    error_code: u32,
) {
    let config = get_config(env);
    let failures = get_failure_count(env) + 1;
    let now = grainlify_time::now(env);

    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::FailureCount, &failures);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::LastFailureTimestamp, &now);

    let mut log: Vec<ErrorEntry> = env
        .storage()
        .persistent()
        .get(&CircuitBreakerKey::ErrorLog)
        .unwrap_or(Vec::new(env));

    let entry = ErrorEntry {
        operation: operation.clone(),
        program_id,
        error_code,
        timestamp: now,
        failure_count_at_time: failures,
    };
    log.push_back(entry);

    while log.len() > config.max_error_log {
        log.remove(0);
    }
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::ErrorLog, &log);

    emit_circuit_event(env, symbol_short!("cb_fail"), failures);

    if failures >= config.failure_threshold {
        open_circuit(env);
    }
}

pub fn open_circuit(env: &Env) {
    let now = grainlify_time::now(env);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::State, &CircuitState::Open);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::OpenedAt, &now);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::SuccessCount, &0u32);

    emit_circuit_event(env, symbol_short!("cb_open"), get_failure_count(env));
}

pub fn half_open_circuit(env: &Env) {
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::State, &CircuitState::HalfOpen);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::SuccessCount, &0u32);

    emit_circuit_event(env, symbol_short!("cb_half"), get_failure_count(env));
}

pub fn close_circuit(env: &Env) {
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::State, &CircuitState::Closed);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::FailureCount, &0u32);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::SuccessCount, &0u32);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::OpenedAt, &0u64);

    emit_circuit_event(env, symbol_short!("cb_close"), 0);
}

pub fn reset_circuit_breaker(env: &Env, admin: &Address) {
    let stored_admin: Option<Address> = env.storage().persistent().get(&CircuitBreakerKey::Admin);

    match stored_admin {
        Some(ref a) if a == admin => {
            admin.require_auth();
        }
        _ => panic!("Unauthorized: only registered circuit breaker admin can reset"),
    }

    let state = get_state(env);
    match state {
        CircuitState::Open => half_open_circuit(env),
        CircuitState::HalfOpen | CircuitState::Closed => close_circuit(env),
    }
}

pub fn set_circuit_admin(env: &Env, new_admin: Address, caller: Option<Address>) {
    let existing: Option<Address> = env.storage().persistent().get(&CircuitBreakerKey::Admin);

    if let Some(ref current) = existing {
        match caller {
            Some(ref c) if c == current => {
                current.require_auth();
            }
            _ => panic!("Unauthorized: only current admin can change circuit breaker admin"),
        }
    }

    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::Admin, &new_admin);
}

pub fn get_circuit_admin(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&CircuitBreakerKey::Admin)
}

pub fn get_error_log(env: &Env) -> Vec<ErrorEntry> {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::ErrorLog)
        .unwrap_or(Vec::new(env))
}

// ─────────────────────────────────────────────────────────
// Retry logic and supporting types
// ─────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: u32,
    pub jitter_percent: u32,
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

    pub fn aggressive(_env: &Env) -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 50,
            max_delay_ms: 3000,
            backoff_multiplier: 2,
            jitter_percent: 15,
        }
    }

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
    let multiplier_power = config.backoff_multiplier.pow(attempt);
    let base_delay = config
        .initial_delay_ms
        .saturating_mul(multiplier_power as u64);

    let capped_delay = base_delay.min(config.max_delay_ms);

    let jitter_range = (capped_delay * config.jitter_percent as u64) / 100;

    if jitter_range > 0 {
        let timestamp = env.ledger().timestamp();
        let jitter_offset = (timestamp % (jitter_range * 2)).saturating_sub(jitter_range);
        capped_delay.saturating_add(jitter_offset)
    } else {
        capped_delay
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryResult {
    pub succeeded: bool,
    pub attempts: u32,
    pub final_error: u32, // ERR_NONE if succeeded
    pub total_delay: u64, // Total backoff delay accumulated
}

// Error State Tracking
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorState {
    pub operation_id: u64,
    pub error_type: u32,
    pub retry_count: u32,
    pub last_retry_timestamp: Timestamp,
    pub first_error_timestamp: Timestamp,
    pub can_recover: bool,
    pub error_message: Symbol,
    pub caller: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorStateKey {
    State(u64),
    OperationCounter,
}

pub fn create_error_state(
    env: &Env,
    operation_id: u64,
    error: RecoveryError,
    caller: Address,
) -> ErrorState {
    let error_class = classify_error(error);
    let can_recover = matches!(error_class, ErrorClass::Transient);
    let now = grainlify_time::now(env);

    ErrorState {
        operation_id,
        error_type: error as u32,
        retry_count: 0,
        last_retry_timestamp: now,
        first_error_timestamp: now,
        can_recover,
        error_message: symbol_short!("err"),
        caller,
    }
}

pub fn store_error_state(env: &Env, state: &ErrorState) {
    let key = ErrorStateKey::State(state.operation_id);
    env.storage().persistent().set(&key, state);
    env.storage().persistent().extend_ttl(&key, 120960, 120960);
}

pub fn get_error_state(env: &Env, operation_id: u64) -> Option<ErrorState> {
    let key = ErrorStateKey::State(operation_id);
    env.storage().persistent().get(&key)
}

pub fn generate_operation_id(env: &Env) -> u64 {
    let key = ErrorStateKey::OperationCounter;
    let counter: u64 = env.storage().persistent().get(&key).unwrap_or(0);
    let new_counter = counter.saturating_add(1);
    env.storage().persistent().set(&key, &new_counter);
    new_counter
}

// Batch Operation Results
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchResult {
    pub total_items: u32,
    pub successful: u32,
    pub failed: u32,
    pub failed_indices: Vec<u32>,
    pub error_details: Vec<BatchItemError>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchItemError {
    pub index: u32,
    pub recipient: Address,
    pub amount: i128,
    pub error_code: u32,
    pub can_retry: bool,
    pub timestamp: Timestamp,
}

impl BatchResult {
    pub fn new(env: &Env, total_items: u32) -> Self {
        Self {
            total_items,
            successful: 0,
            failed: 0,
            failed_indices: Vec::new(env),
            error_details: Vec::new(env),
        }
    }

    pub fn record_success(&mut self) {
        self.successful = self.successful.saturating_add(1);
    }

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
            timestamp: grainlify_time::now(env),
        };

        self.error_details.push_back(error_detail);
    }

    pub fn is_full_success(&self) -> bool {
        self.failed == 0
    }

    pub fn is_partial_success(&self) -> bool {
        self.successful > 0 && self.failed > 0
    }

    pub fn is_complete_failure(&self) -> bool {
        self.successful == 0 && self.failed > 0
    }
}

// ─────────────────────────────────────────────────────────
// Event topics for error recovery
// ─────────────────────────────────────────────────────────

pub const ERROR_OCCURRED: Symbol = symbol_short!("err_occur");
pub const RETRY_ATTEMPTED: Symbol = symbol_short!("retry");
pub const RECOVERY_SUCCESS: Symbol = symbol_short!("recovered");
pub const BATCH_PARTIAL: Symbol = symbol_short!("batch_prt");
pub const CIRCUIT_OPENED: Symbol = symbol_short!("circ_open");
pub const CIRCUIT_CLOSED: Symbol = symbol_short!("circ_cls");

pub fn emit_error_event(env: &Env, operation_id: u64, error: RecoveryError, caller: Address) {
    env.events().publish(
        (ERROR_OCCURRED, operation_id),
        (error as u32, caller, grainlify_time::now(env)),
    );
}

pub fn emit_retry_event(env: &Env, operation_id: u64, attempt: u32, delay_ms: u64) {
    env.events().publish(
        (RETRY_ATTEMPTED, operation_id),
        (attempt, delay_ms, grainlify_time::now(env)),
    );
}

pub fn emit_recovery_success_event(env: &Env, operation_id: u64, total_attempts: u32) {
    env.events().publish(
        (RECOVERY_SUCCESS, operation_id),
        (total_attempts, grainlify_time::now(env)),
    );
}

pub fn emit_batch_partial_event(env: &Env, batch_result: &BatchResult) {
    env.events().publish(
        (BATCH_PARTIAL,),
        (
            batch_result.total_items,
            batch_result.successful,
            batch_result.failed,
            grainlify_time::now(env),
        ),
    );
}

fn emit_circuit_event(env: &Env, event_type: Symbol, value: u32) {
    env.events().publish(
        (symbol_short!("circuit"), event_type),
        (value, grainlify_time::now(env)),
    );
}

// ─────────────────────────────────────────────────────────
// Recovery Strategy
// ─────────────────────────────────────────────────────────

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RecoveryStrategy {
    AutoRetry,
    ManualRetry,
    Skip,
    Abort,
}

pub fn determine_recovery_strategy(error: RecoveryError) -> RecoveryStrategy {
    match classify_error(error) {
        ErrorClass::Transient => RecoveryStrategy::AutoRetry,
        ErrorClass::Permanent => RecoveryStrategy::ManualRetry,
        ErrorClass::Partial => RecoveryStrategy::ManualRetry,
    }
}

// ─────────────────────────────────────────────────────────
// Invariant Verification
// ─────────────────────────────────────────────────────────

pub fn verify_circuit_invariants(env: &Env) -> bool {
    let status = get_status(env);
    let config = get_config(env);

    match status.state {
        CircuitState::Open => {
            if status.opened_at == 0 {
                return false;
            }
        }
        CircuitState::Closed => {
            if status.opened_at != 0 {
                return false;
            }
            if status.failure_count >= config.failure_threshold {
                return false;
            }
        }
        CircuitState::HalfOpen => {
            if status.success_count >= config.success_threshold {
                return false;
            }
        }
    }
    true
}
