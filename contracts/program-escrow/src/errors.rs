//! # Canonical Error Enum for Program Escrow
//!
//! This module defines the single canonical error enum for all public program-escrow
//! entrypoints. Clients should use this enum to parse failures consistently.
//!
//! ## Error Code Ranges
//!
//! - **1-99**: General errors (authorization, validation, state)
//! - **100-199**: Program management errors
//! - **200-299**: Fund operation errors
//! - **300-399**: Payout errors
//! - **400-499**: Schedule errors
//! - **500-599**: Claim errors
//! - **600-699**: Dispute errors
//! - **700-799**: Fee errors
//! - **800-899**: Circuit breaker errors
//! - **900-999**: Threshold monitoring errors
//! - **1000-1099**: Batch recovery errors
//!
//! ## Security Notes
//!
//! - Error messages do NOT contain sensitive data (addresses, amounts, etc.)
//! - Error codes are stable and documented for client integration
//! - All errors are deterministic and reproducible

use soroban_sdk::contracterror;

/// Canonical error enum for all public program-escrow entrypoints.
///
/// This enum consolidates all possible errors that can be returned by the
/// program-escrow contract. Clients should match on these error codes to
/// handle failures consistently.
///
/// # Error Code Stability
///
/// Error codes are stable and will not change without a major version bump.
/// New error variants may be added in future versions, but existing codes
/// will remain unchanged.
///
/// # Security
///
/// Error messages are intentionally generic and do not contain sensitive data
/// such as addresses, amounts, or internal state. This prevents information
/// leakage through error channels.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    // =========================================================================
    // General Errors (1-99)
    // =========================================================================
    
    /// Caller is not authorized to perform this operation.
    ///
    /// This error occurs when:
    /// - Caller is not the admin
    /// - Caller is not the authorized payout key
    /// - Caller does not have required permissions
    Unauthorized = 1,
    
    /// Invalid amount provided.
    ///
    /// This error occurs when:
    /// - Amount is zero or negative
    /// - Amount exceeds maximum allowed value
    /// - Amount causes overflow in calculations
    InvalidAmount = 2,
    
    /// Contract is paused and operation is not allowed.
    ///
    /// This error occurs when:
    /// - Lock operations are paused
    /// - Release operations are paused
    /// - Refund operations are paused
    Paused = 3,
    
    /// Contract is in maintenance mode.
    ///
    /// This error occurs when the contract is in maintenance mode
    /// and the operation is not allowed during maintenance.
    MaintenanceMode = 4,
    
    /// Contract is in read-only mode.
    ///
    /// This error occurs when the contract is in read-only mode
    /// and the operation requires write access.
    ReadOnlyMode = 5,
    
    /// Invalid program ID provided.
    ///
    /// This error occurs when:
    /// - Program ID is empty
    /// - Program ID exceeds maximum length
    /// - Program ID contains invalid characters
    InvalidProgramId = 6,
    
    /// Program not found.
    ///
    /// This error occurs when attempting to access a program
    /// that does not exist in storage.
    ProgramNotFound = 7,
    
    /// Program already exists.
    ///
    /// This error occurs when attempting to initialize a program
    /// with an ID that is already registered.
    ProgramAlreadyExists = 8,
    
    /// Program is archived and cannot be modified.
    ///
    /// This error occurs when attempting to modify an archived program.
    ProgramArchived = 9,
    
    /// Insufficient balance for operation.
    ///
    /// This error occurs when the program's remaining balance
    /// is insufficient for the requested operation.
    InsufficientBalance = 10,
    
    /// Arithmetic overflow occurred.
    ///
    /// This error occurs when a calculation would overflow
    /// the maximum value for the type.
    Overflow = 11,
    
    /// Arithmetic underflow occurred.
    ///
    /// This error occurs when a calculation would underflow
    /// the minimum value for the type.
    Underflow = 12,
    
    /// Invalid address provided.
    ///
    /// This error occurs when an address is invalid or
    /// does not meet the required format.
    InvalidAddress = 13,
    
    /// Operation not allowed in current state.
    ///
    /// This error occurs when the operation is not allowed
    /// given the current state of the contract or program.
    InvalidState = 14,
    
    /// Duplicate entry detected.
    ///
    /// This error occurs when attempting to create a duplicate
    /// entry where uniqueness is required.
    DuplicateEntry = 15,
    
    /// Entry not found.
    ///
    /// This error occurs when attempting to access an entry
    /// that does not exist.
    EntryNotFound = 16,
    
    /// Invalid configuration provided.
    ///
    /// This error occurs when configuration parameters
    /// are invalid or out of acceptable range.
    InvalidConfig = 17,
    
    /// Rate limit exceeded.
    ///
    /// This error occurs when the caller has exceeded
    /// the allowed rate for operations.
    RateLimitExceeded = 18,

    /// Pagination limit is zero.
    ///
    /// This error occurs when a pagination query is called with `limit = 0`.
    InvalidPaginationLimit = 19,

    /// Pagination limit exceeds the configured maximum.
    ///
    /// This error occurs when `limit > HistoryPaginationConfig::max_limit`.
    PaginationLimitExceedsMax = 20,

    // =========================================================================
    // Program Management Errors (100-199)
    // =========================================================================
    
    /// Program initialization failed.
    ///
    /// This error occurs when program initialization fails
    /// due to invalid parameters or system state.
    ProgramInitFailed = 100,
    
    /// Program has no authorized payout key.
    ///
    /// This error occurs when attempting to perform payout
    /// operations without an authorized payout key set.
    NoAuthorizedPayoutKey = 101,
    
    /// Program delegate not set.
    ///
    /// This error occurs when attempting to use a delegate
    /// that has not been set for the program.
    DelegateNotSet = 102,
    
    /// Program delegate permissions insufficient.
    ///
    /// This error occurs when the delegate does not have
    /// the required permissions for the operation.
    DelegatePermissionsInsufficient = 103,
    
    /// Program metadata update failed.
    ///
    /// This error occurs when metadata update fails
    /// due to invalid data or constraints.
    MetadataUpdateFailed = 104,
    
    /// Program risk flags update failed.
    ///
    /// This error occurs when risk flags update fails
    /// due to invalid flags or permissions.
    RiskFlagsUpdateFailed = 105,
    
    /// Cannot archive program with pending operations.
    ///
    /// This error occurs when attempting to archive a program
    /// that has pending claims or scheduled releases.
    CannotArchiveWithPendingOps = 106,
    
    /// Program is not in Active status.
    ///
    /// This error occurs when attempting to perform operations
    /// that require the program to be in Active status (e.g., refunds).
    /// Programs must be published via publish_program() before these operations.
    ProgramNotActive = 107,
    
    // =========================================================================
    // Fund Operation Errors (200-299)
    // =========================================================================
    
    /// Fund locking failed.
    ///
    /// This error occurs when fund locking fails due to
    /// insufficient balance, token transfer issues, or fees.
    FundLockFailed = 200,
    
    /// Fund release failed.
    ///
    /// This error occurs when fund release fails due to
    /// insufficient balance or token transfer issues.
    FundReleaseFailed = 201,
    
    /// Fund refund failed.
    ///
    /// This error occurs when fund refund fails due to
    /// insufficient balance or token transfer issues.
    FundRefundFailed = 202,
    
    /// Token transfer failed.
    ///
    /// This error occurs when the underlying token transfer
    /// fails for any reason.
    TokenTransferFailed = 203,
    
    /// Lock fee exceeds amount.
    ///
    /// This error occurs when the lock fee would consume
    /// the entire lock amount.
    LockFeeExceedsAmount = 204,
    
    /// Payout fee exceeds amount.
    ///
    /// This error occurs when the payout fee would consume
    /// the entire payout amount.
    PayoutFeeExceedsAmount = 205,
    
    /// Emergency withdraw failed.
    ///
    /// This error occurs when emergency withdraw fails
    /// due to insufficient balance or permissions.
    EmergencyWithdrawFailed = 206,
    
    // =========================================================================
    // Payout Errors (300-399)
    // =========================================================================
    
    /// Payout failed.
    ///
    /// This error occurs when a single payout fails for
    /// any reason (insufficient balance, transfer failure, etc.).
    PayoutFailed = 300,
    
    /// Batch payout failed.
    ///
    /// This error occurs when a batch payout operation fails.
    BatchPayoutFailed = 301,
    
    /// Invalid batch size.
    ///
    /// This error occurs when the batch size exceeds the
    /// maximum allowed or is zero.
    InvalidBatchSize = 302,
    
    /// Batch contains duplicate recipients.
    ///
    /// This error occurs when a batch contains duplicate
    /// recipient addresses.
    DuplicateRecipients = 303,
    
    /// Batch amounts mismatch.
    ///
    /// This error occurs when the number of amounts does
    /// not match the number of recipients in a batch.
    BatchAmountsMismatch = 304,
    
    /// Split payout configuration not set.
    ///
    /// This error occurs when attempting to execute a split
    /// payout without configuring splits first.
    SplitConfigNotSet = 305,
    
    /// Split payout configuration disabled.
    ///
    /// This error occurs when attempting to execute a split
    /// payout with a disabled configuration.
    SplitConfigDisabled = 306,
    
    /// Split shares do not sum to 100%.
    ///
    /// This error occurs when split shares do not sum to
    /// exactly 10,000 basis points (100%).
    InvalidSplitShares = 307,
    
    /// Split payout execution failed.
    ///
    /// This error occurs when split payout execution fails
    /// due to insufficient balance or transfer issues.
    SplitPayoutFailed = 308,
    
    // =========================================================================
    // Schedule Errors (400-499)
    // =========================================================================
    
    /// Schedule not found.
    ///
    /// This error occurs when attempting to access a release
    /// schedule that does not exist.
    ScheduleNotFound = 400,
    
    /// Schedule already released.
    ///
    /// This error occurs when attempting to release a schedule
    /// that has already been released.
    ScheduleAlreadyReleased = 401,
    
    /// Schedule not yet due.
    ///
    /// This error occurs when attempting to release a schedule
    /// before its release timestamp.
    ScheduleNotDue = 402,
    
    /// Schedule creation failed.
    ///
    /// This error occurs when schedule creation fails due to
    /// invalid parameters or system state.
    ScheduleCreationFailed = 403,
    
    /// Invalid schedule parameters.
    ///
    /// This error occurs when schedule parameters are invalid
    /// (e.g., release timestamp in the past).
    InvalidScheduleParams = 404,
    
    /// Schedule release failed.
    ///
    /// This error occurs when schedule release fails due to
    /// insufficient balance or transfer issues.
    ScheduleReleaseFailed = 405,
    
    /// Maximum schedules exceeded.
    ///
    /// This error occurs when attempting to create more schedules
    /// than the maximum allowed.
    MaxSchedulesExceeded = 406,
    
    // =========================================================================
    // Claim Errors (500-599)
    // =========================================================================
    
    /// Claim not found.
    ///
    /// This error occurs when attempting to access a claim
    /// that does not exist.
    ClaimNotFound = 500,
    
    /// Claim already executed.
    ///
    /// This error occurs when attempting to execute a claim
    /// that has already been executed.
    ClaimAlreadyExecuted = 501,
    
    /// Claim expired.
    ///
    /// This error occurs when attempting to execute a claim
    /// after its expiration time.
    ClaimExpired = 502,
    
    /// Claim creation failed.
    ///
    /// This error occurs when claim creation fails due to
    /// invalid parameters or system state.
    ClaimCreationFailed = 503,
    
    /// Claim execution failed.
    ///
    /// This error occurs when claim execution fails due to
    /// insufficient balance or transfer issues.
    ClaimExecutionFailed = 504,
    
    /// Claim cancellation failed.
    ///
    /// This error occurs when claim cancellation fails due to
    /// permissions or state issues.
    ClaimCancellationFailed = 505,
    
    /// Invalid claim window.
    ///
    /// This error occurs when the claim window configuration
    /// is invalid or out of acceptable range.
    InvalidClaimWindow = 506,
    
    // =========================================================================
    // Dispute Errors (600-699)
    // =========================================================================
    
    /// Dispute already open.
    ///
    /// This error occurs when attempting to open a dispute
    /// when one is already open.
    DisputeAlreadyOpen = 600,
    
    /// No active dispute.
    ///
    /// This error occurs when attempting to resolve a dispute
    /// when none is open.
    NoActiveDispute = 601,
    
    /// Dispute resolution failed.
    ///
    /// This error occurs when dispute resolution fails
    /// due to permissions or state issues.
    DisputeResolutionFailed = 602,
    
    /// Dispute opening failed.
    ///
    /// This error occurs when dispute opening fails
    /// due to permissions or state issues.
    DisputeOpeningFailed = 603,
    
    // =========================================================================
    // Fee Errors (700-799)
    // =========================================================================
    
    /// Fee configuration update failed.
    ///
    /// This error occurs when fee configuration update fails
    /// due to invalid parameters or permissions.
    FeeConfigUpdateFailed = 700,
    
    /// Invalid fee rate.
    ///
    /// This error occurs when the fee rate exceeds the
    /// maximum allowed or is negative.
    InvalidFeeRate = 701,
    
    /// Fee recipient not set.
    ///
    /// This error occurs when attempting to collect fees
    /// without setting a fee recipient.
    FeeRecipientNotSet = 702,
    
    /// Fee collection failed.
    ///
    /// This error occurs when fee collection fails due to
    /// insufficient balance or transfer issues.
    FeeCollectionFailed = 703,
    
    // =========================================================================
    // Circuit Breaker Errors (800-899)
    // =========================================================================
    
    /// Circuit breaker is open.
    ///
    /// This error occurs when the circuit breaker is open
    /// and the operation is blocked.
    CircuitBreakerOpen = 800,
    
    /// Circuit breaker configuration failed.
    ///
    /// This error occurs when circuit breaker configuration
    /// fails due to invalid parameters.
    CircuitBreakerConfigFailed = 801,
    
    /// Circuit breaker reset failed.
    ///
    /// This error occurs when circuit breaker reset fails
    /// due to permissions or state issues.
    CircuitBreakerResetFailed = 802,
    
    /// Circuit breaker admin not set.
    ///
    /// This error occurs when attempting to manage the circuit
    /// breaker without setting an admin.
    CircuitBreakerAdminNotSet = 803,
    
    // =========================================================================
    // Threshold Monitoring Errors (900-999)
    // =========================================================================
    
    /// Threshold breached.
    ///
    /// This error occurs when an operation would breach
    /// the configured threshold limits.
    ThresholdBreached = 900,
    
    /// Invalid threshold configuration.
    ///
    /// This error occurs when threshold configuration
    /// is invalid or out of acceptable range.
    InvalidThresholdConfig = 901,
    
    /// Cooldown active.
    ///
    /// This error occurs when an operation is attempted
    /// during the cooldown period after a threshold breach.
    CooldownActive = 902,
    
    /// Threshold window not expired.
    ///
    /// This error occurs when attempting to reset metrics
    /// before the current window expires.
    ThresholdWindowNotExpired = 903,
    
    /// Spend limit exceeded.
    ///
    /// This error occurs when a payout operation (single or batch)
    /// would exceed the configured per-program spend threshold.
    SpendLimitExceeded = 904,
    
    // =========================================================================
    // Release Trigger Errors (905-909)
    // =========================================================================
    
    /// Release trigger encountered a critical error.
    ///
    /// This error occurs when the release trigger encounters
    /// unrecoverable internal state corruption or determinism violation.
    ReleaseTriggerFailed = 905,
    
    /// No schedules were due for processing.
    ///
    /// This error occurs when trigger_program_releases is called
    /// but no schedules meet the release timestamp threshold.
    NoSchedulesDue = 906,
    
    /// Determinism violation detected during trigger.
    ///
    /// This error occurs when the trigger execution detects
    /// a violation of the deterministic ordering guarantee.
    DeterminismViolation = 907,
    
    // =========================================================================
    // Batch Recovery Errors (1000-1099)
    // =========================================================================
    
    /// Batch not found.
    ///
    /// This error occurs when attempting to access a batch
    /// that does not exist.
    BatchNotFound = 1000,
    
    /// Batch already complete.
    ///
    /// This error occurs when attempting to modify a batch
    /// that has already been completed.
    BatchAlreadyComplete = 1001,
    
    /// Batch not recoverable.
    ///
    /// This error occurs when attempting to recover a batch
    /// that is not in a recoverable state.
    BatchNotRecoverable = 1002,
    
    /// Unauthorized batch recovery.
    ///
    /// This error occurs when attempting batch recovery
    /// without proper authorization.
    UnauthorizedBatchRecovery = 1003,
    
    /// Batch size exceeded.
    ///
    /// This error occurs when the batch size exceeds the
    /// maximum allowed for recovery operations.
    BatchSizeExceeded = 1004,
    
    /// Batch recovery expired.
    ///
    /// This error occurs when attempting to recover a batch
    /// after the recovery window has expired.
    BatchRecoveryExpired = 1005,
    
    /// Rollback disabled.
    ///
    /// This error occurs when attempting to rollback a batch
    /// when rollback is disabled.
    RollbackDisabled = 1006,
    
    /// No failed items in batch.
    ///
    /// This error occurs when attempting to retry failed items
    /// but none have failed.
    NoFailedItems = 1007,
    
    /// No successful items in batch.
    ///
    /// This error occurs when attempting to rollback successful
    /// items but none have succeeded.
    NoSuccessfulItems = 1008,
    
    /// Invalid batch configuration.
    ///
    /// This error occurs when batch recovery configuration
    /// is invalid or out of acceptable range.
    InvalidBatchConfig = 1009,
    
    /// Batch item not found.
    ///
    /// This error occurs when attempting to access a batch
    /// item that does not exist.
    BatchItemNotFound = 1010,
    
    /// Batch item already processed.
    ///
    /// This error occurs when attempting to process a batch
    /// item that has already been processed.
    BatchItemAlreadyProcessed = 1011,
    
    /// Maximum retries exceeded.
    ///
    /// This error occurs when a batch item has exceeded
    /// the maximum number of retry attempts.
    MaxRetriesExceeded = 1012,

    // =========================================================================
    // Token Allowlist Errors (1100-1199)
    // =========================================================================

    TokenNotAllowed = 1100,
    TokenAlreadyAllowed = 1101,
    TokenNotInAllowlist = 1102,

    // =================================================================
    // =========================================================================
    // Multisig Admin Op Errors (1300-1399)
    // =========================================================================

    AdminRotationInProgress = 1200,
    NoAdminRotationInProgress = 1201,
    InvalidAdminRotationState = 1202,
    ControllerRotationInProgress = 1203,
    NoControllerRotationInProgress = 1204,
    InvalidControllerRotationState = 1205,
    RoleTransitionExpired = 1206,
    InvalidRoleProposal = 1207,
    RoleRotationNotAllowed = 1208,

    /// Rotation timelock has not yet expired.
    ///
    /// This error occurs when `accept_admin` or `accept_controller` is called
    /// before the mandatory 24-hour delay since the proposal has elapsed.
    /// The caller must wait until `proposed_at + ROTATION_TIMELOCK_DELAY` seconds
    /// have passed before accepting the role.
    RotationTimelockActive = 1209,
}

#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BatchPayoutError {
    NotInitialized = 3100,
    Paused = 3101,
    DisputeOpen = 3102,
    Unauthorized = 3103,
    LengthMismatch = 3104,
    EmptyBatch = 3105,
    ZeroAmount = 3106,
    AmountOverflow = 3107,
    SpendLimitExceeded = 3108,
    InsufficientBalance = 3109,
    CircuitBreakerOpen = 3110,
    DuplicateRecipient = 3111,
    FeeConsumesAmount = 3112,
}

impl BatchPayoutError {
    pub fn description(self) -> &'static str {
        match self {
            BatchPayoutError::NotInitialized => "Program not initialized",
            BatchPayoutError::Paused => "Funds Paused",
            BatchPayoutError::DisputeOpen => "Payout blocked: dispute open",
            BatchPayoutError::Unauthorized => "Unauthorized",
            BatchPayoutError::LengthMismatch => "Recipients and amounts vectors must have the same length",
            BatchPayoutError::EmptyBatch => "Cannot process empty batch",
            BatchPayoutError::ZeroAmount => "All amounts must be greater than zero",
            BatchPayoutError::AmountOverflow => "Payout amount overflow",
            BatchPayoutError::SpendLimitExceeded => "Spend threshold exceeded",
            BatchPayoutError::InsufficientBalance => "Insufficient balance",
            BatchPayoutError::CircuitBreakerOpen => "Circuit breaker is OPEN",
            BatchPayoutError::DuplicateRecipient => "Duplicate recipient in batch",
            BatchPayoutError::FeeConsumesAmount => "Payout fee consumes entire payout",
        }
    }

impl ContractError {
    /// Returns a human-readable description of the error.
    ///
    /// This is intended for debugging and logging purposes only.
    /// Clients should use the error code for programmatic handling.
    ///
    /// # Security Note
    ///
    /// Descriptions are intentionally generic and do not contain
    /// sensitive data such as addresses, amounts, or internal state.
    pub fn description(&self) -> &'static str {
        match self {
            // General Errors
            ContractError::Unauthorized => "Caller is not authorized",
            ContractError::InvalidAmount => "Invalid amount provided",
            ContractError::Paused => "Contract is paused",
            ContractError::MaintenanceMode => "Contract is in maintenance mode",
            ContractError::ReadOnlyMode => "Contract is in read-only mode",
            ContractError::InvalidProgramId => "Invalid program ID",
            ContractError::ProgramNotFound => "Program not found",
            ContractError::ProgramAlreadyExists => "Program already exists",
            ContractError::ProgramArchived => "Program is archived",
            ContractError::InsufficientBalance => "Insufficient balance",
            ContractError::Overflow => "Arithmetic overflow",
            ContractError::Underflow => "Arithmetic underflow",
            ContractError::InvalidAddress => "Invalid address",
            ContractError::InvalidState => "Invalid state for operation",
            ContractError::DuplicateEntry => "Duplicate entry",
            ContractError::EntryNotFound => "Entry not found",
            ContractError::InvalidConfig => "Invalid configuration",
            ContractError::RateLimitExceeded => "Rate limit exceeded",
            ContractError::InvalidPaginationLimit => "Pagination limit must be greater than zero",
            ContractError::PaginationLimitExceedsMax => "Pagination limit exceeds maximum",
            
            // Program Management Errors
            ContractError::ProgramInitFailed => "Program initialization failed",
            ContractError::NoAuthorizedPayoutKey => "No authorized payout key",
            ContractError::DelegateNotSet => "Delegate not set",
            ContractError::DelegatePermissionsInsufficient => "Delegate permissions insufficient",
            ContractError::MetadataUpdateFailed => "Metadata update failed",
            ContractError::RiskFlagsUpdateFailed => "Risk flags update failed",
            ContractError::CannotArchiveWithPendingOps => "Cannot archive with pending operations",
            
            // Fund Operation Errors
            ContractError::FundLockFailed => "Fund locking failed",
            ContractError::FundReleaseFailed => "Fund release failed",
            ContractError::FundRefundFailed => "Fund refund failed",
            ContractError::TokenTransferFailed => "Token transfer failed",
            ContractError::LockFeeExceedsAmount => "Lock fee exceeds amount",
            ContractError::PayoutFeeExceedsAmount => "Payout fee exceeds amount",
            ContractError::EmergencyWithdrawFailed => "Emergency withdraw failed",
            
            // Payout Errors
            ContractError::PayoutFailed => "Payout failed",
            ContractError::BatchPayoutFailed => "Batch payout failed",
            ContractError::InvalidBatchSize => "Invalid batch size",
            ContractError::DuplicateRecipients => "Duplicate recipients in batch",
            ContractError::BatchAmountsMismatch => "Batch amounts mismatch",
            ContractError::SplitConfigNotSet => "Split configuration not set",
            ContractError::SplitConfigDisabled => "Split configuration disabled",
            ContractError::InvalidSplitShares => "Invalid split shares",
            ContractError::SplitPayoutFailed => "Split payout failed",
            
            // Schedule Errors
            ContractError::ScheduleNotFound => "Schedule not found",
            ContractError::ScheduleAlreadyReleased => "Schedule already released",
            ContractError::ScheduleNotDue => "Schedule not yet due",
            ContractError::ScheduleCreationFailed => "Schedule creation failed",
            ContractError::InvalidScheduleParams => "Invalid schedule parameters",
            ContractError::ScheduleReleaseFailed => "Schedule release failed",
            ContractError::MaxSchedulesExceeded => "Maximum schedules exceeded",
            
            // Claim Errors
            ContractError::ClaimNotFound => "Claim not found",
            ContractError::ClaimAlreadyExecuted => "Claim already executed",
            ContractError::ClaimExpired => "Claim expired",
            ContractError::ClaimCreationFailed => "Claim creation failed",
            ContractError::ClaimExecutionFailed => "Claim execution failed",
            ContractError::ClaimCancellationFailed => "Claim cancellation failed",
            ContractError::InvalidClaimWindow => "Invalid claim window",
            
            // Dispute Errors
            ContractError::DisputeAlreadyOpen => "Dispute already open",
            ContractError::NoActiveDispute => "No active dispute",
            ContractError::DisputeResolutionFailed => "Dispute resolution failed",
            ContractError::DisputeOpeningFailed => "Dispute opening failed",
            
            // Fee Errors
            ContractError::FeeConfigUpdateFailed => "Fee configuration update failed",
            ContractError::InvalidFeeRate => "Invalid fee rate",
            ContractError::FeeRecipientNotSet => "Fee recipient not set",
            ContractError::FeeCollectionFailed => "Fee collection failed",
            
            // Circuit Breaker Errors
            ContractError::CircuitBreakerOpen => "Circuit breaker is open",
            ContractError::CircuitBreakerConfigFailed => "Circuit breaker configuration failed",
            ContractError::CircuitBreakerResetFailed => "Circuit breaker reset failed",
            ContractError::CircuitBreakerAdminNotSet => "Circuit breaker admin not set",
            
            // Threshold Monitoring Errors
            ContractError::ThresholdBreached => "Threshold breached",
            ContractError::InvalidThresholdConfig => "Invalid threshold configuration",
            ContractError::CooldownActive => "Cooldown active",
            ContractError::ThresholdWindowNotExpired => "Threshold window not expired",
            ContractError::SpendLimitExceeded => "Spend limit exceeded",
            
            // Batch Recovery Errors
            ContractError::BatchNotFound => "Batch not found",
            ContractError::BatchAlreadyComplete => "Batch already complete",
            ContractError::BatchNotRecoverable => "Batch not recoverable",
            ContractError::UnauthorizedBatchRecovery => "Unauthorized batch recovery",
            ContractError::BatchSizeExceeded => "Batch size exceeded",
            ContractError::BatchRecoveryExpired => "Batch recovery expired",
            ContractError::RollbackDisabled => "Rollback disabled",
            ContractError::NoFailedItems => "No failed items",
            ContractError::NoSuccessfulItems => "No successful items",
            ContractError::InvalidBatchConfig => "Invalid batch configuration",
            ContractError::BatchItemNotFound => "Batch item not found",
            ContractError::BatchItemAlreadyProcessed => "Batch item already processed",
            ContractError::MaxRetriesExceeded => "Maximum retries exceeded",

            // Token Allowlist Errors
            ContractError::TokenNotAllowed => "Token is not on the allowlist",
            ContractError::TokenAlreadyAllowed => "Token is already on the allowlist",
            ContractError::TokenNotInAllowlist => "Token is not on the allowlist and cannot be removed",

            // Role Management Errors
            ContractError::AdminRotationInProgress => "Admin rotation already in progress",
            ContractError::NoAdminRotationInProgress => "No admin rotation in progress",
            ContractError::InvalidAdminRotationState => "Invalid admin rotation state",
            ContractError::ControllerRotationInProgress => "Controller rotation already in progress",
            ContractError::NoControllerRotationInProgress => "No controller rotation in progress",
            ContractError::InvalidControllerRotationState => "Invalid controller rotation state",
            ContractError::RoleTransitionExpired => "Role transition period expired",
            ContractError::InvalidRoleProposal => "Invalid role proposal",
            ContractError::RoleRotationNotAllowed => "Role rotation not allowed",

            // Idempotency Key Errors
            ContractError::DuplicateIdempotencyKey => "Idempotency key already used",
            ContractError::ExpiredIdempotencyKey => "Idempotency key has expired",
        }
    }
    
    /// Returns the error code as a u32.
    ///
    /// This is useful for logging and debugging purposes.
    pub fn code(&self) -> u32 {
        *self as u32
    }
}
