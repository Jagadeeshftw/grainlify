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
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::Config, &config);
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

    emit_circuit_event(env, symbol_short!("cb_fail"), failures);

    // Open circuit if threshold exceeded
    if failures >= config.failure_threshold {
        open_circuit(env);
    }
}

/// Transitions the circuit to **Open** state.
pub fn open_circuit(env: &Env) {
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

    emit_circuit_event(env, symbol_short!("cb_open"), get_failure_count(env));
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

    emit_circuit_event(env, symbol_short!("cb_close"), 0);
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
}

impl RetryConfig {
    pub fn default() -> Self {
        RetryConfig { max_attempts: 3 }
    }
}

/// Result of a retry operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryResult {
    pub succeeded: bool,
    pub attempts: u32,
    pub final_error: u32, // ERR_NONE if succeeded
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

    for _ in 0..config.max_attempts {
        // Check circuit before each attempt
        if let Err(e) = check_and_allow(env) {
            return RetryResult {
                succeeded: false,
                attempts,
                final_error: e,
            };
        }

        attempts += 1;
        match op() {
            Ok(()) => {
                record_success(env);
                return RetryResult {
                    succeeded: true,
                    attempts,
                    final_error: ERR_NONE,
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
