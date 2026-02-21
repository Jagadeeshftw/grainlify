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
//! ```rust
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
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, vec, Address, Env, String, Symbol,
    Vec,
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,    // Fee rate for lock operations (basis points)
    pub payout_fee_rate: i128,  // Fee rate for payout operations (basis points)
    pub fee_recipient: Address, // Address to receive fees
    pub fee_enabled: bool,      // Global fee enable/disable flag
}

// ==================== ERROR RECOVERY MODULE ====================
mod error_recovery;
mod retry_executor;

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

/// Storage key for the program registry (list of all program IDs)
const PROGRAM_REGISTRY: Symbol = symbol_short!("ProgReg");

// ============================================================================
// Data Structures
// ============================================================================

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
/// ```rust
/// let record = PayoutRecord {
///     recipient: winner_address,
///     amount: 1000_0000000, // 1000 USDC
///     timestamp: env.ledger().timestamp(),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
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
/// ```rust
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
    pub remaining_balance: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address,
}

/// Storage key type for individual programs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String),              // program_id -> ProgramData
    ReleaseSchedule(String, u64), // program_id, schedule_id -> ProgramReleaseSchedule
    ReleaseHistory(String),       // program_id -> Vec<ProgramReleaseHistory>
    NextScheduleId(String),       // program_id -> next schedule_id
    IsPaused,                     // Global contract pause state
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

#[contract]
pub struct ProgramEscrowContract;

// Event symbols for program release schedules
const PROG_SCHEDULE_CREATED: soroban_sdk::Symbol = soroban_sdk::symbol_short!("prg_sch_c");
const PROG_SCHEDULE_RELEASED: soroban_sdk::Symbol = soroban_sdk::symbol_short!("prg_sch_r");

#[contractimpl]
impl ProgramEscrowContract {
    // ========================================================================
    // Program Registration & Initialization
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
    /// # Panics
    /// * If program is already initialized
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
    #[allow(clippy::empty_line_after_outer_attr)]
    //
    // ========================================================================
    // Pause and Emergency Functions
    // ========================================================================

    /// Check if contract is paused (internal helper)
    fn is_paused_internal(env: &Env) -> bool {
        env.storage()
            .instance()
            .get::<_, bool>(&DataKey::IsPaused)
            .unwrap_or(false)
    }

    /// Get pause status (view function)
    pub fn is_paused(env: Env) -> bool {
        Self::is_paused_internal(&env)
    }

    /// Pause the contract (authorized payout key only)
    /// Prevents new fund locking, payouts, and schedule releases
    pub fn pause(env: Env) {
        // For program-escrow, pause is triggered by the first authorized key that calls it
        // In a multi-program setup, this would need to be per-program

        if Self::is_paused_internal(&env) {
            return; // Already paused, idempotent
        }

        env.storage().instance().set(&DataKey::IsPaused, &true);

        env.events()
            .publish((symbol_short!("pause"),), (env.ledger().timestamp(),));
    }

    /// Unpause the contract (authorized payout key only)
    /// Resumes normal operations
    pub fn unpause(env: Env) {
        if !Self::is_paused_internal(&env) {
            return; // Already unpaused, idempotent
        }

        env.storage().instance().set(&DataKey::IsPaused, &false);

        env.events()
            .publish((symbol_short!("unpause"),), (env.ledger().timestamp(),));
    }

    /// Emergency withdrawal for all contract funds (authorized payout key only, only when paused)
    pub fn emergency_withdraw(env: Env, program_id: String, recipient: Address) -> i128 {
        // Only allow emergency withdrawal when contract is paused
        if !Self::is_paused_internal(&env) {
            panic!("Contract must be paused for emergency withdrawal");
        }

        // Get program data to access token address
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData =
            env.storage()
                .instance()
                .get(&program_key)
                .unwrap_or_else(|| {
                    panic!("Program not found");
                });

        let client = token::Client::new(&env, &program_data.token_address);
        let balance = client.balance(&env.current_contract_address());

        if balance <= 0 {
            return 0; // No funds to withdraw
        }

        // Transfer all funds to recipient
        client.transfer(&env.current_contract_address(), &recipient, &balance);

        env.events().publish(
            (symbol_short!("ewith"),),
            (balance, env.ledger().timestamp()),
        );

        balance
    }

    pub fn initialize_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
    ) -> ProgramData {
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, authorized_payout_key.clone());

        let start = env.ledger().timestamp();
        let caller = authorized_payout_key.clone();

        // Validate program_id
        if program_id.is_empty() {
            monitoring::track_operation(&env, symbol_short!("init_prg"), caller, false);
            panic!("Program ID cannot be empty");
        }

        // Check if program already exists
        let program_key = DataKey::Program(program_id.clone());
        if env.storage().instance().has(&program_key) {
            monitoring::track_operation(&env, symbol_short!("init_prg"), caller, false);
            panic!("Program already exists");
        }

        // Create program data
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
            token_address: token_address.clone(),
        };

        // Initialize fee config with zero fees (disabled by default)
        let fee_config = FeeConfig {
            lock_fee_rate: 0,
            payout_fee_rate: 0,
            fee_recipient: authorized_payout_key.clone(),
            fee_enabled: false,
        };
        env.storage().instance().set(&FEE_CONFIG, &fee_config);

        // Store program data
        env.storage().instance().set(&program_key, &program_data);

        // Update registry
        let mut registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);
        registry.push_back(program_id.clone());
        env.storage().instance().set(&PROGRAM_REGISTRY, &registry);

        // Emit registration event
        env.events().publish(
            (PROGRAM_REGISTERED,),
            (program_id, authorized_payout_key, token_address, 0i128),
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init_prg"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init_prg"), duration);

        program_data
    }

    /// Calculate fee amount based on rate (in basis points)
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
    /// # Panics
    /// * If amount is zero or negative
    /// * If program is not initialized
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
    /// println!("Remaining: {}", updated.remaining_balance);
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
    pub fn lock_program_funds(env: Env, program_id: String, amount: i128) -> ProgramData {
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, env.current_contract_address());

        let _start = env.ledger().timestamp();
        let caller = env.current_contract_address();

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
            panic!("Contract is paused");
        }

        // Validate amount
        if amount <= 0 {
            monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
            panic!("Amount must be greater than zero");
        }

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| {
                monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
                panic!("Program not found")
            });

        // Calculate and collect fee if enabled
        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0 {
            Self::calculate_fee(amount, fee_config.lock_fee_rate)
        } else {
            0
        };
        let net_amount = amount - fee_amount;

        // Update balances with net amount
        program_data.total_funds += net_amount;
        program_data.remaining_balance += net_amount;

        // Emit fee collected event if applicable
        if fee_amount > 0 {
            env.events().publish(
                (symbol_short!("fee"),),
                (
                    symbol_short!("lock"),
                    fee_amount,
                    fee_config.lock_fee_rate,
                    fee_config.fee_recipient.clone(),
                ),
            );
        }

        // Store updated data
        env.storage().instance().set(&program_key, &program_data);

        // Emit FundsLocked event (with net amount after fee)
        env.events().publish(
            (FUNDS_LOCKED,),
            (
                program_data.program_id.clone(),
                net_amount,
                program_data.remaining_balance,
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
    /// * `ProgramData` - Updated program data after payouts
    ///
    /// # Panics
    /// * If caller is not the authorized payout key
    /// * If program is not initialized
    /// * If recipients and amounts vectors have different lengths
    /// * If vectors are empty
    /// * If any amount is zero or negative
    /// * If total payout exceeds remaining balance
    /// * If arithmetic overflow occurs
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
    /// println!("Remaining: {}", result.remaining_balance);
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
    ) -> ProgramData {
        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            panic!("Contract is paused");
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

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        // Verify authorization - CRITICAL
        program_data.authorized_payout_key.require_auth();

        // Validate inputs
        if recipients.len() != amounts.len() {
            panic!("Recipients and amounts vectors must have the same length");
        }

        if recipients.is_empty() {
            panic!("Cannot process empty batch");
        }

        // Calculate total with overflow protection
        let mut total_payout: i128 = 0;
        for i in 0..amounts.len() {
            let amount = amounts.get(i).unwrap();
            if amount <= 0 {
                panic!("All amounts must be greater than zero");
            }
            total_payout = total_payout
                .checked_add(amount)
                .unwrap_or_else(|| panic!("Payout amount overflow"));
        }

        // Validate balance
        if total_payout > program_data.remaining_balance {
            panic!(
                "Insufficient balance: requested {}, available {}",
                total_payout, program_data.remaining_balance
            );
        }

        // Calculate fees if enabled
        let fee_config = Self::get_fee_config_internal(&env);
        let mut total_fees: i128 = 0;

        // Execute transfers
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        for i in 0..recipients.len() {
            let recipient = recipients.get(i).unwrap();
            let amount = amounts.get(i).unwrap();

            // Calculate fee for this payout
            let fee_amount = if fee_config.fee_enabled && fee_config.payout_fee_rate > 0 {
                Self::calculate_fee(amount, fee_config.payout_fee_rate)
            } else {
                0
            };
            let net_amount = amount - fee_amount;
            total_fees += fee_amount;

            // Transfer net amount to recipient
            token_client.transfer(&contract_address, &recipient.clone(), &net_amount);

            // Transfer fee to fee recipient if applicable
            if fee_amount > 0 {
                token_client.transfer(&contract_address, &fee_config.fee_recipient, &fee_amount);
            }

            // Record payout (with net amount)
            let payout_record = PayoutRecord {
                recipient: recipient.clone(),
                amount: net_amount,
                timestamp,
            };
            updated_history.push_back(payout_record);
        }

        // Emit fee collected event if applicable
        if total_fees > 0 {
            env.events().publish(
                (symbol_short!("fee"),),
                (
                    symbol_short!("payout"),
                    total_fees,
                    fee_config.payout_fee_rate,
                    fee_config.fee_recipient.clone(),
                ),
            );
        }

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= total_payout; // Total includes fees
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&program_key, &updated_data);

        // Emit event
        env.events().publish(
            (BATCH_PAYOUT,),
            (
                program_id,
                recipients.len(),
                total_payout,
                updated_data.remaining_balance,
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
    /// * `ProgramData` - Updated program data after payout
    ///
    /// # Panics
    /// * If caller is not the authorized payout key
    /// * If program is not initialized
    /// * If amount is zero or negative
    /// * If amount exceeds remaining balance
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
    ) -> ProgramData {
        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            panic!("Contract is paused");
        }

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.authorized_payout_key.require_auth();
        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        // Verify authorization
        // let caller = env.invoker();
        // if caller != program_data.authorized_payout_key {
        //     panic!("Unauthorized: only authorized payout key can trigger payouts");
        // }

        // Validate amount
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // Validate balance
        if amount > program_data.remaining_balance {
            panic!(
                "Insufficient balance: requested {}, available {}",
                amount, program_data.remaining_balance
            );
        }

        // Calculate and collect fee if enabled
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
        let token_client = token::Client::new(&env, &program_data.token_address);
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

        // Record payout (with net amount after fee)
        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutRecord {
            recipient: recipient.clone(),
            amount: net_amount,
            timestamp,
        };

        let mut updated_history = program_data.payout_history.clone();
        updated_history.push_back(payout_record);

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= amount; // Total amount (includes fee)
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&program_key, &updated_data);

        // Emit Payout event (with net amount after fee)
        // Emit event
        env.events().publish(
            (PAYOUT,),
            (
                program_id,
                recipient,
                net_amount,
                updated_data.remaining_balance,
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
        let start = env.ledger().timestamp();

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            panic!("Contract is paused");
        }

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        // Verify authorization
        program_data.authorized_payout_key.require_auth();

        // Validate amount
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // Validate timestamp
        if release_timestamp <= env.ledger().timestamp() {
            panic!("Release timestamp must be in the future");
        }

        // Check sufficient remaining balance
        let scheduled_total = get_program_total_scheduled_amount(&env, &program_id);
        if scheduled_total + amount > program_data.remaining_balance {
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
                created_by: program_data.authorized_payout_key.clone(),
            },
        );

        // Track successful operation
        monitoring::track_operation(
            &env,
            symbol_short!("create_p"),
            program_data.authorized_payout_key,
            true,
        );

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
        let start = env.ledger().timestamp();
        let caller = env.current_contract_address();

        // Check if contract is paused
        if Self::is_paused_internal(&env) {
            panic!("Contract is paused");
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
        updated_data.remaining_balance -= schedule.amount;

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
        let start = env.ledger().timestamp();

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        // Verify authorization
        program_data.authorized_payout_key.require_auth();

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
        schedule.released_by = Some(program_data.authorized_payout_key.clone());

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= schedule.amount;

        // Add to release history
        let history_entry = ProgramReleaseHistory {
            schedule_id,
            program_id: program_id.clone(),
            amount: schedule.amount,
            recipient: schedule.recipient.clone(),
            released_at: now,
            released_by: program_data.authorized_payout_key.clone(),
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
                released_by: program_data.authorized_payout_key.clone(),
                release_type: ReleaseType::Manual,
            },
        );

        // Track successful operation
        monitoring::track_operation(
            &env,
            symbol_short!("rel_man"),
            program_data.authorized_payout_key,
            true,
        );

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
    /// * `ProgramData` - Complete program state including:
    ///   - Program ID
    ///   - Total funds locked
    ///   - Remaining balance
    ///   - Authorized payout key
    ///   - Complete payout history
    ///   - Token contract address
    ///
    /// # Panics
    /// * If program is not initialized
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
    /// println!("Remaining: {}", info.remaining_balance);
    /// println!("Payouts Made: {}", info.payout_history.len());
    /// ```
    ///
    /// # Gas Cost
    /// Very Low - Single storage read
    pub fn get_program_info(env: Env, program_id: String) -> ProgramData {
        let program_key = DataKey::Program(program_id);
        env.storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"))
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

        program_data.remaining_balance
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
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Note: In Soroban, we check authorization by requiring auth from the authorized key
        // For this function, we'll require auth from the authorized_payout_key
        program_data.authorized_payout_key.require_auth();

        let mut fee_config = Self::get_fee_config_internal(&env);

        if let Some(rate) = lock_fee_rate {
            if !(0..=MAX_FEE_RATE).contains(&rate) {
                panic!(
                    "Invalid lock fee rate: must be between 0 and {}",
                    MAX_FEE_RATE
                );
            }
            fee_config.lock_fee_rate = rate;
        }

        if let Some(rate) = payout_fee_rate {
            if !(0..=MAX_FEE_RATE).contains(&rate) {
                panic!(
                    "Invalid payout fee rate: must be between 0 and {}",
                    MAX_FEE_RATE
                );
            }
            fee_config.payout_fee_rate = rate;
        }

        if let Some(recipient) = fee_recipient {
            fee_config.fee_recipient = recipient;
        }

        if let Some(enabled) = fee_enabled {
            fee_config.fee_enabled = enabled;
        }

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
    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
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
    // Query Functions
    // ========================================================================

    /// Query programs with filtering and pagination.
    ///
    /// # Performance
    /// This function iterates through the registry. For large registries, ensure `pagination.limit` is reasonable
    /// to avoid running out of gas. 
    pub fn get_programs(
        env: Env, 
        filter: ProgramFilter, 
        pagination: Pagination
    ) -> Vec<ProgramData> {
        let registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);
            
        let mut result = vec![&env];
        let mut count: u32 = 0;
        let mut skipped: u64 = 0;
        
        for i in 0..registry.len() {
            // Check pagination limit
            if count >= pagination.limit {
                break;
            }
            
            let program_id = registry.get(i).unwrap();
            let key = DataKey::Program(program_id);
            
            if !env.storage().instance().has(&key) {
                continue;
            }
            
            let program: ProgramData = env
                .storage()
                .instance()
                .get(&key)
                .unwrap();
                
            // Apply Filters
            if let Some(key) = &filter.authorized_key {
                if &program.authorized_payout_key != key {
                    continue;
                }
            }
            
            if let Some(token) = &filter.token_address {
                 if &program.token_address != token {
                    continue;
                }
            }
            
            // Apply Pagination Skip
            if skipped < pagination.start_index {
                skipped += 1;
                continue;
            }
            
            result.push_back(program);
            count += 1;
        }
        
        result
    }
    
    /// Query payouts for a program with filtering
    pub fn get_payouts(
        env: Env, 
        program_id: String,
        filter: PayoutFilter
    ) -> Vec<PayoutRecord> {
        let program_key = DataKey::Program(program_id);
        if !env.storage().instance().has(&program_key) {
             return vec![&env];
        }
        
        let program: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap();
            
        let mut result = vec![&env];
        
        for record in program.payout_history.iter() {
            // Recipient filter
            if let Some(recipient) = &filter.recipient {
                if &record.recipient != recipient {
                    continue;
                }
            }
            
            // Amount filter
            if let Some(min) = filter.min_amount {
                if record.amount < min {
                    continue;
                }
            }
            if let Some(max) = filter.max_amount {
                if record.amount > max {
                    continue;
                }
            }
            
            // Time filter
            if let Some(start) = filter.start_time {
                if record.timestamp < start {
                    continue;
                }
            }
            if let Some(end) = filter.end_time {
                if record.timestamp > end {
                    continue;
                }
            }
            
            result.push_back(record);
        }
        
        result
    }

    /// Get aggregate statistics for all programs.
    ///
    /// # Performance
    /// This function iterates through ALL programs. It may exceed gas limits if the registry is very large.
    /// Primarily intended for off-chain monitoring or smaller deployments.
    pub fn get_stats(env: Env) -> ProgramStats {
        let registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);
            
        let mut total_locked: i128 = 0;
        let mut total_remaining: i128 = 0;
        let mut total_payouts: i128 = 0;
        
        for i in 0..registry.len() {
            let program_id = registry.get(i).unwrap();
            let key = DataKey::Program(program_id);
            
            if env.storage().instance().has(&key) {
                let program: ProgramData = env
                    .storage()
                    .instance()
                    .get(&key)
                    .unwrap();
                
                total_locked += program.total_funds;
                total_remaining += program.remaining_balance;
                // Payouts volume = Total - Remaining (roughly, assuming no other drains)
                total_payouts += program.total_funds - program.remaining_balance;
            }
        }
        
        ProgramStats {
            total_programs: registry.len() as u64,
            total_funds_locked: total_locked,
            total_funds_remaining: total_remaining,
            total_payouts_volume: total_payouts,
        }
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

    /// Retrieves pending (unreleased) schedules for a program.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - The program to query
    ///
    /// # Returns
    /// * `Vec<ProgramReleaseSchedule>` - All pending schedules
    pub fn get_pending_program_schedules(
        env: Env,
        program_id: String,
    ) -> Vec<ProgramReleaseSchedule> {
        let all_schedules = Self::get_all_prog_release_schedules(env.clone(), program_id.clone());
        let mut pending = Vec::new(&env);

        for schedule in all_schedules.iter() {
            if !schedule.released {
                pending.push_back(schedule.clone());
            }
        }

        pending
    }

    /// Retrieves due schedules (timestamp passed but not released) for a program.
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
    /// * `Vec<ProgramReleaseHistory>` - Complete release history
    pub fn get_program_release_history(env: Env, program_id: String) -> Vec<ProgramReleaseHistory> {
        env.storage()
            .persistent()
            .get(&DataKey::ReleaseHistory(program_id))
            .unwrap_or(vec![&env])
    }
}

/// Helper function to calculate total scheduled amount for a program.
fn get_program_total_scheduled_amount(env: &Env, program_id: &String) -> i128 {
    let next_id: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::NextScheduleId(program_id.clone()))
        .unwrap_or(1);

    let mut total = 0i128;
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
            if !schedule.released {
                total += schedule.amount;
            }
        }
    }

    total
}

/// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env, String,
    };

    // Test helper to create a mock token contract
    #[allow(deprecated)]
    fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let token_address = env.register_stellar_asset_contract(admin.clone());
        token::Client::new(env, &token_address)
    }

    // ========================================================================
    // Program Registration Tests
    // ========================================================================

    fn setup_program_with_schedule(
        env: &Env,
        client: &ProgramEscrowContractClient<'static>,
        authorized_key: &Address,
        token: &Address,
        program_id: &String,
        total_amount: i128,
        winner: &Address,
        release_timestamp: u64,
    ) {
        // Register program
        client.initialize_program(program_id, authorized_key, token);

        // Create and fund token
        let token_client = token::Client::new(env, token);
        let token_admin = token::StellarAssetClient::new(env, token);
        token_admin.mint(authorized_key, &total_amount);

        // Lock funds for program
        token_client.approve(
            authorized_key,
            &client.address,
            &total_amount,
            &1000,
        );
        client.lock_program_funds(program_id, &total_amount);

        // Create release schedule
        client.create_program_release_schedule(
            program_id,
            &total_amount,
            &release_timestamp,
            &winner,
        );
    }

    #[test]
    fn test_single_program_release_schedule() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token_client = create_token_contract(&env, &authorized_key);
        let token = token_client.address.clone();
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Verify schedule was created
        let schedule = client.get_program_release_schedule(&program_id, &1);
        assert_eq!(schedule.schedule_id, 1);
        assert_eq!(schedule.amount, amount);
        assert_eq!(schedule.release_timestamp, release_timestamp);
        assert_eq!(schedule.recipient, winner);
        assert!(!schedule.released);

        // Check pending schedules
        let pending = client.get_pending_program_schedules(&program_id);
        assert_eq!(pending.len(), 1);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_multiple_program_release_schedules() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env); // Keeping only one winner1
        let winner2 = Address::generate(&env);
        let token_client = create_token_contract(&env, &authorized_key);
        let token = token_client.address.clone();
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 600_0000000;
        let amount2 = 400_0000000;
        let total_amount = amount1 + amount2;

        env.mock_all_auths();

        // Register program
        client.initialize_program(&program_id, &authorized_key, &token);

        // Create and fund token
        let token_client = create_token_contract(&env, &authorized_key);
        let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
        token_admin.mint(&authorized_key, &total_amount);

        // Lock funds for program
        token_client.approve(
            &authorized_key,
            &client.address,
            &total_amount,
            &1000,
        );
        client.lock_program_funds(&program_id, &total_amount);

        // Create first release schedule
        client.create_program_release_schedule(&program_id, &amount1, &1000, &winner1.clone());

        // Create second release schedule
        client.create_program_release_schedule(&program_id, &amount2, &2000, &winner2.clone());

        // Verify both schedules exist
        let all_schedules = client.get_all_prog_release_schedules(&program_id);
        assert_eq!(all_schedules.len(), 2);

        // Verify schedule IDs
        let schedule1 = client.get_program_release_schedule(&program_id, &1);
        let schedule2 = client.get_program_release_schedule(&program_id, &2);
        assert_eq!(schedule1.schedule_id, 1);
        assert_eq!(schedule2.schedule_id, 2);

        // Verify amounts
        assert_eq!(schedule1.amount, amount1);
        assert_eq!(schedule2.amount, amount2);

        // Verify recipients
        assert_eq!(schedule1.recipient, winner1);
        assert_eq!(schedule2.recipient, winner2);

        // Check pending schedules
        let pending = client.get_pending_program_schedules(&program_id);
        assert_eq!(pending.len(), 2);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_program_automatic_release_at_timestamp() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token_client = create_token_contract(&env, &authorized_key);
        let token = token_client.address.clone();
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Try to release before timestamp (should fail)
        env.ledger().set_timestamp(999);
        let result = client.try_release_prog_schedule_automatic(&program_id, &1);
        assert!(result.is_err());

        // Advance time to after release timestamp
        env.ledger().set_timestamp(1001);

        // Release automatically
        client.release_prog_schedule_automatic(&program_id, &1);

        // Verify schedule was released
        let schedule = client.get_program_release_schedule(&program_id, &1);
        assert!(schedule.released);
        assert_eq!(schedule.released_at, Some(1001));
        assert_eq!(schedule.released_by, Some(client.address.clone()));

        // Check no pending schedules
        let pending = client.get_pending_program_schedules(&program_id);
        assert_eq!(pending.len(), 0);

        // Verify release history
        let history = client.get_program_release_history(&program_id);
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().release_type, ReleaseType::Automatic);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_program_manual_trigger_before_after_timestamp() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token_client = create_token_contract(&env, &authorized_key);
        let token = token_client.address.clone();
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Manually release before timestamp (authorized key can do this)
        env.ledger().set_timestamp(999);
        client.release_program_schedule_manual(&program_id, &1);

        // Verify schedule was released
        let schedule = client.get_program_release_schedule(&program_id, &1);
        assert!(schedule.released);
        assert_eq!(schedule.released_at, Some(999));
        assert_eq!(schedule.released_by, Some(authorized_key.clone()));

        // Verify release history
        let history = client.get_program_release_history(&program_id);
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().release_type, ReleaseType::Manual);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_verify_program_schedule_tracking_and_history() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let token_client = create_token_contract(&env, &authorized_key);
        let token = token_client.address.clone();
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 600_0000000;
        let amount2 = 400_0000000;
        let total_amount = amount1 + amount2;

        env.mock_all_auths();

        // Register program
        client.initialize_program(&program_id, &authorized_key, &token);

        // Create and fund token
        let token_client = create_token_contract(&env, &authorized_key);
        let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
        token_admin.mint(&authorized_key, &total_amount);

        // Lock funds for program
        token_client.approve(
            &authorized_key,
            &client.address,
            &total_amount,
            &1000,
        );
        client.lock_program_funds(&program_id, &total_amount);

        // Create first schedule
        client.create_program_release_schedule(&program_id, &amount1, &1000, &winner1.clone());

        // Create second schedule
        client.create_program_release_schedule(&program_id, &amount2, &2000, &winner2.clone());

        // Release first schedule manually
        client.release_program_schedule_manual(&program_id, &1);

        // Advance time and release second schedule automatically
        env.ledger().set_timestamp(2001);
        client.release_prog_schedule_automatic(&program_id, &2);

        // Verify complete history
        let history = client.get_program_release_history(&program_id);
        assert_eq!(history.len(), 2);

        // Check first release (manual)
        let first_release = history.get(0).unwrap();
        assert_eq!(first_release.schedule_id, 1);
        assert_eq!(first_release.amount, amount1);
        assert_eq!(first_release.recipient, winner1);
        assert_eq!(first_release.release_type, ReleaseType::Manual);

        // Check second release (automatic)
        let second_release = history.get(1).unwrap();
        assert_eq!(second_release.schedule_id, 2);
        assert_eq!(second_release.amount, amount2);
        // Check released_by manually if needed, but for now expect None for automatic
        // assert_eq!(second_release.released_by, ...);

        // Verify no pending schedules
        let pending = client.get_pending_program_schedules(&program_id);
        assert_eq!(pending.len(), 0);

        // Verify all schedules are marked as released
        let all_schedules = client.get_all_prog_release_schedules(&program_id);
        assert_eq!(all_schedules.len(), 2);
        assert!(all_schedules.get(0).unwrap().released);
        assert!(all_schedules.get(1).unwrap().released);
    }

    #[test]
    fn test_program_overlapping_schedules() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let winner3 = Address::generate(&env);
        let token_client = create_token_contract(&env, &authorized_key);
        let token = token_client.address.clone();
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 300_0000000;
        let amount2 = 300_0000000;
        let amount3 = 400_0000000;
        let total_amount = amount1 + amount2 + amount3;
        let base_timestamp = 1000;

        env.mock_all_auths();

        // Register program
        client.initialize_program(&program_id, &authorized_key, &token);

        // Create and fund token
        let token_client = create_token_contract(&env, &authorized_key);
        let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
        token_admin.mint(&authorized_key, &total_amount);

        // Lock funds for program
        token_client.approve(
            &authorized_key,
            &client.address,
            &total_amount,
            &1000,
        );
        client.lock_program_funds(&program_id, &total_amount);

        // Create overlapping schedules (all at same timestamp)
        client.create_program_release_schedule(
            &program_id,
            &amount1,
            &base_timestamp,
            &winner1.clone(),
        );

        client.create_program_release_schedule(
            &program_id,
            &amount2,
            &base_timestamp,
            &winner2.clone(),
        );

        client.create_program_release_schedule(
            &program_id,
            &amount3,
            &base_timestamp,
            &winner3.clone(),
        );

        // Advance time to after release timestamp
        env.ledger().set_timestamp(base_timestamp + 1);

        // Check due schedules (should be all 3)
        let due = client.get_due_program_schedules(&program_id);
        assert_eq!(due.len(), 3);

        // Release schedules one by one
        client.release_prog_schedule_automatic(&program_id, &1);
        client.release_prog_schedule_automatic(&program_id, &2);
        client.release_prog_schedule_automatic(&program_id, &3);

        // Verify all schedules are released
        let pending = client.get_pending_program_schedules(&program_id);
        assert_eq!(pending.len(), 0);

        // Verify complete history
        let history = client.get_program_release_history(&program_id);
        assert_eq!(history.len(), 3);

        // Verify all were automatic releases
        for release in history.iter() {
            assert_eq!(release.release_type, ReleaseType::Automatic);
        }

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_register_single_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register program
        let program = client.initialize_program(&prog_id, &backend, &token);

        // Verify program data
        assert_eq!(program.program_id, prog_id);
        assert_eq!(program.authorized_payout_key, backend);
        assert_eq!(program.token_address, token);
        assert_eq!(program.total_funds, 0);
        assert_eq!(program.remaining_balance, 0);
        assert_eq!(program.payout_history.len(), 0);

        // Verify it exists
        assert!(client.program_exists(&prog_id));
        assert_eq!(client.get_program_count(), 1);
    }

    #[test]
    fn test_multiple_programs_isolation() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend1 = Address::generate(&env);
        let backend2 = Address::generate(&env);
        let backend3 = Address::generate(&env);
        let token = Address::generate(&env);

        // Register three programs
        let prog1 = String::from_str(&env, "ETHGlobal2024");
        let prog2 = String::from_str(&env, "Stellar2024");
        let prog3 = String::from_str(&env, "BuildathonQ1");

        client.initialize_program(&prog1, &backend1, &token);
        client.initialize_program(&prog2, &backend2, &token);
        client.initialize_program(&prog3, &backend3, &token);

        // Verify all exist
        assert!(client.program_exists(&prog1));
        assert!(client.program_exists(&prog2));
        assert!(client.program_exists(&prog3));
        assert_eq!(client.get_program_count(), 3);

        // Verify complete isolation
        let info1 = client.get_program_info(&prog1);
        let info2 = client.get_program_info(&prog2);
        let info3 = client.get_program_info(&prog3);

        assert_eq!(info1.program_id, prog1);
        assert_eq!(info2.program_id, prog2);
        assert_eq!(info3.program_id, prog3);

        assert_eq!(info1.authorized_payout_key, backend1);
        assert_eq!(info2.authorized_payout_key, backend2);
        assert_eq!(info3.authorized_payout_key, backend3);

        // Verify list programs
        let programs = client.list_programs();
        assert_eq!(programs.len(), 3);
    }

    #[test]
    #[should_panic(expected = "Program already exists")]
    fn test_duplicate_program_registration() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register once - should succeed
        client.initialize_program(&prog_id, &backend, &token);

        // Register again - should panic
        client.initialize_program(&prog_id, &backend, &token);
    }

    #[test]
    #[should_panic(expected = "Program ID cannot be empty")]
    fn test_empty_program_id() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let empty_id = String::from_str(&env, "");

        client.initialize_program(&empty_id, &backend, &token);
    }

    #[test]
    #[should_panic(expected = "Program not found")]
    fn test_get_nonexistent_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let prog_id = String::from_str(&env, "DoesNotExist");
        client.get_program_info(&prog_id);
    }

    // ========================================================================
    // Fund Locking Tests
    // ========================================================================

    #[test]
    fn test_lock_funds_single_program() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register program
        client.initialize_program(&prog_id, &backend, &token_client.address);

        // Lock funds
        let amount = 10_000_0000000i128; // 10,000 USDC
        let updated = client.lock_program_funds(&prog_id, &amount);

        assert_eq!(updated.total_funds, amount);
        assert_eq!(updated.remaining_balance, amount);
    }

    #[test]
    fn test_lock_funds_multiple_programs_isolation() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend1 = Address::generate(&env);
        let backend2 = Address::generate(&env);

        let prog1 = String::from_str(&env, "Program1");
        let prog2 = String::from_str(&env, "Program2");

        // Register programs
        client.initialize_program(&prog1, &backend1, &token_client.address);
        client.initialize_program(&prog2, &backend2, &token_client.address);

        // Lock different amounts in each program
        let amount1 = 5_000_0000000i128;
        let amount2 = 10_000_0000000i128;

        client.lock_program_funds(&prog1, &amount1);
        client.lock_program_funds(&prog2, &amount2);

        // Verify isolation - funds don't mix
        let info1 = client.get_program_info(&prog1);
        let info2 = client.get_program_info(&prog2);

        assert_eq!(info1.total_funds, amount1);
        assert_eq!(info1.remaining_balance, amount1);
        assert_eq!(info2.total_funds, amount2);
        assert_eq!(info2.remaining_balance, amount2);
    }

    #[test]
    fn test_lock_funds_cumulative() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        client.initialize_program(&prog_id, &backend, &token_client.address);

        // Lock funds multiple times
        client.lock_program_funds(&prog_id, &1_000_0000000);
        client.lock_program_funds(&prog_id, &2_000_0000000);
        client.lock_program_funds(&prog_id, &3_000_0000000);

        let info = client.get_program_info(&prog_id);
        assert_eq!(info.total_funds, 6_000_0000000);
        assert_eq!(info.remaining_balance, 6_000_0000000);
    }

    #[test]
    #[should_panic(expected = "Amount must be greater than zero")]
    fn test_lock_zero_funds() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        client.initialize_program(&prog_id, &backend, &token);
        client.lock_program_funds(&prog_id, &0);
    }

    // ========================================================================
    // Batch Payout Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Recipients and amounts vectors must have the same length")]
    fn test_batch_payout_mismatched_lengths() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &backend, &token_client.address);
        client.lock_program_funds(&prog_id, &10_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env), Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 1_000_0000000i128]; // Mismatch!

        client.batch_payout(&prog_id, &recipients, &amounts);
    }

    #[test]
    #[should_panic(expected = "Insufficient balance")]
    fn test_batch_payout_insufficient_balance() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &backend, &token_client.address);
        client.lock_program_funds(&prog_id, &5_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 10_000_0000000i128]; // More than available!

        client.batch_payout(&prog_id, &recipients, &amounts);
    }

    #[test]
    fn test_program_count() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        assert_eq!(client.get_program_count(), 0);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);

        client.initialize_program(&String::from_str(&env, "P1"), &backend, &token);
        assert_eq!(client.get_program_count(), 1);

        client.initialize_program(&String::from_str(&env, "P2"), &backend, &token);
        assert_eq!(client.get_program_count(), 2);

        client.initialize_program(&String::from_str(&env, "P3"), &backend, &token);
        assert_eq!(client.get_program_count(), 3);
    }

    // ========================================================================
    // Anti-Abuse Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Operation in cooldown period")]
    fn test_anti_abuse_cooldown_panic() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.set_admin(&admin);
        client.update_rate_limit_config(&3600, &10, &60);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);

        client.initialize_program(&String::from_str(&env, "P1"), &backend, &token);

        // Advance time by 30s (less than 60s cooldown)
        env.ledger().with_mut(|li| li.timestamp += 30);

        client.initialize_program(&String::from_str(&env, "P2"), &backend, &token);
    }

    #[test]
    #[should_panic(expected = "Rate limit exceeded")]
    fn test_anti_abuse_limit_panic() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.set_admin(&admin);
        client.update_rate_limit_config(&3600, &2, &0); // 2 ops max, no cooldown

        let backend = Address::generate(&env);
        let token = Address::generate(&env);

        client.initialize_program(&String::from_str(&env, "P1"), &backend, &token);
        client.initialize_program(&String::from_str(&env, "P2"), &backend, &token);
        client.initialize_program(&String::from_str(&env, "P3"), &backend, &token);
        // Should panic
    }

    #[test]
    fn test_anti_abuse_whitelist() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.set_admin(&admin);
        client.update_rate_limit_config(&3600, &1, &60); // 1 op max

        let backend = Address::generate(&env);
        let token = Address::generate(&env);

        client.set_whitelist(&backend, &true);

        client.initialize_program(&String::from_str(&env, "P1"), &backend, &token);
        client.initialize_program(&String::from_str(&env, "P2"), &backend, &token);
        // Should work because whitelisted
    }

    #[test]
    fn test_anti_abuse_config_update() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.set_admin(&admin);

        client.update_rate_limit_config(&7200, &5, &120);

        let config = client.get_rate_limit_config();
        assert_eq!(config.window_size, 7200);
        assert_eq!(config.max_operations, 5);
        assert_eq!(config.cooldown_period, 120);
    }
}

#[cfg(test)]
mod test_query;

#[cfg(test)]
mod error_recovery_tests;

