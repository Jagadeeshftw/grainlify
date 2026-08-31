//! Domain types, event symbols, storage keys, and constants for the program
//! escrow contract.
//!
//! Extracted from `lib.rs` to keep the root module focused on the
//! `#[contractimpl]` block while preserving the exact same public API.

use soroban_sdk::{contracterror, contracttype, symbol_short, Address, Env, String, Symbol, Vec};
use grainlify_core::CorrelationId;


// Event types
pub const PROGRAM_INITIALIZED: Symbol = symbol_short!("PrgInit");
pub const FUNDS_LOCKED: Symbol = symbol_short!("FndsLock");
pub const BATCH_FUNDS_LOCKED: Symbol = symbol_short!("BatLck");
pub const BATCH_FUNDS_RELEASED: Symbol = symbol_short!("BatRel");
pub const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
pub const PAYOUT: Symbol = symbol_short!("Payout");
pub const PROGRAM_PUBLISHED: Symbol = symbol_short!("PrgPub");
pub const EVENT_VERSION_V2: u32 = 2;
pub const PAUSE_STATE_CHANGED: Symbol = symbol_short!("PauseSt");
pub const PAUSE_STATE_CHANGED_V2: Symbol = symbol_short!("PauseStV2");
pub const AUTO_UNPAUSE: Symbol = symbol_short!("AutoUnpse");
pub const MAINTENANCE_MODE_CHANGED: Symbol = symbol_short!("MaintSt");
pub const PROGRAM_RISK_FLAGS_UPDATED: Symbol = symbol_short!("pr_risk");
pub const PROGRAM_REGISTRY: Symbol = symbol_short!("ProgReg");
pub const PROGRAM_REGISTERED: Symbol = symbol_short!("ProgRgd");
pub const RELEASE_SCHEDULED: Symbol = symbol_short!("RelSched");
pub const SCHEDULE_RELEASED: Symbol = symbol_short!("SchRel");
pub const PROGRAM_DELEGATE_SET: Symbol = symbol_short!("PrgDlgS");
pub const PROGRAM_DELEGATE_REVOKED: Symbol = symbol_short!("PrgDlgR");
pub const PROGRAM_METADATA_UPDATED: Symbol = symbol_short!("PrgMeta");
pub const ADMIN_PROPOSED: Symbol = symbol_short!("AdmProp");
pub const ADMIN_ACCEPTED: Symbol = symbol_short!("AdmAcc");
pub const ADMIN_ROTATION_CANCELLED: Symbol = symbol_short!("AdmCanc");
pub const CONTROLLER_PROPOSED: Symbol = symbol_short!("CtrlProp");
pub const CONTROLLER_ACCEPTED: Symbol = symbol_short!("CtrlAcc");
pub const CONTROLLER_ROTATION_CANCELLED: Symbol = symbol_short!("CtrlCanc");
pub const PRICE_UPDATED: Symbol = symbol_short!("PriceUpd");
pub const DYNAMIC_PRICING_CONFIG_UPDATED: Symbol = symbol_short!("DynPricCg");

// Storage keys
pub const PROGRAM_DATA: Symbol = symbol_short!("ProgData");
pub const RECEIPT_ID: Symbol = symbol_short!("RcptID");
pub const SCHEDULES: Symbol = symbol_short!("Scheds");
pub const RELEASE_HISTORY: Symbol = symbol_short!("RelHist");
pub const NEXT_SCHEDULE_ID: Symbol = symbol_short!("NxtSched");
pub const PROGRAM_INDEX: Symbol = symbol_short!("ProgIdx");
pub const AUTH_KEY_INDEX: Symbol = symbol_short!("AuthIdx");
pub const FEE_CONFIG: Symbol = symbol_short!("FeeCfg");
pub const FEE_COLLECTED: Symbol = symbol_short!("FeeCol");
/// Event symbol for insurance-reserve withdrawal audit events.
pub const INSURANCE_RESERVE_WITHDRAWN: Symbol = insurance_reserve::INSURANCE_RESERVE_WITHDRAWN;
/// Storage key for the set of consumed idempotency keys (batch payout).
pub const PAYOUT_IDEM_KEYS: Symbol = symbol_short!("PayIdem");
/// Event symbol emitted when a batch_payout replay is detected.
pub const BATCH_PAYOUT_REPLAYED: Symbol = symbol_short!("BatPayRp");
pub const TOKEN_ALLOWLIST_V2: Symbol = symbol_short!("TknAlw2");
pub const FOT_ROUTER_SET: Symbol = symbol_short!("FotRtSet");
pub const FOT_ROUTER_CLEARED: Symbol = symbol_short!("FotRtClr");
pub const EPOCH_SNAPSHOTS: Symbol = symbol_short!("EpSnap");
pub const NEXT_EPOCH_ID: Symbol = symbol_short!("NxtEpID");

// Fee rate is stored in basis points (1 basis point = 0.01%)
// Example: 100 basis points = 1%, 1000 basis points = 10%
pub const BASIS_POINTS: i128 = 10_000;
pub const MAX_FEE_RATE: i128 = 1_000; // Maximum 10% fee

/// Bitmask flag for [`FeeConfig::fee_waivers`]: skip fees for [`PayoutType::Single`] payouts.
pub const FEE_WAIVER_SINGLE: u32 = 1 << 0;
/// Bitmask flag for [`FeeConfig::fee_waivers`]: skip fees for [`PayoutType::Batch`] payouts.
pub const FEE_WAIVER_BATCH: u32 = 1 << 1;

pub const RISK_FLAG_HIGH_RISK: u32 = 1 << 0;
pub const RISK_FLAG_UNDER_REVIEW: u32 = 1 << 1;
pub const RISK_FLAG_RESTRICTED: u32 = 1 << 2;
pub const RISK_FLAG_DEPRECATED: u32 = 1 << 3;
pub const DELEGATE_METADATA_UPDATE_INTERVAL: u64 = 60; // 1 minute
pub const MAX_PROGRAM_METADATA_CUSTOM_FIELDS: u32 = 10;

pub const DELEGATE_PERMISSION_RELEASE: u32 = 1 << 0;
pub const DELEGATE_PERMISSION_REFUND: u32 = 1 << 1;
/// # DELEGATE_PERMISSION_UPDATE_META — low-privilege metadata write permission
///
/// ## Purpose
/// Allows a delegate to call `update_program_metadata` / `update_program_metadata_by`
/// without granting any financial power (no release, no refund).
///
/// ## Griefing / DOS vector
/// Because metadata is stored in instance storage and every write extends the
/// entry's TTL, a delegate holding *only* this bit can inflate the program
/// owner's storage rent indefinitely:
///
/// ```text
/// loop {
///     contract.update_program_metadata_by(program_id, delegate, huge_metadata);
/// }
/// ```
///
/// Each call costs ledger fees paid by the *caller* but also charges an
/// incremental XDR-size fee to the *contract instance* (billed to the
/// program owner's funded account).  Repeated writes with large
/// `custom_fields` vectors can grow instance storage costs without bound.
///
/// ## Mitigations applied in this contract
/// 1. **Rate limit** — Delegate-invoked metadata writes are capped at
///    `DELEGATE_META_MAX_OPS_PER_WINDOW` calls per `DELEGATE_META_RATE_LIMIT_WINDOW`
///    seconds (default: 10 per hour per program).  Admin / owner writes bypass
///    this limit.  State is tracked in `DataKey::DelegateMetaRateLimit(program_id)`.
///
/// 2. **`custom_fields` cap** — `ProgramMetadata::custom_fields` is bounded to
///    `MAX_CUSTOM_FIELDS` entries, and each key/value string is limited to
///    `MAX_CUSTOM_FIELD_KEY_LEN` / `MAX_CUSTOM_FIELD_VALUE_LEN` bytes,
///    preventing unbounded storage growth even within the rate-limit window.
///
/// ## Security assumptions
/// - The rate-limit state lives in instance storage (same TTL as the contract).
///   A delegate cannot clear it without admin access.
/// - The admin / owner can call `update_program_metadata` unlimited times;
///   this is intentional because they pay for their own actions and are
///   considered trusted parties.
pub const DELEGATE_PERMISSION_UPDATE_META: u32 = 1 << 2;
pub const DELEGATE_PERMISSION_MASK: u32 =
    DELEGATE_PERMISSION_RELEASE | DELEGATE_PERMISSION_REFUND | DELEGATE_PERMISSION_UPDATE_META;

// Role management constants for deterministic behavior
pub const ROLE_MANAGEMENT_SCHEMA_VERSION_V1: u32 = 1;
pub const MAX_ROLE_TRANSITION_PERIOD: u64 = 30 * 24 * 60 * 60; // 30 days in seconds
pub const PAUSE_REASON_MAX_LEN: u32 = 256;

/// Deterministic role transition state for upgrade-safe storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleTransitionState {
    /// Address proposing the role change
    pub proposer: Address,
    /// Address being proposed for the role
    pub proposed_role: Address,
    /// Ledger timestamp when proposal was created
    pub proposed_at: u64,
    /// Deadline for accepting the role (for deterministic expiration)
    pub deadline: u64,
    /// Nonce for replay protection
    pub nonce: u64,
}

/// Upgrade-safe role management configuration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleManagementConfig {
    /// Whether role rotations are currently enabled
    pub rotation_enabled: bool,
    /// Maximum transition period in seconds
    pub max_transition_period: u64,
    /// Whether emergency mode can block rotations
    pub emergency_blocks_rotations: bool,
}

impl RoleManagementConfig {
    pub fn default(_env: &Env) -> Self {
        Self {
            rotation_enabled: true,
            max_transition_period: MAX_ROLE_TRANSITION_PERIOD,
            emergency_blocks_rotations: true,
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,    // Fee rate for lock operations (basis points)
    pub payout_fee_rate: i128,  // Fee rate for each payout (basis points of gross payout)
    pub lock_fixed_fee: i128,   // Flat fee on lock (token units), capped to lock amount
    pub payout_fixed_fee: i128, // Flat fee per payout (token units), capped to gross payout
    pub fee_recipient: Address, // Address to receive fees
    pub fee_enabled: bool,      // Global fee enable/disable flag
    /// Per-PayoutType fee waiver bitmask.  Set bits suppress fee deduction for that
    /// payout variant regardless of `fee_enabled`.
    ///   bit 0 (`FEE_WAIVER_SINGLE`): waive fees for `PayoutType::Single`
    ///   bit 1 (`FEE_WAIVER_BATCH`):  waive fees for `PayoutType::Batch(_)`
    pub fee_waivers: u32,
    /// Basis-point share of each collected fee that is carved out into the
    /// on-chain insurance reserve instead of being forwarded to `fee_recipient`.
    ///
    /// Range: `0` (disabled, default) – `MAX_FEE_RATE` (10 %).
    /// The carve-out is applied *after* the fee is computed:
    ///
    /// ```text
    /// total_fee      = combined_fee_amount(gross, rate, fixed, enabled)
    /// reserve_share  = ceil(total_fee * insurance_reserve_bps / BASIS_POINTS)
    /// recipient_share = total_fee - reserve_share
    /// ```
    ///
    /// Invariant: `reserve_share + recipient_share == total_fee` (no leakage).
    pub insurance_reserve_bps: u32,
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

/// Emitted by `withdraw_insurance_reserve` (admin-gated).
///
/// Provides a full audit trail: who initiated the withdrawal, where funds
/// went, how much was in the reserve before and after, and the ledger
/// timestamp for cross-reference with the on-chain ledger sequence.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsuranceReserveWithdrawnEvent {
    pub version: u32,
    /// Admin address that authorised the withdrawal.
    pub admin: Address,
    /// Destination address that received the reserve funds.
    pub target: Address,
    /// Amount transferred out of the reserve.
    pub amount: i128,
    /// Reserve balance *before* this withdrawal.
    pub balance_before: i128,
    /// Reserve balance *after* this withdrawal (always 0 when `amount == balance_before`).
    pub balance_after: i128,
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
    /// Internal monitoring analytics.
    ///
    /// **WARNING: Naming Collision**
    /// This `Analytics` struct tracks operational metrics (`operation_count`, `unique_users`, etc.)
    /// and is completely incompatible with the top-level `Analytics` struct (which tracks
    /// financial totals like `total_locked`). 
    /// SDK authors and indexers must not conflate the two.
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
pub struct PayoutIdempotencyKey {
    pub key: String,             // Unique idempotency key provided by caller
    pub program_id: String,      // Program this payout belongs to
    pub payout_type: PayoutType, // Single or batch payout
    pub timestamp: u64,          // When the payout was executed
    // For single payouts
    pub recipient: Option<Address>, // Single payout recipient (None for batch)
    pub amount: Option<i128>,       // Single payout amount (None for batch)
    // For batch payouts
    pub recipients: Option<Vec<Address>>, // Batch payout recipients (None for single)
    pub amounts: Option<Vec<i128>>,       // Batch payout amounts (None for single)
    pub total_amount: i128,               // Total payout amount (for both single and batch)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayoutType {
    Single,
    Batch(u32), // Batch index (for batch payouts, stores the recipient index)
}

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
    /// Optional idempotency key for auditing.
    pub idempotency_key: Option<String>,
    /// Optional correlation identifier linking this event across multi-contract workflows.
    pub correlation_id: Option<CorrelationId>,
}

/// Emitted when a `batch_payout_idempotent` call is rejected because the
/// supplied idempotency key was already consumed by a prior successful payout.
/// Auditors can use this event to confirm that no double-payment occurred.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchPayoutReplayedEvent {
    pub version: u32,
    pub program_id: String,
    /// The idempotency key that was replayed.
    pub idempotency_key: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutEvent {
    pub version: u32,
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub remaining_balance: i128,
    /// Optional correlation identifier linking this event across multi-contract workflows.
    pub correlation_id: Option<CorrelationId>,
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
    /// Optional correlation identifier linking this event across multi-contract workflows.
    pub correlation_id: Option<CorrelationId>,
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
    /// Optional correlation identifier linking this event across multi-contract workflows.
    pub correlation_id: Option<CorrelationId>,
}

/// Summary event emitted once per `trigger_program_releases` invocation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleTriggerSummaryEvent {
    pub version: u32,
    pub program_id: String,
    pub triggered_at: u64,
    /// Number of schedules successfully released this run.
    pub released_count: u32,
    /// Number of schedules skipped due to insufficient contract balance.
    pub skipped_count: u32,
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
    pub delegate: Address,
    pub revoked_by: Address,
    pub timestamp: u64,
    /// `true` when revoked via `emergency_revoke_delegate`; `false` for normal revocation.
    pub emergency: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramMetadataUpdatedEvent {
    pub version: u32,
    pub program_id: String,
    pub updated_by: Address,
    pub timestamp: u64,
}

/// Emitted when a new admin is proposed (two-step rotation, step 1).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminProposedEvent {
    pub version: u32,
    pub proposed_by: Address,
    pub proposed_admin: Address,
    pub timestamp: u64,
}

/// Emitted when the proposed admin accepts and becomes the new admin (step 2).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminAcceptedEvent {
    pub version: u32,
    pub previous_admin: Address,
    pub new_admin: Address,
    pub timestamp: u64,
}

/// Emitted when a pending admin rotation is cancelled.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRotationCancelledEvent {
    pub version: u32,
    pub cancelled_by: Address,
    pub timestamp: u64,
}

/// Emitted when a new controller (authorized_payout_key) is proposed for a program (step 1).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerProposedEvent {
    pub version: u32,
    pub program_id: String,
    pub proposed_by: Address,
    pub proposed_controller: Address,
    pub timestamp: u64,
}

/// Emitted when the proposed controller accepts (step 2).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerAcceptedEvent {
    pub version: u32,
    pub program_id: String,
    pub previous_controller: Address,
    pub new_controller: Address,
    pub timestamp: u64,
}

/// Emitted when a pending controller rotation is cancelled.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerRotationCancelledEvent {
    pub version: u32,
    pub program_id: String,
    pub cancelled_by: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramPublishedEvent {
    pub version: u32,
    pub program_id: String,
    pub publisher: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramMetadataField {
    pub key: soroban_sdk::String,
    pub value: soroban_sdk::String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramMetadata {
    pub program_name: Option<soroban_sdk::String>,
    pub program_type: Option<soroban_sdk::String>,
    pub ecosystem: Option<soroban_sdk::String>,
    pub tags: soroban_sdk::Vec<soroban_sdk::String>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
    pub custom_fields: soroban_sdk::Vec<ProgramMetadataField>,
}

impl ProgramMetadata {
    pub fn empty(env: &soroban_sdk::Env) -> Self {
        Self {
            program_name: None,
            program_type: None,
            ecosystem: None,
            tags: soroban_sdk::Vec::new(env),
            start_date: None,
            end_date: None,
            custom_fields: soroban_sdk::Vec::new(env),
        }
    }
}

/// Validate `custom_fields` size/length limits.
///
/// Enforced identically by both `init_program_with_metadata` and
/// `update_program_metadata` so that metadata accepted at creation is never
/// rejected on update (or vice versa).
///
/// # Limits
/// | Constraint | Constant | Value |
/// |---|---|---|
/// | Max entries | `MAX_CUSTOM_FIELDS` | 20 |
/// | Max key length | `MAX_CUSTOM_FIELD_KEY_LEN` | 64 bytes (byte-based) |
/// | Max value length | `MAX_CUSTOM_FIELD_VALUE_LEN` | 256 bytes (byte-based) |
/// | Aggregate payload | `MAX_METADATA_AGGREGATE_BYTES` | 10 240 bytes |
///
/// Limits are **byte-based** (using `String::len()` which returns UTF-8 byte
/// length).  Soroban `String::from_str` rejects invalid UTF-8 at construction
/// time, so byte-based and character-based limits coincide for valid strings.
///
/// Updates **replace** the entire metadata; they do not merge with existing
/// values.
///
/// # Panics
/// - `"CustomFieldsLimitExceeded"` if `custom_fields.len() > MAX_CUSTOM_FIELDS`.
/// - `"CustomFieldKeyTooLong"` if any key exceeds `MAX_CUSTOM_FIELD_KEY_LEN` bytes.
/// - `"CustomFieldValueTooLong"` if any value exceeds `MAX_CUSTOM_FIELD_VALUE_LEN` bytes.
/// - `"MetadataAggregateSizeExceeded"` if the sum of all key+value byte lengths
///   exceeds `MAX_METADATA_AGGREGATE_BYTES`.
pub fn validate_metadata_custom_fields(metadata: &ProgramMetadata) {
    let num_fields = metadata.custom_fields.len();
    if num_fields > MAX_CUSTOM_FIELDS {
        panic!("CustomFieldsLimitExceeded");
    }
    let mut aggregate: u32 = 0;
    for field in metadata.custom_fields.iter() {
        if field.key.len() > MAX_CUSTOM_FIELD_KEY_LEN {
            panic!("CustomFieldKeyTooLong");
        }
        if field.value.len() > MAX_CUSTOM_FIELD_VALUE_LEN {
            panic!("CustomFieldValueTooLong");
        }
        aggregate = aggregate
            .checked_add(field.key.len())
            .and_then(|s| s.checked_add(field.value.len()))
            .unwrap_or(u32::MAX);
    }
    if aggregate > MAX_METADATA_AGGREGATE_BYTES {
        panic!("MetadataAggregateSizeExceeded");
    }
}

/// Program lifecycle status.
///
/// Programs start in `Draft` state after `init_program` and transition to
/// `Active` after `publish_program` is called.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgramStatus {
    Draft,
    Active,
}

/// Per-program circuit breaker threshold configuration.
///
/// The circuit breaker protects against cascading failures by opening after
/// a configurable number of consecutive failures. Each program can have its
/// own threshold:
///
/// - **None**: Use global default threshold (3 failures)
/// - **Some(n)**: Use custom threshold (1-100 failures)
///
/// Large programs with many participants may need a higher threshold to
/// tolerate expected transient failures, while small programs may benefit
/// from a lower threshold for faster failure detection.
///
/// # Example
/// ```rust,ignore
/// // Set custom threshold for a large program
/// contract.set_program_circuit_breaker_threshold(&program_id, &Some(10u32));
///
/// // Reset to global default
/// contract.set_cb_threshold(&program_id, &None);
/// ```

// ─────────────────────────────────────────────────────────────────────────────
// FoT ROUTER TYPES
// ─────────────────────────────────────────────────────────────────────────────

/// Fee-on-transfer router configuration stored inside `ProgramData`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FotRouter {
    /// Address of the AMM / DEX router contract that exposes a `quote` function.
    pub router_contract: Address,
    /// Slippage tolerance in basis points (0 – 500, i.e. 0 – 5 %).
    pub slippage_bps: u32,
    /// Maximum gross-to-net multiplier for router quotes, in basis points over 10_000.
    ///
    /// For example, `15_000` permits a gross quote up to 1.5x the intended net.
    /// This bound prevents a compromised or misconfigured router from draining
    /// the program with an implausibly inflated quote.
    pub max_fot_multiplier_bps: u32,
}

/// Nullable wrapper for `FotRouter` stored inside `ProgramData`.
///
/// Soroban `contracttype` enums must be C-like (no `Option<T>` fields in
/// top-level struct that hold non-scalar types), so we use an explicit
/// two-variant enum instead of `Option<FotRouter>`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OptionalFotRouter {
    None,
    Some(FotRouter),
}

/// Event emitted when a FoT router is configured for the contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FotRouterSetEvent {
    pub version: u32,
    pub router_contract: Address,
    pub slippage_bps: u32,
    /// Configured upper-bound multiplier for gross router quotes, in basis points over 10_000.
    pub max_fot_multiplier_bps: u32,
    pub set_by: Address,
    pub timestamp: u64,
}

/// Event emitted when the FoT router configuration is cleared.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FotRouterClearedEvent {
    pub version: u32,
    pub set_by: Address,
    pub timestamp: u64,
}

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
    /// Lifecycle status of the program (`Draft` before `publish_program`, `Active` after).
    pub status: ProgramStatus,
    /// Optional per-program circuit breaker failure threshold.
    /// If set, overrides the global default (3) for this program.
    /// Must be between 1 and 100 inclusive when set.
    /// Stored as u32 because Soroban SDK does not support u8 in contracttype.
    pub circuit_breaker_threshold: Option<u32>,
    /// Optional FoT router configuration for fee-on-transfer token handling.
    pub fot_router: OptionalFotRouter,
}

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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramDelegateInfo {
    pub program_id: String,
    pub delegate: Option<Address>,
    pub permissions: u32,
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

// ─────────────────────────────────────────────────────────────────────────────
// CIRCUIT BREAKER THRESHOLD AUDIT EVENTS
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when the admin sets or updates the per-program circuit breaker threshold.
///
/// ### Topics
/// `(CB_THRESHOLD_SET, program_id)`
///
/// ### Security notes
/// - Only the admin can call `set_cb_threshold`.
/// - `previous_threshold` is `None` when no threshold was previously set.
/// - Emitted **after** the new value is persisted so the event reflects
///   the settled on-chain state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerThresholdSetEvent {
    pub version: u32,
    /// Program the threshold applies to.
    pub program_id: String,
    /// Previous threshold value (None = not set, uses global default of 3).
    pub previous_threshold: Option<u32>,
    /// New threshold value (None = reset to global default of 3).
    pub new_threshold: Option<u32>,
    /// Admin that made the change.
    pub set_by: Address,
    /// Ledger timestamp.
    pub timestamp: u64,
}

// ========================================================================
// Idempotency Key Types
// ========================================================================

/// Record of an idempotency key usage for payout operations.
///
/// Stores the outcome of a payout operation to ensure deterministic
/// responses on retry attempts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyRecord {
    /// The idempotency key that was used
    pub idempotency_key: String,
    /// Type of operation that was performed
    pub operation_type: Symbol,
    /// Whether the operation succeeded
    pub success: bool,
    /// Timestamp when the operation was first executed
    pub executed_at: u64,
    /// Address that executed the operation
    pub executor: Address,
    /// Program ID for which the operation was performed
    pub program_id: String,
    /// Total amount involved in the operation
    pub total_amount: i128,
    /// Number of recipients (for batch payouts)
    pub recipient_count: u32,
    /// Error code if the operation failed
    pub error_code: Option<u32>,
}

/// Event emitted when an idempotency key is first used successfully.
///
/// ### Topics
/// `(IDEMPOTENCY_KEY_USED, idempotency_key)`
///
/// ### Security notes
/// - Emitted **after** the operation succeeds so the event reflects
///   the completed state.
/// - Contains operation details for audit trail without exposing
///   sensitive recipient data.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyKeyUsedEvent {
    pub version: u32,
    pub idempotency_key: String,
    pub operation_type: Symbol,
    pub program_id: String,
    pub total_amount: i128,
    pub recipient_count: u32,
    pub executor: Address,
    pub executed_at: u64,
}

/// Event emitted when a retry attempt is made with a used idempotency key.
///
/// ### Topics
/// `(IDEMPOTENCY_KEY_USED, idempotency_key)`
///
/// ### Security notes
/// - Emitted **before** any state changes to prevent duplicate operations.
/// - Contains the original result for deterministic client responses.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencyKeyRetryEvent {
    pub version: u32,
    pub idempotency_key: String,
    pub original_success: bool,
    pub original_executed_at: u64,
    pub original_executor: Address,
    pub retry_attempt_at: u64,
    pub retry_by: Address,
}

/// Emitted once during contract initialization to record the idempotency
/// storage schema version for upgrade-safety tracking.
///
/// ### Topics
/// `(IDEMPOTENCY_SCHEMA,)`
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdempotencySchemaVersionSet {
    pub version: u32,
    /// Schema version written to instance storage.
    pub schema_version: u32,
    /// Ledger timestamp.
    pub timestamp: u64,
}

// Constants for idempotency key validation
pub const IDEMPOTENCY_KEY_MAX_LENGTH: u32 = 256;
// ── Multisig threshold ────────────────────────────────────────────────────────
pub const ADMIN_OP_EXPIRY_LEDGERS: u32 = 17_280;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigThresholdConfig {
    pub signers: soroban_sdk::Vec<Address>,
    pub required_approvals: u32,
    pub high_value_threshold: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminOpKind { UpdateFeeConfig, UpdateMultisigConfig, EmergencyWithdraw }

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdminOp {
    pub kind: AdminOpKind,
    pub value: i128,
    pub proposed_by: Address,
    pub proposed_at: u32,
    pub expires_at: u32,
    pub approvals: soroban_sdk::Vec<Address>,
    pub payload_hash: soroban_sdk::Bytes,
}

pub const ADMIN_OP_PROPOSED: Symbol = symbol_short!("AdmProp");
pub const ADMIN_OP_APPROVED: Symbol = symbol_short!("AdmAppr");
pub const ADMIN_OP_EXECUTED: Symbol = symbol_short!("AdmExec");
pub const ADMIN_OP_EXPIRED:  Symbol = symbol_short!("AdmExp");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminOpProposedEvent { pub version: u32, pub kind: AdminOpKind, pub proposed_by: Address, pub expires_at: u32, pub required_approvals: u32 }
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminOpApprovedEvent { pub version: u32, pub kind: AdminOpKind, pub approved_by: Address, pub approvals_so_far: u32, pub required_approvals: u32 }
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminOpExecutedEvent { pub version: u32, pub kind: AdminOpKind, pub executed_by: Address }

pub const IDEMPOTENCY_SCHEMA_VERSION_V1: u32 = 1;

// Event symbols for dispute lifecycle
pub const DISPUTE_OPENED: Symbol = symbol_short!("DspOpen");
pub const DISPUTE_RESOLVED: Symbol = symbol_short!("DspRslv");
pub const SCHEDULE_SCHEMA: Symbol = symbol_short!("SchSch");

// Event symbols for spend-limit threshold lifecycle
pub const SPEND_LIMIT_SET: Symbol = symbol_short!("SpLimSet");
pub const SPEND_LIMIT_EXCEEDED: Symbol = symbol_short!("SpLimExc");
pub const SPEND_LIMIT_SCHEMA: Symbol = symbol_short!("SpLimSch");
pub const CB_THRESHOLD_SET: Symbol = symbol_short!("CbThrSet");
pub const IDEMPOTENCY_SCHEMA: Symbol = symbol_short!("IdempSch");
pub const IDEMPOTENCY_KEY_USED: Symbol = symbol_short!("IdempUsed");

/// Validate idempotency key format and constraints.
///
/// Allowed characters: ASCII letters, digits, hyphen (`-`), and underscore (`_`).
/// Keys must be between 1 and 256 bytes long.
pub fn validate_idempotency_key(key: &str) -> Result<(), BatchError> {
    let key_len = key.len();
    if key_len < MIN_IDEMPOTENCY_KEY_LENGTH as usize || key_len > MAX_IDEMPOTENCY_KEY_LENGTH as usize {
        return Err(BatchError::IdempotencyKeyInvalid);
    }

    let bytes = key.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let valid_char = (b >= b'a' && b <= b'z')
            || (b >= b'A' && b <= b'Z')
            || (b >= b'0' && b <= b'9')
            || b == b'-'
            || b == b'_';
        if !valid_char {
            return Err(BatchError::IdempotencyKeyInvalid);
        }
        i += 1;
    }

    Ok(())
}

pub const ROLE_MANAGEMENT_SCHEMA: Symbol = symbol_short!("RoleMgmt");

// Event symbol for per-window program spend limit enforcement
pub const PROG_SPEND_LIMIT: Symbol = symbol_short!("prg_lim");

// ─────────────────────────────────────────────────────────────────────────────
// PER-WINDOW SPENDING LIMIT TYPES (Issue #25)
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for a per-program rolling-window spend limit.
///
/// Stored under `DataKey::SpendingConfig(program_id)`.
///
/// ### Fields
/// - `window_size`  – Rolling window duration in seconds (must be > 0).
/// - `max_amount`   – Maximum total amount releasable within one window.
/// - `enabled`      – When `false` the config is persisted but not enforced.
///
/// ### Upgrade safety
/// If new fields are added in a future version, the storage key version in
/// `DataKey` must be incremented and a migration path provided.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramSpendingConfig {
    /// Rolling window duration in seconds (must be > 0).
    pub window_size: u64,
    /// Maximum total amount releasable within one window.
    pub max_amount: i128,
    /// When `false` the config is persisted but not enforced.
    pub enabled: bool,
}

/// Mutable runtime state for a per-program rolling-window spend limit.
///
/// Stored under `DataKey::SpendingState(program_id)`.
///
/// ### Fields
/// - `window_start`     – Ledger timestamp of the current window's start.
/// - `amount_released`  – Cumulative amount released within the current window.
///
/// ### Atomicity guarantee
/// Both fields are written together in a single `env.storage().persistent().set()`
/// call so the state is always consistent.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramSpendingState {
    /// Ledger timestamp of the current window's start.
    pub window_start: u64,
    /// Cumulative amount released within the current window.
    pub amount_released: i128,
}

// ─────────────────────────────────────────────────────────────────────────────
// TOKEN ALLOWLIST TYPES & EVENTS
// ─────────────────────────────────────────────────────────────────────────────

/// An entry in the token allowlist that stores both the token address and its
/// decimal precision.
///
/// Storing decimals at allowlist-add time avoids a cross-contract call on every
/// payout and ensures the normalization factor is admin-controlled and auditable.
///
/// # Decimal Normalization
///
/// All payout `amount` parameters are expressed in **base units** (the smallest
/// indivisible unit of the token, e.g. 1 = 0.000001 USDC for a 6-decimal token).
/// The contract does **not** re-scale amounts — callers must supply amounts
/// already denominated in the token's own base units.
///
/// The `decimals` field is stored for off-chain tooling and event emission so
/// that indexers can display human-readable values without additional RPC calls.
///
/// # Upgrade Safety
///
/// Stored under `TOKEN_ALLOWLIST_V2`. Legacy entries under
/// `DataKey::TokenAllowlist` (plain `Vec<Address>`) are still readable via
/// `get_allowed_tokens()` for backward compatibility.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllowedTokenEntry {
    /// Token contract address.
    pub token: Address,
    /// Number of decimal places for this token (e.g. 6 for USDC, 7 for XLM).
    /// Range: 0–18.
    pub decimals: u32,
}

/// Event emitted when the token allowlist is updated (token added or removed).
///
/// ### Topics
/// `(TOKEN_ALLOWLIST_UPDATED,)`
///
/// ### Security notes
/// - Only the admin can mutate the allowlist.
/// - `added = true` means the token was added; `false` means removed.
/// - Emitted **after** storage is written so the event reflects settled state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenAllowlistUpdatedEvent {
    pub version: u32,
    /// Token contract address that was added or removed.
    pub token: Address,
    /// `true` = added to allowlist, `false` = removed from allowlist.
    pub added: bool,
    /// Admin that performed the update.
    pub updated_by: Address,
    /// Ledger timestamp.
    pub timestamp: u64,
    /// Decimal precision stored for this token (0 when `added = false`).
    pub decimals: u32,
}

/// Event emitted when a program initialization is rejected because the
/// requested token is not on the allowlist.
///
/// ### Topics
/// `(TOKEN_REJECTED,)`
///
/// ### Security notes
/// - Emitted **before** any state mutation so no partial writes occur.
/// - Allows off-chain monitors to detect misconfigured program setups.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenRejectedEvent {
    pub version: u32,
    /// Token that was rejected.
    pub token: Address,
    /// Program ID that attempted to use the rejected token.
    pub program_id: String,
    /// Ledger timestamp.
    pub timestamp: u64,
}

/// Emitted once during contract initialization to record the token-allowlist
/// storage schema version for upgrade-safety tracking.
///
/// ### Topics
/// `(TOKEN_ALLOWLIST_SCHEMA,)`
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenAllowlistSchemaVersionSet {
    pub version: u32,
    /// Schema version written to instance storage.
    pub schema_version: u32,
    /// Ledger timestamp.
    pub timestamp: u64,
}

/// Emitted whenever a token's immutable decimal scale is first configured via
/// `add_allowed_token_with_decimals`.
///
/// ### Topics
/// `(TOKEN_DECIMALS_CONFIGURED,)`
///
/// ### Security notes
/// - The configured scale is immutable; this event marks the one write.
/// - `reported_decimals` is the token contract's live `decimals()` view when it
///   exposes one, recorded for cross-checking against `configured_decimals`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenDecimalsConfiguredEvent {
    pub version: u32,
    /// Token contract address that was configured.
    pub token: Address,
    /// Immutable application-level decimal scale recorded for this token.
    pub configured_decimals: u32,
    /// Live `decimals()` reported by the token contract, if it implements one.
    pub reported_decimals: Option<u32>,
    /// Admin that performed the configuration.
    pub configured_by: Address,
    /// Ledger timestamp.
    pub timestamp: u64,
}

/// Emitted when the token contract's live `decimals()` view disagrees with the
/// admin-configured scale at allowlist-add time.
///
/// ### Topics
/// `(TOKEN_DECIMALS_MISMATCH,)`
///
/// ### Security notes
/// - Non-blocking: some supported tokens use an application-defined accounting
///   scale that legitimately differs from their on-chain `decimals()`.
/// - Surfaced so indexers and operational monitoring can flag misconfiguration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenDecimalsMismatchEvent {
    pub version: u32,
    /// Token contract address with the mismatch.
    pub token: Address,
    /// Scale the admin configured for this token.
    pub configured_decimals: u32,
    /// Scale the token contract reports from its `decimals()` view.
    pub reported_decimals: u32,
    /// Admin that performed the configuration.
    pub configured_by: Address,
    /// Ledger timestamp.
    pub timestamp: u64,
}

// Event symbols for token allowlist lifecycle
pub const TOKEN_ALLOWLIST_UPDATED: Symbol = symbol_short!("TkAllow");
pub const TOKEN_REJECTED: Symbol = symbol_short!("TkReject");
pub const TOKEN_ALLOWLIST_SCHEMA: Symbol = symbol_short!("TkAlSch");
pub const TOKEN_DECIMALS_CONFIGURED: Symbol = symbol_short!("TkDecCfg");
pub const TOKEN_DECIMALS_MISMATCH: Symbol = symbol_short!("TkDecMis");

/// Current token-allowlist storage schema version.
///
/// Increment whenever the allowlist storage layout changes in a breaking way.
pub const TOKEN_ALLOWLIST_SCHEMA_VERSION_V1: u32 = 1;

/// Maximum allowed token decimal places.
///
/// Tokens with more than 18 decimals are rejected at allowlist-add time.
/// This prevents overflow in normalization arithmetic.
pub const MAX_TOKEN_DECIMALS: u32 = 18;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String),                 // program_id -> ProgramData
    Admin,                           // Contract Admin
    MultisigConfig(String),          // program_id -> MultisigConfig
    SplitConfig(String),             // program_id -> SplitConfig (payout splits)
    PendingClaim(String, u64),       // (program_id, schedule_id) -> ClaimRecord
    ClaimWindow,                     // u64 seconds (global config)
    PauseFlags,                      // PauseFlags struct
    ProgramPauseFlags(String),       // program_id -> PauseFlags
    RateLimitConfig,                 // RateLimitConfig struct
    MaintenanceMode,                 // bool flag
    ProgramDependencies(String),     // program_id -> Vec<String>
    DependencyStatus(String),        // program_id -> DependencyStatus
    Dispute,                         // DisputeRecord (single active dispute per contract)
    PayoutIdempotency(String),       // idempotency_key -> PayoutIdempotencyKey
    HistoryPaginationConfig,         // HistoryPaginationConfig
    SpendLimitSchemaVersion,
    PauseSchemaVersion,
    TokenAllowlist,
    /// Per-token configured decimal scale, written once on allowlist add and
    /// cleared on removal. Keyed by token `Address`.
    TokenDecimals(Address),
    /// Dynamic pricing configuration
    DynamicPricingConfig,
    /// Dynamic pricing state
    PricingState,
    /// Demand metrics for dynamic pricing
    DemandMetrics,
    /// Supply metrics for dynamic pricing
    SupplyMetrics,
    /// Oracle data for dynamic pricing
    OracleData,
    /// Upgrade-safe schema version marker for token-allowlist storage.
    /// Written on init; increment when the allowlist storage layout changes.
    TokenAllowlistSchemaVersion,
    SpendingConfig(String),
    SpendingState(String),
    ReadOnlyMode,
    Metadata(String),
    /// Compressed metadata stored under `MetadataFieldKey` enum keys.
    /// Read path falls back to `Metadata(String)` for backwards compatibility.
    MetadataV2(String),
    RotationNonce(String),
    ReleaseTriggerSchemaVersion,
    ReentrancyGuard,
    IdempotencyKey(String),
    IdempotencySchemaVersion,
    BatchPayoutSchemaVersion,
    CircuitBreakerSchemaVersion,
    BatchReceipt(u64),
    PendingAdmin,
    /// Pending admin transition metadata used to invalidate replaced or expired proposals.
    PendingAdminTransition,
    /// Pending controller address for two-step controller rotation (step 1).
    PendingController(String),
    /// Upgrade-safe schema version marker for role management storage.
    /// Written on init; increment when role management layout changes.
    RoleManagementSchemaVersion,
    RoleManagementConfig,
    /// Lazy inverted index: (program_id, recipient) → Vec<PayoutRecord>.
    ///
    /// Written on first payout to a given recipient; never touched until then,
    /// so programs with no payouts pay zero cold-storage cost.
    /// Stored in persistent storage so it survives TTL-based ledger pruning.
    RecipientPayoutIndex(String, Address),
    /// Per-program rate-limit state for delegate-invoked metadata updates.
    ///
    /// Stored as `DelegateMetaRateLimitState` under instance storage.
    /// Keyed by program_id so each program has an independent rate-limit
    /// counter; a malicious delegate for one program cannot exhaust the
    /// budget of another.
    DelegateMetaRateLimit(String),
    /// On-chain insurance reserve balance for the contract (admin-gated withdrawals).
    InsuranceReserve,
    /// Per-program lifecycle status timeline (companion to `ProgramData::status`).
    LifecycleTimeline(String),
    /// Per-program access-signal marker used by RBAC/monitoring subsystems.
    ProgramAccessSignal(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// ANONYMIZATION TYPES (Issue #1291)
// ─────────────────────────────────────────────────────────────────────────────

/// Resolver address used to anonymize payout recipients for a program.
///
/// When set, the resolver acts as an intermediary: the contract pays the
/// resolver instead of the real recipient, and the resolver is responsible
/// for forwarding funds off-chain. This keeps recipient identities off-chain
/// while preserving on-chain auditability of total amounts.
///
/// ### Security notes
/// - Only the admin can set or update the resolver.
/// - The resolver address is stored per-program so different programs can
///   use different resolvers.
/// - Setting the resolver to `None` disables anonymization for that program.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnonymousResolver {
    /// The resolver address that receives anonymized payouts.
    pub resolver: Address,
    /// Admin that set this resolver.
    pub set_by: Address,
    /// Ledger timestamp when the resolver was last updated.
    pub updated_at: u64,
}

/// Event emitted when an anonymous resolver is set or updated for a program.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnonymousResolverSetEvent {
    pub version: u32,
    pub program_id: String,
    pub resolver: Address,
    pub set_by: Address,
    pub timestamp: u64,
}

/// Event emitted when an anonymous resolver is removed from a program.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnonymousResolverRemovedEvent {
    pub version: u32,
    pub program_id: String,
    pub removed_by: Address,
    pub timestamp: u64,
}

pub const ANONYMOUS_RESOLVER_SET: Symbol = symbol_short!("AnonRslvS");
pub const ANONYMOUS_RESOLVER_REMOVED: Symbol = symbol_short!("AnonRslvR");

/// Delegate info for a single program, returned by `query_program_delegates`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseFlags {
    pub lock_paused: bool,
    pub release_paused: bool,
    pub refund_paused: bool,
    pub pause_reason: Option<String>,
    pub paused_at: u64,
    /// Ledger timestamp after which lock_paused is automatically cleared (None = manual-only).
    pub lock_unpause_at: Option<u64>,
    /// Ledger timestamp after which release_paused is automatically cleared (None = manual-only).
    pub release_unpause_at: Option<u64>,
    /// Ledger timestamp after which refund_paused is automatically cleared (None = manual-only).
    pub refund_unpause_at: Option<u64>,
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
/// ### Fields
/// - `actor`: The address that triggered the pause state change (admin or authorized caller).
/// - `reason`: Optional human-readable reason string, bounded to 256 characters.
///
/// ### Security notes
/// - `previous_paused` is read from storage **before** the mutation so the
///   event accurately reflects the transition (old → new).
/// - `invariant_ok` is always `true` on-chain; a `false` value would indicate
///   a storage corruption bug.
/// - `reason` is bounded to [`PAUSE_REASON_MAX_LEN`] characters to prevent storage abuse.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseStateChangedV2 {
    pub version: u32,
    pub operation: Symbol,
    pub previous_paused: bool,
    pub paused: bool,
    /// The address that triggered the pause state change.
    pub actor: Address,
    /// Optional human-readable reason, bounded to 256 characters.
    pub reason: Option<String>,
    pub timestamp: u64,
    pub receipt_id: u64,
    /// Storage schema version for pause-related data (written at init).
    pub schema_version: u32,
}

/// Emitted when a pause mode is automatically cleared because its TTL expired.
///
/// ### Topics
/// `(AUTO_UNPAUSE, operation_symbol)`
///
/// ### Security notes
/// - `actor` is always "system" — triggered by guard logic, not a user call.
/// - Emitted at most once per mode per guard invocation (not per repeated call).
/// - Only emitted when `current_ledger_timestamp > unpause_at` (strictly greater).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoUnpauseEvent {
    pub version: u32,
    pub operation: Symbol,
    /// Always "system" — the auto-unpause was triggered by the guard, not an admin.
    pub actor: String,
    /// The TTL threshold that was exceeded.
    pub unpause_at: u64,
    /// The ledger timestamp at which auto-unpause was triggered.
    pub triggered_at: u64,
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

// ─────────────────────────────────────────────────────────────────────────────
// DELEGATE METADATA RATE LIMIT (DOS-resistance for DELEGATE_PERMISSION_UPDATE_META)
// ─────────────────────────────────────────────────────────────────────────────

/// Rolling-window counter for delegate-invoked metadata writes on a single program.
///
/// Stored under `DataKey::DelegateMetaRateLimit(program_id)` in instance storage.
/// Reset whenever the current ledger timestamp exceeds
/// `window_start + DELEGATE_META_RATE_LIMIT_WINDOW`.
///
/// # Storage cost
/// One entry per program that has ever had a delegate metadata update;
/// size is constant (two u64 words).  The entry is never deleted so the
/// TTL-extension cost is paid once per program.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegateMetaRateLimitState {
    /// Ledger timestamp (seconds) when the current window started.
    pub window_start: u64,
    /// Number of delegate-invoked metadata writes in the current window.
    pub count: u32,
}

/// Rolling-window duration for delegate metadata writes (seconds).
/// Default: 3 600 s (1 hour).
pub const DELEGATE_META_RATE_LIMIT_WINDOW: u64 = 3_600;

/// Maximum delegate metadata writes permitted within one `DELEGATE_META_RATE_LIMIT_WINDOW`.
/// Permits one update every ~6 minutes on average; enough for legitimate use
/// while making sustained spam economically costly.
pub const DELEGATE_META_MAX_OPS_PER_WINDOW: u32 = 10;

/// Maximum number of entries in `ProgramMetadata::custom_fields`.
/// Bounds on-chain storage regardless of who calls the update.
pub const MAX_CUSTOM_FIELDS: u32 = 20;

/// Maximum byte length of a `ProgramMetadataField` key.
pub const MAX_CUSTOM_FIELD_KEY_LEN: u32 = 64;

/// Maximum byte length of a `ProgramMetadataField` value.
pub const MAX_CUSTOM_FIELD_VALUE_LEN: u32 = 256;

/// Maximum aggregate byte size of all custom field keys and values combined.
/// Prevents unbounded storage growth from metadata payloads even when
/// individual field limits are respected.  Each custom field contributes
/// `key.len() + value.len()` bytes toward this ceiling.
pub const MAX_METADATA_AGGREGATE_BYTES: u32 = 10_240;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryPaginationConfig {
    pub max_limit: u32,
    pub schema_version: u32,
}

/// Current history pagination storage schema version.
///
/// Increment whenever `HistoryPaginationConfig` layout changes in a breaking way.
/// Written to instance storage during `init` so upgrade safety checks can
/// detect schema mismatches on legacy deployments.
pub const PAGINATION_SCHEMA_VERSION_V1: u32 = 1;

/// Top-level analytics for the program escrow.
/// 
/// **WARNING: Naming Collision**
/// This `Analytics` struct tracks financial metrics (`total_locked`, `total_released`, etc.) 
/// and is completely incompatible with the `Analytics` struct defined in the internal 
/// `monitoring` module (which tracks `operation_count`, `unique_users`, etc.). 
/// SDK authors and indexers must not conflate the two.
/// (Consider using an alias like `EscrowAnalytics` in off-chain code to avoid confusion).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Analytics {
    pub total_locked: i128,
    pub total_released: i128,
    pub total_payouts: u32,
    pub active_programs: u32,
    pub operation_count: u32,
}

/// A single recorded status transition within a program's lifecycle.
///
/// Each entry captures a transition from one [`ProgramStatus`] to another
/// at a specific ledger timestamp, enabling off-chain computation of
/// dwell times per status.
///
/// # Initial entry convention
/// The first transition for every program records `from_status: Draft` and
/// `to_status: Draft` — both sides are `Draft` because there is no special
/// "Created" or "Null" variant in [`ProgramStatus`].  The timestamp of this
/// entry marks the program's creation time.  Dwell time in Draft is computed
/// as `transitions[1].timestamp - transitions[0].timestamp`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusTransition {
    /// The status the program is transitioning **from**.
    ///
    /// For the initial creation entry this is `ProgramStatus::Draft` (same
    /// as `to_status`), indicating the program entered the lifecycle.
    pub from_status: ProgramStatus,
    /// The status the program is transitioning **to**.
    pub to_status: ProgramStatus,
    /// Ledger timestamp (seconds since Unix epoch) when the transition
    /// was recorded.  Sourced from `env.ledger().timestamp()`, which is
    /// deterministic across all Soroban validators.
    pub timestamp: u64,
}

/// On-chain record of all status transitions for a single program.
///
/// Stored under `DataKey::LifecycleTimeline(program_id)` as a companion
/// record alongside [`ProgramData`].  Because this is stored under its own
/// key, adding it does not change the existing [`Analytics`] field ordering
/// or [`ProgramData`] layout, preserving storage compatibility.
///
/// # Upgrade safety
/// If a future version needs to store additional per-transition metadata
/// (e.g. the caller address that triggered the transition), a new storage
/// key version should be introduced.  This struct is append-only in the
/// sense that new transitions are pushed to the end of the Vec.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramLifecycleTimeline {
    /// Ordered list of status transitions (oldest first).
    pub transitions: soroban_sdk::Vec<StatusTransition>,
}

/// Program reputation metrics tracking performance and reliability.
/// Includes counts of payouts and schedules, funds tracking, and performance scores in basis points.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReputation {
    /// Total number of payout records in history (includes dust; not used in `overall_score_bps`)
    pub total_payouts: u32,
    /// Payouts with amount >= [`REPUTATION_MIN_QUALIFYING_PAYOUT_AMOUNT`]
    pub qualified_payout_count: u32,
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
    /// Defaults to 0 if no funds locked, capped at 10_000.
    /// Value-weighted: dust payouts contribute proportionally to their size, not per-call.
    pub payout_fulfillment_rate_bps: u32,
    /// Overall reputation score in basis points (0-10_000)
    /// Weighted 60% schedule completion + 40% payout fulfillment.
    /// Returns 0 if any overdue releases exist (reputation penalty for overdue milestones).
    /// Resistant to dust spam on score: inflating `total_payouts` alone does not raise this field.
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
pub struct EpochSnapshot {
    pub created_at: u64,
    pub created_by: Address,
    pub schedules: soroban_sdk::Vec<ProgramReleaseSchedule>,
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
// ========================================================================
// Batch Receipt Types
// ========================================================================

pub const BATCH_RECEIPT_VERSION: u32 = 1;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchReceipt {
    pub version: u32,
    pub batch_id: u64,
    pub merkle_root: soroban_sdk::BytesN<32>,
    pub total_amount: i128,
    pub recipient_count: u32,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchReceiptKey {
    Receipt(u64),
    NextId,
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
    IdempotencyKeyConflict = 415,
    IdempotencyKeyInvalid = 416,
    InvalidMerkleRoot = 409,
    BatchReceiptNotFound = 414,
    InvalidPaginationLimit = 411,
    PaginationLimitExceeded = 412,
    InvalidPaginationOffset = 413,
    BatchTooLarge = 410,
}

pub const MAX_BATCH_SIZE: u32 = 100;
pub const DEFAULT_MAX_HISTORY_PAGE_LIMIT: u32 = 200;

/// Current storage schema version constant (upgrade-safe marker).
/// Bumped to 2 after ProgramData field reordering (schema_version: 2).
pub const STORAGE_SCHEMA_VERSION: u32 = 2;

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

/// Current circuit breaker storage schema version.
/// V2 adds compact per-program archives for pruned failure logs.
pub const CIRCUIT_BREAKER_SCHEMA_VERSION_V2: u32 = 2;

// Idempotency key constraints
pub const MAX_IDEMPOTENCY_KEY_LENGTH: u32 = 128; // Maximum 128 characters
pub const MIN_IDEMPOTENCY_KEY_LENGTH: u32 = 1; // Minimum 1 character (non-empty)

// Constants for program scheduling
pub const BASE_FEE: i128 = 100;
pub const MIN_INCREMENT: u64 = 86400; // 1 day in seconds

/// Mandatory delay (in seconds) between proposing and accepting an admin/controller rotation.
///
/// Set to 24 hours (86 400 seconds). During this window the current admin can cancel
/// the proposal if the proposer key was compromised.
pub const ROTATION_TIMELOCK_DELAY: u64 = 86_400; // 24 hours in seconds
pub const MAX_SLOTS: usize = 1000;
/// Current release schedule storage schema version.
///
/// Increment whenever `ProgramReleaseSchedule` layout changes in a breaking way.
/// Written to instance storage during `init` so upgrade safety checks can
/// detect schema mismatches on legacy deployments.
pub const SCHEDULE_SCHEMA_VERSION_V1: u32 = 1;

/// Release trigger execution schema version.
/// Tracks deterministic execution order, explicit error codes, and retry semantics.
pub const RELEASE_TRIGGER_SCHEMA_VERSION_V1: u32 = 1;

pub fn default_history_pagination_config() -> HistoryPaginationConfig {
    HistoryPaginationConfig {
        max_limit: DEFAULT_MAX_HISTORY_PAGE_LIMIT,
        schema_version: PAGINATION_SCHEMA_VERSION_V1,
    }
}

pub fn vec_contains(values: &Vec<String>, target: &String) -> bool {
    for value in values.iter() {
        if value == *target {
            return true;
        }
    }
    false
}

pub fn get_program_dependencies_internal(env: &Env, program_id: &String) -> soroban_sdk::Vec<String> {
    env.storage()
        .instance()
        .get(&DataKey::ProgramDependencies(program_id.clone()))
        .unwrap_or(vec![env])
}

pub fn dependency_status_internal(env: &Env, dependency_id: &String) -> DependencyStatus {
    env.storage()
        .instance()
        .get(&DataKey::DependencyStatus(dependency_id.clone()))
        .unwrap_or(DependencyStatus::Pending)
}

pub fn path_exists_to_target(
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
