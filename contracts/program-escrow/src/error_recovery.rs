// contracts/program-escrow/src/error_recovery.rs
//
// Error Recovery & Circuit Breaker Module
//
// Implements a three-state circuit breaker pattern for protecting the escrow
// contract from cascading failures during token transfers and external calls.
//
// ## Circuit States
//
// ```
//   [Closed] ──(failure_count >= threshold)──> [Open]
//      ^                                          │
//      │                                          │
//   (reset by admin)                    (stays open until reset)
//      │                                          │
//   [HalfOpen] <────────────────────────────────-─┘
//                    (admin calls reset)
// ```
//
// ## Storage Keys
// All circuit breaker state is stored in persistent storage keyed by
// `CircuitBreakerKey::*`.

use soroban_sdk::{contracttype, symbol_short, Address, Env, String};

// ─────────────────────────────────────────────────────────
// Types
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

/// Persistent storage keys for circuit breaker data.
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
    pub operation: soroban_sdk::Symbol,
    pub program_id: String,
    pub error_code: u32,
    pub timestamp: u64,
    pub failure_count_at_time: u32,
}

/// Snapshot of the circuit breaker's current status (returned by `get_status`).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerStatus {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_timestamp: u64,
    pub opened_at: u64,
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

/// Returns the current circuit breaker configuration, or defaults.
pub fn get_config(env: &Env) -> CircuitBreakerConfig {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::Config)
        .unwrap_or(CircuitBreakerConfig::default())
}

/// Sets the circuit breaker configuration. Admin only (caller must enforce auth).
pub fn set_config(env: &Env, config: CircuitBreakerConfig) {
    let prev_config = get_config(env);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::Config, &config);

    // Emit audit event for config change
    env.events().publish(
        (symbol_short!("circuit"), symbol_short!("cb_cfg")),
        (
            prev_config.failure_threshold,
            config.failure_threshold,
            prev_config.success_threshold,
            config.success_threshold,
            env.ledger().timestamp(),
        ),
    );
}

/// Returns the current circuit state.
pub fn get_state(env: &Env) -> CircuitState {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::State)
        .unwrap_or(CircuitState::Closed)
}

/// Returns the current failure count.
pub fn get_failure_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::FailureCount)
        .unwrap_or(0)
}

/// Returns the current success count (since last state transition).
pub fn get_success_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::SuccessCount)
        .unwrap_or(0)
}

/// Returns a full status snapshot.
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

/// **Call this before any protected operation.**
///
/// Returns `Err(ERR_CIRCUIT_OPEN)` if the circuit is Open.
/// Records that we are attempting an operation (no state change yet).
pub fn check_and_allow(env: &Env) -> Result<(), u32> {
    match get_state(env) {
        CircuitState::Open => {
            emit_circuit_event(env, symbol_short!("cb_reject"), get_failure_count(env));
            Err(ERR_CIRCUIT_OPEN)
        }
        CircuitState::Closed | CircuitState::HalfOpen => Ok(()),
    }
}

/// **Call this before any protected operation with threshold monitoring.**
///
/// Checks both circuit breaker state and threshold metrics.
/// Opens circuit if thresholds are breached.
pub fn check_and_allow_with_thresholds(env: &Env) -> Result<(), u32> {
    // First check circuit state
    check_and_allow(env)?;

    // Then check thresholds
    if let Err(breach) = crate::threshold_monitor::check_thresholds(env) {
        // Threshold breached - open circuit
        open_circuit(env);
        crate::threshold_monitor::emit_threshold_breach_event(env, &breach);
        crate::threshold_monitor::apply_cooldown(env);

        // Update breach count in metrics
        let mut metrics = crate::threshold_monitor::get_current_metrics(env);
        metrics.breach_count += 1;
        env.storage().persistent().set(
            &crate::threshold_monitor::ThresholdKey::CurrentMetrics,
            &metrics,
        );

        return Err(crate::threshold_monitor::ERR_THRESHOLD_BREACHED);
    }

    Ok(())
}

/// **Call this after a SUCCESSFUL protected operation.**
///
/// In HalfOpen: increments success counter; closes the circuit when
/// `success_threshold` is reached.
/// In Closed: resets failure counter to 0.
pub fn record_success(env: &Env) {
    let state = get_state(env);
    match state {
        CircuitState::Closed => {
            // Reset failure streak on any success
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
                // Enough successes — close the circuit
                close_circuit(env);
            }
        }
        CircuitState::Open => {
            // Shouldn't happen if check_and_allow is used correctly; ignore.
        }
    }
}

/// **Call this after a FAILED protected operation.**
///
/// Increments the failure counter and opens the circuit if the threshold
/// is exceeded. Records error log entry.
pub fn record_failure(
    env: &Env,
    program_id: String,
    operation: soroban_sdk::Symbol,
    error_code: u32,
) {
    let config = get_config(env);
    let failures = get_failure_count(env) + 1;
    let now = env.ledger().timestamp();

    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::FailureCount, &failures);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::LastFailureTimestamp, &now);

    // Append to error log (capped at max_error_log)
    let mut log: soroban_sdk::Vec<ErrorEntry> = env
        .storage()
        .persistent()
        .get(&CircuitBreakerKey::ErrorLog)
        .unwrap_or(soroban_sdk::Vec::new(env));

    let entry = ErrorEntry {
        operation: operation.clone(),
        program_id,
        error_code,
        timestamp: now,
        failure_count_at_time: failures,
    };
    log.push_back(entry);

    // Trim to max
    while log.len() > config.max_error_log {
        log.remove(0);
    }
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::ErrorLog, &log);

    emit_circuit_event_detailed(
        env,
        symbol_short!("cb_fail"),
        failures,
        Some(operation),
        Some(program_id),
        Some(error_code),
    );

    // Open circuit if threshold exceeded
    if failures >= config.failure_threshold {
        open_circuit_internal(env, symbol_short!("auto"));
    }
}

/// Transitions the circuit to **Open** state.
pub fn open_circuit(env: &Env) {
    open_circuit_internal(env, symbol_short!("manual"));
}

fn open_circuit_internal(env: &Env, reason: soroban_sdk::Symbol) {
    let now = env.ledger().timestamp();
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::State, &CircuitState::Open);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::OpenedAt, &now);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::SuccessCount, &0u32);

    env.events().publish(
        (symbol_short!("circuit"), symbol_short!("cb_open")),
        (get_failure_count(env), reason, now),
    );
}

/// Transitions the circuit to **HalfOpen** state (admin-initiated reset attempt).
pub fn half_open_circuit(env: &Env) {
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::State, &CircuitState::HalfOpen);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::SuccessCount, &0u32);

    emit_circuit_event(env, symbol_short!("cb_half"), get_failure_count(env));
}

/// Transitions the circuit to **Closed** state and resets all counters.
/// Called automatically after sufficient successes in HalfOpen,
/// or directly by admin for a hard reset.
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

    env.events().publish(
        (symbol_short!("circuit"), symbol_short!("cb_close")),
        (env.ledger().timestamp(),),
    );
}

/// **Admin reset**: moves Open → HalfOpen, or HalfOpen/Closed → Closed.
///
/// The caller must have already verified admin authorization before calling this.
pub fn reset_circuit_breaker(env: &Env, admin: &Address) {
    // Verify admin is registered
    let stored_admin: Option<Address> = env.storage().persistent().get(&CircuitBreakerKey::Admin);

    match stored_admin {
        Some(ref a) if a == admin => {
            admin.require_auth();
        }
        _ => panic!("Unauthorized: only registered circuit breaker admin can reset"),
    }

    let state = get_state(env);
    let now = env.ledger().timestamp();

    // Emit audit event for manual reset
    env.events().publish(
        (symbol_short!("circuit"), symbol_short!("cb_reset")),
        (admin.clone(), state.clone(), now),
    );

    match state {
        CircuitState::Open => half_open_circuit(env),
        CircuitState::HalfOpen | CircuitState::Closed => close_circuit(env),
    }
}

/// Register (or update) the admin address for circuit breaker resets.
/// Can only be set once, or updated by the existing admin.
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

    // Emit audit event for admin change
    env.events().publish(
        (symbol_short!("circuit"), symbol_short!("cb_adm")),
        (existing, new_admin, env.ledger().timestamp()),
    );
}

/// Returns the circuit breaker admin address, if set.
pub fn get_circuit_admin(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&CircuitBreakerKey::Admin)
}

/// Returns the full error log.
pub fn get_error_log(env: &Env) -> soroban_sdk::Vec<ErrorEntry> {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::ErrorLog)
        .unwrap_or(soroban_sdk::Vec::new(env))
}

// ─────────────────────────────────────────────────────────
// Retry logic
// ─────────────────────────────────────────────────────────

/// Retry configuration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryConfig {
    /// Maximum number of attempts (1 = no retry).
    pub max_attempts: u32,
    /// Initial backoff delay in ledger timestamps (0 = no delay).
    pub initial_backoff: u64,
    /// Backoff multiplier for exponential backoff (1 = constant delay).
    pub backoff_multiplier: u32,
    /// Maximum backoff delay cap in ledger timestamps.
    pub max_backoff: u64,
}

impl RetryConfig {
    pub fn default() -> Self {
        RetryConfig {
            max_attempts: 3,
            initial_backoff: 0,
            backoff_multiplier: 1,
            max_backoff: 0,
        }
    }

    /// Aggressive retry policy: more attempts, minimal backoff.
    pub fn aggressive() -> Self {
        RetryConfig {
            max_attempts: 5,
            initial_backoff: 1,
            backoff_multiplier: 1,
            max_backoff: 5,
        }
    }

    /// Conservative retry policy: fewer attempts, exponential backoff.
    pub fn conservative() -> Self {
        RetryConfig {
            max_attempts: 3,
            initial_backoff: 10,
            backoff_multiplier: 2,
            max_backoff: 100,
        }
    }

    /// Exponential backoff policy: moderate attempts, strong exponential growth.
    pub fn exponential() -> Self {
        RetryConfig {
            max_attempts: 4,
            initial_backoff: 5,
            backoff_multiplier: 3,
            max_backoff: 200,
        }
    }

    /// Compute the backoff delay for a given attempt number (0-indexed).
    pub fn compute_backoff(&self, attempt: u32) -> u64 {
        if self.initial_backoff == 0 {
            return 0;
        }
        let delay = self.initial_backoff * (self.backoff_multiplier.pow(attempt) as u64);
        delay.min(self.max_backoff)
    }
}

/// Result of a retry operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryResult {
    pub succeeded: bool,
    pub attempts: u32,
    pub final_error: u32, // ERR_NONE if succeeded
    pub total_delay: u64, // Total backoff delay accumulated
}

/// Execute a fallible operation with retry, integrated with the circuit breaker.
///
/// `op` is a closure that returns `Ok(())` on success or `Err(error_code)` on
/// transient failure. A non-zero error triggers a `record_failure` call.
///
/// Returns a `RetryResult` describing the outcome.
///
/// **Note**: In Soroban's no_std environment, closures that capture `env`
/// references must be careful about lifetimes. This function is designed for
/// use with simple operations that can be expressed as a bool-returning function
/// since true closures with captures are complex. Callers should call
/// `check_and_allow` / `record_success` / `record_failure` directly for
/// real contract operations; this helper is useful for test scenarios and
/// simulation.
pub fn execute_with_retry<F>(
    env: &Env,
    config: &RetryConfig,
    program_id: String,
    operation: soroban_sdk::Symbol,
    mut op: F,
) -> RetryResult
where
    F: FnMut() -> Result<(), u32>,
{
    let mut attempts = 0u32;
    let mut last_error = ERR_NONE;
    let mut total_delay = 0u64;

    for attempt_idx in 0..config.max_attempts {
        // Check circuit before each attempt
        if let Err(e) = check_and_allow(env) {
            return RetryResult {
                succeeded: false,
                attempts,
                final_error: e,
                total_delay,
            };
        }

        // Apply backoff delay before retry (skip on first attempt)
        if attempt_idx > 0 {
            let delay = config.compute_backoff(attempt_idx - 1);
            total_delay += delay;
            // In a real implementation, we would wait here.
            // For testing, we just track the delay.
            // env.ledger().set_timestamp(env.ledger().timestamp() + delay);
        }

        attempts += 1;
        match op() {
            Ok(()) => {
                record_success(env);
                return RetryResult {
                    succeeded: true,
                    attempts,
                    final_error: ERR_NONE,
                    total_delay,
                };
            }
            Err(code) => {
                last_error = code;
                record_failure(env, program_id.clone(), operation.clone(), code);
            }
        }
    }

    RetryResult {
        succeeded: false,
        attempts,
        final_error: last_error,
        total_delay,
    }
}

// ─────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────

fn emit_circuit_event(env: &Env, event_type: soroban_sdk::Symbol, value: u32) {
    env.events().publish(
        (symbol_short!("circuit"), event_type),
        (value, env.ledger().timestamp()),
    );
}

fn emit_circuit_event_detailed(
    env: &Env,
    event_type: soroban_sdk::Symbol,
    value: u32,
    operation: Option<soroban_sdk::Symbol>,
    program_id: Option<String>,
    error_code: Option<u32>,
) {
    env.events().publish(
        (symbol_short!("circuit"), event_type),
        (
            value,
            operation,
            program_id,
            error_code,
            env.ledger().timestamp(),
        ),
    );
}

// ─────────────────────────────────────────────────────────
// Invariant Verification
// ─────────────────────────────────────────────────────────

/// Verifies that the circuit breaker state is internally consistent.
pub fn verify_circuit_invariants(env: &Env) -> bool {
    let status = get_status(env);
    let config = get_config(env);

    match status.state {
        CircuitState::Open => {
            // Invariant: In Open state, opened_at must be non-zero
            if status.opened_at == 0 {
                return false;
            }
            // Invariant: In Open state, failure_count should be >= threshold (unless emergency opened)
            // Note: We'll allow emergency open, so we don't strictly check threshold here
            // but we could check if failure_count + emergency_flag is valid.
        }
        CircuitState::Closed => {
            // Invariant: In Closed state, opened_at must be 0
            if status.opened_at != 0 {
                return false;
            }
            // Invariant: In Closed state, failure_count should be < threshold
            if status.failure_count >= config.failure_threshold {
                return false;
            }
        }
        CircuitState::HalfOpen => {
            // Invariant: success_count should be < success_threshold
            if status.success_count >= config.success_threshold {
                return false;
            }
        }
    }
    true
}

// ─────────────────────────────────────────────────────────
// Batch Operation Recovery Module
// ─────────────────────────────────────────────────────────
//
// This module provides resilient batch operation handling to ensure
// partial failures do not strand funds incorrectly.
//
// ## Failure Modes
//
// 1. **Transfer Failure**: Individual token transfer fails mid-batch
// 2. **Circuit Breaker Trip**: Circuit opens during batch execution
// 3. **Storage Failure**: State persistence fails (rare in Soroban)
// 4. **Authorization Failure**: Unauthorized caller attempts recovery
//
// ## Recovery Strategy
//
// For each batch operation:
// 1. Store checkpoint before execution begins
// 2. Track status of each item (Pending/Success/Failed)
// 3. On partial failure, persist recovery state
// 4. Allow retry of failed items or rollback of successful ones
//
// ## Invariants
//
// - INV-1: Total funds (successful + pending + failed) must equal original balance
// - INV-2: No funds can be stranded in an unrecoverable state
// - INV-3: Only authorized key can trigger recovery operations
// - INV-4: Recovery state is cleaned up after successful completion

/// Status of an individual item within a batch operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchItemStatus {
    /// Item has not been processed yet
    Pending,
    /// Item was processed successfully
    Success,
    /// Item processing failed
    Failed,
    /// Item was rolled back after initial success
    RolledBack,
}

/// Status of an individual item within a batch, including amount and recipient.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchItem {
    /// Index in the original batch
    pub index: u32,
    /// Recipient address
    pub recipient: Address,
    /// Amount to transfer
    pub amount: i128,
    /// Current status of this item
    pub status: BatchItemStatus,
    /// Error code if failed (0 if no error)
    pub error_code: u32,
    /// Number of retry attempts
    pub retry_count: u32,
}

/// State of a batch operation for recovery purposes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryState {
    /// Unique identifier for this batch operation
    pub batch_id: u64,
    /// Program ID this batch belongs to
    pub program_id: String,
    /// Original balance before batch started
    pub original_balance: i128,
    /// Total amount attempted in batch
    pub total_amount: i128,
    /// Amount successfully transferred
    pub successful_amount: i128,
    /// Number of items in batch
    pub item_count: u32,
    /// Items and their statuses
    pub items: soroban_sdk::Vec<BatchItem>,
    /// Timestamp when batch was initiated
    pub started_at: u64,
    /// Timestamp when batch completed (success or failure)
    pub completed_at: Option<u64>,
    /// Whether this batch is pending recovery
    pub pending_recovery: bool,
    /// Authorized key that initiated the batch
    pub authorized_key: Address,
}

/// Configuration for batch recovery behavior.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryConfig {
    /// Maximum number of retry attempts for failed items
    pub max_retries: u32,
    /// Whether to automatically retry on failure
    pub auto_retry: bool,
    /// Maximum items per batch
    pub max_batch_size: u32,
    /// Enable rollback capability
    pub rollback_enabled: bool,
    /// Timeout in seconds after which pending recovery expires
    pub recovery_timeout_secs: u64,
}

impl BatchRecoveryConfig {
    /// Default configuration with sensible limits.
    pub fn default() -> Self {
        BatchRecoveryConfig {
            max_retries: 3,
            auto_retry: false,
            max_batch_size: 100,
            rollback_enabled: true,
            recovery_timeout_secs: 86400, // 24 hours
        }
    }

    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_retries > 10 {
            return Err("Max retries cannot exceed 10");
        }
        if self.max_batch_size == 0 || self.max_batch_size > 500 {
            return Err("Batch size must be between 1 and 500");
        }
        if self.recovery_timeout_secs < 3600 || self.recovery_timeout_secs > 604800 {
            return Err("Recovery timeout must be between 1 hour and 7 days");
        }
        Ok(())
    }
}

/// Storage keys for batch recovery data.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchRecoveryKey {
    /// Current batch recovery configuration
    Config,
    /// Next batch ID counter
    NextBatchId,
    /// Active batch recovery state by batch_id
    ActiveBatch(u64),
    /// List of pending recovery batch IDs
    PendingRecoveries,
    /// Recovery history (last N batches)
    RecoveryHistory,
}

/// Error codes for batch recovery operations.
pub const ERR_BATCH_NOT_FOUND: u32 = 3001;
pub const ERR_BATCH_ALREADY_COMPLETE: u32 = 3002;
pub const ERR_BATCH_NOT_RECOVERABLE: u32 = 3003;
pub const ERR_UNAUTHORIZED_RECOVERY: u32 = 3004;
pub const ERR_BATCH_SIZE_EXCEEDED: u32 = 3005;
pub const ERR_RECOVERY_EXPIRED: u32 = 3006;
pub const ERR_ROLLBACK_DISABLED: u32 = 3007;
pub const ERR_NO_FAILED_ITEMS: u32 = 3008;
pub const ERR_NO_SUCCESSFUL_ITEMS: u32 = 3009;
pub const ERR_INVALID_BATCH_CONFIG: u32 = 3010;

// ─────────────────────────────────────────────────────────
// Batch Recovery Configuration
// ─────────────────────────────────────────────────────────

/// Get the current batch recovery configuration.
pub fn get_batch_recovery_config(env: &Env) -> BatchRecoveryConfig {
    env.storage()
        .persistent()
        .get(&BatchRecoveryKey::Config)
        .unwrap_or(BatchRecoveryConfig::default())
}

/// Set the batch recovery configuration.
///
/// # Security
/// Caller must enforce admin authorization before calling this function.
pub fn set_batch_recovery_config(env: &Env, config: BatchRecoveryConfig) -> Result<(), u32> {
    config.validate().map_err(|_| ERR_INVALID_BATCH_CONFIG)?;
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::Config, &config);
    emit_batch_event(env, symbol_short!("br_cfg"), 0, 0);
    Ok(())
}

/// Get the next batch ID and increment the counter.
fn get_next_batch_id(env: &Env) -> u64 {
    let next_id: u64 = env
        .storage()
        .persistent()
        .get(&BatchRecoveryKey::NextBatchId)
        .unwrap_or(1);
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::NextBatchId, &(next_id + 1));
    next_id
}

// ─────────────────────────────────────────────────────────
// Batch State Storage
// ─────────────────────────────────────────────────────────

/// Store batch state before execution begins.
///
/// Creates a checkpoint that can be used for recovery if the batch
/// fails partway through execution.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `program_id` - Program identifier
/// * `recipients` - Vector of recipient addresses
/// * `amounts` - Vector of amounts to transfer
/// * `original_balance` - Balance before batch starts
/// * `authorized_key` - Address authorized to perform recovery
///
/// # Returns
/// The batch ID assigned to this operation
///
/// # Security
/// Caller must verify authorization before storing batch state.
pub fn store_batch_state(
    env: &Env,
    program_id: String,
    recipients: soroban_sdk::Vec<Address>,
    amounts: soroban_sdk::Vec<i128>,
    original_balance: i128,
    authorized_key: Address,
) -> Result<u64, u32> {
    let config = get_batch_recovery_config(env);

    // Validate batch size
    if recipients.len() > config.max_batch_size {
        return Err(ERR_BATCH_SIZE_EXCEEDED);
    }

    let batch_id = get_next_batch_id(env);
    let now = env.ledger().timestamp();

    // Calculate total amount
    let mut total_amount: i128 = 0;
    let mut items = soroban_sdk::Vec::new(env);

    for i in 0..recipients.len() {
        let recipient = recipients.get(i).unwrap();
        let amount = amounts.get(i).unwrap();
        total_amount = total_amount.checked_add(amount).unwrap_or_else(|| {
            panic!("Batch amount overflow");
        });

        items.push_back(BatchItem {
            index: i as u32,
            recipient,
            amount,
            status: BatchItemStatus::Pending,
            error_code: 0,
            retry_count: 0,
        });
    }

    let state = BatchRecoveryState {
        batch_id,
        program_id,
        original_balance,
        total_amount,
        successful_amount: 0,
        item_count: recipients.len() as u32,
        items,
        started_at: now,
        completed_at: None,
        pending_recovery: false,
        authorized_key,
    };

    // Store the state
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::ActiveBatch(batch_id), &state);

    // Add to pending recoveries list
    add_to_pending_recoveries(env, batch_id);

    emit_batch_event(env, symbol_short!("br_start"), batch_id, recipients.len() as u32);

    Ok(batch_id)
}

/// Retrieve batch state for recovery.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `batch_id` - The batch identifier
///
/// # Returns
/// The batch recovery state, or None if not found
pub fn get_batch_state(env: &Env, batch_id: u64) -> Option<BatchRecoveryState> {
    env.storage()
        .persistent()
        .get(&BatchRecoveryKey::ActiveBatch(batch_id))
}

/// Update batch state during execution.
///
/// # Security
/// Should only be called during batch execution or recovery.
pub fn update_batch_state(env: &Env, state: &BatchRecoveryState) {
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::ActiveBatch(state.batch_id), state);
}

/// Clear batch state after successful completion.
///
/// Removes the batch from active storage and pending list.
///
/// # Security
/// Caller must verify authorization before clearing state.
pub fn clear_batch_state(env: &Env, batch_id: u64) {
    // Remove from active batches
    env.storage()
        .persistent()
        .remove(&BatchRecoveryKey::ActiveBatch(batch_id));

    // Remove from pending recoveries
    remove_from_pending_recoveries(env, batch_id);

    emit_batch_event(env, symbol_short!("br_clear"), batch_id, 0);
}

/// Add batch ID to pending recoveries list.
fn add_to_pending_recoveries(env: &Env, batch_id: u64) {
    let mut pending: soroban_sdk::Vec<u64> = env
        .storage()
        .persistent()
        .get(&BatchRecoveryKey::PendingRecoveries)
        .unwrap_or(soroban_sdk::Vec::new(env));

    pending.push_back(batch_id);
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::PendingRecoveries, &pending);
}

/// Remove batch ID from pending recoveries list.
fn remove_from_pending_recoveries(env: &Env, batch_id: u64) {
    let pending: soroban_sdk::Vec<u64> = env
        .storage()
        .persistent()
        .get(&BatchRecoveryKey::PendingRecoveries)
        .unwrap_or(soroban_sdk::Vec::new(env));

    let mut new_pending = soroban_sdk::Vec::new(env);
    for id in pending.iter() {
        if id != batch_id {
            new_pending.push_back(id);
        }
    }
    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::PendingRecoveries, &new_pending);
}

/// Get all pending recovery batch IDs.
pub fn get_pending_recoveries(env: &Env) -> soroban_sdk::Vec<u64> {
    env.storage()
        .persistent()
        .get(&BatchRecoveryKey::PendingRecoveries)
        .unwrap_or(soroban_sdk::Vec::new(env))
}

// ─────────────────────────────────────────────────────────
// Batch Item Status Updates
// ─────────────────────────────────────────────────────────

/// Mark a batch item as successful.
///
/// Updates the item status and increments the successful amount.
pub fn mark_item_success(env: &Env, batch_id: u64, item_index: u32) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    if item_index >= state.item_count {
        return Err(ERR_BATCH_NOT_FOUND);
    }

    let mut item = state.items.get(item_index).unwrap();
    item.status = BatchItemStatus::Success;
    state.successful_amount = state.successful_amount.checked_add(item.amount).unwrap_or_else(|| {
        panic!("Successful amount overflow");
    });
    state.items.set(item_index, item);

    update_batch_state(env, &state);
    Ok(())
}

/// Mark a batch item as failed.
///
/// Updates the item status with error code.
pub fn mark_item_failed(
    env: &Env,
    batch_id: u64,
    item_index: u32,
    error_code: u32,
) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    if item_index >= state.item_count {
        return Err(ERR_BATCH_NOT_FOUND);
    }

    let mut item = state.items.get(item_index).unwrap();
    item.status = BatchItemStatus::Failed;
    item.error_code = error_code;
    state.items.set(item_index, item);
    state.pending_recovery = true;

    update_batch_state(env, &state);
    emit_batch_event(env, symbol_short!("br_fail"), batch_id, item_index);
    Ok(())
}

/// Mark a batch item as rolled back.
pub fn mark_item_rolled_back(env: &Env, batch_id: u64, item_index: u32) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    if item_index >= state.item_count {
        return Err(ERR_BATCH_NOT_FOUND);
    }

    let mut item = state.items.get(item_index).unwrap();
    let was_success = item.status == BatchItemStatus::Success;
    item.status = BatchItemStatus::RolledBack;

    if was_success {
        state.successful_amount = state.successful_amount.saturating_sub(item.amount);
    }

    state.items.set(item_index, item);
    update_batch_state(env, &state);
    Ok(())
}

// ─────────────────────────────────────────────────────────
// Recovery Functions
// ─────────────────────────────────────────────────────────

/// Result of a batch recovery operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRecoveryResult {
    /// Batch ID that was recovered
    pub batch_id: u64,
    /// Number of items retried
    pub items_retried: u32,
    /// Number of items that succeeded on retry
    pub items_succeeded: u32,
    /// Number of items that still failed
    pub items_failed: u32,
    /// Total amount recovered
    pub amount_recovered: i128,
    /// Whether the batch is now complete
    pub complete: bool,
}

/// Get failed items from a batch for retry.
///
/// Returns the indices of all failed items.
pub fn get_failed_items(env: &Env, batch_id: u64) -> Result<soroban_sdk::Vec<u32>, u32> {
    let state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    let mut failed_indices = soroban_sdk::Vec::new(env);
    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        if item.status == BatchItemStatus::Failed {
            failed_indices.push_back(item.index);
        }
    }

    Ok(failed_indices)
}

/// Get successful items from a batch for potential rollback.
///
/// Returns the indices of all successful items.
pub fn get_successful_items(env: &Env, batch_id: u64) -> Result<soroban_sdk::Vec<u32>, u32> {
    let state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    let mut success_indices = soroban_sdk::Vec::new(env);
    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        if item.status == BatchItemStatus::Success {
            success_indices.push_back(item.index);
        }
    }

    Ok(success_indices)
}

/// Check if a batch recovery has expired.
///
/// A recovery is expired if the recovery timeout has passed since
/// the batch was started.
pub fn is_recovery_expired(env: &Env, batch_id: u64) -> bool {
    let state = match get_batch_state(env, batch_id) {
        Some(s) => s,
        None => return true, // Non-existent batches are considered expired
    };

    let config = get_batch_recovery_config(env);
    let now = env.ledger().timestamp();
    now > state.started_at + config.recovery_timeout_secs
}

/// Increment retry count for an item.
///
/// Returns an error if max retries exceeded.
pub fn increment_retry_count(
    env: &Env,
    batch_id: u64,
    item_index: u32,
) -> Result<(), u32> {
    let config = get_batch_recovery_config(env);
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    let mut item = state.items.get(item_index).unwrap();

    if item.retry_count >= config.max_retries {
        return Err(ERR_BATCH_NOT_RECOVERABLE);
    }

    item.retry_count += 1;
    item.status = BatchItemStatus::Pending; // Reset to pending for retry
    state.items.set(item_index, item);

    update_batch_state(env, &state);
    Ok(())
}

/// Cancel a batch recovery and mark all items as non-recoverable.
///
/// # Security
/// Caller must verify that `caller` is the authorized key for this batch.
pub fn cancel_batch_recovery(
    env: &Env,
    batch_id: u64,
    caller: &Address,
) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    // Verify authorization
    if state.authorized_key != *caller {
        return Err(ERR_UNAUTHORIZED_RECOVERY);
    }

    // Check if already complete
    if state.completed_at.is_some() {
        return Err(ERR_BATCH_ALREADY_COMPLETE);
    }

    state.pending_recovery = false;
    state.completed_at = Some(env.ledger().timestamp());

    update_batch_state(env, &state);
    remove_from_pending_recoveries(env, batch_id);

    emit_batch_event(env, symbol_short!("br_cancel"), batch_id, 0);
    Ok(())
}

// ─────────────────────────────────────────────────────────
// Rollback Mechanism
// ─────────────────────────────────────────────────────────

/// Calculate the total amount that would be rolled back.
///
/// This is the sum of all successfully transferred items.
pub fn calculate_rollback_amount(env: &Env, batch_id: u64) -> Result<i128, u32> {
    let state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    let config = get_batch_recovery_config(env);
    if !config.rollback_enabled {
        return Err(ERR_ROLLBACK_DISABLED);
    }

    let mut total: i128 = 0;
    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        if item.status == BatchItemStatus::Success {
            total = total.checked_add(item.amount).unwrap_or_else(|| {
                panic!("Rollback amount overflow");
            });
        }
    }

    Ok(total)
}

/// Result of a rollback operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackResult {
    /// Batch ID that was rolled back
    pub batch_id: u64,
    /// Number of items rolled back
    pub items_rolled_back: u32,
    /// Total amount returned
    pub amount_returned: i128,
    /// Recipients that were rolled back
    pub affected_recipients: soroban_sdk::Vec<Address>,
}

/// Prepare a rollback for a batch.
///
/// Returns the items that need to be rolled back and the total amount.
/// Does not execute the rollback - caller must handle actual token transfers.
///
/// # Security
/// Caller must verify authorization before executing rollback.
pub fn prepare_rollback(
    env: &Env,
    batch_id: u64,
    caller: &Address,
) -> Result<RollbackResult, u32> {
    let state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    // Verify authorization
    if state.authorized_key != *caller {
        return Err(ERR_UNAUTHORIZED_RECOVERY);
    }

    let config = get_batch_recovery_config(env);
    if !config.rollback_enabled {
        return Err(ERR_ROLLBACK_DISABLED);
    }

    let mut total_amount: i128 = 0;
    let mut count: u32 = 0;
    let mut affected = soroban_sdk::Vec::new(env);

    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        if item.status == BatchItemStatus::Success {
            total_amount = total_amount.checked_add(item.amount).unwrap_or_else(|| {
                panic!("Rollback amount overflow");
            });
            count += 1;
            affected.push_back(item.recipient.clone());
        }
    }

    if count == 0 {
        return Err(ERR_NO_SUCCESSFUL_ITEMS);
    }

    Ok(RollbackResult {
        batch_id,
        items_rolled_back: count,
        amount_returned: total_amount,
        affected_recipients: affected,
    })
}

/// Verify batch integrity after completion.
///
/// Ensures that:
/// 1. Total amounts balance correctly
/// 2. No funds are stranded
/// 3. All items have a terminal status
///
/// # Returns
/// `true` if integrity checks pass, `false` otherwise
pub fn verify_batch_integrity(env: &Env, batch_id: u64) -> bool {
    let state = match get_batch_state(env, batch_id) {
        Some(s) => s,
        None => return false,
    };

    // Calculate actual successful amount from items
    let mut calculated_successful: i128 = 0;
    let mut calculated_pending: i128 = 0;
    let mut calculated_failed: i128 = 0;

    for i in 0..state.items.len() {
        let item = state.items.get(i).unwrap();
        match item.status {
            BatchItemStatus::Success => {
                calculated_successful = calculated_successful.checked_add(item.amount).unwrap_or_else(|| {
                    panic!("Amount overflow in integrity check");
                });
            }
            BatchItemStatus::Pending => {
                calculated_pending = calculated_pending.checked_add(item.amount).unwrap_or_else(|| {
                    panic!("Amount overflow in integrity check");
                });
            }
            BatchItemStatus::Failed | BatchItemStatus::RolledBack => {
                calculated_failed = calculated_failed.checked_add(item.amount).unwrap_or_else(|| {
                    panic!("Amount overflow in integrity check");
                });
            }
        }
    }

    // INV-1: Successful amount must match tracked value
    if calculated_successful != state.successful_amount {
        return false;
    }

    // INV-2: Total must equal sum of all items
    let sum = calculated_successful
        .checked_add(calculated_pending)
        .and_then(|s| s.checked_add(calculated_failed));

    match sum {
        Some(s) if s == state.total_amount => {}
        _ => return false,
    }

    // INV-3: If batch is complete, no pending items should remain
    if state.completed_at.is_some() && calculated_pending > 0 {
        return false;
    }

    true
}

/// Finalize a batch after successful completion.
///
/// Marks the batch as complete and cleans up state.
pub fn finalize_batch(env: &Env, batch_id: u64) -> Result<(), u32> {
    let mut state = get_batch_state(env, batch_id).ok_or(ERR_BATCH_NOT_FOUND)?;

    // Verify integrity before finalizing
    if !verify_batch_integrity(env, batch_id) {
        panic!("Batch integrity check failed during finalization");
    }

    state.completed_at = Some(env.ledger().timestamp());
    state.pending_recovery = false;

    update_batch_state(env, &state);

    // Archive to history
    archive_batch(env, &state);

    // Clear active state
    clear_batch_state(env, batch_id);

    emit_batch_event(env, symbol_short!("br_done"), batch_id, state.item_count);
    Ok(())
}

/// Archive a completed batch to history.
fn archive_batch(env: &Env, state: &BatchRecoveryState) {
    let mut history: soroban_sdk::Vec<BatchRecoveryState> = env
        .storage()
        .persistent()
        .get(&BatchRecoveryKey::RecoveryHistory)
        .unwrap_or(soroban_sdk::Vec::new(env));

    // Keep last 50 batches in history
    const MAX_HISTORY: u32 = 50;
    if history.len() >= MAX_HISTORY {
        history.remove(0);
    }
    history.push_back(state.clone());

    env.storage()
        .persistent()
        .set(&BatchRecoveryKey::RecoveryHistory, &history);
}

/// Get batch recovery history.
pub fn get_recovery_history(env: &Env) -> soroban_sdk::Vec<BatchRecoveryState> {
    env.storage()
        .persistent()
        .get(&BatchRecoveryKey::RecoveryHistory)
        .unwrap_or(soroban_sdk::Vec::new(env))
}

// ─────────────────────────────────────────────────────────
// Event Emission
// ─────────────────────────────────────────────────────────

/// Emit a batch recovery event.
fn emit_batch_event(env: &Env, event_type: soroban_sdk::Symbol, batch_id: u64, value: u32) {
    env.events().publish(
        (symbol_short!("batch"), event_type),
        (batch_id, value, env.ledger().timestamp()),
    );
}

// ─────────────────────────────────────────────────────────
// Batch Recovery Invariants
// ─────────────────────────────────────────────────────────

/// Verify all batch recovery invariants.
///
/// This should be called periodically to ensure system integrity.
pub fn verify_batch_recovery_invariants(env: &Env) -> bool {
    let pending = get_pending_recoveries(env);

    for batch_id in pending.iter() {
        if !verify_batch_integrity(env, batch_id) {
            return false;
        }

        // Check for expired recoveries
        if is_recovery_expired(env, batch_id) {
            // Could auto-cleanup here, but just report for now
            return false;
        }
    }

    true
}
