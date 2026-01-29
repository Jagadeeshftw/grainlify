//! # Bounty Escrow Smart Contract
//!
//! A trustless escrow system for bounty payments on the Stellar blockchain.
//! This contract enables secure fund locking, conditional release to contributors,
//! and automatic refunds after deadlines.
//!
//! ## Overview
//!
//! The Bounty Escrow contract manages the complete lifecycle of bounty payments:
//! 1. **Initialization**: Set up admin and token contract
//! 2. **Lock Funds**: Depositor locks tokens for a bounty with a deadline
//! 3. **Release**: Admin releases funds to contributor upon task completion
//! 4. **Refund**: Automatic refund to depositor if deadline passes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Contract Architecture                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────┐                                           │
//! │  │  Depositor   │─────┐                                     │
//! │  └──────────────┘     │                                     │
//! │                       ├──> lock_funds()                     │
//! │  ┌──────────────┐     │         │                           │
//! │  │    Admin     │─────┘         ▼                           │
//! │  └──────────────┘          ┌─────────┐                      │
//! │         │                  │ ESCROW  │                      │
//! │         │                  │ LOCKED  │                      │
//! │         │                  └────┬────┘                      │
//! │         │                       │                           │
//! │         │        ┌──────────────┴───────────────┐           │
//! │         │        │                              │           │
//! │         ▼        ▼                              ▼           │
//! │   release_funds()                          refund()         │
//! │         │                                       │           │
//! │         ▼                                       ▼           │
//! │  ┌──────────────┐                      ┌──────────────┐    │
//! │  │ Contributor  │                      │  Depositor   │    │
//! │  └──────────────┘                      └──────────────┘    │
//! │    (RELEASED)                            (REFUNDED)        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Admin**: Trusted entity (backend service) authorized to release funds
//! - **Depositor**: Self-interested party; funds protected by deadline mechanism
//! - **Contributor**: Receives funds only after admin approval
//! - **Contract**: Trustless; operates according to programmed rules
//!
//! ### Key Security Features
//! 1. **Single Initialization**: Prevents admin takeover
//! 2. **Unique Bounty IDs**: No duplicate escrows
//! 3. **Authorization Checks**: All state changes require proper auth
//! 4. **Deadline Protection**: Prevents indefinite fund locking
//! 5. **State Machine**: Enforces valid state transitions
//! 6. **Atomic Operations**: Transfer + state update in single transaction
//!
//! ## Usage Example
//!
//! ```rust
//! use soroban_sdk::{Address, Env};
//!
//! // 1. Initialize contract (one-time setup)
//! let admin = Address::from_string("GADMIN...");
//! let token = Address::from_string("CUSDC...");
//! escrow_client.init(&admin, &token);
//!
//! // 2. Depositor locks 1000 USDC for bounty #42
//! let depositor = Address::from_string("GDEPOSIT...");
//! let amount = 1000_0000000; // 1000 USDC (7 decimals)
//! let deadline = current_timestamp + (30 * 24 * 60 * 60); // 30 days
//! escrow_client.lock_funds(&depositor, &42, &amount, &deadline);
//!
//! // 3a. Admin releases to contributor (happy path)
//! let contributor = Address::from_string("GCONTRIB...");
//! escrow_client.release_funds(&42, &contributor);
//!
//! // OR
//!
//! // 3b. Refund to depositor after deadline (timeout path)
//! // (Can be called by anyone after deadline passes)
//! escrow_client.refund(&42);
//! ```

#![no_std]
mod events;
mod test_bounty_escrow;
//#[cfg(test)]
//mod test_query;

use events::{
    emit_admin_action_cancelled, emit_admin_action_executed, emit_admin_action_proposed,
    emit_admin_updated, emit_batch_funds_locked, emit_batch_funds_released,
    emit_bounty_initialized, emit_config_limits_updated, emit_contract_paused,
    emit_contract_unpaused, emit_emergency_withdrawal, emit_funds_locked, emit_funds_refunded,
    emit_funds_released, emit_payout_key_updated, AdminActionCancelled, AdminActionExecuted,
    AdminActionProposed, AdminUpdated, BatchFundsLocked, BatchFundsReleased,
    BountyEscrowInitialized, ConfigLimitsUpdated, ContractPaused, ContractUnpaused,
    EmergencyWithdrawal, FundsLocked, FundsRefunded, FundsReleased, PayoutKeyUpdated,
};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    Vec,
};

// ==================== MONITORING MODULE ====================
#[allow(dead_code)]
mod monitoring {
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

    // Storage keys
    const OPERATION_COUNT: &str = "op_count";
    #[allow(dead_code)]
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

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct StateSnapshot {
        pub timestamp: u64,
        pub total_operations: u64,
        pub total_users: u64,
        pub total_errors: u64,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceStats {
        pub function_name: Symbol,
        pub call_count: u64,
        pub total_time: u64,
        pub avg_time: u64,
        pub last_called: u64,
    }

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

    #[allow(dead_code)]
    pub fn _health_check(env: &Env) -> HealthStatus {
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
// ==================== END MONITORING MODULE ====================

// ==================== ANTI-ABUSE MODULE ====================
#[allow(dead_code)]
mod anti_abuse {
    use soroban_sdk::{contracttype, symbol_short, Address, Env};

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct AntiAbuseConfig {
        pub window_size: u64,
        pub max_operations: u32,
        pub cooldown_period: u64,
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
                window_size: 3600,
                max_operations: 10,
                cooldown_period: 60,
            })
    }

    #[allow(dead_code)]
    pub fn _set_config(env: &Env, config: AntiAbuseConfig) {
        env.storage().instance().set(&AntiAbuseKey::Config, &config);
    }

    pub fn is_whitelisted(env: &Env, address: Address) -> bool {
        env.storage()
            .instance()
            .has(&AntiAbuseKey::Whitelist(address))
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn get_admin(env: &Env) -> Option<Address> {
        env.storage().instance().get(&AntiAbuseKey::Admin)
    }

    #[allow(dead_code)]
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

        if now
            >= state
                .window_start_timestamp
                .saturating_add(config.window_size)
        {
            state.window_start_timestamp = now;
            state.operation_count = 1;
        } else {
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
        env.storage().persistent().extend_ttl(&key, 17280, 17280);
    }
}
// ==================== END ANTI-ABUSE MODULE ====================

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
    ContractPaused = 11,
    DuplicateBountyId = 12,
    InvalidAmount = 13,
    InvalidDeadline = 14,
    InsufficientFunds = 16,
    RefundNotApproved = 17,
    BatchSizeMismatch = 18,
    ActionNotFound = 19,
    ActionNotReady = 20,
    InvalidTimeLock = 21,
}

// ============================================================================
// Data Structures
// ============================================================================

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
    PartiallyRefunded,
    PartiallyReleased,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefundMode {
    Full,
    Partial,
    Custom,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundRecord {
    pub amount: i128,
    pub recipient: Address,
    pub mode: RefundMode,
    pub timestamp: u64,
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
pub struct Escrow {
    pub depositor: Address,
    pub amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
    pub refund_history: Vec<RefundRecord>,
    pub payout_history: Vec<PayoutRecord>,
    pub remaining_amount: i128,
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

const MAX_BATCH_SIZE: u32 = 100;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,
    pub release_fee_rate: i128,
    pub fee_recipient: Address,
    pub fee_enabled: bool,
}

const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 1_000;

// ============================================================================
// Admin Configuration Structures
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigLimits {
    pub max_bounty_amount: Option<i128>,
    pub min_bounty_amount: Option<i128>,
    pub max_deadline_duration: Option<u64>,
    pub min_deadline_duration: Option<u64>,
}

// FIXED: Refactored AdminActionType to carry the data, removing problematic Options from AdminAction
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminActionType {
    UpdateAdmin(Address),
    UpdatePayoutKey(Address),
    UpdateConfigLimits(ConfigLimits),
    UpdateFeeConfig(FeeConfig),
}

// FIXED: Removed Option<FeeConfig> and others to resolve trait bound error
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminAction {
    pub action_id: u64,
    pub action_type: AdminActionType,
    pub proposed_by: Address,
    pub execution_time: u64,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractState {
    pub admin: Address,
    pub token: Address,
    pub payout_key: Option<Address>,
    pub fee_config: FeeConfig,
    pub config_limits: ConfigLimits,
    pub is_paused: bool,
    pub time_lock_duration: u64,
    pub total_bounties: u64,
    pub total_locked_amount: i128,
    pub contract_version: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Escrow(u64),
    FeeConfig,
    RefundApproval(u64),
    ReentrancyGuard,
    IsPaused,
    PayoutKey,
    ConfigLimits,
    TimeLockDuration,
    NextActionId,
    AdminAction(u64),
    BountyRegistry, // Vec<u64> of all bounty IDs
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowFilter {
    pub status: Option<u32>, // Using u32 to avoid Option<Enum> XDR issues
    pub depositor: Option<Address>,
    pub min_amount: Option<i128>,
    pub max_amount: Option<i128>,
    pub start_time: Option<u64>, // Filter by deadline (>= start_time)
    pub end_time: Option<u64>,   // Filter by deadline (<= end_time)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pagination {
    pub start_index: u64,
    pub limit: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowStats {
    pub total_bounties: u64,
    pub total_locked_amount: i128,
    pub total_released_amount: i128,
    pub total_refunded_amount: i128,
}

// ============================================================================
// Contract Implementation
// ============================================================================

#[contract]
pub struct BountyEscrowContract;

#[contractimpl]
impl BountyEscrowContract {
    // ========================================================================
    // Initialization
    // ========================================================================

    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        anti_abuse::check_rate_limit(&env, admin.clone());

        let start = env.ledger().timestamp();
        let caller = admin.clone();

        if env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("init"), caller, false);
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        let fee_config = FeeConfig {
            lock_fee_rate: 0,
            release_fee_rate: 0,
            fee_recipient: admin.clone(),
            fee_enabled: false,
        };
        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &fee_config);

        let config_limits = ConfigLimits {
            max_bounty_amount: None,
            min_bounty_amount: None,
            max_deadline_duration: None,
            min_deadline_duration: None,
        };
        env.storage()
            .instance()
            .set(&DataKey::ConfigLimits, &config_limits);

        env.storage()
            .instance()
            .set(&DataKey::TimeLockDuration, &0u64);
        env.storage().instance().set(&DataKey::NextActionId, &1u64);

        emit_bounty_initialized(
            &env,
            BountyEscrowInitialized {
                admin: admin.clone(),
                token,
                timestamp: env.ledger().timestamp(),
            },
        );

        monitoring::track_operation(&env, symbol_short!("init"), caller, true);

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init"), duration);

        Ok(())
    }

    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        if fee_rate == 0 {
            return 0;
        }
        amount
            .checked_mul(fee_rate)
            .and_then(|x| x.checked_div(BASIS_POINTS))
            .unwrap_or(0)
    }

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

    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
    }

    // ========================================================================
    // Admin Configuration Functions
    // ========================================================================

    /// Update admin address (with optional time-lock)
    pub fn update_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let time_lock_duration: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TimeLockDuration)
            .unwrap_or(0);

        if time_lock_duration > 0 {
            let action_id: u64 = env
                .storage()
                .instance()
                .get(&DataKey::NextActionId)
                .unwrap();
            let execution_time = env.ledger().timestamp() + time_lock_duration;

            let action = AdminAction {
                action_id,
                // FIXED: Use the Enum variant carrying the data
                action_type: AdminActionType::UpdateAdmin(new_admin.clone()),
                proposed_by: admin.clone(),
                execution_time,
                executed: false,
            };

            env.storage()
                .persistent()
                .set(&DataKey::AdminAction(action_id), &action);
            env.storage()
                .instance()
                .set(&DataKey::NextActionId, &(action_id + 1));

            emit_admin_action_proposed(
                &env,
                AdminActionProposed {
                    action_id,
                    action_type: AdminActionType::UpdateAdmin(new_admin), // Pass data for event
                    proposed_by: admin,
                    execution_time,
                    timestamp: env.ledger().timestamp(),
                },
            );
        } else {
            let old_admin = admin.clone();
            env.storage().instance().set(&DataKey::Admin, &new_admin);

            emit_admin_updated(
                &env,
                AdminUpdated {
                    old_admin,
                    new_admin,
                    updated_by: admin,
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        Ok(())
    }

    /// Update authorized payout key
    pub fn update_payout_key(env: Env, new_payout_key: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let old_key: Option<Address> = env.storage().instance().get(&DataKey::PayoutKey);

        env.storage()
            .instance()
            .set(&DataKey::PayoutKey, &new_payout_key);

        emit_payout_key_updated(
            &env,
            PayoutKeyUpdated {
                old_key,
                new_key: new_payout_key,
                updated_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Update configuration limits
    pub fn update_config_limits(
        env: Env,
        max_bounty_amount: Option<i128>,
        min_bounty_amount: Option<i128>,
        max_deadline_duration: Option<u64>,
        min_deadline_duration: Option<u64>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let limits = ConfigLimits {
            max_bounty_amount,
            min_bounty_amount,
            max_deadline_duration,
            min_deadline_duration,
        };

        env.storage()
            .instance()
            .set(&DataKey::ConfigLimits, &limits);

        emit_config_limits_updated(
            &env,
            ConfigLimitsUpdated {
                max_bounty_amount: limits.max_bounty_amount,
                min_bounty_amount: limits.min_bounty_amount,
                max_deadline_duration: limits.max_deadline_duration,
                min_deadline_duration: limits.min_deadline_duration,
                updated_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Set time-lock duration for admin actions
    pub fn set_time_lock_duration(env: Env, duration: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::TimeLockDuration, &duration);

        Ok(())
    }

    /// Execute a pending admin action
    pub fn execute_admin_action(env: Env, action_id: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env
            .storage()
            .persistent()
            .has(&DataKey::AdminAction(action_id))
        {
            return Err(Error::ActionNotFound);
        }

        let mut action: AdminAction = env
            .storage()
            .persistent()
            .get(&DataKey::AdminAction(action_id))
            .unwrap();

        if action.executed {
            return Err(Error::ActionNotFound);
        }

        if env.ledger().timestamp() < action.execution_time {
            return Err(Error::ActionNotReady);
        }

        // FIXED: Destructure the Enum data directly
        match action.action_type.clone() {
            AdminActionType::UpdateAdmin(new_admin) => {
                let old_admin = admin.clone();
                env.storage().instance().set(&DataKey::Admin, &new_admin);

                emit_admin_updated(
                    &env,
                    AdminUpdated {
                        old_admin,
                        new_admin,
                        updated_by: admin.clone(),
                        timestamp: env.ledger().timestamp(),
                    },
                );
            }
            AdminActionType::UpdatePayoutKey(new_key) => {
                let old_key: Option<Address> = env.storage().instance().get(&DataKey::PayoutKey);
                env.storage().instance().set(&DataKey::PayoutKey, &new_key);

                emit_payout_key_updated(
                    &env,
                    PayoutKeyUpdated {
                        old_key,
                        new_key,
                        updated_by: admin.clone(),
                        timestamp: env.ledger().timestamp(),
                    },
                );
            }
            AdminActionType::UpdateConfigLimits(limits) => {
                env.storage()
                    .instance()
                    .set(&DataKey::ConfigLimits, &limits);

                emit_config_limits_updated(
                    &env,
                    ConfigLimitsUpdated {
                        max_bounty_amount: limits.max_bounty_amount,
                        min_bounty_amount: limits.min_bounty_amount,
                        max_deadline_duration: limits.max_deadline_duration,
                        min_deadline_duration: limits.min_deadline_duration,
                        updated_by: admin.clone(),
                        timestamp: env.ledger().timestamp(),
                    },
                );
            }
            AdminActionType::UpdateFeeConfig(fee_config) => {
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
            }
        }

        action.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::AdminAction(action_id), &action);

        emit_admin_action_executed(
            &env,
            AdminActionExecuted {
                action_id,
                action_type: action.action_type,
                executed_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Cancel a pending admin action
    pub fn cancel_admin_action(env: Env, action_id: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env
            .storage()
            .persistent()
            .has(&DataKey::AdminAction(action_id))
        {
            return Err(Error::ActionNotFound);
        }

        let action: AdminAction = env
            .storage()
            .persistent()
            .get(&DataKey::AdminAction(action_id))
            .unwrap();

        if action.executed {
            return Err(Error::ActionNotFound);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::AdminAction(action_id));

        emit_admin_action_cancelled(
            &env,
            AdminActionCancelled {
                action_id,
                action_type: action.action_type,
                cancelled_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Get contract state (comprehensive view function)
    pub fn get_contract_state(env: Env) -> Result<ContractState, Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let payout_key: Option<Address> = env.storage().instance().get(&DataKey::PayoutKey);
        let fee_config = Self::get_fee_config_internal(&env);
        let config_limits: ConfigLimits = env
            .storage()
            .instance()
            .get(&DataKey::ConfigLimits)
            .unwrap_or(ConfigLimits {
                max_bounty_amount: None,
                min_bounty_amount: None,
                max_deadline_duration: None,
                min_deadline_duration: None,
            });
        let is_paused = Self::is_paused_internal(&env);
        let time_lock_duration: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TimeLockDuration)
            .unwrap_or(0);

        Ok(ContractState {
            admin,
            token,
            payout_key,
            fee_config,
            config_limits,
            is_paused,
            time_lock_duration,
            total_bounties: 0,
            total_locked_amount: 0,
            contract_version: 1,
        })
    }

    /// Get pending admin action
    pub fn get_admin_action(env: Env, action_id: u64) -> Result<AdminAction, Error> {
        if !env
            .storage()
            .persistent()
            .has(&DataKey::AdminAction(action_id))
        {
            return Err(Error::ActionNotFound);
        }

        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::AdminAction(action_id))
            .unwrap())
    }

    // ========================================================================
    // Pause and Emergency Functions
    // ========================================================================

    fn is_paused_internal(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get::<_, bool>(&DataKey::IsPaused)
            .unwrap_or(false)
    }

    pub fn is_paused(env: Env) -> bool {
        Self::is_paused_internal(&env)
    }

    pub fn pause(env: Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if Self::is_paused_internal(&env) {
            return Ok(());
        }

        env.storage().persistent().set(&DataKey::IsPaused, &true);

        emit_contract_paused(
            &env,
            ContractPaused {
                paused_by: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn unpause(env: Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !Self::is_paused_internal(&env) {
            return Ok(());
        }

        env.storage().persistent().set(&DataKey::IsPaused, &false);

        emit_contract_unpaused(
            &env,
            ContractUnpaused {
                unpaused_by: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn emergency_withdraw(env: Env, recipient: Address) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !Self::is_paused_internal(&env) {
            return Err(Error::Unauthorized);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        let balance = client.balance(&env.current_contract_address());

        if balance <= 0 {
            return Ok(());
        }

        client.transfer(&env.current_contract_address(), &recipient, &balance);

        emit_emergency_withdrawal(
            &env,
            EmergencyWithdrawal {
                withdrawn_by: admin.clone(),
                amount: balance,
                recipient: recipient.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    // ========================================================================
    // Core Functions (Lock, Release, Refund)
    // ========================================================================

    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        anti_abuse::check_rate_limit(&env, depositor.clone());

        let start = env.ledger().timestamp();
        let caller = depositor.clone();

        if Self::is_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            return Err(Error::ContractPaused);
        }

        depositor.require_auth();

        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);

        if amount <= 0 {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidAmount);
        }

        if deadline <= env.ledger().timestamp() {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidDeadline);
        }
        if !env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::NotInitialized);
        }

        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyExists);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0 {
            Self::calculate_fee(amount, fee_config.lock_fee_rate)
        } else {
            0
        };
        let net_amount = amount - fee_amount;

        client.transfer(&depositor, &env.current_contract_address(), &net_amount);

        if fee_amount > 0 {
            client.transfer(&depositor, &fee_config.fee_recipient, &fee_amount);
            events::emit_fee_collected(
                &env,
                events::FeeCollected {
                    operation_type: events::FeeOperationType::Lock,
                    amount: fee_amount,
                    fee_rate: fee_config.lock_fee_rate,
                    recipient: fee_config.fee_recipient.clone(),
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: net_amount,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: vec![&env],
            payout_history: vec![&env],
            remaining_amount: amount,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Update registry
        let mut registry: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::BountyRegistry)
            .unwrap_or(vec![&env]);
        registry.push_back(bounty_id);
        env.storage()
            .instance()
            .set(&DataKey::BountyRegistry, &registry);

        // Emit event for off-chain indexing
        emit_funds_locked(
            &env,
            FundsLocked {
                bounty_id,
                amount: net_amount,
                depositor: depositor.clone(),
                deadline,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        monitoring::track_operation(&env, symbol_short!("lock"), caller, true);

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("lock"), duration);

        Ok(())
    }

    /// Releases escrowed funds to a contributor.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to release funds for
    /// * `contributor` - Address to receive the funds
    ///
    /// # Returns
    /// * `Ok(())` - Funds successfully released
    /// * `Err(Error::NotInitialized)` - Contract not initialized
    /// * `Err(Error::Unauthorized)` - Caller is not the admin
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    /// * `Err(Error::FundsNotLocked)` - Funds not in LOCKED state
    ///
    /// # State Changes
    /// - Transfers tokens from contract to contributor
    /// - Updates escrow status to Released
    /// - Emits FundsReleased event
    ///
    /// # Authorization
    /// - **CRITICAL**: Only admin can call this function
    /// - Admin address must match initialization value
    ///
    /// # Security Considerations
    /// - This is the most security-critical function
    /// - Admin should verify task completion off-chain before calling
    /// - Once released, funds cannot be retrieved
    /// - Recipient address should be verified carefully
    /// - Consider implementing multi-sig for admin
    ///
    /// # Events
    /// Emits: `FundsReleased { bounty_id, amount, recipient, timestamp }`
    ///
    /// # Example
    /// ```rust
    /// // After verifying task completion off-chain:
    /// let contributor = Address::from_string("GCONTRIB...");
    ///
    /// // Admin calls release
    /// escrow_client.release_funds(&42, &contributor)?;
    /// // Funds transferred to contributor, escrow marked as Released
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Token transfer + storage update + event emission
    ///
    /// # Best Practices
    /// 1. Verify contributor identity off-chain
    /// 2. Confirm task completion before release
    /// 3. Log release decisions in backend system
    /// 4. Monitor release events for anomalies
    /// 5. Consider implementing release delays for high-value bounties
    pub fn release_funds(
        env: Env,
        bounty_id: u64,
        contributor: Address,
        amount: Option<i128>, // Optional partial amount
    ) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);
        if !env.storage().instance().has(&DataKey::Admin) {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();

        if Self::is_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::ContractPaused);
        }

        anti_abuse::check_rate_limit(&env, admin.clone());

        admin.require_auth();

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        // Allow release from Locked or PartiallyReleased states
        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyReleased
        {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::FundsNotLocked);
        }

        // Determine payout amount and validate
        let payout_amount = match amount {
            Some(amt) => {
                if amt <= 0 {
                    monitoring::track_operation(
                        &env,
                        symbol_short!("release"),
                        admin.clone(),
                        false,
                    );
                    env.storage().instance().remove(&DataKey::ReentrancyGuard);
                    return Err(Error::InvalidAmount);
                }
                if amt > escrow.remaining_amount {
                    monitoring::track_operation(
                        &env,
                        symbol_short!("release"),
                        admin.clone(),
                        false,
                    );
                    env.storage().instance().remove(&DataKey::ReentrancyGuard);
                    return Err(Error::InvalidAmount); // Attempt to over-pay
                }
                amt
            }
            None => escrow.remaining_amount, // Release full remaining amount
        };

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.release_fee_rate > 0 {
            Self::calculate_fee(payout_amount, fee_config.release_fee_rate)
        } else {
            0
        };
        let net_amount = payout_amount - fee_amount;

        // Ensure contract has sufficient funds
        let contract_balance = client.balance(&env.current_contract_address());
        if contract_balance < net_amount + fee_amount {
            return Err(Error::InsufficientFunds);
        }

        client.transfer(&env.current_contract_address(), &contributor, &net_amount);

        if fee_amount > 0 {
            client.transfer(
                &env.current_contract_address(),
                &fee_config.fee_recipient,
                &fee_amount,
            );
            events::emit_fee_collected(
                &env,
                events::FeeCollected {
                    operation_type: events::FeeOperationType::Release,
                    amount: fee_amount,
                    fee_rate: fee_config.release_fee_rate,
                    recipient: fee_config.fee_recipient.clone(),
                    timestamp: env.ledger().timestamp(),
                },
            );
        }

        // Update escrow state
        escrow.remaining_amount -= payout_amount;

        // Add to payout history
        let payout_record = PayoutRecord {
            amount: payout_amount,
            recipient: contributor.clone(),
            timestamp: env.ledger().timestamp(),
        };
        escrow.payout_history.push_back(payout_record);

        // Update status
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released; // Fully released
        } else {
            escrow.status = EscrowStatus::PartiallyReleased; // Partially released
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        emit_funds_released(
            &env,
            FundsReleased {
                bounty_id,
                amount: net_amount,
                recipient: contributor.clone(),
                timestamp: env.ledger().timestamp(),
                remaining_amount: escrow.remaining_amount,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        monitoring::track_operation(&env, symbol_short!("release"), admin, true);

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("release"), duration);
        Ok(())
    }

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
            mode,
            approved_by: admin.clone(),
            approved_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::RefundApproval(bounty_id), &approval);

        Ok(())
    }

    pub fn refund(
        env: Env,
        bounty_id: u64,
        amount: Option<i128>,
        recipient: Option<Address>,
        mode: RefundMode,
    ) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        if Self::is_paused_internal(&env) {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("refund"), caller, false);
            return Err(Error::ContractPaused);
        }

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("refund"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        let caller = escrow.depositor.clone();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded
        {
            return Err(Error::FundsNotLocked);
        }

        let now = env.ledger().timestamp();
        let is_before_deadline = now < escrow.deadline;

        let refund_amount: i128;
        let refund_recipient: Address;

        match mode {
            RefundMode::Full => {
                refund_amount = escrow.remaining_amount;
                refund_recipient = escrow.depositor.clone();
                if is_before_deadline {
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Partial => {
                refund_amount = amount.unwrap_or(escrow.remaining_amount);
                refund_recipient = escrow.depositor.clone();
                if is_before_deadline {
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Custom => {
                refund_amount = amount.ok_or(Error::InvalidAmount)?;
                refund_recipient = recipient.ok_or(Error::InvalidAmount)?;

                if is_before_deadline {
                    if !env
                        .storage()
                        .persistent()
                        .has(&DataKey::RefundApproval(bounty_id))
                    {
                        return Err(Error::RefundNotApproved);
                    }
                    let approval: RefundApproval = env
                        .storage()
                        .persistent()
                        .get(&DataKey::RefundApproval(bounty_id))
                        .unwrap();

                    if approval.amount != refund_amount
                        || approval.recipient != refund_recipient
                        || approval.mode != mode
                    {
                        return Err(Error::RefundNotApproved);
                    }

                    env.storage()
                        .persistent()
                        .remove(&DataKey::RefundApproval(bounty_id));
                }
            }
        }

        if refund_amount <= 0 || refund_amount > escrow.remaining_amount {
            return Err(Error::InvalidAmount);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        let contract_balance = client.balance(&env.current_contract_address());
        if contract_balance < refund_amount {
            return Err(Error::InsufficientFunds);
        }

        client.transfer(
            &env.current_contract_address(),
            &refund_recipient,
            &refund_amount,
        );

        escrow.remaining_amount -= refund_amount;

        let refund_record = RefundRecord {
            amount: refund_amount,
            recipient: refund_recipient.clone(),
            mode,
            timestamp: env.ledger().timestamp(),
        };
        escrow.refund_history.push_back(refund_record);

        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Refunded;
        } else {
            escrow.status = EscrowStatus::PartiallyRefunded;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        emit_funds_refunded(
            &env,
            FundsRefunded {
                bounty_id,
                amount: refund_amount,
                refund_to: refund_recipient,
                timestamp: env.ledger().timestamp(),
                refund_mode: mode,
                remaining_amount: escrow.remaining_amount,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        monitoring::track_operation(&env, symbol_short!("refund"), caller, true);

        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("refund"), duration);

        Ok(())
    }

    // ========================================================================
    // View Functions (Read-only)
    // ========================================================================

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

    pub fn get_balance(env: Env) -> Result<i128, Error> {
        if !env.storage().instance().has(&DataKey::Token) {
            return Err(Error::NotInitialized);
        }
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        Ok(client.balance(&env.current_contract_address()))
    }

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

    /// Retrieves the payout history for a specific bounty.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `bounty_id` - The bounty to query
    ///
    /// # Returns
    /// * `Ok(Vec<PayoutRecord>)` - The payout history
    /// * `Err(Error::BountyNotFound)` - Bounty doesn't exist
    pub fn get_payout_history(env: Env, bounty_id: u64) -> Result<Vec<PayoutRecord>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Ok(escrow.payout_history)
    }

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

    // ========================================================================
    // Query Functions
    // ========================================================================

    /// Query bounties with filtering and pagination.
    ///
    /// # Performance
    /// This function iterates through the registry. For large datasets, use small `pagination.limit` values
    /// to prevent gas limit errors. This is designed for off-chain indexing.
    pub fn get_bounties(
        env: Env,
        filter: EscrowFilter,
        pagination: Pagination,
    ) -> Vec<(u64, Escrow)> {
        let registry: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::BountyRegistry)
            .unwrap_or(vec![&env]);

        let mut result = vec![&env];
        let mut count: u32 = 0;
        let mut skipped: u64 = 0;

        for i in 0..registry.len() {
            // Check pagination limit
            if count >= pagination.limit {
                break;
            }

            let bounty_id = registry.get(i).unwrap();

            // Skip invalid IDs/missing data
            if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
                continue;
            }

            let escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(bounty_id))
                .unwrap();

            // Apply Filters

            // Status filter
            if let Some(status_val) = filter.status {
                if (escrow.status as u32) != status_val {
                    continue;
                }
            }

            // Depositor filter
            if let Some(depositor) = &filter.depositor {
                if &escrow.depositor != depositor {
                    continue;
                }
            }

            // Amount range filter
            if let Some(min) = filter.min_amount {
                if escrow.amount < min {
                    continue;
                }
            }
            if let Some(max) = filter.max_amount {
                if escrow.amount > max {
                    continue;
                }
            }

            // Date range filter (using deadline)
            if let Some(start) = filter.start_time {
                if escrow.deadline < start {
                    continue;
                }
            }
            if let Some(end) = filter.end_time {
                if escrow.deadline > end {
                    continue;
                }
            }

            // Apply Pagination Skip
            if skipped < pagination.start_index {
                skipped += 1;
                continue;
            }

            // Add to result
            result.push_back((bounty_id, escrow));
            count += 1;
        }

        result
    }

    /// Get aggregate statistics for the contract.
    ///
    /// # Performance
    /// This function iterates over ALL bounties. It is O(N) and may fail on-chain if N is large.
    /// Use primarily for off-chain monitoring/indexing.
    pub fn get_stats(env: Env) -> EscrowStats {
        let registry: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::BountyRegistry)
            .unwrap_or(vec![&env]);

        let mut total_locked: i128 = 0;
        let mut total_released: i128 = 0;
        let mut total_refunded: i128 = 0;

        for i in 0..registry.len() {
            let bounty_id = registry.get(i).unwrap();
            if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
                let escrow: Escrow = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Escrow(bounty_id))
                    .unwrap();

                match escrow.status {
                    EscrowStatus::Locked => {
                        total_locked += escrow.remaining_amount;
                    }
                    EscrowStatus::Released => {
                        total_released += escrow.amount;
                    }
                    EscrowStatus::Refunded => {
                        for record in escrow.refund_history.iter() {
                            total_refunded += record.amount;
                        }
                    }
                    EscrowStatus::PartiallyRefunded => {
                        total_locked += escrow.remaining_amount;
                        for record in escrow.refund_history.iter() {
                            total_refunded += record.amount;
                        }
                    }
                    EscrowStatus::PartiallyReleased => {
                        total_locked += escrow.remaining_amount;
                        // The released amount is the initial amount minus what is left
                        total_released += escrow.amount - escrow.remaining_amount;
                    }
                }
            }
        }

        EscrowStats {
            total_bounties: registry.len() as u64,
            total_locked_amount: total_locked,
            total_released_amount: total_released,
            total_refunded_amount: total_refunded,
        }
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
    pub fn batch_lock_funds(env: Env, items: Vec<LockFundsItem>) -> Result<u32, Error> {
        // Validate batch size
        let batch_size = items.len();
        if batch_size == 0 {
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        if Self::is_paused_internal(&env) {
            return Err(Error::ContractPaused);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();
        let timestamp = env.ledger().timestamp();

        for item in items.iter() {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Escrow(item.bounty_id))
            {
                return Err(Error::BountyExists);
            }

            if item.amount <= 0 {
                return Err(Error::InvalidAmount);
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
        }

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

        let mut locked_count = 0u32;
        for item in items.iter() {
            client.transfer(&item.depositor, &contract_address, &item.amount);

            let escrow = Escrow {
                depositor: item.depositor.clone(),
                amount: item.amount,
                status: EscrowStatus::Locked,
                deadline: item.deadline,
                refund_history: vec![&env],
                payout_history: vec![&env],
                remaining_amount: item.amount,
            };
            // Store escrow
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);

            emit_funds_locked(
                &env,
                FundsLocked {
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
                count: locked_count,
                total_amount: items.iter().map(|i| i.amount).sum(),
                timestamp,
            },
        );

        Ok(locked_count)
    }

    pub fn batch_release_funds(env: Env, items: Vec<ReleaseFundsItem>) -> Result<u32, Error> {
        // Validate batch size
        let batch_size = items.len();
        if batch_size == 0 {
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        if Self::is_paused_internal(&env) {
            return Err(Error::ContractPaused);
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

        let mut released_count = 0u32;
        for item in items.iter() {
            let mut escrow: Escrow = env
                .storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            client.transfer(&contract_address, &item.contributor, &escrow.amount);

            escrow.status = EscrowStatus::Released;
            env.storage()
                .persistent()
                .set(&DataKey::Escrow(item.bounty_id), &escrow);

            emit_funds_released(
                &env,
                FundsReleased {
                    bounty_id: item.bounty_id,
                    amount: escrow.amount,
                    recipient: item.contributor.clone(),
                    timestamp,
                    remaining_amount: escrow.remaining_amount,
                },
            );

            released_count += 1;
        }

        emit_batch_funds_released(
            &env,
            BatchFundsReleased {
                count: released_count,
                total_amount,
                timestamp,
            },
        );

        Ok(released_count)
    }
}

#[cfg(test)]
mod test;
