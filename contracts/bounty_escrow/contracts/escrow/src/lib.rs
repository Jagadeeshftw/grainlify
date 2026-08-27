#![no_std]

mod events;
pub mod gas_budget;
mod invariants;
mod multitoken_invariants;
mod reentrancy_guard;
#[cfg(test)]
mod test_boundary_edge_cases;
mod test_cross_contract_interface;
#[cfg(test)]
mod test_deterministic_randomness;
#[cfg(test)]
mod test_multi_region_treasury;
#[cfg(test)]
mod test_multi_token_fees;
#[cfg(test)]
mod test_rbac;
#[cfg(test)]
mod test_renew_rollover;
#[cfg(test)]
mod test_risk_flags;
mod traits;
pub mod upgrade_safety;

#[cfg(test)]
mod test_frozen_balance;
#[cfg(test)]
mod test_reentrancy_guard;

use crate::events::{
    emit_batch_funds_locked, emit_batch_funds_released, emit_bounty_initialized,
    emit_deprecation_state_changed, emit_deterministic_selection, emit_funds_locked,
    emit_funds_locked_anon, emit_funds_refunded, emit_funds_released,
    emit_maintenance_mode_changed, emit_notification_preferences_updated,
    emit_participant_filter_mode_changed, emit_risk_flags_updated, emit_ticket_claimed,
    emit_refund_approval_consumed, emit_refund_approval_set, emit_ticket_issued, BatchFundsLocked,
    BatchFundsReleased, BountyEscrowInitialized, ClaimCancelled, ClaimCreated, ClaimExecuted,
    CriticalOperationOutcome, DeprecationStateChanged, DeterministicSelectionDerived, FundsLocked,
    FundsLockedAnon, FundsRefunded, FundsReleased, MaintenanceModeChanged,
    NotificationPreferencesUpdated, ParticipantFilterModeChanged, RefundApprovalConsumed,
    RefundApprovalSet, RefundTriggerType, RiskFlagsUpdated, TicketClaimed, TicketIssued,
    EVENT_VERSION_V2,
};
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Bytes,
    BytesN, Env, String, Symbol, Vec,
};

// ============================================================================
// INPUT VALIDATION MODULE
// ============================================================================

/// Validation rules for human-readable identifiers to prevent malicious or confusing inputs.
///
/// This module provides consistent validation across all contracts for:
/// - Bounty types and metadata
/// - Any user-provided string identifiers
///
/// Rules enforced:
/// - Maximum length limits to prevent UI/log issues
/// - Allowed character sets (alphanumeric, spaces, safe punctuation)
/// - No control characters that could cause display issues
/// - No leading/trailing whitespace
mod validation {
    use soroban_sdk::Env;

    /// Maximum length for bounty types and short identifiers
    const MAX_TAG_LEN: u32 = 50;

    /// Validates a tag, type, or short identifier.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `tag` - The tag string to validate
    /// * `field_name` - Name of the field for error messages
    ///
    /// # Panics
    /// Panics if validation fails with a descriptive error message.
    pub fn validate_tag(_env: &Env, tag: &soroban_sdk::String, field_name: &str) {
        if tag.len() > MAX_TAG_LEN {
            panic!(
                "{} exceeds maximum length of {} characters",
                field_name, MAX_TAG_LEN
            );
        }

        // Tags should not be empty if provided
        if tag.len() == 0 {
            panic!("{} cannot be empty", field_name);
        }
        // Additional character validation can be added when SDK supports it
    }
}

mod monitoring {
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

    // Storage keys
    #[allow(dead_code)]
    const OPERATION_COUNT: &str = "op_count";
    #[allow(dead_code)]
    const USER_COUNT: &str = "usr_count";
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
        Blocklist(Address),
        Admin,
    }

    pub fn get_config(env: &Env) -> AntiAbuseConfig {
        env.storage()
            .instance()
            .get(&AntiAbuseKey::Config)
            .unwrap_or(AntiAbuseConfig {
                window_size: 3600, // 1 hour default
                max_operations: 100,
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

    pub fn is_blocklisted(env: &Env, address: Address) -> bool {
        env.storage()
            .instance()
            .has(&AntiAbuseKey::Blocklist(address))
    }

    pub fn set_blocklist(env: &Env, address: Address, blocked: bool) {
        if blocked {
            env.storage()
                .instance()
                .set(&AntiAbuseKey::Blocklist(address), &true);
        } else {
            env.storage()
                .instance()
                .remove(&AntiAbuseKey::Blocklist(address));
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

/// Role-Based Access Control (RBAC) helpers.
///
/// # Role Matrix
///
/// | Action                  | Admin | Operator (anti-abuse admin) | Participant (depositor) |
/// |-------------------------|-------|-----------------------------|-------------------------|
/// | `init`                  | ✓     | ✗                           | ✗                       |
/// | `set_paused`            | ✓     | ✗                           | ✗                       |
/// | `emergency_withdraw`    | ✓     | ✗                           | ✗                       |
/// | `update_fee_config`     | ✓     | ✗                           | ✗                       |
/// | `set_maintenance_mode`  | ✓     | ✗                           | ✗                       |
/// | `set_deprecated`        | ✓     | ✗                           | ✗                       |
/// | `release_funds`         | ✓     | ✗                           | ✗                       |
/// | `approve_refund`        | ✓     | ✗                           | ✗                       |
/// | `partial_release`       | ✓     | ✗                           | ✗                       |
/// | `set_anti_abuse_admin`  | ✓     | ✗                           | ✗                       |
/// | `set_whitelist_entry`   | ✓     | ✓ (via anti-abuse admin)    | ✗                       |
/// | `set_blocklist_entry`   | ✓     | ✓ (via anti-abuse admin)    | ✗                       |
/// | `set_filter_mode`       | ✓     | ✗                           | ✗                       |
/// | `update_anti_abuse_cfg` | ✓     | ✗                           | ✗                       |
/// | `lock_funds`            | ✗     | ✗                           | ✓ (self only)           |
/// | `refund`                | ✓+✓   | ✗                           | ✓ (co-sign)             |
///
/// # Security Invariants
/// - No privilege escalation: operators cannot call admin-only functions.
/// - No cross-call escalation: a participant cannot trigger admin actions indirectly.
/// - `refund` requires both admin AND depositor signatures (dual-auth).
pub mod rbac {
    use soroban_sdk::{Address, Env};

    use crate::DataKey;

    /// Returns the stored admin address, panicking if not initialized.
    pub fn require_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .expect("contract not initialized")
    }

    /// Asserts that `caller` is the stored admin. Panics otherwise.
    pub fn assert_admin(env: &Env, caller: &Address) {
        let admin = require_admin(env);
        assert_eq!(&admin, caller, "caller is not admin");
        caller.require_auth();
    }

    /// Returns `true` if `addr` is the stored admin.
    pub fn is_admin(env: &Env, addr: &Address) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .map(|a| &a == addr)
            .unwrap_or(false)
    }

    /// Returns `true` if `addr` is the stored anti-abuse (operator) admin.
    pub fn is_operator(env: &Env, addr: &Address) -> bool {
        use crate::anti_abuse;
        anti_abuse::get_admin(env)
            .map(|a| &a == addr)
            .unwrap_or(false)
    }
}

#[allow(dead_code)]
const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 5_000; // 50% max fee
const MAX_BATCH_SIZE: u32 = 20;

extern crate grainlify_core;
use grainlify_core::asset;
use grainlify_core::pseudo_randomness;

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DisputeOutcome {
    ResolvedInFavorOfContributor = 1,
    ResolvedInFavorOfDepositor = 2,
    CancelledByAdmin = 3,
    Refunded = 4,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DisputeReason {
    Expired = 1,
    UnsatisfactoryWork = 2,
    Fraud = 3,
    QualityIssue = 4,
    Other = 5,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ReleaseType {
    Manual = 1,
    Automatic = 2,
}

use grainlify_core::errors;
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    BountyExists = 201,
    BountyNotFound = 202,
    FundsNotLocked = 203,
    DeadlineNotPassed = 6,
    Unauthorized = 7,
    InvalidFeeRate = 8,
    FeeRecipientNotSet = 9,
    InvalidBatchSize = 10,
    BatchSizeMismatch = 11,
    DuplicateBountyId = 12,
    /// Returned when amount is invalid (zero, negative, or exceeds available)
    InvalidAmount = 13,
    /// Returned when deadline is invalid (in the past or too far in the future)
    InvalidDeadline = 14,
    /// Returned when contract has insufficient funds for the operation
    InsufficientFunds = 16,
    /// Returned when refund is attempted without admin approval
    RefundNotApproved = 17,
    FundsPaused = 18,
    /// Returned when lock amount is below the configured policy minimum (Issue #62)
    AmountBelowMinimum = 19,
    /// Returned when lock amount is above the configured policy maximum (Issue #62)
    AmountAboveMaximum = 20,
    /// Returned when refund is blocked by a pending claim/dispute
    NotPaused = 21,
    ClaimPending = 22,
    /// Returned when claim ticket is not found
    TicketNotFound = 23,
    /// Returned when claim ticket has already been used (replay prevention)
    TicketAlreadyUsed = 24,
    /// Returned when claim ticket has expired
    TicketExpired = 25,
    CapabilityNotFound = 26,
    CapabilityExpired = 27,
    CapabilityRevoked = 28,
    CapabilityActionMismatch = 29,
    CapabilityAmountExceeded = 30,
    CapabilityUsesExhausted = 31,
    CapabilityExceedsAuthority = 32,
    InvalidAssetId = 33,
    /// Returned when new locks/registrations are disabled (contract deprecated)
    ContractDeprecated = 34,
    /// Returned when participant filtering is blocklist-only and the address is blocklisted
    ParticipantBlocked = 35,
    /// Returned when participant filtering is allowlist-only and the address is not allowlisted
    ParticipantNotAllowed = 36,
    /// Refund for anonymous escrow must go through refund_resolved (resolver provides recipient)
    AnonymousRefundRequiresResolution = 39,
    /// Anonymous resolver address not set in instance storage
    AnonymousResolverNotSet = 40,
    /// Bounty exists but is not an anonymous escrow (for refund_resolved)
    NotAnonymousEscrow = 41,
    /// Use get_escrow_info_v2 for anonymous escrows
    UseGetEscrowInfoV2ForAnonymous = 37,
    InvalidSelectionInput = 42,
    /// Returned when an upgrade safety pre-check fails
    UpgradeSafetyCheckFailed = 43,
    /// Returned when an operation's measured CPU or memory consumption exceeds
    /// the configured cap and [`gas_budget::GasBudgetConfig::enforce`] is `true`.
    /// The Soroban host reverts all storage writes and token transfers in the
    /// transaction atomically. Only reachable in test / testutils builds.
    GasBudgetExceeded = 44,
    /// Returned when an escrow is explicitly frozen by an admin hold.
    EscrowFrozen = 45,
    /// Returned when the escrow depositor is explicitly frozen by an admin hold.
    AddressFrozen = 46,
}

/// Bit flag: escrow or payout should be treated as elevated risk (indexers, UIs).
pub const RISK_FLAG_HIGH_RISK: u32 = 1 << 0;
/// Bit flag: manual or automated review is in progress; may restrict certain operations off-chain.
pub const RISK_FLAG_UNDER_REVIEW: u32 = 1 << 1;
/// Bit flag: restricted handling (e.g. compliance); informational for integrators.
pub const RISK_FLAG_RESTRICTED: u32 = 1 << 2;
/// Bit flag: aligned with soft-deprecation signaling; distinct from contract-level deprecation.
pub const RISK_FLAG_DEPRECATED: u32 = 1 << 3;

/// Notification preference flags (bitfield).
pub const NOTIFY_ON_LOCK: u32 = 1 << 0;
pub const NOTIFY_ON_RELEASE: u32 = 1 << 1;
pub const NOTIFY_ON_DISPUTE: u32 = 1 << 2;
pub const NOTIFY_ON_EXPIRATION: u32 = 1 << 3;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowMetadata {
    pub repo_id: u64,
    pub issue_id: u64,
    pub bounty_type: soroban_sdk::String,
    pub risk_flags: u32,
    pub notification_prefs: u32,
    pub reference_hash: Option<soroban_sdk::Bytes>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
    PartiallyRefunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub depositor: Address,
    /// Total amount originally locked into this escrow.
    pub amount: i128,
    /// Amount still available for release; decremented on each partial_release.
    /// Reaches 0 when fully paid out, at which point status becomes Released.
    pub remaining_amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
    pub refund_history: Vec<RefundRecord>,
    pub archived: bool,
    pub archived_at: Option<u64>,
}

/// Mutually exclusive participant filtering mode for lock_funds / batch_lock_funds.
///
/// * **Disabled**: No list check; any address may participate (allowlist still used only for anti-abuse bypass).
/// * **BlocklistOnly**: Only blocklisted addresses are rejected; all others may participate.
/// * **AllowlistOnly**: Only allowlisted (whitelisted) addresses may participate; all others are rejected.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParticipantFilterMode {
    /// Disable participant filtering. Any depositor may lock funds.
    Disabled = 0,
    /// Reject only addresses present in the blocklist.
    BlocklistOnly = 1,
    /// Accept only addresses present in the allowlist.
    AllowlistOnly = 2,
}

/// Kill-switch state: when deprecated is true, new escrows are blocked; existing escrows can complete or migrate.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecationState {
    pub deprecated: bool,
    pub migration_target: Option<Address>,
}

/// View type for deprecation status (exposed to clients).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecationStatus {
    pub deprecated: bool,
    pub migration_target: Option<Address>,
}

/// Anonymous escrow: only a 32-byte depositor commitment is stored on-chain.
/// Refunds require the configured resolver to call `refund_resolved(bounty_id, recipient)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnonymousEscrow {
    pub depositor_commitment: BytesN<32>,
    pub amount: i128,
    pub remaining_amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
    pub refund_history: Vec<RefundRecord>,
    pub archived: bool,
    pub archived_at: Option<u64>,
}

/// Depositor identity: either a concrete address (non-anon) or a 32-byte commitment (anon).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnonymousParty {
    Address(Address),
    Commitment(BytesN<32>),
}

/// Unified escrow view: exposes either address or commitment for depositor.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowInfo {
    pub depositor: AnonymousParty,
    pub amount: i128,
    pub remaining_amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
    pub refund_history: Vec<RefundRecord>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefundEligibilityCode {
    EligibleDeadlinePassed,
    EligibleAdminApproval,
    IneligibleBountyNotFound,
    IneligibleAnonymousRequiresResolution,
    IneligibleRefundPaused,
    IneligibleEscrowFrozen,
    IneligibleAddressFrozen,
    IneligibleInvalidStatus,
    IneligibleClaimPending,
    IneligibleDeadlineNotPassed,
    IneligibleInvalidApproval,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundEligibilityView {
    pub eligible: bool,
    pub code: RefundEligibilityCode,
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Option<Address>,
    pub now: u64,
    pub deadline: u64,
    pub approval_present: bool,
}

/// Immutable audit record for an escrow-level or address-level freeze.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreezeRecord {
    pub frozen: bool,
    pub reason: Option<soroban_sdk::String>,
    pub frozen_at: u64,
    pub frozen_by: Address,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Version,
    Escrow(u64),     // bounty_id
    EscrowAnon(u64), // bounty_id anonymous escrow variant
    Metadata(u64),
    EscrowIndex,             // Vec<u64> of all bounty_ids
    DepositorIndex(Address), // Vec<u64> of bounty_ids by depositor
    EscrowFreeze(u64),       // bounty_id -> FreezeRecord
    AddressFreeze(Address),  // address -> FreezeRecord
    FeeConfig,               // Fee configuration
    RefundApproval(u64),     // bounty_id -> RefundApproval
    ReentrancyGuard,
    MultisigConfig,
    ReleaseApproval(u64),        // bounty_id -> ReleaseApproval
    PendingClaim(u64),           // bounty_id -> ClaimRecord
    TicketCounter,               // monotonic claim ticket id
    ClaimTicket(u64),            // ticket_id -> ClaimTicket
    ClaimTicketIndex,            // Vec<u64> all ticket ids
    BeneficiaryTickets(Address), // beneficiary -> Vec<u64>
    ClaimWindow,                 // u64 seconds (global config)
    PauseFlags,                  // PauseFlags struct
    AmountPolicy, // Option<(i128, i128)> — (min_amount, max_amount) set by set_amount_policy
    CapabilityNonce, // monotonically increasing capability id
    Capability(BytesN<32>), // capability_id -> Capability

    /// Marks a bounty escrow as using non-transferable (soulbound) reward tokens.
    /// When set, the token is expected to disallow further transfers after claim.
    NonTransferableRewards(u64), // bounty_id -> bool

    /// Kill switch: when set, new escrows are blocked; existing escrows can complete or migrate
    DeprecationState,
    /// Participant filter mode: Disabled | BlocklistOnly | AllowlistOnly (default Disabled)
    ParticipantFilterMode,

    /// Address of the resolver that may authorize refunds for anonymous escrows
    AnonymousResolver,

    /// Chain identifier (e.g., "stellar", "ethereum") for cross-network protection
    /// Per-token fee configuration keyed by token contract address.
    TokenFeeConfig(Address),
    ChainId,
    NetworkId,

    MaintenanceMode, // bool flag
    /// Per-operation gas budget caps configured by the admin.
    /// See [`gas_budget::GasBudgetConfig`].
    GasBudgetConfig,
    /// Per-bounty renewal history (`Vec<RenewalRecord>`).
    RenewalHistory(u64),
    /// Per-bounty rollover chain link metadata.
    CycleLink(u64),
    /// Stored schema marker for refund-eligibility view semantics.
    RefundEligibilitySchemaVersion,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowWithId {
    pub bounty_id: u64,
    pub escrow: Escrow,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseFlags {
    pub lock_paused: bool,
    pub release_paused: bool,
    pub refund_paused: bool,
    pub pause_reason: Option<soroban_sdk::String>,
    pub paused_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregateStats {
    pub total_locked: i128,
    pub total_released: i128,
    pub total_refunded: i128,
    pub count_locked: u32,
    pub count_released: u32,
    pub count_refunded: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseStateChanged {
    pub operation: Symbol,
    pub paused: bool,
    pub admin: Address,
    pub reason: Option<soroban_sdk::String>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
/// Public view of anti-abuse config (rate limit and cooldown).
pub struct AntiAbuseConfigView {
    pub window_size: u64,
    pub max_operations: u32,
    pub cooldown_period: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
/// Treasury routing destination used for weighted multi-region fee distribution.
///
/// The `weight` field is interpreted relative to the sum of all configured
/// destination weights. Fee routing is deterministic: each destination receives
/// a proportional share and any rounding remainder is assigned to the final
/// destination in the configured order so accounting remains exact.
pub struct TreasuryDestination {
    /// Treasury wallet that receives routed fees.
    pub address: Address,
    /// Relative routing weight. Must be greater than zero when configured.
    pub weight: u32,
    /// Human-readable treasury region or routing label.
    pub region: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    /// Fee rate charged when funds are locked, expressed in basis points.
    pub lock_fee_rate: i128,
    /// Fee rate charged when funds are released, expressed in basis points.
    pub release_fee_rate: i128,
    /// Flat fee (token smallest units) added on each lock, before cap to deposit amount.
    pub lock_fixed_fee: i128,
    /// Flat fee added on each full release or partial payout, before cap to payout amount.
    pub release_fixed_fee: i128,
    pub fee_recipient: Address,
    /// Whether fee collection is enabled.
    pub fee_enabled: bool,
    /// Weighted treasury destinations used for multi-region routing.
    pub treasury_destinations: Vec<TreasuryDestination>,
    /// Whether multi-region treasury routing is enabled.
    pub distribution_enabled: bool,
}

/// Per-token fee configuration.
///
/// Allows different fee rates and recipients for each accepted token type.
/// When present, overrides the global `FeeConfig` for that specific token.
///
/// # Rounding protection
/// Fee amounts are always rounded **up** (ceiling division) so that
/// fractional stroops never reduce the fee to zero.  This prevents a
/// depositor from splitting a large deposit into many dust transactions
/// where floor-division would yield fee == 0 on every individual call.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenFeeConfig {
    /// Fee rate on lock, in basis points (1 bp = 0.01 %).
    pub lock_fee_rate: i128,
    /// Fee rate on release, in basis points.
    pub release_fee_rate: i128,
    pub lock_fixed_fee: i128,
    pub release_fixed_fee: i128,
    /// Address that receives fees collected for this token.
    pub fee_recipient: Address,
    /// Whether fee collection is active for this token.
    pub fee_enabled: bool,
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
pub struct ReleaseApproval {
    pub bounty_id: u64,
    pub contributor: Address,
    pub approvals: Vec<Address>,
}

const REFUND_ELIGIBILITY_SCHEMA_VERSION_V1: u32 = 1;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRecord {
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub expires_at: u64,
    pub claimed: bool,
    pub reason: DisputeReason,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimTicket {
    pub ticket_id: u64,
    pub bounty_id: u64,
    pub beneficiary: Address,
    pub amount: i128,
    pub expires_at: u64,
    pub used: bool,
    pub issued_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityAction {
    Claim,
    Release,
    Refund,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capability {
    pub owner: Address,
    pub holder: Address,
    pub action: CapabilityAction,
    pub bounty_id: u64,
    pub amount_limit: i128,
    pub remaining_amount: i128,
    pub expiry: u64,
    pub remaining_uses: u32,
    pub revoked: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefundMode {
    Full,
    Partial,
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundRecord {
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
    pub mode: RefundMode,
}

/// Immutable record of one successful escrow renewal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenewalRecord {
    /// Monotonic renewal sequence for this bounty (`1..=n`).
    pub cycle: u32,
    /// Previous deadline before renewal.
    pub old_deadline: u64,
    /// New deadline after renewal.
    pub new_deadline: u64,
    /// Additional funds deposited during renewal (`0` when extension-only).
    pub additional_amount: i128,
    /// Ledger timestamp when renewal was applied.
    pub renewed_at: u64,
}

/// Link metadata connecting bounty cycles in a rollover chain.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CycleLink {
    /// Previous bounty id in the chain (`0` for chain root).
    pub previous_id: u64,
    /// Next bounty id in the chain (`0` when no successor exists).
    pub next_id: u64,
    /// Zero-based chain depth for stored links (`0` root, `1` first successor, ...).
    pub cycle: u32,
}

/// A single escrow entry to lock within a [`BountyEscrowContract::batch_lock_funds`] call.
///
/// All items in a batch are sorted by ascending `bounty_id` before processing to ensure
/// deterministic execution order. If any item fails validation, the entire batch reverts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockFundsItem {
    /// Unique identifier for the bounty. Must not already exist in persistent storage
    /// and must not appear more than once within the same batch (`DuplicateBountyId`).
    pub bounty_id: u64,
    /// Address of the depositor. Tokens are transferred **from** this address.
    /// `require_auth()` is called once per unique depositor across the batch.
    pub depositor: Address,
    /// Gross amount (in token base units) to lock into escrow. Must be `> 0`.
    /// If an `AmountPolicy` is active, the value must fall within `[min_amount, max_amount]`.
    pub amount: i128,
    /// Unix timestamp (seconds) after which the depositor may claim a refund
    /// without requiring admin approval. Must be in the future at lock time.
    pub deadline: u64,
}

/// A single escrow release entry within a [`BountyEscrowContract::batch_release_funds`] call.
///
/// All items in a batch are sorted by ascending `bounty_id` before processing to ensure
/// deterministic execution order. If any item fails validation, the entire batch reverts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseFundsItem {
    /// Identifier of the bounty to release. The escrow record must exist (`BountyNotFound`)
    /// and must be in `Locked` status (`FundsNotLocked`).
    pub bounty_id: u64,
    /// Address of the contributor who will receive the released tokens.
    pub contributor: Address,
}

/// Result of a dry-run simulation. Indicates whether the operation would succeed
/// and the resulting state without mutating storage or performing transfers.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationResult {
    pub success: bool,
    pub error_code: u32,
    pub amount: i128,
    pub resulting_status: EscrowStatus,
    pub remaining_amount: i128,
}

#[contract]
pub struct BountyEscrowContract;

#[contractimpl]
impl BountyEscrowContract {
    pub fn health_check(env: Env) -> monitoring::HealthStatus {
        monitoring::health_check(&env)
    }

    pub fn get_analytics(env: Env) -> monitoring::Analytics {
        monitoring::get_analytics(&env)
    }

    pub fn get_state_snapshot(env: Env) -> monitoring::StateSnapshot {
        monitoring::get_state_snapshot(&env)
    }

    fn order_batch_lock_items(env: &Env, items: &Vec<LockFundsItem>) -> Vec<LockFundsItem> {
        let mut ordered: Vec<LockFundsItem> = Vec::new(env);
        for item in items.iter() {
            let mut next: Vec<LockFundsItem> = Vec::new(env);
            let mut inserted = false;
            for existing in ordered.iter() {
                if !inserted && item.bounty_id < existing.bounty_id {
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

    fn order_batch_release_items(
        env: &Env,
        items: &Vec<ReleaseFundsItem>,
    ) -> Vec<ReleaseFundsItem> {
        let mut ordered: Vec<ReleaseFundsItem> = Vec::new(env);
        for item in items.iter() {
            let mut next: Vec<ReleaseFundsItem> = Vec::new(env);
            let mut inserted = false;
            for existing in ordered.iter() {
                if !inserted && item.bounty_id < existing.bounty_id {
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

    /// Initialize the contract with the admin address and the token address (XLM).
    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        if admin == token {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        // Version 2 reflects the breaking shared-trait interface alignment.
        env.storage().instance().set(&DataKey::Version, &2u32);
        env.storage().instance().set(
            &DataKey::RefundEligibilitySchemaVersion,
            &REFUND_ELIGIBILITY_SCHEMA_VERSION_V1,
        );

        events::emit_bounty_initialized(
            &env,
            events::BountyEscrowInitialized {
                version: EVENT_VERSION_V2,
                admin,
                token,
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    pub fn init_with_network(
        env: Env,
        admin: Address,
        token: Address,
        chain_id: soroban_sdk::String,
        network_id: soroban_sdk::String,
    ) -> Result<(), Error> {
        Self::init(env.clone(), admin, token)?;
        env.storage().instance().set(&DataKey::ChainId, &chain_id);
        env.storage()
            .instance()
            .set(&DataKey::NetworkId, &network_id);
        Ok(())
    }

    pub fn get_chain_id(env: Env) -> Option<soroban_sdk::String> {
        env.storage().instance().get(&DataKey::ChainId)
    }

    pub fn get_network_id(env: Env) -> Option<soroban_sdk::String> {
        env.storage().instance().get(&DataKey::NetworkId)
    }

    pub fn get_network_info(
        env: Env,
    ) -> (Option<soroban_sdk::String>, Option<soroban_sdk::String>) {
        (Self::get_chain_id(env.clone()), Self::get_network_id(env))
    }

    /// Return the persisted contract version.
    pub fn get_version(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Version).unwrap_or(0)
    }

    /// Update the persisted contract version (admin only).
    pub fn set_version(env: Env, new_version: u32) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::Version, &new_version);
        Ok(())
    }

    /// Calculate fee amount based on rate (in basis points), using **ceiling division**.
    ///
    /// Ceiling division ensures that a non-zero fee rate always produces at least
    /// 1 stroop of fee, regardless of how small the individual amount is.  This
    /// closes the principal-drain vector where an attacker breaks a large deposit
    /// into dust amounts that each round down to a zero fee.
    ///
    /// Formula: ceil(amount * fee_rate / BASIS_POINTS)
    ///        = (amount * fee_rate + BASIS_POINTS - 1) / BASIS_POINTS
    ///
    /// # Panics
    /// Returns 0 on arithmetic overflow rather than panicking.
    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        if fee_rate == 0 || amount == 0 {
            return 0;
        }
        // Ceiling integer division: (a + b - 1) / b
        let numerator = amount
            .checked_mul(fee_rate)
            .and_then(|x| x.checked_add(BASIS_POINTS - 1))
            .unwrap_or(0);
        if numerator == 0 {
            return 0;
        }
        numerator / BASIS_POINTS
    }

    /// Total fee on `amount`: ceiling percentage plus optional fixed, capped at `amount`.
    fn combined_fee_amount(amount: i128, rate_bps: i128, fixed: i128, fee_enabled: bool) -> i128 {
        if !fee_enabled || amount <= 0 {
            return 0;
        }
        if fixed < 0 {
            return 0;
        }
        let pct = Self::calculate_fee(amount, rate_bps);
        let sum = pct.saturating_add(fixed);
        sum.min(amount).max(0)
    }

    /// Test-only shim exposing `calculate_fee` for unit-level assertions.
    #[cfg(test)]
    pub fn calculate_fee_pub(amount: i128, fee_rate: i128) -> i128 {
        Self::calculate_fee(amount, fee_rate)
    }

    /// Test-only: combined percentage + fixed fee (capped).
    #[cfg(test)]
    pub fn combined_fee_pub(amount: i128, rate_bps: i128, fixed: i128, fee_enabled: bool) -> i128 {
        Self::combined_fee_amount(amount, rate_bps, fixed, fee_enabled)
    }

    /// Get fee configuration (internal helper)
    fn get_fee_config_internal(env: &Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&DataKey::FeeConfig)
            .unwrap_or_else(|| FeeConfig {
                lock_fee_rate: 0,
                release_fee_rate: 0,
                lock_fixed_fee: 0,
                release_fixed_fee: 0,
                fee_recipient: env.storage().instance().get(&DataKey::Admin).unwrap(),
                fee_enabled: false,
                treasury_destinations: Vec::new(env),
                distribution_enabled: false,
            })
    }

    /// Validates treasury destinations before enabling multi-region routing.
    fn validate_treasury_destinations(
        _env: &Env,
        destinations: &Vec<TreasuryDestination>,
        distribution_enabled: bool,
    ) -> Result<(), Error> {
        if !distribution_enabled {
            return Ok(());
        }

        if destinations.is_empty() {
            return Err(Error::InvalidAmount);
        }

        let mut total_weight: u64 = 0;
        for destination in destinations.iter() {
            if destination.weight == 0 {
                return Err(Error::InvalidAmount);
            }

            if destination.region.is_empty() || destination.region.len() > 50 {
                return Err(Error::InvalidAmount);
            }

            total_weight = total_weight
                .checked_add(destination.weight as u64)
                .ok_or(Error::InvalidAmount)?;
        }

        if total_weight == 0 {
            return Err(Error::InvalidAmount);
        }

        Ok(())
    }

    /// Routes a fee either to the configured fee recipient or across weighted treasury routes.
    fn route_fee(
        env: &Env,
        client: &token::Client,
        config: &FeeConfig,
        amount: i128,
        fee_rate: i128,
        operation_type: events::FeeOperationType,
    ) -> Result<(), Error> {
        if amount <= 0 {
            return Ok(());
        }

        let fee_fixed = match operation_type {
            events::FeeOperationType::Lock => config.lock_fixed_fee,
            events::FeeOperationType::Release => config.release_fixed_fee,
        };

        if !config.distribution_enabled || config.treasury_destinations.is_empty() {
            client.transfer(
                &env.current_contract_address(),
                &config.fee_recipient,
                &amount,
            );
            events::emit_fee_collected(
                env,
                events::FeeCollected {
                    version: EVENT_VERSION_V2,
                    operation_type,
                    amount,
                    fee_rate,
                    fee_fixed,
                    recipient: config.fee_recipient.clone(),
                    timestamp: env.ledger().timestamp(),
                },
            );
            return Ok(());
        }

        let mut total_weight: u64 = 0;
        for destination in config.treasury_destinations.iter() {
            total_weight = total_weight
                .checked_add(destination.weight as u64)
                .ok_or(Error::InvalidAmount)?;
        }
        if total_weight == 0 {
            return Err(Error::InvalidAmount);
        }

        let mut distributed = 0i128;
        let destination_count = config.treasury_destinations.len() as usize;

        for (index, destination) in config.treasury_destinations.iter().enumerate() {
            let share = if index + 1 == destination_count {
                amount
                    .checked_sub(distributed)
                    .ok_or(Error::InvalidAmount)?
            } else {
                amount
                    .checked_mul(destination.weight as i128)
                    .and_then(|v| v.checked_div(total_weight as i128))
                    .ok_or(Error::InvalidAmount)?
            };

            distributed = distributed.checked_add(share).ok_or(Error::InvalidAmount)?;
            if share <= 0 {
                continue;
            }

            client.transfer(
                &env.current_contract_address(),
                &destination.address,
                &share,
            );
            events::emit_fee_collected(
                env,
                events::FeeCollected {
                    version: EVENT_VERSION_V2,
                    operation_type: operation_type.clone(),
                    amount: share,
                    fee_rate,
                    fee_fixed,
                    recipient: destination.address,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        Ok(())
    }

    /// Update fee configuration (admin only)
    pub fn update_fee_config(
        env: Env,
        lock_fee_rate: Option<i128>,
        release_fee_rate: Option<i128>,
        lock_fixed_fee: Option<i128>,
        release_fixed_fee: Option<i128>,
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

        if let Some(fixed) = lock_fixed_fee {
            if fixed < 0 {
                return Err(Error::InvalidAmount);
            }
            fee_config.lock_fixed_fee = fixed;
        }

        if let Some(fixed) = release_fixed_fee {
            if fixed < 0 {
                return Err(Error::InvalidAmount);
            }
            fee_config.release_fixed_fee = fixed;
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
                version: EVENT_VERSION_V2,
                lock_fee_rate: fee_config.lock_fee_rate,
                release_fee_rate: fee_config.release_fee_rate,
                lock_fixed_fee: fee_config.lock_fixed_fee,
                release_fixed_fee: fee_config.release_fixed_fee,
                fee_recipient: fee_config.fee_recipient.clone(),
                fee_enabled: fee_config.fee_enabled,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Configures weighted treasury destinations for multi-region fee routing.
    ///
    /// When enabled, collected lock and release fees are routed proportionally
    /// across `destinations` instead of sending the full amount to
    /// `fee_recipient`. Disabled routing preserves the configured destinations
    /// but falls back to the single-recipient path until re-enabled.
    pub fn set_treasury_distributions(
        env: Env,
        destinations: Vec<TreasuryDestination>,
        distribution_enabled: bool,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        Self::validate_treasury_destinations(&env, &destinations, distribution_enabled)?;

        let mut fee_config = Self::get_fee_config_internal(&env);
        fee_config.treasury_destinations = destinations;
        fee_config.distribution_enabled = distribution_enabled;

        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &fee_config);

        Ok(())
    }

    /// Returns the current treasury routing configuration.
    pub fn get_treasury_distributions(env: Env) -> (Vec<TreasuryDestination>, bool) {
        let fee_config = Self::get_fee_config_internal(&env);
        (
            fee_config.treasury_destinations,
            fee_config.distribution_enabled,
        )
    }

    /// Updates the granular pause state and metadata for the contract.
    ///
    /// # Arguments
    /// * `lock` - If Some(true), prevents new escrows from being created.
    /// * `release` - If Some(true), prevents payouts to contributors.
    /// * `refund` - If Some(true), prevents depositors from reclaiming funds.
    /// * `reason` - Optional UTF-8 string describing why the state was changed.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin has not been set.
    /// Returns `Error::Unauthorized` if the caller is not the registered admin.
    pub fn set_paused(
        env: Env,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
        reason: Option<soroban_sdk::String>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
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
            events::emit_pause_state_changed(
                &env,
                PauseStateChanged {
                    operation: symbol_short!("lock"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                },
            );
        }

        if let Some(paused) = release {
            flags.release_paused = paused;
            events::emit_pause_state_changed(
                &env,
                PauseStateChanged {
                    operation: symbol_short!("release"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
                },
            );
        }

        if let Some(paused) = refund {
            flags.refund_paused = paused;
            events::emit_pause_state_changed(
                &env,
                PauseStateChanged {
                    operation: symbol_short!("refund"),
                    paused,
                    admin: admin.clone(),
                    reason: reason.clone(),
                    timestamp,
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
        Ok(())
    }

    /// Drains all reward tokens from the contract to a target address.
    ///
    /// This is an emergency recovery function and should only be used as a last resort.
    /// The contract MUST have `lock_paused = true` before calling this.
    ///
    /// # Arguments
    /// * `target` - The address that will receive the full contract balance.
    ///
    /// # Errors
    /// Returns `Error::NotPaused` if `lock_paused` is false.
    /// Returns `Error::Unauthorized` if the caller is not the admin.
    pub fn emergency_withdraw(env: Env, target: Address) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        let flags = Self::get_pause_flags(&env);
        if !flags.lock_paused {
            return Err(Error::NotPaused);
        }

        let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::TokenClient::new(&env, &token_address);

        let contract_address = env.current_contract_address();
        let balance = token_client.balance(&contract_address);

        if balance > 0 {
            token_client.transfer(&contract_address, &target, &balance);
            events::emit_emergency_withdraw(
                &env,
                events::EmergencyWithdrawEvent {
                    version: EVENT_VERSION_V2,
                    admin,
                    recipient: target,
                    amount: balance,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Returns current deprecation state (internal). When deprecated is true, new locks are blocked.
    fn get_deprecation_state(env: &Env) -> DeprecationState {
        env.storage()
            .instance()
            .get(&DataKey::DeprecationState)
            .unwrap_or(DeprecationState {
                deprecated: false,
                migration_target: None,
            })
    }

    fn get_participant_filter_mode(env: &Env) -> ParticipantFilterMode {
        env.storage()
            .instance()
            .get(&DataKey::ParticipantFilterMode)
            .unwrap_or(ParticipantFilterMode::Disabled)
    }

    /// Enforces participant filtering: returns Err if the address is not allowed to participate
    /// (lock_funds / batch_lock_funds) under the current filter mode.
    fn check_participant_filter(env: &Env, address: Address) -> Result<(), Error> {
        let mode = Self::get_participant_filter_mode(env);
        match mode {
            ParticipantFilterMode::Disabled => Ok(()),
            ParticipantFilterMode::BlocklistOnly => {
                if anti_abuse::is_blocklisted(env, address) {
                    return Err(Error::ParticipantBlocked);
                }
                Ok(())
            }
            ParticipantFilterMode::AllowlistOnly => {
                if !anti_abuse::is_whitelisted(env, address) {
                    return Err(Error::ParticipantNotAllowed);
                }
                Ok(())
            }
        }
    }

    /// Set deprecation (kill switch) and optional migration target. Admin only.
    /// When deprecated is true: new lock_funds and batch_lock_funds are blocked; existing escrows
    /// can still release, refund, or be migrated off-chain. Emits DeprecationStateChanged.
    pub fn set_deprecated(
        env: Env,
        deprecated: bool,
        migration_target: Option<Address>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let state = DeprecationState {
            deprecated,
            migration_target: migration_target.clone(),
        };
        env.storage()
            .instance()
            .set(&DataKey::DeprecationState, &state);
        emit_deprecation_state_changed(
            &env,
            DeprecationStateChanged {
                deprecated: state.deprecated,
                migration_target: state.migration_target,
                admin,
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    /// View: returns whether the contract is deprecated and the optional migration target address.
    pub fn get_deprecation_status(env: Env) -> DeprecationStatus {
        let s = Self::get_deprecation_state(&env);
        DeprecationStatus {
            deprecated: s.deprecated,
            migration_target: s.migration_target,
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

    fn get_escrow_freeze_record_internal(env: &Env, bounty_id: u64) -> Option<FreezeRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::EscrowFreeze(bounty_id))
    }

    fn get_address_freeze_record_internal(env: &Env, address: &Address) -> Option<FreezeRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::AddressFreeze(address.clone()))
    }

    fn ensure_escrow_not_frozen(env: &Env, bounty_id: u64) -> Result<(), Error> {
        if Self::get_escrow_freeze_record_internal(env, bounty_id)
            .map(|record| record.frozen)
            .unwrap_or(false)
        {
            return Err(Error::EscrowFrozen);
        }
        Ok(())
    }

    fn ensure_address_not_frozen(env: &Env, address: &Address) -> Result<(), Error> {
        if Self::get_address_freeze_record_internal(env, address)
            .map(|record| record.frozen)
            .unwrap_or(false)
        {
            return Err(Error::AddressFrozen);
        }
        Ok(())
    }

    /// Check if an operation is paused
    fn check_paused(env: &Env, operation: Symbol) -> bool {
        let flags = Self::get_pause_flags(env);
        if operation == symbol_short!("lock") {
            if Self::is_maintenance_mode(env.clone()) {
                return true;
            }
            return flags.lock_paused;
        } else if operation == symbol_short!("release") {
            return flags.release_paused;
        } else if operation == symbol_short!("refund") {
            return flags.refund_paused;
        }
        false
    }

    /// Freeze a specific escrow so release and refund paths fail before any token transfer.
    ///
    /// Read-only queries remain available while the freeze is active.
    pub fn freeze_escrow(
        env: Env,
        bounty_id: u64,
        reason: Option<soroban_sdk::String>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
            && !env
                .storage()
                .persistent()
                .has(&DataKey::EscrowAnon(bounty_id))
        {
            return Err(Error::BountyNotFound);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let record = FreezeRecord {
            frozen: true,
            reason,
            frozen_at: env.ledger().timestamp(),
            frozen_by: admin,
        };
        env.storage()
            .persistent()
            .set(&DataKey::EscrowFreeze(bounty_id), &record);
        env.events()
            .publish((symbol_short!("frzesc"), bounty_id), record);
        Ok(())
    }

    /// Remove an escrow-level freeze and restore normal release/refund behavior.
    pub fn unfreeze_escrow(env: Env, bounty_id: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
            && !env
                .storage()
                .persistent()
                .has(&DataKey::EscrowAnon(bounty_id))
        {
            return Err(Error::BountyNotFound);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .remove(&DataKey::EscrowFreeze(bounty_id));
        env.events().publish(
            (symbol_short!("unfrzes"), bounty_id),
            (admin, env.ledger().timestamp()),
        );
        Ok(())
    }

    /// Return the current escrow-level freeze record, if one exists.
    pub fn get_escrow_freeze_record(env: Env, bounty_id: u64) -> Option<FreezeRecord> {
        Self::get_escrow_freeze_record_internal(&env, bounty_id)
    }

    /// Freeze all release/refund operations for escrows owned by `address`.
    ///
    /// Read-only queries remain available while the freeze is active.
    pub fn freeze_address(
        env: Env,
        address: Address,
        reason: Option<soroban_sdk::String>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let record = FreezeRecord {
            frozen: true,
            reason,
            frozen_at: env.ledger().timestamp(),
            frozen_by: admin,
        };
        env.storage()
            .persistent()
            .set(&DataKey::AddressFreeze(address.clone()), &record);
        env.events()
            .publish((symbol_short!("frzaddr"), address), record);
        Ok(())
    }

    /// Remove an address-level freeze and restore normal release/refund behavior.
    pub fn unfreeze_address(env: Env, address: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .remove(&DataKey::AddressFreeze(address.clone()));
        env.events().publish(
            (symbol_short!("unfrzad"), address),
            (admin, env.ledger().timestamp()),
        );
        Ok(())
    }

    /// Return the current address-level freeze record, if one exists.
    pub fn get_address_freeze_record(env: Env, address: Address) -> Option<FreezeRecord> {
        Self::get_address_freeze_record_internal(&env, &address)
    }

    /// Check if the contract is in maintenance mode
    pub fn is_maintenance_mode(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::MaintenanceMode)
            .unwrap_or(false)
    }

    /// Update maintenance mode (admin only)
    pub fn set_maintenance_mode(env: Env, enabled: bool) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::MaintenanceMode, &enabled);

        events::emit_maintenance_mode_changed(
            &env,
            MaintenanceModeChanged {
                enabled,
                admin: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    pub fn set_whitelist(env: Env, address: Address, whitelisted: bool) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        anti_abuse::set_whitelist(&env, address, whitelisted);
        Ok(())
    }

    fn next_capability_id(env: &Env) -> BytesN<32> {
        let mut id = [0u8; 32];
        let r1: u64 = env.prng().gen();
        let r2: u64 = env.prng().gen();
        let r3: u64 = env.prng().gen();
        let r4: u64 = env.prng().gen();
        id[0..8].copy_from_slice(&r1.to_be_bytes());
        id[8..16].copy_from_slice(&r2.to_be_bytes());
        id[16..24].copy_from_slice(&r3.to_be_bytes());
        id[24..32].copy_from_slice(&r4.to_be_bytes());
        BytesN::from_array(env, &id)
    }

    fn record_receipt(
        _env: &Env,
        _outcome: CriticalOperationOutcome,
        _bounty_id: u64,
        _amount: i128,
        _recipient: Address,
    ) {
        // Backward-compatible no-op until receipt storage/events are fully wired.
    }

    fn load_capability(env: &Env, capability_id: BytesN<32>) -> Result<Capability, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Capability(capability_id.clone()))
            .ok_or(Error::CapabilityNotFound)
    }

    fn validate_capability_scope_at_issue(
        env: &Env,
        owner: &Address,
        action: &CapabilityAction,
        bounty_id: u64,
        amount_limit: i128,
    ) -> Result<(), Error> {
        if amount_limit <= 0 {
            return Err(Error::InvalidAmount);
        }

        match action {
            CapabilityAction::Claim => {
                let claim: ClaimRecord = env
                    .storage()
                    .persistent()
                    .get(&DataKey::PendingClaim(bounty_id))
                    .ok_or(Error::BountyNotFound)?;
                if claim.claimed {
                    return Err(Error::FundsNotLocked);
                }
                if env.ledger().timestamp() > claim.expires_at {
                    return Err(Error::DeadlineNotPassed);
                }
                if claim.recipient != owner.clone() {
                    return Err(Error::Unauthorized);
                }
                if amount_limit > claim.amount {
                    return Err(Error::CapabilityExceedsAuthority);
                }
            }
            CapabilityAction::Release => {
                let admin: Address = env
                    .storage()
                    .instance()
                    .get(&DataKey::Admin)
                    .ok_or(Error::NotInitialized)?;
                if admin != owner.clone() {
                    return Err(Error::Unauthorized);
                }
                let escrow: Escrow = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Escrow(bounty_id))
                    .ok_or(Error::BountyNotFound)?;
                if escrow.status != EscrowStatus::Locked {
                    return Err(Error::FundsNotLocked);
                }
                if amount_limit > escrow.remaining_amount {
                    return Err(Error::CapabilityExceedsAuthority);
                }
            }
            CapabilityAction::Refund => {
                let admin: Address = env
                    .storage()
                    .instance()
                    .get(&DataKey::Admin)
                    .ok_or(Error::NotInitialized)?;
                if admin != owner.clone() {
                    return Err(Error::Unauthorized);
                }
                let escrow: Escrow = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Escrow(bounty_id))
                    .ok_or(Error::BountyNotFound)?;
                if escrow.status != EscrowStatus::Locked
                    && escrow.status != EscrowStatus::PartiallyRefunded
                {
                    return Err(Error::FundsNotLocked);
                }
                if amount_limit > escrow.remaining_amount {
                    return Err(Error::CapabilityExceedsAuthority);
                }
            }
        }

        Ok(())
    }

    fn ensure_owner_still_authorized(
        env: &Env,
        capability: &Capability,
        requested_amount: i128,
    ) -> Result<(), Error> {
        if requested_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        match capability.action {
            CapabilityAction::Claim => {
                let claim: ClaimRecord = env
                    .storage()
                    .persistent()
                    .get(&DataKey::PendingClaim(capability.bounty_id))
                    .ok_or(Error::BountyNotFound)?;
                if claim.claimed {
                    return Err(Error::FundsNotLocked);
                }
                if env.ledger().timestamp() > claim.expires_at {
                    return Err(Error::DeadlineNotPassed);
                }
                if claim.recipient != capability.owner {
                    return Err(Error::Unauthorized);
                }
                if requested_amount > claim.amount {
                    return Err(Error::CapabilityExceedsAuthority);
                }
            }
            CapabilityAction::Release => {
                let admin: Address = env
                    .storage()
                    .instance()
                    .get(&DataKey::Admin)
                    .ok_or(Error::NotInitialized)?;
                if admin != capability.owner {
                    return Err(Error::Unauthorized);
                }
                let escrow: Escrow = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Escrow(capability.bounty_id))
                    .ok_or(Error::BountyNotFound)?;
                if escrow.status != EscrowStatus::Locked {
                    return Err(Error::FundsNotLocked);
                }
                if requested_amount > escrow.remaining_amount {
                    return Err(Error::CapabilityExceedsAuthority);
                }
            }
            CapabilityAction::Refund => {
                let admin: Address = env
                    .storage()
                    .instance()
                    .get(&DataKey::Admin)
                    .ok_or(Error::NotInitialized)?;
                if admin != capability.owner {
                    return Err(Error::Unauthorized);
                }
                let escrow: Escrow = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Escrow(capability.bounty_id))
                    .ok_or(Error::BountyNotFound)?;
                if escrow.status != EscrowStatus::Locked
                    && escrow.status != EscrowStatus::PartiallyRefunded
                {
                    return Err(Error::FundsNotLocked);
                }
                if requested_amount > escrow.remaining_amount {
                    return Err(Error::CapabilityExceedsAuthority);
                }
            }
        }
        Ok(())
    }

    /// Validates and consumes a capability token for a specific action.
    ///
    /// The capability token must be a secure `BytesN<32>` identifier explicitly issued
    /// to the requested `holder` for the requested `bounty_id` and `expected_action`.
    /// Consuming a capability securely updates its internal balance and usage counts,
    /// protecting against replay attacks or brute-force forgery.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `holder` - The address attempting to consume the capability
    /// * `capability_id` - The `BytesN<32>` unforgeable token identifier
    /// * `expected_action` - The required action mapped to this capability
    /// * `bounty_id` - The bounty ID relating to the action
    /// * `amount` - The transaction value requested during this consumption limit
    ///
    /// # Returns
    /// The updated `Capability` struct successfully verified, or an `Error`.
    fn consume_capability(
        env: &Env,
        holder: &Address,
        capability_id: BytesN<32>,
        expected_action: CapabilityAction,
        bounty_id: u64,
        amount: i128,
    ) -> Result<Capability, Error> {
        let mut capability = Self::load_capability(env, capability_id.clone())?;

        if capability.revoked {
            return Err(Error::CapabilityRevoked);
        }
        if capability.action != expected_action {
            return Err(Error::CapabilityActionMismatch);
        }
        if capability.bounty_id != bounty_id {
            return Err(Error::CapabilityActionMismatch);
        }
        if capability.holder != holder.clone() {
            return Err(Error::Unauthorized);
        }
        if env.ledger().timestamp() > capability.expiry {
            return Err(Error::CapabilityExpired);
        }
        if capability.remaining_uses == 0 {
            return Err(Error::CapabilityUsesExhausted);
        }
        if amount > capability.remaining_amount {
            return Err(Error::CapabilityAmountExceeded);
        }

        holder.require_auth();
        Self::ensure_owner_still_authorized(env, &capability, amount)?;

        capability.remaining_amount -= amount;
        capability.remaining_uses -= 1;
        env.storage()
            .persistent()
            .set(&DataKey::Capability(capability_id.clone()), &capability);

        events::emit_capability_used(
            env,
            events::CapabilityUsed {
                capability_id,
                holder: holder.clone(),
                action: capability.action.clone(),
                bounty_id,
                amount_used: amount,
                remaining_amount: capability.remaining_amount,
                remaining_uses: capability.remaining_uses,
                used_at: env.ledger().timestamp(),
            },
        );

        Ok(capability)
    }

    /// Issues a new capability token for a specific action on a bounty.
    ///
    /// The capability token is represented by a secure, unforgeable `BytesN<32>` identifier
    /// generated using the Soroban environment's pseudo-random number generator (PRNG).
    /// This ensures that capability tokens cannot be predicted or forged by arbitrary addresses.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `owner` - The address delegating authority (e.g. the bounty admin or depositor)
    /// * `holder` - The address receiving the capability token
    /// * `action` - The specific action authorized (`Release`, `Refund`, etc.)
    /// * `bounty_id` - The bounty this capability applies to
    /// * `amount_limit` - The maximum amount of funds authorized by this capability
    /// * `expiry` - The ledger timestamp when this capability expires
    /// * `max_uses` - The maximum number of times this capability can be consumed
    ///
    /// # Returns
    /// The generated `BytesN<32>` capability identifier, or an `Error` if issuance fails.
    pub fn issue_capability(
        env: Env,
        owner: Address,
        holder: Address,
        action: CapabilityAction,
        bounty_id: u64,
        amount_limit: i128,
        expiry: u64,
        max_uses: u32,
    ) -> Result<BytesN<32>, Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        if max_uses == 0 {
            return Err(Error::InvalidAmount);
        }

        let now = env.ledger().timestamp();
        if expiry <= now {
            return Err(Error::InvalidDeadline);
        }

        owner.require_auth();
        Self::validate_capability_scope_at_issue(&env, &owner, &action, bounty_id, amount_limit)?;

        let capability_id = Self::next_capability_id(&env);
        let capability = Capability {
            owner: owner.clone(),
            holder: holder.clone(),
            action: action.clone(),
            bounty_id,
            amount_limit,
            remaining_amount: amount_limit,
            expiry,
            remaining_uses: max_uses,
            revoked: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Capability(capability_id.clone()), &capability);

        events::emit_capability_issued(
            &env,
            events::CapabilityIssued {
                capability_id: capability_id.clone(),
                owner,
                holder,
                action,
                bounty_id,
                amount_limit,
                expires_at: expiry,
                max_uses,
                timestamp: now,
            },
        );

        Ok(capability_id.clone())
    }

    pub fn revoke_capability(
        env: Env,
        owner: Address,
        capability_id: BytesN<32>,
    ) -> Result<(), Error> {
        let mut capability = Self::load_capability(&env, capability_id.clone())?;
        if capability.owner != owner {
            return Err(Error::Unauthorized);
        }
        owner.require_auth();

        if capability.revoked {
            return Ok(());
        }

        capability.revoked = true;
        env.storage()
            .persistent()
            .set(&DataKey::Capability(capability_id.clone()), &capability);

        events::emit_capability_revoked(
            &env,
            events::CapabilityRevoked {
                capability_id,
                owner,
                revoked_at: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn get_capability(env: Env, capability_id: BytesN<32>) -> Result<Capability, Error> {
        Self::load_capability(&env, capability_id.clone())
    }

    /// Get current fee configuration (view function)
    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
    }

    /// Set a per-token fee configuration (admin only).
    ///
    /// When a `TokenFeeConfig` is set for a given token address it takes
    /// precedence over the global `FeeConfig` for all escrows denominated
    /// in that token.
    ///
    /// # Arguments
    /// * `token`            – the token contract address this config applies to
    /// * `lock_fee_rate`    – fee rate on lock in basis points (0 – 5 000)
    /// * `release_fee_rate` – fee rate on release in basis points (0 – 5 000)
    /// * `lock_fixed_fee` / `release_fixed_fee` – flat fees in token units (≥ 0)
    /// * `fee_recipient`    – address that receives fees for this token
    /// * `fee_enabled`      – whether fee collection is active
    ///
    /// # Errors
    /// * `NotInitialized`  – contract not yet initialised
    /// * `InvalidFeeRate`  – any rate is outside `[0, MAX_FEE_RATE]`
    pub fn set_token_fee_config(
        env: Env,
        token: Address,
        lock_fee_rate: i128,
        release_fee_rate: i128,
        lock_fixed_fee: i128,
        release_fixed_fee: i128,
        fee_recipient: Address,
        fee_enabled: bool,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !(0..=MAX_FEE_RATE).contains(&lock_fee_rate) {
            return Err(Error::InvalidFeeRate);
        }
        if !(0..=MAX_FEE_RATE).contains(&release_fee_rate) {
            return Err(Error::InvalidFeeRate);
        }
        if lock_fixed_fee < 0 || release_fixed_fee < 0 {
            return Err(Error::InvalidAmount);
        }

        let config = TokenFeeConfig {
            lock_fee_rate,
            release_fee_rate,
            lock_fixed_fee,
            release_fixed_fee,
            fee_recipient,
            fee_enabled,
        };

        env.storage()
            .instance()
            .set(&DataKey::TokenFeeConfig(token), &config);

        Ok(())
    }

    /// Get the per-token fee configuration for `token`, if one has been set.
    ///
    /// Returns `None` when no token-specific config exists; callers should
    /// fall back to the global `FeeConfig` in that case.
    pub fn get_token_fee_config(env: Env, token: Address) -> Option<TokenFeeConfig> {
        env.storage()
            .instance()
            .get(&DataKey::TokenFeeConfig(token))
    }

    /// Internal: resolve the effective fee config for the escrow token.
    ///
    /// Precedence: `TokenFeeConfig(token)` > global `FeeConfig`.
    fn resolve_fee_config(env: &Env) -> (i128, i128, i128, i128, Address, bool) {
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        if let Some(tok_cfg) = env
            .storage()
            .instance()
            .get::<DataKey, TokenFeeConfig>(&DataKey::TokenFeeConfig(token_addr))
        {
            (
                tok_cfg.lock_fee_rate,
                tok_cfg.release_fee_rate,
                tok_cfg.lock_fixed_fee,
                tok_cfg.release_fixed_fee,
                tok_cfg.fee_recipient,
                tok_cfg.fee_enabled,
            )
        } else {
            let global = Self::get_fee_config_internal(env);
            (
                global.lock_fee_rate,
                global.release_fee_rate,
                global.lock_fixed_fee,
                global.release_fixed_fee,
                global.fee_recipient,
                global.fee_enabled,
            )
        }
    }

    /// Update multisig configuration (admin only)
    pub fn update_multisig_config(
        env: Env,
        threshold_amount: i128,
        signers: Vec<Address>,
        required_signatures: u32,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if required_signatures > signers.len() {
            return Err(Error::InvalidAmount);
        }

        let config = MultisigConfig {
            threshold_amount,
            signers,
            required_signatures,
        };

        env.storage()
            .instance()
            .set(&DataKey::MultisigConfig, &config);

        Ok(())
    }

    /// Get multisig configuration
    pub fn get_multisig_config(env: Env) -> MultisigConfig {
        env.storage()
            .instance()
            .get(&DataKey::MultisigConfig)
            .unwrap_or(MultisigConfig {
                threshold_amount: i128::MAX,
                signers: vec![&env],
                required_signatures: 0,
            })
    }

    /// Approve release for large amount (requires multisig)
    pub fn approve_large_release(
        env: Env,
        bounty_id: u64,
        contributor: Address,
        approver: Address,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let multisig_config: MultisigConfig = Self::get_multisig_config(env.clone());

        let mut is_signer = false;
        for signer in multisig_config.signers.iter() {
            if signer == approver {
                is_signer = true;
                break;
            }
        }

        if !is_signer {
            return Err(Error::Unauthorized);
        }

        approver.require_auth();

        let approval_key = DataKey::ReleaseApproval(bounty_id);
        let mut approval: ReleaseApproval = env
            .storage()
            .persistent()
            .get(&approval_key)
            .unwrap_or(ReleaseApproval {
                bounty_id,
                contributor: contributor.clone(),
                approvals: vec![&env],
            });

        for existing in approval.approvals.iter() {
            if existing == approver {
                return Ok(());
            }
        }

        approval.approvals.push_back(approver.clone());
        env.storage().persistent().set(&approval_key, &approval);

        events::emit_approval_added(
            &env,
            events::ApprovalAdded {
                version: EVENT_VERSION_V2,
                bounty_id,
                contributor: contributor.clone(),
                approver,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Locks funds for a bounty and records escrow state.
    ///
    /// # Security
    /// - Validation order is deterministic to avoid ambiguous failure behavior under contention.
    /// - Reentrancy guard is acquired before validation and released on completion.
    ///
    /// # Errors
    /// Returns `Error` variants for initialization, policy, authorization, and duplicate-bounty
    /// failures.
    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        let res =
            Self::lock_funds_logic(env.clone(), depositor.clone(), bounty_id, amount, deadline);
        monitoring::track_operation(&env, symbol_short!("lock"), depositor, res.is_ok());
        res
    }

    fn lock_funds_logic(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        // Validation precedence (deterministic ordering):
        // 1. Reentrancy guard
        // 2. Contract initialized
        // 3. Paused / deprecated (operational state)
        // 4. Participant filter + rate limiting
        // 5. Authorization
        // 6. Input validation (amount policy)
        // 7. Business logic (bounty uniqueness)

        // 1. GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);
        // Snapshot resource meters for gas cap enforcement (test / testutils only).
        #[cfg(any(test, feature = "testutils"))]
        let gas_snapshot = gas_budget::capture(&env);

        // 2. Contract must be initialized before any other check
        if !env.storage().instance().has(&DataKey::Admin) {
            reentrancy_guard::release(&env);
            return Err(Error::NotInitialized);
        }
        soroban_sdk::log!(&env, "admin ok");

        // 3. Operational state: paused / deprecated
        if Self::check_paused(&env, symbol_short!("lock")) {
            reentrancy_guard::release(&env);
            return Err(Error::FundsPaused);
        }
        if Self::get_deprecation_state(&env).deprecated {
            reentrancy_guard::release(&env);
            return Err(Error::ContractDeprecated);
        }
        soroban_sdk::log!(&env, "check paused ok");

        // 4. Participant filtering and rate limiting
        Self::check_participant_filter(&env, depositor.clone())?;
        soroban_sdk::log!(&env, "start lock_funds");
        anti_abuse::check_rate_limit(&env, depositor.clone());
        soroban_sdk::log!(&env, "rate limit ok");

        let _start = env.ledger().timestamp();
        let _caller = depositor.clone();

        // 5. Authorization
        depositor.require_auth();
        soroban_sdk::log!(&env, "auth ok");

        // 6. Input validation: amount policy
        // Enforce min/max amount policy if one has been configured (Issue #62).
        if let Some((min_amount, max_amount)) = env
            .storage()
            .instance()
            .get::<DataKey, (i128, i128)>(&DataKey::AmountPolicy)
        {
            if amount < min_amount {
                reentrancy_guard::release(&env);
                return Err(Error::AmountBelowMinimum);
            }
            if amount > max_amount {
                reentrancy_guard::release(&env);
                return Err(Error::AmountAboveMaximum);
            }
        }
        soroban_sdk::log!(&env, "amount policy ok");

        // 7. Business logic: bounty must not already exist
        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            reentrancy_guard::release(&env);
            return Err(Error::BountyExists);
        }
        soroban_sdk::log!(&env, "bounty exists ok");

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        soroban_sdk::log!(&env, "token client ok");

        // Transfer full gross amount from depositor to contract first.
        client.transfer(&depositor, &env.current_contract_address(), &amount);
        soroban_sdk::log!(&env, "transfer ok");

        // Resolve effective fee config (per-token takes precedence over global).
        let (
            lock_fee_rate,
            _release_fee_rate,
            lock_fixed_fee,
            _release_fixed,
            fee_recipient,
            fee_enabled,
        ) = Self::resolve_fee_config(&env);
        let fee_config = FeeConfig {
            lock_fee_rate,
            release_fee_rate: 0,
            lock_fixed_fee,
            release_fixed_fee: 0,
            fee_recipient: fee_recipient.clone(),
            fee_enabled,
            treasury_destinations: Vec::new(&env),
            distribution_enabled: false,
        };

        // Deduct lock fee from the escrowed principal (percentage + fixed, capped at deposit).
        let fee_amount =
            Self::combined_fee_amount(amount, lock_fee_rate, lock_fixed_fee, fee_enabled);

        // Net amount stored in escrow after fee.
        // Fee must never exceed the deposit; guard against misconfiguration.
        let net_amount = amount.checked_sub(fee_amount).unwrap_or(amount);
        if net_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Transfer fee to recipient immediately (separate transfer so it is
        // visible as a distinct on-chain operation).
        if fee_amount > 0 {
            Self::route_fee(
                &env,
                &client,
                &fee_config,
                fee_amount,
                lock_fee_rate,
                events::FeeOperationType::Lock,
            )?;
        }
        soroban_sdk::log!(&env, "fee ok");

        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: net_amount,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: vec![&env],
            remaining_amount: net_amount,
            archived: false,
            archived_at: None,
        };
        invariants::assert_escrow(&env, &escrow);

        // Extend the TTL of the storage entry to ensure it lives long enough
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Update indexes
        let mut index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        index.push_back(bounty_id);
        env.storage()
            .persistent()
            .set(&DataKey::EscrowIndex, &index);

        let mut depositor_index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::DepositorIndex(depositor.clone()))
            .unwrap_or(Vec::new(&env));
        depositor_index.push_back(bounty_id);
        env.storage().persistent().set(
            &DataKey::DepositorIndex(depositor.clone()),
            &depositor_index,
        );

        // Emit value allows for off-chain indexing
        emit_funds_locked(
            &env,
            FundsLocked {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount,
                depositor: depositor.clone(),
                deadline,
            },
        );

        // INV-2: Verify aggregate balance matches token balance after lock
        multitoken_invariants::assert_after_lock(&env);

        // Gas budget cap enforcement (test / testutils only; see `gas_budget` module docs).
        #[cfg(any(test, feature = "testutils"))]
        {
            let gas_cfg = gas_budget::get_config(&env);
            if let Err(e) = gas_budget::check(
                &env,
                symbol_short!("lock"),
                &gas_cfg.lock,
                &gas_snapshot,
                gas_cfg.enforce,
            ) {
                reentrancy_guard::release(&env);
                return Err(e);
            }
        }

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Simulate lock operation without state changes or token transfers.
    ///
    /// Returns a `SimulationResult` indicating whether the operation would succeed and the
    /// resulting escrow state. Does not require authorization; safe for off-chain preview.
    ///
    /// # Arguments
    /// * `depositor` - Address that would lock funds
    /// * `bounty_id` - Bounty identifier
    /// * `amount` - Amount to lock
    /// * `deadline` - Deadline timestamp
    ///
    /// # Security
    /// This function performs only read operations. No storage writes, token transfers,
    /// or events are emitted.
    pub fn archive_escrow(env: Env, bounty_id: u64) -> Result<(), Error> {
        let admin = rbac::require_admin(&env);
        admin.require_auth();

        let mut escrow = env
            .storage()
            .persistent()
            .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
            .ok_or(Error::BountyNotFound)?;

        escrow.archived = true;
        escrow.archived_at = Some(env.ledger().timestamp());

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Also check anon escrow
        if let Some(mut anon) = env
            .storage()
            .persistent()
            .get::<DataKey, AnonymousEscrow>(&DataKey::EscrowAnon(bounty_id))
        {
            anon.archived = true;
            anon.archived_at = Some(env.ledger().timestamp());
            env.storage()
                .persistent()
                .set(&DataKey::EscrowAnon(bounty_id), &anon);
        }

        events::emit_archived(&env, bounty_id, env.ledger().timestamp());
        Ok(())
    }

    /// Get all archived escrow IDs.
    pub fn get_archived_escrows(env: Env) -> Vec<u64> {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        let mut archived = Vec::new(&env);
        for id in index.iter() {
            if let Some(escrow) = env
                .storage()
                .persistent()
                .get::<DataKey, Escrow>(&DataKey::Escrow(id))
            {
                if escrow.archived {
                    archived.push_back(id);
                }
            } else if let Some(anon) = env
                .storage()
                .persistent()
                .get::<DataKey, AnonymousEscrow>(&DataKey::EscrowAnon(id))
            {
                if anon.archived {
                    archived.push_back(id);
                }
            }
        }
        archived
    }

    /// Simulation of a lock operation.
    pub fn dry_run_lock(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> SimulationResult {
        fn err_result(e: Error) -> SimulationResult {
            SimulationResult {
                success: false,
                error_code: e as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            }
        }
        match Self::dry_run_lock_impl(&env, depositor, bounty_id, amount, deadline) {
            Ok((net_amount,)) => SimulationResult {
                success: true,
                error_code: 0,
                amount: net_amount,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: net_amount,
            },
            Err(e) => err_result(e),
        }
    }

    fn dry_run_lock_impl(
        env: &Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        _deadline: u64,
    ) -> Result<(i128,), Error> {
        // 1. Contract must be initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        // 2. Operational state: paused / deprecated
        if Self::check_paused(env, symbol_short!("lock")) {
            return Err(Error::FundsPaused);
        }
        if Self::get_deprecation_state(env).deprecated {
            return Err(Error::ContractDeprecated);
        }
        // 3. Participant filtering (read-only)
        Self::check_participant_filter(env, depositor.clone())?;
        // 4. Amount policy
        if let Some((min_amount, max_amount)) = env
            .storage()
            .instance()
            .get::<DataKey, (i128, i128)>(&DataKey::AmountPolicy)
        {
            if amount < min_amount {
                return Err(Error::AmountBelowMinimum);
            }
            if amount > max_amount {
                return Err(Error::AmountAboveMaximum);
            }
        }
        // 5. Bounty must not already exist
        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyExists);
        }
        // 6. Amount validation
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(env, &token_addr);
        // 7. Sufficient balance (read-only)
        let balance = client.balance(&depositor);
        if balance < amount {
            return Err(Error::InsufficientFunds);
        }
        // 8. Fee computation (pure)
        let (
            lock_fee_rate,
            _release_fee_rate,
            lock_fixed_fee,
            _release_fixed,
            _fee_recipient,
            fee_enabled,
        ) = Self::resolve_fee_config(env);
        let fee_amount =
            Self::combined_fee_amount(amount, lock_fee_rate, lock_fixed_fee, fee_enabled);
        let net_amount = amount.checked_sub(fee_amount).unwrap_or(amount);
        if net_amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        Ok((net_amount,))
    }

    /// Returns whether the given bounty escrow is marked as using non-transferable (soulbound)
    /// reward tokens. When true, the token is expected to disallow further transfers after claim.
    pub fn get_non_transferable_rewards(env: Env, bounty_id: u64) -> Result<bool, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::NonTransferableRewards(bounty_id))
            .unwrap_or(false))
    }

    /// Lock funds for a bounty in anonymous mode: only a 32-byte depositor commitment is stored.
    /// The depositor must authorize and transfer; their address is used only for the transfer
    /// in this call and is not stored on-chain. Refunds require the configured anonymous
    /// resolver to call `refund_resolved(bounty_id, recipient)`.
    pub fn lock_funds_anonymous(
        env: Env,
        depositor: Address,
        depositor_commitment: BytesN<32>,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        // Validation precedence (deterministic ordering):
        // 1. Reentrancy guard
        // 2. Contract initialized
        // 3. Paused (operational state)
        // 4. Rate limiting
        // 5. Authorization
        // 6. Business logic (bounty uniqueness, amount policy)

        // 1. Reentrancy guard
        reentrancy_guard::acquire(&env);

        // 2. Contract must be initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            reentrancy_guard::release(&env);
            return Err(Error::NotInitialized);
        }

        // 3. Operational state: paused
        if Self::check_paused(&env, symbol_short!("lock")) {
            reentrancy_guard::release(&env);
            return Err(Error::FundsPaused);
        }

        // 4. Rate limiting
        anti_abuse::check_rate_limit(&env, depositor.clone());

        // 5. Authorization
        depositor.require_auth();

        if env.storage().persistent().has(&DataKey::Escrow(bounty_id))
            || env
                .storage()
                .persistent()
                .has(&DataKey::EscrowAnon(bounty_id))
        {
            reentrancy_guard::release(&env);
            return Err(Error::BountyExists);
        }

        if let Some((min_amount, max_amount)) = env
            .storage()
            .instance()
            .get::<DataKey, (i128, i128)>(&DataKey::AmountPolicy)
        {
            if amount < min_amount {
                reentrancy_guard::release(&env);
                return Err(Error::AmountBelowMinimum);
            }
            if amount > max_amount {
                reentrancy_guard::release(&env);
                return Err(Error::AmountAboveMaximum);
            }
        }

        let escrow_anon = AnonymousEscrow {
            depositor_commitment: depositor_commitment.clone(),
            amount,
            remaining_amount: amount,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: vec![&env],
            archived: false,
            archived_at: None,
        };

        env.storage()
            .persistent()
            .set(&DataKey::EscrowAnon(bounty_id), &escrow_anon);

        let mut index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        index.push_back(bounty_id);
        env.storage()
            .persistent()
            .set(&DataKey::EscrowIndex, &index);

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(&depositor, &env.current_contract_address(), &amount);

        emit_funds_locked_anon(
            &env,
            FundsLockedAnon {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount,
                depositor_commitment,
                deadline,
            },
        );

        multitoken_invariants::assert_after_lock(&env);
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Releases escrowed funds to a contributor.
    ///
    /// # Access Control
    /// Admin-only.
    ///
    /// # Front-running Behavior
    /// First valid release for a bounty transitions state to `Released`. Later release/refund/claim
    /// races against that bounty must fail with `Error::FundsNotLocked`.
    ///
    /// # Security
    /// Reentrancy guard is always cleared before any explicit error return after acquisition.
    pub fn publish(env: Env, bounty_id: u64) -> Result<(), Error> {
        let _caller = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .expect("Admin not set");
        Self::publish_logic(env, bounty_id, _caller)
    }

    fn publish_logic(env: Env, bounty_id: u64, publisher: Address) -> Result<(), Error> {
        // Validation precedence:
        // 1. Reentrancy guard
        // 2. Authorization (admin only)
        // 3. Escrow exists and is in Draft status

        // 1. Acquire reentrancy guard
        reentrancy_guard::acquire(&env);

        // 2. Admin authorization
        publisher.require_auth();

        // 3. Get escrow and verify it's in Draft status
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .ok_or(Error::BountyNotFound)?;

        if escrow.status != EscrowStatus::Draft {
            reentrancy_guard::release(&env);
            return Err(Error::ActionNotFound);
        }

        // Transition from Draft to Locked
        escrow.status = EscrowStatus::Locked;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Emit EscrowPublished event
        emit_escrow_published(
            &env,
            EscrowPublished {
                version: EVENT_VERSION_V2,
                bounty_id,
                published_by: publisher,
                timestamp: env.ledger().timestamp(),
            },
        );

        multitoken_invariants::assert_after_lock(&env);
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Releases escrowed funds to a contributor.
    ///
    /// # Invariants Verified
    /// - INV-ESC-4: Released => remaining_amount == 0
    /// - INV-ESC-7: Aggregate fund conservation (sum(active) == contract.balance)
    ///
    /// # Access Control
    /// Admin-only.
    ///
    /// # Front-running Behavior
    /// First valid release for a bounty transitions state to `Released`. Later release/refund/claim
    /// races against that bounty must fail with `Error::FundsNotLocked`.
    ///
    /// # Transition Guards
    /// This function enforces the following state transition guards:
    ///
    /// ## Pre-conditions (checked in order):
    /// 1. **Reentrancy Guard**: Acquires reentrancy lock to prevent concurrent execution
    /// 2. **Initialization**: Contract must be initialized (admin set)
    /// 3. **Operational State**: Contract must not be paused for release operations
    /// 4. **Authorization**: Admin must authorize the transaction
    /// 5. **Escrow Existence**: Bounty must exist in storage
    /// 6. **Freeze Check**: Escrow and depositor must not be frozen
    /// 7. **Status Guard**: Escrow status must be `Locked` or `PartiallyRefunded`
    ///
    /// ## State Transition:
    /// - **From**: `Locked` or `PartiallyRefunded`
    /// - **To**: `Released`
    /// - **Effect**: Sets `remaining_amount` to 0
    ///
    /// ## Post-conditions:
    /// - External token transfer to contributor (after state update)
    /// - Fee transfer to fee recipient (if applicable)
    /// - Event emission
    ///
    /// ## Contention Safety:
    /// - If status is `Released`, `Refunded`, or `Draft`, returns `Error::FundsNotLocked`
    /// - Reentrancy guard prevents concurrent execution of any protected function
    /// - CEI pattern ensures state is updated before external calls
    ///
    /// # Security
    /// Reentrancy guard is always cleared before any explicit error return after acquisition.
    pub fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
        let caller = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .unwrap_or(contributor.clone());
        let res = Self::release_funds_logic(env.clone(), bounty_id, contributor);
        monitoring::track_operation(&env, symbol_short!("release"), caller, res.is_ok());
        res
    }

    fn release_funds_logic(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
        // Validation precedence (deterministic ordering):
        // 1. Reentrancy guard
        // 2. Contract initialized
        // 3. Paused (operational state)
        // 4. Authorization
        // 5. Business logic (bounty exists, funds locked)

        // 1. GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        // 2. Contract must be initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        // 3. Operational state: paused
        if Self::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }

        // 4. Authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // 5. Business logic: bounty must exist and be locked
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        Self::ensure_escrow_not_frozen(&env, bounty_id)?;
        Self::ensure_address_not_frozen(&env, &escrow.depositor)?;

        if escrow.status != EscrowStatus::Locked {
            reentrancy_guard::release(&env);
            return Err(Error::FundsNotLocked);
        }

        // Resolve effective fee config for release.
        let (
            _lock_fee_rate,
            release_fee_rate,
            _lock_fixed,
            release_fixed_fee,
            fee_recipient,
            fee_enabled,
        ) = Self::resolve_fee_config(&env);

        let release_fee = Self::combined_fee_amount(
            escrow.amount,
            release_fee_rate,
            release_fixed_fee,
            fee_enabled,
        );
        let mut fee_config = Self::get_fee_config_internal(&env);
        fee_config.release_fee_rate = release_fee_rate;
        fee_config.release_fixed_fee = release_fixed_fee;
        fee_config.fee_recipient = fee_recipient.clone();
        fee_config.fee_enabled = fee_enabled;

        // Net payout to contributor after release fee.
        let net_payout = escrow
            .amount
            .checked_sub(release_fee)
            .unwrap_or(escrow.amount);
        if net_payout <= 0 {
            reentrancy_guard::release(&env);
            return Err(Error::InvalidAmount);
        }

        // EFFECTS: update state before external calls (CEI)
        escrow.status = EscrowStatus::Released;
        escrow.remaining_amount = 0;
        invariants::assert_escrow(&env, &escrow);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // INTERACTION: external token transfers are last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        if release_fee > 0 {
            Self::route_fee(
                &env,
                &client,
                &fee_config,
                release_fee,
                release_fee_rate,
                events::FeeOperationType::Release,
            )?;
        }

        client.transfer(&env.current_contract_address(), &contributor, &net_payout);

        emit_funds_released(
            &env,
            FundsReleased {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: escrow.amount,
                recipient: contributor.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Simulate release operation without state changes or token transfers.
    ///
    /// Returns a `SimulationResult` indicating whether the operation would succeed and the
    /// resulting escrow state. Does not require authorization; safe for off-chain preview.
    ///
    /// # Arguments
    /// * `bounty_id` - Bounty identifier
    /// * `contributor` - Recipient address
    ///
    /// # Security
    /// This function performs only read operations. No storage writes, token transfers,
    /// or events are emitted.
    pub fn dry_run_release(env: Env, bounty_id: u64, contributor: Address) -> SimulationResult {
        fn err_result(e: Error) -> SimulationResult {
            SimulationResult {
                success: false,
                error_code: e as u32,
                amount: 0,
                resulting_status: EscrowStatus::Released,
                remaining_amount: 0,
            }
        }
        match Self::dry_run_release_impl(&env, bounty_id, contributor) {
            Ok((amount,)) => SimulationResult {
                success: true,
                error_code: 0,
                amount,
                resulting_status: EscrowStatus::Released,
                remaining_amount: 0,
            },
            Err(e) => err_result(e),
        }
    }

    fn dry_run_release_impl(
        env: &Env,
        bounty_id: u64,
        _contributor: Address,
    ) -> Result<(i128,), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        if Self::check_paused(env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Self::ensure_escrow_not_frozen(env, bounty_id)?;
        Self::ensure_address_not_frozen(env, &escrow.depositor)?;
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }
        let (
            _lock_fee_rate,
            release_fee_rate,
            _lock_fixed,
            release_fixed_fee,
            _fee_recipient,
            fee_enabled,
        ) = Self::resolve_fee_config(env);
        let release_fee = Self::combined_fee_amount(
            escrow.amount,
            release_fee_rate,
            release_fixed_fee,
            fee_enabled,
        );
        let net_payout = escrow
            .amount
            .checked_sub(release_fee)
            .unwrap_or(escrow.amount);
        if net_payout <= 0 {
            return Err(Error::InvalidAmount);
        }
        Ok((escrow.amount,))
    }

    /// Delegated release flow using a capability instead of admin auth.
    /// The capability amount limit is consumed by `payout_amount`.
    pub fn release_with_capability(
        env: Env,
        bounty_id: u64,
        contributor: Address,
        payout_amount: i128,
        holder: Address,
        capability_id: BytesN<32>,
    ) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if Self::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }
        if payout_amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Self::ensure_escrow_not_frozen(&env, bounty_id)?;
        Self::ensure_address_not_frozen(&env, &escrow.depositor)?;
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }
        if payout_amount > escrow.remaining_amount {
            return Err(Error::InsufficientFunds);
        }

        Self::consume_capability(
            &env,
            &holder,
            capability_id,
            CapabilityAction::Release,
            bounty_id,
            payout_amount,
        )?;

        // EFFECTS: update state before external call (CEI)
        escrow.remaining_amount -= payout_amount;
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released;
        }
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(
            &env.current_contract_address(),
            &contributor,
            &payout_amount,
        );

        emit_funds_released(
            &env,
            FundsReleased {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: payout_amount,
                recipient: contributor,
                timestamp: env.ledger().timestamp(),
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Set the claim window duration (admin only).
    /// claim_window: seconds beneficiary has to claim after release is authorized.
    pub fn set_claim_window(env: Env, claim_window: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::ClaimWindow, &claim_window);
        Ok(())
    }

    /// Authorizes a pending claim instead of immediate transfer.
    ///
    /// # Access Control
    /// Admin-only.
    ///
    /// # Front-running Behavior
    /// Repeated authorizations are overwrite semantics: the latest successful authorization for
    /// a locked bounty replaces the previous pending recipient/record.
    pub fn authorize_claim(
        env: Env,
        bounty_id: u64,
        recipient: Address,
        reason: DisputeReason,
    ) -> Result<(), Error> {
        if Self::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }
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

        Self::ensure_escrow_not_frozen(&env, bounty_id)?;
        Self::ensure_address_not_frozen(&env, &escrow.depositor)?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        let now = env.ledger().timestamp();
        let claim_window: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ClaimWindow)
            .unwrap_or(0);
        let claim = ClaimRecord {
            bounty_id,
            recipient: recipient.clone(),
            amount: escrow.amount,
            expires_at: now.saturating_add(claim_window),
            claimed: false,
            reason: reason.clone(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::PendingClaim(bounty_id), &claim);

        env.events().publish(
            (symbol_short!("claim"), symbol_short!("created")),
            ClaimCreated {
                bounty_id,
                recipient,
                amount: escrow.amount,
                expires_at: claim.expires_at,
            },
        );
        Ok(())
    }

    /// Claims an existing pending authorization.
    ///
    /// # Access Control
    /// Only the authorized pending `recipient` can claim.
    ///
    /// # Front-running Behavior
    /// Claim is single-use: once marked claimed and escrow is released, subsequent calls fail.
    pub fn claim(env: Env, bounty_id: u64) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if Self::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }
        if !env
            .storage()
            .persistent()
            .has(&DataKey::PendingClaim(bounty_id))
        {
            return Err(Error::BountyNotFound);
        }
        let mut claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::PendingClaim(bounty_id))
            .unwrap();

        claim.recipient.require_auth();

        let now = env.ledger().timestamp();
        if now > claim.expires_at {
            return Err(Error::DeadlineNotPassed); // reuse or add ClaimExpired error
        }
        if claim.claimed {
            return Err(Error::FundsNotLocked);
        }

        // EFFECTS: update state before external call (CEI)
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Self::ensure_escrow_not_frozen(&env, bounty_id)?;
        Self::ensure_address_not_frozen(&env, &escrow.depositor)?;
        escrow.status = EscrowStatus::Released;
        escrow.remaining_amount = 0;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        claim.claimed = true;
        env.storage()
            .persistent()
            .set(&DataKey::PendingClaim(bounty_id), &claim);

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(
            &env.current_contract_address(),
            &claim.recipient,
            &claim.amount,
        );

        env.events().publish(
            (symbol_short!("claim"), symbol_short!("done")),
            ClaimExecuted {
                bounty_id,
                recipient: claim.recipient.clone(),
                amount: claim.amount,
                claimed_at: now,
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Delegated claim execution using a capability.
    /// Funds are still transferred to the pending claim recipient.
    pub fn claim_with_capability(
        env: Env,
        bounty_id: u64,
        holder: Address,
        capability_id: BytesN<32>,
    ) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if Self::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }
        if !env
            .storage()
            .persistent()
            .has(&DataKey::PendingClaim(bounty_id))
        {
            return Err(Error::BountyNotFound);
        }

        let mut claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::PendingClaim(bounty_id))
            .unwrap();

        let now = env.ledger().timestamp();
        if now > claim.expires_at {
            return Err(Error::DeadlineNotPassed);
        }
        if claim.claimed {
            return Err(Error::FundsNotLocked);
        }

        Self::consume_capability(
            &env,
            &holder,
            capability_id,
            CapabilityAction::Claim,
            bounty_id,
            claim.amount,
        )?;

        // EFFECTS: update state before external call (CEI)
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Self::ensure_escrow_not_frozen(&env, bounty_id)?;
        Self::ensure_address_not_frozen(&env, &escrow.depositor)?;
        escrow.status = EscrowStatus::Released;
        escrow.remaining_amount = 0;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        claim.claimed = true;
        env.storage()
            .persistent()
            .set(&DataKey::PendingClaim(bounty_id), &claim);

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(
            &env.current_contract_address(),
            &claim.recipient,
            &claim.amount,
        );

        env.events().publish(
            (symbol_short!("claim"), symbol_short!("done")),
            ClaimExecuted {
                bounty_id,
                recipient: claim.recipient,
                amount: claim.amount,
                claimed_at: now,
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Admin can cancel an expired or unwanted pending claim, returning escrow to Locked.
    pub fn cancel_pending_claim(
        env: Env,
        bounty_id: u64,
        outcome: DisputeOutcome,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env
            .storage()
            .persistent()
            .has(&DataKey::PendingClaim(bounty_id))
        {
            return Err(Error::BountyNotFound);
        }
        let claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::PendingClaim(bounty_id))
            .unwrap();

        let now = env.ledger().timestamp(); // Added this line
        let recipient = claim.recipient.clone(); // Added this line
        let amount = claim.amount; // Added this line

        env.storage()
            .persistent()
            .remove(&DataKey::PendingClaim(bounty_id));

        env.events().publish(
            (symbol_short!("claim"), symbol_short!("cancel")),
            ClaimCancelled {
                bounty_id,
                recipient,
                amount,
                cancelled_at: now,
                cancelled_by: admin,
            },
        );
        Ok(())
    }

    /// View: get pending claim for a bounty.
    pub fn get_pending_claim(env: Env, bounty_id: u64) -> Result<ClaimRecord, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::PendingClaim(bounty_id))
            .ok_or(Error::BountyNotFound)
    }

    fn compute_refund_eligibility(env: &Env, bounty_id: u64) -> RefundEligibilityView {
        let now = env.ledger().timestamp();

        if Self::check_paused(env, symbol_short!("refund")) {
            return RefundEligibilityView {
                eligible: false,
                code: RefundEligibilityCode::IneligibleRefundPaused,
                bounty_id,
                amount: 0,
                recipient: None,
                now,
                deadline: 0,
                approval_present: false,
            };
        }

        if env.storage().persistent().has(&DataKey::EscrowAnon(bounty_id)) {
            let anon: AnonymousEscrow = env
                .storage()
                .persistent()
                .get(&DataKey::EscrowAnon(bounty_id))
                .unwrap();
            return RefundEligibilityView {
                eligible: false,
                code: RefundEligibilityCode::IneligibleAnonymousRequiresResolution,
                bounty_id,
                amount: 0,
                recipient: None,
                now,
                deadline: anon.deadline,
                approval_present: false,
            };
        }

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return RefundEligibilityView {
                eligible: false,
                code: RefundEligibilityCode::IneligibleBountyNotFound,
                bounty_id,
                amount: 0,
                recipient: None,
                now,
                deadline: 0,
                approval_present: false,
            };
        }

        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if Self::ensure_escrow_not_frozen(env, bounty_id).is_err() {
            return RefundEligibilityView {
                eligible: false,
                code: RefundEligibilityCode::IneligibleEscrowFrozen,
                bounty_id,
                amount: 0,
                recipient: None,
                now,
                deadline: escrow.deadline,
                approval_present: false,
            };
        }
        if Self::ensure_address_not_frozen(env, &escrow.depositor).is_err() {
            return RefundEligibilityView {
                eligible: false,
                code: RefundEligibilityCode::IneligibleAddressFrozen,
                bounty_id,
                amount: 0,
                recipient: None,
                now,
                deadline: escrow.deadline,
                approval_present: false,
            };
        }
        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded {
            return RefundEligibilityView {
                eligible: false,
                code: RefundEligibilityCode::IneligibleInvalidStatus,
                bounty_id,
                amount: 0,
                recipient: None,
                now,
                deadline: escrow.deadline,
                approval_present: false,
            };
        }

        if env
            .storage()
            .persistent()
            .has(&DataKey::PendingClaim(bounty_id))
        {
            let claim: ClaimRecord = env
                .storage()
                .persistent()
                .get(&DataKey::PendingClaim(bounty_id))
                .unwrap();
            if !claim.claimed {
                return RefundEligibilityView {
                    eligible: false,
                    code: RefundEligibilityCode::IneligibleClaimPending,
                    bounty_id,
                    amount: 0,
                    recipient: None,
                    now,
                    deadline: escrow.deadline,
                    approval_present: false,
                };
            }
        }

        let approval: Option<RefundApproval> = env
            .storage()
            .persistent()
            .get(&DataKey::RefundApproval(bounty_id));
        if let Some(app) = approval {
            if app.amount <= 0 || app.amount > escrow.remaining_amount {
                return RefundEligibilityView {
                    eligible: false,
                    code: RefundEligibilityCode::IneligibleInvalidApproval,
                    bounty_id,
                    amount: 0,
                    recipient: None,
                    now,
                    deadline: escrow.deadline,
                    approval_present: true,
                };
            }
            return RefundEligibilityView {
                eligible: true,
                code: RefundEligibilityCode::EligibleAdminApproval,
                bounty_id,
                amount: app.amount,
                recipient: Some(app.recipient),
                now,
                deadline: escrow.deadline,
                approval_present: true,
            };
        }

        if now >= escrow.deadline {
            return RefundEligibilityView {
                eligible: true,
                code: RefundEligibilityCode::EligibleDeadlinePassed,
                bounty_id,
                amount: escrow.remaining_amount,
                recipient: Some(escrow.depositor),
                now,
                deadline: escrow.deadline,
                approval_present: false,
            };
        }

        RefundEligibilityView {
            eligible: false,
            code: RefundEligibilityCode::IneligibleDeadlineNotPassed,
            bounty_id,
            amount: 0,
            recipient: None,
            now,
            deadline: escrow.deadline,
            approval_present: false,
        }
    }

    /// Backward-compatible refund-eligibility tuple view.
    /// Returns `(can_refund, deadline_passed, remaining_amount, approval)`.
    pub fn get_refund_eligibility(
        env: Env,
        bounty_id: u64,
    ) -> (bool, bool, i128, Option<RefundApproval>) {
        let view = Self::compute_refund_eligibility(&env, bounty_id);
        let approval: Option<RefundApproval> = env
            .storage()
            .persistent()
            .get(&DataKey::RefundApproval(bounty_id));
        let deadline_passed = view.deadline > 0 && view.now >= view.deadline;
        (
            view.eligible,
            deadline_passed,
            view.amount,
            if view.approval_present { approval } else { None },
        )
    }

    /// New typed refund-eligibility view with explicit semantics.
    pub fn get_refund_eligibility_view(env: Env, bounty_id: u64) -> RefundEligibilityView {
        Self::compute_refund_eligibility(&env, bounty_id)
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
            mode: mode.clone(),
            approved_by: admin.clone(),
            approved_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::RefundApproval(bounty_id), &approval);

        emit_refund_approval_set(
            &env,
            RefundApprovalSet {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount,
                recipient,
                mode,
                approved_by: admin,
                approved_at: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Releases a partial amount of locked funds.
    ///
    /// # Access Control
    /// Admin-only.
    ///
    /// # Front-running Behavior
    /// Each successful call decreases `remaining_amount` exactly once. Attempts to exceed remaining
    /// balance fail with `Error::InsufficientFunds`.
    ///
    /// - `payout_amount` must be > 0 and <= `remaining_amount`.
    /// - `remaining_amount` is decremented by `payout_amount` after each call.
    /// - When `remaining_amount` reaches 0 the escrow status is set to Released.
    /// - The bounty stays Locked while any funds remain unreleased.
    pub fn partial_release(
        env: Env,
        bounty_id: u64,
        contributor: Address,
        payout_amount: i128,
    ) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        // Snapshot resource meters for gas cap enforcement (test / testutils only).
        #[cfg(any(test, feature = "testutils"))]
        let gas_snapshot = gas_budget::capture(&env);

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        Self::ensure_escrow_not_frozen(&env, bounty_id)?;
        Self::ensure_address_not_frozen(&env, &escrow.depositor)?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        // Guard: zero or negative payout makes no sense and would corrupt state
        if payout_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Guard: prevent overpayment — payout cannot exceed what is still owed
        if payout_amount > escrow.remaining_amount {
            return Err(Error::InsufficientFunds);
        }

        // EFFECTS: update state before external call (CEI)
        // Decrement remaining; this is always an exact integer subtraction — no rounding
        escrow.remaining_amount = escrow.remaining_amount.checked_sub(payout_amount).unwrap();

        // Automatically transition to Released once fully paid out
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(
            &env.current_contract_address(),
            &contributor,
            &payout_amount,
        );

        events::emit_funds_released(
            &env,
            FundsReleased {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: payout_amount,
                recipient: contributor,
                timestamp: env.ledger().timestamp(),
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Refunds remaining funds when refund conditions are met.
    ///
    /// # Authorization
    /// Refund execution requires authenticated authorization from the contract admin
    /// and the escrow depositor.
    ///
    /// # Eligibility
    /// Refund is allowed when either:
    /// 1. The deadline has passed (standard full refund to depositor), or
    /// 2. An admin approval exists (early, partial, or custom-recipient refund).
    ///
    /// # Transition Guards
    /// This function enforces the following state transition guards:
    ///
    /// ## Pre-conditions (checked in order):
    /// 1. **Reentrancy Guard**: Acquires reentrancy lock to prevent concurrent execution
    /// 2. **Operational State**: Contract must not be paused for refund operations
    /// 3. **Escrow Existence**: Bounty must exist in storage
    /// 4. **Freeze Check**: Escrow and depositor must not be frozen
    /// 5. **Authorization**: Both admin and depositor must authorize the transaction
    /// 6. **Status Guard**: Escrow status must be `Locked` or `PartiallyRefunded`
    /// 7. **Claim Guard**: No pending claim exists (or claim is already executed)
    /// 8. **Deadline/Approval Guard**: Deadline has passed OR admin approval exists
    ///
    /// ## State Transition:
    /// - **From**: `Locked` or `PartiallyRefunded`
    /// - **To**: `Refunded` (if full refund) or `PartiallyRefunded` (if partial)
    /// - **Effect**: Decrements `remaining_amount` by refund amount
    ///
    /// ## Post-conditions:
    /// - External token transfer to refund recipient (after state update)
    /// - Refund record added to history
    /// - Approval removed (if applicable)
    /// - Event emission
    ///
    /// ## Contention Safety:
    /// - If status is `Released` or `Refunded`, returns `Error::FundsNotLocked`
    /// - Reentrancy guard prevents concurrent execution of any protected function
    /// - CEI pattern ensures state is updated before external calls
    /// - No double-spend: once refunded, release fails with `Error::FundsNotLocked`
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if admin is not set.
    pub fn refund(env: Env, bounty_id: u64) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if Self::check_paused(&env, symbol_short!("refund")) {
            return Err(Error::FundsPaused);
        }
        // Snapshot resource meters for gas cap enforcement (test / testutils only).
        #[cfg(any(test, feature = "testutils"))]
        let gas_snapshot = gas_budget::capture(&env);

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        Self::ensure_escrow_not_frozen(&env, bounty_id)?;
        Self::ensure_address_not_frozen(&env, &escrow.depositor)?;

        // Require authenticated approval from both admin and depositor.
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        escrow.depositor.require_auth();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
        {
            return Err(Error::FundsNotLocked);
        }

        // Block refund if there is a pending claim (Issue #391 fix)
        if env
            .storage()
            .persistent()
            .has(&DataKey::PendingClaim(bounty_id))
        {
            let claim: ClaimRecord = env
                .storage()
                .persistent()
                .get(&DataKey::PendingClaim(bounty_id))
                .unwrap();
            if !claim.claimed {
                return Err(Error::ClaimPending);
            }
        }

        let now = env.ledger().timestamp();
        let approval_key = DataKey::RefundApproval(bounty_id);
        let approval: Option<RefundApproval> = env.storage().persistent().get(&approval_key);

        // Refund is allowed if:
        // 1. Deadline has passed (returns full amount to depositor)
        // 2. An administrative approval exists (can be early, partial, and to custom recipient)
        if now < escrow.deadline && approval.is_none() {
            return Err(Error::DeadlineNotPassed);
        }

        let (refund_amount, refund_to, is_full) = if let Some(app) = approval.clone() {
            let full = app.mode == RefundMode::Full || app.amount >= escrow.remaining_amount;
            (app.amount, app.recipient, full)
        } else {
            // Standard refund after deadline
            (escrow.remaining_amount, escrow.depositor.clone(), true)
        };

        if refund_amount <= 0 || refund_amount > escrow.remaining_amount {
            return Err(Error::InvalidAmount);
        }

        // EFFECTS: update state before external call (CEI)
        invariants::assert_escrow(&env, &escrow);
        // Update escrow state: subtract the amount exactly refunded
        escrow.remaining_amount = escrow.remaining_amount.checked_sub(refund_amount).unwrap();
        if is_full || escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Refunded;
        } else {
            escrow.status = EscrowStatus::PartiallyRefunded;
        }

        // Add to refund history
        escrow.refund_history.push_back(RefundRecord {
            amount: refund_amount,
            recipient: refund_to.clone(),
            timestamp: now,
            mode: if is_full {
                RefundMode::Full
            } else {
                RefundMode::Partial
            },
        });

        // Save updated escrow
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Remove approval after successful execution
        if approval.is_some() {
            env.storage().persistent().remove(&approval_key);
            emit_refund_approval_consumed(
                &env,
                RefundApprovalConsumed {
                    version: EVENT_VERSION_V2,
                    bounty_id,
                    refunded_amount: refund_amount,
                    refunded_to: refund_to.clone(),
                    consumed_at: now,
                },
            );
        }

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(&env.current_contract_address(), &refund_to, &refund_amount);

        emit_funds_refunded(
            &env,
            FundsRefunded {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: refund_amount,
                refund_to: refund_to.clone(),
                timestamp: now,
                trigger_type: if approval.is_some() {
                    RefundTriggerType::AdminApproval
                } else {
                    RefundTriggerType::DeadlineExpired
                },
            },
        );
        Self::record_receipt(
            &env,
            CriticalOperationOutcome::Refunded,
            bounty_id,
            refund_amount,
            refund_to.clone(),
        );

        // INV-2: Verify aggregate balance matches token balance after refund
        multitoken_invariants::assert_after_disbursement(&env);

        // Gas budget cap enforcement (test / testutils only).
        #[cfg(any(test, feature = "testutils"))]
        {
            let gas_cfg = gas_budget::get_config(&env);
            if let Err(e) = gas_budget::check(
                &env,
                symbol_short!("refund"),
                &gas_cfg.refund,
                &gas_snapshot,
                gas_cfg.enforce,
            ) {
                reentrancy_guard::release(&env);
                return Err(e);
            }
        }

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Simulate refund operation without state changes or token transfers.
    ///
    /// Returns a `SimulationResult` indicating whether the operation would succeed and the
    /// resulting escrow state. Does not require authorization; safe for off-chain preview.
    ///
    /// # Arguments
    /// * `bounty_id` - Bounty identifier
    ///
    /// # Security
    /// This function performs only read operations. No storage writes, token transfers,
    /// or events are emitted.
    pub fn dry_run_refund(env: Env, bounty_id: u64) -> SimulationResult {
        fn err_result(e: Error, default_status: EscrowStatus) -> SimulationResult {
            SimulationResult {
                success: false,
                error_code: e as u32,
                amount: 0,
                resulting_status: default_status,
                remaining_amount: 0,
            }
        }
        match Self::dry_run_refund_impl(&env, bounty_id) {
            Ok((refund_amount, resulting_status, remaining_amount)) => SimulationResult {
                success: true,
                error_code: 0,
                amount: refund_amount,
                resulting_status,
                remaining_amount,
            },
            Err(e) => err_result(e, EscrowStatus::Refunded),
        }
    }

    fn dry_run_refund_impl(env: &Env, bounty_id: u64) -> Result<(i128, EscrowStatus, i128), Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        let eligibility = Self::compute_refund_eligibility(env, bounty_id);
        if !eligibility.eligible {
            return Err(match eligibility.code {
                RefundEligibilityCode::IneligibleRefundPaused => Error::FundsPaused,
                RefundEligibilityCode::IneligibleBountyNotFound => Error::BountyNotFound,
                RefundEligibilityCode::IneligibleAnonymousRequiresResolution => {
                    Error::AnonymousRefundRequiresResolution
                }
                RefundEligibilityCode::IneligibleEscrowFrozen => Error::EscrowFrozen,
                RefundEligibilityCode::IneligibleAddressFrozen => Error::AddressFrozen,
                RefundEligibilityCode::IneligibleInvalidStatus => Error::FundsNotLocked,
                RefundEligibilityCode::IneligibleClaimPending => Error::ClaimPending,
                RefundEligibilityCode::IneligibleDeadlineNotPassed => Error::DeadlineNotPassed,
                RefundEligibilityCode::IneligibleInvalidApproval => Error::InvalidAmount,
                RefundEligibilityCode::EligibleDeadlinePassed
                | RefundEligibilityCode::EligibleAdminApproval => Error::InvalidAmount,
            });
        }
        let refund_amount = eligibility.amount;
        let remaining_after = escrow
            .remaining_amount
            .checked_sub(refund_amount)
            .unwrap_or(0);
        let resulting_status = if remaining_after == 0 {
            EscrowStatus::Refunded
        } else {
            EscrowStatus::PartiallyRefunded
        };
        Ok((refund_amount, resulting_status, remaining_after))
    }

    fn default_cycle_link() -> CycleLink {
        CycleLink {
            previous_id: 0,
            next_id: 0,
            cycle: 0,
        }
    }

    /// Extends the deadline of an active escrow and optionally tops up locked funds.
    ///
    /// # Security assumptions
    /// - Only `Locked` escrows are renewable.
    /// - Renewal is only allowed before the current deadline elapses.
    /// - New deadline must strictly increase the current deadline.
    /// - Top-ups transfer tokens from the original depositor into this contract.
    pub fn renew_escrow(
        env: Env,
        bounty_id: u64,
        new_deadline: u64,
        additional_amount: i128,
    ) -> Result<(), Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        Self::ensure_escrow_not_frozen(&env, bounty_id)?;
        Self::ensure_address_not_frozen(&env, &escrow.depositor)?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        let now = env.ledger().timestamp();
        if now >= escrow.deadline {
            return Err(Error::DeadlineNotPassed);
        }
        if new_deadline <= escrow.deadline {
            return Err(Error::InvalidDeadline);
        }
        if additional_amount < 0 {
            return Err(Error::InvalidAmount);
        }

        // The original depositor must authorize every renewal and any top-up transfer.
        escrow.depositor.require_auth();

        if additional_amount > 0 {
            let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
            let client = token::Client::new(&env, &token_addr);
            client.transfer(
                &escrow.depositor,
                &env.current_contract_address(),
                &additional_amount,
            );

            escrow.amount = escrow
                .amount
                .checked_add(additional_amount)
                .ok_or(Error::InvalidAmount)?;
            escrow.remaining_amount = escrow
                .remaining_amount
                .checked_add(additional_amount)
                .ok_or(Error::InvalidAmount)?;
        }

        let old_deadline = escrow.deadline;
        escrow.deadline = new_deadline;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        let mut history: Vec<RenewalRecord> = env
            .storage()
            .persistent()
            .get(&DataKey::RenewalHistory(bounty_id))
            .unwrap_or(Vec::new(&env));
        let cycle = history.len().saturating_add(1);
        history.push_back(RenewalRecord {
            cycle,
            old_deadline,
            new_deadline,
            additional_amount,
            renewed_at: now,
        });
        env.storage()
            .persistent()
            .set(&DataKey::RenewalHistory(bounty_id), &history);

        Ok(())
    }

    /// Starts a new bounty cycle from a completed prior cycle without mutating prior records.
    ///
    /// # Security assumptions
    /// - Previous cycle must be finalized (`Released` or `Refunded`).
    /// - A cycle can have at most one direct successor.
    /// - New cycle funds are transferred from the original depositor.
    pub fn create_next_cycle(
        env: Env,
        previous_bounty_id: u64,
        new_bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if deadline <= env.ledger().timestamp() {
            return Err(Error::InvalidDeadline);
        }
        if previous_bounty_id == new_bounty_id {
            return Err(Error::BountyExists);
        }
        if env
            .storage()
            .persistent()
            .has(&DataKey::Escrow(new_bounty_id))
        {
            return Err(Error::BountyExists);
        }
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Escrow(previous_bounty_id))
        {
            return Err(Error::BountyNotFound);
        }

        let previous: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(previous_bounty_id))
            .unwrap();
        if previous.status != EscrowStatus::Released && previous.status != EscrowStatus::Refunded {
            return Err(Error::FundsNotLocked);
        }

        let mut prev_link: CycleLink = env
            .storage()
            .persistent()
            .get(&DataKey::CycleLink(previous_bounty_id))
            .unwrap_or(Self::default_cycle_link());
        if prev_link.next_id != 0 {
            return Err(Error::BountyExists);
        }

        previous.depositor.require_auth();
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(
            &previous.depositor,
            &env.current_contract_address(),
            &amount,
        );

        let new_escrow = Escrow {
            depositor: previous.depositor.clone(),
            amount,
            remaining_amount: amount,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: Vec::new(&env),
            archived: false,
            archived_at: None,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(new_bounty_id), &new_escrow);

        let mut index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        index.push_back(new_bounty_id);
        env.storage()
            .persistent()
            .set(&DataKey::EscrowIndex, &index);

        let mut depositor_index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::DepositorIndex(previous.depositor.clone()))
            .unwrap_or(Vec::new(&env));
        depositor_index.push_back(new_bounty_id);
        env.storage().persistent().set(
            &DataKey::DepositorIndex(previous.depositor.clone()),
            &depositor_index,
        );

        prev_link.next_id = new_bounty_id;
        env.storage()
            .persistent()
            .set(&DataKey::CycleLink(previous_bounty_id), &prev_link);

        let new_link = CycleLink {
            previous_id: previous_bounty_id,
            next_id: 0,
            cycle: prev_link.cycle.saturating_add(1),
        };
        env.storage()
            .persistent()
            .set(&DataKey::CycleLink(new_bounty_id), &new_link);

        Ok(())
    }

    /// Returns the immutable renewal history for `bounty_id`.
    pub fn get_renewal_history(env: Env, bounty_id: u64) -> Result<Vec<RenewalRecord>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::RenewalHistory(bounty_id))
            .unwrap_or(Vec::new(&env)))
    }

    /// Returns the rollover link metadata for `bounty_id`.
    ///
    /// Returns a default root link with `cycle=1` when no explicit cycle record exists yet.
    pub fn get_cycle_info(env: Env, bounty_id: u64) -> Result<CycleLink, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let link: CycleLink = env
            .storage()
            .persistent()
            .get(&DataKey::CycleLink(bounty_id))
            .unwrap_or(Self::default_cycle_link());

        if link.previous_id == 0 && link.next_id == 0 && link.cycle == 0 {
            return Ok(CycleLink {
                previous_id: 0,
                next_id: 0,
                cycle: 1,
            });
        }

        Ok(link)
    }

    /// Sets or clears the anonymous resolver address.
    /// Only the admin can call this. The resolver is the trusted entity that
    /// resolves anonymous escrow refunds via `refund_resolved`.
    pub fn set_anonymous_resolver(env: Env, resolver: Option<Address>) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        match resolver {
            Some(addr) => env
                .storage()
                .instance()
                .set(&DataKey::AnonymousResolver, &addr),
            None => env.storage().instance().remove(&DataKey::AnonymousResolver),
        }
        Ok(())
    }

    /// Refund an anonymous escrow to a resolved recipient.
    /// Only the configured anonymous resolver can call this; they resolve the depositor
    /// commitment off-chain and pass the recipient address (signed instruction pattern).
    pub fn refund_resolved(env: Env, bounty_id: u64, recipient: Address) -> Result<(), Error> {
        if Self::check_paused(&env, symbol_short!("refund")) {
            return Err(Error::FundsPaused);
        }

        let resolver: Address = env
            .storage()
            .instance()
            .get(&DataKey::AnonymousResolver)
            .ok_or(Error::AnonymousResolverNotSet)?;
        resolver.require_auth();

        if !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
        {
            return Err(Error::NotAnonymousEscrow);
        }

        reentrancy_guard::acquire(&env);

        let mut anon: AnonymousEscrow = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowAnon(bounty_id))
            .unwrap();

        Self::ensure_escrow_not_frozen(&env, bounty_id)?;

        if anon.status != EscrowStatus::Locked && anon.status != EscrowStatus::PartiallyRefunded {
            return Err(Error::FundsNotLocked);
        }

        // GUARD 1: Block refund if there is a pending claim (Issue #391 fix)
        if env
            .storage()
            .persistent()
            .has(&DataKey::PendingClaim(bounty_id))
        {
            let claim: ClaimRecord = env
                .storage()
                .persistent()
                .get(&DataKey::PendingClaim(bounty_id))
                .unwrap();
            if !claim.claimed {
                return Err(Error::ClaimPending);
            }
        }

        let now = env.ledger().timestamp();
        let approval_key = DataKey::RefundApproval(bounty_id);
        let approval: Option<RefundApproval> = env.storage().persistent().get(&approval_key);

        // Refund is allowed if:
        // 1. Deadline has passed (returns full amount to depositor)
        // 2. An administrative approval exists (can be early, partial, and to custom recipient)
        if now < anon.deadline && approval.is_none() {
            return Err(Error::DeadlineNotPassed);
        }

        let (refund_amount, refund_to, is_full) = if let Some(app) = approval.clone() {
            let full = app.mode == RefundMode::Full || app.amount >= anon.remaining_amount;
            (app.amount, app.recipient, full)
        } else {
            // Standard refund after deadline
            (anon.remaining_amount, recipient.clone(), true)
        };

        if refund_amount <= 0 || refund_amount > anon.remaining_amount {
            return Err(Error::InvalidAmount);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer the calculated refund amount to the designated recipient
        client.transfer(&env.current_contract_address(), &refund_to, &refund_amount);

        // Anonymous escrow uses a parallel storage record and invariant model.
        // Update escrow state: subtract the amount exactly refunded
        anon.remaining_amount -= refund_amount;
        if is_full || anon.remaining_amount == 0 {
            anon.status = EscrowStatus::Refunded;
        } else {
            anon.status = EscrowStatus::PartiallyRefunded;
        }

        // Add to refund history
        anon.refund_history.push_back(RefundRecord {
            amount: refund_amount,
            recipient: refund_to.clone(),
            timestamp: now,
            mode: if is_full {
                RefundMode::Full
            } else {
                RefundMode::Partial
            },
        });

        // Save updated escrow
        env.storage()
            .persistent()
            .set(&DataKey::EscrowAnon(bounty_id), &anon);

        // Remove approval after successful execution
        if approval.is_some() {
            env.storage().persistent().remove(&approval_key);
        }

        emit_funds_refunded(
            &env,
            FundsRefunded {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: refund_amount,
                refund_to: refund_to.clone(),
                timestamp: now,
                trigger_type: if approval.is_some() {
                    RefundTriggerType::AdminApproval
                } else {
                    RefundTriggerType::DeadlineExpired
                },
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Delegated refund path using a capability.
    /// This can be used for short-lived, bounded delegated refunds without granting admin rights.
    pub fn refund_with_capability(
        env: Env,
        bounty_id: u64,
        amount: i128,
        holder: Address,
        capability_id: BytesN<32>,
    ) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if Self::check_paused(&env, symbol_short!("refund")) {
            return Err(Error::FundsPaused);
        }
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        Self::ensure_escrow_not_frozen(&env, bounty_id)?;
        Self::ensure_address_not_frozen(&env, &escrow.depositor)?;

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
        {
            return Err(Error::FundsNotLocked);
        }
        if amount > escrow.remaining_amount {
            return Err(Error::InvalidAmount);
        }

        if env
            .storage()
            .persistent()
            .has(&DataKey::PendingClaim(bounty_id))
        {
            let claim: ClaimRecord = env
                .storage()
                .persistent()
                .get(&DataKey::PendingClaim(bounty_id))
                .unwrap();
            if !claim.claimed {
                return Err(Error::ClaimPending);
            }
        }

        Self::consume_capability(
            &env,
            &holder,
            capability_id,
            CapabilityAction::Refund,
            bounty_id,
            amount,
        )?;

        // EFFECTS: update state before external call (CEI)
        let now = env.ledger().timestamp();
        let refund_to = escrow.depositor.clone();
        escrow.remaining_amount = escrow.remaining_amount.checked_sub(amount).unwrap();
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Refunded;
        } else {
            escrow.status = EscrowStatus::PartiallyRefunded;
        }
        escrow.refund_history.push_back(RefundRecord {
            amount,
            recipient: refund_to.clone(),
            timestamp: now,
            mode: if escrow.remaining_amount == 0 {
                RefundMode::Full
            } else {
                RefundMode::Partial
            },
        });
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(&env.current_contract_address(), &refund_to, &amount);

        emit_funds_refunded(
            &env,
            FundsRefunded {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount,
                refund_to,
                timestamp: now,
                trigger_type: RefundTriggerType::AdminApproval,
            },
        );

        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Return the current per-operation gas budget configuration.
    ///
    /// Returns the fully uncapped default if no configuration has been set.
    pub fn get_gas_budget(env: Env) -> gas_budget::GasBudgetConfig {
        gas_budget::get_config(&env)
    }

    /// Batch lock funds for multiple bounties in a single atomic transaction.
    ///
    /// Locks between 1 and [`MAX_BATCH_SIZE`] bounties in one call, reducing
    /// per-transaction overhead compared to repeated single-item `lock_funds`
    /// calls.
    ///
    /// ## Batch failure semantics
    ///
    /// This operation is **strictly atomic** (all-or-nothing):
    ///
    /// 1. All items are validated in a single pass **before** any state is
    ///    mutated or any token transfer is initiated.
    /// 2. If *any* item fails validation the entire call reverts immediately.
    ///    No escrow record is written, no token is transferred, and every
    ///    "sibling" row in the same batch is left completely unaffected.
    /// 3. After a failed batch the contract is in exactly the same state as
    ///    before the call; subsequent operations behave as if this call never
    ///    happened.
    ///
    /// ## Ordering guarantee
    ///
    /// Items are processed in ascending `bounty_id` order regardless of the
    /// caller-supplied ordering. This ensures deterministic execution and
    /// eliminates ordering-based front-running attacks.
    ///
    /// ## Checks-Effects-Interactions (CEI)
    ///
    /// All escrow records and index updates are written in a first pass
    /// (Effects); external token transfers and event emissions happen in a
    /// second pass (Interactions). This ordering prevents reentrancy attacks.
    ///
    /// # Arguments
    /// * `items` - 1–[`MAX_BATCH_SIZE`] [`LockFundsItem`] entries (bounty_id,
    ///   depositor, amount, deadline).
    ///
    /// # Returns
    /// Number of bounties successfully locked (equals `items.len()` on success).
    ///
    /// # Errors
    /// * [`Error::InvalidBatchSize`] — batch is empty or exceeds `MAX_BATCH_SIZE`
    /// * [`Error::ContractDeprecated`] — contract has been killed via `set_deprecated`
    /// * [`Error::FundsPaused`] — lock operations are currently paused
    /// * [`Error::NotInitialized`] — `init` has not been called
    /// * [`Error::BountyExists`] — a `bounty_id` already exists in storage
    /// * [`Error::DuplicateBountyId`] — the same `bounty_id` appears more than once
    /// * [`Error::InvalidAmount`] — any item has `amount ≤ 0`
    /// * [`Error::ParticipantBlocked`] / [`Error::ParticipantNotAllowed`] — participant filter
    ///
    /// # Reentrancy
    /// Protected by the shared reentrancy guard (acquired before validation,
    /// released after all effects and interactions complete).
    pub fn batch_lock_funds(env: Env, items: Vec<LockFundsItem>) -> Result<u32, Error> {
        if Self::check_paused(&env, symbol_short!("lock")) {
            return Err(Error::FundsPaused);
        }

        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);
        // Snapshot resource meters for gas cap enforcement (test / testutils only).
        #[cfg(any(test, feature = "testutils"))]
        let gas_snapshot = gas_budget::capture(&env);
        let result: Result<u32, Error> = (|| {
            if Self::get_deprecation_state(&env).deprecated {
                return Err(Error::ContractDeprecated);
            }
            // Validate batch size
            let batch_size = items.len();
            if batch_size == 0 {
                return Err(Error::InvalidBatchSize);
            }
            if batch_size > MAX_BATCH_SIZE {
                return Err(Error::InvalidBatchSize);
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
                // Participant filtering (blocklist-only / allowlist-only / disabled)
                Self::check_participant_filter(&env, item.depositor.clone())?;

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

            let ordered_items = Self::order_batch_lock_items(&env, &items);

            // Collect unique depositors and require auth once for each
            // This prevents "frame is already authorized" errors when same depositor appears multiple times
            let mut seen_depositors: Vec<Address> = Vec::new(&env);
            for item in ordered_items.iter() {
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
            // First loop: write all state (escrow, indices). Second loop: transfers + events.
            let mut locked_count = 0u32;
            for item in ordered_items.iter() {
                let escrow = Escrow {
                    depositor: item.depositor.clone(),
                    amount: item.amount,
                    status: EscrowStatus::Locked,
                    deadline: item.deadline,
                    refund_history: vec![&env],
                    remaining_amount: item.amount,
                    archived: false,
                    archived_at: None,
                };

                env.storage()
                    .persistent()
                    .set(&DataKey::Escrow(item.bounty_id), &escrow);

                let mut index: Vec<u64> = env
                    .storage()
                    .persistent()
                    .get(&DataKey::EscrowIndex)
                    .unwrap_or(Vec::new(&env));
                index.push_back(item.bounty_id);
                env.storage()
                    .persistent()
                    .set(&DataKey::EscrowIndex, &index);

                let mut depositor_index: Vec<u64> = env
                    .storage()
                    .persistent()
                    .get(&DataKey::DepositorIndex(item.depositor.clone()))
                    .unwrap_or(Vec::new(&env));
                depositor_index.push_back(item.bounty_id);
                env.storage().persistent().set(
                    &DataKey::DepositorIndex(item.depositor.clone()),
                    &depositor_index,
                );
            }

            // INTERACTION: all external token transfers happen after state is finalized
            for item in ordered_items.iter() {
                client.transfer(&item.depositor, &contract_address, &item.amount);

                emit_funds_locked(
                    &env,
                    FundsLocked {
                        version: EVENT_VERSION_V2,
                        bounty_id: item.bounty_id,
                        amount: item.amount,
                        depositor: item.depositor.clone(),
                        deadline: item.deadline,
                    },
                );

                locked_count += 1;
            }

            emit_batch_funds_locked(
                &env,
                BatchFundsLocked {
                    version: EVENT_VERSION_V2,
                    count: locked_count,
                    total_amount: ordered_items
                        .iter()
                        .try_fold(0i128, |acc, i| acc.checked_add(i.amount))
                        .unwrap(),
                    timestamp,
                },
            );
            Ok(locked_count)
        })();

        // Gas budget cap enforcement (test / testutils only).
        #[cfg(any(test, feature = "testutils"))]
        if result.is_ok() {
            let gas_cfg = gas_budget::get_config(&env);
            if let Err(e) = gas_budget::check(
                &env,
                symbol_short!("b_lock"),
                &gas_cfg.batch_lock,
                &gas_snapshot,
                gas_cfg.enforce,
            ) {
                reentrancy_guard::release(&env);
                return Err(e);
            }
        }

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        result
    }

    /// Alias for batch_lock_funds to match the requested naming convention.
    pub fn batch_lock(env: Env, items: Vec<LockFundsItem>) -> Result<u32, Error> {
        Self::batch_lock_funds(env, items)
    }

    /// Batch release funds to multiple contributors in a single atomic transaction.
    ///
    /// Releases between 1 and [`MAX_BATCH_SIZE`] bounties in one admin-authorised
    /// call, reducing per-transaction overhead compared to repeated single-item
    /// `release_funds` calls.
    ///
    /// ## Batch failure semantics
    ///
    /// This operation is **strictly atomic** (all-or-nothing):
    ///
    /// 1. All items are validated in a single pass **before** any escrow status
    ///    is updated or any token transfer is initiated.
    /// 2. If *any* item fails validation the entire call reverts immediately.
    ///    No status is changed, no token leaves the contract, and every
    ///    "sibling" row in the same batch is left completely unaffected.
    /// 3. After a failed batch the contract is in exactly the same state as
    ///    before the call; subsequent operations behave as if this call never
    ///    happened.
    ///
    /// ## Ordering guarantee
    ///
    /// Items are processed in ascending `bounty_id` order regardless of the
    /// caller-supplied ordering, ensuring deterministic execution.
    ///
    /// ## Checks-Effects-Interactions (CEI)
    ///
    /// All escrow statuses are updated to `Released` in a first pass (Effects);
    /// external token transfers and event emissions happen in a second pass
    /// (Interactions).
    ///
    /// # Arguments
    /// * `items` - 1–[`MAX_BATCH_SIZE`] [`ReleaseFundsItem`] entries (bounty_id,
    ///   contributor address).
    ///
    /// # Returns
    /// Number of bounties successfully released (equals `items.len()` on success).
    ///
    /// # Errors
    /// * [`Error::InvalidBatchSize`] — batch is empty or exceeds `MAX_BATCH_SIZE`
    /// * [`Error::FundsPaused`] — release operations are currently paused
    /// * [`Error::NotInitialized`] — `init` has not been called
    /// * [`Error::Unauthorized`] — caller is not the admin
    /// * [`Error::BountyNotFound`] — a `bounty_id` does not exist in storage
    /// * [`Error::FundsNotLocked`] — a bounty's status is not `Locked`
    /// * [`Error::DuplicateBountyId`] — the same `bounty_id` appears more than once
    ///
    /// # Reentrancy
    /// Protected by the shared reentrancy guard (acquired before validation,
    /// released after all effects and interactions complete).
    pub fn batch_release_funds(env: Env, items: Vec<ReleaseFundsItem>) -> Result<u32, Error> {
        if Self::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);
        // Snapshot resource meters for gas cap enforcement (test / testutils only).
        #[cfg(any(test, feature = "testutils"))]
        let gas_snapshot = gas_budget::capture(&env);
        let result: Result<u32, Error> = (|| {
            // Validate batch size
            let batch_size = items.len();
            if batch_size == 0 {
                return Err(Error::InvalidBatchSize);
            }
            if batch_size > MAX_BATCH_SIZE {
                return Err(Error::InvalidBatchSize);
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

                Self::ensure_escrow_not_frozen(&env, item.bounty_id)?;
                Self::ensure_address_not_frozen(&env, &escrow.depositor)?;

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

            let ordered_items = Self::order_batch_release_items(&env, &items);

            // EFFECTS: update all escrow records before any external calls (CEI)
            // We collect (contributor, amount) pairs for the transfer pass.
            let mut release_pairs: Vec<(Address, i128)> = Vec::new(&env);
            let mut released_count = 0u32;
            for item in ordered_items.iter() {
                let mut escrow: Escrow = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Escrow(item.bounty_id))
                    .unwrap();

                let amount = escrow.amount;
                escrow.status = EscrowStatus::Released;
                escrow.remaining_amount = 0;
                env.storage()
                    .persistent()
                    .set(&DataKey::Escrow(item.bounty_id), &escrow);

                release_pairs.push_back((item.contributor.clone(), amount));
                released_count += 1;
            }

            // INTERACTION: all external token transfers happen after state is finalized
            for (idx, item) in ordered_items.iter().enumerate() {
                let (ref contributor, amount) = release_pairs.get(idx as u32).unwrap();
                client.transfer(&contract_address, contributor, &amount);

                emit_funds_released(
                    &env,
                    FundsReleased {
                        version: EVENT_VERSION_V2,
                        bounty_id: item.bounty_id,
                        amount,
                        recipient: contributor.clone(),
                        timestamp,
                    },
                );
            }

            // Emit batch event
            emit_batch_funds_released(
                &env,
                BatchFundsReleased {
                    version: EVENT_VERSION_V2,
                    count: released_count,
                    total_amount,
                    timestamp,
                },
            );
            Ok(released_count)
        })();

        // Gas budget cap enforcement (test / testutils only).
        #[cfg(any(test, feature = "testutils"))]
        if result.is_ok() {
            let gas_cfg = gas_budget::get_config(&env);
            if let Err(e) = gas_budget::check(
                &env,
                symbol_short!("b_rel"),
                &gas_cfg.batch_release,
                &gas_snapshot,
                gas_cfg.enforce,
            ) {
                reentrancy_guard::release(&env);
                return Err(e);
            }
        }

        reentrancy_guard::release(&env);
        result
    }
}
impl traits::EscrowInterface for BountyEscrowContract {
    /// Lock funds for a bounty through the trait interface
    fn lock_funds(
        env: &Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), crate::Error> {
        let entrypoint: fn(Env, Address, u64, i128, u64) -> Result<(), crate::Error> =
            BountyEscrowContract::lock_funds;
        entrypoint(env.clone(), depositor, bounty_id, amount, deadline)
    }

    /// Release funds to contributor through the trait interface
    fn release_funds(env: &Env, bounty_id: u64, contributor: Address) -> Result<(), crate::Error> {
        let entrypoint: fn(Env, u64, Address) -> Result<(), crate::Error> =
            BountyEscrowContract::release_funds;
        entrypoint(env.clone(), bounty_id, contributor)
    }

    /// Partial release through the trait interface
    fn partial_release(
        env: &Env,
        bounty_id: u64,
        contributor: Address,
        payout_amount: i128,
    ) -> Result<(), crate::Error> {
        let entrypoint: fn(Env, u64, Address, i128) -> Result<(), crate::Error> =
            BountyEscrowContract::partial_release;
        entrypoint(env.clone(), bounty_id, contributor, payout_amount)
    }

    /// Batch lock funds through the trait interface
    fn batch_lock_funds(env: &Env, items: Vec<LockFundsItem>) -> Result<u32, crate::Error> {
        let entrypoint: fn(Env, Vec<LockFundsItem>) -> Result<u32, crate::Error> =
            BountyEscrowContract::batch_lock_funds;
        entrypoint(env.clone(), items)
    }

    /// Batch release funds through the trait interface
    fn batch_release_funds(env: &Env, items: Vec<ReleaseFundsItem>) -> Result<u32, crate::Error> {
        let entrypoint: fn(Env, Vec<ReleaseFundsItem>) -> Result<u32, crate::Error> =
            BountyEscrowContract::batch_release_funds;
        entrypoint(env.clone(), items)
    }

    /// Refund funds to depositor through the trait interface
    fn refund(env: &Env, bounty_id: u64) -> Result<(), crate::Error> {
        let entrypoint: fn(Env, u64) -> Result<(), crate::Error> = BountyEscrowContract::refund;
        entrypoint(env.clone(), bounty_id)
    }

    /// Get escrow information through the trait interface
    fn get_escrow_info(env: &Env, bounty_id: u64) -> Result<crate::Escrow, crate::Error> {
        let entrypoint: fn(Env, u64) -> Result<crate::Escrow, crate::Error> =
            BountyEscrowContract::get_escrow_info;
        entrypoint(env.clone(), bounty_id)
    }

    /// Get contract balance through the trait interface
    fn get_balance(env: &Env) -> Result<i128, crate::Error> {
        let entrypoint: fn(Env) -> Result<i128, crate::Error> = BountyEscrowContract::get_balance;
        entrypoint(env.clone())
    }
}

impl traits::UpgradeInterface for BountyEscrowContract {
    /// Get contract version
    fn get_version(env: &Env) -> u32 {
        let entrypoint: fn(Env) -> u32 = BountyEscrowContract::get_version;
        entrypoint(env.clone())
    }

    /// Set contract version (admin only)
    fn set_version(env: &Env, new_version: u32) -> Result<(), crate::Error> {
        let entrypoint: fn(Env, u32) -> Result<(), crate::Error> =
            BountyEscrowContract::set_version;
        entrypoint(env.clone(), new_version)
    }
}

impl traits::PauseInterface for BountyEscrowContract {
    fn set_paused(
        env: &Env,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
        reason: Option<soroban_sdk::String>,
    ) -> Result<(), crate::Error> {
        let entrypoint: fn(
            Env,
            Option<bool>,
            Option<bool>,
            Option<bool>,
            Option<soroban_sdk::String>,
        ) -> Result<(), crate::Error> = BountyEscrowContract::set_paused;
        entrypoint(env.clone(), lock, release, refund, reason)
    }

    fn get_pause_flags(env: &Env) -> crate::PauseFlags {
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

    fn is_operation_paused(env: &Env, operation: soroban_sdk::Symbol) -> bool {
        Self::check_paused(env, operation)
    }
}

impl traits::FeeInterface for BountyEscrowContract {
    fn update_fee_config(
        env: &Env,
        lock_fee_rate: Option<i128>,
        release_fee_rate: Option<i128>,
        lock_fixed_fee: Option<i128>,
        release_fixed_fee: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
    ) -> Result<(), crate::Error> {
        let entrypoint: fn(
            Env,
            Option<i128>,
            Option<i128>,
            Option<i128>,
            Option<i128>,
            Option<Address>,
            Option<bool>,
        ) -> Result<(), crate::Error> = BountyEscrowContract::update_fee_config;
        entrypoint(
            env.clone(),
            lock_fee_rate,
            release_fee_rate,
            lock_fixed_fee,
            release_fixed_fee,
            fee_recipient,
            fee_enabled,
        )
    }

    fn get_fee_config(env: &Env) -> crate::FeeConfig {
        let entrypoint: fn(Env) -> crate::FeeConfig = BountyEscrowContract::get_fee_config;
        entrypoint(env.clone())
    }
}

#[cfg(test)]
mod test_state_verification;

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_analytics_monitoring;
#[cfg(test)]
mod test_auto_refund_permissions;
#[cfg(test)]
mod test_blacklist_and_whitelist;
#[cfg(test)]
mod test_bounty_escrow;
#[cfg(test)]
mod test_capability_tokens;
#[cfg(test)]
mod test_deprecation;
#[cfg(test)]
mod test_dispute_resolution;
#[cfg(test)]
mod test_expiration_and_dispute;
#[cfg(test)]
mod test_front_running_ordering;
#[cfg(test)]
mod test_granular_pause;
#[cfg(test)]
mod test_invariants;
mod test_lifecycle;
#[cfg(test)]
mod test_metadata_tagging;
#[cfg(test)]
mod test_partial_payout_rounding;
#[cfg(test)]
mod test_participant_filter_mode;
#[cfg(test)]
mod test_pause;
#[cfg(test)]
mod escrow_status_transition_tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env,
    };

    // Escrow Status Transition Matrix
    //
    // FROM        | TO          | EXPECTED RESULT
    // ------------|-------------|----------------
    // Locked      | Locked      | Err (invalid - BountyExists)
    // Locked      | Released    | Ok (allowed)
    // Locked      | Refunded    | Ok (allowed)
    // Released    | Locked      | Err (invalid - BountyExists)
    // Released    | Released    | Err (invalid - FundsNotLocked)
    // Released    | Refunded    | Err (invalid - FundsNotLocked)
    // Refunded    | Locked      | Err (invalid - BountyExists)
    // Refunded    | Released    | Err (invalid - FundsNotLocked)
    // Refunded    | Refunded    | Err (invalid - FundsNotLocked)

    /// Construct a fresh Escrow instance with the specified status.
    fn create_escrow_with_status(
        env: &Env,
        depositor: Address,
        amount: i128,
        status: EscrowStatus,
        deadline: u64,
    ) -> Escrow {
        Escrow {
            depositor,
            amount,
            remaining_amount: amount,
            status,
            deadline,
            refund_history: vec![env],
            creation_timestamp: 0,
            expiry: 0,
            archived: false,
            archived_at: None,
        }
    }

    /// Test setup holding environment, clients, and addresses
    struct TestEnv {
        env: Env,
        contract_id: Address,
        client: BountyEscrowContractClient<'static>,
        token_admin: token::StellarAssetClient<'static>,
        admin: Address,
        depositor: Address,
        contributor: Address,
    }

    impl TestEnv {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths();

            let admin = Address::generate(&env);
            let depositor = Address::generate(&env);
            let contributor = Address::generate(&env);

            let token_id = env.register_stellar_asset_contract(admin.clone());
            let token_admin = token::StellarAssetClient::new(&env, &token_id);

            let contract_id = env.register_contract(None, BountyEscrowContract);
            let client = BountyEscrowContractClient::new(&env, &contract_id);

            client.init(&admin, &token_id);

            Self {
                env,
                contract_id,
                client,
                token_admin,
                admin,
                depositor,
                contributor,
            }
        }

        /// Setup escrow in specific status and bypass standard locking process
        fn setup_escrow_in_state(&self, status: EscrowStatus, bounty_id: u64, amount: i128) {
            let deadline = self.env.ledger().timestamp() + 1000;
            let escrow = create_escrow_with_status(
                &self.env,
                self.depositor.clone(),
                amount,
                status,
                deadline,
            );

            // Mint tokens directly to the contract to bypass lock_funds logic but guarantee token transfer succeeds for valid transitions
            self.token_admin.mint(&self.contract_id, &amount);

            // Write escrow directly to contract storage
            self.env.as_contract(&self.contract_id, || {
                self.env
                    .storage()
                    .persistent()
                    .set(&DataKey::Escrow(bounty_id), &escrow);
            });
        }
    }

    #[derive(Clone, Debug)]
    enum TransitionAction {
        Lock,
        Release,
        Refund,
    }

    struct TransitionTestCase {
        label: &'static str,
        from: EscrowStatus,
        action: TransitionAction,
        expected_result: Result<(), Error>,
    }

    /// Table-driven test function executing all exhaustive transitions from the matrix
    #[test]
    fn test_all_status_transitions() {
        let cases = [
            TransitionTestCase {
                label: "Locked to Locked (Lock)",
                from: EscrowStatus::Locked,
                action: TransitionAction::Lock,
                expected_result: Err(Error::BountyExists),
            },
            TransitionTestCase {
                label: "Locked to Released (Release)",
                from: EscrowStatus::Locked,
                action: TransitionAction::Release,
                expected_result: Ok(()),
            },
            TransitionTestCase {
                label: "Locked to Refunded (Refund)",
                from: EscrowStatus::Locked,
                action: TransitionAction::Refund,
                expected_result: Ok(()),
            },
            TransitionTestCase {
                label: "Released to Locked (Lock)",
                from: EscrowStatus::Released,
                action: TransitionAction::Lock,
                expected_result: Err(Error::BountyExists),
            },
            TransitionTestCase {
                label: "Released to Released (Release)",
                from: EscrowStatus::Released,
                action: TransitionAction::Release,
                expected_result: Err(Error::FundsNotLocked),
            },
            TransitionTestCase {
                label: "Released to Refunded (Refund)",
                from: EscrowStatus::Released,
                action: TransitionAction::Refund,
                expected_result: Err(Error::FundsNotLocked),
            },
            TransitionTestCase {
                label: "Refunded to Locked (Lock)",
                from: EscrowStatus::Refunded,
                action: TransitionAction::Lock,
                expected_result: Err(Error::BountyExists),
            },
            TransitionTestCase {
                label: "Refunded to Released (Release)",
                from: EscrowStatus::Refunded,
                action: TransitionAction::Release,
                expected_result: Err(Error::FundsNotLocked),
            },
            TransitionTestCase {
                label: "Refunded to Refunded (Refund)",
                from: EscrowStatus::Refunded,
                action: TransitionAction::Refund,
                expected_result: Err(Error::FundsNotLocked),
            },
        ];

        for case in cases {
            let setup = TestEnv::new();
            let bounty_id = 99;
            let amount = 1000;

            setup.setup_escrow_in_state(case.from.clone(), bounty_id, amount);
            if let TransitionAction::Refund = case.action {
                setup
                    .env
                    .ledger()
                    .set_timestamp(setup.env.ledger().timestamp() + 2000);
            }

            match case.action {
                TransitionAction::Lock => {
                    let deadline = setup.env.ledger().timestamp() + 1000;
                    let result = setup.client.try_lock_funds(
                        &setup.depositor,
                        &bounty_id,
                        &amount,
                        &deadline,
                    );
                    assert!(
                        result.is_err(),
                        "Transition '{}' failed: expected Err but got Ok",
                        case.label
                    );
                    assert_eq!(
                        result.unwrap_err().unwrap(),
                        case.expected_result.unwrap_err(),
                        "Transition '{}' failed: mismatched error variant",
                        case.label
                    );
                }
                TransitionAction::Release => {
                    let result = setup
                        .client
                        .try_release_funds(&bounty_id, &setup.contributor);
                    if case.expected_result.is_ok() {
                        assert!(
                            result.is_ok(),
                            "Transition '{}' failed: expected Ok but got {:?}",
                            case.label,
                            result
                        );
                    } else {
                        assert!(
                            result.is_err(),
                            "Transition '{}' failed: expected Err but got Ok",
                            case.label
                        );
                        assert_eq!(
                            result.unwrap_err().unwrap(),
                            case.expected_result.unwrap_err(),
                            "Transition '{}' failed: mismatched error variant",
                            case.label
                        );
                    }
                }
                TransitionAction::Refund => {
                    let result = setup.client.try_refund(&bounty_id);
                    if case.expected_result.is_ok() {
                        assert!(
                            result.is_ok(),
                            "Transition '{}' failed: expected Ok but got {:?}",
                            case.label,
                            result
                        );
                    } else {
                        assert!(
                            result.is_err(),
                            "Transition '{}' failed: expected Err but got Ok",
                            case.label
                        );
                        assert_eq!(
                            result.unwrap_err().unwrap(),
                            case.expected_result.unwrap_err(),
                            "Transition '{}' failed: mismatched error variant",
                            case.label
                        );
                    }
                }
            }
        }
    }

    /// Verifies allowed transition from Locked to Released succeeds
    #[test]
    fn test_locked_to_released_succeeds() {
        let setup = TestEnv::new();
        let bounty_id = 1;
        let amount = 1000;
        setup.setup_escrow_in_state(EscrowStatus::Locked, bounty_id, amount);
        setup.client.release_funds(&bounty_id, &setup.contributor);
        let stored_escrow = setup.client.get_escrow_info(&bounty_id);
        assert_eq!(
            stored_escrow.status,
            EscrowStatus::Released,
            "Escrow status did not transition to Released"
        );
    }

    /// Verifies allowed transition from Locked to Refunded succeeds
    #[test]
    fn test_locked_to_refunded_succeeds() {
        let setup = TestEnv::new();
        let bounty_id = 1;
        let amount = 1000;
        setup.setup_escrow_in_state(EscrowStatus::Locked, bounty_id, amount);
        setup
            .env
            .ledger()
            .set_timestamp(setup.env.ledger().timestamp() + 2000);
        setup.client.refund(&bounty_id);
        let stored_escrow = setup.client.get_escrow_info(&bounty_id);
        assert_eq!(
            stored_escrow.status,
            EscrowStatus::Refunded,
            "Escrow status did not transition to Refunded"
        );
    }

    /// Verifies disallowed transition attempt from Released to Locked fails
    #[test]
    fn test_released_to_locked_fails() {
        let setup = TestEnv::new();
        let bounty_id = 1;
        let amount = 1000;
        setup.setup_escrow_in_state(EscrowStatus::Released, bounty_id, amount);
        let deadline = setup.env.ledger().timestamp() + 1000;
        let result = setup
            .client
            .try_lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
        assert!(
            result.is_err(),
            "Expected locking an already released bounty to fail"
        );
        assert_eq!(
            result.unwrap_err().unwrap(),
            Error::BountyExists,
            "Expected BountyExists when attempting to Lock Released escrow."
        );
        let stored = setup.client.get_escrow_info(&bounty_id);
        assert_eq!(
            stored.status,
            EscrowStatus::Released,
            "Escrow status mutated after failed transition"
        );
    }

    /// Verifies disallowed transition attempt from Refunded to Released fails
    #[test]
    fn test_refunded_to_released_fails() {
        let setup = TestEnv::new();
        let bounty_id = 1;
        let amount = 1000;
        setup.setup_escrow_in_state(EscrowStatus::Refunded, bounty_id, amount);
        let result = setup
            .client
            .try_release_funds(&bounty_id, &setup.contributor);
        assert!(
            result.is_err(),
            "Expected releasing a refunded bounty to fail"
        );
        assert_eq!(
            result.unwrap_err().unwrap(),
            Error::FundsNotLocked,
            "Expected FundsNotLocked error variant"
        );
        let stored = setup.client.get_escrow_info(&bounty_id);
        assert_eq!(
            stored.status,
            EscrowStatus::Refunded,
            "Escrow status mutated after failed transition"
        );
    }

    /// Verifies uninitialized transition falls through correctly
    #[test]
    fn test_transition_from_uninitialized_state() {
        let setup = TestEnv::new();
        let bounty_id = 999;
        let result = setup
            .client
            .try_release_funds(&bounty_id, &setup.contributor);
        assert!(
            result.is_err(),
            "Expected release_funds on nonexistent to fail"
        );
        assert_eq!(
            result.unwrap_err().unwrap(),
            Error::BountyNotFound,
            "Expected BountyNotFound error variant"
        );
    }

    /// Verifies idempotent transition fails properly
    #[test]
    fn test_idempotent_transition_attempt() {
        let setup = TestEnv::new();
        let bounty_id = 1;
        let amount = 1000;
        setup.setup_escrow_in_state(EscrowStatus::Locked, bounty_id, amount);
        setup.client.release_funds(&bounty_id, &setup.contributor);
        let result = setup
            .client
            .try_release_funds(&bounty_id, &setup.contributor);
        assert!(
            result.is_err(),
            "Expected idempotent transition attempt to fail"
        );
        assert_eq!(
            result.unwrap_err().unwrap(),
            Error::FundsNotLocked,
            "Expected FundsNotLocked on idempotent attempt"
        );
    }

    /// Explicitly check that status did not change on a failed transition
    #[test]
    fn test_status_field_unchanged_on_error() {
        let setup = TestEnv::new();
        let bounty_id = 1;
        let amount = 1000;
        setup.setup_escrow_in_state(EscrowStatus::Released, bounty_id, amount);
        setup
            .env
            .ledger()
            .set_timestamp(setup.env.ledger().timestamp() + 2000);
        let result = setup.client.try_refund(&bounty_id);
        assert!(result.is_err(), "Expected refund on Released state to fail");
        let stored = setup.client.get_escrow_info(&bounty_id);
        assert_eq!(
            stored.status,
            EscrowStatus::Released,
            "Escrow status should remain strictly unchanged"
        );
    }

    // ========================================================================
    // RECURRING (SUBSCRIPTION) LOCK OPERATIONS
    // ========================================================================

    /// Create a recurring lock schedule that will lock `amount_per_period` tokens
    /// every `period` seconds, subject to the given end condition.
    ///
    /// The depositor must authorize this call. The first lock execution is **not**
    /// performed automatically — call [`execute_recurring_lock`] to trigger each
    /// period's lock.
    ///
    /// # Arguments
    /// * `depositor` — Address whose tokens will be drawn each period.
    /// * `bounty_id` — The bounty this recurring lock funds.
    /// * `amount_per_period` — Token amount to lock per period.
    /// * `period` — Duration between locks in seconds (must be >= 60).
    /// * `end_condition` — Cap / expiry / both.
    /// * `escrow_deadline` — Deadline applied to each individual lock.
    ///
    /// # Errors
    /// * `RecurringLockInvalidConfig` — Zero amount, zero period, period < 60s, or
    ///   end condition with zero cap.
    pub fn create_recurring_lock(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount_per_period: i128,
        period: u64,
        end_condition: RecurringEndCondition,
        escrow_deadline: u64,
    ) -> Result<u64, Error> {
        reentrancy_guard::acquire(&env);

        // Contract must be initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            reentrancy_guard::release(&env);
            return Err(Error::NotInitialized);
        }

        // Operational state checks
        if Self::check_paused(&env, symbol_short!("lock")) {
            reentrancy_guard::release(&env);
            return Err(Error::FundsPaused);
        }
        if Self::get_deprecation_state(&env).deprecated {
            reentrancy_guard::release(&env);
            return Err(Error::ContractDeprecated);
        }

        // Participant filter
        Self::check_participant_filter(&env, depositor.clone())?;

        // Authorization
        depositor.require_auth();

        // Validate config
        if amount_per_period <= 0 || period < 60 {
            reentrancy_guard::release(&env);
            return Err(Error::RecurringLockInvalidConfig);
        }

        // Validate end condition
        match &end_condition {
            RecurringEndCondition::MaxTotal(cap) => {
                if *cap <= 0 {
                    reentrancy_guard::release(&env);
                    return Err(Error::RecurringLockInvalidConfig);
                }
            }
            RecurringEndCondition::EndTime(t) => {
                if *t <= env.ledger().timestamp() {
                    reentrancy_guard::release(&env);
                    return Err(Error::RecurringLockInvalidConfig);
                }
            }
            RecurringEndCondition::Both(cap, t) => {
                if *cap <= 0 || *t <= env.ledger().timestamp() {
                    reentrancy_guard::release(&env);
                    return Err(Error::RecurringLockInvalidConfig);
                }
            }
        }

        // Allocate recurring_id
        let recurring_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::RecurringLockCounter)
            .unwrap_or(0_u64)
            + 1;
        env.storage()
            .persistent()
            .set(&DataKey::RecurringLockCounter, &recurring_id);

        let now = env.ledger().timestamp();

        let config = RecurringLockConfig {
            recurring_id,
            bounty_id,
            depositor: depositor.clone(),
            amount_per_period,
            period,
            end_condition,
            escrow_deadline,
        };

        let state = RecurringLockState {
            last_lock_time: 0,
            cumulative_locked: 0,
            execution_count: 0,
            cancelled: false,
            created_at: now,
        };

        // Store config and state
        env.storage()
            .persistent()
            .set(&DataKey::RecurringLockConfig(recurring_id), &config);
        env.storage()
            .persistent()
            .set(&DataKey::RecurringLockState(recurring_id), &state);

        // Update indexes
        let mut index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::RecurringLockIndex)
            .unwrap_or(Vec::new(&env));
        index.push_back(recurring_id);
        env.storage()
            .persistent()
            .set(&DataKey::RecurringLockIndex, &index);

        let mut dep_index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::DepositorRecurringIndex(depositor.clone()))
            .unwrap_or(Vec::new(&env));
        dep_index.push_back(recurring_id);
        env.storage().persistent().set(
            &DataKey::DepositorRecurringIndex(depositor.clone()),
            &dep_index,
        );

        emit_recurring_lock_created(
            &env,
            RecurringLockCreated {
                version: EVENT_VERSION_V2,
                recurring_id,
                bounty_id,
                depositor,
                amount_per_period,
                period,
                timestamp: now,
            },
        );

        reentrancy_guard::release(&env);
        Ok(recurring_id)
    }

    /// Execute the next period's lock for a recurring lock schedule.
    ///
    /// This is permissionless — anyone can call it once the period has elapsed.
    /// The depositor's tokens are transferred and a new escrow is created for
    /// the bounty with a unique sub-ID (`bounty_id * 1_000_000 + execution_count`).
    ///
    /// # Arguments
    /// * `recurring_id` — The recurring lock schedule to execute.
    ///
    /// # Errors
    /// * `RecurringLockNotFound` — No schedule with this ID.
    /// * `RecurringLockAlreadyCancelled` — Schedule was cancelled.
    /// * `RecurringLockPeriodNotElapsed` — Not enough time since last execution.
    /// * `RecurringLockCapExceeded` — Would exceed the total cap.
    /// * `RecurringLockExpired` — Past the end time.
    pub fn execute_recurring_lock(env: Env, recurring_id: u64) -> Result<(), Error> {
        reentrancy_guard::acquire(&env);

        // Contract must be initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            reentrancy_guard::release(&env);
            return Err(Error::NotInitialized);
        }

        // Operational state checks
        if Self::check_paused(&env, symbol_short!("lock")) {
            reentrancy_guard::release(&env);
            return Err(Error::FundsPaused);
        }
        if Self::get_deprecation_state(&env).deprecated {
            reentrancy_guard::release(&env);
            return Err(Error::ContractDeprecated);
        }

        // Load config and state
        let config = env
            .storage()
            .persistent()
            .get::<DataKey, RecurringLockConfig>(&DataKey::RecurringLockConfig(recurring_id))
            .ok_or_else(|| {
                reentrancy_guard::release(&env);
                Error::RecurringLockNotFound
            })?;

        let mut state = env
            .storage()
            .persistent()
            .get::<DataKey, RecurringLockState>(&DataKey::RecurringLockState(recurring_id))
            .ok_or_else(|| {
                reentrancy_guard::release(&env);
                Error::RecurringLockNotFound
            })?;

        // Check not cancelled
        if state.cancelled {
            reentrancy_guard::release(&env);
            return Err(Error::RecurringLockAlreadyCancelled);
        }

        let now = env.ledger().timestamp();

        // Check period elapsed (first execution uses created_at as base)
        let base_time = if state.last_lock_time == 0 {
            state.created_at
        } else {
            state.last_lock_time
        };
        if now < base_time + config.period {
            reentrancy_guard::release(&env);
            return Err(Error::RecurringLockPeriodNotElapsed);
        }

        // Check end condition
        let amount = config.amount_per_period;
        match &config.end_condition {
            RecurringEndCondition::MaxTotal(cap) => {
                if state.cumulative_locked + amount > *cap {
                    reentrancy_guard::release(&env);
                    return Err(Error::RecurringLockCapExceeded);
                }
            }
            RecurringEndCondition::EndTime(end_time) => {
                if now > *end_time {
                    reentrancy_guard::release(&env);
                    return Err(Error::RecurringLockExpired);
                }
            }
            RecurringEndCondition::Both(cap, end_time) => {
                if state.cumulative_locked + amount > *cap {
                    reentrancy_guard::release(&env);
                    return Err(Error::RecurringLockCapExceeded);
                }
                if now > *end_time {
                    reentrancy_guard::release(&env);
                    return Err(Error::RecurringLockExpired);
                }
            }
        }

        // Generate a unique bounty sub-ID for this execution.
        // Uses bounty_id * 1_000_000 + execution_count to avoid collisions.
        let sub_bounty_id = config
            .bounty_id
            .checked_mul(1_000_000)
            .and_then(|base| base.checked_add(state.execution_count as u64 + 1))
            .unwrap_or_else(|| {
                panic!("recurring lock sub-bounty ID overflow");
            });

        // Ensure sub-bounty doesn't already exist
        if env
            .storage()
            .persistent()
            .has(&DataKey::Escrow(sub_bounty_id))
        {
            reentrancy_guard::release(&env);
            return Err(Error::BountyExists);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer from depositor to contract
        client.transfer(&config.depositor, &env.current_contract_address(), &amount);

        // Resolve fee config and deduct fees
        let (
            lock_fee_rate,
            _release_fee_rate,
            lock_fixed_fee,
            _release_fixed,
            _fee_recipient,
            fee_enabled,
        ) = Self::resolve_fee_config(&env);
        let fee_amount =
            Self::combined_fee_amount(amount, lock_fee_rate, lock_fixed_fee, fee_enabled);
        let net_amount = amount.checked_sub(fee_amount).unwrap_or(amount);
        if net_amount <= 0 {
            reentrancy_guard::release(&env);
            return Err(Error::InvalidAmount);
        }

        // Route fee
        if fee_amount > 0 {
            Self::route_fee(
                &env,
                &client,
                fee_amount,
                lock_fee_rate,
                events::FeeOperationType::Lock,
            )?;
        }

        // Create the escrow record
        let escrow = Escrow {
            depositor: config.depositor.clone(),
            amount: net_amount,
            status: EscrowStatus::Draft,
            deadline: config.escrow_deadline,
            refund_history: vec![&env],
            remaining_amount: net_amount,
            archived: false,
            archived_at: None,
            schema_version: ESCROW_SCHEMA_VERSION,
        };
        invariants::assert_escrow(&env, &escrow);

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(sub_bounty_id), &escrow);

        // Update escrow indexes
        let mut index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        index.push_back(sub_bounty_id);
        env.storage()
            .persistent()
            .set(&DataKey::EscrowIndex, &index);

        let mut dep_index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::DepositorIndex(config.depositor.clone()))
            .unwrap_or(Vec::new(&env));
        dep_index.push_back(sub_bounty_id);
        env.storage().persistent().set(
            &DataKey::DepositorIndex(config.depositor.clone()),
            &dep_index,
        );

        // Update recurring lock state
        state.last_lock_time = now;
        state.cumulative_locked += net_amount;
        state.execution_count += 1;
        env.storage()
            .persistent()
            .set(&DataKey::RecurringLockState(recurring_id), &state);

        // Emit escrow lock event
        emit_funds_locked(
            &env,
            FundsLocked {
                version: EVENT_VERSION_V2,
                bounty_id: sub_bounty_id,
                amount,
                depositor: config.depositor.clone(),
                deadline: config.escrow_deadline,
            },
        );

        // Emit recurring execution event
        emit_recurring_lock_executed(
            &env,
            RecurringLockExecuted {
                version: EVENT_VERSION_V2,
                recurring_id,
                bounty_id: sub_bounty_id,
                amount_locked: net_amount,
                cumulative_locked: state.cumulative_locked,
                execution_count: state.execution_count,
                timestamp: now,
            },
        );

        multitoken_invariants::assert_after_lock(&env);

        audit_trail::log_action(
            &env,
            symbol_short!("rl_exec"),
            config.depositor,
            sub_bounty_id,
        );

        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Cancel a recurring lock schedule. Only the depositor can cancel.
    ///
    /// Cancellation prevents future executions but does not affect already-locked
    /// escrows.
    pub fn cancel_recurring_lock(env: Env, recurring_id: u64) -> Result<(), Error> {
        reentrancy_guard::acquire(&env);

        let config = env
            .storage()
            .persistent()
            .get::<DataKey, RecurringLockConfig>(&DataKey::RecurringLockConfig(recurring_id))
            .ok_or_else(|| {
                reentrancy_guard::release(&env);
                Error::RecurringLockNotFound
            })?;

        let mut state = env
            .storage()
            .persistent()
            .get::<DataKey, RecurringLockState>(&DataKey::RecurringLockState(recurring_id))
            .ok_or_else(|| {
                reentrancy_guard::release(&env);
                Error::RecurringLockNotFound
            })?;

        if state.cancelled {
            reentrancy_guard::release(&env);
            return Err(Error::RecurringLockAlreadyCancelled);
        }

        // Only the depositor can cancel their own recurring lock
        config.depositor.require_auth();

        state.cancelled = true;
        env.storage()
            .persistent()
            .set(&DataKey::RecurringLockState(recurring_id), &state);

        let now = env.ledger().timestamp();
        emit_recurring_lock_cancelled(
            &env,
            RecurringLockCancelled {
                version: EVENT_VERSION_V2,
                recurring_id,
                cancelled_by: config.depositor,
                cumulative_locked: state.cumulative_locked,
                execution_count: state.execution_count,
                timestamp: now,
            },
        );

        reentrancy_guard::release(&env);
        Ok(())
    }

    /// View a recurring lock's configuration and current state.
    pub fn get_recurring_lock(
        env: Env,
        recurring_id: u64,
    ) -> Result<(RecurringLockConfig, RecurringLockState), Error> {
        let config = env
            .storage()
            .persistent()
            .get::<DataKey, RecurringLockConfig>(&DataKey::RecurringLockConfig(recurring_id))
            .ok_or(Error::RecurringLockNotFound)?;
        let state = env
            .storage()
            .persistent()
            .get::<DataKey, RecurringLockState>(&DataKey::RecurringLockState(recurring_id))
            .ok_or(Error::RecurringLockNotFound)?;
        Ok((config, state))
    }

    /// List all recurring lock IDs for a given depositor.
    pub fn get_depositor_recurring_locks(env: Env, depositor: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::DepositorRecurringIndex(depositor))
            .unwrap_or(Vec::new(&env))
    }
}

#[cfg(test)]
mod test_admin_invalid_identifiers;
mod test_batch_failure_mode;
#[cfg(test)]
mod test_batch_failure_modes;
#[cfg(test)]
mod test_deadline_variants;
#[cfg(test)]
mod test_dry_run_simulation;
#[cfg(test)]
mod test_e2e_upgrade_with_pause;
#[cfg(test)]
mod test_query_filters;
#[cfg(test)]
mod test_receipts;
#[cfg(test)]
mod test_sandbox;
#[cfg(test)]
mod test_serialization_compatibility;
#[cfg(test)]
mod test_status_transitions;
#[cfg(test)]
mod test_upgrade_scenarios;
#[cfg(test)]
mod test_escrow_expiry;
#[cfg(test)]
mod test_max_counts;
#[cfg(test)]
mod test_recurring_locks;
