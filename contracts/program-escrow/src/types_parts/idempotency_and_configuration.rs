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
