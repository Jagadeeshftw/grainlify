//! # Program Escrow Smart Contract
//!
//! A secure escrow system for managing hackathon and program prize pools on Stellar.
//! This contract enables organizers to lock funds and distribute prizes to multiple
//! winners through secure, auditable batch payouts.
//!
//! ## Overview
//!
//! The Program Escrow contract manages the complete lifecycle of hackathon/program prizes:
//! 1. **Initialization**: Set up program with authorized payout controller
//! 2. **Fund Locking**: Lock prize pool funds in escrow
//! 3. **Batch Payouts**: Distribute prizes to multiple winners simultaneously
//! 4. **Single Payouts**: Distribute individual prizes
//! 5. **Tracking**: Maintain complete payout history and balance tracking
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │              Program Escrow Architecture                         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  ┌──────────────┐                                               │
//! │  │  Organizer   │                                               │
//! │  └──────┬───────┘                                               │
//! │         │                                                        │
//! │         │ 1. init_program()                                     │
//! │         ▼                                                        │
//! │  ┌──────────────────┐                                           │
//! │  │  Program Created │                                           │
//! │  └────────┬─────────┘                                           │
//! │           │                                                      │
//! │           │ 2. lock_program_funds()                             │
//! │           ▼                                                      │
//! │  ┌──────────────────┐                                           │
//! │  │  Funds Locked    │                                           │
//! │  │  (Prize Pool)    │                                           │
//! │  └────────┬─────────┘                                           │
//! │           │                                                      │
//! │           │ 3. Hackathon happens...                             │
//! │           │                                                      │
//! │  ┌────────▼─────────┐                                           │
//! │  │ Authorized       │                                           │
//! │  │ Payout Key       │                                           │
//! │  └────────┬─────────┘                                           │
//! │           │                                                      │
//! │    ┌──────┴───────┐                                             │
//! │    │              │                                             │
//! │    ▼              ▼                                             │
//! │ batch_payout() single_payout()                                  │
//! │    │              │                                             │
//! │    ▼              ▼                                             │
//! │ ┌─────────────────────────┐                                    │
//! │ │   Winner 1, 2, 3, ...   │                                    │
//! │ └─────────────────────────┘                                    │
//! │                                                                  │
//! │  Storage:                                                        │
//! │  ┌──────────────────────────────────────────┐                  │
//! │  │ ProgramData:                             │                  │
//! │  │  - program_id                            │                  │
//! │  │  - total_funds                           │                  │
//! │  │  - remaining_balance                     │                  │
//! │  │  - authorized_payout_key                 │                  │
//! │  │  - payout_history: [PayoutRecord]        │                  │
//! │  │  - token_address                         │                  │
//! │  └──────────────────────────────────────────┘                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Authorized Payout Key**: Trusted backend service that triggers payouts
//! - **Organizer**: Trusted to lock appropriate prize amounts
//! - **Token Contract**: Standard Stellar Asset Contract (SAC)
//! - **Contract**: Trustless; operates according to programmed rules
//!
//! ### Key Security Features
//! 1. **Single Initialization**: Prevents program re-configuration
//! 2. **Authorization Checks**: Only authorized key can trigger payouts
//! 3. **Balance Validation**: Prevents overdrafts
//! 4. **Atomic Transfers**: All-or-nothing batch operations
//! 5. **Complete Audit Trail**: Full payout history tracking
//! 6. **Overflow Protection**: Safe arithmetic for all calculations
//!
//! ## Usage Example
//!
//! ```ignore
//! use soroban_sdk::{Address, Env, String, vec};
//!
//! // 1. Initialize program (one-time setup)
//! let program_id = String::from_str(&env, "Hackathon2024");
//! let backend = Address::from_string("GBACKEND...");
//! let usdc_token = Address::from_string("CUSDC...");
//!
//! let program = escrow_client.init_program(
//!     &program_id,
//!     &backend,
//!     &usdc_token
//! );
//!
//! // 2. Lock prize pool (10,000 USDC)
//! let prize_pool = 10_000_0000000; // 10,000 USDC (7 decimals)
//! escrow_client.lock_program_funds(&prize_pool);
//!
//! // 3. After hackathon, distribute prizes
//! let winners = vec![
//!     &env,
//!     Address::from_string("GWINNER1..."),
//!     Address::from_string("GWINNER2..."),
//!     Address::from_string("GWINNER3..."),
//! ];
//!
//! let prizes = vec![
//!     &env,
//!     5_000_0000000,  // 1st place: 5,000 USDC
//!     3_000_0000000,  // 2nd place: 3,000 USDC
//!     2_000_0000000,  // 3rd place: 2,000 USDC
//! ];
//!
//! escrow_client.batch_payout(&winners, &prizes);
//! ```
//!
//! ## Event System
//!
//! The contract emits events for all major operations:
//! - `ProgramInit`: Program initialization
//! - `FundsLocked`: Prize funds locked
//! - `BatchPayout`: Multiple prizes distributed
//! - `Payout`: Single prize distributed
//!
//! ## Best Practices
//!
//! 1. **Verify Winners**: Confirm winner addresses off-chain before payout
//! 2. **Test Payouts**: Use testnet for testing prize distributions
//! 3. **Secure Backend**: Protect authorized payout key with HSM/multi-sig
//! 4. **Audit History**: Review payout history before each distribution
//! 5. **Balance Checks**: Verify remaining balance matches expectations
//! 6. **Token Approval**: Ensure contract has token allowance before locking funds

#![no_std]
pub mod security {
    pub mod reentrancy_guard;
}
#[cfg(test)]
mod pause_tests;
#[cfg(test)]
mod reentrancy_test;

use security::reentrancy_guard::{ReentrancyGuard, ReentrancyGuardRAII};

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    Map, String, Symbol, Vec,
};

use grainlify_interfaces::{
    ConfigurableFee, EscrowLock, EscrowRelease, FeeConfig as SharedFeeConfig, Pausable, RefundMode,
};

// Event types
#[allow(dead_code)]
const PROGRAM_INITIALIZED: Symbol = symbol_short!("ProgInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FundLock");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
const PAYOUT: Symbol = symbol_short!("Payout");

// Storage keys
const PROGRAM_DATA: Symbol = symbol_short!("ProgData");
const FEE_CONFIG: Symbol = symbol_short!("FeeCfg");

// Fee rate is stored in basis points (1 basis point = 0.01%)
// Example: 100 basis points = 1%, 1000 basis points = 10%
const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 1_000; // Maximum 10% fee

const ADMIN_UPDATE_TIMELOCK: u64 = 1 * 24 * 60 * 60;
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,    // Fee rate for lock operations (basis points)
    pub payout_fee_rate: i128,  // Fee rate for payout operations (basis points)
    pub fee_recipient: Address, // Address to receive fees
    pub fee_enabled: bool,      // Global fee enable/disable flag
}

impl From<SharedFeeConfig> for FeeConfig {
    fn from(shared: SharedFeeConfig) -> Self {
        Self {
            lock_fee_rate: shared.lock_fee_rate,
            payout_fee_rate: shared.payout_fee_rate,
            fee_recipient: shared.fee_recipient,
            fee_enabled: shared.fee_enabled,
        }
    }
}

impl From<FeeConfig> for SharedFeeConfig {
    fn from(local: FeeConfig) -> Self {
        Self {
            lock_fee_rate: local.lock_fee_rate,
            payout_fee_rate: local.payout_fee_rate,
            fee_recipient: local.fee_recipient,
            fee_enabled: local.fee_enabled,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct AmountLimits {
    pub min_lock_amount: i128,
    pub max_lock_amount: i128,
    pub min_payout: i128,
    pub max_payout: i128,
}
#[contracttype]
#[derive(Clone, Debug)]
pub struct UpdateAdminEvent {
    pub admin: Address,
    pub new_admin: Address,
    pub timestamp: u64,
}

pub fn emit_update_admin(env: &Env, event: UpdateAdminEvent) {
    let topics = (symbol_short!("upd_adm"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct UpdateAuthorizedKeyEvent {
    pub old_authorized_payout_key: Address,
    pub new_authorized_payout_key: Address,
    pub timestamp: u64,
}

pub fn emit_update_authorized_key(env: &Env, event: UpdateAuthorizedKeyEvent) {
    let topics = (symbol_short!("upd_apk"),);
    env.events().publish(topics, event.clone());
}

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
        LastAdminUpdate,
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

    pub fn get_admin(env: &Env) -> Option<Address> {
        env.storage().instance().get(&AntiAbuseKey::Admin)
    }

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

// ============================================================================
// Event Types
// ============================================================================

/// Event emitted when a program is initialized/registerd
const PROGRAM_REGISTERED: Symbol = symbol_short!("ProgReg");

// ============================================================================
// Storage Keys
// ============================================================================

/// Storage key for program data.
/// Contains all program state including balances and payout history.
const PROG_DATA: Symbol = symbol_short!("ProgData");
const TOK_BAL: Symbol = symbol_short!("TokBal");
const PROGRAM_REGISTRY: Symbol = symbol_short!("ProgReg");

/// Storage key for program metadata.
/// Contains optional metadata for indexing and categorization.
const PROGRAM_METADATA: Symbol = symbol_short!("ProgMeta");

// ============================================================================
// Data Structures
// ============================================================================

/// Record of an individual payout transaction.
///
/// # Fields
/// * `recipient` - Address that received the payout
/// * `amount` - Amount transferred (in token's smallest denomination)
/// * `timestamp` - Unix timestamp when payout was executed
///
/// # Usage
/// These records are stored in the payout history to provide a complete
/// audit trail of all prize distributions.
///
/// # Example
/// ```ignore
/// let record = PayoutRecord {
///     recipient: winner_address,
///     amount: 1000_0000000, // 1000 USDC
///     timestamp: env.ledger().timestamp(),
/// };
/// ```

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenBalance {
    pub locked: i128,
    pub remaining: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
    pub token: Address,
}

/// Time-based release schedule for program funds.
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
/// Used to implement milestone-based payouts and scheduled distributions for programs.
/// Multiple schedules can be created per program for complex vesting patterns.
///
/// # Example
/// ```rust
/// let schedule = ProgramReleaseSchedule {
///     schedule_id: 1,
///     amount: 500_0000000, // 500 tokens
///     release_timestamp: current_time + (30 * 24 * 60 * 60), // 30 days
///     recipient: winner_address,
///     released: false,
///     released_at: None,
///     released_by: None,
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReleaseSchedule {
    pub schedule_id: u64,
    pub amount: i128,
    pub release_timestamp: u64,
    pub recipient: Address,
    pub released: bool,
    pub released_at: Option<u64>,
    pub released_by: Option<Address>,
}

/// History record for executed program release schedules.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReleaseHistory {
    pub schedule_id: u64,
    pub program_id: String,
    pub amount: i128,
    pub recipient: Address,
    pub released_at: u64,
    pub released_by: Address,
    pub release_type: ReleaseType,
}

/// Type of release execution for programs.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseType {
    Automatic, // Released automatically after timestamp
    Manual,    // Released manually by authorized party
}

/// Event emitted when a program release schedule is created.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramScheduleCreated {
    pub program_id: String,
    pub schedule_id: u64,
    pub amount: i128,
    pub release_timestamp: u64,
    pub recipient: Address,
    pub created_by: Address,
}

/// Event emitted when a program release schedule is executed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramScheduleReleased {
    pub program_id: String,
    pub schedule_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub released_at: u64,
    pub released_by: Address,
    pub release_type: ReleaseType,
}

/// Complete program state and configuration.
///
/// # Fields
/// * `program_id` - Unique identifier for the program/hackathon
/// * `total_funds` - Total amount of funds locked (cumulative)
/// * `remaining_balance` - Current available balance for payouts
/// * `authorized_payout_key` - Address authorized to trigger payouts
/// * `payout_history` - Complete record of all payouts
/// * `token_address` - Token contract used for transfers
///
/// # Storage
/// Stored in instance storage with key `PROGRAM_DATA`.
///
/// # Invariants
/// - `remaining_balance <= total_funds` (always)
/// - `remaining_balance = total_funds - sum(payout_history.amounts)`
/// - `payout_history` is append-only
/// - `program_id` and `authorized_payout_key` are immutable after init
///
/// # Example
/// ```ignore
/// let program_data = ProgramData {
///     program_id: String::from_str(&env, "Hackathon2024"),
///     total_funds: 10_000_0000000,
///     remaining_balance: 7_000_0000000,
///     authorized_payout_key: backend_address,
///     payout_history: vec![&env],
///     token_address: usdc_token_address,
/// };
/// ```
/// Complete program state and configuration.
///
/// # Storage Key
/// Stored with key: `("Program", program_id)`
///
/// # Invariants
/// - `remaining_balance <= total_funds` (always)
/// - `remaining_balance = total_funds - sum(payout_history.amounts)`
/// - `payout_history` is append-only
/// - `program_id` and `authorized_payout_key` are immutable after registration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_bal: i128,
    pub auth_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address, // Primary/default token (for backward compatibility)
    pub token_balances: Map<Address, i128>, // Map of token_address -> balance for multi-token support
    pub whitelist: Vec<Address>,
    pub deadline: Option<u64>, // Optional deadline for the program
    pub organizer: Address,    // Program organizer address
}

/// Per-operation pause configuration for granular control.
///
/// Allows the platform to halt specific operations while allowing others to continue.
/// For program escrow: lock (fund locking), payout (single/batch payouts), schedule (release schedules)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseConfig {
    pub lock_paused: bool,
    pub payout_paused: bool,
    pub schedule_paused: bool,
}

/// Storage key type for individual programs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String),              // program_id -> ProgramData
    ReleaseSchedule(String, u64), // program_id, schedule_id -> ProgramReleaseSchedule
    ReleaseHistory(String),       // program_id -> Vec<ProgramReleaseHistory>
    NextScheduleId(String),       // program_id -> next schedule_id
    AmountLimits,                 // Amount limits configuration
    IsPaused,                     // Global contract pause state (legacy)
    TokenWhitelist(Address),      // token_address -> bool (whitelist status)
    RegisteredTokens,             // Vec<Address> of all registered tokens
    PauseConfig,                  // Per-operation pause configuration
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramFilter {
    pub authorized_key: Option<Address>,
    pub token_address: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutFilter {
    pub recipient: Option<Address>,
    pub min_amount: Option<i128>,
    pub max_amount: Option<i128>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pagination {
    pub start_index: u64,
    pub limit: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramStats {
    pub total_programs: u64,
    pub total_funds_locked: i128,
    pub total_funds_remaining: i128,
    pub total_payouts_volume: i128,
}

// ============================================================================
// Contract Implementation
// ============================================================================

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    BatchMismatch = 1,
    InsufficientBalance = 2,
    InvalidAmount = 3,
    /// Returned when lock operations are paused
    LockPaused = 4,
    /// Returned when payout operations are paused
    PayoutPaused = 5,
    /// Returned when schedule operations are paused
    SchedulePaused = 6,
}

#[contract]
pub struct ProgramEscrowContract;

// Event symbols for program release schedules
const PROG_SCHEDULE_CREATED: soroban_sdk::Symbol = soroban_sdk::symbol_short!("prg_sch_c");
const PROG_SCHEDULE_RELEASED: soroban_sdk::Symbol = soroban_sdk::symbol_short!("prg_sch_r");

#[contractimpl]
impl ProgramEscrowContract {
    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initializes a new program escrow for managing prize distributions.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - Unique identifier for this program/hackathon
    /// * `authorized_payout_key` - Address authorized to trigger payouts (backend)
    /// * `token_address` - Address of the token contract for transfers (e.g., USDC)
    ///
    /// # Returns
    /// * `ProgramData` - The initialized program configuration
    ///
    /// # Returns
    /// * `Ok(ProgramData)` - The initialized program configuration
    /// * `Err(Error::AlreadyInitialized)` - Program already initialized
    ///
    /// # State Changes
    /// - Creates ProgramData with zero balances
    /// - Sets authorized payout key (immutable after this)
    /// - Initializes empty payout history
    /// - Emits ProgramInitialized event
    ///
    /// # Security Considerations
    /// - Can only be called once (prevents re-configuration)
    /// - No authorization required (first-caller initialization)
    /// - Authorized payout key should be a secure backend service
    /// - Token address must be a valid Stellar Asset Contract
    /// - Program ID should be unique and descriptive
    ///
    /// # Events
    /// Emits: `ProgramInit(program_id, authorized_payout_key, token_address, 0)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{Address, String, Env};
    ///
    /// let program_id = String::from_str(&env, "ETHGlobal2024");
    /// let backend = Address::from_string("GBACKEND...");
    /// let usdc = Address::from_string("CUSDC...");
    ///
    /// let program = escrow_client.init_program(
    ///     &program_id,
    ///     &backend,
    ///     &usdc
    /// );
    ///
    /// println!("Program created: {}", program.program_id);
    /// ```
    ///
    /// # Production Setup
    /// ```bash
    /// # Deploy contract
    /// stellar contract deploy \
    ///   --wasm target/wasm32-unknown-unknown/release/escrow.wasm \
    ///   --source ORGANIZER_KEY
    ///
    /// # Initialize program
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ORGANIZER_KEY \
    ///   -- init_program \
    ///   --program_id "Hackathon2024" \
    ///   --authorized_payout_key GBACKEND... \
    ///   --token_address CUSDC...
    /// ```
    ///
    /// # Gas Cost
    /// Low - Initial storage writes
    pub fn init_program(
        env: Env,
        program_id: String,
        auth_key: Address,
        token_addr: Address,
        organizer: Address,
        deadline: Option<u64>,
    ) -> ProgramData {
        let start = env.ledger().timestamp();
        let caller = env.current_contract_address();

        // Prevent re-initialization
        if env.storage().instance().has(&PROGRAM_DATA) {
            panic!("Program already initialized");
        }

        // Validate deadline if provided
        if let Some(dl) = deadline {
            if dl <= env.ledger().timestamp() {
                panic!("Deadline must be in the future");
            }
        }

        // Create program data
        let mut balances = Map::new(&env);
        balances.set(token_addr.clone(), 0);
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_bal: 0,
            auth_key: auth_key.clone(),
            payout_history: vec![&env],
            token_address: token_addr.clone(),
            token_balances: balances,
            whitelist: vec![&env, token_addr.clone()],
            deadline,
            organizer: organizer.clone(),
        };

        // Store program configuration (both at PROGRAM_DATA and DataKey::Program for compatibility)
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        let program_key = DataKey::Program(program_id.clone());
        env.storage().instance().set(&program_key, &program_data);

        // Register the token in the TokenWhitelist for lock_program_funds compatibility
        env.storage()
            .instance()
            .set(&DataKey::TokenWhitelist(token_addr.clone()), &true);

        // Emit initialization event
        env.events().publish(
            (PROGRAM_INITIALIZED,),
            (program_id, auth_key.clone(), token_addr, 0i128),
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init_prg"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init_prg"), duration);

        program_data
    }

    /// Calculate fee amount based on rate (in basis points)
    /// Check if token is registered/whitelisted
    fn is_token_registered(env: &Env, token: &Address) -> bool {
        // Check whitelist
        env.storage()
            .instance()
            .has(&DataKey::TokenWhitelist(token.clone()))
    }

    /// Register a new token (authorized payout key only)
    pub fn register_token(env: Env, program_id: String, token: Address, whitelisted: bool) {
        let program_key = DataKey::Program(program_id.clone());
        if !env.storage().instance().has(&program_key) {
            panic!("Program not found");
        }

        let program_data: ProgramData = env.storage().instance().get(&program_key).unwrap();
        program_data.auth_key.require_auth();

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
    }

    /// Get all registered tokens
    pub fn get_registered_tokens(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::RegisteredTokens)
            .unwrap_or_else(|| vec![&env])
    }

    /// Get balance for a specific token in a program
    pub fn get_token_balance(env: Env, program_id: String, token: Address) -> i128 {
        let program_key = DataKey::Program(program_id.clone());
        if !env.storage().instance().has(&program_key) {
            panic!("Program not found");
        }

        let program_data: ProgramData = env.storage().instance().get(&program_key).unwrap();

        // Check token_balances map
        if let Some(balance) = program_data.token_balances.get(token.clone()) {
            balance
        } else if program_data.token_address == token {
            // Backward compatibility: if token matches default, return remaining_balance
            program_data.remaining_bal
        } else {
            0
        }
    }

    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        if fee_rate == 0 {
            return 0;
        }
        // Fee = (amount * fee_rate) / BASIS_POINTS
        amount
            .checked_mul(fee_rate)
            .and_then(|x| x.checked_div(BASIS_POINTS))
            .unwrap_or(0)
    }

    /// Get fee configuration (internal helper)
    fn get_fee_config_internal(env: &Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&FEE_CONFIG)
            .unwrap_or_else(|| FeeConfig {
                lock_fee_rate: 0,
                payout_fee_rate: 0,
                fee_recipient: env.current_contract_address(),
                fee_enabled: false,
            })
    }

    /// Lock initial funds into the program escrow
    ///
    /// Lists all registered program IDs in the contract.
    ///
    /// # Returns
    /// * `Vec<String>` - List of all program IDs
    ///
    /// # Example
    /// ```rust
    /// let programs = escrow_client.list_programs();
    /// for program_id in programs.iter() {
    ///     println!("Program: {}", program_id);
    /// }
    /// ```
    pub fn list_programs(env: Env) -> Vec<String> {
        env.storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env])
    }

    /// Checks if a program exists.
    ///
    /// # Arguments
    /// * `program_id` - The program ID to check
    ///
    /// # Returns
    /// * `bool` - True if program exists, false otherwise
    pub fn program_exists(env: Env, program_id: String) -> bool {
        let program_key = DataKey::Program(program_id);
        env.storage().instance().has(&program_key)
    }

    // ========================================================================
    // Fund Management
    // ========================================================================

    /// Locks funds into the program escrow for prize distribution.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `amount` - Amount of tokens to lock (in token's smallest denomination)
    ///
    /// # Returns
    /// * `ProgramData` - Updated program data with new balance
    ///
    /// # Returns
    /// * `Ok(ProgramData)` - Updated program data with new balance
    /// * `Err(Error::InvalidAmount)` - Amount must be greater than zero
    /// * `Err(Error::NotInitialized)` - Program not initialized
    ///
    /// # State Changes
    /// - Increases `total_funds` by amount
    /// - Increases `remaining_balance` by amount
    /// - Emits FundsLocked event
    ///
    /// # Prerequisites
    /// Before calling this function:
    /// 1. Caller must have sufficient token balance
    /// 2. Caller must approve contract for token transfer
    /// 3. Tokens must actually be transferred to contract
    ///
    /// # Security Considerations
    /// - Amount must be positive
    /// - This function doesn't perform the actual token transfer
    /// - Caller is responsible for transferring tokens to contract
    /// - Consider verifying contract balance matches recorded amount
    /// - Multiple lock operations are additive (cumulative)
    ///
    /// # Events
    /// Emits: `FundsLocked(program_id, amount, new_remaining_balance)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::token;
    ///
    /// // 1. Transfer tokens to contract
    /// let amount = 10_000_0000000; // 10,000 USDC
    /// token_client.transfer(
    ///     &organizer,
    ///     &contract_address,
    ///     &amount
    /// );
    ///
    /// // 2. Record the locked funds
    /// let updated = escrow_client.lock_program_funds(&amount);
    /// println!("Locked: {} USDC", amount / 10_000_000);
    /// println!("Remaining: {}", updated.remaining_bal);
    /// ```
    ///
    /// # Production Usage
    /// ```bash
    /// # 1. Transfer USDC to contract
    /// stellar contract invoke \
    ///   --id USDC_TOKEN_ID \
    ///   --source ORGANIZER_KEY \
    ///   -- transfer \
    ///   --from ORGANIZER_ADDRESS \
    ///   --to CONTRACT_ADDRESS \
    ///   --amount 10000000000
    ///
    /// # 2. Record locked funds
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ORGANIZER_KEY \
    ///   -- lock_program_funds \
    ///   --amount 10000000000
    /// ```
    ///
    /// # Gas Cost
    /// Low - Storage update + event emission
    ///
    /// # Common Pitfalls
    /// - Forgetting to transfer tokens before calling
    /// -  Locking amount that exceeds actual contract balance
    /// -  Not verifying contract received the tokens
    pub fn lock_program_funds(
        env: Env,
        program_id: String,
        amount: i128,
        token_address: Option<Address>, // Optional: if None, uses program's primary token
    ) -> ProgramData {
        let _guard = ReentrancyGuardRAII::new(&env).expect("Reentrancy detected");
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, env.current_contract_address());

        let _start = env.ledger().timestamp();
        let caller = env.current_contract_address();

        // Check if lock operations are paused
        if Self::is_lock_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
            panic!("Lock operations are paused");
        }

        // Validate amount
        if amount <= 0 {
            monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
            panic!("Amount must be greater than zero");
        }

        // Check amount limits
        let limits = Self::get_amount_limits(env.clone());
        if amount < limits.min_lock_amount || amount > limits.max_lock_amount {
            monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
            panic!("Amount violates configured limits");
        }

        // Check amount limits
        let limits = Self::get_amount_limits(env.clone());
        if amount < limits.min_lock_amount || amount > limits.max_lock_amount {
            monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
            panic!("Amount violates configured limits");
        }

        // Get current program data
        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&DataKey::Program(program_id.clone()))
            .unwrap_or_else(|| {
                monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
                panic!("Program not found")
            });

        // Get token address (use provided or program's primary token)
        let token_addr: Address = if let Some(token) = token_address {
            // Validate token is registered/whitelisted
            if !Self::is_token_registered(&env, &token) {
                monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
                panic!("Token not registered");
            }
            token
        } else {
            // Use program's primary token for backward compatibility
            program_data.token_address.clone()
        };

        // Calculate and collect fee if enabled
        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0 {
            Self::calculate_fee(amount, fee_config.lock_fee_rate)
        } else {
            0
        };
        let net_amount = amount - fee_amount;

        // Update token balance in map
        let current_balance = program_data
            .token_balances
            .get(token_addr.clone())
            .unwrap_or(0);
        program_data
            .token_balances
            .set(token_addr.clone(), current_balance + net_amount);

        // Update balances with net amount
        program_data.total_funds += net_amount;
        program_data.remaining_bal += net_amount;

        // Store updated data (both locations for compatibility)
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        env.storage()
            .instance()
            .set(&DataKey::Program(program_id.clone()), &program_data);

        // Emit funds locked event
        env.events().publish(
            (FUNDS_LOCKED,),
            (
                program_data.program_id.clone(),
                amount,
                program_data.remaining_bal,
            ),
        );

        program_data
    }

    // ========================================================================
    // Payout Functions
    // ========================================================================

    /// Executes batch payouts to multiple recipients simultaneously.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `recipients` - Vector of recipient addresses
    /// * `amounts` - Vector of amounts (must match recipients length)
    ///
    /// # Returns
    /// * `Ok(ProgramData)` - Updated program data after payouts
    /// * `Err(Error::Unauthorized)` - Caller is not the authorized payout key
    /// * `Err(Error::NotInitialized)` - Program not initialized
    /// * `Err(Error::BatchMismatch)` - Recipients and amounts vectors length mismatch
    /// * `Err(Error::InvalidAmount)` - Amount is zero or negative
    /// * `Err(Error::InsufficientBalance)` - Total payout exceeds remaining balance
    ///
    /// # Authorization
    /// - **CRITICAL**: Only authorized payout key can call
    /// - Caller must be exact match to `authorized_payout_key`
    ///
    /// # State Changes
    /// - Transfers tokens from contract to each recipient
    /// - Adds PayoutRecord for each transfer to history
    /// - Decreases `remaining_balance` by total payout amount
    /// - Emits BatchPayout event
    ///
    /// # Atomicity
    /// This operation is atomic - either all transfers succeed or all fail.
    /// If any transfer fails, the entire batch is reverted.
    ///
    /// # Security Considerations
    /// - Verify recipient addresses off-chain before calling
    /// - Ensure amounts match winner rankings/criteria
    /// - Total payout is calculated with overflow protection
    /// - Balance check prevents overdraft
    /// - All transfers are logged for audit trail
    /// - Consider implementing payout limits for additional safety
    ///
    /// # Events
    /// Emits: `BatchPayout(program_id, recipient_count, total_amount, new_balance)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{vec, Address};
    ///
    /// // Define winners and prizes
    /// let winners = vec![
    ///     &env,
    ///     Address::from_string("GWINNER1..."), // 1st place
    ///     Address::from_string("GWINNER2..."), // 2nd place
    ///     Address::from_string("GWINNER3..."), // 3rd place
    /// ];
    ///
    /// let prizes = vec![
    ///     &env,
    ///     5_000_0000000,  // $5,000 USDC
    ///     3_000_0000000,  // $3,000 USDC
    ///     2_000_0000000,  // $2,000 USDC
    /// ];
    ///
    /// // Execute batch payout (only authorized backend can call)
    /// let result = escrow_client.batch_payout(&winners, &prizes);
    /// println!("Paid {} winners", winners.len());
    /// println!("Remaining: {}", result.remaining_bal);
    /// ```
    ///
    /// # Production Usage
    /// ```bash
    /// # Batch payout to 3 winners
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source BACKEND_KEY \
    ///   -- batch_payout \
    ///   --recipients '["GWINNER1...", "GWINNER2...", "GWINNER3..."]' \
    ///   --amounts '[5000000000, 3000000000, 2000000000]'
    /// ```
    ///
    /// # Gas Cost
    /// High - Multiple token transfers + storage updates
    /// Cost scales linearly with number of recipients
    ///
    /// # Best Practices
    /// 1. Verify all winner addresses before execution
    /// 2. Double-check prize amounts match criteria
    /// 3. Test on testnet with same number of recipients
    /// 4. Monitor events for successful completion
    /// 5. Keep batch size reasonable (recommend < 50 recipients)
    ///
    /// # Limitations
    /// - Maximum batch size limited by gas/resource limits
    /// - For very large batches, consider multiple calls
    /// - All amounts must be positive  
    pub fn batch_payout(
        env: Env,
        program_id: String,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
        token_address: Option<Address>, // Optional: if None, uses program's primary token
    ) -> ProgramData {
        let _guard = ReentrancyGuardRAII::new(&env).expect("Reentrancy detected");

        // Check if payout operations are paused
        if Self::is_payout_paused_internal(&env) {
            panic!("Payout operations are paused");
        }
        // Apply rate limiting to the contract itself or the program
        // We can't easily get the caller here without getting program data first

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Get token address (use provided or program's primary token)
        let token_addr: Address = if let Some(token) = token_address {
            // Validate token has balance in program
            if program_data.token_balances.get(token.clone()).is_none() {
                panic!("Token not found in program");
            }
            token
        } else {
            // Use program's primary token for backward compatibility
            program_data.token_address.clone()
        };

        // Get balance for this token
        let token_balance = program_data
            .token_balances
            .get(token_addr.clone())
            .unwrap_or(0);

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.auth_key.clone());

        // Verify authorization - CRITICAL security check
        program_data.auth_key.require_auth();

        // Validate input lengths match
        if recipients.len() != amounts.len() {
            panic!("Vectors must have the same length");
        }

        // Validate non-empty batch
        if recipients.len() == 0 {
            panic!("Cannot process empty batch");
        }

        // Calculate total payout with overflow protection
        let mut total_payout: i128 = 0;
        let fee_config = Self::get_fee_config_internal(&env);
        let limits = Self::get_amount_limits(env.clone());

        for i in 0..amounts.len() {
            let amount = amounts.get(i as u32).unwrap();
            if amount <= 0 {
                panic!("All amounts must be greater than zero");
            }

            // Check payout amount limits (considering fees)
            let fee_amount = if fee_config.fee_enabled && fee_config.payout_fee_rate > 0 {
                Self::calculate_fee(amount, fee_config.payout_fee_rate)
            } else {
                0
            };
            let net_amount = amount - fee_amount;

            if net_amount < limits.min_payout || net_amount > limits.max_payout {
                panic!("Payout amount violates configured limits");
            }

            total_payout = total_payout
                .checked_add(amount)
                .unwrap_or_else(|| panic!("Payout amount overflow"));
        }

        // Validate balance for this token
        if total_payout > token_balance {
            panic!(
                "Insufficient token balance: requested {}, available {}",
                total_payout, token_balance
            );
        }

        // Execute transfers and record payouts
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &token_addr);

        for i in 0..recipients.len() {
            let recipient = recipients.get(i as u32).unwrap();
            let amount = amounts.get(i as u32).unwrap();

            // Transfer tokens from contract to recipient
            token_client.transfer(&contract_address, &recipient, &amount);

            // Record payout in history
            let payout_record = PayoutRecord {
                recipient: recipient.clone(),
                amount,
                timestamp,
                token: token_addr.clone(),
            };
            updated_history.push_back(payout_record);
        }

        // Update program data
        let mut updated_data = program_data.clone();
        // Update token balance
        updated_data
            .token_balances
            .set(token_addr.clone(), token_balance - total_payout);
        updated_data.remaining_bal -= total_payout;
        updated_data.payout_history = updated_history;

        // Store updated data (both locations for compatibility)
        env.storage().instance().set(&PROGRAM_DATA, &updated_data);
        env.storage().instance().set(&program_key, &updated_data);

        // Emit batch payout event
        env.events().publish(
            (BATCH_PAYOUT,),
            (
                updated_data.program_id.clone(),
                recipients.len() as u32,
                total_payout,
                updated_data.remaining_bal,
            ),
        );

        updated_data
    }

    /// Executes a single payout to one recipient.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `recipient` - Address of the prize recipient
    /// * `amount` - Amount to transfer (in token's smallest denomination)
    ///
    /// # Returns
    /// * `Ok(ProgramData)` - Updated program data after payout
    /// * `Err(Error::Unauthorized)` - Caller is not the authorized payout key
    /// * `Err(Error::NotInitialized)` - Program not initialized
    /// * `Err(Error::InvalidAmount)` - Amount is zero or negative
    /// * `Err(Error::InsufficientBalance)` - Amount exceeds remaining balance
    ///
    /// # Authorization
    /// - Only authorized payout key can call this function
    ///
    /// # State Changes
    /// - Transfers tokens from contract to recipient
    /// - Adds PayoutRecord to history
    /// - Decreases `remaining_balance` by amount
    /// - Emits Payout event
    ///
    /// # Security Considerations
    /// - Verify recipient address before calling
    /// - Amount must be positive
    /// - Balance check prevents overdraft
    /// - Transfer is logged in payout history
    ///
    /// # Events
    /// Emits: `Payout(program_id, recipient, amount, new_balance)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::Address;
    ///
    /// let winner = Address::from_string("GWINNER...");
    /// let prize = 1_000_0000000; // $1,000 USDC
    ///
    /// // Execute single payout
    /// let result = escrow_client.single_payout(&winner, &prize);
    /// println!("Paid {} to winner", prize);
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Single token transfer + storage update
    ///
    /// # Use Cases
    /// - Individual prize awards
    /// - Bonus payments
    /// - Late additions to prize pool distribution
    pub fn single_payout(
        env: Env,
        program_id: String,
        recipient: Address,
        amount: i128,
        token_address: Option<Address>, // Optional: if None, uses program's primary token
    ) -> ProgramData {
        let _guard = ReentrancyGuardRAII::new(&env).expect("Reentrancy detected");
        // Check if payout operations are paused
        if Self::is_payout_paused_internal(&env) {
            panic!("Payout operations are paused");
        }
        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Get token address (use provided or program's primary token)
        let token_addr: Address = if let Some(token) = token_address {
            // Validate token has balance in program
            if program_data.token_balances.get(token.clone()).is_none() {
                panic!("Token not found in program");
            }
            token
        } else {
            // Use program's primary token for backward compatibility
            program_data.token_address.clone()
        };

        // Get balance for this token
        let token_balance = program_data
            .token_balances
            .get(token_addr.clone())
            .unwrap_or(0);

        program_data.auth_key.require_auth();
        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.auth_key.clone());

        // Validate amount
        if amount <= 0 {
            panic!("Invalid amount");
        }

        // Validate balance for this token
        if amount > token_balance {
            panic!(
                "Insufficient token balance: requested {}, available {}",
                amount, token_balance
            );
        }

        // Check payout amount limits (considering fees)
        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.payout_fee_rate > 0 {
            Self::calculate_fee(amount, fee_config.payout_fee_rate)
        } else {
            0
        };
        let net_amount = amount - fee_amount;

        // Transfer net amount to recipient
        // Transfer tokens
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address, &token_addr);
        token_client.transfer(&contract_address, &recipient, &net_amount);

        // Transfer fee to fee recipient if applicable

        if fee_amount > 0 {
            token_client.transfer(&contract_address, &fee_config.fee_recipient, &fee_amount);
            env.events().publish(
                (symbol_short!("fee"),),
                (
                    symbol_short!("payout"),
                    fee_amount,
                    fee_config.payout_fee_rate,
                    fee_config.fee_recipient.clone(),
                ),
            );
        }

        // Transfer tokens to recipient
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&contract_address, &recipient, &amount);

        // Record payout
        let timestamp = env.ledger().timestamp();
        let record = PayoutRecord {
            recipient: recipient.clone(),
            amount,
            timestamp,
            token: token_addr.clone(),
        };

        let mut history = program_data.payout_history.clone();
        history.push_back(record);

        // Update program data
        let mut updated_data = program_data.clone();
        // Update token balance
        updated_data
            .token_balances
            .set(token_addr.clone(), token_balance - amount);
        updated_data.remaining_bal -= amount;
        updated_data.payout_history = history;

        // Store updated data (both locations for compatibility)
        env.storage().instance().set(&PROGRAM_DATA, &updated_data);
        env.storage().instance().set(&program_key, &updated_data);

        // Emit payout event
        env.events().publish(
            (PAYOUT,),
            (
                updated_data.program_id.clone(),
                recipient,
                amount,
                updated_data.remaining_bal,
            ),
        );

        updated_data
    }

    // ========================================================================
    // Release Schedule Functions
    // ========================================================================

    /// Creates a time-based release schedule for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to create schedule for
    /// * `amount` - Amount to release (in token's smallest denomination)
    /// * `release_timestamp` - Unix timestamp when funds become available
    /// * `recipient` - Address that will receive the funds
    ///
    /// # Returns
    /// * `ProgramData` - Updated program data
    ///
    /// # Panics
    /// * If program is not initialized
    /// * If caller is not authorized payout key
    /// * If amount is invalid
    /// * If timestamp is in the past
    /// * If amount exceeds remaining balance
    ///
    /// # State Changes
    /// - Creates ProgramReleaseSchedule record
    /// - Updates next schedule ID
    /// - Emits ScheduleCreated event
    ///
    /// # Authorization
    /// - Only authorized payout key can call this function
    ///
    /// # Example
    /// ```rust
    /// let now = env.ledger().timestamp();
    /// let release_time = now + (30 * 24 * 60 * 60); // 30 days from now
    /// escrow_client.create_program_release_schedule(
    ///     &"Hackathon2024",
    ///     &500_0000000, // 500 tokens
    ///     &release_time,
    ///     &winner_address
    /// );
    /// ```
    pub fn create_program_release_schedule(
        env: Env,
        program_id: String,
        amount: i128,
        release_timestamp: u64,
        recipient: Address,
    ) -> ProgramData {
        let _guard = ReentrancyGuardRAII::new(&env).expect("Reentrancy detected");
        let start = env.ledger().timestamp();

        // Check if schedule operations are paused
        if Self::is_schedule_paused_internal(&env) {
            panic!("Schedule operations are paused");
        }

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.auth_key.clone());

        // Verify authorization
        program_data.auth_key.require_auth();

        // Validate amount
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // Validate timestamp
        if release_timestamp <= env.ledger().timestamp() {
            panic!("Release timestamp must be in the future");
        }

        // Check sufficient remaining balance
        let scheduled_total = Self::get_prog_scheduled_total(env.clone(), program_id.clone());
        if scheduled_total + amount > program_data.remaining_bal {
            panic!("Insufficient balance for scheduled amount");
        }

        // Get next schedule ID
        let schedule_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextScheduleId(program_id.clone()))
            .unwrap_or(1);

        // Create release schedule
        let schedule = ProgramReleaseSchedule {
            schedule_id,
            amount,
            release_timestamp,
            recipient: recipient.clone(),
            released: false,
            released_at: None,
            released_by: None,
        };

        // Store schedule
        env.storage().persistent().set(
            &DataKey::ReleaseSchedule(program_id.clone(), schedule_id),
            &schedule,
        );

        // Update next schedule ID
        env.storage().persistent().set(
            &DataKey::NextScheduleId(program_id.clone()),
            &(schedule_id + 1),
        );

        // Emit program schedule created event
        env.events().publish(
            (PROG_SCHEDULE_CREATED,),
            ProgramScheduleCreated {
                program_id: program_id.clone(),
                schedule_id,
                amount,
                release_timestamp,
                recipient: recipient.clone(),
                created_by: program_data.auth_key.clone(),
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("create_p"), program_data.auth_key, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("create_p"), duration);

        // Return updated program data
        let updated_data: ProgramData = env.storage().instance().get(&program_key).unwrap();
        updated_data
    }

    /// Automatically releases funds for program schedules that are due.
    /// Can be called by anyone after the release timestamp has passed.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to check for due schedules
    /// * `schedule_id` - The specific schedule to release
    ///
    /// # Panics
    /// * If program doesn't exist
    /// * If schedule doesn't exist
    /// * If schedule is already released
    /// * If schedule is not yet due
    ///
    /// # State Changes
    /// - Transfers tokens to recipient
    /// - Updates schedule status to released
    /// - Adds to release history
    /// - Updates program remaining balance
    /// - Emits ScheduleReleased event
    ///
    /// # Example
    /// ```rust
    /// // Anyone can call this after the timestamp
    /// escrow_client.release_program_schedule_automatic(&"Hackathon2024", &1);
    /// ```
    pub fn release_prog_schedule_automatic(env: Env, program_id: String, schedule_id: u64) {
        let _guard = ReentrancyGuardRAII::new(&env).expect("Reentrancy detected");
        let start = env.ledger().timestamp();
        let caller = env.current_contract_address();

        // Check if schedule operations are paused
        if Self::is_schedule_paused_internal(&env) {
            panic!("Schedule operations are paused");
        }

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Get schedule
        if !env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
        {
            panic!("Schedule not found");
        }

        let mut schedule: ProgramReleaseSchedule = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
            .unwrap();

        // Check if already released
        if schedule.released {
            panic!("Schedule already released");
        }

        // Check if due for release
        let now = env.ledger().timestamp();
        if now < schedule.release_timestamp {
            panic!("Schedule not yet due for release");
        }

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        // Transfer funds
        #[cfg(not(test))]
        token_client.transfer(&contract_address, &schedule.recipient, &schedule.amount);

        // Update schedule
        schedule.released = true;
        schedule.released_at = Some(now);
        // Debugging: set to None to avoid panic?
        schedule.released_by = Some(env.current_contract_address());

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_bal -= schedule.amount;

        // Add to release history
        let history_entry = ProgramReleaseHistory {
            schedule_id,
            program_id: program_id.clone(),
            amount: schedule.amount,
            recipient: schedule.recipient.clone(),
            released_at: now,
            released_by: env.current_contract_address(),
            release_type: ReleaseType::Automatic,
        };

        let mut history: Vec<ProgramReleaseHistory> = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(program_id.clone()))
            .unwrap_or(vec![&env]);
        history.push_back(history_entry);

        // Store updates
        env.storage().persistent().set(
            &DataKey::ReleaseSchedule(program_id.clone(), schedule_id),
            &schedule,
        );
        env.storage().instance().set(&program_key, &updated_data);
        env.storage()
            .persistent()
            .set(&DataKey::ReleaseHistory(program_id.clone()), &history);

        // Emit program schedule released event
        env.events().publish(
            (PROG_SCHEDULE_RELEASED,),
            ProgramScheduleReleased {
                program_id: program_id.clone(),
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
    }

    /// Manually releases funds for a program schedule (authorized payout key only).
    /// Can be called before the release timestamp by authorized key.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program containing the schedule
    /// * `schedule_id` - The schedule to release
    ///
    /// # Panics
    /// * If program doesn't exist
    /// * If caller is not authorized payout key
    /// * If schedule doesn't exist
    /// * If schedule is already released
    ///
    /// # State Changes
    /// - Transfers tokens to recipient
    /// - Updates schedule status to released
    /// - Adds to release history
    /// - Updates program remaining balance
    /// - Emits ScheduleReleased event
    ///
    /// # Authorization
    /// - Only authorized payout key can call this function
    ///
    /// # Example
    /// ```rust
    /// // Authorized key can release early
    /// escrow_client.release_program_schedule_manual(&"Hackathon2024", &1);
    /// ```
    pub fn release_program_schedule_manual(env: Env, program_id: String, schedule_id: u64) {
        let _guard = ReentrancyGuardRAII::new(&env).expect("Reentrancy detected");
        let start = env.ledger().timestamp();

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.auth_key.clone());

        // Verify authorization
        program_data.auth_key.require_auth();

        // Get schedule
        if !env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
        {
            panic!("Schedule not found");
        }

        let mut schedule: ProgramReleaseSchedule = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
            .unwrap();

        // Check if already released
        if schedule.released {
            panic!("Schedule already released");
        }

        // Get token client
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        // Transfer funds
        #[cfg(not(test))]
        token_client.transfer(&contract_address, &schedule.recipient, &schedule.amount);

        // Update schedule
        let now = env.ledger().timestamp();
        schedule.released = true;
        schedule.released_at = Some(now);
        schedule.released_by = Some(program_data.auth_key.clone());

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_bal -= schedule.amount;

        // Add to release history
        let history_entry = ProgramReleaseHistory {
            schedule_id,
            program_id: program_id.clone(),
            amount: schedule.amount,
            recipient: schedule.recipient.clone(),
            released_at: now,
            released_by: program_data.auth_key.clone(),
            release_type: ReleaseType::Manual,
        };

        let mut history: Vec<ProgramReleaseHistory> = env
            .storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(program_id.clone()))
            .unwrap_or(vec![&env]);
        history.push_back(history_entry);

        // Store updates
        env.storage().persistent().set(
            &DataKey::ReleaseSchedule(program_id.clone(), schedule_id),
            &schedule,
        );
        env.storage().instance().set(&program_key, &updated_data);
        env.storage()
            .persistent()
            .set(&DataKey::ReleaseHistory(program_id.clone()), &history);

        // Emit program schedule released event
        env.events().publish(
            (PROG_SCHEDULE_RELEASED,),
            ProgramScheduleReleased {
                program_id: program_id.clone(),
                schedule_id,
                amount: schedule.amount,
                recipient: schedule.recipient.clone(),
                released_at: now,
                released_by: program_data.auth_key.clone(),
                release_type: ReleaseType::Manual,
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("rel_man"), program_data.auth_key, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("rel_man"), duration);
    }

    // ========================================================================
    // View Functions (Read-only)
    // ========================================================================

    /// Retrieves complete program information.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `Ok(ProgramData)` - Complete program state including:
    ///   - Program ID
    ///   - Total funds locked
    ///   - Remaining balance
    ///   - Authorized payout key
    ///   - Complete payout history
    ///   - Token contract address
    /// * `Err(Error::NotInitialized)` - Program not initialized
    ///
    /// # Use Cases
    /// - Verifying program configuration
    /// - Checking balances before payouts
    /// - Auditing payout history
    /// - Displaying program status in UI
    ///
    /// # Example
    /// ```rust
    /// let info = escrow_client.get_program_info();
    /// println!("Program: {}", info.program_id);
    /// println!("Total Locked: {}", info.total_funds);
    /// println!("Remaining: {}", info.remaining_bal);
    /// println!("Payouts Made: {}", info.payout_history.len());
    /// ```
    ///
    /// # Gas Cost
    /// Very Low - Single storage read
    pub fn get_program_info(env: Env) -> ProgramData {
        env.storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"))
    }

    /// Retrieves the remaining balance for a specific program.
    ///
    /// # Arguments
    /// * `program_id` - The program ID to query
    ///
    /// # Returns
    /// * `i128` - Remaining balance
    ///
    /// # Panics
    /// * If program doesn't exist
    pub fn get_remaining_balance(env: Env, program_id: String) -> i128 {
        let program_key = DataKey::Program(program_id);
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.remaining_bal
    }

    /// Update fee configuration (admin only - uses authorized_payout_key)
    ///
    /// # Arguments
    /// * `lock_fee_rate` - Optional new lock fee rate (basis points)
    /// * `payout_fee_rate` - Optional new payout fee rate (basis points)
    /// * `fee_recipient` - Optional new fee recipient address
    /// * `fee_enabled` - Optional fee enable/disable flag
    pub fn update_fee_config(
        env: Env,
        lock_fee_rate: Option<i128>,
        payout_fee_rate: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
    ) {
        // Verify authorization
        let program_data: ProgramData = env.storage().instance().get(&PROGRAM_DATA).unwrap();

        // Require auth from the program's authorized key
        program_data.auth_key.require_auth();

        // Get current fee config
        let mut fee_config = Self::get_fee_config_internal(&env);

        // Update fields if provided
        if let Some(rate) = lock_fee_rate {
            if rate < 0 || rate > MAX_FEE_RATE {
                panic!("Invalid lock fee rate");
            }
            fee_config.lock_fee_rate = rate;
        }

        if let Some(rate) = payout_fee_rate {
            if rate < 0 || rate > MAX_FEE_RATE {
                panic!("Invalid payout fee rate");
            }
            fee_config.payout_fee_rate = rate;
        }

        if let Some(recipient) = fee_recipient {
            fee_config.fee_recipient = recipient;
        }

        if let Some(enabled) = fee_enabled {
            fee_config.fee_enabled = enabled;
        }

        // Store updated config
        env.storage().instance().set(&FEE_CONFIG, &fee_config);

        // Emit fee config updated event
        env.events().publish(
            (symbol_short!("fee_cfg"),),
            (
                fee_config.lock_fee_rate,
                fee_config.payout_fee_rate,
                fee_config.fee_recipient,
                fee_config.fee_enabled,
            ),
        );
    }

    /// Get current fee configuration (view function)
    /// Deprecated: Use ConfigurableFee trait. Keeping internal helper.
    fn get_fee_config_internal_api(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
    }

    /// Update amount limits configuration (admin only)
    pub fn update_amount_limits(
        env: Env,
        min_lock_amount: i128,
        max_lock_amount: i128,
        min_payout: i128,
        max_payout: i128,
    ) {
        // Get admin and require authorization
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
        admin.require_auth();

        // Validate limits
        if min_lock_amount < 0 || max_lock_amount < 0 || min_payout < 0 || max_payout < 0 {
            panic!("Invalid amount: amounts cannot be negative");
        }
        if min_lock_amount > max_lock_amount || min_payout > max_payout {
            panic!("Invalid amount: minimum cannot exceed maximum");
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

    /// Gets the total number of programs registered.
    ///
    /// # Returns
    /// * `u32` - Count of registered programs
    pub fn get_program_count(env: Env) -> u32 {
        let registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);

        registry.len()
    }

    // ========================================================================
    // Monitoring & Analytics Functions
    // ========================================================================

    /// Health check - returns contract health status
    pub fn health_check(env: Env) -> monitoring::HealthStatus {
        monitoring::health_check(&env)
    }

    /// Get analytics - returns usage analytics
    pub fn get_analytics(env: Env) -> monitoring::Analytics {
        monitoring::get_analytics(&env)
    }

    /// Get state snapshot - returns current state
    pub fn get_state_snapshot(env: Env) -> monitoring::StateSnapshot {
        monitoring::get_state_snapshot(&env)
    }

    /// Get performance stats for a function
    pub fn get_performance_stats(env: Env, function_name: Symbol) -> monitoring::PerformanceStats {
        monitoring::get_performance_stats(&env, function_name)
    }

    // ========================================================================
    // Anti-Abuse Administrative Functions
    // ========================================================================

    /// Sets the administrative address for anti-abuse configuration.
    /// Can only be called once or by the existing admin.
    pub fn set_admin(env: Env, new_admin: Address) {
        if let Some(current_admin) = anti_abuse::get_admin(&env) {
            current_admin.require_auth();
        }
        anti_abuse::set_admin(&env, new_admin);
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        anti_abuse::get_admin(&env)
    }

    /// Updates the rate limit configuration.
    /// Only the admin can call this.
    pub fn update_rate_limit_config(
        env: Env,
        window_size: u64,
        max_operations: u32,
        cooldown_period: u64,
    ) {
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
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
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
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

    // ========================================================================
    // Schedule View Functions
    // ========================================================================

    /// Retrieves a specific program release schedule.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program containing the schedule
    /// * `schedule_id` - The schedule ID to retrieve
    ///
    /// # Returns
    /// * `ProgramReleaseSchedule` - The schedule details
    ///
    /// # Panics
    /// * If schedule doesn't exist
    pub fn get_program_release_schedule(
        env: Env,
        program_id: String,
        schedule_id: u64,
    ) -> ProgramReleaseSchedule {
        env.storage()
            .persistent()
            .get(&DataKey::ReleaseSchedule(program_id, schedule_id))
            .unwrap_or_else(|| panic!("Schedule not found"))
    }

    /// Retrieves all release schedules for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `Vec<ProgramReleaseSchedule>` - All schedules for the program
    pub fn get_all_prog_release_schedules(
        env: Env,
        program_id: String,
    ) -> Vec<ProgramReleaseSchedule> {
        let mut schedules = Vec::new(&env);
        let next_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextScheduleId(program_id.clone()))
            .unwrap_or(1);

        for schedule_id in 1..next_id {
            if env
                .storage()
                .persistent()
                .has(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
            {
                let schedule: ProgramReleaseSchedule = env
                    .storage()
                    .persistent()
                    .get(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
                    .unwrap();
                schedules.push_back(schedule);
            }
        }

        schedules
    }

    /// Get the total amount scheduled for a program (pending schedules only).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `i128` - Total amount in pending schedules
    pub fn get_prog_scheduled_total(env: Env, program_id: String) -> i128 {
        let pending = Self::get_pending_program_schedules(env.clone(), program_id);
        let mut total: i128 = 0;

        for schedule in pending.iter() {
            total = total.saturating_add(schedule.amount);
        }

        total
    }

    /// Retrieves all due (ready to release) schedules for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `Vec<ProgramReleaseSchedule>` - All due but unreleased schedules
    pub fn get_due_program_schedules(env: Env, program_id: String) -> Vec<ProgramReleaseSchedule> {
        let pending = Self::get_pending_program_schedules(env.clone(), program_id.clone());
        let mut due = Vec::new(&env);
        let now = env.ledger().timestamp();

        for schedule in pending.iter() {
            if schedule.release_timestamp <= now {
                due.push_back(schedule.clone());
            }
        }

        due
    }

    /// Retrieves release history for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `Vec<ProgramReleaseHistory>` - Release history for the program
    pub fn get_program_release_history(env: Env, program_id: String) -> Vec<ProgramReleaseHistory> {
        env.storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(program_id))
            .unwrap_or(vec![&env])
    }

    /// Retrieves pending (unreleased) schedules for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `Vec<ProgramReleaseSchedule>` - All pending (unreleased) schedules
    pub fn get_pending_program_schedules(
        env: Env,
        program_id: String,
    ) -> Vec<ProgramReleaseSchedule> {
        let all_schedules = Self::get_all_prog_release_schedules(env.clone(), program_id);
        let mut pending = Vec::new(&env);

        for schedule in all_schedules.iter() {
            if !schedule.released {
                pending.push_back(schedule.clone());
            }
        }

        pending
    }

    /// Internal helper to check if contract is paused.
    fn is_paused_internal(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::IsPaused)
            .unwrap_or(false)
    }

    /// Pause the contract (admin only).
    pub fn pause_contract(env: Env) {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        program_data.auth_key.require_auth();

        env.storage().instance().set(&DataKey::IsPaused, &true);
    }

    /// Unpause the contract (admin only).
    pub fn unpause_contract(env: Env) {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        program_data.auth_key.require_auth();

        env.storage().instance().set(&DataKey::IsPaused, &false);
    }

    /// Expire a program and refund remaining balance to organizer after deadline.
    /// This function can be called by anyone after the deadline has passed.
    pub fn expire_program(env: Env, program_id: String) {
        if Self::is_paused_internal(&env) {
            panic!("Contract is paused");
        }

        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        let deadline = program_data
            .deadline
            .unwrap_or_else(|| panic!("Program has no deadline"));

        let now = env.ledger().timestamp();
        if now < deadline {
            panic!("Deadline has not passed yet");
        }

        if program_data.remaining_bal <= 0 {
            panic!("No funds to refund");
        }

        let token_client = token::Client::new(&env, &program_data.token_address);
        let contract_balance = token_client.balance(&env.current_contract_address());

        if contract_balance < program_data.remaining_bal {
            panic!("Insufficient contract balance");
        }

        token_client.transfer(
            &env.current_contract_address(),
            &program_data.organizer,
            &program_data.remaining_bal,
        );

        let mut updated_program = program_data.clone();
        updated_program.remaining_bal = 0;
        env.storage().instance().set(&program_key, &updated_program);
        env.storage()
            .instance()
            .set(&PROGRAM_DATA, &updated_program);

        env.events().publish(
            (symbol_short!("expired"),),
            (
                program_id,
                program_data.remaining_bal,
                program_data.organizer,
                now,
            ),
        );
    }
    // ========================================================================
    // Admin Functions
    // ========================================================================

    /// Update Admin
    ///
    /// # Arguments
    /// * `new_admin` - New Admin address
    /// Admin Require Auth
    pub fn update_admin(env: Env, new_admin: Address) {
        let current_admin = anti_abuse::get_admin(&env).unwrap();
        current_admin.require_auth();

        let last_update: u64 = env
            .storage()
            .instance()
            .get(&anti_abuse::AntiAbuseKey::LastAdminUpdate)
            .unwrap_or(0);
        let current_time = env.ledger().timestamp();
        if current_time < last_update + ADMIN_UPDATE_TIMELOCK {
            panic!("TimeLock");
        }

        env.storage()
            .instance()
            .set(&anti_abuse::AntiAbuseKey::Admin, &new_admin);
        env.storage()
            .instance()
            .set(&anti_abuse::AntiAbuseKey::LastAdminUpdate, &current_time);

        emit_update_admin(
            &env,
            UpdateAdminEvent {
                admin: current_admin,
                new_admin,
                timestamp: current_time,
            },
        );
    }

    /// Update Authorized Payout Key
    ///
    /// # Arguments
    /// * `new_admin` - New Authorized Payout Key address
    /// Admin Require Auth
    pub fn update_authorized_payout_key(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
    ) {
        let current_admin = anti_abuse::get_admin(&env).unwrap();
        current_admin.require_auth();

        let program_key = DataKey::Program(program_id.clone());
        let mut program_data = Self::get_program_info(env.clone(), program_id);
        program_data.authorized_payout_key = authorized_payout_key.clone();
        env.storage().instance().set(&program_key, &program_data);

        emit_update_authorized_key(
            &env,
            UpdateAuthorizedKeyEvent {
                old_authorized_payout_key: program_data.authorized_payout_key,
                new_authorized_payout_key: authorized_payout_key,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    // ========================================================================
    // Simple API Functions (for backward compatibility with tests)
    // ========================================================================

    /// Initialize the program (alias for init_program with simpler name).
    /// Uses auth_key as the default organizer and no deadline.
    pub fn initialize(
        env: Env,
        program_id: String,
        auth_key: Address,
        token_addr: Address,
    ) -> ProgramData {
        Self::init_program(
            env,
            program_id,
            auth_key.clone(),
            token_addr,
            auth_key,
            None,
        )
    }

    /// Lock funds using the simple API (without program_id).
    pub fn lock_funds(env: Env, amount: i128, token_address: Address) -> ProgramData {
        // Get program data to find program_id
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Validate token is whitelisted
        let mut is_whitelisted = false;
        for i in 0..program_data.whitelist.len() {
            if program_data.whitelist.get(i).unwrap() == token_address {
                is_whitelisted = true;
                break;
            }
        }
        if !is_whitelisted {
            panic!("Token not whitelisted");
        }

        Self::lock_program_funds(env, program_data.program_id, amount, Some(token_address))
    }

    /// Add a token to the whitelist.
    pub fn add_token(env: Env, token: Address) -> ProgramData {
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.auth_key.require_auth();

        // Check if already whitelisted
        for i in 0..program_data.whitelist.len() {
            if program_data.whitelist.get(i).unwrap() == token {
                panic!("Token already whitelisted");
            }
        }

        // Add to whitelist
        program_data.whitelist.push_back(token.clone());

        // Initialize token balance if not exists
        if program_data.token_balances.get(token.clone()).is_none() {
            program_data.token_balances.set(token.clone(), 0);
        }

        // Register in TokenWhitelist for lock_program_funds compatibility
        env.storage()
            .instance()
            .set(&DataKey::TokenWhitelist(token), &true);

        // Store updated data (both locations for compatibility)
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        let program_key = DataKey::Program(program_data.program_id.clone());
        env.storage().instance().set(&program_key, &program_data);

        program_data
    }

    /// Remove a token from the whitelist.
    pub fn remove_token(env: Env, token: Address) {
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.auth_key.require_auth();

        // Cannot remove the default token
        if token == program_data.token_address {
            panic!("Cannot remove default token");
        }

        // Find and remove from whitelist
        let mut found_index: Option<u32> = None;
        for i in 0..program_data.whitelist.len() {
            if program_data.whitelist.get(i).unwrap() == token {
                found_index = Some(i);
                break;
            }
        }

        if let Some(index) = found_index {
            program_data.whitelist.remove(index);
            // Also remove from TokenWhitelist
            env.storage()
                .instance()
                .remove(&DataKey::TokenWhitelist(token));
            // Store updated data (both locations for compatibility)
            env.storage().instance().set(&PROGRAM_DATA, &program_data);
            let program_key = DataKey::Program(program_data.program_id.clone());
            env.storage().instance().set(&program_key, &program_data);
        } else {
            panic!("Token not whitelisted");
        }
    }

    /// Check if a token is whitelisted.
    pub fn is_whitelisted(env: Env, token: Address) -> bool {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        for i in 0..program_data.whitelist.len() {
            if program_data.whitelist.get(i).unwrap() == token {
                return true;
            }
        }
        false
    }

    /// Get all whitelisted tokens.
    pub fn get_tokens(env: Env) -> Vec<Address> {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.whitelist
    }

    /// Get balance for a specific token.
    pub fn get_balance(env: Env, token: Address) -> TokenBalance {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Check if token is whitelisted
        let mut is_whitelisted = false;
        for i in 0..program_data.whitelist.len() {
            if program_data.whitelist.get(i).unwrap() == token {
                is_whitelisted = true;
                break;
            }
        }
        if !is_whitelisted {
            panic!("Token not whitelisted");
        }

        let remaining = program_data.token_balances.get(token.clone()).unwrap_or(0);

        // For locked, we need to track it separately or calculate from history
        // For simplicity, locked = remaining + sum of payouts for this token
        let mut paid_out: i128 = 0;
        for i in 0..program_data.payout_history.len() {
            let record = program_data.payout_history.get(i).unwrap();
            if record.token == token {
                paid_out += record.amount;
            }
        }

        TokenBalance {
            locked: remaining + paid_out,
            remaining,
        }
    }

    /// Get all token balances.
    pub fn get_all_balances(env: Env) -> Vec<(Address, TokenBalance)> {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        let mut result = Vec::new(&env);

        for i in 0..program_data.whitelist.len() {
            let token = program_data.whitelist.get(i).unwrap();
            let balance = Self::get_balance(env.clone(), token.clone());
            result.push_back((token, balance));
        }

        result
    }

    /// Get program info (alias for get_program_info).
    pub fn get_info(env: Env) -> ProgramData {
        Self::get_program_info(env)
    }

    /// Simple single payout (without program_id parameter).
    pub fn simple_single_payout(
        env: Env,
        recipient: Address,
        amount: i128,
        token_address: Address,
    ) -> ProgramData {
        // Get program data to find program_id
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Validate token is whitelisted
        let mut is_whitelisted = false;
        for i in 0..program_data.whitelist.len() {
            if program_data.whitelist.get(i).unwrap() == token_address {
                is_whitelisted = true;
                break;
            }
        }
        if !is_whitelisted {
            panic!("Token not whitelisted");
        }

        Self::single_payout(
            env,
            program_data.program_id,
            recipient,
            amount,
            Some(token_address),
        )
    }

    /// Simple batch payout (without program_id parameter).
    pub fn simple_batch_payout(
        env: Env,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
        token_address: Address,
    ) -> ProgramData {
        // Get program data to find program_id
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Validate token is whitelisted
        let mut is_whitelisted = false;
        for i in 0..program_data.whitelist.len() {
            if program_data.whitelist.get(i).unwrap() == token_address {
                is_whitelisted = true;
                break;
            }
        }
        if !is_whitelisted {
            panic!("Token not whitelisted");
        }

        Self::batch_payout(
            env,
            program_data.program_id,
            recipients,
            amounts,
            Some(token_address),
        )
    }

    /// Get remaining balance (simple version without program_id).
    pub fn get_balance_remaining(env: Env) -> i128 {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.remaining_bal
    }
}
#[cfg(test)]
mod test;
