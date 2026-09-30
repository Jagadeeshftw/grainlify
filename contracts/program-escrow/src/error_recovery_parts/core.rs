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

use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol, Vec};

/// Maximum number of full failure records retained in hot contract storage.
/// Older records are moved into compact per-program archives.
pub const MAX_FAILURE_LOG_SIZE: u32 = 50;

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
    /// Per-program compact archive of pruned failure timestamps
    ErrorArchive(String),
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
    /// Recovery window in ledger timestamps (seconds) after which Open transitions to HalfOpen.
    pub recovery_window: u64,
}

impl CircuitBreakerConfig {
    pub fn default() -> Self {
        CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 1,
            max_error_log: MAX_FAILURE_LOG_SIZE,
            recovery_window: 300, // 5 minutes default recovery window
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

/// Compact archive for failure records pruned out of the hot error log.
///
/// Timestamps are delta-packed as u32 offsets from `timestamp_base`. This keeps
/// the recent log reviewable while preserving old failure timing at lower cost.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompactFailureArchive {
    pub program_id: String,
    pub archived_count: u32,
    pub timestamp_base: u64,
    pub timestamp_offsets: Vec<u32>,
    pub error_codes: Vec<u32>,
    pub failure_counts: Vec<u32>,
    pub first_timestamp: u64,
    pub last_timestamp: u64,
    pub overflow_count: u32,
    pub last_archived_at: u64,
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
    pub recovery_window: u64,
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
    let normalized = CircuitBreakerConfig {
        failure_threshold: config.failure_threshold,
        success_threshold: config.success_threshold,
        max_error_log: normalize_error_log_limit(config.max_error_log),
        recovery_window: config.recovery_window,
    };
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::Config, &normalized);

    // Emit audit event for config change
    env.events().publish(
        (symbol_short!("circuit"), symbol_short!("cb_cfg")),
        (
            prev_config.failure_threshold,
            normalized.failure_threshold,
            prev_config.success_threshold,
            normalized.success_threshold,
            prev_config.recovery_window,
            normalized.recovery_window,
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
        recovery_window: config.recovery_window,
    }
}

/// **Call this before any protected operation.**
///
/// Returns `Err(ERR_CIRCUIT_OPEN)` if the circuit is Open.
/// Automatically transitions Open → HalfOpen if recovery_window has elapsed.
/// Records that we are attempting an operation (no state change yet).
pub fn check_and_allow(env: &Env) -> Result<(), u32> {
    // Check for automatic timeout transitions first
    check_timeout_transitions(env);

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

/// Checks for automatic timeout transitions and applies them if needed.
///
/// - Open → HalfOpen: After recovery_window has elapsed since opened_at
/// - HalfOpen → Closed: Automatically after first successful probe operation (handled in record_success)
pub fn check_timeout_transitions(env: &Env) {
    let state = get_state(env);
    let now = env.ledger().timestamp();

    match state {
        CircuitState::Open => {
            let opened_at: u64 = env
                .storage()
                .persistent()
                .get(&CircuitBreakerKey::OpenedAt)
                .unwrap_or(0);

            if opened_at > 0 {
                let config = get_config(env);
                if now >= opened_at + config.recovery_window {
                    // Recovery window has elapsed - transition to HalfOpen
                    transition_to_half_open_timeout(env);
                }
            }
        }
        CircuitState::HalfOpen | CircuitState::Closed => {
            // No automatic transitions needed for these states
        }
    }
}

/// Transitions the circuit from Open to HalfOpen due to timeout.
/// This is an internal function called by check_timeout_transitions.
fn transition_to_half_open_timeout(env: &Env) {
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::State, &CircuitState::HalfOpen);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::SuccessCount, &0u32);

// Emit event indicating automatic timeout transition
    env.events().publish(
        (symbol_short!("circuit"), Symbol::new(env, "cb_timeout")),
        (symbol_short!("auto_half"), env.ledger().timestamp()),
    );
}

/// **Call this after a FAILED protected operation.**
///
/// Increments the failure counter and opens the circuit if the threshold
/// is exceeded. Records error log entry.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `program_id` - Program identifier for logging
/// * `operation` - Operation that failed
/// * `error_code` - Error code that occurred
/// * `threshold_override` - Optional per-program threshold. If None, uses global config.
pub fn record_failure(
    env: &Env,
    program_id: String,
    operation: soroban_sdk::Symbol,
    error_code: u32,
    threshold_override: Option<u32>,
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
        program_id: program_id.clone(),
        error_code,
        timestamp: now,
        failure_count_at_time: failures,
    };
    log.push_back(entry);

    prune_error_log(
        env,
        &mut log,
        normalize_error_log_limit(config.max_error_log),
    );
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

    // Use override if provided, otherwise use global config threshold
    let threshold = threshold_override.unwrap_or(config.failure_threshold);

    // Open circuit if threshold exceeded
    if failures >= threshold {
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

/// Archive all hot failure records for `program_id` into compact storage.
///
/// This is intended for admin cleanup of long-running programs. The circuit
/// admin must authorize the call.
pub fn archive_circuit_breaker_logs(env: &Env, program_id: String) -> CompactFailureArchive {
    let admin = get_circuit_admin(env).expect("Circuit admin not set");
    admin.require_auth();

    let mut log = get_error_log(env);
    let mut retained = Vec::new(env);
    let mut archived = Vec::new(env);

    while log.len() > 0 {
        let entry = log.get(0).unwrap();
        log.remove(0);
        if entry.program_id == program_id {
            archived.push_back(entry);
        } else {
            retained.push_back(entry);
        }
    }

    archive_entries(env, program_id.clone(), &archived);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::ErrorLog, &retained);

    let archive = get_failure_archive(env, program_id.clone());
    env.events().publish(
        (symbol_short!("circuit"), symbol_short!("cb_arch")),
        (
            program_id,
            archived.len(),
            archive.archived_count,
            env.ledger().timestamp(),
        ),
    );
    archive
}
