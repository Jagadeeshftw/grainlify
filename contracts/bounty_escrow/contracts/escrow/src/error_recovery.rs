// Re-export the error recovery module from program-escrow
// This ensures consistency across all contracts

// Note: In a real implementation, this would be a shared crate
// For now, we duplicate the implementation to maintain independence

// Copy the exact same implementation from program-escrow/src/error_recovery.rs
// This is intentionally duplicated to keep contracts independent
// In production, extract to a shared library crate

// For this implementation, we'll use a module alias approach
pub use crate::recovery_impl::*;

mod recovery_impl {
    // Include the full error_recovery implementation here
    // This is a placeholder - in production, use a shared crate
    
    use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol, Vec};

    #[contracttype]
    #[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
    #[repr(u32)]
    pub enum RecoveryError {
        NetworkTimeout = 100,
        TemporaryUnavailable = 101,
        RateLimitExceeded = 102,
        ResourceExhausted = 103,
        InsufficientFunds = 200,
        InvalidRecipient = 201,
        Unauthorized = 202,
        InvalidAmount = 203,
        ProgramNotFound = 204,
        PartialBatchFailure = 300,
        AllBatchItemsFailed = 301,
        BatchSizeMismatch = 302,
        MaxRetriesExceeded = 400,
        RecoveryInProgress = 401,
        CircuitBreakerOpen = 402,
        InvalidRetryConfig = 403,
    }

    #[contracttype]
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub enum ErrorClass {
        Transient,
        Permanent,
        Partial,
    }

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
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct BatchResult {
        pub total_items: u32,
        pub successful: u32,
        pub failed: u32,
        pub failed_indices: Vec<u32>,
    }

    impl BatchResult {
        pub fn new(env: &Env, total_items: u32) -> Self {
            Self {
                total_items,
                successful: 0,
                failed: 0,
                failed_indices: Vec::new(env),
            }
        }

        pub fn record_success(&mut self) {
            self.successful = self.successful.saturating_add(1);
        }

        pub fn record_failure(&mut self, index: u32) {
            self.failed = self.failed.saturating_add(1);
            self.failed_indices.push_back(index);
        }

        pub fn is_full_success(&self) -> bool {
            self.failed == 0
        }

        pub fn is_partial_success(&self) -> bool {
            self.successful > 0 && self.failed > 0
        }
    }

    #[contracttype]
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub enum CircuitState {
        Closed = 0,
        Open = 1,
        HalfOpen = 2,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct CircuitBreaker {
        pub state: CircuitState,
        pub failure_count: u32,
        pub failure_threshold: u32,
        pub timeout_duration: u64,
        pub last_failure_time: u64,
    }

    impl CircuitBreaker {
        pub fn new(env: &Env) -> Self {
            Self {
                state: CircuitState::Closed,
                failure_count: 0,
                failure_threshold: 5,
                timeout_duration: 60,
                last_failure_time: 0,
            }
        }

        pub fn record_success(&mut self, _env: &Env) {
            self.failure_count = 0;
            if self.state == CircuitState::HalfOpen {
                self.state = CircuitState::Closed;
            }
        }

        pub fn record_failure(&mut self, env: &Env) {
            self.failure_count = self.failure_count.saturating_add(1);
            self.last_failure_time = env.ledger().timestamp();
            
            if self.failure_count >= self.failure_threshold {
                self.state = CircuitState::Open;
            }
        }

        pub fn is_request_allowed(&mut self, env: &Env) -> bool {
            match self.state {
                CircuitState::Closed => true,
                CircuitState::Open => {
                    let now = env.ledger().timestamp();
                    if now.saturating_sub(self.last_failure_time) >= self.timeout_duration {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                }
                CircuitState::HalfOpen => true,
            }
        }
    }

    pub const ERROR_OCCURRED: Symbol = symbol_short!("err_occur");
    pub const RETRY_ATTEMPTED: Symbol = symbol_short!("retry");
    pub const RECOVERY_SUCCESS: Symbol = symbol_short!("recovered");
    pub const BATCH_PARTIAL: Symbol = symbol_short!("batch_part");

    pub fn emit_error_event(env: &Env, operation_id: u64, error: RecoveryError, caller: Address) {
        env.events().publish(
            (ERROR_OCCURRED, operation_id),
            (error as u32, caller, env.ledger().timestamp()),
        );
    }

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
}
