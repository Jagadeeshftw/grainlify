#![no_std]
#[allow(dead_code)]
mod events;
mod invariants;
#[cfg(test)]
mod test_metadata;
#[cfg(test)]
mod test_token_math;
pub mod token_math;

mod reentrancy_guard;
mod test_cross_contract_interface;
#[cfg(test)]
mod test_rbac;
#[cfg(test)]
mod test_multi_token_fees;
mod traits;

use events::{
    emit_batch_funds_locked, emit_batch_funds_released, emit_bounty_initialized, emit_funds_locked,
    emit_funds_refunded, emit_funds_released, BatchFundsLocked, BatchFundsReleased,
    BountyEscrowInitialized, ClaimCancelled, ClaimCreated, ClaimExecuted, FundsLocked,
    FundsRefunded, FundsReleased, EVENT_VERSION_V2,
};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    Symbol, Vec,
};

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

const MAX_FEE_RATE: i128 = token_math::MAX_FEE_RATE;
const MAX_BATCH_SIZE: u32 = 20;

extern crate grainlify_core;
use grainlify_core::asset;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    BountyExists = 3,
    BountyNotFound = 4,
    FundsNotLocked = 5,
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
    CapabilityNotFound = 23,
    CapabilityExpired = 24,
    CapabilityRevoked = 25,
    CapabilityActionMismatch = 26,
    CapabilityAmountExceeded = 27,
    CapabilityUsesExhausted = 28,
    CapabilityExceedsAuthority = 29,
    InvalidAssetId = 30,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowMetadata {
    pub repo_id: u64,
    pub issue_id: u64,
    pub bounty_type: soroban_sdk::String,
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
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Escrow(u64), // bounty_id
    Metadata(u64),
    EscrowIndex,             // Vec<u64> of all bounty_ids
    DepositorIndex(Address), // Vec<u64> of bounty_ids by depositor
    FeeConfig,               // Fee configuration
    RefundApproval(u64),     // bounty_id -> RefundApproval
    ReentrancyGuard,
    MultisigConfig,
    ReleaseApproval(u64), // bounty_id -> ReleaseApproval
    PendingClaim(u64),    // bounty_id -> ClaimRecord
    ClaimWindow,          // u64 seconds (global config)
    PauseFlags,           // PauseFlags struct
    AmountPolicy, // Option<(i128, i128)> — (min_amount, max_amount) set by set_amount_policy
    CapabilityNonce, // monotonically increasing capability id
    Capability(u64), // capability_id -> Capability
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

/// Public view of anti-abuse config (rate limit and cooldown).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AntiAbuseConfigView {
    pub window_size: u64,
    pub max_operations: u32,
    pub cooldown_period: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,
    pub release_fee_rate: i128,
    pub fee_recipient: Address,
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRecord {
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub expires_at: u64,
    pub claimed: bool,
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

/// Result returned by dry-run simulation entrypoints.
///
/// These view functions run the full validation pipeline for lock / release /
/// refund without mutating any state.  They allow UIs and integrators to
/// preview what *would* happen and surface errors before submitting a real
/// transaction.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationResult {
    /// `true` when the operation would succeed at the current ledger state.
    pub success: bool,
    /// When `success` is `false`, the error code that would be returned.
    /// Zero when the operation would succeed.
    pub error_code: u32,
    /// The amount that would be transferred (lock amount, release amount, or
    /// refund amount).
    pub amount: i128,
    /// Escrow status *after* the simulated operation.
    pub resulting_status: EscrowStatus,
    /// Remaining amount in the escrow *after* the simulated operation.
    pub remaining_amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundRecord {
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
    pub mode: RefundMode,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockFundsItem {
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseFundsItem {
    pub bounty_id: u64,
    pub contributor: Address,
}

#[contract]
pub struct BountyEscrowContract;

#[contractimpl]
impl BountyEscrowContract {
    /// Initialize the contract with the admin address and the token address (XLM).
    pub fn init(env: Env, admin: Address, token: asset::AssetId) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        let normalized_token =
            asset::normalize_asset_id(&env, &token).map_err(|_| Error::InvalidAssetId)?;
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::Token, &normalized_token);

        emit_bounty_initialized(
            &env,
            BountyEscrowInitialized {
                version: EVENT_VERSION_V2,
                admin,
                token: normalized_token,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Calculate fee using floor rounding. Delegates to `token_math::calculate_fee`.
    #[allow(dead_code)]
    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        token_math::calculate_fee(amount, fee_rate)
    }

    /// Get fee configuration (internal helper)
    fn get_fee_config_internal(env: &Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&DataKey::FeeConfig)
            .unwrap_or_else(|| FeeConfig {
                lock_fee_rate: 0,
                release_fee_rate: 0,
                fee_recipient: env.storage().instance().get(&DataKey::Admin).unwrap(),
                fee_enabled: false,
            })
    }

    /// Update fee configuration (admin only)
    pub fn update_fee_config(
        env: Env,
        lock_fee_rate: Option<i128>,
        release_fee_rate: Option<i128>,
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
                lock_fee_rate: fee_config.lock_fee_rate,
                release_fee_rate: fee_config.release_fee_rate,
                fee_recipient: fee_config.fee_recipient.clone(),
                fee_enabled: fee_config.fee_enabled,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Update pause flags (admin only)
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

    /// Emergency withdraw all funds (admin only, must have lock_paused = true)
    ///
    /// # Reentrancy
    /// Protected by the shared reentrancy guard. The token transfer is the
    /// last operation (checks-effects-interactions).
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
            // INTERACTION: external token transfer is last
            token_client.transfer(&contract_address, &target, &balance);
            events::emit_emergency_withdraw(
                &env,
                events::EmergencyWithdrawEvent {
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

    fn next_capability_id(env: &Env) -> u64 {
        let last_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CapabilityNonce)
            .unwrap_or(0);
        let next_id = last_id.saturating_add(1);
        env.storage()
            .instance()
            .set(&DataKey::CapabilityNonce, &next_id);
        next_id
    }

    fn load_capability(env: &Env, capability_id: u64) -> Result<Capability, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Capability(capability_id))
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

    fn consume_capability(
        env: &Env,
        holder: &Address,
        capability_id: u64,
        expected_action: CapabilityAction,
        bounty_id: u64,
        amount: i128,
    ) -> Result<Capability, Error> {
        let mut capability = Self::load_capability(env, capability_id)?;

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
            .set(&DataKey::Capability(capability_id), &capability);

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

    #[allow(clippy::too_many_arguments)]
    pub fn issue_capability(
        env: Env,
        owner: Address,
        holder: Address,
        action: CapabilityAction,
        bounty_id: u64,
        amount_limit: i128,
        expiry: u64,
        max_uses: u32,
    ) -> Result<u64, Error> {
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
            .set(&DataKey::Capability(capability_id), &capability);

        events::emit_capability_issued(
            &env,
            events::CapabilityIssued {
                capability_id,
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

        Ok(capability_id)
    }

    pub fn revoke_capability(env: Env, owner: Address, capability_id: u64) -> Result<(), Error> {
        let mut capability = Self::load_capability(&env, capability_id)?;
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
            .set(&DataKey::Capability(capability_id), &capability);

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

    pub fn get_capability(env: Env, capability_id: u64) -> Result<Capability, Error> {
        Self::load_capability(&env, capability_id)
    }

    /// Get current fee configuration (view function)
    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
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
                bounty_id,
                contributor: contributor.clone(),
                approver,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Lock funds for a specific bounty.
    ///
    /// # Reentrancy
    /// Protected by the shared reentrancy guard. State (escrow record,
    /// indexes) is written before the inbound token transfer so that
    /// a malicious token callback cannot re-enter with stale state.
    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, depositor.clone());

        if Self::check_paused(&env, symbol_short!("lock")) {
            return Err(Error::FundsPaused);
        }

        let _start = env.ledger().timestamp();
        let _caller = depositor.clone();

        // Verify depositor authorization
        depositor.require_auth();

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyExists);
        }

        // Enforce min/max amount policy if one has been configured (Issue #62).
        // When no policy is set this block is skipped entirely, preserving
        // backward-compatible behaviour for callers that never call set_amount_policy.
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

        // EFFECTS: write escrow state and indexes before the external call
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: vec![&env],
            remaining_amount: amount,
        };
        invariants::assert_escrow(&env, &escrow);

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

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(&depositor, &env.current_contract_address(), &amount);

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

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Release funds to the contributor.
    /// Only the admin (backend) can authorize this.
    ///
    /// # Reentrancy
    /// Protected by the shared reentrancy guard. Escrow state is updated
    /// to `Released` *before* the outbound token transfer (CEI pattern).
    pub fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
        if Self::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }

        // Block direct release while an active dispute (pending claim) exists.
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

        let _start = env.ledger().timestamp();

        // GUARD: acquire reentrancy lock (replaces inline guard)
        reentrancy_guard::acquire(&env);

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        // EFFECTS: update state before external call (CEI)
        let release_amount = escrow.amount;
        escrow.status = EscrowStatus::Released;
        escrow.remaining_amount = 0;
        invariants::assert_escrow(&env, &escrow);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // INTERACTION: external token transfer is last
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(
            &env.current_contract_address(),
            &contributor,
            &release_amount,
        );

        emit_funds_released(
            &env,
            FundsReleased {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: release_amount,
                recipient: contributor.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Delegated release flow using a capability instead of admin auth.
    /// The capability amount limit is consumed by `payout_amount`.
    pub fn release_with_capability(
        env: Env,
        bounty_id: u64,
        contributor: Address,
        payout_amount: i128,
        holder: Address,
        capability_id: u64,
    ) -> Result<(), Error> {
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

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(
            &env.current_contract_address(),
            &contributor,
            &payout_amount,
        );

        escrow.remaining_amount -= payout_amount;
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released;
        }
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

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

    /// Authorize a release as a pending claim instead of immediate transfer.
    /// Admin calls this instead of release_funds when claim period is active.
    /// Beneficiary must call claim() within the window to receive funds.
    pub fn authorize_claim(env: Env, bounty_id: u64, recipient: Address) -> Result<(), Error> {
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

    /// Beneficiary calls this to claim their authorized funds within the window.
    ///
    /// # Reentrancy
    /// Protected by the shared reentrancy guard. Escrow and claim state
    /// are updated *before* the outbound token transfer (CEI pattern).
    pub fn claim(env: Env, bounty_id: u64) -> Result<(), Error> {
        if Self::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }

        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

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

        // EFFECTS: update escrow and claim state before external call (CEI)
        let claim_amount = claim.amount;
        let claim_recipient = claim.recipient.clone();

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
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
            &claim_recipient,
            &claim_amount,
        );

        env.events().publish(
            (symbol_short!("claim"), symbol_short!("done")),
            ClaimExecuted {
                bounty_id,
                recipient: claim_recipient,
                amount: claim_amount,
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
        capability_id: u64,
    ) -> Result<(), Error> {
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

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        client.transfer(
            &env.current_contract_address(),
            &claim.recipient,
            &claim.amount,
        );

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        escrow.status = EscrowStatus::Released;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        claim.claimed = true;
        env.storage()
            .persistent()
            .set(&DataKey::PendingClaim(bounty_id), &claim);

        env.events().publish(
            (symbol_short!("claim"), symbol_short!("done")),
            ClaimExecuted {
                bounty_id,
                recipient: claim.recipient,
                amount: claim.amount,
                claimed_at: now,
            },
        );
        Ok(())
    }

    /// Admin can cancel an expired or unwanted pending claim, returning escrow to Locked.
    pub fn cancel_pending_claim(env: Env, bounty_id: u64) -> Result<(), Error> {
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

        if claim.claimed {
            return Err(Error::FundsNotLocked);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::PendingClaim(bounty_id));

        env.events().publish(
            (symbol_short!("claim"), symbol_short!("cancel")),
            ClaimCancelled {
                bounty_id,
                recipient: claim.recipient,
                amount: claim.amount,
                cancelled_at: env.ledger().timestamp(),
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

        Ok(())
    }

    /// Release a partial amount of the locked funds to the contributor.
    /// Only the admin (backend) can authorize this.
    ///
    /// - `payout_amount` must be > 0 and <= `remaining_amount`.
    /// - `remaining_amount` is decremented by `payout_amount` after each call.
    /// - When `remaining_amount` reaches 0 the escrow status is set to Released.
    /// - The bounty stays Locked while any funds remain unreleased.
    ///
    /// # Reentrancy
    /// Protected by the shared reentrancy guard. Escrow state is updated
    /// *before* the outbound token transfer (CEI pattern).
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

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

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

        // EFFECTS: update escrow state before external call (CEI)
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

        events::emit_funds_released(
            &env,
            FundsReleased {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount: payout_amount,
                recipient: contributor.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(())
    }

    /// Refund funds to the original depositor if the deadline has passed.
    /// Refunds the full remaining_amount (accounts for any prior partial releases).
    ///
    /// # Reentrancy
    /// Protected by the shared reentrancy guard. Escrow state, refund
    /// history, and approval cleanup are performed *before* the outbound
    /// token transfer (CEI pattern).
    pub fn refund(env: Env, bounty_id: u64) -> Result<(), Error> {
        if Self::check_paused(&env, symbol_short!("refund")) {
            return Err(Error::FundsPaused);
        }

        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

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
        escrow.remaining_amount -= refund_amount;
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
        capability_id: u64,
    ) -> Result<(), Error> {
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

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        let now = env.ledger().timestamp();
        let refund_to = escrow.depositor.clone();

        client.transfer(&env.current_contract_address(), &refund_to, &amount);

        escrow.remaining_amount -= amount;
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Refunded;
        } else {
            escrow.status = EscrowStatus::PartiallyRefunded;
        }

        escrow.refund_history.push_back(RefundRecord {
            amount,
            recipient: refund_to.clone(),
            timestamp: now,
            mode: if escrow.status == EscrowStatus::Refunded {
                RefundMode::Full
            } else {
                RefundMode::Partial
            },
        });

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        emit_funds_refunded(
            &env,
            FundsRefunded {
                version: EVENT_VERSION_V2,
                bounty_id,
                amount,
                refund_to,
                timestamp: now,
            },
        );

        Ok(())
    }

    /// view function to get escrow info
    pub fn get_escrow_info(env: Env, bounty_id: u64) -> Result<Escrow, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap())
    }

    /// view function to get contract balance of the token
    pub fn get_balance(env: Env) -> Result<i128, Error> {
        if !env.storage().instance().has(&DataKey::Token) {
            return Err(Error::NotInitialized);
        }
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        Ok(client.balance(&env.current_contract_address()))
    }

    // =========================================================================
    // Dry-Run Simulation Entry Points  (Issue #567)
    //
    // These are **view-only** functions that replay the validation logic of
    // lock / release / refund WITHOUT writing any state or performing token
    // transfers.  They return a `SimulationResult` so callers can preview
    // the outcome and surface user-facing errors before submitting a real tx.
    // =========================================================================

    /// Simulate a `lock_funds` call.
    ///
    /// Checks initialisation, duplicate bounty, pause state, and amount policy
    /// exactly as the real function does, then returns what the resulting
    /// escrow state would look like.  No auth is required.
    pub fn simulate_lock(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> SimulationResult {
        // --- Checks (mirrors lock_funds validation) ---

        if Self::check_paused(&env, symbol_short!("lock")) {
            return SimulationResult {
                success: false,
                error_code: Error::FundsPaused as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return SimulationResult {
                success: false,
                error_code: Error::NotInitialized as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return SimulationResult {
                success: false,
                error_code: Error::BountyExists as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        if amount <= 0 {
            return SimulationResult {
                success: false,
                error_code: Error::InvalidAmount as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        if deadline <= env.ledger().timestamp() {
            return SimulationResult {
                success: false,
                error_code: Error::InvalidDeadline as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        // Enforce amount policy if set
        if let Some((min_amount, max_amount)) = env
            .storage()
            .instance()
            .get::<DataKey, (i128, i128)>(&DataKey::AmountPolicy)
        {
            if amount < min_amount {
                return SimulationResult {
                    success: false,
                    error_code: Error::AmountBelowMinimum as u32,
                    amount: 0,
                    resulting_status: EscrowStatus::Locked,
                    remaining_amount: 0,
                };
            }
            if amount > max_amount {
                return SimulationResult {
                    success: false,
                    error_code: Error::AmountAboveMaximum as u32,
                    amount: 0,
                    resulting_status: EscrowStatus::Locked,
                    remaining_amount: 0,
                };
            }
        }

        // Check depositor has sufficient balance
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        let depositor_balance = token_client.balance(&depositor);
        if depositor_balance < amount {
            return SimulationResult {
                success: false,
                error_code: Error::InsufficientFunds as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        // --- Would succeed ---
        SimulationResult {
            success: true,
            error_code: 0,
            amount,
            resulting_status: EscrowStatus::Locked,
            remaining_amount: amount,
        }
    }

    /// Simulate a `release_funds` call.
    ///
    /// Checks initialisation, existence, pause state, and escrow status
    /// exactly as the real function does.  Returns the projected released
    /// state.  No auth is required.
    pub fn simulate_release(env: Env, bounty_id: u64, _contributor: Address) -> SimulationResult {
        if Self::check_paused(&env, symbol_short!("release")) {
            return SimulationResult {
                success: false,
                error_code: Error::FundsPaused as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return SimulationResult {
                success: false,
                error_code: Error::NotInitialized as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return SimulationResult {
                success: false,
                error_code: Error::BountyNotFound as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked {
            return SimulationResult {
                success: false,
                error_code: Error::FundsNotLocked as u32,
                amount: 0,
                resulting_status: escrow.status,
                remaining_amount: escrow.remaining_amount,
            };
        }

        // --- Would succeed ---
        SimulationResult {
            success: true,
            error_code: 0,
            amount: escrow.amount,
            resulting_status: EscrowStatus::Released,
            remaining_amount: 0,
        }
    }

    /// Simulate a `refund` call.
    ///
    /// Checks pause state, existence, escrow status, pending claims,
    /// deadline, and refund approval exactly as the real function does.
    /// Returns the projected refund amount and resulting status.
    /// No auth is required.
    pub fn simulate_refund(env: Env, bounty_id: u64) -> SimulationResult {
        if Self::check_paused(&env, symbol_short!("refund")) {
            return SimulationResult {
                success: false,
                error_code: Error::FundsPaused as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return SimulationResult {
                success: false,
                error_code: Error::BountyNotFound as u32,
                amount: 0,
                resulting_status: EscrowStatus::Locked,
                remaining_amount: 0,
            };
        }

        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
        {
            return SimulationResult {
                success: false,
                error_code: Error::FundsNotLocked as u32,
                amount: 0,
                resulting_status: escrow.status,
                remaining_amount: escrow.remaining_amount,
            };
        }

        // Block if there is an active pending claim
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
                return SimulationResult {
                    success: false,
                    error_code: Error::ClaimPending as u32,
                    amount: 0,
                    resulting_status: escrow.status,
                    remaining_amount: escrow.remaining_amount,
                };
            }
        }

        let now = env.ledger().timestamp();
        let approval_key = DataKey::RefundApproval(bounty_id);
        let approval: Option<RefundApproval> = env.storage().persistent().get(&approval_key);

        if now < escrow.deadline && approval.is_none() {
            return SimulationResult {
                success: false,
                error_code: Error::DeadlineNotPassed as u32,
                amount: 0,
                resulting_status: escrow.status,
                remaining_amount: escrow.remaining_amount,
            };
        }

        // Calculate refund parameters (same logic as real refund)
        let (refund_amount, is_full) = if let Some(app) = approval {
            let full = app.mode == RefundMode::Full || app.amount >= escrow.remaining_amount;
            (app.amount, full)
        } else {
            (escrow.remaining_amount, true)
        };

        if refund_amount <= 0 || refund_amount > escrow.remaining_amount {
            return SimulationResult {
                success: false,
                error_code: Error::InvalidAmount as u32,
                amount: 0,
                resulting_status: escrow.status,
                remaining_amount: escrow.remaining_amount,
            };
        }

        // --- Would succeed ---
        let new_remaining = escrow.remaining_amount - refund_amount;
        let new_status = if is_full || new_remaining == 0 {
            EscrowStatus::Refunded
        } else {
            EscrowStatus::PartiallyRefunded
        };

        SimulationResult {
            success: true,
            error_code: 0,
            amount: refund_amount,
            resulting_status: new_status,
            remaining_amount: new_remaining,
        }
    }

    /// Query escrows with filtering and pagination
    /// Pass 0 for min values and i128::MAX/u64::MAX for max values to disable those filters
    pub fn query_escrows_by_status(
        env: Env,
        status: EscrowStatus,
        offset: u32,
        limit: u32,
    ) -> Vec<EscrowWithId> {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..index.len() {
            if count >= limit {
                break;
            }

            let bounty_id = index.get(i).unwrap();
            if let Some(escrow) = env
                .storage()
                .persistent()
                .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
            {
                if escrow.status == status {
                    if skipped < offset {
                        skipped += 1;
                        continue;
                    }
                    results.push_back(EscrowWithId { bounty_id, escrow });
                    count += 1;
                }
            }
        }
        results
    }

    /// Query escrows with amount range filtering
    pub fn query_escrows_by_amount(
        env: Env,
        min_amount: i128,
        max_amount: i128,
        offset: u32,
        limit: u32,
    ) -> Vec<EscrowWithId> {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..index.len() {
            if count >= limit {
                break;
            }

            let bounty_id = index.get(i).unwrap();
            if let Some(escrow) = env
                .storage()
                .persistent()
                .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
            {
                if escrow.amount >= min_amount && escrow.amount <= max_amount {
                    if skipped < offset {
                        skipped += 1;
                        continue;
                    }
                    results.push_back(EscrowWithId { bounty_id, escrow });
                    count += 1;
                }
            }
        }
        results
    }

    /// Query escrows with deadline range filtering
    pub fn query_escrows_by_deadline(
        env: Env,
        min_deadline: u64,
        max_deadline: u64,
        offset: u32,
        limit: u32,
    ) -> Vec<EscrowWithId> {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..index.len() {
            if count >= limit {
                break;
            }

            let bounty_id = index.get(i).unwrap();
            if let Some(escrow) = env
                .storage()
                .persistent()
                .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
            {
                if escrow.deadline >= min_deadline && escrow.deadline <= max_deadline {
                    if skipped < offset {
                        skipped += 1;
                        continue;
                    }
                    results.push_back(EscrowWithId { bounty_id, escrow });
                    count += 1;
                }
            }
        }
        results
    }

    /// Query escrows by depositor
    pub fn query_escrows_by_depositor(
        env: Env,
        depositor: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<EscrowWithId> {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::DepositorIndex(depositor))
            .unwrap_or(Vec::new(&env));
        let mut results = Vec::new(&env);
        let start = offset.min(index.len());
        let end = (offset + limit).min(index.len());

        for i in start..end {
            let bounty_id = index.get(i).unwrap();
            if let Some(escrow) = env
                .storage()
                .persistent()
                .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
            {
                results.push_back(EscrowWithId { bounty_id, escrow });
            }
        }
        results
    }

    /// Get aggregate statistics
    pub fn get_aggregate_stats(env: Env) -> AggregateStats {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        let mut stats = AggregateStats {
            total_locked: 0,
            total_released: 0,
            total_refunded: 0,
            count_locked: 0,
            count_released: 0,
            count_refunded: 0,
        };

        for i in 0..index.len() {
            let bounty_id = index.get(i).unwrap();
            if let Some(escrow) = env
                .storage()
                .persistent()
                .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
            {
                match escrow.status {
                    EscrowStatus::Locked => {
                        stats.total_locked += escrow.amount;
                        stats.count_locked += 1;
                    }
                    EscrowStatus::Released => {
                        stats.total_released += escrow.amount;
                        stats.count_released += 1;
                    }
                    EscrowStatus::Refunded | EscrowStatus::PartiallyRefunded => {
                        stats.total_refunded += escrow.amount;
                        stats.count_refunded += 1;
                    }
                }
            }
        }
        stats
    }

    /// Get total count of escrows
    pub fn get_escrow_count(env: Env) -> u32 {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        index.len()
    }

    /// Set the minimum and maximum allowed lock amount (admin only).
    ///
    /// Once set, any call to lock_funds with an amount outside [min_amount, max_amount]
    /// will be rejected with AmountBelowMinimum or AmountAboveMaximum respectively.
    /// The policy can be updated at any time by the admin; new limits take effect
    /// immediately for subsequent lock_funds calls.
    ///
    /// Passing min_amount == max_amount restricts locking to a single exact value.
    /// min_amount must not exceed max_amount — the call panics if this invariant
    /// is violated.
    pub fn set_amount_policy(
        env: Env,
        caller: Address,
        min_amount: i128,
        max_amount: i128,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        if min_amount > max_amount {
            panic!("invalid policy: min_amount cannot exceed max_amount");
        }

        // Persist the policy so lock_funds can enforce it on every subsequent call.
        env.storage()
            .instance()
            .set(&DataKey::AmountPolicy, &(min_amount, max_amount));

        Ok(())
    }

    /// Get escrow IDs by status
    pub fn get_escrow_ids_by_status(
        env: Env,
        status: EscrowStatus,
        offset: u32,
        limit: u32,
    ) -> Vec<u64> {
        let index: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowIndex)
            .unwrap_or(Vec::new(&env));
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..index.len() {
            if count >= limit {
                break;
            }
            let bounty_id = index.get(i).unwrap();
            if let Some(escrow) = env
                .storage()
                .persistent()
                .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
            {
                if escrow.status == status {
                    if skipped < offset {
                        skipped += 1;
                        continue;
                    }
                    results.push_back(bounty_id);
                    count += 1;
                }
            }
        }
        results
    }

    pub fn set_anti_abuse_admin(env: Env, admin: Address) -> Result<(), Error> {
        let current: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        current.require_auth();
        anti_abuse::set_admin(&env, admin);
        Ok(())
    }

    pub fn get_anti_abuse_admin(env: Env) -> Option<Address> {
        anti_abuse::get_admin(&env)
    }

    /// Set whitelist status for an address (admin only). Named to avoid SDK client method conflict.
    pub fn set_whitelist_entry(
        env: Env,
        whitelisted_address: Address,
        whitelisted: bool,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        anti_abuse::set_whitelist(&env, whitelisted_address, whitelisted);
        Ok(())
    }

    /// Update anti-abuse config (rate limit window, max operations per window, cooldown). Admin only.
    pub fn update_anti_abuse_config(
        env: Env,
        window_size: u64,
        max_operations: u32,
        cooldown_period: u64,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        let config = anti_abuse::AntiAbuseConfig {
            window_size,
            max_operations,
            cooldown_period,
        };
        anti_abuse::set_config(&env, config);
        Ok(())
    }

    /// Get current anti-abuse config (rate limit and cooldown).
    pub fn get_anti_abuse_config(env: Env) -> AntiAbuseConfigView {
        let c = anti_abuse::get_config(&env);
        AntiAbuseConfigView {
            window_size: c.window_size,
            max_operations: c.max_operations,
            cooldown_period: c.cooldown_period,
        }
    }

    /// Retrieves the refund history for a specific bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok(Vec<RefundRecord>)` - The refund history
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    pub fn get_refund_history(env: Env, bounty_id: u64) -> Result<Vec<RefundRecord>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Ok(escrow.refund_history)
    }

    /// NEW: Verify escrow invariants for a specific bounty
    pub fn verify_state(env: Env, bounty_id: u64) -> bool {
        if let Some(escrow) = env
            .storage()
            .persistent()
            .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
        {
            invariants::verify_escrow_invariants(&escrow)
        } else {
            false
        }
    }
    /// Gets refund eligibility information for a bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok((bool, bool, i128, Option<RefundApproval>))` - Tuple containing:
    ///   - can_refund: Whether refund is possible
    ///   - deadline_passed: Whether the deadline has passed
    ///   - remaining: Remaining amount in escrow
    ///   - approval: Optional refund approval if exists
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    pub fn get_refund_eligibility(
        env: Env,
        bounty_id: u64,
    ) -> Result<(bool, bool, i128, Option<RefundApproval>), Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        let now = env.ledger().timestamp();
        let deadline_passed = now >= escrow.deadline;

        let approval = if env
            .storage()
            .persistent()
            .has(&DataKey::RefundApproval(bounty_id))
        {
            Some(
                env.storage()
                    .persistent()
                    .get(&DataKey::RefundApproval(bounty_id))
                    .unwrap(),
            )
        } else {
            None
        };

        // can_refund is true if:
        // 1. Status is Locked or PartiallyRefunded AND
        // 2. (deadline has passed OR there's an approval)
        let can_refund = (escrow.status == EscrowStatus::Locked
            || escrow.status == EscrowStatus::PartiallyRefunded)
            && (deadline_passed || approval.is_some());

        Ok((
            can_refund,
            deadline_passed,
            escrow.remaining_amount,
            approval,
        ))
    }

    /// Batch lock funds for multiple bounties in a single transaction.
    /// This improves gas efficiency by reducing transaction overhead.
    ///
    /// # Arguments
    /// * `items` - Vector of LockFundsItem containing bounty_id, depositor, amount, and deadline
    ///
    /// # Returns
    /// Number of successfully locked bounties
    ///
    /// # Errors
    /// * InvalidBatchSize - if batch size exceeds MAX_BATCH_SIZE or is zero
    /// * BountyExists - if any bounty_id already exists
    /// * NotInitialized - if contract is not initialized
    ///
    /// # Note
    /// This operation is atomic - if any item fails, the entire transaction reverts.
    /// # Reentrancy
    /// Protected by the shared reentrancy guard. All escrow records are
    /// written first; token transfers happen in a second pass (CEI).
    pub fn batch_lock_funds(env: Env, items: Vec<LockFundsItem>) -> Result<u32, Error> {
        if Self::check_paused(&env, symbol_short!("lock")) {
            return Err(Error::FundsPaused);
        }

        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

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

        // Collect unique depositors and require auth once for each
        // This prevents "frame is already authorized" errors when same depositor appears multiple times
        let mut seen_depositors: Vec<Address> = Vec::new(&env);
        for item in items.iter() {
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

        // EFFECTS: write all escrow records before any external calls (CEI)
        let mut locked_count = 0u32;
        for item in items.iter() {
            let escrow = Escrow {
                depositor: item.depositor.clone(),
                amount: item.amount,
                status: EscrowStatus::Locked,
                deadline: item.deadline,
                refund_history: vec![&env],
                remaining_amount: item.amount,
            };

            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);

            locked_count += 1;
        }

        // INTERACTION: all external token transfers happen after state is finalized
        for item in items.iter() {
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
        }

        // Emit batch event
        emit_batch_funds_locked(
            &env,
            BatchFundsLocked {
                count: locked_count,
                total_amount: items.iter().map(|i| i.amount).sum(),
                timestamp,
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(locked_count)
    }

    /// Batch release funds to multiple contributors in a single transaction.
    /// This improves gas efficiency by reducing transaction overhead.
    ///
    /// # Arguments
    /// * `items` - Vector of ReleaseFundsItem containing bounty_id and contributor address
    ///
    /// # Returns
    /// Number of successfully released bounties
    ///
    /// # Errors
    /// * InvalidBatchSize - if batch size exceeds MAX_BATCH_SIZE or is zero
    /// * BountyNotFound - if any bounty_id doesn't exist
    /// * FundsNotLocked - if any bounty is not in Locked status
    /// * Unauthorized - if caller is not admin
    ///
    /// # Note
    /// This operation is atomic - if any item fails, the entire transaction reverts.
    /// # Reentrancy
    /// Protected by the shared reentrancy guard. All escrow records are
    /// updated to `Released` first; token transfers happen in a second
    /// pass (CEI).
    pub fn batch_release_funds(env: Env, items: Vec<ReleaseFundsItem>) -> Result<u32, Error> {
        if Self::check_paused(&env, symbol_short!("release")) {
            return Err(Error::FundsPaused);
        }

        // GUARD: acquire reentrancy lock
        reentrancy_guard::acquire(&env);

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

            if escrow.status != EscrowStatus::Locked {
                return Err(Error::FundsNotLocked);
            }

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

        // EFFECTS: update all escrow records before any external calls (CEI)
        // We collect (contributor, amount) pairs for the transfer pass.
        let mut release_pairs: Vec<(Address, i128)> = Vec::new(&env);
        let mut released_count = 0u32;
        for item in items.iter() {
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
        for (idx, item) in items.iter().enumerate() {
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
                count: released_count,
                total_amount,
                timestamp,
            },
        );

        // GUARD: release reentrancy lock
        reentrancy_guard::release(&env);
        Ok(released_count)
    }
    pub fn update_metadata(
        env: Env,
        _admin: Address,
        bounty_id: u64,
        repo_id: u64,
        issue_id: u64,
        bounty_type: soroban_sdk::String,
    ) -> Result<(), Error> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        stored_admin.require_auth();

        let metadata = EscrowMetadata {
            repo_id,
            issue_id,
            bounty_type,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Metadata(bounty_id), &metadata);
        Ok(())
    }

    pub fn get_metadata(env: Env, bounty_id: u64) -> Result<EscrowMetadata, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Metadata(bounty_id))
            .ok_or(Error::BountyNotFound)
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
        BountyEscrowContract::lock_funds(env.clone(), depositor, bounty_id, amount, deadline)
    }

    /// Release funds to contributor through the trait interface
    fn release_funds(env: &Env, bounty_id: u64, contributor: Address) -> Result<(), crate::Error> {
        BountyEscrowContract::release_funds(env.clone(), bounty_id, contributor)
    }

    /// Refund funds to depositor through the trait interface
    fn refund(env: &Env, bounty_id: u64) -> Result<(), crate::Error> {
        BountyEscrowContract::refund(env.clone(), bounty_id)
    }

    /// Get escrow information through the trait interface
    fn get_escrow_info(env: &Env, bounty_id: u64) -> Result<crate::Escrow, crate::Error> {
        BountyEscrowContract::get_escrow_info(env.clone(), bounty_id)
    }

    /// Get contract balance through the trait interface
    fn get_balance(env: &Env) -> Result<i128, crate::Error> {
        BountyEscrowContract::get_balance(env.clone())
    }
}

impl traits::UpgradeInterface for BountyEscrowContract {
    /// Get contract version
    fn get_version(_env: &Env) -> u32 {
        1 // Current version
    }

    /// Set contract version (admin only)
    fn set_version(_env: &Env, _new_version: u32) -> Result<(), soroban_sdk::String> {
        // Version management - reserved for future use
        // Currently, version is hardcoded to 1
        Ok(())
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
// Temporarily disabled: this suite targets a different blacklist API surface
// (`initialize`, `set_blacklist`, `set_whitelist_mode`) than this contract exposes.
// Re-enable after API/test alignment.
// mod test_blacklist_and_whitelist;
#[cfg(test)]
mod test_bounty_escrow;
#[cfg(test)]
mod test_capability_tokens;
#[cfg(test)]
mod test_dispute_resolution;
#[cfg(test)]
mod test_dry_run_simulation;
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
mod test_pause;
#[cfg(test)]
mod test_reentrancy_guard;
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
}
#[cfg(test)]
mod test_deadline_variants;
#[cfg(test)]
mod test_query_filters;
#[cfg(test)]
mod test_status_transitions;
