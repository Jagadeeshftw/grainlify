#![no_std]
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
const PROGRAM_INITIALIZED: Symbol = symbol_short!("ProgInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FundLock");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
const PAYOUT: Symbol = symbol_short!("Payout");
const DEPENDENCY_CREATED: Symbol = symbol_short!("dep_add");
const DEPENDENCY_CLEARED: Symbol = symbol_short!("dep_clr");
const DEPENDENCY_STATUS_UPDATED: Symbol = symbol_short!("dep_sts");

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
    pub lock_fee_rate: i128,      // Fee rate for lock operations (basis points)
    pub payout_fee_rate: i128,     // Fee rate for payout operations (basis points)
    pub fee_recipient: Address,    // Address to receive fees
    pub fee_enabled: bool,         // Global fee enable/disable flag
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
// ── Step 1: Add module declarations near the top of lib.rs ──────────────
// (after `mod anti_abuse;` and before the contract struct)

mod claim_period;
pub mod token_math;
pub use claim_period::{ClaimRecord, ClaimStatus};
mod error_recovery;
mod reentrancy_guard;
#[cfg(test)]
mod test_claim_period_expiry_cancellation;
#[cfg(test)]
mod test_token_math;

#[cfg(test)]
mod test_circuit_breaker_audit;

#[cfg(test)]
mod error_recovery_tests;

#[cfg(any())]
mod reentrancy_tests;
#[cfg(test)]
mod test_dispute_resolution;

#[cfg(test)]
mod reentrancy_guard_standalone_test;

#[cfg(test)]
mod malicious_reentrant;

#[cfg(test)]
#[cfg(any())]
mod test_granular_pause;

#[cfg(test)]
mod test_lifecycle;

#[cfg(test)]
mod test_full_lifecycle;

// ── Step 2: Add these public contract functions to the ProgramEscrowContract
//    impl block (alongside the existing admin functions) ──────────────────

// ========================================================================
// Circuit Breaker Management
// ========================================================================

/// Register the circuit breaker admin. Can only be set once, or changed
/// by the existing admin.
///
/// # Arguments
/// * `new_admin` - Address to register as circuit breaker admin
/// * `caller`    - Existing admin (None if setting for the first time)
pub fn set_circuit_admin(env: Env, new_admin: Address, caller: Option<Address>) {
    error_recovery::set_circuit_admin(&env, new_admin, caller);
}

/// Returns the registered circuit breaker admin, if any.
pub fn get_circuit_admin(env: Env) -> Option<Address> {
    error_recovery::get_circuit_admin(&env)
}

/// Returns the full circuit breaker status snapshot.
///
/// # Returns
/// * `CircuitBreakerStatus` with state, failure/success counts, timestamps
pub fn get_circuit_status(env: Env) -> error_recovery::CircuitBreakerStatus {
    error_recovery::get_status(&env)
}

/// Admin resets the circuit breaker.
///
/// Transitions:
/// - Open     → HalfOpen  (probe mode)
/// - HalfOpen → Closed    (hard reset)
/// - Closed   → Closed    (no-op reset)
///
/// # Panics
/// * If caller is not the registered circuit breaker admin
pub fn reset_circuit_breaker(env: Env, admin: Address) {
    error_recovery::reset_circuit_breaker(&env, &admin);
}

/// Updates the circuit breaker configuration. Admin only.
///
/// # Arguments
/// * `failure_threshold` - Consecutive failures needed to open circuit
/// * `success_threshold` - Consecutive successes in HalfOpen to close it
/// * `max_error_log`     - Maximum error log entries to retain
pub fn configure_circuit_breaker(
    env: Env,
    admin: Address,
    failure_threshold: u32,
    success_threshold: u32,
    max_error_log: u32,
) {
    let stored = error_recovery::get_circuit_admin(&env);
    match stored {
        Some(ref a) if a == &admin => {
            admin.require_auth();
        }
        _ => panic!("Unauthorized: only circuit breaker admin can configure"),
    }
    error_recovery::set_config(
        &env,
        error_recovery::CircuitBreakerConfig {
            failure_threshold,
            success_threshold,
            max_error_log,
        },
    );
}

/// Returns the error log (last N failures recorded by the circuit breaker).
pub fn get_circuit_error_log(env: Env) -> soroban_sdk::Vec<error_recovery::ErrorEntry> {
    error_recovery::get_error_log(&env)
}

/// Directly open the circuit (emergency lockout). Admin only.
pub fn emergency_open_circuit(env: Env, admin: Address) {
    let stored = error_recovery::get_circuit_admin(&env);
    match stored {
        Some(ref a) if a == &admin => {
            admin.require_auth();
        }
        _ => panic!("Unauthorized"),
    }
    error_recovery::open_circuit(&env);
}

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    String, Symbol, Vec,
};

extern crate grainlify_core;
use grainlify_core::{asset, nonce};

// Event types
const PROGRAM_INITIALIZED: Symbol = symbol_short!("PrgInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FndsLock");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
const PAYOUT: Symbol = symbol_short!("Payout");
const EVENT_VERSION_V2: u32 = 2;
const PAUSE_STATE_CHANGED: Symbol = symbol_short!("PauseSt");
const PROGRAM_REGISTRY: Symbol = symbol_short!("ProgReg");
const PROGRAM_REGISTERED: Symbol = symbol_short!("ProgRgd");
const FEE_CONFIG: Symbol = symbol_short!("FeeCfg");
const BASIS_POINTS: i128 = token_math::BASIS_POINTS;

// Storage keys
const PROGRAM_DATA: Symbol = symbol_short!("ProgData");
const SCHEDULES: Symbol = symbol_short!("Scheds");
const RELEASE_HISTORY: Symbol = symbol_short!("RelHist");
const NEXT_SCHEDULE_ID: Symbol = symbol_short!("NxtSched");
const PROGRAM_INDEX: Symbol = symbol_short!("ProgIdx");
const AUTH_KEY_INDEX: Symbol = symbol_short!("AuthIdx");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,
    pub payout_fee_rate: i128,
    pub fee_recipient: Address,
    pub fee_enabled: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramInitializedEvent {
    pub version: u32,
    pub program_id: String,
    pub authorized_payout_key: Address,
    pub token_address: Address,
    pub total_funds: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsLockedEvent {
    pub version: u32,
    pub program_id: String,
    pub amount: i128,
    pub remaining_balance: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchPayoutEvent {
    pub version: u32,
    pub program_id: String,
    pub recipient_count: u32,
    pub total_amount: i128,
    pub remaining_balance: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutEvent {
    pub version: u32,
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub remaining_balance: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address,  // Token contract address for transfers
    pub initial_liquidity: i128, // Initial liquidity provided by creator
}

/// Storage key type for individual programs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String),                 // program_id -> ProgramData
    Admin,                           // Contract Admin
    ReleaseSchedule(String, u64),    // program_id, schedule_id -> ProgramReleaseSchedule
    ReleaseHistory(String),          // program_id -> Vec<ProgramReleaseHistory>
    NextScheduleId(String),          // program_id -> next schedule_id
    MultisigConfig(String),          // program_id -> MultisigConfig
    PayoutApproval(String, Address), // program_id, recipient -> PayoutApproval
    PendingClaim(String, u64),       // (program_id, schedule_id) -> ClaimRecord
    ClaimWindow,                     // u64 seconds (global config)
    PauseFlags,                      // PauseFlags struct
    RateLimitConfig,                 // RateLimitConfig struct
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseFlags {
    pub lock_paused: bool,
    pub release_paused: bool,
    pub refund_paused: bool,
    pub pause_reason: Option<String>,
    pub paused_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseStateChanged {
    pub operation: Symbol,
    pub paused: bool,
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitConfig {
    pub window_size: u64,
    pub max_operations: u32,
    pub cooldown_period: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Analytics {
    pub total_locked: i128,
    pub total_released: i128,
    pub total_payouts: u32,
    pub active_programs: u32,
    pub operation_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReleaseSchedule {
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub release_timestamp: u64,
    pub released: bool,
    pub released_at: Option<u64>,
    pub released_by: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseType {
    Manual,
    Automatic,
    Oracle,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReleaseHistory {
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub released_at: u64,
    pub release_type: ReleaseType,
}

/// Dependency resolution status for a program or external escrow identifier.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyStatus {
    Pending,
    Completed,
    Failed,
}


#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramAggregateStats {
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub total_paid_out: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address,
    pub payout_count: u32,
    pub scheduled_count: u32,
    pub released_count: u32,
}

/// Input item for batch program registration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String), // program_id -> ProgramData
    ReleaseSchedule(String, u64), // program_id, schedule_id -> ProgramReleaseSchedule
    ReleaseHistory(String), // program_id -> Vec<ProgramReleaseHistory>
    NextScheduleId(String), // program_id -> next schedule_id
    ProgramDependencies(String), // program_id -> Vec<dependency_id>
    DependencyStatus(String), // dependency_id -> DependencyStatus
}

fn vec_contains(values: &Vec<String>, target: &String) -> bool {
    for value in values.iter() {
        if value == *target {
            return true;
        }
    }
    false
}

fn get_program_dependencies_internal(env: &Env, program_id: &String) -> Vec<String> {
    env.storage()
        .instance()
        .get(&DataKey::ProgramDependencies(program_id.clone()))
        .unwrap_or(vec![env])
}

fn dependency_status_internal(env: &Env, dependency_id: &String) -> DependencyStatus {
    env.storage()
        .instance()
        .get(&DataKey::DependencyStatus(dependency_id.clone()))
        .unwrap_or(DependencyStatus::Pending)
}

fn path_exists_to_target(
    env: &Env,
    from_program: &String,
    target_program: &String,
    visited: &mut Vec<String>,
) -> bool {
    if *from_program == *target_program {
        return true;
    }
    if vec_contains(visited, from_program) {
        return false;
    }

    visited.push_back(from_program.clone());
    let deps = get_program_dependencies_internal(env, from_program);
    for dep in deps.iter() {
        if env.storage().instance().has(&DataKey::Program(dep.clone()))
            && path_exists_to_target(env, &dep, target_program, visited)
        {
            return true;
        }
    }

    false
pub struct ProgramInitItem {
    pub program_id: String,
    pub authorized_payout_key: Address,
    pub token_address: Address,
}

/// Maximum number of programs per batch (aligned with bounty_escrow).
pub const MAX_BATCH_SIZE: u32 = 100;

/// Errors for batch program registration.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BatchError {
    InvalidBatchSize = 1,
    ProgramAlreadyExists = 2,
    DuplicateProgramId = 3,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigConfig {
    pub threshold_amount: i128,
    pub signers: Vec<Address>,
    pub required_signatures: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutApproval {
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub approvals: Vec<Address>,
}

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    /// Initialize a new program escrow
    ///
    /// # Arguments
    /// * `program_id` - Unique identifier for the program/hackathon
    /// * `authorized_payout_key` - Address authorized to trigger payouts (backend)
    /// * `token_address` - Address of the token contract to use for transfers
    ///
    /// # Returns
    /// The initialized ProgramData
    pub fn init_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
        creator: Address,
        initial_liquidity: Option<i128>,
    ) -> ProgramData {
        Self::initialize_program(
            env,
            program_id,
            authorized_payout_key,
            token_address,
            creator,
            initial_liquidity,
        )
    }

    pub fn initialize_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
        creator: Address,
        initial_liquidity: Option<i128>,
    ) -> ProgramData {
        // Check if program already exists
        if env.storage().instance().has(&PROGRAM_DATA) {
            panic!("Program already initialized");
        }

        let mut total_funds = 0i128;
        let mut remaining_balance = 0i128;
        let mut init_liquidity = 0i128;

        if let Some(amount) = initial_liquidity {
            if amount > 0 {
                // Transfer initial liquidity from creator to contract
                let contract_address = env.current_contract_address();
                let token_client = token::Client::new(&env, &token_address);
                creator.require_auth();
                token_client.transfer(&creator, &contract_address, &amount);
                total_funds = amount;
                remaining_balance = amount;
                init_liquidity = amount;
            }
        }

        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds,
            remaining_balance,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
            token_address: token_address.clone(),
            initial_liquidity: init_liquidity,
        };

        // Store program data
        env.storage().instance().set(&program_key, &program_data);
        let empty_dependencies: Vec<String> = vec![&env];
        env.storage()
            .instance()
            .set(&DataKey::ProgramDependencies(program_id.clone()), &empty_dependencies);
        env.storage().instance().set(
            &DataKey::DependencyStatus(program_id.clone()),
            &DependencyStatus::Pending,
        );
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        env.storage()
            .instance()
            .set(&SCHEDULES, &Vec::<ProgramReleaseSchedule>::new(&env));
        env.storage()
            .instance()
            .set(&RELEASE_HISTORY, &Vec::<ProgramReleaseHistory>::new(&env));
        env.storage().instance().set(&NEXT_SCHEDULE_ID, &1_u64);

        // Emit ProgramInitialized event
        env.events().publish(
            (PROGRAM_INITIALIZED,),
            ProgramInitializedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                authorized_payout_key,
                token_address,
                total_funds,
            },
        );

        program_data
    }

    /// Batch-initialize multiple programs in one transaction (all-or-nothing).
    pub fn batch_initialize_programs(
        env: Env,
        items: Vec<ProgramInitItem>,
    ) -> Result<u32, BatchError> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(BatchError::InvalidBatchSize);
        }
        for i in 0..batch_size {
            for j in (i + 1)..batch_size {
                if items.get(i).unwrap().program_id == items.get(j).unwrap().program_id {
                    return Err(BatchError::DuplicateProgramId);
                }
            }
        }
        for i in 0..batch_size {
            let program_key = DataKey::Program(items.get(i).unwrap().program_id.clone());
            if env.storage().instance().has(&program_key) {
                return Err(BatchError::ProgramAlreadyExists);
            }
        }

        let mut registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);

        for i in 0..batch_size {
            let item = items.get(i).unwrap();
            let program_id = item.program_id.clone();
            let authorized_payout_key = item.authorized_payout_key.clone();
            let token_address = item.token_address.clone();

            if program_id.is_empty() {
                return Err(BatchError::InvalidBatchSize);
            }

            let program_data = ProgramData {
                program_id: program_id.clone(),
                total_funds: 0,
                remaining_balance: 0,
                authorized_payout_key: authorized_payout_key.clone(),
                payout_history: vec![&env],
                token_address: token_address.clone(),
                initial_liquidity: 0,
            };
            let program_key = DataKey::Program(program_id.clone());
            env.storage().instance().set(&program_key, &program_data);

            if i == 0 {
                let fee_config = FeeConfig {
                    lock_fee_rate: 0,
                    payout_fee_rate: 0,
                    fee_recipient: authorized_payout_key.clone(),
                    fee_enabled: false,
                };
                env.storage().instance().set(&FEE_CONFIG, &fee_config);
            }

            let multisig_config = MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![&env],
                required_signatures: 0,
            };
            env.storage().persistent().set(
                &DataKey::MultisigConfig(program_id.clone()),
                &multisig_config,
            );

            registry.push_back(program_id.clone());
            env.events().publish(
                (PROGRAM_REGISTERED,),
                (program_id, authorized_payout_key, token_address, 0i128),
            );
        }
        env.storage().instance().set(&PROGRAM_REGISTRY, &registry);

        Ok(batch_size as u32)
    }

    /// Calculate fee using floor rounding. Delegates to `token_math::calculate_fee`.
    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        token_math::calculate_fee(amount, fee_rate)
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
    /// Check if a program exists (legacy single-program check)
    ///
    /// # Returns
    /// * `bool` - True if program exists, false otherwise
    pub fn program_exists(env: Env) -> bool {
        env.storage().instance().has(&PROGRAM_DATA)
            || env.storage().instance().has(&PROGRAM_REGISTRY)
    }

    /// Check if a program exists by its program_id (for batch-registered programs).
    pub fn program_exists_by_id(env: Env, program_id: String) -> bool {
        env.storage().instance().has(&DataKey::Program(program_id))
    }

    fn assert_dependencies_satisfied(env: &Env, program_id: &String) {
        let dependencies = get_program_dependencies_internal(env, program_id);
        for dependency_id in dependencies.iter() {
            match dependency_status_internal(env, &dependency_id) {
                DependencyStatus::Completed => {}
                DependencyStatus::Pending => panic!("Dependency not satisfied"),
                DependencyStatus::Failed => panic!("Dependency failed"),
            }
        }
    }

    /// Defines explicit dependencies for a program.
    ///
    /// Dependencies can point to:
    /// - another registered program id; or
    /// - an externally managed dependency id with a pre-registered status.
    ///
    /// Cycle checks are applied for program-to-program edges.
    pub fn set_program_dependencies(
        env: Env,
        program_id: String,
        dependency_ids: Vec<String>,
    ) -> Vec<String> {
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.authorized_payout_key.require_auth();

        let old_dependencies = get_program_dependencies_internal(&env, &program_id);
        let mut validated_dependencies = vec![&env];

        for dependency_id in dependency_ids.iter() {
            if dependency_id.len() == 0 {
                panic!("Dependency id cannot be empty");
            }
            if dependency_id == program_id {
                panic!("Program cannot depend on itself");
            }
            if vec_contains(&validated_dependencies, &dependency_id) {
                panic!("Duplicate dependency");
            }

            let is_program_dependency = env
                .storage()
                .instance()
                .has(&DataKey::Program(dependency_id.clone()));
            let is_registered_external = env
                .storage()
                .instance()
                .has(&DataKey::DependencyStatus(dependency_id.clone()));
            if !is_program_dependency && !is_registered_external {
                panic!("Dependency not registered");
            }

            if is_program_dependency {
                let mut visited = Vec::new(&env);
                if path_exists_to_target(&env, &dependency_id, &program_id, &mut visited) {
                    panic!("Dependency cycle detected");
                }
            }

            validated_dependencies.push_back(dependency_id.clone());
        }

        env.storage().instance().set(
            &DataKey::ProgramDependencies(program_id.clone()),
            &validated_dependencies,
        );

        for dependency_id in validated_dependencies.iter() {
            if !vec_contains(&old_dependencies, &dependency_id) {
                env.events().publish(
                    (DEPENDENCY_CREATED,),
                    (program_id.clone(), dependency_id.clone()),
                );
            }
        }
        for dependency_id in old_dependencies.iter() {
            if !vec_contains(&validated_dependencies, &dependency_id) {
                env.events().publish(
                    (DEPENDENCY_CLEARED,),
                    (program_id.clone(), dependency_id.clone()),
                );
            }
        }

        validated_dependencies
    }

    /// Clears all dependencies for a program.
    pub fn clear_program_dependencies(env: Env, program_id: String) {
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.authorized_payout_key.require_auth();

        let old_dependencies = get_program_dependencies_internal(&env, &program_id);
        let empty_dependencies: Vec<String> = vec![&env];
        env.storage()
            .instance()
            .set(&DataKey::ProgramDependencies(program_id.clone()), &empty_dependencies);

        for dependency_id in old_dependencies.iter() {
            env.events().publish(
                (DEPENDENCY_CLEARED,),
                (program_id.clone(), dependency_id.clone()),
            );
        }
    }

    /// Reads all dependencies configured for a program.
    pub fn get_program_dependencies(env: Env, program_id: String) -> Vec<String> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Program(program_id.clone()))
        {
            panic!("Program not found");
        }
        get_program_dependencies_internal(&env, &program_id)
    }

    /// Updates dependency status.
    ///
    /// For registered programs, only that program's authorized payout key can update status.
    /// For external dependency ids, anti-abuse admin authorization is required.
    pub fn set_dependency_status(env: Env, dependency_id: String, status: DependencyStatus) {
        if dependency_id.len() == 0 {
            panic!("Dependency id cannot be empty");
        }

        if env
            .storage()
            .instance()
            .has(&DataKey::Program(dependency_id.clone()))
        {
            let program_data: ProgramData = env
                .storage()
                .instance()
                .get(&DataKey::Program(dependency_id.clone()))
                .unwrap();
            program_data.authorized_payout_key.require_auth();
        } else {
            let admin = anti_abuse::get_admin(&env)
                .unwrap_or_else(|| panic!("Admin not set for external dependency status update"));
            admin.require_auth();
        }

        env.storage()
            .instance()
            .set(&DataKey::DependencyStatus(dependency_id.clone()), &status.clone());
        env.events()
            .publish((DEPENDENCY_STATUS_UPDATED,), (dependency_id, status));
    }

    /// Reads dependency status; defaults to pending if no explicit status exists.
    pub fn get_dependency_status(env: Env, dependency_id: String) -> DependencyStatus {
        dependency_status_internal(&env, &dependency_id)
    }

    // ========================================================================
    // Fund Management
    // ========================================================================

    /// Lock initial funds into the program escrow
    ///
    /// # Arguments
    /// * `amount` - Amount of funds to lock (in native token units)
    ///
    /// # Returns
    /// * `ProgramData` - Updated program data with new balance

    pub fn lock_program_funds(env: Env, program_id: String, amount: i128) -> ProgramData {
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, env.current_contract_address());

        let caller = env.current_contract_address();
    /// Updated ProgramData with locked funds
    pub fn lock_program_funds(env: Env, amount: i128) -> ProgramData {
        if Self::check_paused(&env, symbol_short!("lock")) {
            panic!("Funds Paused");
        }

        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Update balances
        program_data.total_funds += amount;
        program_data.remaining_balance += amount;

        // Store updated data
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        // Emit FundsLocked event
        env.events().publish(
            (FUNDS_LOCKED,),
            FundsLockedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                amount,
                remaining_balance: program_data.remaining_balance,
            },
        );

        program_data
    }

    // ========================================================================
    // Initialization & Admin
    // ========================================================================

    /// Initialize the contract with an admin.
    /// This must be called before any admin protected functions (like pause) can be used.
    pub fn initialize_contract(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Set or rotate admin. If no admin is set, sets initial admin. If admin exists, current admin must authorize and the new address becomes admin.
    pub fn set_admin(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            let current: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
            current.require_auth();
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Returns the current admin address, if set.
    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    pub fn get_program_release_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        env.storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Update pause flags (admin only)
    pub fn set_paused(
        env: Env,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
        reason: Option<String>,
    ) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut flags = Self::get_pause_flags(&env);
        let timestamp = env.ledger().timestamp();

        if reason.is_some() {
            flags.pause_reason = reason.clone();
        }

        if let Some(paused) = lock {
            flags.lock_paused = paused;
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                (
                    symbol_short!("lock"),
                    paused,
                    admin.clone(),
                    reason.clone(),
                    timestamp,
                ),
            );
        }

        if let Some(paused) = release {
            flags.release_paused = paused;
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                (
                    symbol_short!("release"),
                    paused,
                    admin.clone(),
                    reason.clone(),
                    timestamp,
                ),
            );
        }

        if let Some(paused) = refund {
            flags.refund_paused = paused;
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                (
                    symbol_short!("refund"),
                    paused,
                    admin.clone(),
                    reason.clone(),
                    timestamp,
                ),
            );
        }

        let any_paused = flags.lock_paused || flags.release_paused || flags.refund_paused;

        if any_paused {
            if flags.paused_at == 0 {
                flags.paused_at = timestamp;
            }
        } else {
            flags.pause_reason = None;
            flags.paused_at = 0;
        }

        env.storage().instance().set(&DataKey::PauseFlags, &flags);
    }

    /// Emergency withdraw all program funds (admin only, must have lock_paused = true)
    pub fn emergency_withdraw(env: Env, target: Address) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let flags = Self::get_pause_flags(&env);
        if !flags.lock_paused {
            panic!("Not paused");
        }

        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let token_client = token::TokenClient::new(&env, &program_data.token_address);

        let contract_address = env.current_contract_address();
        let balance = token_client.balance(&contract_address);

        if balance > 0 {
            token_client.transfer(&contract_address, &target, &balance);
            env.events().publish(
                (symbol_short!("em_wtd"),),
                (admin, target.clone(), balance, env.ledger().timestamp()),
            );
        }
    }

    /// Get current pause flags
    pub fn get_pause_flags(env: &Env) -> PauseFlags {
        env.storage()
            .instance()
            .get(&DataKey::PauseFlags)
            .unwrap_or(PauseFlags {
                lock_paused: false,
                release_paused: false,
                refund_paused: false,
                pause_reason: None,
                paused_at: 0,
            })
    }

    /// Check if an operation is paused
    fn check_paused(env: &Env, operation: Symbol) -> bool {
        let flags = Self::get_pause_flags(env);
        if operation == symbol_short!("lock") {
            return flags.lock_paused;
        } else if operation == symbol_short!("release") {
            return flags.release_paused;
        } else if operation == symbol_short!("refund") {
            return flags.refund_paused;
        }
        false
    }

    // --- Circuit Breaker & Rate Limit ---

    pub fn set_circuit_admin(env: Env, new_admin: Address, caller: Option<Address>) {
        error_recovery::set_circuit_admin(&env, new_admin, caller);
    }

    pub fn get_circuit_admin(env: Env) -> Option<Address> {
        error_recovery::get_circuit_admin(&env)
    }

    pub fn reset_circuit_breaker(env: Env, caller: Address) {
        caller.require_auth();
        let admin = error_recovery::get_circuit_admin(&env).expect("Circuit admin not set");
        if caller != admin {
            panic!("Unauthorized: only circuit admin can reset");
        }
        error_recovery::reset_circuit_breaker(&env, &admin);
    }

    pub fn configure_circuit_breaker(
        env: Env,
        caller: Address,
        _threshold: u32,
        _lookback: u32,
        _cooldown: u32,
    ) {
        caller.require_auth();
        let admin = error_recovery::get_circuit_admin(&env).expect("Circuit admin not set");
        if caller != admin {
            panic!("Unauthorized: only circuit admin can configure");
        }
        // Logic to update config in storage would go here
    }

    pub fn update_rate_limit_config(
        env: Env,
        window_size: u64,
        max_operations: u32,
        cooldown_period: u64,
    ) {
        // Only admin can update rate limit config
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let config = RateLimitConfig {
            window_size,
            max_operations,
            cooldown_period,
        };
        env.storage()
            .instance()
            .set(&DataKey::RateLimitConfig, &config);
    }

    pub fn get_rate_limit_config(env: Env) -> RateLimitConfig {
        env.storage()
            .instance()
            .get(&DataKey::RateLimitConfig)
            .unwrap_or(RateLimitConfig {
                window_size: 3600,
                max_operations: 10,
                cooldown_period: 60,
            })
    }

    pub fn get_analytics(_env: Env) -> Analytics {
        Analytics {
            total_locked: 0,
            total_released: 0,
            total_payouts: 0,
            active_programs: 0,
            operation_count: 0,
        }
    }

    pub fn set_whitelist(env: Env, _address: Address, _whitelisted: bool) {
        // Only admin can set whitelist
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Not initialized"));
        admin.require_auth();
    }
    // ========================================================================
    // Payout Functions
    // ========================================================================

    /// Execute batch payouts to multiple recipients
    ///
    /// # Arguments
    /// * `recipients` - Vector of recipient addresses
    /// * `amounts` - Vector of amounts (must match recipients length)
    ///
    /// # Returns
    /// Updated ProgramData after payouts
    pub fn batch_payout(env: Env, recipients: Vec<Address>, amounts: Vec<i128>) -> ProgramData {
        // Reentrancy guard: Check and set
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        Self::assert_dependencies_satisfied(&env, &program_id);

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());
        // Verify authorization
        let program_data: ProgramData =
            env.storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Program not initialized")
                });

        program_data.authorized_payout_key.require_auth();

        // Validate input lengths match
        if recipients.len() != amounts.len() {
            reentrancy_guard::clear_entered(&env);
            panic!("Recipients and amounts vectors must have the same length");
        }

        if recipients.len() == 0 {
            reentrancy_guard::clear_entered(&env);
            panic!("Cannot process empty batch");
        }

        // Calculate total payout amount
        let mut total_payout: i128 = 0;
        for amount in amounts.iter() {
            if amount <= 0 {
                reentrancy_guard::clear_entered(&env);
                panic!("All amounts must be greater than zero");
            }
            total_payout = total_payout.checked_add(amount).unwrap_or_else(|| {
                reentrancy_guard::clear_entered(&env);
                panic!("Payout amount overflow")
            });
        }

        // Validate sufficient balance
        if total_payout > program_data.remaining_balance {
            reentrancy_guard::clear_entered(&env);
            panic!("Insufficient balance");
        }

        // Execute transfers
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        for i in 0..recipients.len() {
            let recipient = recipients.get(i).unwrap();
            let amount = amounts.get(i).unwrap();

            // Transfer funds from contract to recipient
            token_client.transfer(&contract_address, &recipient, &amount);

            // Record payout
            let payout_record = PayoutRecord {
                recipient,
                amount,
                timestamp,
            };
            updated_history.push_back(payout_record);
        }

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= total_payout;
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&PROGRAM_DATA, &updated_data);

        // Emit BatchPayout event
        env.events().publish(
            (BATCH_PAYOUT,),
            BatchPayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: updated_data.program_id.clone(),
                recipient_count: recipients.len() as u32,
                total_amount: total_payout,
                remaining_balance: updated_data.remaining_balance,
            },
        );

        // Clear reentrancy guard before returning
        reentrancy_guard::clear_entered(&env);

        updated_data
    }

    /// Execute a single payout to one recipient
    ///
    /// # Arguments
    /// * `recipient` - Address of the recipient
    /// * `amount` - Amount to transfer
    ///
    pub fn single_payout(
        env: Env,
        program_id: String,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        Self::assert_dependencies_satisfied(&env, &program_id);

        program_data.authorized_payout_key.require_auth();
        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());
    pub fn single_payout(env: Env, recipient: Address, amount: i128) -> ProgramData {
        // Reentrancy guard: Check and set
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        // Verify authorization
        let program_data: ProgramData =
            env.storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Program not initialized")
                });

        program_data.authorized_payout_key.require_auth();

        // Validate amount
        if amount <= 0 {
            reentrancy_guard::clear_entered(&env);
            panic!("Amount must be greater than zero");
        }

        // Validate sufficient balance
        if amount > program_data.remaining_balance {
            reentrancy_guard::clear_entered(&env);
            panic!("Insufficient balance");
        }

        // Transfer funds from contract to recipient
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&contract_address, &recipient, &amount);

        // Record payout
        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutRecord {
            recipient: recipient.clone(),
            amount,
            timestamp,
        };

        let mut updated_history = program_data.payout_history.clone();
        updated_history.push_back(payout_record);

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= amount;
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&PROGRAM_DATA, &updated_data);

        // Emit Payout event
        env.events().publish(
            (PAYOUT,),
            PayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: updated_data.program_id.clone(),
                recipient,
                amount,
                remaining_balance: updated_data.remaining_balance,
            },
        );

        // Clear reentrancy guard before returning
        reentrancy_guard::clear_entered(&env);

        updated_data
    }

    /// Get program information
    ///
    /// # Returns
    /// ProgramData containing all program information
    pub fn get_program_info(env: Env) -> ProgramData {
        env.storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"))
    }

    /// Get remaining balance
    ///
    /// # Returns
    /// Current remaining balance
    pub fn get_remaining_balance(env: Env) -> i128 {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.remaining_balance
    }

    /// Create a release schedule entry that can be triggered at/after `release_timestamp`.
    pub fn create_program_release_schedule(
        env: Env,
        recipient: Address,
        amount: i128,
        release_timestamp: u64,
    ) -> ProgramReleaseSchedule {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.authorized_payout_key.require_auth();

        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let schedule_id: u64 = env
            .storage()
            .instance()
            .get(&NEXT_SCHEDULE_ID)
            .unwrap_or(1_u64);

        let schedule = ProgramReleaseSchedule {
            schedule_id,
            recipient,
            amount,
            release_timestamp,
            released: false,
            released_at: None,
            released_by: None,
        };
        schedules.push_back(schedule.clone());

        env.storage().instance().set(&SCHEDULES, &schedules);
        env.storage()
            .instance()
            .set(&NEXT_SCHEDULE_ID, &(schedule_id + 1));

        schedule
    }

    /// Trigger all due schedules where `now >= release_timestamp`.
    pub fn trigger_program_releases(env: Env) -> u32 {
        // Reentrancy guard: Check and set
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| {
                reentrancy_guard::clear_entered(&env);
                panic!("Program not initialized")
            });
        program_data.authorized_payout_key.require_auth();

        let mut schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        Self::assert_dependencies_satisfied(&env, &program_id);

        // Get schedule
        if !env
            .storage()
            .persistent()
            .has(&DataKey::ReleaseSchedule(program_id.clone(), schedule_id))
        {
            panic!("Schedule not found");
        }

        let mut schedule: ProgramReleaseSchedule = env
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut release_history: Vec<ProgramReleaseHistory> = env
            .storage()
            .instance()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env));

        let now = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        let mut released_count: u32 = 0;

        for i in 0..schedules.len() {
            let mut schedule = schedules.get(i).unwrap();
            if schedule.released || now < schedule.release_timestamp {
                continue;
            }

            if schedule.amount > program_data.remaining_balance {
                reentrancy_guard::clear_entered(&env);
                panic!("Insufficient balance");
            }

            token_client.transfer(&contract_address, &schedule.recipient, &schedule.amount);
            schedule.released = true;
            schedule.released_at = Some(now);
            schedule.released_by = Some(contract_address.clone());
            schedules.set(i, schedule.clone());

            program_data.remaining_balance -= schedule.amount;
            program_data.payout_history.push_back(PayoutRecord {
                recipient: schedule.recipient.clone(),
                amount: schedule.amount,
                timestamp: now,
            });
            release_history.push_back(ProgramReleaseHistory {
                schedule_id: schedule.schedule_id,
                recipient: schedule.recipient,
                amount: schedule.amount,
                released_at: now,
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
    /// Can be called before the release timestamp by authorized key
    pub fn release_program_schedule_manual(
        env: Env,
        program_id: String,
        schedule_id: u64,
    ) {
        let start = env.ledger().timestamp();

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        Self::assert_dependencies_satisfied(&env, &program_id);

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
            });
            released_count += 1;
        }

        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        env.storage().instance().set(&SCHEDULES, &schedules);
        env.storage()
            .instance()
            .set(&RELEASE_HISTORY, &release_history);

        // Clear reentrancy guard before returning
        reentrancy_guard::clear_entered(&env);

        released_count
    }

    pub fn get_release_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        env.storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_program_release_history(env: Env) -> Vec<ProgramReleaseHistory> {
        env.storage()
            .instance()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env))
    }

    // ========================================================================
    // Multi-tenant / Multi-program Migration Wrappers (ignore id for now)
    // ========================================================================

    pub fn get_program_info_v2(env: Env, _program_id: String) -> ProgramData {
        Self::get_program_info(env)
    }

    pub fn lock_program_funds_v2(env: Env, _program_id: String, amount: i128) -> ProgramData {
        Self::lock_program_funds(env, amount)
    }

    pub fn single_payout_v2(
        env: Env,
        _program_id: String,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
        Self::single_payout(env, recipient, amount)
    }

    pub fn batch_payout_v2(
        env: Env,
        _program_id: String,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> ProgramData {
        Self::batch_payout(env, recipients, amounts)
    }

    /// Query payout history by recipient with pagination
    pub fn query_payouts_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<PayoutRecord> {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let history = program_data.payout_history;
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.recipient == recipient {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Query payout history by amount range
    pub fn query_payouts_by_amount(
        env: Env,
        min_amount: i128,
        max_amount: i128,
        offset: u32,
        limit: u32,
    ) -> Vec<PayoutRecord> {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let history = program_data.payout_history;
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.amount >= min_amount && record.amount <= max_amount {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Query payout history by timestamp range
    pub fn query_payouts_by_timestamp(
        env: Env,
        min_timestamp: u64,
        max_timestamp: u64,
        offset: u32,
        limit: u32,
    ) -> Vec<PayoutRecord> {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let history = program_data.payout_history;
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.timestamp >= min_timestamp && record.timestamp <= max_timestamp {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Query release schedules by recipient
    pub fn query_schedules_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<ProgramReleaseSchedule> {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..schedules.len() {
            if count >= limit {
                break;
            }
            let schedule = schedules.get(i).unwrap();
            if schedule.recipient == recipient {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(schedule);
                count += 1;
            }
        }
        results
    }

    /// Query release schedules by released status
    pub fn query_schedules_by_status(
        env: Env,
        released: bool,
        offset: u32,
        limit: u32,
    ) -> Vec<ProgramReleaseSchedule> {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..schedules.len() {
            if count >= limit {
                break;
            }
            let schedule = schedules.get(i).unwrap();
            if schedule.released == released {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(schedule);
                count += 1;
            }
        }
        results
    }

    /// Query release history with filtering and pagination
    pub fn query_releases_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<ProgramReleaseHistory> {
        let history: Vec<ProgramReleaseHistory> = env
            .storage()
            .instance()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env));
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.recipient == recipient {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Get aggregate statistics for the program
    pub fn get_program_aggregate_stats(env: Env) -> ProgramAggregateStats {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));

        let mut scheduled_count = 0u32;
        let mut released_count = 0u32;

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if schedule.released {
                released_count += 1;
            } else {
                scheduled_count += 1;
            }
        }

        ProgramAggregateStats {
            total_funds: program_data.total_funds,
            remaining_balance: program_data.remaining_balance,
            total_paid_out: program_data.total_funds - program_data.remaining_balance,
            authorized_payout_key: program_data.authorized_payout_key.clone(),
            payout_history: program_data.payout_history.clone(),
            token_address: program_data.token_address.clone(),
            payout_count: program_data.payout_history.len(),
            scheduled_count,
            released_count,
        }
    }

    /// Get payouts by recipient
    pub fn get_payouts_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<PayoutRecord> {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let history = program_data.payout_history;
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.recipient == recipient {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Get pending schedules (not yet released)
    pub fn get_pending_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut results = Vec::new(&env);

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if !schedule.released {
                results.push_back(schedule);
            }
        }
        results
    }

    /// Get due schedules (ready to be released)
    pub fn get_due_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let now = env.ledger().timestamp();
        let mut results = Vec::new(&env);

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if !schedule.released && schedule.release_timestamp <= now {
                results.push_back(schedule);
            }
        }
        results
    }

    /// Get total amount in pending schedules
    pub fn get_total_scheduled_amount(env: Env) -> i128 {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut total = 0i128;

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if !schedule.released {
                total += schedule.amount;
            }
        }
        total
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
    fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_address = token_contract.address();
        token::Client::new(env, &token_address)
    pub fn get_program_count(env: Env) -> u32 {
        if env.storage().instance().has(&PROGRAM_DATA) {
            1
        } else {
            0
        }
    }

    pub fn list_programs(env: Env) -> Vec<ProgramData> {
        let mut results = Vec::new(&env);
        if env.storage().instance().has(&PROGRAM_DATA) {
            results.push_back(Self::get_program_info(env.clone()));
        }
        results
    }

    pub fn get_program_release_schedule(env: Env, schedule_id: u64) -> ProgramReleaseSchedule {
        let schedules = Self::get_release_schedules(env);
        for s in schedules.iter() {
            if s.schedule_id == schedule_id {
                return s;
            }
        }
        panic!("Schedule not found");
    }

    pub fn get_all_prog_release_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        Self::get_release_schedules(env)
    }

    pub fn get_pending_program_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        Self::get_pending_schedules(env)
    }

    pub fn get_due_program_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        Self::get_due_schedules(env)
    }

    pub fn release_program_schedule_manual(env: Env, schedule_id: u64) {
        let mut schedules = Self::get_release_schedules(env.clone());
        let program_data = Self::get_program_info(env.clone());

        program_data.authorized_payout_key.require_auth();

        let caller = program_data.authorized_payout_key.clone();
        let now = env.ledger().timestamp();
        let mut released_schedule: Option<ProgramReleaseSchedule> = None;

        let mut found = false;
        for i in 0..schedules.len() {
            let mut s = schedules.get(i).unwrap();
            if s.schedule_id == schedule_id {
                if s.released {
                    panic!("Already released");
                }

                // Transfer funds
                let token_client = token::Client::new(&env, &program_data.token_address);
                token_client.transfer(&env.current_contract_address(), &s.recipient, &s.amount);

                s.released = true;
                s.released_at = Some(now);
                s.released_by = Some(caller.clone());
                released_schedule = Some(s.clone());
                schedules.set(i, s);
                found = true;
                break;
            }
        }

        if !found {
            panic!("Schedule not found");
        }

        env.storage().instance().set(&SCHEDULES, &schedules);

        // Write to release history
        if let Some(s) = released_schedule {
            let mut updated_program_data = program_data.clone();
            updated_program_data.remaining_balance -= s.amount;
            env.storage()
                .instance()
                .set(&PROGRAM_DATA, &updated_program_data);

            let mut history: Vec<ProgramReleaseHistory> = env
                .storage()
                .instance()
                .get(&RELEASE_HISTORY)
                .unwrap_or_else(|| Vec::new(&env));
            history.push_back(ProgramReleaseHistory {
                schedule_id: s.schedule_id,
                recipient: s.recipient,
                amount: s.amount,
                released_at: now,
                release_type: ReleaseType::Manual,
            });
            env.storage().instance().set(&RELEASE_HISTORY, &history);
        }
    }

    pub fn release_prog_schedule_automatic(env: Env, schedule_id: u64) {
        let mut schedules = Self::get_release_schedules(env.clone());
        let program_data = Self::get_program_info(env.clone());
        let now = env.ledger().timestamp();
        let mut released_schedule: Option<ProgramReleaseSchedule> = None;

        let mut found = false;
        for i in 0..schedules.len() {
            let mut s = schedules.get(i).unwrap();
            if s.schedule_id == schedule_id {
                if s.released {
                    panic!("Already released");
                }
                if now < s.release_timestamp {
                    panic!("Not yet due");
                }

                // Transfer funds
                let token_client = token::Client::new(&env, &program_data.token_address);
                token_client.transfer(&env.current_contract_address(), &s.recipient, &s.amount);

                s.released = true;
                s.released_at = Some(now);
                s.released_by = Some(env.current_contract_address());
                released_schedule = Some(s.clone());
                schedules.set(i, s);
                found = true;
                break;
            }
        }

        if !found {
            panic!("Schedule not found");
        }

        env.storage().instance().set(&SCHEDULES, &schedules);

    #[test]
    #[should_panic(expected = "Program not found")]
    fn test_get_nonexistent_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let prog_id = String::from_str(&env, "DoesNotExist");
        client.get_program_info(&prog_id);
    }

    #[test]
    fn test_dependency_gated_release_flow() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_client = create_token_contract(&env, &token_admin);
        let token_asset = token::StellarAssetClient::new(&env, &token_client.address);

        let dep_backend = Address::generate(&env);
        let target_backend = Address::generate(&env);
        let dependency_program = String::from_str(&env, "dependency-program");
        let target_program = String::from_str(&env, "target-program");
        let winner = Address::generate(&env);
        let amount = 10_000_000i128;

        token_asset.mint(&token_admin, &amount);
        token_client.transfer(&token_admin, &contract_id, &amount);

        client.initialize_program(&dependency_program, &dep_backend, &token_client.address);
        client.initialize_program(&target_program, &target_backend, &token_client.address);
        client.lock_program_funds(&target_program, &amount);
        client.create_program_release_schedule(&target_program, &amount, &1000, &winner);

        let dependencies = soroban_sdk::vec![&env, dependency_program.clone()];
        client.set_program_dependencies(&target_program, &dependencies);

        env.ledger().set_timestamp(1001);
        let blocked = client.try_release_prog_schedule_automatic(&target_program, &1);
        assert!(blocked.is_err());

        client.set_dependency_status(&dependency_program, &DependencyStatus::Completed);
        client.release_prog_schedule_automatic(&target_program, &1);

        let schedule = client.get_program_release_schedule(&target_program, &1);
        assert!(schedule.released);
    }

    #[test]
    #[should_panic(expected = "Dependency failed")]
    fn test_dependency_failed_blocks_release() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let token_client = create_token_contract(&env, &token_admin);
        let token_asset = token::StellarAssetClient::new(&env, &token_client.address);

        let dep_backend = Address::generate(&env);
        let target_backend = Address::generate(&env);
        let dependency_program = String::from_str(&env, "dependency-failed");
        let target_program = String::from_str(&env, "target-failed");
        let winner = Address::generate(&env);
        let amount = 5_000_000i128;

        token_asset.mint(&token_admin, &amount);
        token_client.transfer(&token_admin, &contract_id, &amount);

        client.initialize_program(&dependency_program, &dep_backend, &token_client.address);
        client.initialize_program(&target_program, &target_backend, &token_client.address);
        client.lock_program_funds(&target_program, &amount);
        client.create_program_release_schedule(&target_program, &amount, &1000, &winner);
        client.set_program_dependencies(
            &target_program,
            &soroban_sdk::vec![&env, dependency_program.clone()],
        );

        client.set_dependency_status(&dependency_program, &DependencyStatus::Failed);
        env.ledger().set_timestamp(1001);
        client.release_prog_schedule_automatic(&target_program, &1);
    }

    #[test]
    #[should_panic(expected = "Dependency cycle detected")]
    fn test_dependency_cycle_rejection() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend_a = Address::generate(&env);
        let backend_b = Address::generate(&env);
        let token = Address::generate(&env);
        let program_a = String::from_str(&env, "cycle-a");
        let program_b = String::from_str(&env, "cycle-b");

        client.initialize_program(&program_a, &backend_a, &token);
        client.initialize_program(&program_b, &backend_b, &token);
        client.set_program_dependencies(&program_a, &soroban_sdk::vec![&env, program_b.clone()]);
        client.set_program_dependencies(&program_b, &soroban_sdk::vec![&env, program_a.clone()]);
    }

    #[test]
    fn test_dependency_events_created_and_cleared() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend_a = Address::generate(&env);
        let backend_b = Address::generate(&env);
        let token = Address::generate(&env);
        let program_a = String::from_str(&env, "event-a");
        let program_b = String::from_str(&env, "event-b");

        client.initialize_program(&program_a, &backend_a, &token);
        client.initialize_program(&program_b, &backend_b, &token);

        client.set_program_dependencies(&program_a, &soroban_sdk::vec![&env, program_b.clone()]);
        let dependencies = client.get_program_dependencies(&program_a);
        assert_eq!(dependencies.len(), 1);

        client.clear_program_dependencies(&program_a);
        let cleared_dependencies = client.get_program_dependencies(&program_a);
        assert_eq!(cleared_dependencies.len(), 0);
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
        // Write to release history
        if let Some(s) = released_schedule {
            let mut updated_program_data = program_data.clone();
            updated_program_data.remaining_balance -= s.amount;
            env.storage()
                .instance()
                .set(&PROGRAM_DATA, &updated_program_data);

            let mut history: Vec<ProgramReleaseHistory> = env
                .storage()
                .instance()
                .get(&RELEASE_HISTORY)
                .unwrap_or_else(|| Vec::new(&env));
            history.push_back(ProgramReleaseHistory {
                schedule_id: s.schedule_id,
                recipient: s.recipient,
                amount: s.amount,
                released_at: now,
                release_type: ReleaseType::Automatic,
            });
            env.storage().instance().set(&RELEASE_HISTORY, &history);
        }
    }

    pub fn create_pending_claim(
        env: Env,
        program_id: String,
        recipient: Address,
        amount: i128,
        claim_deadline: u64,
    ) -> u64 {
        claim_period::create_pending_claim(&env, &program_id, &recipient, amount, claim_deadline)
    }

    pub fn execute_claim(env: Env, program_id: String, claim_id: u64, recipient: Address) {
        claim_period::execute_claim(&env, &program_id, claim_id, &recipient)
    }

    pub fn cancel_claim(env: Env, program_id: String, claim_id: u64, admin: Address) {
        claim_period::cancel_claim(&env, &program_id, claim_id, &admin)
    }

    pub fn get_claim(env: Env, program_id: String, claim_id: u64) -> claim_period::ClaimRecord {
        claim_period::get_claim(&env, &program_id, claim_id)
    }

    pub fn set_claim_window(env: Env, admin: Address, window_seconds: u64) {
        claim_period::set_claim_window(&env, &admin, window_seconds)
    }

    pub fn get_claim_window(env: Env) -> u64 {
        claim_period::get_claim_window(&env)
    }
}

#[cfg(test)]
mod test;

#[cfg(test)]
mod test_pause;

#[cfg(test)]
#[cfg(any())]
mod rbac_tests;
