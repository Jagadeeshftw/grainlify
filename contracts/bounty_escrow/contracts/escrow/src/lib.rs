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
mod events;
mod test_bounty_escrow;

use events::{
    emit_batch_funds_locked, emit_batch_funds_released, emit_bounty_initialized, emit_funds_locked,
    emit_funds_refunded, emit_funds_released, BatchFundsLocked, BatchFundsReleased,
    BountyEscrowInitialized, FundsLocked, FundsRefunded, FundsReleased,
};

// Event symbols for release schedules
const SCHEDULE_CREATED: soroban_sdk::Symbol = soroban_sdk::symbol_short!("sch_crt");
const SCHEDULE_RELEASED: soroban_sdk::Symbol = soroban_sdk::symbol_short!("sch_rel");
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    Vec,
};


// ==================== MONITORING MODULE ====================
mod monitoring {
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

    // Storage keys
    const OPERATION_COUNT: &str = "op_count";
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

    // Data: State snapshot
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct StateSnapshot {
        pub timestamp: u64,
        pub total_operations: u64,
        pub total_users: u64,
        pub total_errors: u64,
    }

    // Data: Performance stats
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceStats {
        pub function_name: Symbol,
        pub call_count: u64,
        pub total_time: u64,
        pub avg_time: u64,
        pub last_called: u64,
    }

    // Track operation
    pub fn track_operation(env: &Env, operation: Symbol, caller: Address, success: bool) {
        let timestamp = env.ledger().timestamp();
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
                timestamp,
                success,
            },
        );
    }

    // Track performance
    pub fn emit_performance(env: &Env, function: Symbol, duration: u64) {
        let timestamp = env.ledger().timestamp();
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
                timestamp,
            },
        );
    }

    // Health check
    pub fn health_check(env: &Env) -> HealthStatus {
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
    pub fn get_state_snapshot(env: &Env) -> StateSnapshot {
        let timestamp = env.ledger().timestamp();
        let op_key = Symbol::new(env, OPERATION_COUNT);
        let usr_key = Symbol::new(env, USER_COUNT);
        let err_key = Symbol::new(env, ERROR_COUNT);

        StateSnapshot {
            timestamp,
            total_operations: env.storage().persistent().get(&op_key).unwrap_or(0),
            total_users: env.storage().persistent().get(&usr_key).unwrap_or(0),
            total_errors: env.storage().persistent().get(&err_key).unwrap_or(0),
        }
    }

    // Get performance stats
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
mod anti_abuse {
    use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct AntiAbuseConfig {
        pub window_size: u64,     // Window size in seconds
        pub max_operations: u32,  // Max operations allowed in window
        pub cooldown_period: u64, // Minimum seconds between operations
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
    }

    pub fn get_config(env: &Env) -> AntiAbuseConfig {
        env.storage()
            .instance()
            .get(&AntiAbuseKey::Config)
            .unwrap_or(AntiAbuseConfig {
                window_size: 3600, // 1 hour default
                max_operations: 10,
                cooldown_period: 60, // 1 minute default
            })
    }

    pub fn set_config(env: &Env, config: AntiAbuseConfig) {
        env.storage().instance().set(&AntiAbuseKey::Config, &config);
    }

    pub fn is_whitelisted(env: &Env, address: Address) -> bool {
        env.storage()
            .instance()
            .has(&AntiAbuseKey::Whitelist(address))
    }

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

    pub fn check_rate_limit(env: &Env, address: Address) {
        if is_whitelisted(env, address.clone()) {
            return;
        }

        let config = get_config(env);
        let now = env.ledger().timestamp();
        let key = AntiAbuseKey::State(address.clone());

        let mut state: AddressState = env.storage().persistent().get(&key).unwrap_or(AddressState {
            last_operation_timestamp: 0,
            window_start_timestamp: now,
            operation_count: 0,
        });

        if state.last_operation_timestamp > 0
            && now < state.last_operation_timestamp.saturating_add(config.cooldown_period)
        {
            env.events().publish(
                (symbol_short!("abuse"), symbol_short!("cooldown")),
                (address, now),
            );
            panic!("Operation in cooldown period");
        }

        if now >= state.window_start_timestamp.saturating_add(config.window_size) {
            state.window_start_timestamp = now;
            state.operation_count = 1;
        } else {
            if state.operation_count >= config.max_operations {
                env.events().publish(
                    (symbol_short!("abuse"), symbol_short!("limit")),
                    (address, now),
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

// ============================================================================
// Error Definitions
// ============================================================================

/// Contract error codes for the Bounty Escrow system.
///
/// # Error Codes
/// * `AlreadyInitialized (1)` - Contract has already been initialized
/// * `NotInitialized (2)` - Contract must be initialized before use
/// * `BountyExists (3)` - Bounty ID already has funds locked
/// * `BountyNotFound (4)` - No escrow exists for this bounty ID
/// * `FundsNotLocked (5)` - Funds are not in LOCKED state
/// * `DeadlineNotPassed (6)` - Cannot refund before deadline
/// * `Unauthorized (7)` - Caller lacks required authorization
///
/// # Usage in Error Handling
/// ```rust
/// if env.storage().instance().has(&DataKey::Admin) {
///     return Err(Error::AlreadyInitialized);
/// }
/// ```
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Returned when attempting to initialize an already initialized contract
    AlreadyInitialized = 1,

    /// Returned when calling contract functions before initialization
    NotInitialized = 2,

    /// Returned when attempting to lock funds with a duplicate bounty ID
    BountyExists = 3,

    /// Returned when querying or operating on a non-existent bounty
    BountyNotFound = 4,

    /// Returned when attempting operations on non-LOCKED funds
    FundsNotLocked = 5,

    /// Returned when attempting refund before the deadline has passed
    DeadlineNotPassed = 6,

    /// Returned when caller lacks required authorization for the operation
    Unauthorized = 7,
    /// Returned when amount is invalid (zero, negative, or exceeds available)
    InvalidAmount = 8,

    /// Returned when deadline is invalid (in the past or too far in the future)
    InvalidDeadline = 9,

    /// Returned when batch size or data is invalid
    InvalidBatchSize = 10,

    /// Returned when batch input lengths don't match
    BatchSizeMismatch = 11,

    /// Returned when a duplicate bounty ID is found within a batch
    DuplicateBountyId = 12,

    /// Returned when contract has insufficient funds for the operation
    InsufficientFunds = 20,

    /// Returned when refund is attempted without admin approval
    RefundNotApproved = 13,
    /// Returned when schedule ID already exists
    ScheduleExists = 14,
    /// Returned when schedule not found
    ScheduleNotFound = 15,
    /// Returned when schedule timestamp is in the past
    InvalidScheduleTimestamp = 16,
    /// Returned when schedule amount exceeds available funds
    InsufficientScheduledAmount = 17,
    /// Returned when schedule is already released
    ScheduleAlreadyReleased = 18,
    /// Returned when schedule is not yet due for release
    ScheduleNotDue = 19,
}

// ============================================================================
// Data Structures
// ============================================================================

/// Represents the current state of escrowed funds.
///
/// # State Transitions
/// ```text
/// NONE → Locked → Released (final)
///           ↓
///        Refunded (final)
/// ```
///
/// # States
/// * `Locked` - Funds are held in escrow, awaiting release or refund
/// * `Released` - Funds have been transferred to contributor (final state)
/// * `Refunded` - Funds have been returned to depositor (final state)
///
/// # Invariants
/// - Once in Released or Refunded state, no further transitions allowed
/// - Only Locked state allows state changes
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
    PartiallyRefunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefundMode {
    Full,
    Partial,
    Custom,
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

/// Time-based release schedule for vesting funds.
///
/// # Fields
/// * `schedule_id` - Unique identifier for this schedule
/// * `amount` - Amount to release (in token's smallest denomination)
/// * `release_timestamp` - Unix timestamp when funds become available for release
/// * `recipient` - Address that will receive the funds
/// * `released` - Whether this schedule has been executed
/// * `released_at` - Timestamp when the schedule was executed (None if not released)
/// * `released_by` - Address that triggered the release (None if not released)
///
/// # Usage
/// Used to implement milestone-based payouts and scheduled distributions.
/// Multiple schedules can be created per bounty for complex vesting patterns.
///
/// # Example
/// ```rust
/// let schedule = ReleaseSchedule {
///     schedule_id: 1,
///     amount: 500_0000000, // 500 tokens
///     release_timestamp: current_time + (30 * 24 * 60 * 60), // 30 days
///     recipient: contributor_address,
///     released: false,
///     released_at: None,
///     released_by: None,
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseSchedule {
    pub schedule_id: u64,
    pub amount: i128,
    pub release_timestamp: u64,
    pub recipient: Address,
    pub released: bool,
    pub released_at: Option<u64>,
    pub released_by: Option<Address>,
}

/// History record for executed release schedules.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseHistory {
    pub schedule_id: u64,
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub released_at: u64,
    pub released_by: Address,
    pub release_type: ReleaseType,
}

/// Type of release execution.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseType {
    Automatic, // Released automatically after timestamp
    Manual,    // Released manually by authorized party
}

/// Event emitted when a release schedule is created.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleCreated {
    pub bounty_id: u64,
    pub schedule_id: u64,
    pub amount: i128,
    pub release_timestamp: u64,
    pub recipient: Address,
    pub created_by: Address,
}

/// Event emitted when a release schedule is executed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleReleased {
    pub bounty_id: u64,
    pub schedule_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub released_at: u64,
    pub released_by: Address,
    pub release_type: ReleaseType,
}

/// Complete escrow record for a bounty.
///
/// # Fields
/// * `depositor` - Address that locked the funds (receives refunds)
/// * `amount` - Token amount held in escrow (in smallest denomination)
/// * `status` - Current state of the escrow (Locked/Released/Refunded)
/// * `deadline` - Unix timestamp after which refunds are allowed
///
/// # Storage
/// Stored in persistent storage with key `DataKey::Escrow(bounty_id)`.
/// TTL is automatically extended on access.
///
/// # Example
/// ```rust
/// let escrow = Escrow {
///     depositor: depositor_address,
///     amount: 1000_0000000, // 1000 tokens
///     status: EscrowStatus::Locked,
///     deadline: current_time + 2592000, // 30 days
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub depositor: Address,
    pub amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
    pub refund_history: Vec<RefundRecord>,
    pub remaining_amount: i128,
}

/// Storage keys for contract data.
///
/// # Keys
/// * `Admin` - Stores the admin address (instance storage)
/// * `Token` - Stores the token contract address (instance storage)
/// * `Escrow(u64)` - Stores escrow data indexed by bounty_id (persistent storage)
///
/// # Storage Types
/// - **Instance Storage**: Admin and Token (never expires, tied to contract)
/// - **Persistent Storage**: Individual escrow records (extended TTL on access)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockFundsItem {
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseFundsItem {
    pub bounty_id: u64,
    pub contributor: Address,
}

// Maximum batch size to prevent gas limit issues
const MAX_BATCH_SIZE: u32 = 100;

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Escrow(u64), // bounty_id
    RefundApproval(u64), // bounty_id -> RefundApproval
    ReentrancyGuard,
    ReleaseSchedule(u64, u64), // bounty_id, schedule_id -> ReleaseSchedule
    ReleaseHistory(u64),       // bounty_id -> Vec<ReleaseHistory>
    NextScheduleId(u64),       // bounty_id -> next schedule_id
}

// ============================================================================
// Contract Implementation
// ============================================================================

#[contract]
pub struct BountyEscrowContract;

#[contractimpl]
impl BountyEscrowContract {
    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initializes the Bounty Escrow contract with admin and token addresses.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to release funds
    /// * `token` - Token contract address for escrow payments (e.g., XLM, USDC)
    ///
    /// # Returns
    /// * `Ok(())` - Contract successfully initialized
    /// * `Err(Error::AlreadyInitialized)` - Contract already initialized
    ///
    /// # State Changes
    /// - Sets Admin address in instance storage
    /// - Sets Token address in instance storage
    /// - Emits BountyEscrowInitialized event
    ///
    /// # Security Considerations
    /// - Can only be called once (prevents admin takeover)
    /// - Admin should be a secure backend service address
    /// - Token must be a valid Stellar Asset Contract
    /// - No authorization required (first-caller initialization)
    ///
    /// # Events
    /// Emits: `BountyEscrowInitialized { admin, token, timestamp }`
    ///
    /// # Example
    /// ```rust
    /// let admin = Address::from_string("GADMIN...");
    /// let usdc_token = Address::from_string("CUSDC...");
    /// escrow_client.init(&admin, &usdc_token)?;
    /// ```
    ///
    /// # Gas Cost
    /// Low - Only two storage writes
    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        let now = env.ledger().timestamp();

        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, admin.clone());

        let caller = admin.clone();

        // Prevent re-initialization
        if env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("init"), caller, false);
            return Err(Error::AlreadyInitialized);
        }

        // Store configuration
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        // Emit initialization event
        emit_bounty_initialized(
            &env,
            BountyEscrowInitialized {
                admin: admin.clone(),
                token,
                timestamp: now,
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init"), caller, true);

        // Track performance
        monitoring::emit_performance(&env, symbol_short!("init"), 0);

        Ok(())
    }

    // ========================================================================
    // Core Escrow Functions
    // ========================================================================

    /// Locks funds in escrow for a specific bounty.
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
    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        let now = env.ledger().timestamp();

        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, depositor.clone());

        let caller = depositor.clone();

        // Verify depositor authorization
        depositor.require_auth();

        // Ensure contract is initialized
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);

        if amount <= 0 {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidAmount);
        }

        if deadline <= now {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidDeadline);
        }
        if !env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::NotInitialized);
        }

        // Prevent duplicate bounty IDs
        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyExists);
        }

        // Get token contract and transfer funds
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer funds from depositor to contract
        client.transfer(&depositor, &env.current_contract_address(), &amount);

        // Create escrow record
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: Vec::new(&env),
            remaining_amount: amount,
        };

        // Store in persistent storage with extended TTL
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Emit event for off-chain indexing
        emit_funds_locked(
            &env,
            FundsLocked {
                bounty_id,
                amount,
                depositor: depositor.clone(),
                deadline,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("lock"), caller, true);

        monitoring::emit_performance(&env, symbol_short!("lock"), 0);

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
    pub fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        // Ensure contract is initialized
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);
        if !env.storage().instance().has(&DataKey::Admin) {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::NotInitialized);
        }

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();

        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, admin.clone());

        admin.require_auth();

        // Verify bounty exists
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyNotFound);
        }

        // Get and verify escrow state
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::FundsNotLocked);
        }

        // Transfer funds to contributor
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        escrow.status = EscrowStatus::Released;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Transfer funds to contributor
        client.transfer(
            &env.current_contract_address(),
            &contributor,
            &escrow.amount,
        );

        // Emit release event
        emit_funds_released(
            &env,
            FundsReleased {
                bounty_id,
                amount: escrow.amount,
                recipient: contributor.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("release"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("release"), duration);
        Ok(())
    }

    /// Approve a refund before deadline (admin only).
    /// This allows early refunds with admin approval.
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
            mode: mode.clone(),
            approved_by: admin.clone(),
            approved_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::RefundApproval(bounty_id), &approval);

        Ok(())
    }

    /// Refund funds with support for Full, Partial, and Custom refunds.
    /// - Full: refunds all remaining funds to depositor
    /// - Partial: refunds specified amount to depositor
    /// - Custom: refunds specified amount to specified recipient (requires admin approval if before deadline)
    pub fn refund(
        env: Env,
        bounty_id: u64,
        amount: Option<i128>,
        recipient: Option<Address>,
        mode: RefundMode,
    ) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        // Reentrancy guard – protect the whole refund flow including external token calls.
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("refund"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyNotFound);
        }

        // Get and verify escrow state
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        let caller = escrow.depositor.clone();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
        {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::FundsNotLocked);
        }

        // Verify deadline has passed
        let now = env.ledger().timestamp();
        let is_before_deadline = now < escrow.deadline;

        // Determine refund amount and recipient
        let refund_amount: i128;
        let refund_recipient: Address;

        match mode {
            RefundMode::Full => {
                refund_amount = escrow.remaining_amount;
                refund_recipient = escrow.depositor.clone();
                if is_before_deadline {
                    env.storage().instance().remove(&DataKey::ReentrancyGuard);
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Partial => {
                refund_amount = amount.unwrap_or(escrow.remaining_amount);
                refund_recipient = escrow.depositor.clone();
                if is_before_deadline {
                    env.storage().instance().remove(&DataKey::ReentrancyGuard);
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Custom => {
                refund_amount = match amount {
                    Some(a) => a,
                    None => {
                        env.storage().instance().remove(&DataKey::ReentrancyGuard);
                        return Err(Error::InvalidAmount);
                    }
                };
                refund_recipient = match recipient {
                    Some(r) => r,
                    None => {
                        env.storage().instance().remove(&DataKey::ReentrancyGuard);
                        return Err(Error::InvalidAmount);
                    }
                };

                // Custom refunds before deadline require admin approval
                if is_before_deadline {
                    if !env
                        .storage()
                        .persistent()
                        .has(&DataKey::RefundApproval(bounty_id))
                    {
                        env.storage().instance().remove(&DataKey::ReentrancyGuard);
                        return Err(Error::RefundNotApproved);
                    }
                    let approval: RefundApproval = env
                        .storage()
                        .persistent()
                        .get(&DataKey::RefundApproval(bounty_id))
                        .unwrap();

                    // Verify approval matches request
                    if approval.amount != refund_amount
                        || approval.recipient != refund_recipient
                        || approval.mode != mode
                    {
                        env.storage().instance().remove(&DataKey::ReentrancyGuard);
                        return Err(Error::RefundNotApproved);
                    }

                    // Clear approval after use
                    env.storage()
                        .persistent()
                        .remove(&DataKey::RefundApproval(bounty_id));
                }
            }
        }

        // Validate amount
        if refund_amount <= 0 || refund_amount > escrow.remaining_amount {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidAmount);
        }

        // Transfer funds back to depositor
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Check contract balance
        let contract_balance = client.balance(&env.current_contract_address());
        if contract_balance < refund_amount {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InsufficientFunds);
        }

        // Transfer funds
        client.transfer(
            &env.current_contract_address(),
            &refund_recipient,
            &refund_amount,
        );

        // Update escrow state
        escrow.remaining_amount -= refund_amount;

        // Add to refund history
        let refund_record = RefundRecord {
            amount: refund_amount,
            recipient: refund_recipient.clone(),
            mode: mode.clone(),
            timestamp: env.ledger().timestamp(),
        };
        escrow.refund_history.push_back(refund_record);

        // Update status
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Refunded;
        } else {
            escrow.status = EscrowStatus::PartiallyRefunded;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Emit refund event
        emit_funds_refunded(
            &env,
            FundsRefunded {
                bounty_id,
                amount: refund_amount,
                refund_to: refund_recipient,
                timestamp: env.ledger().timestamp(),
                refund_mode: mode.clone(),
                remaining_amount: escrow.remaining_amount,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("refund"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("refund"), duration);

        Ok(())
    }

    // ========================================================================
    // View Functions (Read-only)
    // ========================================================================

    /// Creates a time-based release schedule for a bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to create schedule for
    /// * `amount` - Amount to release (in token's smallest denomination)
    /// * `release_timestamp` - Unix timestamp when funds become available
    /// * `recipient` - Address that will receive the funds
    ///
    /// # Returns
    /// * `Ok(())` - Schedule successfully created
    /// * `Err(Error::NotInitialized)` - Contract not initialized
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    /// * `Err(Error::FundsNotLocked)` - Bounty not in Locked state
    /// * `Err(Error::Unauthorized)` - Caller is not admin
    /// * `Err(Error::InvalidAmount)` - Amount is invalid
    /// * `Err(Error::InvalidScheduleTimestamp)` - Timestamp is in the past
    /// * `Err(Error::InsufficientScheduledAmount)` - Amount exceeds remaining funds
    ///
    /// # State Changes
    /// - Creates ReleaseSchedule record
    /// - Updates next schedule ID
    /// - Emits ScheduleCreated event
    ///
    /// # Authorization
    /// - Only admin can call this function
    ///
    /// # Example
    /// ```rust
    /// let now = env.ledger().timestamp();
    /// let release_time = now + (30 * 24 * 60 * 60); // 30 days from now
    /// escrow_client.create_release_schedule(
    ///     &42,
    ///     &500_0000000, // 500 tokens
    ///     &release_time,
    ///     &contributor_address
    /// )?;
    /// ```
    pub fn create_release_schedule(
        env: Env,
        bounty_id: u64,
        amount: i128,
        release_timestamp: u64,
        recipient: Address,
    ) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        // Ensure contract is initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, admin.clone());

        // Verify bounty exists and is locked
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        // Validate amount
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Validate timestamp
        if release_timestamp <= env.ledger().timestamp() {
            return Err(Error::InvalidScheduleTimestamp);
        }

        // Check sufficient remaining funds
        let scheduled_total = get_total_scheduled_amount(&env, bounty_id);
        if scheduled_total + amount > escrow.remaining_amount {
            return Err(Error::InsufficientScheduledAmount);
        }

        // Get next schedule ID
        let schedule_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextScheduleId(bounty_id))
            .unwrap_or(1);

        // Check for duplicate schedule ID
        if env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
        {
            return Err(Error::ScheduleExists);
        }

        // Create release schedule
        let schedule = ReleaseSchedule {
            schedule_id,
            amount,
            release_timestamp,
            recipient: recipient.clone(),
            released: false,
            released_at: None,
            released_by: None,
        };

        // Store schedule
        env.storage()
            .persistent()
            .set(&DataKey::ReleaseSchedule(bounty_id, schedule_id), &schedule);

        // Update next schedule ID
        env.storage()
            .persistent()
            .set(&DataKey::NextScheduleId(bounty_id), &(schedule_id + 1));

        // Emit schedule created event
        env.events().publish(
            (SCHEDULE_CREATED,),
            ScheduleCreated {
                bounty_id,
                schedule_id,
                amount,
                release_timestamp,
                recipient: recipient.clone(),
                created_by: admin.clone(),
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("create_s"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("create_s"), duration);

        Ok(())
    }

    /// Automatically releases funds for schedules that are due.
    /// Can be called by anyone after the release timestamp has passed.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to check for due schedules
    /// * `schedule_id` - The specific schedule to release
    ///
    /// # Returns
    /// * `Ok(())` - Schedule successfully released
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    /// * `Err(Error::ScheduleNotFound)` - Schedule doesn't exist
    /// * `Err(Error::ScheduleAlreadyReleased)` - Schedule already released
    /// * `Err(Error::ScheduleNotDue)` - Release timestamp not yet reached
    ///
    /// # State Changes
    /// - Transfers tokens to recipient
    /// - Updates schedule status to released
    /// - Adds to release history
    /// - Updates escrow remaining amount
    /// - Emits ScheduleReleased event
    ///
    /// # Example
    /// ```rust
    /// // Anyone can call this after the timestamp
    /// escrow_client.release_schedule_automatic(&42, &1)?;
    /// ```
    pub fn release_schedule_automatic(
        env: Env,
        bounty_id: u64,
        schedule_id: u64,
    ) -> Result<(), Error> {
        let start = env.ledger().timestamp();
        let caller = env.current_contract_address();

        // Verify bounty exists
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        // Get schedule
        if !env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
        {
            return Err(Error::ScheduleNotFound);
        }

        let mut schedule: ReleaseSchedule = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
            .unwrap();

        // Check if already released
        if schedule.released {
            return Err(Error::ScheduleAlreadyReleased);
        }

        // Check if due for release
        let now = env.ledger().timestamp();
        if now < schedule.release_timestamp {
            return Err(Error::ScheduleNotDue);
        }

        // Get escrow and token client
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer funds
        client.transfer(
            &env.current_contract_address(),
            &schedule.recipient,
            &schedule.amount,
        );

        // Update schedule
        schedule.released = true;
        schedule.released_at = Some(now);
        schedule.released_by = Some(env.current_contract_address());

        // Update escrow
        escrow.remaining_amount -= schedule.amount;
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released;
        }

        // Add to release history
        let history_entry = ReleaseHistory {
            schedule_id,
            bounty_id,
            amount: schedule.amount,
            recipient: schedule.recipient.clone(),
            released_at: now,
            released_by: env.current_contract_address(),
            release_type: ReleaseType::Automatic,
        };

        let mut history: Vec<ReleaseHistory> = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(bounty_id))
            .unwrap_or(vec![&env]);
        history.push_back(history_entry);

        // Store updates
        env.storage()
            .persistent()
            .set(&DataKey::ReleaseSchedule(bounty_id, schedule_id), &schedule);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);
        env.storage()
            .persistent()
            .set(&DataKey::ReleaseHistory(bounty_id), &history);

        // Emit schedule released event
        env.events().publish(
            (SCHEDULE_RELEASED,),
            ScheduleReleased {
                bounty_id,
                schedule_id,
                amount: schedule.amount,
                recipient: schedule.recipient.clone(),
                released_at: now,
                released_by: env.current_contract_address(),
                release_type: ReleaseType::Automatic,
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("rel_auto"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("rel_auto"), duration);

        Ok(())
    }

    /// Manually releases funds for a schedule (admin only).
    /// Can be called before the release timestamp by admin.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty containing the schedule
    /// * `schedule_id` - The schedule to release
    ///
    /// # Returns
    /// * `Ok(())` - Schedule successfully released
    /// * `Err(Error::NotInitialized)` - Contract not initialized
    /// * `Err(Error::Unauthorized)` - Caller is not admin
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    /// * `Err(Error::ScheduleNotFound)` - Schedule doesn't exist
    /// * `Err(Error::ScheduleAlreadyReleased)` - Schedule already released
    ///
    /// # State Changes
    /// - Transfers tokens to recipient
    /// - Updates schedule status to released
    /// - Adds to release history
    /// - Updates escrow remaining amount
    /// - Emits ScheduleReleased event
    ///
    /// # Authorization
    /// - Only admin can call this function
    ///
    /// # Example
    /// ```rust
    /// // Admin can release early
    /// escrow_client.release_schedule_manual(&42, &1)?;
    /// ```
    pub fn release_schedule_manual(
        env: Env,
        bounty_id: u64,
        schedule_id: u64,
    ) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        // Ensure contract is initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, admin.clone());

        // Verify bounty exists
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        // Get schedule
        if !env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
        {
            return Err(Error::ScheduleNotFound);
        }

        let mut schedule: ReleaseSchedule = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
            .unwrap();

        // Check if already released
        if schedule.released {
            return Err(Error::ScheduleAlreadyReleased);
        }

        // Get escrow and token client
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer funds
        client.transfer(
            &env.current_contract_address(),
            &schedule.recipient,
            &schedule.amount,
        );

        // Update schedule
        let now = env.ledger().timestamp();
        schedule.released = true;
        schedule.released_at = Some(now);
        schedule.released_by = Some(admin.clone());

        // Update escrow
        escrow.remaining_amount -= schedule.amount;
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released;
        }

        // Add to release history
        let history_entry = ReleaseHistory {
            schedule_id,
            bounty_id,
            amount: schedule.amount,
            recipient: schedule.recipient.clone(),
            released_at: now,
            released_by: admin.clone(),
            release_type: ReleaseType::Manual,
        };

        let mut history: Vec<ReleaseHistory> = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(bounty_id))
            .unwrap_or(vec![&env]);
        history.push_back(history_entry);

        // Store updates
        env.storage()
            .persistent()
            .set(&DataKey::ReleaseSchedule(bounty_id, schedule_id), &schedule);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);
        env.storage()
            .persistent()
            .set(&DataKey::ReleaseHistory(bounty_id), &history);

        // Emit schedule released event
        env.events().publish(
            (SCHEDULE_RELEASED,),
            ScheduleReleased {
                bounty_id,
                schedule_id,
                amount: schedule.amount,
                recipient: schedule.recipient.clone(),
                released_at: now,
                released_by: admin.clone(),
                release_type: ReleaseType::Manual,
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("rel_man"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("rel_man"), duration);

        Ok(())
    }
    /// Retrieves escrow information for a specific bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok(Escrow)` - The complete escrow record
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    ///
    /// # Gas Cost
    /// Very Low - Single storage read
    ///
    /// # Example
    /// ```rust
    /// let escrow_info = escrow_client.get_escrow_info(&42)?;
    /// println!("Amount: {}", escrow_info.amount);
    /// println!("Status: {:?}", escrow_info.status);
    /// println!("Deadline: {}", escrow_info.deadline);
    /// ```
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

    /// Retrieves a specific release schedule.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty containing the schedule
    /// * `schedule_id` - The schedule ID to retrieve
    ///
    /// # Returns
    /// * `Ok(ReleaseSchedule)` - The schedule details
    /// * `Err(Error::ScheduleNotFound)` - Schedule doesn't exist
    pub fn get_release_schedule(
        env: Env,
        bounty_id: u64,
        schedule_id: u64,
    ) -> Result<ReleaseSchedule, Error> {
        if !env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
        {
            return Err(Error::ScheduleNotFound);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
            .unwrap())
    }

    /// Retrieves all release schedules for a bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Vec<ReleaseSchedule>` - All schedules for the bounty
    pub fn get_all_release_schedules(env: Env, bounty_id: u64) -> Vec<ReleaseSchedule> {
        let mut schedules = Vec::new(&env);
        let next_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextScheduleId(bounty_id))
            .unwrap_or(1);

        for schedule_id in 1..next_id {
            if env
                .storage()
                .persistent()
                .has(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
            {
                let schedule: ReleaseSchedule = env
                    .storage()
                    .persistent()
                    .get(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
                    .unwrap();
                schedules.push_back(schedule);
            }
        }

        schedules
    }

    /// Retrieves pending (unreleased) schedules for a bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Vec<ReleaseSchedule>` - All pending schedules
    pub fn get_pending_schedules(env: Env, bounty_id: u64) -> Vec<ReleaseSchedule> {
        let all_schedules = Self::get_all_release_schedules(env.clone(), bounty_id);
        let mut pending = Vec::new(&env);

        for schedule in all_schedules.iter() {
            if !schedule.released {
                pending.push_back(schedule.clone());
            }
        }

        pending
    }

    /// Retrieves due schedules (timestamp passed but not released).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Vec<ReleaseSchedule>` - All due but unreleased schedules
    pub fn get_due_schedules(env: Env, bounty_id: u64) -> Vec<ReleaseSchedule> {
        let pending = Self::get_pending_schedules(env.clone(), bounty_id);
        let mut due = Vec::new(&env);
        let now = env.ledger().timestamp();

        for schedule in pending.iter() {
            if schedule.release_timestamp <= now {
                due.push_back(schedule.clone());
            }
        }

        due
    }

    /// Retrieves release history for a bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Vec<ReleaseHistory>` - Complete release history
    pub fn get_release_history(env: Env, bounty_id: u64) -> Vec<ReleaseHistory> {
        env.storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(bounty_id))
            .unwrap_or(vec![&env])
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
    pub fn get_balance(env: Env) -> Result<i128, Error> {
        if !env.storage().instance().has(&DataKey::Token) {
            return Err(Error::NotInitialized);
        }
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        Ok(client.balance(&env.current_contract_address()))
    }

    /// Retrieves the refund history for a specific bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok(Vec<RefundRecord>)` - The refund history
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
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

    /// Gets refund eligibility information for a bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok((bool, bool, i128, Option<RefundApproval>))` - Tuple containing:
    ///   - can_refund: Whether refund is possible
    ///   - deadline_passed: Whether the deadline has passed
    ///   - remaining: Remaining amount in escrow
    ///   - approval: Optional refund approval if exists
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
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

        // can_refund is true if:
        // 1. Status is Locked or PartiallyRefunded AND
        // 2. (deadline has passed OR there's an approval)
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
        // Reentrancy guard for batch operation.
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);

        // Validate batch size
        let batch_size = items.len() as u32;
        if batch_size == 0 {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::NotInitialized);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();
        let timestamp = env.ledger().timestamp();

        // Validate all items before processing (all-or-nothing approach)
        for item in items.iter() {
            // Check if bounty already exists
            if env
                .storage()
                .persistent()
                .has(&DataKey::Escrow(item.bounty_id))
            {
                env.storage().instance().remove(&DataKey::ReentrancyGuard);
                return Err(Error::BountyExists);
            }

            // Validate amount
            if item.amount <= 0 {
                env.storage().instance().remove(&DataKey::ReentrancyGuard);
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
                env.storage().instance().remove(&DataKey::ReentrancyGuard);
                return Err(Error::DuplicateBountyId);
            }
        }

        // Collect unique depositors and require auth once for each
        // This prevents "frame is already authorized" errors when same depositor appears multiple times
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

        // Process all items (atomic - all succeed or all fail)
        let mut locked_count = 0u32;
        for item in items.iter() {
            // Transfer funds from depositor to contract
            client.transfer(&item.depositor, &contract_address, &item.amount);

            // Create escrow record
            let escrow = Escrow {
                depositor: item.depositor.clone(),
                amount: item.amount,
                status: EscrowStatus::Locked,
                deadline: item.deadline,
                refund_history: Vec::new(&env),
                remaining_amount: item.amount,
            };

            // Store escrow
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);

            // Emit individual event for each locked bounty
            emit_funds_locked(
                &env,
                FundsLocked {
                    bounty_id: item.bounty_id,
                    amount: item.amount,
                    depositor: item.depositor.clone(),
                    deadline: item.deadline,
                },
            );

            locked_count += 1;
        }

        // Emit batch event
        emit_batch_funds_locked(
            &env,
            BatchFundsLocked {
                count: locked_count,
                total_amount: items.iter().map(|i| i.amount).sum(),
                timestamp,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);
        Ok(locked_count)
    }

    /// Batch release funds to multiple contributors in a single transaction.
    /// This improves gas efficiency by reducing transaction overhead.
    ///
    /// # Arguments
    /// * `items` - Vector of ReleaseFundsItem containing bounty_id and contributor address
    ///
    /// # Returns
    /// Number of successfully released bounties
    ///
    /// # Errors
    /// * InvalidBatchSize - if batch size exceeds MAX_BATCH_SIZE or is zero
    /// * BountyNotFound - if any bounty_id doesn't exist
    /// * FundsNotLocked - if any bounty is not in Locked status
    /// * Unauthorized - if caller is not admin
    ///
    /// # Note
    /// This operation is atomic - if any item fails, the entire transaction reverts.
    pub fn batch_release_funds(env: Env, items: Vec<ReleaseFundsItem>) -> Result<u32, Error> {
        // Reentrancy guard for batch operation.
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);

        // Validate batch size
        let batch_size = items.len() as u32;
        if batch_size == 0 {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();
        let timestamp = env.ledger().timestamp();

        // Validate all items before processing (all-or-nothing approach)
        let mut total_amount: i128 = 0;
        for item in items.iter() {
            // Check if bounty exists
            if !env
                .storage()
                .persistent()
                .has(&DataKey::Escrow(item.bounty_id))
            {
                env.storage().instance().remove(&DataKey::ReentrancyGuard);
                return Err(Error::BountyNotFound);
            }

            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            // Check if funds are locked
            if escrow.status != EscrowStatus::Locked {
                env.storage().instance().remove(&DataKey::ReentrancyGuard);
                return Err(Error::FundsNotLocked);
            }

            // Check for duplicate bounty_ids in the batch
            let mut count = 0u32;
            for other_item in items.iter() {
                if other_item.bounty_id == item.bounty_id {
                    count += 1;
                }
            }
            if count > 1 {
                env.storage().instance().remove(&DataKey::ReentrancyGuard);
                return Err(Error::DuplicateBountyId);
            }

            total_amount = total_amount
                .checked_add(escrow.amount)
                .ok_or(Error::InvalidAmount)?;
        }

        // Process all items (atomic - all succeed or all fail)
        let mut released_count = 0u32;
        for item in items.iter() {
            let mut escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            // Update escrow status before external transfer (checks-effects-interactions).
            escrow.status = EscrowStatus::Released;
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);

            // Transfer funds to contributor
            client.transfer(&contract_address, &item.contributor, &escrow.amount);

            // Emit individual event for each released bounty
            emit_funds_released(
                &env,
                FundsReleased {
                    bounty_id: item.bounty_id,
                    amount: escrow.amount,
                    recipient: item.contributor.clone(),
                    timestamp,
                },
            );

            released_count += 1;
        }

        // Emit batch event
        emit_batch_funds_released(
            &env,
            BatchFundsReleased {
                count: released_count,
                total_amount,
                timestamp,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);
        Ok(released_count)
    }

    /// Updates the rate limit configuration.
    /// Only the admin can call this.
    pub fn update_rate_limit_config(
        env: Env,
        window_size: u64,
        max_operations: u32,
        cooldown_period: u64,
    ) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not set");
        admin.require_auth();
        
        anti_abuse::set_config(
            &env,
            anti_abuse::AntiAbuseConfig {
                window_size,
                max_operations,
                cooldown_period,
            },
        );
    }

    /// Adds or removes an address from the whitelist.
    /// Only the admin can call this.
    pub fn set_whitelist(env: Env, address: Address, whitelisted: bool) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not set");
        admin.require_auth();
        anti_abuse::set_whitelist(&env, address, whitelisted);
    }

    /// Checks if an address is whitelisted.
    pub fn is_whitelisted(env: Env, address: Address) -> bool {
        anti_abuse::is_whitelisted(&env, address)
    }

    /// Gets the current rate limit configuration.
    pub fn get_rate_limit_config(env: Env) -> anti_abuse::AntiAbuseConfig {
        anti_abuse::get_config(&env)
    }
}

/// Helper function to calculate total scheduled amount for a bounty.
fn get_total_scheduled_amount(env: &Env, bounty_id: u64) -> i128 {
    let next_id: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::NextScheduleId(bounty_id))
        .unwrap_or(1);

    let mut total = 0i128;
    for schedule_id in 1..next_id {
        if env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
        {
            let schedule: ReleaseSchedule = env
                .storage()
                .persistent()
                .get(&DataKey::ReleaseSchedule(bounty_id, schedule_id))
                .unwrap();
            if !schedule.released {
                total += schedule.amount;
            }
        }
    }

    total
}

#[cfg(test)]
mod test;
