/// Return the compact failure archive for a program.
pub fn get_failure_archive(env: &Env, program_id: String) -> CompactFailureArchive {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::ErrorArchive(program_id.clone()))
        .unwrap_or(empty_failure_archive(env, program_id))
}

// ─────────────────────────────────────────────────────────
// Retry logic
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod circuit_log_archive_tests {
    use super::*;
    use crate::ProgramEscrowContract;
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Ledger},
        Address,
    };

    fn with_contract(env: &Env) -> soroban_sdk::Address {
        env.register_contract(None, ProgramEscrowContract)
    }

    #[test]
    fn record_failure_rotates_old_entries_into_compact_archive() {
        let env = Env::default();
        let contract_id = with_contract(&env);
        let program_id = String::from_str(&env, "program-a");

        env.as_contract(&contract_id, || {
            set_config(
                &env,
                CircuitBreakerConfig {
                    failure_threshold: 100,
                    success_threshold: 1,
                    max_error_log: 100,
                    recovery_window: 300,
                },
            );

            for i in 0..55u32 {
                env.ledger().set_timestamp(1_000 + i as u64);
                record_failure(&env, program_id.clone(), symbol_short!("payout"), 5_000 + i, None);
            }

            let log = get_error_log(&env);
            assert_eq!(log.len(), MAX_FAILURE_LOG_SIZE);
            assert_eq!(log.get(0).unwrap().timestamp, 1_005);

            let archive = get_failure_archive(&env, program_id.clone());
            assert_eq!(archive.archived_count, 5);
            assert_eq!(archive.timestamp_base, 1_000);
            assert_eq!(archive.timestamp_offsets.len(), 5);
            assert_eq!(archive.timestamp_offsets.get(0).unwrap(), 0);
            assert_eq!(archive.timestamp_offsets.get(4).unwrap(), 4);
            assert_eq!(archive.error_codes.get(4).unwrap(), 5_004);
            assert_eq!(archive.overflow_count, 0);
        });
    }

    #[test]
    fn admin_cleanup_archives_only_requested_program_logs() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = with_contract(&env);
        let admin = Address::generate(&env);
        let program_a = String::from_str(&env, "program-a");
        let program_b = String::from_str(&env, "program-b");

        env.as_contract(&contract_id, || {
            set_circuit_admin(&env, admin, None);

            env.ledger().set_timestamp(2_000);
            record_failure(&env, program_a.clone(), symbol_short!("payout"), 7, None);
            env.ledger().set_timestamp(2_001);
            record_failure(&env, program_b.clone(), symbol_short!("refund"), 8, None);
            env.ledger().set_timestamp(2_002);
            record_failure(&env, program_a.clone(), symbol_short!("payout"), 9, None);

            let archive = archive_circuit_breaker_logs(&env, program_a.clone());
            assert_eq!(archive.archived_count, 2);
            assert_eq!(archive.timestamp_offsets.get(0).unwrap(), 0);
            assert_eq!(archive.timestamp_offsets.get(1).unwrap(), 2);

            let log = get_error_log(&env);
            assert_eq!(log.len(), 1);
            let retained = log.get(0).unwrap();
            assert_eq!(retained.program_id, program_b);
            assert_eq!(retained.error_code, 8);
        });
    }
}

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
    threshold_override: Option<u32>,
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
                record_failure(env, program_id.clone(), operation.clone(), code, threshold_override);
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

fn normalize_error_log_limit(requested: u32) -> u32 {
    if requested == 0 {
        1
    } else if requested > MAX_FAILURE_LOG_SIZE {
        MAX_FAILURE_LOG_SIZE
    } else {
        requested
    }
}

fn prune_error_log(env: &Env, log: &mut Vec<ErrorEntry>, max_entries: u32) {
    while log.len() > max_entries {
        let entry = log.get(0).unwrap();
        log.remove(0);
        let mut archived = Vec::new(env);
        let program_id = entry.program_id.clone();
        archived.push_back(entry);
        archive_entries(env, program_id, &archived);
    }
}

fn archive_entries(env: &Env, program_id: String, entries: &Vec<ErrorEntry>) {
    if entries.len() == 0 {
        return;
    }

    let mut archive = get_failure_archive(env, program_id.clone());
    for entry in entries.iter() {
        append_archive_entry(env, &mut archive, &entry);
    }

    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::ErrorArchive(program_id), &archive);
}

fn append_archive_entry(env: &Env, archive: &mut CompactFailureArchive, entry: &ErrorEntry) {
    if archive.archived_count == 0 {
        archive.timestamp_base = entry.timestamp;
        archive.first_timestamp = entry.timestamp;
    }

    if entry.timestamp >= archive.timestamp_base {
        let delta = entry.timestamp - archive.timestamp_base;
        if delta <= u32::MAX as u64 {
            archive.timestamp_offsets.push_back(delta as u32);
        } else {
            archive.timestamp_offsets.push_back(u32::MAX);
            archive.overflow_count += 1;
        }
    } else {
        archive.timestamp_offsets.push_back(0);
        archive.overflow_count += 1;
    }

    archive.error_codes.push_back(entry.error_code);
    archive
        .failure_counts
        .push_back(entry.failure_count_at_time);
    archive.archived_count += 1;
    archive.last_timestamp = entry.timestamp;
    archive.last_archived_at = env.ledger().timestamp();
}

fn empty_failure_archive(env: &Env, program_id: String) -> CompactFailureArchive {
    CompactFailureArchive {
        program_id,
        archived_count: 0,
        timestamp_base: 0,
        timestamp_offsets: Vec::new(env),
        error_codes: Vec::new(env),
        failure_counts: Vec::new(env),
        first_timestamp: 0,
        last_timestamp: 0,
        overflow_count: 0,
        last_archived_at: 0,
    }
}

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
