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
pub const INSURANCE_RESERVE_WITHDRAWN: Symbol = crate::insurance_reserve::INSURANCE_RESERVE_WITHDRAWN;
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
