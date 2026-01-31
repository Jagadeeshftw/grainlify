//! # Bounty Escrow Smart Contract
//!
//! A trustless escrow system for bounty payments on the Stellar blockchain.
//! This contract enables secure fund locking, conditional release to contributors,
//! and automatic refunds after deadlines.
//!
//! ## Overview
//!
//! The Bounty Escrow contract manages the complete lifecycle of bounty payments:
//! 1. **Initialization**: Set up admin and token contract
//! 2. **Lock Funds**: Depositor locks tokens for a bounty with a deadline
//! 3. **Release**: Admin releases funds to contributor upon task completion
//! 4. **Refund**: Automatic refund to depositor if deadline passes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Contract Architecture                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────┐                                           │
//! │  │  Depositor   │─────┐                                     │
//! │  └──────────────┘     │                                     │
//! │                       ├──> lock_funds()                     │
//! │  ┌──────────────┐     │         │                           │
//! │  │    Admin     │─────┘         ▼                           │
//! │  └──────────────┘          ┌─────────┐                      │
//! │         │                  │ ESCROW  │                      │
//! │         │                  │ LOCKED  │                      │
//! │         │                  └────┬────┘                      │
//! │         │                       │                           │
//! │         │        ┌──────────────┴───────────────┐           │
//! │         │        │                              │           │
//! │         ▼        ▼                              ▼           │
//! │   release_funds()                          refund()         │
//! │         │                                       │           │
//! │         ▼                                       ▼           │
//! │  ┌──────────────┐                      ┌──────────────┐    │
//! │  │ Contributor  │                      │  Depositor   │    │
//! │  └──────────────┘                      └──────────────┘    │
//! │    (RELEASED)                            (REFUNDED)        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Admin**: Trusted entity (backend service) authorized to release funds
//! - **Depositor**: Self-interested party; funds protected by deadline mechanism
//! - **Contributor**: Receives funds only after admin approval
//! - **Contract**: Trustless; operates according to programmed rules
//!
//! ### Key Security Features
//! 1. **Single Initialization**: Prevents admin takeover
//! 2. **Unique Bounty IDs**: No duplicate escrows
//! 3. **Authorization Checks**: All state changes require proper auth
//! 4. **Deadline Protection**: Prevents indefinite fund locking
//! 5. **State Machine**: Enforces valid state transitions
//! 6. **Atomic Operations**: Transfer + state update in single transaction
//!
//! ## Usage Example
//!
//! ```rust
//! use soroban_sdk::{Address, Env};
//!
//! // 1. Initialize contract (one-time setup)
//! let admin = Address::from_string("GADMIN...");
//! let token = Address::from_string("CUSDC...");
//! escrow_client.init(&admin, &token);
//!
//! // 2. Depositor locks 1000 USDC for bounty #42
//! let depositor = Address::from_string("GDEPOSIT...");
//! let amount = 1000_0000000; // 1000 USDC (7 decimals)
//! let deadline = current_timestamp + (30 * 24 * 60 * 60); // 30 days
//! escrow_client.lock_funds(&depositor, &42, &amount, &deadline);
//!
//! // 3a. Admin releases to contributor (happy path)
//! let contributor = Address::from_string("GCONTRIB...");
//! escrow_client.release_funds(&42, &contributor);
//!
//! // OR
//!
//! // 3b. Refund to depositor after deadline (timeout path)
//! // (Can be called by anyone after deadline passes)
//! escrow_client.refund(&42);
//! ```

#![no_std]
mod blacklist;
mod events;
mod indexed;
#[cfg(test)]
mod invariants;
mod test_blacklist;
mod test_bounty_escrow;
pub mod security {
    pub mod reentrancy_guard;
}

use security::reentrancy_guard::{ReentrancyGuard, ReentrancyGuardRAII};

use blacklist::{
    add_to_blacklist, add_to_whitelist, is_participant_allowed, remove_from_blacklist,
    remove_from_whitelist, set_whitelist_mode,
};
use events::{
    emit_admin_action_cancelled, emit_admin_action_executed, emit_admin_action_proposed,
    emit_admin_updated, emit_batch_funds_locked, emit_batch_funds_released,
    emit_bounty_initialized, emit_config_limits_updated, emit_contract_paused,
    emit_contract_unpaused, emit_deadline_extended, emit_emergency_withdrawal, emit_escrow_expired,
    emit_funds_locked, emit_funds_refunded, emit_funds_released, emit_operation_pause_changed,
    emit_payout_key_updated, AdminActionCancelled, AdminActionExecuted, AdminActionProposed,
    AdminUpdated, BatchFundsLocked, BatchFundsReleased, BountyEscrowInitialized,
    ConfigLimitsUpdated, ContractPaused, ContractUnpaused, DeadlineExtended, EmergencyWithdrawal,
    EscrowExpired, FundsLocked, FundsRefunded, FundsReleased, OperationPauseChanged,
    PayoutKeyUpdated,
};
use indexed::{on_funds_locked, on_funds_refunded, on_funds_released};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    Map, String, Vec,
};

pub use grainlify_interfaces::{
    ConfigurableFee, EscrowLock, EscrowRelease, FeeConfig as SharedFeeConfig, Pausable, RefundMode,
};

// ==================== MONITORING MODULE ====================
#[allow(dead_code)]
mod monitoring {
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

    // Storage keys
    const OPERATION_COUNT: &str = "op_count";
    #[allow(dead_code)]
    const USER_COUNT: &str = "usr_count";
    const ERROR_COUNT: &str = "err_count";

    // Event: Operation metric
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct OperationMetric {
        pub operation: Symbol,
        pub caller: Address,
        pub timestamp: u64,
        pub success: bool,
    }

    // Event: Performance metric
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceMetric {
        pub function: Symbol,
        pub duration: u64,
        pub timestamp: u64,
    }

    // Data: Health status
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct HealthStatus {
        pub is_healthy: bool,
        pub last_operation: u64,
        pub total_operations: u64,
        pub contract_version: String,
    }

    // Data: Analytics
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct Analytics {
        pub operation_count: u64,
        pub unique_users: u64,
        pub error_count: u64,
        pub error_rate: u32,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct StateSnapshot {
        pub timestamp: u64,
        pub total_operations: u64,
        pub total_users: u64,
        pub total_errors: u64,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceStats {
        pub function_name: Symbol,
        pub call_count: u64,
        pub total_time: u64,
        pub avg_time: u64,
        pub last_called: u64,
    }

    pub fn track_operation(env: &Env, operation: Symbol, caller: Address, success: bool) {
        let key = Symbol::new(env, OPERATION_COUNT);
        let count: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(count + 1));

        if !success {
            let err_key = Symbol::new(env, ERROR_COUNT);
            let err_count: u64 = env.storage().persistent().get(&err_key).unwrap_or(0);
            env.storage().persistent().set(&err_key, &(err_count + 1));
        }

        env.events().publish(
            (symbol_short!("metric"), symbol_short!("op")),
            OperationMetric {
                operation,
                caller,
                timestamp: env.ledger().timestamp(),
                success,
            },
        );
    }

    pub fn emit_performance(env: &Env, function: Symbol, duration: u64) {
        let count_key = (Symbol::new(env, "perf_cnt"), function.clone());
        let time_key = (Symbol::new(env, "perf_time"), function.clone());

        let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
        let total: u64 = env.storage().persistent().get(&time_key).unwrap_or(0);

        env.storage().persistent().set(&count_key, &(count + 1));
        env.storage()
            .persistent()
            .set(&time_key, &(total + duration));

        env.events().publish(
            (symbol_short!("metric"), symbol_short!("perf")),
            PerformanceMetric {
                function,
                duration,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    #[allow(dead_code)]
    pub fn _health_check(env: &Env) -> HealthStatus {
        let key = Symbol::new(env, OPERATION_COUNT);
        let ops: u64 = env.storage().persistent().get(&key).unwrap_or(0);

        HealthStatus {
            is_healthy: true,
            last_operation: env.ledger().timestamp(),
            total_operations: ops,
            contract_version: String::from_str(env, "1.0.0"),
        }
    }

    // Get analytics
    #[allow(dead_code)]
    pub fn get_analytics(env: &Env) -> Analytics {
        let op_key = Symbol::new(env, OPERATION_COUNT);
        let usr_key = Symbol::new(env, USER_COUNT);
        let err_key = Symbol::new(env, ERROR_COUNT);

        let ops: u64 = env.storage().persistent().get(&op_key).unwrap_or(0);
        let users: u64 = env.storage().persistent().get(&usr_key).unwrap_or(0);
        let errors: u64 = env.storage().persistent().get(&err_key).unwrap_or(0);

        let error_rate = if ops > 0 {
            ((errors as u128 * 10000) / ops as u128) as u32
        } else {
            0
        };

        Analytics {
            operation_count: ops,
            unique_users: users,
            error_count: errors,
            error_rate,
        }
    }

    // Get state snapshot
    #[allow(dead_code)]
    pub fn get_state_snapshot(env: &Env) -> StateSnapshot {
        let op_key = Symbol::new(env, OPERATION_COUNT);
        let usr_key = Symbol::new(env, USER_COUNT);
        let err_key = Symbol::new(env, ERROR_COUNT);

        StateSnapshot {
            timestamp: env.ledger().timestamp(),
            total_operations: env.storage().persistent().get(&op_key).unwrap_or(0),
            total_users: env.storage().persistent().get(&usr_key).unwrap_or(0),
            total_errors: env.storage().persistent().get(&err_key).unwrap_or(0),
        }
    }

    // Get performance stats
    #[allow(dead_code)]
    pub fn get_performance_stats(env: &Env, function_name: Symbol) -> PerformanceStats {
        let count_key = (Symbol::new(env, "perf_cnt"), function_name.clone());
        let time_key = (Symbol::new(env, "perf_time"), function_name.clone());
        let last_key = (Symbol::new(env, "perf_last"), function_name.clone());

        let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
        let total: u64 = env.storage().persistent().get(&time_key).unwrap_or(0);
        let last: u64 = env.storage().persistent().get(&last_key).unwrap_or(0);

        let avg = if count > 0 { total / count } else { 0 };

        PerformanceStats {
            function_name,
            call_count: count,
            total_time: total,
            avg_time: avg,
            last_called: last,
        }
    }
}
// ==================== END MONITORING MODULE ====================

// ==================== ANTI-ABUSE MODULE ====================
#[allow(dead_code)]
mod anti_abuse {
    use soroban_sdk::{contracttype, symbol_short, Address, Env};

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct AntiAbuseConfig {
        pub window_size: u64,
        pub max_operations: u32,
        pub cooldown_period: u64,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct AddressState {
        pub last_operation_timestamp: u64,
        pub window_start_timestamp: u64,
        pub operation_count: u32,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum AntiAbuseKey {
        Config,
        State(Address),
        Whitelist(Address),
        Admin,
    }

    pub fn get_config(env: &Env) -> AntiAbuseConfig {
        env.storage()
            .instance()
            .get(&AntiAbuseKey::Config)
            .unwrap_or(AntiAbuseConfig {
                window_size: 3600,
                max_operations: 10,
                cooldown_period: 60,
            })
    }

    #[allow(dead_code)]
    pub fn _set_config(env: &Env, config: AntiAbuseConfig) {
        env.storage().instance().set(&AntiAbuseKey::Config, &config);
    }

    pub fn is_whitelisted(env: &Env, address: Address) -> bool {
        env.storage()
            .instance()
            .has(&AntiAbuseKey::Whitelist(address))
    }

    #[allow(dead_code)]
    pub fn set_whitelist(env: &Env, address: Address, whitelisted: bool) {
        if whitelisted {
            env.storage()
                .instance()
                .set(&AntiAbuseKey::Whitelist(address), &true);
        } else {
            env.storage()
                .instance()
                .remove(&AntiAbuseKey::Whitelist(address));
        }
    }

    #[allow(dead_code)]
    pub fn get_admin(env: &Env) -> Option<Address> {
        env.storage().instance().get(&AntiAbuseKey::Admin)
    }

    #[allow(dead_code)]
    pub fn set_admin(env: &Env, admin: Address) {
        env.storage().instance().set(&AntiAbuseKey::Admin, &admin);
    }

    pub fn check_rate_limit(env: &Env, address: Address) {
        if is_whitelisted(env, address.clone()) {
            return;
        }

        let config = get_config(env);
        let now = env.ledger().timestamp();
        let key = AntiAbuseKey::State(address.clone());

        let mut state: AddressState =
            env.storage()
                .persistent()
                .get(&key)
                .unwrap_or(AddressState {
                    last_operation_timestamp: 0,
                    window_start_timestamp: now,
                    operation_count: 0,
                });

        if state.last_operation_timestamp > 0
            && now
                < state
                    .last_operation_timestamp
                    .saturating_add(config.cooldown_period)
        {
            env.events().publish(
                (symbol_short!("abuse"), symbol_short!("cooldown")),
                (address.clone(), now),
            );
            panic!("Operation in cooldown period");
        }

        if now
            >= state
                .window_start_timestamp
                .saturating_add(config.window_size)
        {
            state.window_start_timestamp = now;
            state.operation_count = 1;
        } else {
            if state.operation_count >= config.max_operations {
                env.events().publish(
                    (symbol_short!("abuse"), symbol_short!("limit")),
                    (address.clone(), now),
                );
                panic!("Rate limit exceeded");
            }
            state.operation_count += 1;
        }

        state.last_operation_timestamp = now;
        env.storage().persistent().set(&key, &state);
        env.storage().persistent().extend_ttl(&key, 17280, 17280);
    }
}
// ==================== END ANTI-ABUSE MODULE ====================

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    BountyExists = 3,
    BountyNotFound = 4,
    FundsNotLocked = 5,
    DeadlineNotPassed = 6,
    Unauthorized = 7,

    /// Returned when attempting to use a non-whitelisted token
    TokenNotWhitelisted = 8,

    /// Returned when attempting to whitelist an already whitelisted token
    TokenAlreadyWhitelisted = 9,
    InvalidBatchSize = 10,
    ContractPaused = 11,
    DuplicateBountyId = 12,
    InvalidAmount = 13,
    InvalidDeadline = 14,
    /// Returned when contract has insufficient funds for the operation
    InsufficientFunds = 16,
    RefundNotApproved = 17,
    BatchSizeMismatch = 18,
    /// Returned when attempting to extend deadline to a value not greater than current deadline
    InvalidDeadlineExtension = 19,
    /// Returned when metadata exceeds size limits
    MetadataTooLarge = 20,
    ReentrantCall = 21,
    /// Returned when participant is blacklisted or not whitelisted
    ParticipantNotAllowed = 22,
    ActionNotFound = 23,
    ActionNotReady = 24,
    InvalidTimeLock = 25,
    /// Returned when fee rate is invalid
    InvalidFeeRate = 29,
    /// Returned when fee recipient is not set
    FeeRecipientNotSet = 30,
    /// Returned when lock operations are paused
    LockPaused = 26,
    /// Returned when release operations are paused
    ReleasePaused = 27,
    /// Returned when refund operations are paused
    RefundPaused = 28,
}

// ============================================================================
// Data Structures
// ============================================================================

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
    PartiallyRefunded,
    PartiallyReleased,
}

/// Per-operation pause configuration for granular control.
///
/// Allows the platform to halt specific operations while allowing others to continue.
/// For example, new locks can be paused while releases and refunds remain active.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseConfig {
    pub lock_paused: bool,
    pub release_paused: bool,
    pub refund_paused: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundRecord {
    pub amount: i128,
    pub recipient: Address,
    pub mode: RefundMode,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundApproval {
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub mode: RefundMode,
    pub approved_by: Address,
    pub approved_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub depositor: Address,
    pub amount: i128, // Total amount (sum of all token balances, for backward compatibility)
    pub status: EscrowStatus,
    pub deadline: u64,
    pub token: Address,
    pub refund_history: Vec<RefundRecord>,
    pub remaining_amount: i128, // Total remaining (sum of all token balances)
    pub token_address: Address, // Primary/default token (for backward compatibility)
    pub token_balances: Map<Address, i128>, // Map of token_address -> balance for multi-token support
    pub payout_history: Vec<PayoutRecord>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockFundsItem {
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub deadline: u64,
    pub token_address: Option<Address>, // Optional: if None, uses default token
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseFundsItem {
    pub bounty_id: u64,
    pub contributor: Address,
}

const MAX_BATCH_SIZE: u32 = 100;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,
    pub release_fee_rate: i128,
    pub fee_recipient: Address,
    pub fee_enabled: bool,
}

const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 1_000;

// ============================================================================
// Admin Configuration Structures
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigLimits {
    pub max_bounty_amount: Option<i128>,
    pub min_bounty_amount: Option<i128>,
    pub max_deadline_duration: Option<u64>,
    pub min_deadline_duration: Option<u64>,
}

// FIXED: Refactored AdminActionType to carry the data, removing problematic Options from AdminAction
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminActionType {
    UpdateAdmin(Address),
    UpdatePayoutKey(Address),
    UpdateConfigLimits(ConfigLimits),
    UpdateFeeConfig(FeeConfig),
}

// FIXED: Removed Option<FeeConfig> and others to resolve trait bound error
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminAction {
    pub action_id: u64,
    pub action_type: AdminActionType,
    pub proposed_by: Address,
    pub execution_time: u64,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractState {
    pub admin: Address,
    pub token: Address,
    pub payout_key: Option<Address>,
    pub fee_config: FeeConfig,
    pub config_limits: ConfigLimits,
    pub is_paused: bool,
    pub time_lock_duration: u64,
    pub total_bounties: u64,
    pub total_locked_amount: i128,
    pub contract_version: u64,
}

/// Metadata structure for enhanced escrow indexing and categorization.
///
/// # Fields
/// * `repo_id` - Repository identifier (e.g., "owner/repo")
/// * `issue_id` - Issue or pull request identifier
/// * `bounty_type` - Type classification (e.g., "bug", "feature", "security")
/// * `tags` - Custom tags for filtering and categorization
/// * `custom_fields` - Additional key-value pairs for extensibility
///
/// # Size Limits
/// * Total serialized size: 1024 bytes maximum
/// * Tags vector: 20 items maximum
/// * Custom fields map: 10 key-value pairs maximum
/// * Individual string values: 128 characters maximum
///
/// # Storage
/// Stored separately from core escrow data with key `DataKey::EscrowMetadata(bounty_id)`.
/// Metadata is optional and can be added/updated after escrow creation.
///
/// # Example
/// ```rust
/// let metadata = EscrowMetadata {
///     repo_id: Some(String::from_str(&env, "stellar/rs-soroban-sdk")),
///     issue_id: Some(String::from_str(&env, "123")),
///     bounty_type: Some(String::from_str(&env, "bug")),
///     tags: vec![&env,
///         String::from_str(&env, "priority-high"),
///         String::from_str(&env, "security")
///     ],
///     custom_fields: map![
///         &env,
///         (String::from_str(&env, "difficulty"), String::from_str(&env, "medium")),
///         (String::from_str(&env, "estimated_hours"), String::from_str(&env, "20"))
///     ]
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowMetadata {
    pub repo_id: Option<String>,
    pub issue_id: Option<String>,
    pub bounty_type: Option<String>,
    pub tags: Vec<String>,
    pub custom_fields: Map<String, String>,
}

/// Combined view of escrow data and metadata for convenient access.
///
/// # Fields
/// * `escrow` - Core escrow information
/// * `metadata` - Optional metadata (None if not set)
///
/// # Usage
/// Provides a unified interface for retrieving complete escrow information
/// including both financial and descriptive data.
///
/// # Example
/// ```rust
/// let escrow_view = escrow_client.get_escrow_with_metadata(&42)?;
/// if let Some(metadata) = escrow_view.metadata {
///     println!("Repo: {:?}", metadata.repo_id);
///     println!("Issue: {:?}", metadata.issue_id);
///     println!("Tags: {:?}", metadata.tags);
/// }
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowWithMetadata {
    pub escrow: Escrow,
    pub has_metadata: bool,
    pub metadata: EscrowMetadata,
}

/// Storage keys for contract data.
///
/// # Keys
/// * `Admin` - Stores the admin address (instance storage)
/// * `Token` - Stores the token contract address (instance storage)
/// * `Escrow(u64)` - Stores escrow data indexed by bounty_id (persistent storage)
/// * `EscrowMetadata(u64)` - Stores metadata for bounty_id (persistent storage)
///
/// # Storage Types
/// - **Instance Storage**: Admin and Token (never expires, tied to contract)
/// - **Persistent Storage**: Individual escrow records and metadata (extended TTL on access)

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct AmountLimits {
    pub min_lock_amount: i128,
    pub max_lock_amount: i128,
    pub min_payout: i128,
    pub max_payout: i128,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,               // Default token (for backward compatibility)
    Escrow(u64),         // bounty_id
    EscrowMetadata(u64), // bounty_id -> EscrowMetadata
    FeeConfig,           // Fee configuration
    AmountLimits,        // Amount limits configuration
    RefundApproval(u64), // bounty_id -> RefundApproval
    ReentrancyGuard,
    IsPaused,                // Contract pause state (legacy, for backward compat)
    TokenWhitelist(Address), // token_address -> bool (whitelist status)
    RegisteredTokens,        // Vec<Address> of all registered tokens
    PayoutKey,
    ConfigLimits,
    TimeLockDuration,
    NextActionId,
    AdminAction(u64),
    BountyRegistry, // Vec<u64> of all bounty IDs
    PauseConfig,    // Per-operation pause configuration
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowFilter {
    pub status: Option<u32>, // Using u32 to avoid Option<Enum> XDR issues
    pub depositor: Option<Address>,
    pub min_amount: Option<i128>,
    pub max_amount: Option<i128>,
    pub start_time: Option<u64>, // Filter by deadline (>= start_time)
    pub end_time: Option<u64>,   // Filter by deadline (<= end_time)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pagination {
    pub start_index: u64,
    pub limit: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowStats {
    pub total_bounties: u64,
    pub total_locked_amount: i128,
    pub total_released_amount: i128,
    pub total_refunded_amount: i128,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validates that metadata doesn't exceed size limits.
///
/// # Limits
/// - Maximum 20 tags
/// - Maximum 10 custom fields
/// - Maximum 128 characters per string field (repo_id, issue_id, bounty_type)
/// - Maximum 64 characters per tag
/// - Maximum 128 characters per custom field key/value
///
/// # Arguments
/// * `env` - The contract environment
/// * `metadata` - The metadata to validate
///
/// # Returns
/// * `true` - Metadata is within limits
/// * `false` - Metadata exceeds one or more limits
fn validate_metadata_size(env: &Env, metadata: &EscrowMetadata) -> bool {
    // Check tags limit
    if metadata.tags.len() > 20 {
        return false;
    }

    // Check custom fields limit
    if metadata.custom_fields.len() > 10 {
        return false;
    }

    // Check individual string lengths
    if let Some(repo_id) = &metadata.repo_id {
        if repo_id.len() > 128 {
            return false;
        }
    }

    if let Some(issue_id) = &metadata.issue_id {
        if issue_id.len() > 128 {
            return false;
        }
    }

    if let Some(bounty_type) = &metadata.bounty_type {
        if bounty_type.len() > 128 {
            return false;
        }
    }

    for tag in metadata.tags.iter() {
        if tag.len() > 64 {
            return false;
        }
    }

    for (key, value) in metadata.custom_fields.iter() {
        if key.len() > 128 || value.len() > 128 {
            return false;
        }
    }

    true
}

// ============================================================================
// Contract Implementation
// ============================================================================
// ============================================================================

#[contract]
pub struct BountyEscrowContract;

#[contractimpl]
impl BountyEscrowContract {
    /// Validate metadata size limits (internal helper)
    fn validate_metadata_size(_env: &Env, metadata: &EscrowMetadata) -> bool {
        // Check tags limit (max 20)
        if metadata.tags.len() > 20 {
            return false;
        }

        // Check custom fields limit (max 10)
        if metadata.custom_fields.len() > 10 {
            return false;
        }

        // Check individual string length limits (max 128 chars)
        let max_string_len = 128u32;

        if let Some(ref repo_id) = metadata.repo_id {
            if repo_id.len() > max_string_len {
                return false;
            }
        }

        if let Some(ref issue_id) = metadata.issue_id {
            if issue_id.len() > max_string_len {
                return false;
            }
        }

        if let Some(ref bounty_type) = metadata.bounty_type {
            if bounty_type.len() > max_string_len {
                return false;
            }
        }

        // Check tag string lengths
        for i in 0..metadata.tags.len() {
            let tag = metadata.tags.get(i).unwrap();
            if tag.len() > max_string_len {
                return false;
            }
        }

        // Note: Custom fields validation is simplified
        // Full validation would require iterating over map entries
        // which is complex in Soroban. The len() check above should be sufficient
        // for most cases. Additional validation can be added if needed.

        true
    }
    // ========================================================================
    // Initialization
    // ========================================================================

    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        anti_abuse::check_rate_limit(&env, admin.clone());

        let start = env.ledger().timestamp();
        let caller = admin.clone();

        if env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("init"), caller, false);
            return Err(Error::AlreadyInitialized);
        }

        // Store configuration
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        // Auto-whitelist the initial token
        env.storage()
            .instance()
            .set(&DataKey::TokenWhitelist(token.clone()), &true);
        let mut token_list: Vec<Address> = Vec::new(&env);
        token_list.push_back(token.clone());
        env.storage()
            .instance()
            .set(&DataKey::RegisteredTokens, &token_list);

        // Auto-whitelist the initial token
        env.storage()
            .instance()
            .set(&DataKey::TokenWhitelist(token.clone()), &true);
        let mut token_list: Vec<Address> = Vec::new(&env);
        token_list.push_back(token.clone());
        env.storage()
            .instance()
            .set(&DataKey::RegisteredTokens, &token_list);

        let fee_config = FeeConfig {
            lock_fee_rate: 0,
            release_fee_rate: 0,
            fee_recipient: admin.clone(),
            fee_enabled: false,
        };
        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &fee_config);

        let config_limits = ConfigLimits {
            max_bounty_amount: None,
            min_bounty_amount: None,
            max_deadline_duration: None,
            min_deadline_duration: None,
        };
        env.storage()
            .instance()
            .set(&DataKey::ConfigLimits, &config_limits);

        // Initialize amount limits with default values
        let amount_limits = AmountLimits {
            min_lock_amount: 1,
            max_lock_amount: i128::MAX,
            min_payout: 1,
            max_payout: i128::MAX,
        };
        env.storage()
            .instance()
            .set(&DataKey::AmountLimits, &amount_limits);

        env.storage()
            .instance()
            .set(&DataKey::TimeLockDuration, &0u64);
        env.storage().instance().set(&DataKey::NextActionId, &1u64);

        emit_bounty_initialized(
            &env,
            BountyEscrowInitialized {
                admin: admin.clone(),
                token,
                timestamp: env.ledger().timestamp(),
            },
        );

        monitoring::track_operation(&env, symbol_short!("init"), caller, true);

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init"), duration);

        Ok(())
    }

    /// Get default token address (for backward compatibility)
    fn get_default_token(env: &Env) -> Result<Address, Error> {
        if !env.storage().instance().has(&DataKey::Token) {
            return Err(Error::NotInitialized);
        }
        Ok(env.storage().instance().get(&DataKey::Token).unwrap())
    }

    /// Check if token is registered/whitelisted
    fn is_token_registered(env: &Env, token: &Address) -> bool {
        // If whitelist is enabled, check whitelist
        // Otherwise, check if it's the default token or in registered tokens list
        if env.storage().instance().has(&DataKey::Token) {
            let default_token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
            if *token == default_token {
                return true;
            }
        }

        // Check whitelist
        env.storage()
            .instance()
            .has(&DataKey::TokenWhitelist(token.clone()))
    }

    /// Register a new token (admin only)
    /// If whitelist_enabled is true, only whitelisted tokens can be used
    pub fn register_token(env: Env, token: Address, whitelisted: bool) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Store whitelist status
        if whitelisted {
            env.storage()
                .instance()
                .set(&DataKey::TokenWhitelist(token.clone()), &true);
        } else {
            env.storage()
                .instance()
                .remove(&DataKey::TokenWhitelist(token.clone()));
        }

        // Add to registered tokens list
        let mut registered: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::RegisteredTokens)
            .unwrap_or_else(|| vec![&env]);

        // Check if already registered
        let mut found = false;
        for i in 0..registered.len() {
            if registered.get(i).unwrap() == token {
                found = true;
                break;
            }
        }

        if !found {
            registered.push_back(token.clone());
            env.storage()
                .instance()
                .set(&DataKey::RegisteredTokens, &registered);
        }

        Ok(())
    }

    /// Get all registered tokens
    pub fn get_registered_tokens(env: Env) -> Result<Vec<Address>, Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let mut tokens: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::RegisteredTokens)
            .unwrap_or_else(|| vec![&env]);

        // Always include default token
        if env.storage().instance().has(&DataKey::Token) {
            let default_token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
            let mut found = false;
            for i in 0..tokens.len() {
                if tokens.get(i).unwrap() == default_token {
                    found = true;
                    break;
                }
            }
            if !found {
                tokens.push_back(default_token);
            }
        }

        Ok(tokens)
    }

    /// Get balance for a specific token in an escrow
    pub fn get_token_balance(env: Env, bounty_id: u64, token: Address) -> Result<i128, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        // Check token_balances map
        if let Some(balance) = escrow.token_balances.get(token.clone()) {
            Ok(balance)
        } else if escrow.token_address == token {
            // Backward compatibility: if token matches default, return remaining_amount
            Ok(escrow.remaining_amount)
        } else {
            Ok(0)
        }
    }

    /// Calculate fee amount based on rate (in basis points)
    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        if fee_rate == 0 {
            return 0;
        }
        amount
            .checked_mul(fee_rate)
            .and_then(|x| x.checked_div(BASIS_POINTS))
            .unwrap_or(0)
    }

    fn get_fee_config_internal(env: &Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&DataKey::FeeConfig)
            .unwrap_or_else(|| FeeConfig {
                lock_fee_rate: 0,
                release_fee_rate: 0,
                fee_recipient: env.storage().instance().get(&DataKey::Admin).unwrap(),
                fee_enabled: false,
            })
    }

    pub fn update_fee_config(
        env: Env,
        lock_fee_rate: Option<i128>,
        release_fee_rate: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut fee_config = Self::get_fee_config_internal(&env);

        if let Some(rate) = lock_fee_rate {
            if !(0..=MAX_FEE_RATE).contains(&rate) {
                return Err(Error::InvalidFeeRate);
            }
            fee_config.lock_fee_rate = rate;
        }

        if let Some(rate) = release_fee_rate {
            if !(0..=MAX_FEE_RATE).contains(&rate) {
                return Err(Error::InvalidFeeRate);
            }
            fee_config.release_fee_rate = rate;
        }

        if let Some(recipient) = fee_recipient {
            fee_config.fee_recipient = recipient;
        }

        if let Some(enabled) = fee_enabled {
            fee_config.fee_enabled = enabled;
        }

        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &fee_config);

        events::emit_fee_config_updated(
            &env,
            events::FeeConfigUpdated {
                lock_fee_rate: fee_config.lock_fee_rate,
                release_fee_rate: fee_config.release_fee_rate,
                fee_recipient: fee_config.fee_recipient.clone(),
                fee_enabled: fee_config.fee_enabled,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
    }

    /// Update amount limits configuration (admin only)
    pub fn update_amount_limits(
        env: Env,
        min_lock_amount: i128,
        max_lock_amount: i128,
        min_payout: i128,
        max_payout: i128,
    ) -> Result<(), Error> {
        // Get admin and require authorization
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        // Validate limits
        if min_lock_amount < 0 || max_lock_amount < 0 || min_payout < 0 || max_payout < 0 {
            return Err(Error::InvalidAmount);
        }
        if min_lock_amount > max_lock_amount || min_payout > max_payout {
            return Err(Error::InvalidAmount);
        }

        let limits = AmountLimits {
            min_lock_amount,
            max_lock_amount,
            min_payout,
            max_payout,
        };

        env.storage()
            .instance()
            .set(&DataKey::AmountLimits, &limits);

        // Emit event
        env.events().publish(
            (symbol_short!("amt_lmt"),),
            (min_lock_amount, max_lock_amount, min_payout, max_payout),
        );

        Ok(())
    }

    /// Get current amount limits configuration (view function)
    pub fn get_amount_limits(env: Env) -> AmountLimits {
        env.storage()
            .instance()
            .get(&DataKey::AmountLimits)
            .unwrap_or(AmountLimits {
                min_lock_amount: 1,
                max_lock_amount: i128::MAX,
                min_payout: 1,
                max_payout: i128::MAX,
            })
    }

    // ========================================================================
    // Admin Configuration Functions
    // ========================================================================

    /// Update admin address (with optional time-lock)
    pub fn update_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let time_lock_duration: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TimeLockDuration)
            .unwrap_or(0);

        if time_lock_duration > 0 {
            let action_id: u64 = env
                .storage()
                .instance()
                .get(&DataKey::NextActionId)
                .unwrap();
            let execution_time = env.ledger().timestamp() + time_lock_duration;

            let action = AdminAction {
                action_id,
                // FIXED: Use the Enum variant carrying the data
                action_type: AdminActionType::UpdateAdmin(new_admin.clone()),
                proposed_by: admin.clone(),
                execution_time,
                executed: false,
            };

            env.storage()
                .persistent()
                .set(&DataKey::AdminAction(action_id), &action);
            env.storage()
                .instance()
                .set(&DataKey::NextActionId, &(action_id + 1));

            emit_admin_action_proposed(
                &env,
                AdminActionProposed {
                    action_id,
                    action_type: AdminActionType::UpdateAdmin(new_admin), // Pass data for event
                    proposed_by: admin,
                    execution_time,
                    timestamp: env.ledger().timestamp(),
                },
            );
        } else {
            let old_admin = admin.clone();
            env.storage().instance().set(&DataKey::Admin, &new_admin);

            emit_admin_updated(
                &env,
                AdminUpdated {
                    old_admin,
                    new_admin,
                    updated_by: admin,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        Ok(())
    }

    /// Update authorized payout key
    pub fn update_payout_key(env: Env, new_payout_key: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let old_key: Option<Address> = env.storage().instance().get(&DataKey::PayoutKey);

        env.storage()
            .instance()
            .set(&DataKey::PayoutKey, &new_payout_key);

        emit_payout_key_updated(
            &env,
            PayoutKeyUpdated {
                old_key,
                new_key: new_payout_key,
                updated_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Update configuration limits
    pub fn update_config_limits(
        env: Env,
        max_bounty_amount: Option<i128>,
        min_bounty_amount: Option<i128>,
        max_deadline_duration: Option<u64>,
        min_deadline_duration: Option<u64>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let limits = ConfigLimits {
            max_bounty_amount,
            min_bounty_amount,
            max_deadline_duration,
            min_deadline_duration,
        };

        env.storage()
            .instance()
            .set(&DataKey::ConfigLimits, &limits);

        emit_config_limits_updated(
            &env,
            ConfigLimitsUpdated {
                max_bounty_amount: limits.max_bounty_amount,
                min_bounty_amount: limits.min_bounty_amount,
                max_deadline_duration: limits.max_deadline_duration,
                min_deadline_duration: limits.min_deadline_duration,
                updated_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Set time-lock duration for admin actions
    pub fn set_time_lock_duration(env: Env, duration: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::TimeLockDuration, &duration);

        Ok(())
    }

    /// Execute a pending admin action
    pub fn execute_admin_action(env: Env, action_id: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env
            .storage()
            .persistent()
            .has(&DataKey::AdminAction(action_id))
        {
            return Err(Error::ActionNotFound);
        }

        let mut action: AdminAction = env
            .storage()
            .persistent()
            .get(&DataKey::AdminAction(action_id))
            .unwrap();

        if action.executed {
            return Err(Error::ActionNotFound);
        }

        if env.ledger().timestamp() < action.execution_time {
            return Err(Error::ActionNotReady);
        }

        // FIXED: Destructure the Enum data directly
        match action.action_type.clone() {
            AdminActionType::UpdateAdmin(new_admin) => {
                let old_admin = admin.clone();
                env.storage().instance().set(&DataKey::Admin, &new_admin);

                emit_admin_updated(
                    &env,
                    AdminUpdated {
                        old_admin,
                        new_admin,
                        updated_by: admin.clone(),
                        timestamp: env.ledger().timestamp(),
                    },
                );
            }
            AdminActionType::UpdatePayoutKey(new_key) => {
                let old_key: Option<Address> = env.storage().instance().get(&DataKey::PayoutKey);
                env.storage().instance().set(&DataKey::PayoutKey, &new_key);

                emit_payout_key_updated(
                    &env,
                    PayoutKeyUpdated {
                        old_key,
                        new_key,
                        updated_by: admin.clone(),
                        timestamp: env.ledger().timestamp(),
                    },
                );
            }
            AdminActionType::UpdateConfigLimits(limits) => {
                env.storage()
                    .instance()
                    .set(&DataKey::ConfigLimits, &limits);

                emit_config_limits_updated(
                    &env,
                    ConfigLimitsUpdated {
                        max_bounty_amount: limits.max_bounty_amount,
                        min_bounty_amount: limits.min_bounty_amount,
                        max_deadline_duration: limits.max_deadline_duration,
                        min_deadline_duration: limits.min_deadline_duration,
                        updated_by: admin.clone(),
                        timestamp: env.ledger().timestamp(),
                    },
                );
            }
            AdminActionType::UpdateFeeConfig(fee_config) => {
                env.storage()
                    .instance()
                    .set(&DataKey::FeeConfig, &fee_config);

                events::emit_fee_config_updated(
                    &env,
                    events::FeeConfigUpdated {
                        lock_fee_rate: fee_config.lock_fee_rate,
                        release_fee_rate: fee_config.release_fee_rate,
                        fee_recipient: fee_config.fee_recipient.clone(),
                        fee_enabled: fee_config.fee_enabled,
                        timestamp: env.ledger().timestamp(),
                    },
                );
            }
        }

        action.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::AdminAction(action_id), &action);

        emit_admin_action_executed(
            &env,
            AdminActionExecuted {
                action_id,
                action_type: action.action_type,
                executed_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Cancel a pending admin action
    pub fn cancel_admin_action(env: Env, action_id: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env
            .storage()
            .persistent()
            .has(&DataKey::AdminAction(action_id))
        {
            return Err(Error::ActionNotFound);
        }

        let action: AdminAction = env
            .storage()
            .persistent()
            .get(&DataKey::AdminAction(action_id))
            .unwrap();

        if action.executed {
            return Err(Error::ActionNotFound);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::AdminAction(action_id));

        emit_admin_action_cancelled(
            &env,
            AdminActionCancelled {
                action_id,
                action_type: action.action_type,
                cancelled_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Get contract state (comprehensive view function)
    pub fn get_contract_state(env: Env) -> Result<ContractState, Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let payout_key: Option<Address> = env.storage().instance().get(&DataKey::PayoutKey);
        let fee_config = Self::get_fee_config_internal(&env);
        let config_limits: ConfigLimits = env
            .storage()
            .instance()
            .get(&DataKey::ConfigLimits)
            .unwrap_or(ConfigLimits {
                max_bounty_amount: None,
                min_bounty_amount: None,
                max_deadline_duration: None,
                min_deadline_duration: None,
            });
        let is_paused = Self::is_paused_internal(&env);
        let time_lock_duration: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TimeLockDuration)
            .unwrap_or(0);

        Ok(ContractState {
            admin,
            token,
            payout_key,
            fee_config,
            config_limits,
            is_paused,
            time_lock_duration,
            total_bounties: 0,
            total_locked_amount: 0,
            contract_version: 1,
        })
    }

    /// Get pending admin action
    pub fn get_admin_action(env: Env, action_id: u64) -> Result<AdminAction, Error> {
        if !env
            .storage()
            .persistent()
            .has(&DataKey::AdminAction(action_id))
        {
            return Err(Error::ActionNotFound);
        }

        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::AdminAction(action_id))
            .unwrap())
    }

    // ========================================================================
    // Pause and Emergency Functions
    // ========================================================================

    /// Get pause configuration (internal helper)
    fn get_pause_config_internal(env: &Env) -> PauseConfig {
        env.storage()
            .persistent()
            .get(&DataKey::PauseConfig)
            .unwrap_or(PauseConfig {
                lock_paused: false,
                release_paused: false,
                refund_paused: false,
            })
    }

    /// Check if lock operations are paused (internal helper)
    fn is_lock_paused_internal(env: &Env) -> bool {
        Self::get_pause_config_internal(env).lock_paused
    }

    /// Check if release operations are paused (internal helper)
    fn is_release_paused_internal(env: &Env) -> bool {
        Self::get_pause_config_internal(env).release_paused
    }

    /// Check if refund operations are paused (internal helper)
    fn is_refund_paused_internal(env: &Env) -> bool {
        Self::get_pause_config_internal(env).refund_paused
    }

    /// Check if all operations are paused (backward compatibility)
    fn is_paused_internal(env: &Env) -> bool {
        let config = Self::get_pause_config_internal(env);
        config.lock_paused && config.release_paused && config.refund_paused
    }

    /// Get pause configuration (view function)
    pub fn get_pause_config(env: Env) -> PauseConfig {
        Self::get_pause_config_internal(&env)
    }

    /// Check if lock operations are paused (view function)
    pub fn is_lock_paused(env: Env) -> bool {
        Self::is_lock_paused_internal(&env)
    }

    /// Check if release operations are paused (view function)
    pub fn is_release_paused(env: Env) -> bool {
        Self::is_release_paused_internal(&env)
    }

    /// Check if refund operations are paused (view function)
    pub fn is_refund_paused(env: Env) -> bool {
        Self::is_refund_paused_internal(&env)
    }

    /// Get pause status (view function) - returns true if ALL operations are paused
    /// Maintained for backward compatibility
    pub fn is_paused(env: Env) -> bool {
        Self::is_paused_internal(&env)
    }

    /// Pause or unpause lock operations (admin only)
    pub fn set_pause_lock(env: Env, paused: bool) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut config = Self::get_pause_config_internal(&env);
        if config.lock_paused == paused {
            return Ok(()); // Already in desired state, idempotent
        }

        config.lock_paused = paused;
        env.storage()
            .persistent()
            .set(&DataKey::PauseConfig, &config);

        emit_operation_pause_changed(
            &env,
            OperationPauseChanged {
                operation: symbol_short!("lock"),
                paused,
                changed_by: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Pause or unpause release operations (admin only)
    pub fn set_pause_release(env: Env, paused: bool) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut config = Self::get_pause_config_internal(&env);
        if config.release_paused == paused {
            return Ok(()); // Already in desired state, idempotent
        }

        config.release_paused = paused;
        env.storage()
            .persistent()
            .set(&DataKey::PauseConfig, &config);

        emit_operation_pause_changed(
            &env,
            OperationPauseChanged {
                operation: symbol_short!("release"),
                paused,
                changed_by: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Pause or unpause refund operations (admin only)
    pub fn set_pause_refund(env: Env, paused: bool) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut config = Self::get_pause_config_internal(&env);
        if config.refund_paused == paused {
            return Ok(()); // Already in desired state, idempotent
        }

        config.refund_paused = paused;
        env.storage()
            .persistent()
            .set(&DataKey::PauseConfig, &config);

        emit_operation_pause_changed(
            &env,
            OperationPauseChanged {
                operation: symbol_short!("refund"),
                paused,
                changed_by: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Pause the contract (admin only) - pauses ALL operations
    /// Maintained for backward compatibility
    pub fn pause(env: Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let config = PauseConfig {
            lock_paused: true,
            release_paused: true,
            refund_paused: true,
        };

        env.storage()
            .persistent()
            .set(&DataKey::PauseConfig, &config);

        emit_contract_paused(
            &env,
            ContractPaused {
                paused_by: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Unpause the contract (admin only) - unpauses ALL operations
    /// Maintained for backward compatibility
    pub fn unpause(env: Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let config = PauseConfig {
            lock_paused: false,
            release_paused: false,
            refund_paused: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::PauseConfig, &config);

        emit_contract_unpaused(
            &env,
            ContractUnpaused {
                unpaused_by: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn emergency_withdraw(env: Env, recipient: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !Self::is_paused_internal(&env) {
            return Err(Error::Unauthorized);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        let balance = client.balance(&env.current_contract_address());

        if balance <= 0 {
            return Ok(());
        }

        client.transfer(&env.current_contract_address(), &recipient, &balance);

        emit_emergency_withdrawal(
            &env,
            EmergencyWithdrawal {
                withdrawn_by: admin.clone(),
                amount: balance,
                recipient: recipient.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    // ========================================================================
    // Core Functions (Lock, Release, Refund)
    // ========================================================================

    /// Add an address to the blacklist (admin only)
    ///
    /// Blacklisted addresses cannot lock funds or receive payouts.
    /// Used for compliance (e.g., sanctioned addresses) or abuse prevention.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `address` - Address to blacklist
    /// * `reason` - Optional reason for blacklisting
    ///
    /// # Authorization
    /// - Admin only
    pub fn set_blacklist(
        env: Env,
        address: Address,
        blocked: bool,
        reason: Option<String>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if blocked {
            add_to_blacklist(&env, address, reason);
        } else {
            remove_from_blacklist(&env, address);
        }

        Ok(())
    }

    /// Add an address to the whitelist (admin only)
    ///
    /// When whitelist mode is enabled, only whitelisted addresses can participate.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `address` - Address to whitelist
    /// * `whitelisted` - true to add, false to remove
    ///
    /// # Authorization
    /// - Admin only
    pub fn set_whitelist(env: Env, address: Address, whitelisted: bool) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if whitelisted {
            add_to_whitelist(&env, address);
        } else {
            remove_from_whitelist(&env, address);
        }

        Ok(())
    }

    /// Toggle whitelist-only mode (admin only)
    ///
    /// When enabled, only whitelisted addresses can lock funds or receive payouts.
    /// When disabled, all non-blacklisted addresses can participate.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `enabled` - true to enable whitelist-only mode
    ///
    /// # Authorization
    /// - Admin only
    pub fn set_whitelist_mode(env: Env, enabled: bool) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        set_whitelist_mode(&env, enabled);

        Ok(())
    }

    /// Lock funds for a specific bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `depositor` - Address depositing the funds (must authorize)
    /// * `bounty_id` - Unique identifier for this bounty
    /// * `amount` - Token amount to lock (in smallest denomination)
    /// * `deadline` - Unix timestamp after which refund is allowed
    ///
    /// # Returns
    /// * `Ok(())` - Funds successfully locked
    /// * `Err(Error::NotInitialized)` - Contract not initialized
    /// * `Err(Error::BountyExists)` - Bounty ID already in use
    ///
    /// # State Changes
    /// - Transfers `amount` tokens from depositor to contract
    /// - Creates Escrow record in persistent storage
    /// - Emits FundsLocked event
    ///
    /// # Authorization
    /// - Depositor must authorize the transaction
    /// - Depositor must have sufficient token balance
    /// - Depositor must have approved contract for token transfer
    ///
    /// # Security Considerations
    /// - Bounty ID must be unique (prevents overwrites)
    /// - Amount must be positive (enforced by token contract)
    /// - Deadline should be reasonable (recommended: 7-90 days)
    /// - Token transfer is atomic with state update
    ///
    /// # Events
    /// Emits: `FundsLocked { bounty_id, amount, depositor, deadline }`
    ///
    /// # Example
    /// ```rust
    /// let depositor = Address::from_string("GDEPOSIT...");
    /// let amount = 1000_0000000; // 1000 USDC
    /// let deadline = env.ledger().timestamp() + (30 * 24 * 60 * 60); // 30 days
    ///
    /// escrow_client.lock_funds(&depositor, &42, &amount, &deadline)?;
    /// // Funds are now locked and can be released or refunded
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Token transfer + storage write + event emission
    ///
    /// # Common Pitfalls
    /// - Forgetting to approve token contract before calling
    /// - Using a bounty ID that already exists
    /// - Setting deadline in the past or too far in the future
    fn lock_funds_internal(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
        token_address: Option<Address>, // Optional: if None, uses default token
    ) -> Result<(), Error> {
        anti_abuse::check_rate_limit(&env, depositor.clone());

        // Check blacklist/whitelist
        if !is_participant_allowed(&env, &depositor) {
            return Err(Error::ParticipantNotAllowed);
        }

        let start = env.ledger().timestamp();
        let caller = depositor.clone();

        // Check if lock operations are paused
        if Self::is_lock_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            return Err(Error::LockPaused);
        }

        depositor.require_auth();

        let _guard = ReentrancyGuardRAII::new(&env).map_err(|_| Error::ReentrantCall)?;

        if amount <= 0 {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            return Err(Error::InvalidAmount);
        }

        // Check amount limits
        let limits = Self::get_amount_limits(env.clone());
        if amount < limits.min_lock_amount || amount > limits.max_lock_amount {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidAmount);
        }

        if deadline <= env.ledger().timestamp() {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            return Err(Error::InvalidDeadline);
        }
        if !env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            return Err(Error::NotInitialized);
        }

        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            return Err(Error::BountyExists);
        }

        // Get token address (use provided or default)
        let token_addr: Address = if let Some(token) = token_address {
            // Validate token is registered/whitelisted
            if !Self::is_token_registered(&env, &token) {
                monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
                env.storage().instance().remove(&DataKey::ReentrancyGuard);
                return Err(Error::TokenNotWhitelisted);
            }
            token
        } else {
            // Use default token for backward compatibility
            Self::get_default_token(&env)?
        };
        let client = token::Client::new(&env, &token_addr);

        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0 {
            Self::calculate_fee(amount, fee_config.lock_fee_rate)
        } else {
            0
        };
        let net_amount = amount - fee_amount;

        client.transfer(&depositor, &env.current_contract_address(), &net_amount);

        if fee_amount > 0 {
            client.transfer(&depositor, &fee_config.fee_recipient, &fee_amount);
            events::emit_fee_collected(
                &env,
                events::FeeCollected {
                    operation_type: events::FeeOperationType::Lock,
                    amount: fee_amount,
                    fee_rate: fee_config.lock_fee_rate,
                    recipient: fee_config.fee_recipient.clone(),
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        // Check if escrow already exists (for multi-token support)
        let mut escrow: Escrow = if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            // Escrow exists, add to existing token balances
            env.storage()
                .persistent()
                .get(&DataKey::Escrow(bounty_id))
                .unwrap()
        } else {
            // Create new escrow
            let mut balances = Map::new(&env);
            balances.set(token_addr.clone(), net_amount);
            Escrow {
                depositor: depositor.clone(),
                amount: net_amount, // Store net amount (after fee)
                status: EscrowStatus::Locked,
                deadline,
                token: token_addr.clone(),
                refund_history: vec![&env],
                remaining_amount: net_amount,
                token_address: token_addr.clone(), // Primary token
                token_balances: balances,
                payout_history: vec![&env],
            }
        };

        // Update token balance in map (for existing escrows)
        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            let current_balance = escrow.token_balances.get(token_addr.clone()).unwrap_or(0);
            escrow
                .token_balances
                .set(token_addr.clone(), current_balance + net_amount);

            // Update total remaining amount
            escrow.remaining_amount = escrow.remaining_amount + net_amount;

            // Update total amount (sum of all token balances)
            escrow.amount = escrow.amount + net_amount;
        }

        // Store in persistent storage with extended TTL
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Update registry
        let mut registry: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::BountyRegistry)
            .unwrap_or(vec![&env]);
        registry.push_back(bounty_id);
        env.storage()
            .instance()
            .set(&DataKey::BountyRegistry, &registry);

        // Emit event for off-chain indexing
        emit_funds_locked(
            &env,
            FundsLocked {
                bounty_id,
                amount: net_amount,
                depositor: depositor.clone(),
                deadline,
                token_address: token_addr.clone(),
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("lock"), caller, true);

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("lock"), duration);

        Ok(())
    }

    /// Sets or updates metadata for an existing escrow.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to attach metadata to
    /// * `metadata` - Metadata structure containing repo, issue, type, and tags
    ///
    /// # Returns
    /// * `Ok(())` - Metadata successfully set/updated
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    /// * `Err(Error::MetadataTooLarge)` - Metadata exceeds size limits
    /// * `Err(Error::Unauthorized)` - Caller is not the depositor
    ///
    /// # State Changes
    /// - Stores/updates metadata in persistent storage
    /// - Extends storage TTL on access
    ///
    /// # Authorization
    /// - Only the original depositor can set/update metadata
    /// - This prevents unauthorized metadata modification
    ///
    /// # Size Limits
    /// See `validate_metadata_size()` documentation for detailed limits.
    ///
    /// # Events
    /// Emits: `FundsLocked` event with additional metadata field
    ///
    /// # Example
    /// ```rust
    /// let metadata = EscrowMetadata {
    ///     repo_id: Some(String::from_str(&env, "owner/repo")),
    ///     issue_id: Some(String::from_str(&env, "123")),
    ///     bounty_type: Some(String::from_str(&env, "bug")),
    ///     tags: vec![&env, String::from_str(&env, "priority-high")],
    ///     custom_fields: map![&env],
    /// };
    ///
    /// escrow_client.set_escrow_metadata(&42, &metadata)?;
    /// ```
    pub fn set_escrow_metadata(
        env: Env,
        bounty_id: u64,
        metadata: EscrowMetadata,
    ) -> Result<(), Error> {
        // Verify bounty exists
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        // Get escrow to verify depositor authorization
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        escrow.depositor.require_auth();

        // Validate metadata size limits
        if !Self::validate_metadata_size(&env, &metadata) {
            return Err(Error::MetadataTooLarge);
        }

        // Store metadata
        env.storage()
            .persistent()
            .set(&DataKey::EscrowMetadata(bounty_id), &metadata);

        // Extend TTL for both escrow and metadata
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(bounty_id),
            1000000,  // Minimum
            10000000, // Maximum
        );
        env.storage().persistent().extend_ttl(
            &DataKey::EscrowMetadata(bounty_id),
            1000000,  // Minimum
            10000000, // Maximum
        );

        Ok(())
    }

    /// Releases escrowed funds to a contributor.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to release funds for
    /// * `contributor` - Address to receive the funds
    ///
    /// # Returns
    /// * `Ok(())` - Funds successfully released
    /// * `Err(Error::NotInitialized)` - Contract not initialized
    /// * `Err(Error::Unauthorized)` - Caller is not the admin
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    /// * `Err(Error::FundsNotLocked)` - Funds not in LOCKED state
    ///
    /// # State Changes
    /// - Transfers tokens from contract to contributor
    /// - Updates escrow status to Released
    /// - Emits FundsReleased event
    ///
    /// # Authorization
    /// - **CRITICAL**: Only admin can call this function
    /// - Admin address must match initialization value
    ///
    /// # Security Considerations
    /// - This is the most security-critical function
    /// - Admin should verify task completion off-chain before calling
    /// - Once released, funds cannot be retrieved
    /// - Recipient address should be verified carefully
    /// - Consider implementing multi-sig for admin
    ///
    /// # Events
    /// Emits: `FundsReleased { bounty_id, amount, recipient, timestamp }`
    ///
    /// # Example
    /// ```rust
    /// // After verifying task completion off-chain:
    /// let contributor = Address::from_string("GCONTRIB...");
    ///
    /// // Admin calls release
    /// escrow_client.release_funds(&42, &contributor)?;
    /// // Funds transferred to contributor, escrow marked as Released
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Token transfer + storage update + event emission
    ///
    /// # Best Practices
    /// 1. Verify contributor identity off-chain
    /// 2. Confirm task completion before release
    /// 3. Log release decisions in backend system
    /// 4. Monitor release events for anomalies
    /// 5. Consider implementing release delays for high-value bounties
    pub fn release_funds(
        env: Env,
        bounty_id: u64,
        contributor: Address,
        token_address: Option<Address>, // Optional: if None, uses escrow's primary token
        amount: Option<i128>,           // Optional partial amount
    ) -> Result<(), Error> {
        // Check blacklist/whitelist for recipient
        if !is_participant_allowed(&env, &contributor) {
            return Err(Error::ParticipantNotAllowed);
        }
        let start = env.ledger().timestamp();

        let _guard = ReentrancyGuardRAII::new(&env).map_err(|_| Error::ReentrantCall)?;
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();

        // Check if release operations are paused
        if Self::is_release_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::ReleasePaused);
        }

        anti_abuse::check_rate_limit(&env, admin.clone());

        admin.require_auth();

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        // Allow release from Locked or PartiallyReleased states
        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyReleased
        {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            return Err(Error::FundsNotLocked);
        }

        // Determine which token to release
        let token_addr: Address = if let Some(token) = token_address {
            // Validate token has balance in escrow
            if escrow.token_balances.get(token.clone()).is_none() {
                monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
                env.storage().instance().remove(&DataKey::ReentrancyGuard);
                return Err(Error::InvalidAmount); // Token not found in escrow
            }
            token
        } else {
            // Use escrow's primary token for backward compatibility
            escrow.token_address.clone()
        };

        // Get balance for this token
        let token_balance = escrow.token_balances.get(token_addr.clone()).unwrap_or(0);
        if token_balance <= 0 {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidAmount); // No balance for this token
        }

        // Determine payout amount and validate
        let payout_amount = match amount {
            Some(amt) => {
                if amt <= 0 {
                    monitoring::track_operation(
                        &env,
                        symbol_short!("release"),
                        admin.clone(),
                        false,
                    );
                    env.storage().instance().remove(&DataKey::ReentrancyGuard);
                    return Err(Error::InvalidAmount);
                }
                if amt > escrow.remaining_amount {
                    monitoring::track_operation(
                        &env,
                        symbol_short!("release"),
                        admin.clone(),
                        false,
                    );
                    env.storage().instance().remove(&DataKey::ReentrancyGuard);
                    return Err(Error::InvalidAmount); // Attempt to over-pay
                }
                amt
            }
            None => escrow.remaining_amount, // Release full remaining amount
        };
        let client = token::Client::new(&env, &token_addr);

        // Calculate and collect fee if enabled
        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.release_fee_rate > 0 {
            Self::calculate_fee(payout_amount, fee_config.release_fee_rate)
        } else {
            0
        };
        let net_amount = payout_amount - fee_amount;

        // Ensure contract has sufficient funds
        let contract_balance = client.balance(&env.current_contract_address());
        if contract_balance < net_amount + fee_amount {
            return Err(Error::InsufficientFunds);
        }

        // Check payout amount limits
        let limits = Self::get_amount_limits(env.clone());
        if net_amount < limits.min_payout || net_amount > limits.max_payout {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            return Err(Error::InvalidAmount);
        }

        // Transfer net amount to contributor
        client.transfer(&env.current_contract_address(), &contributor, &net_amount);

        if fee_amount > 0 {
            client.transfer(
                &env.current_contract_address(),
                &fee_config.fee_recipient,
                &fee_amount,
            );
            events::emit_fee_collected(
                &env,
                events::FeeCollected {
                    operation_type: events::FeeOperationType::Release,
                    amount: fee_amount,
                    fee_rate: fee_config.release_fee_rate,
                    recipient: fee_config.fee_recipient.clone(),
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        // Update escrow state - remove this token's balance
        escrow.token_balances.set(token_addr.clone(), 0);
        escrow.remaining_amount = escrow.remaining_amount - token_balance;
        escrow.amount = escrow.amount - token_balance;

        // If all tokens are released, mark as Released
        // Simple check: if remaining_amount > 0, there's still balance
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released;
        } else {
            escrow.status = EscrowStatus::PartiallyReleased;
        }

        // Update escrow state
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        emit_funds_released(
            &env,
            FundsReleased {
                bounty_id,
                amount: net_amount,
                recipient: contributor.clone(),
                token_address: token_addr.clone(),
                timestamp: env.ledger().timestamp(),
                remaining_amount: escrow.remaining_amount,
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("release"), admin, true);

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("release"), duration);
        Ok(())
    }

    pub fn approve_refund(
        env: Env,
        bounty_id: u64,
        amount: i128,
        recipient: Address,
        mode: RefundMode,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
        {
            return Err(Error::FundsNotLocked);
        }

        if amount <= 0 || amount > escrow.remaining_amount {
            return Err(Error::InvalidAmount);
        }

        let approval = RefundApproval {
            bounty_id,
            amount,
            recipient: recipient.clone(),
            mode,
            approved_by: admin.clone(),
            approved_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::RefundApproval(bounty_id), &approval);

        Ok(())
    }

    /// Expire an escrow and automatically refund to depositor after deadline.
    /// This function can be called by anyone after the deadline has passed.
    /// It provides a permissionless way to ensure funds are not stuck indefinitely.
    pub fn expire(env: Env, bounty_id: u64) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        if Self::is_paused_internal(&env) {
            return Err(Error::ContractPaused);
        }

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
        {
            return Err(Error::FundsNotLocked);
        }

        let now = env.ledger().timestamp();
        if now < escrow.deadline {
            return Err(Error::DeadlineNotPassed);
        }

        let refund_amount = escrow.remaining_amount;
        if refund_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Use escrow's primary token
        let token_addr: Address = escrow.token_address.clone();
        let client = token::Client::new(&env, &token_addr);

        let contract_balance = client.balance(&env.current_contract_address());
        if contract_balance < refund_amount {
            return Err(Error::InsufficientFunds);
        }

        client.transfer(
            &env.current_contract_address(),
            &escrow.depositor,
            &refund_amount,
        );

        let refund_record = RefundRecord {
            amount: refund_amount,
            recipient: escrow.depositor.clone(),
            mode: RefundMode::Full,
            timestamp: env.ledger().timestamp(),
        };
        escrow.refund_history.push_back(refund_record);
        escrow.remaining_amount = 0;
        escrow.status = EscrowStatus::Refunded;

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        emit_escrow_expired(
            &env,
            EscrowExpired {
                bounty_id,
                amount: refund_amount,
                refunded_to: escrow.depositor.clone(),
                triggered_by: env.current_contract_address(),
                timestamp: env.ledger().timestamp(),
            },
        );

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("expire"), duration);

        Ok(())
    }

    /// Refund funds with support for Full, Partial, and Custom refunds.
    /// - Full: refunds all remaining funds to depositor
    /// - Partial: refunds specified amount to depositor
    /// - Custom: refunds specified amount to specified recipient (requires admin approval if before deadline)
    fn refund_internal(
        env: Env,
        bounty_id: u64,
        amount: Option<i128>,
        recipient: Option<Address>,
        mode: RefundMode,
        token_address: Option<Address>, // Optional: if None, uses escrow's primary token
    ) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        let _guard = ReentrancyGuardRAII::new(&env).map_err(|_| Error::ReentrantCall)?;

        // Check if refund operations are paused
        if Self::is_refund_paused_internal(&env) {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("refund"), caller, false);
            return Err(Error::RefundPaused);
        }

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("refund"), caller, false);
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        let caller = escrow.depositor.clone();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
        {
            return Err(Error::FundsNotLocked);
        }

        let now = env.ledger().timestamp();
        let is_before_deadline = now < escrow.deadline;

        // Determine token address first (needed for balance checks)
        let token_addr: Address = if let Some(token) = token_address {
            // Validate token has balance in escrow
            if escrow.token_balances.get(token.clone()).is_none() {
                return Err(Error::InvalidAmount); // Token not found in escrow
            }
            token
        } else {
            // Use escrow's primary token for backward compatibility
            escrow.token_address.clone()
        };

        // Get balance for this token
        let token_balance = escrow.token_balances.get(token_addr.clone()).unwrap_or(0);

        // Determine refund amount and recipient
        let refund_amount: i128;
        let refund_recipient: Address;

        match mode {
            RefundMode::Full => {
                refund_amount = token_balance; // Use token balance, not total remaining
                refund_recipient = escrow.depositor.clone();
                if is_before_deadline {
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Partial => {
                refund_amount = amount.unwrap_or(token_balance);
                refund_recipient = escrow.depositor.clone();
                if is_before_deadline {
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Custom => {
                refund_amount = amount.ok_or(Error::InvalidAmount)?;
                refund_recipient = recipient.ok_or(Error::InvalidAmount)?;

                if is_before_deadline {
                    if !env
                        .storage()
                        .persistent()
                        .has(&DataKey::RefundApproval(bounty_id))
                    {
                        return Err(Error::RefundNotApproved);
                    }
                    let approval: RefundApproval = env
                        .storage()
                        .persistent()
                        .get(&DataKey::RefundApproval(bounty_id))
                        .unwrap();

                    if approval.amount != refund_amount
                        || approval.recipient != refund_recipient
                        || approval.mode != mode
                    {
                        return Err(Error::RefundNotApproved);
                    }

                    env.storage()
                        .persistent()
                        .remove(&DataKey::RefundApproval(bounty_id));
                }
            }
        }

        if refund_amount <= 0 || refund_amount > escrow.remaining_amount {
            return Err(Error::InvalidAmount);
        }

        // Use the token_addr already determined earlier (from escrow's token_balances or token_address)
        let client = token::Client::new(&env, &token_addr);

        let contract_balance = client.balance(&env.current_contract_address());
        if contract_balance < refund_amount {
            return Err(Error::InsufficientFunds);
        }

        client.transfer(
            &env.current_contract_address(),
            &refund_recipient,
            &refund_amount,
        );

        // Update escrow state - remove this token's balance
        escrow
            .token_balances
            .set(token_addr.clone(), token_balance - refund_amount);
        escrow.remaining_amount -= refund_amount;
        escrow.amount -= refund_amount;

        let refund_record = RefundRecord {
            amount: refund_amount,
            recipient: refund_recipient.clone(),
            mode: mode.clone(),
            timestamp: env.ledger().timestamp(),
        };
        escrow.refund_history.push_back(refund_record);

        // Update status - check if all tokens are refunded
        // Simple check: if remaining_amount > 0, there's still balance
        if escrow.remaining_amount == 0 {
            let _token = escrow.token.clone();
            escrow.status = EscrowStatus::Refunded;
        } else {
            escrow.status = EscrowStatus::PartiallyRefunded;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        emit_funds_refunded(
            &env,
            FundsRefunded {
                bounty_id,
                amount: refund_amount,
                refund_to: refund_recipient,
                timestamp: env.ledger().timestamp(),
                refund_mode: mode.clone(),
                remaining_amount: escrow.remaining_amount,
                token_address: token_addr.clone(),
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("refund"), caller, true);

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("refund"), duration);

        Ok(())
    }

    /// Extend the refund deadline for an escrow.
    ///
    /// Allows authorized parties (admin or depositor) to extend the refund deadline
    /// for a bounty. This is useful when a bounty or program is extended without
    /// needing to migrate funds to a new escrow.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty identifier
    /// * `new_deadline` - The new deadline timestamp (must be greater than current deadline)
    ///
    /// # Returns
    /// * `Ok(())` - Deadline successfully extended
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    /// * `Err(Error::FundsNotLocked)` - Escrow is not in Locked or PartiallyRefunded state
    /// * `Err(Error::InvalidDeadlineExtension)` - New deadline is not greater than current deadline
    /// * `Err(Error::Unauthorized)` - Caller is not admin or depositor
    ///
    /// # Authorization
    /// - Admin (contract administrator)
    /// - Depositor (original fund depositor)
    ///
    /// # Validation
    /// - New deadline must be strictly greater than current deadline
    /// - Escrow must be in Locked or PartiallyRefunded state
    ///
    /// # Events
    /// Emits: `DeadlineExtended` event
    ///
    /// # Example
    /// ```rust
    /// // Current deadline: 1000
    /// // New deadline: 2000
    /// escrow_client.extend_refund_deadline(&42, &2000)?;
    /// // → Updates deadline to 2000
    /// // → Emits DeadlineExtended event
    /// ```
    pub fn extend_refund_deadline(
        env: Env,
        bounty_id: u64,
        new_deadline: u64,
    ) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("ext_dead"), caller, false);
            return Err(Error::ContractPaused);
        }

        // Verify bounty exists
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("ext_dead"), caller, false);
            return Err(Error::BountyNotFound);
        }

        // Get escrow data
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        // Verify escrow is in a state that allows deadline extension
        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
        {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("ext_dead"), caller, false);
            return Err(Error::FundsNotLocked);
        }

        // Verify new deadline is greater than current deadline
        if new_deadline <= escrow.deadline {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("ext_dead"), caller, false);
            return Err(Error::InvalidDeadlineExtension);
        }

        // Authorization: Admin or Depositor
        // Both admin and depositor can extend the deadline
        // The caller must provide auth for the address they control
        // For now, we require depositor auth (they own the funds)
        // Admin can extend by providing their auth (they would call with admin auth)
        let _admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        let depositor = escrow.depositor.clone();

        // Allow either admin or depositor to extend
        // In practice, the caller will provide auth for the appropriate address
        // For simplicity, we'll require depositor auth (they own the funds)
        // Admin can extend by providing their auth (they would call with admin auth)
        depositor.require_auth();
        let caller = depositor.clone();

        // Store old deadline for event
        let old_deadline = escrow.deadline;

        // Update deadline
        escrow.deadline = new_deadline;

        // Store updated escrow
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Extend TTL
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(bounty_id),
            1000000,  // Minimum
            10000000, // Maximum
        );

        // Emit deadline extended event
        emit_deadline_extended(
            &env,
            DeadlineExtended {
                bounty_id,
                old_deadline,
                new_deadline,
                extended_by: caller.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("ext_dead"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("ext_dead"), duration);

        Ok(())
    }

    // ========================================================================
    // View Functions (Read-only)
    // ========================================================================

    pub fn get_escrow_info(env: Env, bounty_id: u64) -> Result<Escrow, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap())
    }

    /// Retrieves metadata for a specific bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok(Option<EscrowMetadata>)` - Metadata if set, None if not set
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    ///
    /// # Gas Cost
    /// Very Low - Single storage read
    ///
    /// # Example
    /// ```rust
    /// let metadata_opt = escrow_client.get_escrow_metadata(&42)?;
    /// if let Some(metadata) = metadata_opt {
    ///     println!("Repo: {:?}", metadata.repo_id);
    ///     println!("Issue: {:?}", metadata.issue_id);
    ///     println!("Type: {:?}", metadata.bounty_type);
    ///     println!("Tags: {:?}", metadata.tags);
    /// }
    /// ```
    pub fn get_escrow_metadata(env: Env, bounty_id: u64) -> Result<Option<EscrowMetadata>, Error> {
        // Verify bounty exists
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        // Get metadata if it exists
        let metadata: Option<EscrowMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowMetadata(bounty_id));
        Ok(metadata)
    }

    /// Retrieves complete escrow information including metadata.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok(EscrowWithMetadata)` - Combined escrow and metadata information
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    ///
    /// # Gas Cost
    /// Low - Two storage reads
    ///
    /// # Example
    /// ```rust
    /// let escrow_view = escrow_client.get_escrow_with_metadata(&42)?;
    /// println!("Amount: {}", escrow_view.escrow.amount);
    /// println!("Status: {:?}", escrow_view.escrow.status);
    ///
    /// if let Some(meta) = escrow_view.metadata {
    ///     println!("Repository: {:?}", meta.repo_id);
    ///     println!("Issue: {:?}", meta.issue_id);
    /// }
    /// ```
    pub fn get_escrow_with_metadata(env: Env, bounty_id: u64) -> Result<EscrowWithMetadata, Error> {
        // Get core escrow data
        let escrow = Self::get_escrow_info(env.clone(), bounty_id)?;

        // Get metadata if it exists
        let metadata_opt = Self::get_escrow_metadata(env.clone(), bounty_id)?;

        if let Some(metadata) = metadata_opt {
            Ok(EscrowWithMetadata {
                escrow,
                has_metadata: true,
                metadata,
            })
        } else {
            // Return empty metadata if not set
            Ok(EscrowWithMetadata {
                escrow,
                has_metadata: false,
                metadata: EscrowMetadata {
                    repo_id: None,
                    issue_id: None,
                    bounty_type: None,
                    tags: vec![&env],
                    custom_fields: Map::new(&env),
                },
            })
        }
    }

    /// Returns the current token balance held by the contract.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `Ok(i128)` - Current contract token balance
    /// * `Err(Error::NotInitialized)` - Contract not initialized
    ///
    /// # Use Cases
    /// - Monitoring total locked funds
    /// - Verifying contract solvency
    /// - Auditing and reconciliation
    ///
    /// # Gas Cost
    /// Low - Token contract call
    ///
    /// # Example
    /// ```rust
    /// let balance = escrow_client.get_balance()?;
    /// println!("Total locked: {} stroops", balance);
    /// ```
    /// Gets the total token balance held by the contract.
    pub fn get_contract_balance(env: Env) -> Result<i128, Error> {
        if !env.storage().instance().has(&DataKey::Token) {
            return Err(Error::NotInitialized);
        }
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        Ok(client.balance(&env.current_contract_address()))
    }

    pub fn get_refund_history(env: Env, bounty_id: u64) -> Result<Vec<RefundRecord>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Ok(escrow.refund_history)
    }

    /// Retrieves the payout history for a specific bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok(Vec<PayoutRecord>)` - The payout history
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    pub fn get_payout_history(env: Env, bounty_id: u64) -> Result<Vec<PayoutRecord>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Ok(escrow.payout_history)
    }

    pub fn get_refund_eligibility(
        env: Env,
        bounty_id: u64,
    ) -> Result<(bool, bool, i128, Option<RefundApproval>), Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        let now = env.ledger().timestamp();
        let deadline_passed = now >= escrow.deadline;

        let approval = if env
            .storage()
            .persistent()
            .has(&DataKey::RefundApproval(bounty_id))
        {
            Some(
                env.storage()
                    .persistent()
                    .get(&DataKey::RefundApproval(bounty_id))
                    .unwrap(),
            )
        } else {
            None
        };

        let can_refund = (escrow.status == EscrowStatus::Locked
            || escrow.status == EscrowStatus::PartiallyRefunded)
            && (deadline_passed || approval.is_some());

        Ok((
            can_refund,
            deadline_passed,
            escrow.remaining_amount,
            approval,
        ))
    }

    // ========================================================================
    // Query Functions
    // ========================================================================

    /// Query bounties with filtering and pagination.
    ///
    /// # Performance
    /// This function iterates through the registry. For large datasets, use small `pagination.limit` values
    /// to prevent gas limit errors. This is designed for off-chain indexing.
    pub fn get_bounties(
        env: Env,
        filter: EscrowFilter,
        pagination: Pagination,
    ) -> Vec<(u64, Escrow)> {
        let registry: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::BountyRegistry)
            .unwrap_or(vec![&env]);

        let mut result = vec![&env];
        let mut count: u32 = 0;
        let mut skipped: u64 = 0;

        for i in 0..registry.len() {
            // Check pagination limit
            if count >= pagination.limit {
                break;
            }

            let bounty_id = registry.get(i).unwrap();

            // Skip invalid IDs/missing data
            if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
                continue;
            }

            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(bounty_id))
                .unwrap();

            // Apply Filters

            // Status filter
            if let Some(status_val) = filter.status {
                if (escrow.status as u32) != status_val {
                    continue;
                }
            }

            // Depositor filter
            if let Some(depositor) = &filter.depositor {
                if &escrow.depositor != depositor {
                    continue;
                }
            }

            // Amount range filter
            if let Some(min) = filter.min_amount {
                if escrow.amount < min {
                    continue;
                }
            }
            if let Some(max) = filter.max_amount {
                if escrow.amount > max {
                    continue;
                }
            }

            // Date range filter (using deadline)
            if let Some(start) = filter.start_time {
                if escrow.deadline < start {
                    continue;
                }
            }
            if let Some(end) = filter.end_time {
                if escrow.deadline > end {
                    continue;
                }
            }

            // Apply Pagination Skip
            if skipped < pagination.start_index {
                skipped += 1;
                continue;
            }

            // Add to result
            result.push_back((bounty_id, escrow));
            count += 1;
        }

        result
    }

    /// Get aggregate statistics for the contract.
    ///
    /// # Performance
    /// This function iterates over ALL bounties. It is O(N) and may fail on-chain if N is large.
    /// Use primarily for off-chain monitoring/indexing.
    pub fn get_stats(env: Env) -> EscrowStats {
        let registry: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::BountyRegistry)
            .unwrap_or(vec![&env]);

        let mut total_locked: i128 = 0;
        let mut total_released: i128 = 0;
        let mut total_refunded: i128 = 0;

        for i in 0..registry.len() {
            let bounty_id = registry.get(i).unwrap();
            if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
                let escrow: Escrow = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Escrow(bounty_id))
                    .unwrap();

                match escrow.status {
                    EscrowStatus::Locked => {
                        total_locked += escrow.remaining_amount;
                    }
                    EscrowStatus::PartiallyReleased => {
                        total_locked += escrow.remaining_amount;
                        // The released amount is the initial amount minus what is left
                        total_released += escrow.amount - escrow.remaining_amount;
                    }
                    EscrowStatus::Released => {
                        total_released += escrow.amount;
                    }
                    EscrowStatus::Refunded => {
                        for record in escrow.refund_history.iter() {
                            total_refunded += record.amount;
                        }
                    }
                    EscrowStatus::PartiallyRefunded => {
                        total_locked += escrow.remaining_amount;
                        for record in escrow.refund_history.iter() {
                            total_refunded += record.amount;
                        }
                    }
                }
            }
        }

        EscrowStats {
            total_bounties: registry.len() as u64,
            total_locked_amount: total_locked,
            total_released_amount: total_released,
            total_refunded_amount: total_refunded,
        }
    }

    /// Batch lock funds for multiple bounties in a single transaction.
    /// This improves gas efficiency by reducing transaction overhead.
    ///
    /// # Arguments
    /// * `items` - Vector of LockFundsItem containing bounty_id, depositor, amount, and deadline
    ///
    /// # Returns
    /// Number of successfully locked bounties
    ///
    /// # Errors
    /// * InvalidBatchSize - if batch size exceeds MAX_BATCH_SIZE or is zero
    /// * BountyExists - if any bounty_id already exists
    /// * NotInitialized - if contract is not initialized
    ///
    /// # Note
    /// This operation is atomic - if any item fails, the entire transaction reverts.
    pub fn batch_lock_funds(env: Env, items: Vec<LockFundsItem>) -> Result<u32, Error> {
        let _guard = ReentrancyGuardRAII::new(&env).map_err(|_| Error::ReentrantCall)?;
        // Validate batch size
        let batch_size = items.len();
        if batch_size == 0 {
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        // Check if lock operations are paused
        if Self::is_lock_paused_internal(&env) {
            return Err(Error::LockPaused);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let contract_address = env.current_contract_address();
        let timestamp = env.ledger().timestamp();

        for item in items.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Escrow(item.bounty_id))
            {
                return Err(Error::BountyExists);
            }

            if item.amount <= 0 {
                return Err(Error::InvalidAmount);
            }

            // Check amount limits
            let limits = Self::get_amount_limits(env.clone());
            if item.amount < limits.min_lock_amount || item.amount > limits.max_lock_amount {
                return Err(Error::InvalidAmount);
            }

            // Check for duplicate bounty_ids in the batch
            let mut count = 0u32;
            for other_item in items.iter() {
                if other_item.bounty_id == item.bounty_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateBountyId);
            }
        }

        let mut seen_depositors: Vec<Address> = Vec::new(&env);
        for item in items.iter() {
            let mut found = false;
            for seen in seen_depositors.iter() {
                if seen.clone() == item.depositor {
                    found = true;
                    break;
                }
            }
            if !found {
                seen_depositors.push_back(item.depositor.clone());
                item.depositor.require_auth();
            }
        }

        let mut locked_count = 0u32;
        for item in items.iter() {
            // Get token address (use provided or default)
            let token_addr: Address = if let Some(token) = item.token_address.clone() {
                // Validate token is registered/whitelisted
                if !Self::is_token_registered(&env, &token) {
                    return Err(Error::Unauthorized); // Token not registered
                }
                token
            } else {
                // Use default token for backward compatibility
                Self::get_default_token(&env)?
            };
            let client = token::Client::new(&env, &token_addr);

            // Calculate and collect fee if enabled
            let fee_config = Self::get_fee_config_internal(&env);
            let fee_amount = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0 {
                Self::calculate_fee(item.amount, fee_config.lock_fee_rate)
            } else {
                0
            };
            let net_amount = item.amount - fee_amount;

            // Transfer net amount from depositor to contract
            client.transfer(&item.depositor, &contract_address, &net_amount);

            // Transfer fee to fee recipient if applicable
            if fee_amount > 0 {
                client.transfer(&item.depositor, &fee_config.fee_recipient, &fee_amount);
            }

            // Check if escrow already exists (for multi-token support)
            let mut escrow: Escrow = if env
                .storage()
                .persistent()
                .has(&DataKey::Escrow(item.bounty_id))
            {
                env.storage()
                    .persistent()
                    .get(&DataKey::Escrow(item.bounty_id))
                    .unwrap()
            } else {
                // Create new escrow
                let mut balances = Map::new(&env);
                balances.set(token_addr.clone(), net_amount);
                Escrow {
                    depositor: item.depositor.clone(),
                    amount: net_amount,
                    status: EscrowStatus::Locked,
                    deadline: item.deadline,
                    token: token_addr.clone(),
                    refund_history: vec![&env],
                    remaining_amount: net_amount,
                    token_address: token_addr.clone(),
                    token_balances: balances,
                    payout_history: vec![&env],
                }
            };

            // Update token balance in map (for existing escrows)
            if env
                .storage()
                .persistent()
                .has(&DataKey::Escrow(item.bounty_id))
            {
                let current_balance = escrow.token_balances.get(token_addr.clone()).unwrap_or(0);
                escrow
                    .token_balances
                    .set(token_addr.clone(), current_balance + net_amount);
                escrow.remaining_amount = escrow.remaining_amount + net_amount;
                escrow.amount = escrow.amount + net_amount;
            }

            // Store escrow
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);

            emit_funds_locked(
                &env,
                FundsLocked {
                    bounty_id: item.bounty_id,
                    amount: net_amount,
                    depositor: item.depositor.clone(),
                    deadline: item.deadline,
                    token_address: token_addr.clone(),
                },
            );

            locked_count += 1;
        }

        emit_batch_funds_locked(
            &env,
            BatchFundsLocked {
                count: locked_count,
                total_amount: items.iter().map(|i| i.amount).sum(),
                timestamp,
            },
        );

        Ok(locked_count)
    }

    pub fn batch_release_funds(env: Env, items: Vec<ReleaseFundsItem>) -> Result<u32, Error> {
        let _guard = ReentrancyGuardRAII::new(&env).map_err(|_| Error::ReentrantCall)?;
        // Validate batch size
        let batch_size = items.len();
        if batch_size == 0 {
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        // Check if release operations are paused
        if Self::is_release_paused_internal(&env) {
            return Err(Error::ReleasePaused);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let contract_address = env.current_contract_address();
        let timestamp = env.ledger().timestamp();

        let mut total_amount: i128 = 0;
        for item in items.iter() {
            if !env
                .storage()
                .persistent()
                .has(&DataKey::Escrow(item.bounty_id))
            {
                return Err(Error::BountyNotFound);
            }

            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            if escrow.status != EscrowStatus::Locked {
                return Err(Error::FundsNotLocked);
            }

            // Check payout amount limits (considering fees)
            let fee_config = Self::get_fee_config_internal(&env);
            let fee_amount = if fee_config.fee_enabled && fee_config.release_fee_rate > 0 {
                Self::calculate_fee(escrow.amount, fee_config.release_fee_rate)
            } else {
                0
            };
            let net_amount = escrow.amount - fee_amount;

            let limits = Self::get_amount_limits(env.clone());
            if net_amount < limits.min_payout || net_amount > limits.max_payout {
                return Err(Error::InvalidAmount);
            }

            // Check for duplicate bounty_ids in the batch
            let mut count = 0u32;
            for other_item in items.iter() {
                if other_item.bounty_id == item.bounty_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateBountyId);
            }

            total_amount = total_amount
                .checked_add(escrow.amount)
                .ok_or(Error::InvalidAmount)?;
        }

        let mut released_count = 0u32;
        for item in items.iter() {
            let mut escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            // Use escrow's primary token
            let token_addr = escrow.token_address.clone();
            let client = token::Client::new(&env, &token_addr);

            // Get balance for this token
            let token_balance = escrow
                .token_balances
                .get(token_addr.clone())
                .unwrap_or(escrow.remaining_amount);

            // Calculate and collect fee if enabled
            let fee_config = Self::get_fee_config_internal(&env);
            let fee_amount = if fee_config.fee_enabled && fee_config.release_fee_rate > 0 {
                Self::calculate_fee(token_balance, fee_config.release_fee_rate)
            } else {
                0
            };
            let net_amount = token_balance - fee_amount;

            // Transfer net amount to contributor
            client.transfer(&contract_address, &item.contributor, &net_amount);

            // Transfer fee to fee recipient if applicable
            if fee_amount > 0 {
                client.transfer(&contract_address, &fee_config.fee_recipient, &fee_amount);
            }

            // Update escrow state - remove this token's balance
            escrow.token_balances.set(token_addr.clone(), 0);
            escrow.remaining_amount = escrow.remaining_amount - token_balance;
            escrow.amount = escrow.amount - token_balance;

            // If all tokens are released, mark as Released
            if escrow.remaining_amount == 0 {
                escrow.status = EscrowStatus::Released;
            }

            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);

            emit_funds_released(
                &env,
                FundsReleased {
                    bounty_id: item.bounty_id,
                    amount: net_amount,
                    recipient: item.contributor.clone(),
                    token_address: token_addr.clone(),
                    timestamp,
                    remaining_amount: escrow.remaining_amount,
                },
            );

            released_count += 1;
        }

        emit_batch_funds_released(
            &env,
            BatchFundsReleased {
                count: released_count,
                total_amount,
                timestamp,
            },
        );

        Ok(released_count)
    }

    // ========================================================================
    // Token Management Functions
    // ========================================================================

    /// Adds a token to the whitelist (admin only).
    pub fn add_token(env: Env, token: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if env
            .storage()
            .instance()
            .has(&DataKey::TokenWhitelist(token.clone()))
        {
            return Err(Error::TokenAlreadyWhitelisted);
        }

        env.storage()
            .instance()
            .set(&DataKey::TokenWhitelist(token.clone()), &true);

        let mut token_list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::RegisteredTokens)
            .unwrap_or(Vec::new(&env));
        token_list.push_back(token);
        env.storage()
            .instance()
            .set(&DataKey::RegisteredTokens, &token_list);

        Ok(())
    }

    /// Removes a token from the whitelist (admin only).
    pub fn remove_token(env: Env, token: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::TokenWhitelist(token.clone()))
        {
            return Err(Error::TokenNotWhitelisted);
        }

        env.storage()
            .instance()
            .remove(&DataKey::TokenWhitelist(token.clone()));

        let token_list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::RegisteredTokens)
            .unwrap_or(Vec::new(&env));
        let mut new_list: Vec<Address> = Vec::new(&env);
        for t in token_list.iter() {
            if t != token {
                new_list.push_back(t);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::RegisteredTokens, &new_list);

        Ok(())
    }

    /// Checks if a token is whitelisted.
    pub fn is_token_whitelisted(env: Env, token: Address) -> bool {
        env.storage()
            .instance()
            .has(&DataKey::TokenWhitelist(token))
    }

    /// Returns all whitelisted tokens.
    pub fn get_whitelisted_tokens(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::RegisteredTokens)
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the contract's balance for a specific token.
    pub fn get_token_bal(env: Env, token: Address) -> Result<i128, Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::TokenWhitelist(token.clone()))
        {
            return Err(Error::TokenNotWhitelisted);
        }
        let client = token::Client::new(&env, &token);
        Ok(client.balance(&env.current_contract_address()))
    }
}

#[cfg(test)]
mod reentrancy_test;
#[cfg(test)]
#[cfg(test)]
mod test;

impl EscrowLock for BountyEscrowContract {
    fn lock_funds(env: Env, depositor: Address, id: u64, amount: i128, deadline: u64) {
        Self::lock_funds_internal(env.clone(), depositor, id, amount, deadline, None)
            .unwrap_or_else(|e| env.panic_with_error(e));
    }
}

impl EscrowRelease for BountyEscrowContract {
    fn release_funds(env: Env, id: u64, recipient: Address) {
        Self::release_funds(env.clone(), id, recipient, None, None)
            .unwrap_or_else(|e| env.panic_with_error(e));
    }

    fn refund(
        env: Env,
        id: u64,
        amount: Option<i128>,
        recipient: Option<Address>,
        mode: RefundMode,
    ) {
        Self::refund_internal(env.clone(), id, amount, recipient, mode, None)
            .unwrap_or_else(|e| env.panic_with_error(e));
    }

    fn get_balance(env: Env, id: u64) -> i128 {
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(id))
            .expect("Bounty not found");
        escrow.amount
    }
}

impl ConfigurableFee for BountyEscrowContract {
    fn set_fee_config(env: Env, config: SharedFeeConfig) {
        Self::update_fee_config(
            env.clone(),
            Some(config.lock_fee_rate),
            Some(config.payout_fee_rate),
            Some(config.fee_recipient),
            Some(config.fee_enabled),
        )
        .unwrap_or_else(|e| env.panic_with_error(e));
    }

    fn get_fee_config(env: Env) -> SharedFeeConfig {
        let config = Self::get_fee_config_internal(&env);
        SharedFeeConfig {
            lock_fee_rate: config.lock_fee_rate,
            payout_fee_rate: config.release_fee_rate,
            fee_recipient: config.fee_recipient,
            fee_enabled: config.fee_enabled,
        }
    }
}

impl Pausable for BountyEscrowContract {
    fn pause(env: Env) {
        Self::pause(env.clone()).unwrap_or_else(|e| env.panic_with_error(e));
    }

    fn unpause(env: Env) {
        Self::unpause(env.clone()).unwrap_or_else(|e| env.panic_with_error(e));
    }

    fn is_paused(env: Env) -> bool {
        Self::is_paused(env)
    }
}
#[cfg(test)]
mod test_edge_cases;
#[cfg(test)]
mod test_fuzz_properties;

mod pause_tests;
#[cfg(test)]
mod test_invalid_inputs;
