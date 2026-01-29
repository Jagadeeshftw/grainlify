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
#[cfg(test)]
mod test_query;

use events::{
    emit_batch_funds_locked, emit_batch_funds_released, emit_bounty_initialized,
    emit_contract_paused, emit_contract_unpaused, emit_emergency_withdrawal, emit_funds_locked,
    emit_funds_refunded, emit_funds_released, BatchFundsLocked, BatchFundsReleased,
    BountyEscrowInitialized, ContractPaused, ContractUnpaused, EmergencyWithdrawal, FundsLocked,
    FundsRefunded, FundsReleased,
};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    Vec,
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

    // Track performance
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

    // Health check
    #[allow(dead_code)]
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
        Admin,
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

    #[allow(dead_code)]
    pub fn set_config(env: &Env, config: AntiAbuseConfig) {
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

        // 1. Cooldown check
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

        // 2. Window check
        if now
            >= state
                .window_start_timestamp
                .saturating_add(config.window_size)
        {
            // New window
            state.window_start_timestamp = now;
            state.operation_count = 1;
        } else {
            // Same window
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

        // Extend TTL for state (approx 1 day)
        env.storage().persistent().extend_ttl(&key, 17280, 17280);
    }
}
// ==================== END ANTI-ABUSE MODULE ====================

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
    InvalidFeeRate = 8,
    FeeRecipientNotSet = 9,
    InvalidBatchSize = 10,
    /// Returned when contract is paused and operation is blocked
    ContractPaused = 11,
    DuplicateBountyId = 12,
    /// Returned when amount is invalid (zero, negative, or exceeds available)
    InvalidAmount = 13,
    /// Returned when deadline is invalid (in the past or too far in the future)
    InvalidDeadline = 14,
    /// Returned when contract has insufficient funds for the operation
    InsufficientFunds = 16,
    /// Returned when refund is attempted without admin approval
    RefundNotApproved = 17,
    BatchSizeMismatch = 18,
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
    PartiallyRefunded,
    PartiallyReleased,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefundMode {
    Full,
    Partial,
    Custom,
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
    pub payout_history: Vec<PayoutRecord>,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128, // Fee rate for lock operations (basis points, e.g., 100 = 1%)
    pub release_fee_rate: i128, // Fee rate for release operations (basis points)
    pub fee_recipient: Address, // Address to receive fees
    pub fee_enabled: bool,   // Global fee enable/disable flag
}

// Fee rate is stored in basis points (1 basis point = 0.01%)
// Example: 100 basis points = 1%, 1000 basis points = 10%
const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 1_000; // Maximum 10% fee

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Escrow(u64),         // bounty_id
    FeeConfig,           // Fee configuration
    RefundApproval(u64), // bounty_id -> RefundApproval
    ReentrancyGuard,
    IsPaused,       // Contract pause state
    BountyRegistry, // Vec<u64> of all bounty IDs
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
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, admin.clone());

        let start = env.ledger().timestamp();
        let caller = admin.clone();

        // Prevent re-initialization
        if env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("init"), caller, false);
            return Err(Error::AlreadyInitialized);
        }

        // Store configuration
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        // Initialize fee config with zero fees (disabled by default)
        let fee_config = FeeConfig {
            lock_fee_rate: 0,
            release_fee_rate: 0,
            fee_recipient: admin.clone(),
            fee_enabled: false,
        };
        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &fee_config);

        // Emit initialization event
        emit_bounty_initialized(
            &env,
            BountyEscrowInitialized {
                admin: admin.clone(),
                token,
                timestamp: env.ledger().timestamp(),
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init"), duration);

        Ok(())
    }

    /// Calculate fee amount based on rate (in basis points)
    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        if fee_rate == 0 {
            return 0;
        }
        // Fee = (amount * fee_rate) / BASIS_POINTS
        // Using checked arithmetic to prevent overflow
        amount
            .checked_mul(fee_rate)
            .and_then(|x| x.checked_div(BASIS_POINTS))
            .unwrap_or(0)
    }

    /// Get fee configuration (internal helper)
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

    /// Update fee configuration (admin only)
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

    /// Get current fee configuration (view function)
    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
    }

    // ========================================================================
    // Pause and Emergency Functions
    // ========================================================================

    /// Check if contract is paused (internal helper)
    fn is_paused_internal(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get::<_, bool>(&DataKey::IsPaused)
            .unwrap_or(false)
    }

    /// Get pause status (view function)
    pub fn is_paused(env: Env) -> bool {
        Self::is_paused_internal(&env)
    }

    /// Pause the contract (admin only)
    /// Prevents new fund locks, releases, and refunds
    pub fn pause(env: Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if Self::is_paused_internal(&env) {
            return Ok(()); // Already paused, idempotent
        }

        env.storage().persistent().set(&DataKey::IsPaused, &true);

        emit_contract_paused(
            &env,
            ContractPaused {
                paused_by: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Unpause the contract (admin only)
    /// Resumes normal operations
    pub fn unpause(env: Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !Self::is_paused_internal(&env) {
            return Ok(()); // Already unpaused, idempotent
        }

        env.storage().persistent().set(&DataKey::IsPaused, &false);

        emit_contract_unpaused(
            &env,
            ContractUnpaused {
                unpaused_by: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Emergency withdrawal for all contract funds (admin only, only when paused)
    /// This function allows admins to recover all contract funds in case of critical
    /// security issues or unrecoverable bugs. It can only be called when the contract
    /// is paused to prevent misuse.
    pub fn emergency_withdraw(env: Env, recipient: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Only allow emergency withdrawal when contract is paused
        if !Self::is_paused_internal(&env) {
            return Err(Error::Unauthorized);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Get contract balance
        let balance = client.balance(&env.current_contract_address());

        if balance <= 0 {
            return Ok(()); // No funds to withdraw
        }

        // Transfer all funds to recipient
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
    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, depositor.clone());

        let start = env.ledger().timestamp();
        let caller = depositor.clone();

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            return Err(Error::ContractPaused);
        }

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

        if deadline <= env.ledger().timestamp() {
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

        // Calculate and collect fee if enabled
        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0 {
            Self::calculate_fee(amount, fee_config.lock_fee_rate)
        } else {
            0
        };
        let net_amount = amount - fee_amount;

        // Transfer net amount from depositor to contract
        client.transfer(&depositor, &env.current_contract_address(), &net_amount);

        // Transfer fee to fee recipient if applicable
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

        // Create escrow record
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: net_amount, // Store net amount (after fee)
            status: EscrowStatus::Locked,
            deadline,
            refund_history: vec![&env],
            payout_history: vec![&env],
            remaining_amount: amount,
        };

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
                amount: net_amount, // Emit net amount (after fee)
                depositor: depositor.clone(),
                deadline,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("lock"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("lock"), duration);

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
        amount: Option<i128>, // Optional partial amount
    ) -> Result<(), Error> {
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

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::ContractPaused);
        }

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

        // Allow release from Locked or PartiallyReleased states
        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyReleased
        {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::FundsNotLocked);
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

        // Transfer funds to contributor
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
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

        // Transfer net amount to contributor
        client.transfer(&env.current_contract_address(), &contributor, &net_amount);

        // Transfer fee to fee recipient if applicable
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

        // Update escrow state
        escrow.remaining_amount -= payout_amount;

        // Add to payout history
        let payout_record = PayoutRecord {
            amount: payout_amount,
            recipient: contributor.clone(),
            timestamp: env.ledger().timestamp(),
        };
        escrow.payout_history.push_back(payout_record);

        // Update status
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released; // Fully released
        } else {
            escrow.status = EscrowStatus::PartiallyReleased; // Partially released
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Emit release event
        emit_funds_released(
            &env,
            FundsReleased {
                bounty_id,
                amount: net_amount,
                recipient: contributor.clone(),
                timestamp: env.ledger().timestamp(),
                remaining_amount: escrow.remaining_amount,
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
            mode,
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

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("refund"), caller, false);
            return Err(Error::ContractPaused);
        }

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
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Partial => {
                refund_amount = amount.unwrap_or(escrow.remaining_amount);
                refund_recipient = escrow.depositor.clone();
                if is_before_deadline {
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Custom => {
                refund_amount = amount.ok_or(Error::InvalidAmount)?;
                refund_recipient = recipient.ok_or(Error::InvalidAmount)?;

                // Custom refunds before deadline require admin approval
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

                    // Verify approval matches request
                    if approval.amount != refund_amount
                        || approval.recipient != refund_recipient
                        || approval.mode != mode
                    {
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
            return Err(Error::InvalidAmount);
        }

        // Transfer funds back to depositor
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Check contract balance
        let contract_balance = client.balance(&env.current_contract_address());
        if contract_balance < refund_amount {
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
            mode,
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
                refund_mode: mode,
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
        // Validate batch size
        let batch_size = items.len();
        if batch_size == 0 {
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            return Err(Error::ContractPaused);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
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
                return Err(Error::BountyExists);
            }

            // Validate amount
            if item.amount <= 0 {
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
                refund_history: vec![&env],
                payout_history: vec![&env],
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
        // Validate batch size
        let batch_size = items.len();
        if batch_size == 0 {
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            return Err(Error::ContractPaused);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
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
                return Err(Error::BountyNotFound);
            }

            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            // Check if funds are locked
            if escrow.status != EscrowStatus::Locked {
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

            // Transfer funds to contributor
            client.transfer(&contract_address, &item.contributor, &escrow.amount);

            // Update escrow status
            escrow.status = EscrowStatus::Released;
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);

            // Emit individual event for each released bounty
            emit_funds_released(
                &env,
                FundsReleased {
                    bounty_id: item.bounty_id,
                    amount: escrow.amount,
                    recipient: item.contributor.clone(),
                    timestamp,
                    remaining_amount: escrow.remaining_amount,
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

        Ok(released_count)
    }
}

#[cfg(test)]
mod test;
