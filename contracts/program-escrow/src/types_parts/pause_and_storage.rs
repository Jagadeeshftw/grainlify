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
