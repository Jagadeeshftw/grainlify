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

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    String, Symbol, Vec,
};

// Event types
const PROGRAM_INITIALIZED: Symbol = symbol_short!("PrgInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FndsLock");
const BATCH_FUNDS_LOCKED: Symbol = symbol_short!("BatLck");
const BATCH_FUNDS_RELEASED: Symbol = symbol_short!("BatRel");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
const PAYOUT: Symbol = symbol_short!("Payout");
const EVENT_VERSION_V2: u32 = 2;
const PAUSE_STATE_CHANGED: Symbol = symbol_short!("PauseSt");
const PAUSE_STATE_CHANGED_V2: Symbol = symbol_short!("PauseStV2");
const MAINTENANCE_MODE_CHANGED: Symbol = symbol_short!("MaintSt");
const PROGRAM_RISK_FLAGS_UPDATED: Symbol = symbol_short!("pr_risk");
const PROGRAM_REGISTRY: Symbol = symbol_short!("ProgReg");
const PROGRAM_REGISTERED: Symbol = symbol_short!("ProgRgd");
const RELEASE_SCHEDULED: Symbol = symbol_short!("RelSched");
const SCHEDULE_RELEASED: Symbol = symbol_short!("SchRel");
const PROGRAM_DELEGATE_SET: Symbol = symbol_short!("PrgDlgS");
const PROGRAM_DELEGATE_REVOKED: Symbol = symbol_short!("PrgDlgR");
const PROGRAM_METADATA_UPDATED: Symbol = symbol_short!("PrgMeta");

// Storage keys
const PROGRAM_DATA: Symbol = symbol_short!("ProgData");
const RECEIPT_ID: Symbol = symbol_short!("RcptID");
const SCHEDULES: Symbol = symbol_short!("Scheds");
const RELEASE_HISTORY: Symbol = symbol_short!("RelHist");
const NEXT_SCHEDULE_ID: Symbol = symbol_short!("NxtSched");
const PROGRAM_INDEX: Symbol = symbol_short!("ProgIdx");
const AUTH_KEY_INDEX: Symbol = symbol_short!("AuthIdx");
const FEE_CONFIG: Symbol = symbol_short!("FeeCfg");
const FEE_COLLECTED: Symbol = symbol_short!("FeeCol");

// Fee rate is stored in basis points (1 basis point = 0.01%)
// Example: 100 basis points = 1%, 1000 basis points = 10%
const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 1_000; // Maximum 10% fee

pub const RISK_FLAG_HIGH_RISK: u32 = 1 << 0;
pub const RISK_FLAG_UNDER_REVIEW: u32 = 1 << 1;
pub const RISK_FLAG_RESTRICTED: u32 = 1 << 2;
pub const RISK_FLAG_DEPRECATED: u32 = 1 << 3;
pub const DELEGATE_PERMISSION_RELEASE: u32 = 1 << 0;
pub const DELEGATE_PERMISSION_REFUND: u32 = 1 << 1;
pub const DELEGATE_PERMISSION_UPDATE_META: u32 = 1 << 2;
pub const DELEGATE_PERMISSION_MASK: u32 = DELEGATE_PERMISSION_RELEASE
    | DELEGATE_PERMISSION_REFUND
    | DELEGATE_PERMISSION_UPDATE_META;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,    // Fee rate for lock operations (basis points)
    pub payout_fee_rate: i128,  // Fee rate for each payout (basis points of gross payout)
    pub lock_fixed_fee: i128,   // Flat fee on lock (token units), capped to lock amount
    pub payout_fixed_fee: i128, // Flat fee per payout (token units), capped to gross payout
    pub fee_recipient: Address, // Address to receive fees
    pub fee_enabled: bool,      // Global fee enable/disable flag
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollectedEvent {
    pub version: u32,
    pub operation: Symbol,
    pub fee_amount: i128,
    pub fee_rate_bps: i128,
    pub fee_fixed: i128,
    pub recipient: Address,
    pub timestamp: u64,
}
// ==================== MONITORING MODULE ====================
mod monitoring {
    use soroban_sdk::{contracttype, Address, Env, String, Symbol};

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
    pub fn track_operation(env: &Env, _operation: Symbol, _caller: Address, success: bool) {
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
pub struct ProgramDelegateSetEvent {
    pub version: u32,
    pub program_id: String,
    pub delegate: Address,
    pub permissions: u32,
    pub updated_by: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramDelegateRevokedEvent {
    pub version: u32,
    pub program_id: String,
    pub revoked_by: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramMetadataUpdatedEvent {
    pub version: u32,
    pub program_id: String,
    pub updated_by: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramPublishedEvent {
    pub version: u32,
    pub program_id: String,
    pub published_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramMetadataField {
    pub key: String,
    pub value: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramMetadata {
    pub program_name: Option<String>,
    pub program_type: Option<String>,
    pub ecosystem: Option<String>,
    pub tags: soroban_sdk::Vec<String>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
    pub custom_fields: soroban_sdk::Vec<ProgramMetadataField>,
}

/// Lifecycle status of a program.
///
/// Upgrade-safe: new variants may be added in future versions.
/// Stored as part of `ProgramData` — increment `STORAGE_SCHEMA_VERSION`
/// if the discriminant layout changes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgramStatus {
    /// Program created but not yet published; payouts are blocked.
    Draft,
    /// Program is live; payouts are allowed.
    Active,
}

/// Current storage schema version for `ProgramData`.
///
/// Increment whenever `ProgramData` layout changes in a breaking way.
/// Written to instance storage during `init` so upgrade-safety checks
/// can detect schema mismatches on legacy deployments.
pub const STORAGE_SCHEMA_VERSION: u32 = 2;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub authorized_payout_key: Address,
    pub delegate: Option<Address>,
    pub delegate_permissions: u32,
    pub payout_history: soroban_sdk::Vec<PayoutRecord>,
    pub token_address: Address,
    pub initial_liquidity: i128,
    pub risk_flags: u32,
    pub reference_hash: Option<soroban_sdk::Bytes>,
    pub archived: bool,
    pub archived_at: Option<u64>,
    /// Lifecycle status — upgrade-safe field added in schema v2.
    pub status: ProgramStatus,
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

// ─────────────────────────────────────────────────────────────────────────────
// SPEND-LIMIT THRESHOLD AUDIT EVENTS
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when the admin sets or updates the per-program spend threshold.
///
/// ### Topics
/// `(SPEND_LIMIT_SET, program_id)`
///
/// ### Security notes
/// - Only the admin can call `set_program_spend_threshold`.
/// - `previous_threshold` is `i128::MAX` when no threshold was previously set.
/// - Emitted **after** the new value is persisted so the event reflects
///   the settled on-chain state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpendLimitSetEvent {
    pub version: u32,
    /// Program the threshold applies to.
    pub program_id: String,
    /// Previous threshold value (`i128::MAX` = unlimited).
    pub previous_threshold: i128,
    /// New threshold value.
    pub new_threshold: i128,
    /// Admin that made the change.
    pub set_by: Address,
    /// Ledger timestamp.
    pub timestamp: u64,
}

/// Emitted when a payout is rejected because it would exceed the spend threshold.
///
/// ### Topics
/// `(SPEND_LIMIT_EXCEEDED, program_id)`
///
/// ### Security notes
/// - Emitted **before** any token transfer so no funds move on rejection.
/// - `requested_amount` and `threshold` are published so auditors can
///   verify the rejection was correct without re-reading storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpendLimitExceededEvent {
    pub version: u32,
    /// Program the threshold applies to.
    pub program_id: String,
    /// Amount that was requested (and rejected).
    pub requested_amount: i128,
    /// Configured threshold that was exceeded.
    pub threshold: i128,
    /// Ledger timestamp.
    pub timestamp: u64,
}

/// Emitted once during contract initialization to record the spend-limit
/// storage schema version for upgrade-safety tracking.
///
/// ### Topics
/// `(SPEND_LIMIT_SCHEMA,)`
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpendLimitSchemaVersionSet {
    pub version: u32,
    /// Schema version written to instance storage.
    pub schema_version: u32,
    /// Ledger timestamp.
    pub timestamp: u64,
}

// Event symbols for dispute lifecycle
const DISPUTE_OPENED: Symbol = symbol_short!("DspOpen");
const DISPUTE_RESOLVED: Symbol = symbol_short!("DspRslv");

// Event symbols for spend-limit threshold lifecycle
const SPEND_LIMIT_SET: Symbol = symbol_short!("SpLimSet");
const SPEND_LIMIT_EXCEEDED: Symbol = symbol_short!("SpLimExc");
const SPEND_LIMIT_SCHEMA: Symbol = symbol_short!("SpLimSch");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String),                 // program_id -> ProgramData
    Admin,                           // Contract Admin
    ReleaseSchedule(String, u64),    // program_id, schedule_id -> ProgramReleaseSchedule
    ReleaseHistory(String),          // program_id -> soroban_sdk::Vec<ProgramReleaseHistory>
    NextScheduleId(String),          // program_id -> next schedule_id
    MultisigConfig(String),          // program_id -> MultisigConfig
    SplitConfig(String),             // program_id -> SplitConfig (payout splits)
    PayoutApproval(String, Address), // program_id, recipient -> PayoutApproval
    PendingClaim(String, u64),       // (program_id, schedule_id) -> ClaimRecord
    ClaimWindow,                     // u64 seconds (global config)
    PauseFlags,                      // PauseFlags struct
    RateLimitConfig,                 // RateLimitConfig struct
    MaintenanceMode,                 // bool flag
    ProgramDependencies(String),     // program_id -> Vec<String>
    DependencyStatus(String),        // program_id -> DependencyStatus
    Dispute,                         // DisputeRecord (single active dispute per contract)
    HistoryPaginationConfig,         // HistoryPaginationConfig
    ReadOnlyMode,                    // bool — blocks mutations during indexer backfills/upgrades
    ProgramMetadataKey(String),      // program_id -> ProgramMetadata (stored separately for upgrade safety)
    /// Upgrade-safe schema version marker for spend-limit threshold storage.
    /// Written on init; increment when `MultisigConfig` layout changes.
    SpendLimitSchemaVersion,
    /// Upgrade-safe schema version marker for pause flags storage.
    /// Written on init; increment when `PauseFlags` layout changes.
    PauseSchemaVersion,
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

/// V2 audit event for pause state changes — deterministic, upgrade-safe.
///
/// Emitted alongside [`PauseStateChanged`] for every `set_paused` call.
/// Adds `version`, `previous_paused`, and `schema_version` fields so
/// indexers can detect schema mismatches and reconstruct state transitions
/// without reading storage.
///
/// ### Topics
/// `(PAUSE_STATE_CHANGED_V2, operation_symbol)`
///
/// ### Security notes
/// - `previous_paused` is read from storage **before** the mutation so the
///   event accurately reflects the transition (old → new).
/// - `invariant_ok` is always `true` on-chain; a `false` value would indicate
///   a storage corruption bug.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseStateChangedV2 {
    pub version: u32,
    pub operation: Symbol,
    pub previous_paused: bool,
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
pub struct HistoryPaginationConfig {
    pub max_limit: u32,
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

/// Program reputation metrics tracking performance and reliability.
/// Includes counts of payouts and schedules, funds tracking, and performance scores in basis points.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReputation {
    /// Total number of direct payouts executed
    pub total_payouts: u32,
    /// Total number of release schedules created
    pub total_scheduled: u32,
    /// Number of schedules successfully released
    pub completed_releases: u32,
    /// Number of schedules awaiting release
    pub pending_releases: u32,
    /// Number of schedules past their release timestamp (not yet released)
    pub overdue_releases: u32,
    /// Count of disputes (reserved for future use)
    pub dispute_count: u32,
    /// Count of refunds (reserved for future use)
    pub refund_count: u32,
    /// Total funds locked in escrow
    pub total_funds_locked: i128,
    /// Total funds distributed via payouts
    pub total_funds_distributed: i128,
    /// Completion rate: (completed_releases / total_scheduled) * 10_000, capped at 10_000
    /// Defaults to 10_000 if no schedules exist
    pub completion_rate_bps: u32,
    /// Payout fulfillment rate: (total_funds_distributed / total_funds_locked) * 10_000
    /// Defaults to 0 if no funds locked, capped at 10_000
    pub payout_fulfillment_rate_bps: u32,
    /// Overall reputation score in basis points (0-10_000)
    /// Returns 0 if any overdue releases exist (reputation penalty for overdue milestones)
    pub overall_score_bps: u32,
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
    /// Maximum gross spend allowed in one payout operation.
    /// - `single_payout`: compared against the requested `amount`
    /// - `batch_payout`: compared against the computed batch `total_payout`
    /// `i128::MAX` disables spend-threshold enforcement.
    pub threshold_amount: i128,
    pub signers: soroban_sdk::Vec<Address>,
    pub required_signatures: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramAggregateStats {
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub total_paid_out: i128,
    pub authorized_payout_key: Address,
    pub payout_history: soroban_sdk::Vec<PayoutRecord>,
    pub token_address: Address,
    pub payout_count: u32,
    pub scheduled_count: u32,
    pub released_count: u32,
}


#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockItem {
    pub program_id: String,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseItem {
    pub program_id: String,
    pub schedule_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchFundsLocked {
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchFundsReleased {
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BatchError {
    InvalidBatchSizeProgram = 403,
    ProgramAlreadyExists = 401,
    DuplicateProgramId = 402,
    ProgramNotFound = 404,
    InvalidAmount = 4,
    ScheduleNotFound = 405,
    AlreadyReleased = 406,
    Unauthorized = 3,
    FundsPaused = 407,
    DuplicateScheduleId = 408,
}

pub const MAX_BATCH_SIZE: u32 = 100;
pub const DEFAULT_MAX_HISTORY_PAGE_LIMIT: u32 = 200;

/// Current spend-limit threshold storage schema version.
///
/// Increment whenever `MultisigConfig` layout changes in a breaking way.
/// Written to instance storage during `init` so upgrade safety checks can
/// detect schema mismatches on legacy deployments.
pub const SPEND_LIMIT_SCHEMA_VERSION_V1: u32 = 1;

/// Current pause flags storage schema version.
///
/// Increment whenever `PauseFlags` layout changes in a breaking way.
/// Written to instance storage during `init` so upgrade safety checks can
/// detect schema mismatches on legacy deployments.
pub const PAUSE_SCHEMA_VERSION_V1: u32 = 1;

fn default_history_pagination_config() -> HistoryPaginationConfig {
    HistoryPaginationConfig {
        max_limit: DEFAULT_MAX_HISTORY_PAGE_LIMIT,
    }
}

fn vec_contains(values: &Vec<String>, target: &String) -> bool {
    for value in values.iter() {
        if value == *target {
            return true;
        }
    }
    false
}

fn get_program_dependencies_internal(env: &Env, program_id: &String) -> soroban_sdk::Vec<String> {
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
    visited: &mut soroban_sdk::Vec<String>,
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
pub use payout_splits::{BeneficiarySplit, SplitConfig, SplitPayoutResult};
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

#[cfg(test)]
mod test_payout_splits;

// ========================================================================
// Contract Implementation
// ========================================================================

// ========================================================================
// Contract Implementation
// ========================================================================

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    fn get_history_pagination_config(env: &Env) -> HistoryPaginationConfig {
        env.storage()
            .instance()
            .get(&DataKey::HistoryPaginationConfig)
            .unwrap_or_else(default_history_pagination_config)
    }

    fn ensure_history_pagination_config(env: &Env) {
        if !env.storage().instance().has(&DataKey::HistoryPaginationConfig) {
            env.storage().instance().set(
                &DataKey::HistoryPaginationConfig,
                &default_history_pagination_config(),
            );
        }
    }

    fn validate_pagination(env: &Env, limit: u32) {
        if limit == 0 {
            panic!("Pagination limit must be greater than zero");
        }
        let cfg = Self::get_history_pagination_config(env);
        if limit > cfg.max_limit {
            panic!("Pagination limit exceeds maximum");
        }
    }

    fn paginate_filtered<T, F>(
        env: &Env,
        entries: soroban_sdk::Vec<T>,
        offset: u32,
        limit: u32,
        mut predicate: F,
    ) -> soroban_sdk::Vec<T>
    where
        T: Clone + soroban_sdk::TryFromVal<Env, soroban_sdk::Val> + soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
        F: FnMut(&T) -> bool,
    {
        let mut results = Vec::new(env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..entries.len() {
            if count >= limit {
                break;
            }
            let entry = entries.get(i).unwrap();
            if predicate(&entry) {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(entry);
                count += 1;
            }
        }
        results
    }

    fn order_batch_lock_items(env: &Env, items: &Vec<LockItem>) -> soroban_sdk::Vec<LockItem> {
        let mut ordered: soroban_sdk::Vec<LockItem> = Vec::new(env);
        for item in items.iter() {
            let mut next: soroban_sdk::Vec<LockItem> = Vec::new(env);
            let mut inserted = false;
            for existing in ordered.iter() {
                // String comparison for deterministic ordering
                if !inserted && item.program_id < existing.program_id {
                    next.push_back(item.clone());
                    inserted = true;
                }
                next.push_back(existing);
            }
            if !inserted {
                next.push_back(item.clone());
            }
            ordered = next;
        }
        ordered
    }

    fn order_batch_release_items(env: &Env, items: &Vec<ReleaseItem>) -> soroban_sdk::Vec<ReleaseItem> {
        let mut ordered: soroban_sdk::Vec<ReleaseItem> = Vec::new(env);
        for item in items.iter() {
            let mut next: soroban_sdk::Vec<ReleaseItem> = Vec::new(env);
            let mut inserted = false;
            for existing in ordered.iter() {
                // Sort by program_id then schedule_id
                let cmp = if item.program_id < existing.program_id {
                    true
                } else if item.program_id == existing.program_id {
                    item.schedule_id < existing.schedule_id
                } else {
                    false
                };

                if !inserted && cmp {
                    next.push_back(item.clone());
                    inserted = true;
                }
                next.push_back(existing);
            }
            if !inserted {
                next.push_back(item.clone());
            }
            ordered = next;
        }
        ordered
    }

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
        let program_key = DataKey::Program(program_id.clone());
        if env.storage().instance().has(&program_key) {
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

                let cfg = Self::get_fee_config_internal(&env);
                let fee = Self::combined_fee_amount(
                    amount,
                    cfg.lock_fee_rate,
                    cfg.lock_fixed_fee,
                    cfg.fee_enabled,
                );
                let net = amount.checked_sub(fee).unwrap_or(0);
                if net <= 0 {
                    panic!("Lock fee consumes entire initial liquidity");
                }
                if fee > 0 {
                    token_client.transfer(&contract_address, &cfg.fee_recipient, &fee);
                    Self::emit_fee_collected(
                        &env,
                        symbol_short!("lock"),
                        fee,
                        cfg.lock_fee_rate,
                        cfg.lock_fixed_fee,
                        cfg.fee_recipient.clone(),
                    );
                }
                total_funds = net;
                remaining_balance = net;
                init_liquidity = net;
            }
        }

        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds,
            remaining_balance,
            authorized_payout_key: authorized_payout_key.clone(),
            delegate: None,
            delegate_permissions: 0,
            payout_history: Vec::new(&env),
            token_address: token_address.clone(),
            initial_liquidity: init_liquidity,
            risk_flags: 0,
            reference_hash,
            archived: false,
            archived_at: None,
            // Legacy init_program flow: immediately Active (no publish step required).
            // The Draft → Active lifecycle is enforced only for batch_initialize_programs.
            status: ProgramStatus::Active,
        };

        // Store program data in registry
        let program_key = DataKey::Program(program_id.clone());
        env.storage().instance().set(&program_key, &program_data);

        let mut registry: soroban_sdk::Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(Vec::new(&env));
        let mut exists = false;
        for r in registry.iter() {
            if r == program_id {
                exists = true;
                break;
            }
        }
        if !exists {
            registry.push_back(program_id.clone());
            env.storage().instance().set(&PROGRAM_REGISTRY, &registry);
        }

        // Track dependencies (default empty)
        let empty_dependencies: soroban_sdk::Vec<String> = vec![&env];
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

        if !env.storage().instance().has(&FEE_CONFIG) {
            env.storage().instance().set(
                &FEE_CONFIG,
                &FeeConfig {
                    lock_fee_rate: 0,
                    payout_fee_rate: 0,
                    lock_fixed_fee: 0,
                    payout_fixed_fee: 0,
                    fee_recipient: authorized_payout_key.clone(),
                    fee_enabled: false,
                },
            );
        }

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
        Self::ensure_history_pagination_config(&env);

        // Write upgrade-safe spend-limit schema version marker.
        if !env
            .storage()
            .instance()
            .has(&DataKey::SpendLimitSchemaVersion)
        {
            env.storage().instance().set(
                &DataKey::SpendLimitSchemaVersion,
                &SPEND_LIMIT_SCHEMA_VERSION_V1,
            );
            env.events().publish(
                (SPEND_LIMIT_SCHEMA,),
                SpendLimitSchemaVersionSet {
                    version: EVENT_VERSION_V2,
                    schema_version: SPEND_LIMIT_SCHEMA_VERSION_V1,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        // Write upgrade-safe pause flags schema version marker.
        if !env
            .storage()
            .instance()
            .has(&DataKey::PauseSchemaVersion)
        {
            env.storage().instance().set(
                &DataKey::PauseSchemaVersion,
                &PAUSE_SCHEMA_VERSION_V1,
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

    /// Publish the program stored under the legacy single-program key (`PROGRAM_DATA`).
    ///
    /// Transitions the program from `Draft` → `Active`, enabling payouts.
    /// Called by tests and the single-program flow.
    pub fn publish_program(env: Env) -> ProgramData {
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.authorized_payout_key.require_auth();

        // If already Active (e.g., created via legacy init_program), this is a no-op.
        if program_data.status != ProgramStatus::Draft {
            return program_data;
        }

        program_data.status = ProgramStatus::Active;

        // Persist under both keys for compatibility
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        let program_key = DataKey::Program(program_data.program_id.clone());
        if env.storage().instance().has(&program_key) {
            env.storage().instance().set(&program_key, &program_data);
        }

        env.events().publish(
            (symbol_short!("PrgPub"),),
            ProgramPublishedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                published_at: env.ledger().timestamp(),
            },
        );

        program_data
    }

    /// Publish a named program from the multi-program registry.
    ///
    /// Transitions the program from `Draft` → `Active`, enabling payouts.
    pub fn publish_program_by_id(env: Env, program_id: String) -> ProgramData {
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        program_data.authorized_payout_key.require_auth();

        if program_data.status != ProgramStatus::Draft {
            panic!("Program already published");
        }

        program_data.status = ProgramStatus::Active;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (symbol_short!("PrgPub"),),
            ProgramPublishedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                published_at: env.ledger().timestamp(),
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

        let _start = env.ledger().timestamp();
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

        let mut program_data = Self::initialize_program(
            env.clone(),
            program_id,
            authorized_payout_key,
            token_address,
            organizer.unwrap_or(caller),
            None,
            None,
        );

        if let Some(program_metadata) = metadata {
            let program_id = program_data.program_id.clone();
            env.storage().instance().set(
                &DataKey::ProgramMetadataKey(program_id),
                &program_metadata,
            );
        }

        program_data
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
            return Err(BatchError::InvalidBatchSizeProgram);
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
        let mut registry: soroban_sdk::Vec<String> = env
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
                return Err(BatchError::InvalidBatchSizeProgram);
            }

            let program_data = ProgramData {
                program_id: program_id.clone(),
                total_funds: 0,
                remaining_balance: 0,
                authorized_payout_key: authorized_payout_key.clone(),
                delegate: None,
                delegate_permissions: 0,
                payout_history: Vec::new(&env),
                token_address: token_address.clone(),
                initial_liquidity: 0,
                risk_flags: 0,
                reference_hash: item.reference_hash.clone(),
                archived: false,
                archived_at: None,
                status: ProgramStatus::Draft,
            };
            let program_key = DataKey::Program(program_id.clone());
            env.storage().instance().set(&program_key, &program_data);

            if i == 0 {
                let fee_config = FeeConfig {
                    lock_fee_rate: 0,
                    payout_fee_rate: 0,
                    lock_fixed_fee: 0,
                    payout_fixed_fee: 0,
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

    /// Batch-lock funds into multiple programs atomically (all-or-nothing).
    ///
    /// Iterates over `items`, adding `amount` to each program's `total_funds`
    /// and `remaining_balance`. If any item fails (program not found, amount ≤ 0,
    /// or paused), the entire batch is rejected via `BatchError`.
    ///
    /// # Returns
    /// Number of programs successfully updated.
    pub fn batch_lock(env: Env, items: soroban_sdk::Vec<LockItem>) -> Result<u32, BatchError> {
        if items.len() == 0 || items.len() > MAX_BATCH_SIZE {
            return Err(BatchError::InvalidBatchSizeProgram);
        }

        // Validate all items before mutating any state (atomicity pre-check)
        for item in items.iter() {
            if item.amount <= 0 {
                return Err(BatchError::InvalidAmount);
            }
            let program_key = DataKey::Program(item.program_id.clone());
            if !env.storage().instance().has(&program_key) {
                return Err(BatchError::ProgramNotFound);
            }
        }

        // All valid — apply mutations
        for item in items.iter() {
            let program_key = DataKey::Program(item.program_id.clone());
            let mut program_data: ProgramData = env
                .storage()
                .instance()
                .get(&program_key)
                .unwrap();

            program_data.total_funds = program_data
                .total_funds
                .checked_add(item.amount)
                .unwrap_or_else(|| panic!("Overflow in batch_lock"));
            program_data.remaining_balance = program_data
                .remaining_balance
                .checked_add(item.amount)
                .unwrap_or_else(|| panic!("Overflow in batch_lock"));

            env.storage().instance().set(&program_key, &program_data);
        }

        env.events().publish(
            (BATCH_FUNDS_LOCKED,),
            BatchFundsLocked {
                count: items.len() as u32,
                total_amount: items.iter().map(|i| i.amount).fold(0i128, |a, b| a + b),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(items.len() as u32)
    }

    /// Batch-release scheduled payouts atomically (all-or-nothing).
    ///
    /// Iterates over `items`, executing each release schedule. If any schedule
    /// is not found, already released, or a duplicate appears in the batch,
    /// the entire batch is rejected via `BatchError`.
    ///
    /// # Returns
    /// Number of schedules successfully released.
    pub fn batch_release(env: Env, items: soroban_sdk::Vec<ReleaseItem>) -> Result<u32, BatchError> {
        if items.len() == 0 || items.len() > MAX_BATCH_SIZE {
            return Err(BatchError::InvalidBatchSizeProgram);
        }

        // Duplicate-schedule-id check within the batch
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                let a = items.get(i).unwrap();
                let b = items.get(j).unwrap();
                if a.program_id == b.program_id && a.schedule_id == b.schedule_id {
                    return Err(BatchError::DuplicateScheduleId);
                }
            }
        }

        let mut schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));

        // Validate all schedules exist and are unreleased (pre-check for atomicity)
        for item in items.iter() {
            let mut found = false;
            for s in schedules.iter() {
                if s.schedule_id == item.schedule_id {
                    if s.released {
                        return Err(BatchError::AlreadyReleased);
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(BatchError::ScheduleNotFound);
            }
        }

        // All valid — execute releases
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        let token_client = token::Client::new(&env, &program_data.token_address);
        let contract_address = env.current_contract_address();
        let mut total_released: i128 = 0;

        for item in items.iter() {
            for i in 0..schedules.len() {
                let mut s = schedules.get(i).unwrap();
                if s.schedule_id == item.schedule_id {
                    token_client.transfer(&contract_address, &s.recipient, &s.amount);
                    program_data.remaining_balance -= s.amount;
                    total_released += s.amount;
                    s.released = true;
                    schedules.set(i, s);
                    break;
                }
            }
        }

        env.storage().instance().set(&SCHEDULES, &schedules);
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        // Sync registry key if it exists
        let program_key = DataKey::Program(program_data.program_id.clone());
        if env.storage().instance().has(&program_key) {
            env.storage().instance().set(&program_key, &program_data);
        }

        env.events().publish(
            (BATCH_FUNDS_RELEASED,),
            BatchFundsReleased {
                count: items.len() as u32,
                total_amount: total_released,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(items.len() as u32)
    }


    /// Fee from basis points using floor division.
    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        if fee_rate == 0 || amount == 0 {
            return 0;
        }
        amount.checked_mul(fee_rate).unwrap_or(0) / BASIS_POINTS
    }

    /// Percentage + fixed fee, capped to `amount`.
    fn combined_fee_amount(amount: i128, rate_bps: i128, fixed: i128, fee_enabled: bool) -> i128 {
        if !fee_enabled || amount <= 0 || fixed < 0 {
            return 0;
        }
        let pct = Self::calculate_fee(amount, rate_bps);
        pct.saturating_add(fixed).min(amount).max(0)
    }

    fn emit_fee_collected(
        env: &Env,
        operation: Symbol,
        fee_amount: i128,
        fee_rate_bps: i128,
        fee_fixed: i128,
        recipient: Address,
    ) {
        if fee_amount <= 0 {
            return;
        }
        env.events().publish(
            (FEE_COLLECTED,),
            FeeCollectedEvent {
                version: EVENT_VERSION_V2,
                operation,
                fee_amount,
                fee_rate_bps,
                fee_fixed,
                recipient,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Get fee configuration (internal helper)
    fn get_fee_config_internal(env: &Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&FEE_CONFIG)
            .unwrap_or_else(|| FeeConfig {
                lock_fee_rate: 0,
                payout_fee_rate: 0,
                lock_fixed_fee: 0,
                payout_fixed_fee: 0,
                fee_recipient: env.current_contract_address(),
                fee_enabled: false,
            })
    }

    /// Read fee configuration (view).
    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
    }

    /// Update fee parameters (admin only). `None` leaves a field unchanged.
    pub fn update_fee_config(
        env: Env,
        lock_fee_rate: Option<i128>,
        payout_fee_rate: Option<i128>,
        lock_fixed_fee: Option<i128>,
        payout_fixed_fee: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
    ) {
        Self::require_admin(&env);
        let mut cfg = Self::get_fee_config_internal(&env);
        if let Some(r) = lock_fee_rate {
            if !(0..=MAX_FEE_RATE).contains(&r) {
                panic!("Invalid lock fee rate");
            }
            cfg.lock_fee_rate = r;
        }
        if let Some(r) = payout_fee_rate {
            if !(0..=MAX_FEE_RATE).contains(&r) {
                panic!("Invalid payout fee rate");
            }
            cfg.payout_fee_rate = r;
        }
        if let Some(f) = lock_fixed_fee {
            if f < 0 {
                panic!("Invalid lock fixed fee");
            }
            cfg.lock_fixed_fee = f;
        }
        if let Some(f) = payout_fixed_fee {
            if f < 0 {
                panic!("Invalid payout fixed fee");
            }
            cfg.payout_fixed_fee = f;
        }
        if let Some(a) = fee_recipient {
            cfg.fee_recipient = a;
        }
        if let Some(e) = fee_enabled {
            cfg.fee_enabled = e;
        }
        env.storage().instance().set(&FEE_CONFIG, &cfg);
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

    /// Lock funds into the program escrow with optional fee deduction.
    ///
    /// When fees are enabled, the lock fee is deducted from `amount`. Only the net
    /// amount is added to `total_funds` and `remaining_balance`. The fee is transferred
    /// to the configured fee recipient.
    ///
    /// # Arguments
    /// * `amount` - Gross amount to lock (in native token units)
    ///
    /// # Returns
    /// Updated ProgramData with locked funds and net balance after fees
    ///
    /// # Overflow Safety
    /// Uses `checked_add` to prevent balance overflow. Panics if overflow would occur.
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

        let mut program_data: ProgramData = env.storage().instance().get(&PROGRAM_DATA).unwrap();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        // Tokens are assumed to already be present in the contract (minted or pre-transferred).
        let actual_received = amount;

        // Get fee configuration
        let fee_config = Self::get_fee_config_internal(&env);

        // Calculate fees based on actually received tokens
        let fee_amount = Self::combined_fee_amount(
            actual_received,
            fee_config.lock_fee_rate,
            fee_config.lock_fixed_fee,
            fee_config.fee_enabled,
        );
        let net_amount = amount.checked_sub(fee_amount).unwrap_or(0);
        if net_amount <= 0 {
            panic!("Lock fee consumes entire lock amount");
        }

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        if fee_amount > 0 {
            token_client.transfer(&contract_address, &fee_config.fee_recipient, &fee_amount);
            Self::emit_fee_collected(
                &env,
                symbol_short!("lock"),
                fee_amount,
                fee_config.lock_fee_rate,
                fee_config.lock_fixed_fee,
                fee_config.fee_recipient.clone(),
            );
        }

        // Credit to program accounting:
        // - total_funds tracks gross amount locked (before fees)
        // - remaining_balance tracks net amount available for payouts (after fees)
        program_data.total_funds = program_data
            .total_funds
            .checked_add(amount)
            .unwrap_or_else(|| panic!("Total funds overflow"));

        program_data.remaining_balance = program_data
            .remaining_balance
            .checked_add(net_amount)
            .unwrap_or_else(|| panic!("Remaining balance overflow"));

        // Store updated data — sync both legacy and registry keys
        env.storage().instance().set(&PROGRAM_DATA, &program_data);
        let program_key = DataKey::Program(program_data.program_id.clone());
        if env.storage().instance().has(&program_key) {
            env.storage().instance().set(&program_key, &program_data);
        }

        // Emit FundsLocked event
        env.events().publish(
            (FUNDS_LOCKED,),
            FundsLockedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                amount: net_amount,
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
        Self::ensure_history_pagination_config(&env);
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

    /// Alias for `get_admin` — used by tests that call `get_program_admin()`.
    pub fn get_program_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    /// Returns the current rotation nonce for a program's payout key.
    ///
    /// The nonce is incremented on every successful `rotate_payout_key` call,
    /// providing replay protection.
    pub fn get_rotation_nonce(env: Env, program_id: String) -> u64 {
        let key = (symbol_short!("RotNonce"), program_id);
        env.storage().instance().get(&key).unwrap_or(0u64)
    }

    /// Rotate the authorized payout key for a program.
    ///
    /// # Authorization
    /// Caller must be either the current `authorized_payout_key` or the contract admin.
    ///
    /// # Replay protection
    /// `nonce` must equal the current rotation nonce; it is incremented on success.
    pub fn rotate_payout_key(
        env: Env,
        program_id: String,
        caller: Address,
        new_key: Address,
        nonce: u64,
    ) -> ProgramData {
        caller.require_auth();

        let mut program_data = Self::get_program_data_by_id(&env, &program_id);

        // Authorize: caller must be current payout key or admin
        let is_payout_key = caller == program_data.authorized_payout_key;
        let is_admin = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .map(|a| a == caller)
            .unwrap_or(false);

        if !is_payout_key && !is_admin {
            panic!("Unauthorized: caller is not payout key or admin");
        }

        // Replay protection
        let nonce_key = (symbol_short!("RotNonce"), program_id.clone());
        let current_nonce: u64 = env.storage().instance().get(&nonce_key).unwrap_or(0u64);
        if nonce != current_nonce {
            panic!("Invalid nonce");
        }

        // New key must differ
        if new_key == program_data.authorized_payout_key {
            panic!("New key must differ from current key");
        }

        program_data.authorized_payout_key = new_key;
        Self::store_program_data(&env, &program_id, &program_data);
        env.storage().instance().set(&nonce_key, &(current_nonce + 1));

        program_data
    }

    /// Update program metadata on behalf of a delegate or payout key.
    ///
    /// Alias for `update_program_metadata` with explicit caller parameter.
    pub fn update_program_metadata_by(
        env: Env,
        caller: Address,
        program_id: String,
        metadata: ProgramMetadata,
    ) -> ProgramData {
        Self::update_program_metadata(env, program_id, caller, metadata)
    }

    /// Archive a program (mark as historical/read-only). Admin-only.
    pub fn archive_program(env: Env, program_id: String) {
        Self::require_admin(&env);
        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .expect("Program not found");

        program_data.archived = true;
        program_data.archived_at = Some(env.ledger().timestamp());

        env.storage().instance().set(&program_key, &program_data);

        // Sync with global if applicable
        if let Some(global_data) = env
            .storage()
            .instance()
            .get::<Symbol, ProgramData>(&PROGRAM_DATA)
        {
            if global_data.program_id == program_id {
                env.storage().instance().set(&PROGRAM_DATA, &program_data);
            }
        }

        env.events().publish(
            (symbol_short!("Archived"),),
            (program_id, env.ledger().timestamp()),
        );
    }

    /// Get all archived program IDs.
    pub fn get_archived_programs(env: Env) -> soroban_sdk::Vec<String> {
        let registry: soroban_sdk::Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(Vec::new(&env));
        let mut archived = Vec::new(&env);
        for program_id in registry.iter() {
            let program_key = DataKey::Program(program_id.clone());
            if let Some(data) = env
                .storage()
                .instance()
                .get::<DataKey, ProgramData>(&program_key)
            {
                if data.archived {
                    archived.push_back(program_id);
                }
            }
        }
        archived
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

    /// Panics if the contract is in read-only mode.
    ///
    /// Read-only mode is set via `DataKey::ReadOnlyMode` and blocks all
    /// state-mutating operations. Used during indexer backfills and upgrades.
    fn require_not_read_only(env: &Env) {
        let read_only: bool = env
            .storage()
            .instance()
            .get(&DataKey::ReadOnlyMode)
            .unwrap_or(false);
        if read_only {
            panic!("Contract is in read-only mode");
        }
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

    fn require_program_owner_or_admin(
        env: &Env,
        program_data: &ProgramData,
        caller: &Address,
    ) -> Address {
        caller.require_auth();

        if *caller == program_data.authorized_payout_key {
            return caller.clone();
        }

        let is_admin = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Admin)
            .map(|admin| admin == *caller)
            .unwrap_or(false);
        if is_admin {
            return caller.clone();
        }

        panic!("Unauthorized");
    }

    fn require_program_actor(
        env: &Env,
        program_data: &ProgramData,
        caller: &Address,
        required_permission: u32,
    ) -> Address {
        caller.require_auth();

        if *caller == program_data.authorized_payout_key {
            return caller.clone();
        }

        let is_admin = env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Admin)
            .map(|admin| admin == *caller)
            .unwrap_or(false);
        if is_admin {
            return caller.clone();
        }

        let delegate_matches = program_data
            .delegate
            .as_ref()
            .map(|delegate| delegate == caller)
            .unwrap_or(false);
        if delegate_matches
            && (program_data.delegate_permissions & required_permission) == required_permission
        {
            return caller.clone();
        }

        panic!("Unauthorized");
    }

    fn validate_delegate_permissions(permissions: u32) {
        if permissions == 0 {
            panic!("Delegate permissions cannot be empty");
        }
        if permissions & !DELEGATE_PERMISSION_MASK != 0 {
            panic!("Unsupported delegate permissions");
        }
    }

    fn authorize_release_actor(
        env: &Env,
        program_data: &ProgramData,
        caller: Option<&Address>,
    ) -> Address {
        if let Some(address) = caller {
            return Self::require_program_actor(
                env,
                program_data,
                address,
                DELEGATE_PERMISSION_RELEASE,
            );
        }

        program_data.authorized_payout_key.require_auth();
        program_data.authorized_payout_key.clone()
    }

    pub fn set_program_delegate(
        env: Env,
        program_id: String,
        caller: Address,
        delegate: Address,
        permissions: u32,
    ) -> ProgramData {
        Self::validate_delegate_permissions(permissions);

        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        let updated_by = Self::require_program_owner_or_admin(&env, &program_data, &caller);

        if delegate == program_data.authorized_payout_key {
            panic!("Delegate must differ from owner");
        }

        program_data.delegate = Some(delegate.clone());
        program_data.delegate_permissions = permissions;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (PROGRAM_DELEGATE_SET, program_id.clone()),
            ProgramDelegateSetEvent {
                version: EVENT_VERSION_V2,
                program_id,
                delegate,
                permissions,
                updated_by,
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
    }

    pub fn revoke_program_delegate(env: Env, program_id: String, caller: Address) -> ProgramData {
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        let revoked_by = Self::require_program_owner_or_admin(&env, &program_data, &caller);

        program_data.delegate = None;
        program_data.delegate_permissions = 0;
        Self::store_program_data(&env, &program_id, &program_data);

        env.events().publish(
            (PROGRAM_DELEGATE_REVOKED, program_id.clone()),
            ProgramDelegateRevokedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                revoked_by,
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
    }

    pub fn update_program_metadata(
        env: Env,
        program_id: String,
        caller: Address,
        metadata: ProgramMetadata,
    ) -> ProgramData {
        let mut program_data = Self::get_program_data_by_id(&env, &program_id);
        let updated_by = Self::require_program_actor(
            &env,
            &program_data,
            &caller,
            DELEGATE_PERMISSION_UPDATE_META,
        );

        // Metadata is stored separately (upgrade-safe: not embedded in ProgramData)
        env.storage().instance().set(
            &DataKey::ProgramMetadataKey(program_id.clone()),
            &metadata,
        );

        env.events().publish(
            (PROGRAM_METADATA_UPDATED, program_id.clone()),
            ProgramMetadataUpdatedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                updated_by,
                timestamp: env.ledger().timestamp(),
            },
        );

        program_data
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

    pub fn get_program_release_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
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
            let previous_paused = flags.lock_paused;
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
            env.events().publish(
                (PAUSE_STATE_CHANGED_V2, symbol_short!("lock")),
                PauseStateChangedV2 {
                    version: EVENT_VERSION_V2,
                    operation: symbol_short!("lock"),
                    previous_paused,
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
        }

        if let Some(paused) = release {
            let previous_paused = flags.release_paused;
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
            env.events().publish(
                (PAUSE_STATE_CHANGED_V2, symbol_short!("release")),
                PauseStateChangedV2 {
                    version: EVENT_VERSION_V2,
                    operation: symbol_short!("release"),
                    previous_paused,
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                    receipt_id,
                },
            );
        }

        if let Some(paused) = refund {
            let previous_paused = flags.refund_paused;
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
            env.events().publish(
                (PAUSE_STATE_CHANGED_V2, symbol_short!("refund")),
                PauseStateChangedV2 {
                    version: EVENT_VERSION_V2,
                    operation: symbol_short!("refund"),
                    previous_paused,
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

    /// Returns the stored pause flags schema version.
    ///
    /// Returns `PAUSE_SCHEMA_VERSION_V1` (1) for contracts initialized after
    /// this upgrade. Returns `0` for legacy contracts that predate the schema
    /// version marker — callers should treat `0` as "unknown / pre-v1".
    pub fn get_pause_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::PauseSchemaVersion)
            .unwrap_or(0)
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
            (
                window_size,
                max_operations,
                cooldown_period,
                admin,
                env.ledger().timestamp(),
            ),
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

    /// Set the per-program spend threshold.
    ///
    /// # Invariant
    /// After this call, any single payout or batch total exceeding
    /// `threshold_amount` will be rejected with `SpendLimitExceeded` and
    /// a `SpendLimitExceededEvent` audit event will be emitted.
    ///
    /// # Security and deterministic behavior
    /// - Admin only.
    /// - `threshold_amount` must be strictly positive; zero or negative
    ///   values are rejected with `InvalidAmount`.
    /// - Payout validation checks this threshold **before** balance checks
    ///   so clients observe stable, deterministic failures.
    /// - Emits `SpendLimitSetEvent` after the new value is persisted.
    pub fn set_program_spend_threshold(env: Env, program_id: String, threshold_amount: i128) {
        let admin = Self::require_admin(&env);
        if threshold_amount <= 0 {
            panic!("Invalid spend threshold");
        }

        let mut cfg: MultisigConfig = env
            .storage()
            .persistent()
            .get(&DataKey::MultisigConfig(program_id.clone()))
            .unwrap_or(MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![&env],
                required_signatures: 0,
            });

        let previous_threshold = cfg.threshold_amount;
        cfg.threshold_amount = threshold_amount;
        env.storage()
            .persistent()
            .set(&DataKey::MultisigConfig(program_id.clone()), &cfg);

        // Emit audit event after storage write (CEI ordering).
        env.events().publish(
            (SPEND_LIMIT_SET, program_id.clone()),
            SpendLimitSetEvent {
                version: EVENT_VERSION_V2,
                program_id,
                previous_threshold,
                new_threshold: threshold_amount,
                set_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    /// Read per-program spend threshold. Returns `i128::MAX` when unset (unlimited).
    pub fn get_program_spend_threshold(env: Env, program_id: String) -> i128 {
        let cfg: MultisigConfig = env
            .storage()
            .persistent()
            .get(&DataKey::MultisigConfig(program_id))
            .unwrap_or(MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![&env],
                required_signatures: 0,
            });
        cfg.threshold_amount
    }

    /// Returns the spend-limit storage schema version written during `init_program`.
    /// Returns `0` on legacy deployments where the marker was never written.
    pub fn get_spend_limit_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::SpendLimitSchemaVersion)
            .unwrap_or(0u32)
    }

    /// Enforce the per-program spend threshold.
    ///
    /// Returns `Err(())` and emits a `SpendLimitExceededEvent` when
    /// `requested_amount > threshold`. The caller is responsible for
    /// clearing the reentrancy guard and panicking with the appropriate
    /// error before any token transfer occurs.
    fn enforce_spend_threshold(
        env: &Env,
        program_id: &String,
        requested_amount: i128,
    ) -> Result<(), ()> {
        let cfg: MultisigConfig = env
            .storage()
            .persistent()
            .get(&DataKey::MultisigConfig(program_id.clone()))
            .unwrap_or(MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![env],
                required_signatures: 0,
            });
        if requested_amount > cfg.threshold_amount {
            // Emit audit event before returning the error so the rejection
            // is always visible on-chain even if the caller panics.
            env.events().publish(
                (SPEND_LIMIT_EXCEEDED, program_id.clone()),
                SpendLimitExceededEvent {
                    version: EVENT_VERSION_V2,
                    program_id: program_id.clone(),
                    requested_amount,
                    threshold: cfg.threshold_amount,
                    timestamp: env.ledger().timestamp(),
                },
            );
            return Err(());
        }
        Ok(())
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
    pub fn batch_payout(env: Env, recipients: soroban_sdk::Vec<Address>, amounts: soroban_sdk::Vec<i128>) -> ProgramData {
        Self::batch_payout_internal(env, None, recipients, amounts)
    }

    pub fn batch_payout_by(
        env: Env,
        caller: Address,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> ProgramData {
        Self::batch_payout_internal(env, Some(caller), recipients, amounts)
    }

    fn batch_payout_internal(
        env: Env,
        caller: Option<Address>,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> ProgramData {
        // Deterministic validation order (stable across upgrades):
        // 1. Reentrancy guard
        // 2. Contract initialized
        // 3. Paused (operational state)
        // 4. Dispute guard
        // 5. Authorization
        // 6. Input validation (length mismatch, empty batch, zero/negative amounts)
        // 7. Overflow-safe total computation
        // 8. Spend threshold
        // 9. Sufficient balance
        // 10. Circuit breaker

        // 1. Reentrancy guard
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        // 2. Contract must be initialized
        let program_data: ProgramData =
            env.storage()
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

        // 5. Dispute guard — payouts blocked while a dispute is open
        if Self::dispute_state(&env) == DisputeState::Open {
            reentrancy_guard::clear_entered(&env);
            panic!("Payout blocked: dispute open");
        }

        // 6. Authorization
        Self::authorize_release_actor(&env, &program_data, caller.as_ref());

        // 7. Input validation
        if recipients.len() != amounts.len() {
            reentrancy_guard::clear_entered(&env);
            panic!("Recipients and amounts vectors must have the same length");
        }

        if recipients.len() == 0 {
            reentrancy_guard::clear_entered(&env);
            panic!("Cannot process empty batch");
        }

        // 8. Overflow-safe total computation; validate each amount > 0
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

        // 9. Spend threshold (deterministic: runs before balance/circuit checks)
        if Self::enforce_spend_threshold(&env, &program_data.program_id, total_payout).is_err() {
            reentrancy_guard::clear_entered(&env);
            panic!("Spend threshold exceeded");
        }

        // 10. Sufficient balance
        if total_payout > program_data.remaining_balance {
            reentrancy_guard::clear_entered(&env);
            panic!("Insufficient balance");
        }

        // 11. Circuit breaker check
        if let Err(err_code) = error_recovery::check_and_allow_with_thresholds(&env) {
            reentrancy_guard::clear_entered(&env);
            if err_code == error_recovery::ERR_CIRCUIT_OPEN {
                panic!("Circuit breaker is OPEN");
            } else {
                panic!("Operation rejected by circuit breaker");
            }
        }

        // Execute transfers — all-or-nothing: any token transfer failure panics
        // and Soroban reverts the entire transaction, preserving atomicity.
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        let cfg = Self::get_fee_config_internal(&env);

        for i in 0..recipients.len() {
            let recipient = recipients.get(i).unwrap().clone();
            let gross = amounts.get(i).unwrap();

            let pay_fee = Self::combined_fee_amount(
                gross,
                cfg.payout_fee_rate,
                cfg.payout_fixed_fee,
                cfg.fee_enabled,
            );
            let net = gross.checked_sub(pay_fee).unwrap_or(0);
            if net <= 0 {
                reentrancy_guard::clear_entered(&env);
                panic!("Payout fee consumes entire payout");
            }

            if pay_fee > 0 {
                token_client.transfer(&contract_address, &cfg.fee_recipient, &pay_fee);
                Self::emit_fee_collected(
                    &env,
                    symbol_short!("payout"),
                    pay_fee,
                    cfg.payout_fee_rate,
                    cfg.payout_fixed_fee,
                    cfg.fee_recipient.clone(),
                );
            }

            token_client.transfer(&contract_address, &recipient, &net);

            error_recovery::record_success(&env);
            threshold_monitor::record_operation_success(&env);
            threshold_monitor::record_outflow(&env, gross);

            let payout_record = PayoutRecord {
                recipient,
                amount: net,
                timestamp,
            };
            updated_history.push_back(payout_record);
        }

        // Update program data atomically after all transfers succeed
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
        Self::single_payout_internal(env, None, recipient, amount)
    }

    pub fn single_payout_by(
        env: Env,
        caller: Address,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
        Self::single_payout_internal(env, Some(caller), recipient, amount)
    }

    fn single_payout_internal(
        env: Env,
        caller: Option<Address>,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
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
        let program_data: ProgramData =
            env.storage()
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
        Self::authorize_release_actor(&env, &program_data, caller.as_ref());

        // 5. Input validation
        if amount <= 0 {
            reentrancy_guard::clear_entered(&env);
            panic!("Amount must be greater than zero");
        }

        // 6. Business logic: sufficient balance
        // Deterministic error ordering: spend threshold check runs before
        // balance/circuit checks, so clients observe stable failures.
        if Self::enforce_spend_threshold(&env, &program_data.program_id, amount).is_err() {
            reentrancy_guard::clear_entered(&env);
            panic!("Spend threshold exceeded");
        }

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

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        let cfg = Self::get_fee_config_internal(&env);
        let pay_fee = Self::combined_fee_amount(
            amount,
            cfg.payout_fee_rate,
            cfg.payout_fixed_fee,
            cfg.fee_enabled,
        );
        let net = amount.checked_sub(pay_fee).unwrap_or(0);
        if net <= 0 {
            reentrancy_guard::clear_entered(&env);
            panic!("Payout fee consumes entire payout");
        }

        if pay_fee > 0 {
            token_client.transfer(&contract_address, &cfg.fee_recipient, &pay_fee);
            Self::emit_fee_collected(
                &env,
                symbol_short!("payout"),
                pay_fee,
                cfg.payout_fee_rate,
                cfg.payout_fixed_fee,
                cfg.fee_recipient.clone(),
            );
        }

        token_client.transfer(&contract_address, &recipient, &net);

        error_recovery::record_success(&env);
        threshold_monitor::record_operation_success(&env);
        threshold_monitor::record_outflow(&env, amount);

        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutRecord {
            recipient: recipient.clone(),
            amount: net,
            timestamp,
        };

        let mut updated_history = program_data.payout_history.clone();
        updated_history.push_back(payout_record);

        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= amount;
        updated_data.payout_history = updated_history;

        env.storage().instance().set(&PROGRAM_DATA, &updated_data);

        env.events().publish(
            (PAYOUT,),
            PayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: updated_data.program_id.clone(),
                recipient: recipient.clone(),
                amount: net,
                remaining_balance: updated_data.remaining_balance,
            },
        );

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
        Self::create_program_release_schedule_internal(
            env,
            None,
            recipient,
            amount,
            release_timestamp,
        )
    }

    pub fn create_prog_release_schedule_by(
        env: Env,
        caller: Address,
        recipient: Address,
        amount: i128,
        release_timestamp: u64,
    ) -> ProgramReleaseSchedule {
        Self::create_program_release_schedule_internal(
            env,
            Some(caller),
            recipient,
            amount,
            release_timestamp,
        )
    }

    fn create_program_release_schedule_internal(
        env: Env,
        caller: Option<Address>,
        recipient: Address,
        amount: i128,
        release_timestamp: u64,
    ) -> ProgramReleaseSchedule {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        Self::authorize_release_actor(&env, &program_data, caller.as_ref());

        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
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
            recipient: recipient.clone(),
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
                recipient: recipient.clone(),
                amount,
                release_timestamp,
            },
        );

        schedule
    }

    /// Trigger all due schedules where `now >= release_timestamp`.
    pub fn trigger_program_releases(env: Env) -> u32 {
        Self::trigger_program_releases_internal(env, None)
    }

    pub fn trigger_program_releases_by(env: Env, caller: Address) -> u32 {
        Self::trigger_program_releases_internal(env, Some(caller))
    }

    fn trigger_program_releases_internal(env: Env, caller: Option<Address>) -> u32 {
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

        Self::authorize_release_actor(&env, &program_data, caller.as_ref());

        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        let mut schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        let mut release_history: soroban_sdk::Vec<ProgramReleaseHistory> = env
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

    pub fn get_release_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        if let Some(info) = env
            .storage()
            .instance()
            .get::<Symbol, ProgramData>(&PROGRAM_DATA)
        {
            if info.archived {
                return Vec::new(&env);
            }
        }
        env.storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_program_release_history(env: Env) -> soroban_sdk::Vec<ProgramReleaseHistory> {
        env.storage()
            .instance()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env))
    }

    // ========================================================================
    // Multi-tenant / Multi-program Migration Wrappers (ignore id for now)
    // ========================================================================

    pub fn get_program_info_v2(env: Env, program_id: String) -> ProgramData {
        let program_key = DataKey::Program(program_id);
        env.storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"))
    }

    pub fn lock_program_funds_v2(env: Env, program_id: String, amount: i128) -> ProgramData {
        Self::require_not_read_only(&env);
        // Validation precedence (deterministic ordering):
        // 1. Amount > 0
        // 2. Program exists
        // 3. Program must be in Active status (not Draft)
        // 4. Contract balance check (detects FoT issues if tokens were sent beforehand)

        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        if program_data.status == ProgramStatus::Draft {
            panic!("Program is in Draft status. Publish the program first.");
        }

        let token_client = token::Client::new(&env, &program_data.token_address);
        let contract_address = env.current_contract_address();

        // Ensure contract actually holds enough tokens to cover this lock.
        // If tokens were sent via direct transfer and a fee was taken, this check will catch it.
        if token_client.balance(&contract_address) < amount {
            panic!("Insufficient contract balance to cover lock (possible fee-on-transfer issue)");
        }

        let fee_config = Self::get_fee_config_internal(&env);
        let (fee_amount, net_amount) = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0 {
            token_math::split_amount(amount, fee_config.lock_fee_rate)
        } else {
            (0i128, amount)
        };

        if fee_amount > 0 {
            token_client.transfer(&contract_address, &fee_config.fee_recipient, &fee_amount);
        }

        program_data.total_funds = program_data
            .total_funds
            .checked_add(amount)
            .expect("Total funds overflow");
        program_data.remaining_balance = program_data
            .remaining_balance
            .checked_add(net_amount)
            .expect("Remaining balance overflow");

        env.storage().instance().set(&program_key, &program_data);

        // Sync with global if applicable
        if let Some(global_data) = env
            .storage()
            .instance()
            .get::<Symbol, ProgramData>(&PROGRAM_DATA)
        {
            if global_data.program_id == program_id {
                env.storage().instance().set(&PROGRAM_DATA, &program_data);
            }
        }

        env.events().publish(
            (FUNDS_LOCKED,),
            FundsLockedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                amount,
                remaining_balance: program_data.remaining_balance,
            },
        );

        program_data
    }

    pub fn single_payout_v2(
        env: Env,
        program_id: String,
        caller: Address,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
        Self::require_not_read_only(&env);
        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        if program_data.status == ProgramStatus::Draft {
            panic!("Program is in Draft status. Publish the program first.");
        }

        if amount <= 0 || amount > program_data.remaining_balance {
            panic!("Invalid payout amount");
        }

        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);

        program_data.remaining_balance -= amount;
        env.storage().instance().set(&program_key, &program_data);

        if let Some(global_data) = env
            .storage()
            .instance()
            .get::<Symbol, ProgramData>(&PROGRAM_DATA)
        {
            if global_data.program_id == program_id {
                env.storage().instance().set(&PROGRAM_DATA, &program_data);
            }
        }

        env.events()
            .publish((symbol_short!("Payout"),), (program_id, recipient, amount));

        program_data
    }

    pub fn batch_payout_v2(
        env: Env,
        _program_id: String,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) -> ProgramData {
        Self::batch_payout(env, recipients, amounts)
    }

    // --- Payout Splits (Ratio-based) ---

    pub fn set_split_config(
        env: Env,
        program_id: String,
        beneficiaries: soroban_sdk::Vec<BeneficiarySplit>,
    ) -> SplitConfig {
        if let Some(admin) = env.storage().instance().get::<_, Address>(&DataKey::Admin) {
            admin.require_auth();
        } else {
            let program: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));
            program.authorized_payout_key.require_auth();
        }
        payout_splits::set_split_config(&env, &program_id, beneficiaries)
    }

    pub fn get_split_config(env: Env, program_id: String) -> Option<SplitConfig> {
        payout_splits::get_split_config(&env, &program_id)
    }

    pub fn disable_split_config(env: Env, program_id: String) {
        if let Some(admin) = env.storage().instance().get::<_, Address>(&DataKey::Admin) {
            admin.require_auth();
        } else {
            let program: ProgramData = env
                .storage()
                .instance()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| panic!("Program not initialized"));
            program.authorized_payout_key.require_auth();
        }
        payout_splits::disable_split_config(&env, &program_id);
    }

    pub fn execute_split_payout(
        env: Env,
        program_id: String,
        total_amount: i128,
    ) -> payout_splits::SplitPayoutResult {
        let program: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program.authorized_payout_key.require_auth();
        payout_splits::execute_split_payout(&env, &program_id, total_amount)
    }

    pub fn preview_split(
        env: Env,
        program_id: String,
        total_amount: i128,
    ) -> soroban_sdk::Vec<BeneficiarySplit> {
        payout_splits::preview_split(&env, &program_id, total_amount)
    }

    /// Query payout history by recipient with pagination
    pub fn query_payouts_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<PayoutRecord> {
        Self::validate_pagination(&env, limit);
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::paginate_filtered(&env, program_data.payout_history, offset, limit, |record| {
            record.recipient == recipient
        })
    }

    /// Query payout history by amount range
    pub fn query_payouts_by_amount(
        env: Env,
        min_amount: i128,
        max_amount: i128,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<PayoutRecord> {
        Self::validate_pagination(&env, limit);
        if min_amount > max_amount {
            panic!("Invalid amount range");
        }
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::paginate_filtered(&env, program_data.payout_history, offset, limit, |record| {
            record.amount >= min_amount && record.amount <= max_amount
        })
    }

    /// Query payout history by timestamp range
    pub fn query_payouts_by_timestamp(
        env: Env,
        min_timestamp: u64,
        max_timestamp: u64,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<PayoutRecord> {
        Self::validate_pagination(&env, limit);
        if min_timestamp > max_timestamp {
            panic!("Invalid timestamp range");
        }
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::paginate_filtered(&env, program_data.payout_history, offset, limit, |record| {
            record.timestamp >= min_timestamp && record.timestamp <= max_timestamp
        })
    }

    /// Query release schedules by recipient
    pub fn query_schedules_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        Self::validate_pagination(&env, limit);
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::paginate_filtered(&env, schedules, offset, limit, |schedule| {
            schedule.recipient == recipient
        })
    }

    /// Query release schedules by released status
    pub fn query_schedules_by_status(
        env: Env,
        released: bool,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        Self::validate_pagination(&env, limit);
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::paginate_filtered(&env, schedules, offset, limit, |schedule| {
            schedule.released == released
        })
    }

    /// Query release history with filtering and pagination
    pub fn query_releases_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<ProgramReleaseHistory> {
        Self::validate_pagination(&env, limit);
        let history: soroban_sdk::Vec<ProgramReleaseHistory> = env
            .storage()
            .instance()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env));
        Self::paginate_filtered(&env, history, offset, limit, |record| {
            record.recipient == recipient
        })
    }

    /// Get aggregate statistics for the program
    pub fn get_program_aggregate_stats(env: Env) -> ProgramAggregateStats {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
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
    ) -> soroban_sdk::Vec<PayoutRecord> {
        Self::validate_pagination(&env, limit);
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::paginate_filtered(&env, program_data.payout_history, offset, limit, |record| {
            record.recipient == recipient
        })
    }

    /// Get pending schedules (not yet released)
    pub fn get_pending_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
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
    pub fn get_due_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
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
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
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

    pub fn list_programs(env: Env) -> soroban_sdk::Vec<ProgramData> {
        let mut results = Vec::new(&env);
        if env.storage().instance().has(&PROGRAM_DATA) {
            let data = Self::get_program_info(env.clone());
            if !data.archived {
                results.push_back(data);
            }
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

    pub fn get_all_prog_release_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        Self::get_release_schedules(env)
    }

    pub fn get_pending_program_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        Self::get_pending_schedules(env)
    }

    pub fn get_due_program_schedules(env: Env) -> soroban_sdk::Vec<ProgramReleaseSchedule> {
        Self::get_due_schedules(env)
    }

    pub fn release_program_schedule_manual(env: Env, schedule_id: u64) {
        Self::release_program_schedule_manual_internal(env, None, schedule_id)
    }

    pub fn release_prog_schedule_manual_by(env: Env, caller: Address, schedule_id: u64) {
        Self::release_program_schedule_manual_internal(env, Some(caller), schedule_id)
    }

    fn release_program_schedule_manual_internal(
        env: Env,
        caller: Option<Address>,
        schedule_id: u64,
    ) {
        let mut schedules = Self::get_release_schedules(env.clone());
        let program_data = Self::get_program_info(env.clone());

        let caller = Self::authorize_release_actor(&env, &program_data, caller.as_ref());
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

            let mut history: soroban_sdk::Vec<ProgramReleaseHistory> = env
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

            let mut history: soroban_sdk::Vec<ProgramReleaseHistory> = env
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

    /// Reserve funds for a recipient-controlled claim.
    ///
    /// This is treated as part of the release path because it authorizes
    /// a payout claim against escrowed program funds.
    pub fn create_pending_claim(
        env: Env,
        program_id: String,
        recipient: Address,
        amount: i128,
        claim_deadline: u64,
    ) -> u64 {
        if Self::check_paused(&env, symbol_short!("release")) {
            panic!("Funds Paused");
        }
        claim_period::create_pending_claim(&env, &program_id, &recipient, amount, claim_deadline)
    }

    /// Execute a previously approved claim and transfer its reserved funds.
    ///
    /// Claims are part of the release path, so `release_paused` blocks them.
    pub fn execute_claim(env: Env, program_id: String, claim_id: u64, recipient: Address) {
        if Self::check_paused(&env, symbol_short!("release")) {
            panic!("Funds Paused");
        }
        claim_period::execute_claim(&env, &program_id, claim_id, &recipient)
    }

    /// Cancel a pending claim and return its reserved amount to escrow.
    ///
    /// Claim cancellation is a refund-path operation, so `refund_paused`
    /// blocks it independently of lock and release operations.
    pub fn cancel_claim(env: Env, program_id: String, claim_id: u64, admin: Address) {
        if Self::check_paused(&env, symbol_short!("refund")) {
            panic!("Funds Paused");
        }
        claim_period::cancel_claim(&env, &program_id, claim_id, &admin)
    }

    /// Retrieve a stored claim record by program and claim id.
    pub fn get_claim(env: Env, program_id: String, claim_id: u64) -> claim_period::ClaimRecord {
        claim_period::get_claim(&env, &program_id, claim_id)
    }

    /// Set the default claim window used by off-chain workflows.
    pub fn set_claim_window(env: Env, admin: Address, window_seconds: u64) {
        claim_period::set_claim_window(&env, &admin, window_seconds)
    }

    /// Return the configured default claim window duration in seconds.
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

        env.storage().instance().set(&DataKey::Dispute, &record);

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

        env.storage().instance().set(&DataKey::Dispute, &record);

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

    /// Get reputation metrics for the current program.
    /// Computes reputation based on schedules, payouts, and funds.
    /// Returns zero overall_score_bps if any releases are overdue (penalty for missed milestones).
    pub fn get_program_reputation(env: Env) -> ProgramReputation {
        let program_data: Option<ProgramData> = env.storage().instance().get(&PROGRAM_DATA);

        if program_data.is_none() {
            // Return zero reputation for uninitialized program
            return ProgramReputation {
                total_payouts: 0,
                total_scheduled: 0,
                completed_releases: 0,
                pending_releases: 0,
                overdue_releases: 0,
                dispute_count: 0,
                refund_count: 0,
                total_funds_locked: 0,
                total_funds_distributed: 0,
                completion_rate_bps: 10_000,
                payout_fulfillment_rate_bps: 10_000,
                overall_score_bps: 10_000,
            };
        }

        let program_data = program_data.unwrap();
        let schedules: soroban_sdk::Vec<ProgramReleaseSchedule> = env
            .storage()
            .instance()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));

        let now = env.ledger().timestamp();

        // Count schedule states
        let mut total_scheduled: u32 = 0;
        let mut completed_releases: u32 = 0;
        let mut pending_releases: u32 = 0;
        let mut overdue_releases: u32 = 0;

        for schedule in schedules.iter() {
            total_scheduled = total_scheduled.saturating_add(1);
            if schedule.released {
                completed_releases = completed_releases.saturating_add(1);
            } else {
                // Not yet released
                pending_releases = pending_releases.saturating_add(1);
                // Check if also overdue (past deadline but not released)
                if schedule.release_timestamp <= now {
                    overdue_releases = overdue_releases.saturating_add(1);
                }
            }
        }

        // Compute distributed funds from payout history
        let mut total_funds_distributed: i128 = 0;
        for payout in program_data.payout_history.iter() {
            total_funds_distributed = total_funds_distributed.saturating_add(payout.amount);
        }

        let total_payouts = program_data.payout_history.len() as u32;
        let total_funds_locked = program_data.total_funds;

        // Compute completion_rate_bps
        let completion_rate_bps = if total_scheduled == 0 {
            10_000 // Default to perfect if no schedules
        } else {
            let rate = (completed_releases as u64)
                .saturating_mul(10_000)
                .saturating_div(total_scheduled as u64);
            (rate.min(10_000)) as u32
        };

        // Compute payout_fulfillment_rate_bps
        let payout_fulfillment_rate_bps = if total_funds_locked == 0 {
            10_000 // Default to perfect if no funds locked
        } else {
            let rate = total_funds_distributed
                .saturating_mul(10_000)
                .saturating_div(total_funds_locked);
            (rate.min(10_000)) as u32
        };

        // Compute overall_score_bps: 0 if overdue releases exist, else weighted average
        let overall_score_bps = if overdue_releases > 0 {
            0 // Reputation penalty: any overdue release results in zero overall score
        } else {
            let weighted = (completion_rate_bps as u64)
                .saturating_mul(60)
                .saturating_add((payout_fulfillment_rate_bps as u64).saturating_mul(40))
                .saturating_div(100);
            (weighted.min(10_000)) as u32
        };

        ProgramReputation {
            total_payouts,
            total_scheduled,
            completed_releases,
            pending_releases,
            overdue_releases,
            dispute_count: 0,
            refund_count: 0,
            total_funds_locked,
            total_funds_distributed,
            completion_rate_bps,
            payout_fulfillment_rate_bps,
            overall_score_bps,
        }
    }
}

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_archival;
#[cfg(test)]
mod test_batch_operations;

#[cfg(test)]
mod test_draft_state;

#[cfg(test)]
mod test_pause;

#[cfg(test)]
#[cfg(any())]
mod rbac_tests;
