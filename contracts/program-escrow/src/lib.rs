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
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    String, Symbol, Vec,
};

// Event types
const PROGRAM_INITIALIZED: Symbol = symbol_short!("PrgInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FndsLock");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
const PAYOUT: Symbol = symbol_short!("Payout");
const EVENT_VERSION_V2: u32 = 2;
const PAUSE_STATE_CHANGED: Symbol = symbol_short!("PauseSt");
const MAINTENANCE_MODE_CHANGED: Symbol = symbol_short!("MaintSt");
const PROGRAM_RISK_FLAGS_UPDATED: Symbol = symbol_short!("pr_risk");
const PROGRAM_REGISTRY: Symbol = symbol_short!("ProgReg");
const PROGRAM_REGISTERED: Symbol = symbol_short!("ProgRgd");
const RELEASE_SCHEDULED: Symbol = symbol_short!("RelSched");
const SCHEDULE_RELEASED: Symbol = symbol_short!("SchRel");

// Storage keys
const PROGRAM_DATA: Symbol = symbol_short!("ProgData");
const RECEIPT_ID: Symbol = symbol_short!("RcptID");
const SCHEDULES: Symbol = symbol_short!("Scheds");
const RELEASE_HISTORY: Symbol = symbol_short!("RelHist");
const NEXT_SCHEDULE_ID: Symbol = symbol_short!("NxtSched");
const PROGRAM_INDEX: Symbol = symbol_short!("ProgIdx");
const AUTH_KEY_INDEX: Symbol = symbol_short!("AuthIdx");
const FEE_CONFIG: Symbol = symbol_short!("FeeCfg");

// Fee rate is stored in basis points (1 basis point = 0.01%)
// Example: 100 basis points = 1%, 1000 basis points = 10%
const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 1_000; // Maximum 10% fee

pub const RISK_FLAG_HIGH_RISK: u32 = 1 << 0;
pub const RISK_FLAG_UNDER_REVIEW: u32 = 1 << 1;
pub const RISK_FLAG_RESTRICTED: u32 = 1 << 2;
pub const RISK_FLAG_DEPRECATED: u32 = 1 << 3;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,    // Fee rate for lock operations (basis points)
    pub payout_fee_rate: i128,  // Fee rate for payout operations (basis points)
    pub fee_recipient: Address, // Address to receive fees
    pub fee_enabled: bool,      // Global fee enable/disable flag
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
    }
}

// ── Step 1: Add module declarations near the top of lib.rs ──────────────
// (after `mod anti_abuse;` and before the contract struct)

// ========================================================================
// Contract Data Structures & Keys
// ========================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
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
pub struct ReleaseScheduledEvent {
    pub version: u32,
    pub program_id: String,
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub release_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleReleasedEvent {
    pub version: u32,
    pub program_id: String,
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub released_at: u64,
    pub released_by: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRiskFlagsUpdated {
    pub version: u32,
    pub program_id: String,
    pub previous_flags: u32,
    pub new_flags: u32,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramMetadata {
    pub program_name: Option<String>,
    pub program_type: Option<String>,
    pub ecosystem: Option<String>,
    pub tags: Vec<String>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
    pub custom_fields: Vec<(String, String)>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address,
    pub initial_liquidity: i128,
    pub risk_flags: u32,
    pub reference_hash: Option<soroban_sdk::Bytes>,
}

// ========================================================================
// Dispute Resolution Types
// ========================================================================

/// The lifecycle state of a dispute on a program.
///
/// Transitions:
/// ```text
/// (none) ──open_dispute()──► Open ──resolve_dispute()──► Resolved
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeState {
    /// No active dispute; payouts proceed normally.
    None,
    /// Dispute is open; all payouts are blocked.
    Open,
    /// Dispute has been resolved; payouts are unblocked.
    Resolved,
}

/// On-chain record of a dispute raised against a program.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeRecord {
    /// Address that raised the dispute (must be admin).
    pub raised_by: Address,
    /// Human-readable reason for the dispute.
    pub reason: String,
    /// Ledger timestamp when the dispute was opened.
    pub opened_at: u64,
    /// Current lifecycle state.
    pub state: DisputeState,
    /// Address that resolved the dispute, if any.
    pub resolved_by: Option<Address>,
    /// Ledger timestamp when the dispute was resolved, if any.
    pub resolved_at: Option<u64>,
    /// Resolution notes provided by the resolver.
    pub resolution_notes: Option<String>,
}

/// Event emitted when a dispute is opened.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeOpenedEvent {
    pub version: u32,
    pub program_id: String,
    pub raised_by: Address,
    pub reason: String,
    pub opened_at: u64,
}

/// Event emitted when a dispute is resolved.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolvedEvent {
    pub version: u32,
    pub program_id: String,
    pub resolved_by: Address,
    pub resolution_notes: String,
    pub resolved_at: u64,
}

// Event symbols for dispute lifecycle
const DISPUTE_OPENED: Symbol = symbol_short!("DspOpen");
const DISPUTE_RESOLVED: Symbol = symbol_short!("DspRslv");

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
    MaintenanceMode,                 // bool flag
    ProgramDependencies(String),     // program_id -> Vec<String>
    DependencyStatus(String),        // program_id -> DependencyStatus
    Dispute,                         // DisputeRecord (single active dispute per contract)
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
    pub reason: Option<String>,
    pub timestamp: u64,
    pub receipt_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaintenanceModeChanged {
    pub enabled: bool,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyWithdrawEvent {
    pub admin: Address,
    pub target: Address,
    pub amount: i128,
    pub timestamp: u64,
    pub receipt_id: u64,
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
pub struct ProgramReleaseHistory {
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub released_at: u64,
    pub release_type: ReleaseType,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseType {
    Manual,
    Automatic,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyStatus {
    Pending,
    Verified,
    Rejected,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramInitItem {
    pub program_id: String,
    pub authorized_payout_key: Address,
    pub token_address: Address,
    pub reference_hash: Option<soroban_sdk::Bytes>,
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

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BatchError {
    InvalidBatchSize = 1,
    ProgramAlreadyExists = 2,
    DuplicateProgramId = 3,
}

pub const MAX_BATCH_SIZE: u32 = 100;

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
}

mod anti_abuse {
    use soroban_sdk::{symbol_short, Address, Env, Symbol};

    const RATE_LIMIT: Symbol = symbol_short!("RateLim");

    pub fn check_rate_limit(env: &Env, _caller: Address) {
        let count: u32 = env.storage().instance().get(&RATE_LIMIT).unwrap_or(0);
        env.storage().instance().set(&RATE_LIMIT, &(count + 1));
    }
}

mod claim_period;
pub use claim_period::{ClaimRecord, ClaimStatus};
mod payout_splits;
pub use payout_splits::{BeneficiarySplit, SplitConfig};
#[cfg(test)]
mod test_claim_period_expiry_cancellation;

mod error_recovery;
mod reentrancy_guard;
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
mod threshold_monitor;
mod token_math;

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

mod test_maintenance_mode;
mod test_risk_flags;
#[cfg(test)]
#[cfg(test)]
mod test_serialization_compatibility;

// ========================================================================
// Contract Implementation
// ========================================================================

// ========================================================================

// ========================================================================
// Contract Implementation
// ========================================================================

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    fn increment_receipt_id(env: &Env) -> u64 {
        let mut id: u64 = env.storage().instance().get(&RECEIPT_ID).unwrap_or(0);
        id += 1;
        env.storage().instance().set(&RECEIPT_ID, &id);
        id
    }

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
        reference_hash: Option<soroban_sdk::Bytes>,
    ) -> ProgramData {
        Self::initialize_program(
            env,
            program_id,
            authorized_payout_key,
            token_address,
            creator,
            initial_liquidity,
            reference_hash,
        )
    }

    pub fn initialize_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
        creator: Address,
        initial_liquidity: Option<i128>,
        reference_hash: Option<soroban_sdk::Bytes>,
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
            payout_history: Vec::new(&env),
            token_address: token_address.clone(),
            initial_liquidity: init_liquidity,
            risk_flags: 0,
            reference_hash,
        };

        // Store program data in registry
        let program_key = DataKey::Program(program_id.clone());
        env.storage().instance().set(&program_key, &program_data);

        // Track dependencies (default empty)
        let empty_dependencies: Vec<String> = vec![&env];
        env.storage().instance().set(
            &DataKey::ProgramDependencies(program_id.clone()),
            &empty_dependencies,
        );
        env.storage().instance().set(
            &DataKey::DependencyStatus(program_id.clone()),
            &DependencyStatus::Pending,
        );

        // Store program data
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        // Fallback for legacy tests: if admin not set, set it to authorized_payout_key
        if !env.storage().instance().has(&DataKey::Admin) {
            env.storage()
                .instance()
                .set(&DataKey::Admin, &authorized_payout_key);
        }
        if !env.storage().instance().has(&DataKey::MaintenanceMode) {
            env.storage()
                .instance()
                .set(&DataKey::MaintenanceMode, &false);
        }
        if !env.storage().instance().has(&DataKey::PauseFlags) {
            env.storage().instance().set(
                &DataKey::PauseFlags,
                &PauseFlags {
                    lock_paused: false,
                    release_paused: false,
                    refund_paused: false,
                    pause_reason: None,
                    paused_at: 0,
                },
            );
        }

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

    pub fn init_program_with_metadata(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
        organizer: Option<Address>,
        metadata: Option<ProgramMetadata>,
    ) -> ProgramData {
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, authorized_payout_key.clone());

        let start = env.ledger().timestamp();
        let caller = authorized_payout_key.clone();

        // Validate program_id (basic length check)
        if program_id.len() == 0 {
            panic!("Program ID cannot be empty");
        }

        if let Some(ref meta) = metadata {
            // Validate metadata fields (basic checks)
            if let Some(ref name) = meta.program_name {
                if name.len() == 0 {
                    panic!("Program name cannot be empty if provided");
                }
            }
        }

        Self::initialize_program(
            env,
            program_id,
            authorized_payout_key,
            token_address,
            organizer.unwrap_or(caller),
            None,
            None,
        )
    }

    /// Batch-initialize multiple programs in one transaction (all-or-nothing).
    ///
    /// # Errors
    /// * `BatchError::InvalidBatchSize` - empty or len > MAX_BATCH_SIZE
    /// * `BatchError::DuplicateProgramId` - duplicate program_id in items
    /// * `BatchError::ProgramAlreadyExists` - a program_id already registered
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

        // Update registry
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
                payout_history: Vec::new(&env),
                token_address: token_address.clone(),
                initial_liquidity: 0,
                risk_flags: 0,
                reference_hash: item.reference_hash.clone(),
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

    // ========================================================================
    // Fund Management
    // ========================================================================

    /// Lock initial funds into the program escrow
    ///
    /// # Arguments
    /// * `amount` - Amount of funds to lock (in native token units)
    ///
    /// # Returns
    /// Updated ProgramData with locked funds
    pub fn lock_program_funds(env: Env, amount: i128) -> ProgramData {
        // Validation precedence (deterministic ordering):
        // 1. Contract initialized
        // 2. Paused (operational state)
        // 3. Input validation (amount)

        // 1. Contract must be initialized
        if !env.storage().instance().has(&PROGRAM_DATA) {
            panic!("Program not initialized");
        }

        // 2. Operational state: paused
        if Self::check_paused(&env, symbol_short!("lock")) {
            panic!("Funds Paused");
        }

        // 3. Input validation
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap();

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
        env.storage()
            .instance()
            .set(&DataKey::MaintenanceMode, &false);
        env.storage().instance().set(
            &DataKey::PauseFlags,
            &PauseFlags {
                lock_paused: false,
                release_paused: false,
                refund_paused: false,
                pause_reason: None,
                paused_at: 0,
            },
        );
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

    fn require_admin(env: &Env) -> Address {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Not initialized"));
        admin.require_auth();
        admin
    }

    fn get_program_data_by_id(env: &Env, program_id: &String) -> ProgramData {
        let program_key = DataKey::Program(program_id.clone());
        if env.storage().instance().has(&program_key) {
            return env
                .storage()
                .instance()
                .get(&program_key)
                .unwrap_or_else(|| panic!("Program not found"));
        }

        if env.storage().instance().has(&PROGRAM_DATA) {
            let program_data: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));
            if &program_data.program_id == program_id {
                return program_data;
            }
        }

        panic!("Program not found");
    }

    fn store_program_data(env: &Env, program_id: &String, program_data: &ProgramData) {
        let program_key = DataKey::Program(program_id.clone());
        env.storage().instance().set(&program_key, program_data);

        if env.storage().instance().has(&PROGRAM_DATA) {
            let existing: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));
            if &existing.program_id == program_id {
                env.storage().instance().set(&PROGRAM_DATA, program_data);
            }
        }
    }

    /// Set risk flags for a program (admin only).
    pub fn set_program_risk_flags(env: Env, program_id: String, flags: u32) -> ProgramData {
        let admin = Self::require_admin(&env);
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        let previous_flags = program_data.risk_flags;
        program_data.risk_flags = flags;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (PROGRAM_RISK_FLAGS_UPDATED, program_id.clone()),
            ProgramRiskFlagsUpdated {
                version: EVENT_VERSION_V2,
                program_id,
                previous_flags,
                new_flags: program_data.risk_flags,
                admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
    }

    /// Clear specific risk flags for a program (admin only).
    pub fn clear_program_risk_flags(env: Env, program_id: String, flags: u32) -> ProgramData {
        let admin = Self::require_admin(&env);
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        let previous_flags = program_data.risk_flags;
        program_data.risk_flags &= !flags;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (PROGRAM_RISK_FLAGS_UPDATED, program_id.clone()),
            ProgramRiskFlagsUpdated {
                version: EVENT_VERSION_V2,
                program_id,
                previous_flags,
                new_flags: program_data.risk_flags,
                admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
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
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                PauseStateChanged {
                    operation: symbol_short!("lock"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
        }

        if let Some(paused) = release {
            flags.release_paused = paused;
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                PauseStateChanged {
                    operation: symbol_short!("release"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
        }

        if let Some(paused) = refund {
            flags.refund_paused = paused;
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                PauseStateChanged {
                    operation: symbol_short!("refund"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
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

    /// Check if the contract is in maintenance mode
    pub fn is_maintenance_mode(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::MaintenanceMode)
            .unwrap_or(false)
    }

    /// Update maintenance mode (admin only)
    pub fn set_maintenance_mode(env: Env, enabled: bool) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::MaintenanceMode, &enabled);
        env.events().publish(
            (MAINTENANCE_MODE_CHANGED,),
            MaintenanceModeChanged {
                enabled,
                admin: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );
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
            let receipt_id = Self::increment_receipt_id(&env);
            env.events().publish(
                (symbol_short!("em_wtd"),),
                EmergencyWithdrawEvent {
                    admin,
                    target: target.clone(),
                    amount: balance,
                    timestamp: env.ledger().timestamp(),
                    receipt_id,
                },
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
        if Self::is_maintenance_mode(env.clone()) && operation == symbol_short!("lock") {
            return true;
        }
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
        failure_threshold: u32,
        success_threshold: u32,
        max_error_log: u32,
    ) {
        caller.require_auth();
        let admin = error_recovery::get_circuit_admin(&env).expect("Circuit admin not set");
        if caller != admin {
            panic!("Unauthorized: only circuit admin can configure");
        }
        
        let config = error_recovery::CircuitBreakerConfig {
            failure_threshold,
            success_threshold,
            max_error_log,
        };
        error_recovery::set_config(&env, config);
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

        // Emit audit event for rate limit config update
        env.events().publish(
            (symbol_short!("rate_lim"), symbol_short!("update")),
            (window_size, max_operations, cooldown_period, admin, env.ledger().timestamp()),
        );
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

    /// Execute batch payouts to multiple winners.
    ///
    /// This function distributes prizes to multiple recipients in a single atomic transaction.
    /// It enforces "all-or-nothing" semantics: if any individual transfer fails, the entire
    /// batch operation reverts, ensuring accounting consistency.
    ///
    /// # Arguments
    /// * `recipients` - Vector of winner addresses.
    /// * `amounts` - Vector of prize amounts (must match recipients length).
    ///
    /// # Returns
    /// The updated `ProgramData` reflecting the new balance and payout history.
    ///
    /// # Security
    /// - Requires authorization from the `authorized_payout_key`.
    /// - Protected by reentrancy guard.
    /// - Respects circuit breaker and threshold limits.
    pub fn batch_payout(env: Env, recipients: Vec<Address>, amounts: Vec<i128>) -> ProgramData {
        // Validation precedence (deterministic ordering):
        // 1. Reentrancy guard
        // 2. Contract initialized
        // 3. Paused (operational state)
        // 4. Authorization
        // 6. Business logic (sufficient balance)
        // 7. Circuit breaker check

        // 1. Reentrancy guard
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        // 2. Contract must be initialized
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| {
                reentrancy_guard::clear_entered(&env);
                panic!("Program not initialized")
            });

        // 3. Operational state: paused
        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        // 3b. Dispute guard — payouts blocked while a dispute is open
        if Self::dispute_state(&env) == DisputeState::Open {
            reentrancy_guard::clear_entered(&env);
            panic!("Payout blocked: dispute open");
        }

        // 4. Authorization
        program_data.authorized_payout_key.require_auth();

        // 5. Input validation
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

        // 6. Business logic: sufficient balance
        if total_payout > program_data.remaining_balance {
            reentrancy_guard::clear_entered(&env);
            panic!("Insufficient balance");
        }

        // 7. Circuit breaker check
        if let Err(err_code) = error_recovery::check_and_allow_with_thresholds(&env) {
            reentrancy_guard::clear_entered(&env);
            if err_code == error_recovery::ERR_CIRCUIT_OPEN {
                panic!("Circuit breaker is OPEN");
            } else {
                panic!("Operation rejected by circuit breaker");
            }
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
            
            // Record success for circuit breaker and threshold monitor
            error_recovery::record_success(&env);
            threshold_monitor::record_operation_success(&env);
            threshold_monitor::record_outflow(&env, amount);

            // Record success for circuit breaker and threshold monitor
            error_recovery::record_success(&env);
            threshold_monitor::record_operation_success(&env);
            threshold_monitor::record_outflow(&env, amount);

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

    /// Execute a single payout to one winner.
    ///
    /// # Arguments
    /// * `recipient` - Address of the winner.
    /// * `amount` - Amount to transfer.
    ///
    /// # Returns
    /// The updated `ProgramData`.
    ///
    /// # Security
    /// - Requires authorization from the `authorized_payout_key`.
    /// - Protected by reentrancy guard.
    /// - Respects circuit breaker and threshold limits.
    pub fn single_payout(env: Env, recipient: Address, amount: i128) -> ProgramData {
        // Validation precedence (deterministic ordering):
        // 1. Reentrancy guard
        // 2. Contract initialized
        // 3. Paused (operational state)
        // 4. Authorization
        // 6. Business logic (sufficient balance)
        // 7. Circuit breaker check

        // 1. Reentrancy guard
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        // 2. Contract must be initialized
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| {
                reentrancy_guard::clear_entered(&env);
                panic!("Program not initialized")
            });

        // 3. Operational state: paused
        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        // 3b. Dispute guard — payouts blocked while a dispute is open
        if Self::dispute_state(&env) == DisputeState::Open {
            reentrancy_guard::clear_entered(&env);
            panic!("Payout blocked: dispute open");
        }

        // 4. Authorization
        program_data.authorized_payout_key.require_auth();

        // 5. Input validation
        if amount <= 0 {
            reentrancy_guard::clear_entered(&env);
            panic!("Amount must be greater than zero");
        }

        // 6. Business logic: sufficient balance
        if amount > program_data.remaining_balance {
            reentrancy_guard::clear_entered(&env);
            panic!("Insufficient balance");
        }

        // 7. Circuit breaker check
        if let Err(err_code) = error_recovery::check_and_allow_with_thresholds(&env) {
            reentrancy_guard::clear_entered(&env);
            if err_code == error_recovery::ERR_CIRCUIT_OPEN {
                panic!("Circuit breaker is OPEN");
            } else {
                panic!("Operation rejected by circuit breaker");
            }
        }

        // Transfer funds from contract to recipient
        token_client.transfer(&contract_address, &recipient, &amount);

        // Record success for circuit breaker and threshold monitor
        error_recovery::record_success(&env);
        threshold_monitor::record_operation_success(&env);
        threshold_monitor::record_outflow(&env, amount);

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
    ///
    /// # Arguments
    /// * `recipient` - Address of the recipient
    /// * `amount` - Amount to be released
    /// * `release_timestamp` - Unix timestamp when the release becomes available
    ///
    /// # Returns
    /// The created ProgramReleaseSchedule
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

        // Emit ReleaseScheduled event
        env.events().publish(
            (RELEASE_SCHEDULED,),
            ReleaseScheduledEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id,
                schedule_id,
                recipient,
                amount,
                release_timestamp,
            },
        );

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

        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        let mut schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
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
                recipient: schedule.recipient.clone(),
                amount: schedule.amount,
                released_at: now,
                release_type: ReleaseType::Automatic,
            });

            // Emit ScheduleReleased event
            env.events().publish(
                (SCHEDULE_RELEASED,),
                ScheduleReleasedEvent {
                    version: EVENT_VERSION_V2,
                    program_id: program_data.program_id.clone(),
                    schedule_id: schedule.schedule_id,
                    recipient: schedule.recipient,
                    amount: schedule.amount,
                    released_at: now,
                    released_by: contract_address.clone(),
                },
            );

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

    // --- Payout Splits (Ratio-based) ---

    pub fn set_split_config(
        env: Env,
        program_id: String,
        beneficiaries: Vec<BeneficiarySplit>,
    ) -> SplitConfig {
        payout_splits::set_split_config(&env, &program_id, beneficiaries)
    }

    pub fn get_split_config(env: Env, program_id: String) -> Option<SplitConfig> {
        payout_splits::get_split_config(&env, &program_id)
    }

    pub fn disable_split_config(env: Env, program_id: String) {
        payout_splits::disable_split_config(&env, &program_id);
    }

    pub fn execute_split_payout(
        env: Env,
        program_id: String,
        total_amount: i128,
    ) -> payout_splits::SplitPayoutResult {
        payout_splits::execute_split_payout(&env, &program_id, total_amount)
    }

    pub fn preview_split(
        env: Env,
        program_id: String,
        total_amount: i128,
    ) -> Vec<BeneficiarySplit> {
        payout_splits::preview_split(&env, &program_id, total_amount)
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

    // ========================================================================
    // Dispute Resolution
    // ========================================================================

    /// Returns the current dispute state for this contract instance.
    ///
    /// `DisputeState::None` is returned when no dispute record exists.
    fn dispute_state(env: &Env) -> DisputeState {
        env.storage()
            .instance()
            .get::<DataKey, DisputeRecord>(&DataKey::Dispute)
            .map(|r| r.state)
            .unwrap_or(DisputeState::None)
    }

    /// Open a dispute on the program, blocking all payouts until resolved.
    ///
    /// # Authorization
    /// Caller must be the contract admin.
    ///
    /// # Errors
    /// Panics if:
    /// - Contract is not initialized (no admin set).
    /// - A dispute is already open (`DisputeState::Open`).
    ///
    /// # Events
    /// Emits `DspOpen` with [`DisputeOpenedEvent`].
    pub fn open_dispute(env: Env, reason: String) -> DisputeRecord {
        let admin = Self::require_admin(&env);

        // Only one active dispute at a time
        if Self::dispute_state(&env) == DisputeState::Open {
            panic!("Dispute already open");
        }

        let now = env.ledger().timestamp();
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        let record = DisputeRecord {
            raised_by: admin.clone(),
            reason: reason.clone(),
            opened_at: now,
            state: DisputeState::Open,
            resolved_by: None,
            resolved_at: None,
            resolution_notes: None,
        };

        env.storage()
            .instance()
            .set(&DataKey::Dispute, &record);

        env.events().publish(
            (DISPUTE_OPENED,),
            DisputeOpenedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id,
                raised_by: admin,
                reason,
                opened_at: now,
            },
        );

        record
    }

    /// Resolve an open dispute, unblocking payouts.
    ///
    /// # Authorization
    /// Caller must be the contract admin.
    ///
    /// # Errors
    /// Panics if:
    /// - Contract is not initialized (no admin set).
    /// - No dispute is currently open.
    ///
    /// # Events
    /// Emits `DspRslv` with [`DisputeResolvedEvent`].
    pub fn resolve_dispute(env: Env, resolution_notes: String) -> DisputeRecord {
        let admin = Self::require_admin(&env);

        let mut record: DisputeRecord = env
            .storage()
            .instance()
            .get(&DataKey::Dispute)
            .unwrap_or_else(|| panic!("No dispute found"));

        if record.state != DisputeState::Open {
            panic!("No open dispute to resolve");
        }

        let now = env.ledger().timestamp();
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        record.state = DisputeState::Resolved;
        record.resolved_by = Some(admin.clone());
        record.resolved_at = Some(now);
        record.resolution_notes = Some(resolution_notes.clone());

        env.storage()
            .instance()
            .set(&DataKey::Dispute, &record);

        env.events().publish(
            (DISPUTE_RESOLVED,),
            DisputeResolvedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id,
                resolved_by: admin,
                resolution_notes,
                resolved_at: now,
            },
        );

        record
    }

    /// Return the current dispute record, if any.
    ///
    /// Returns `None` when no dispute has ever been opened.
    pub fn get_dispute(env: Env) -> Option<DisputeRecord> {
        env.storage().instance().get(&DataKey::Dispute)
    }
}

#[cfg(test)]
mod test;

#[cfg(test)]
mod test_pause;

#[cfg(test)]
#[cfg(any())]
mod rbac_tests;
