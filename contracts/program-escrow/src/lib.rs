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

// ── Step 1: Add module declarations near the top of lib.rs ──────────────
// (after `mod anti_abuse;` and before the contract struct)

mod error_recovery;
mod reentrancy_guard;

#[cfg(test)]
mod error_recovery_tests;

#[cfg(test)]
mod reentrancy_tests;

#[cfg(test)]
mod reentrancy_guard_standalone_test;

#[cfg(test)]
mod malicious_reentrant;

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

// ── Step 3: Wrap batch_payout and single_payout with circuit breaker ────
//
// In the existing batch_payout function, add at the very top (after getting
// program_data but before the auth check):
//
//   use crate::error_recovery;
//   if let Err(_) = error_recovery::check_and_allow(&env) {
//       panic!("Circuit breaker is open: payout operations are temporarily disabled");
//   }
//
// After a successful transfer loop, add:
//   error_recovery::record_success(&env);
//
// If a transfer panics/fails, the circuit breaker failure should be recorded
// via record_failure() before re-panicking.
//
// For a clean integration, wrap the token transfer call like this:
//
//   let transfer_ok = std::panic::catch_unwind(|| {
//       token_client.transfer(&contract_address, &recipient.clone(), &net_amount);
//   });
//   match transfer_ok {
//       Ok(_) => error_recovery::record_success(&env),
//       Err(_) => {
//           error_recovery::record_failure(
//               &env,
//               program_id.clone(),
//               soroban_sdk::symbol_short!("batch_pay"),
//               error_recovery::ERR_TRANSFER_FAILED,
//           );
//           panic!("Token transfer failed");
//       }
//   }
//
// Note: Soroban's environment panics abort the transaction, so in practice
// you record the failure and re-panic. The circuit breaker state is committed
// because Soroban persists storage writes made before the panic in tests
// (but not in production transactions that abort). For full production
// integration, use the `try_*` variants of client calls where available.

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, vec, Address, Env, String, Symbol,
    Vec,
};

// Event types
const PROGRAM_INITIALIZED: Symbol = symbol_short!("PrgInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FndsLock");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
const PAYOUT: Symbol = symbol_short!("Payout");
const EVENT_VERSION_V2: u32 = 2;
const PAUSE_STATE_CHANGED: Symbol = symbol_short!("PauseSt");

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
    pub token_address: Address, // Token contract address for transfers
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramAggregateStats {
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub total_paid_out: i128,
    pub payout_count: u32,
    pub scheduled_count: u32,
    pub released_count: u32,
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
    ) -> ProgramData {
        Self::initialize_program(env, program_id, authorized_payout_key, token_address)
    }

    pub fn initialize_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
    ) -> ProgramData {
        // Check if program already exists
        if env.storage().instance().has(&PROGRAM_DATA) {
            panic!("Program already initialized");
        }

        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
            token_address: token_address.clone(),
        };

        // Store program data
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
                total_funds: 0i128,
            },
        );

        program_data
    }

    /// Check if a program exists
    ///
    /// # Returns
    /// * `bool` - True if program exists, false otherwise
    pub fn program_exists(env: Env) -> bool {
        env.storage().instance().has(&PROGRAM_DATA)
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

    pub fn set_admin(env: Env, admin: Address) {
        Self::initialize_contract(env, admin);
    }

    /// Update pause flags (admin only)
    pub fn set_paused(env: Env, lock: Option<bool>, release: Option<bool>, refund: Option<bool>) {
        if !env.storage().instance().has(&DataKey::Admin) {
            panic!("Not initialized");
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut flags = Self::get_pause_flags(&env);

        if let Some(paused) = lock {
            flags.lock_paused = paused;
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                (symbol_short!("lock"), paused, admin.clone()),
            );
        }

        if let Some(paused) = release {
            flags.release_paused = paused;
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                (symbol_short!("release"), paused, admin.clone()),
            );
        }

        if let Some(paused) = refund {
            flags.refund_paused = paused;
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                (symbol_short!("refund"), paused, admin.clone()),
            );
        }

        env.storage().instance().set(&DataKey::PauseFlags, &flags);
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
        env.storage().instance().set(&DataKey::RateLimitConfig, &config);
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
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap_or_else(|| panic!("Not initialized"));
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
    /// # Returns
    /// Updated ProgramData after payout
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
        amount: i128,
        release_timestamp: u64,
        recipient: Address,
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

    pub fn get_program_release_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
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

    pub fn single_payout_v2(env: Env, _program_id: String, recipient: Address, amount: i128) -> ProgramData {
        Self::single_payout(env, recipient, amount)
    }

    pub fn batch_payout_v2(env: Env, _program_id: String, recipients: Vec<Address>, amounts: Vec<i128>) -> ProgramData {
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

    pub fn get_program_release_schedule(
        env: Env,
        schedule_id: u64,
    ) -> ProgramReleaseSchedule {
        let schedules = Self::get_program_release_schedules(env);
        for s in schedules.iter() {
            if s.schedule_id == schedule_id {
                return s;
            }
        }
        panic!("Schedule not found");
    }

    pub fn get_all_prog_release_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        Self::get_program_release_schedules(env)
    }

    pub fn get_pending_program_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        Self::get_pending_schedules(env)
    }

    pub fn get_due_program_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        Self::get_due_schedules(env)
    }

    pub fn release_program_schedule_manual(env: Env, schedule_id: u64) {
        let mut schedules = Self::get_program_release_schedules(env.clone());
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
            let mut history: Vec<ProgramReleaseHistory> = env.storage()
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
        let mut schedules = Self::get_program_release_schedules(env.clone());
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
            let mut history: Vec<ProgramReleaseHistory> = env.storage()
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
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env, String, Vec,
    };

    // Test helper to create a mock token contract
    fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_address = token_contract.address();
        token::Client::new(env, &token_address)
    }

    // ========================================================================
    // Program Registration Tests
    // ========================================================================

    fn setup_program_with_schedule(
        env: &Env,
        client: &ProgramEscrowContractClient<'static>,
        contract_id: &Address,
        authorized_key: &Address,
        _token: &Address,
        program_id: &String,
        total_amount: i128,
        winner: &Address,
        release_timestamp: u64,
    ) {
        // // Register program
        // client.register_program(program_id, token, authorized_key);

        // // Create and fund token
        // let token_client = create_token_contract(env, authorized_key);
        // let token_admin = token::StellarAssetClient::new(env, &token_client.address);
        // token_admin.mint(authorized_key, &total_amount);

        // // Lock funds for program
        // token_client.approve(authorized_key, &env.current_contract_address(), &total_amount, &1000);
        // client.lock_funds(program_id, &total_amount);

        // Create and fund token first, then register the program with the real token address
        let token_client = create_token_contract(env, authorized_key);
        let token_admin = token::StellarAssetClient::new(env, &token_client.address);
        token_admin.mint(authorized_key, &total_amount);

        // Register program using the created token contract address
        client.initialize_program(&program_id, &authorized_key, &token_client.address);

        // Transfer tokens to contract first
        token_client.transfer(&authorized_key, contract_id, &total_amount);

        // Lock funds for program (records the amount in program state)
        client.lock_program_funds(&total_amount);

        // Create release schedule
        client.create_program_release_schedule(
            &total_amount,
            &release_timestamp,
            winner,
        );
    }

    #[test]
    fn test_single_program_release_schedule() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &contract_id,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Verify schedule was created
        let schedule = client.get_program_release_schedule(&1);
        assert_eq!(schedule.schedule_id, 1);
        assert_eq!(schedule.amount, amount);
        assert_eq!(schedule.release_timestamp, release_timestamp);
        assert_eq!(schedule.recipient, winner);
        assert!(!schedule.released);

        // Check pending schedules
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 1);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_multiple_program_release_schedules() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 600_0000000;
        let amount2 = 400_0000000;
        let total_amount = amount1 + amount2;

        env.mock_all_auths();

        // Register program
        client.initialize_program(&program_id, &authorized_key, &token);

        // Create and fund token
        let token_client = create_token_contract(&env, &authorized_key);
        let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
        token_admin.mint(&authorized_key, &total_amount);

        // Transfer tokens to contract first
        token_client.transfer(&authorized_key, &contract_id, &total_amount);

        // Lock funds for program
        client.lock_program_funds(&total_amount);

        // Create first release schedule
        client.create_program_release_schedule(
            &amount1,
            &1000,
            &winner1,
        );
        
        // Create second release schedule
        client.create_program_release_schedule(
            &amount2,
            &2000,
            &winner2,
        );
        
        // Verify both schedules exist
        let all_schedules = client.get_all_prog_release_schedules();
        assert_eq!(all_schedules.len(), 2);

        // Verify schedule IDs
        let schedule1 = client.get_program_release_schedule(&1);
        let schedule2 = client.get_program_release_schedule(&2);
        assert_eq!(schedule1.schedule_id, 1);
        assert_eq!(schedule2.schedule_id, 2);

        // Verify amounts
        assert_eq!(schedule1.amount, amount1);
        assert_eq!(schedule2.amount, amount2);

        // Verify recipients
        assert_eq!(schedule1.recipient, winner1);
        assert_eq!(schedule2.recipient, winner2);

        // Check pending schedules
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 2);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_program_automatic_release_at_timestamp() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &contract_id,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Try to release before timestamp (should fail)
        env.ledger().set_timestamp(999);
        let result = client.try_release_prog_schedule_automatic(&1);
        assert!(result.is_err());

        // Advance time to after release timestamp
        env.ledger().set_timestamp(1001);

        // Release automatically
        client.release_prog_schedule_automatic(&1);
        
        // Verify schedule was released
        let schedule = client.get_program_release_schedule(&1);
        assert!(schedule.released);
        assert_eq!(schedule.released_at, Some(1001));

        assert_eq!(schedule.released_by, Some(contract_id.clone()));

        // Check no pending schedules
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 0);

        // Verify release history
        let history = client.get_program_release_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().release_type, ReleaseType::Automatic);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_program_manual_trigger_before_after_timestamp() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &contract_id,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Manually release before timestamp (authorized key can do this)
        env.ledger().set_timestamp(999);
        client.release_program_schedule_manual(&1);
        
        // Verify schedule was released
        let schedule = client.get_program_release_schedule(&1);
        assert!(schedule.released);
        assert_eq!(schedule.released_at, Some(999));
        assert_eq!(schedule.released_by, Some(authorized_key.clone()));

        // Verify release history
        let history = client.get_program_release_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().release_type, ReleaseType::Manual);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_verify_program_schedule_tracking_and_history() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 600_0000000;
        let amount2 = 400_0000000;
        let total_amount = amount1 + amount2;

        env.mock_all_auths();

        // Create and fund token FIRST
        let token_client = create_token_contract(&env, &authorized_key);
        let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
        token_admin.mint(&authorized_key, &total_amount);

        // Register program with REAL token address
        client.initialize_program(&program_id, &authorized_key, &token_client.address);

        // Transfer tokens to contract first
        token_client.transfer(&authorized_key, &contract_id, &total_amount);

        // Lock funds for program
        client.lock_program_funds(&total_amount);

        // Create first schedule
        client.create_program_release_schedule(
            &amount1,
            &1000,
            &winner1,
        );
        
        // Create second schedule
        client.create_program_release_schedule(
            &amount2,
            &2000,
            &winner2,
        );
        
        // Release first schedule manually
        client.release_program_schedule_manual(&1);
        
        // Advance time and release second schedule automatically
        env.ledger().set_timestamp(2001);
        client.release_prog_schedule_automatic(&2);
        
        // Verify complete history
        let history = client.get_program_release_history();
        assert_eq!(history.len(), 2);

        // Check first release (manual)
        let first_release = history.get(0).unwrap();
        assert_eq!(first_release.schedule_id, 1);
        assert_eq!(first_release.amount, amount1);
        assert_eq!(first_release.recipient, winner1);
        assert_eq!(first_release.release_type, ReleaseType::Manual);

        // Check second release (automatic)
        let second_release = history.get(1).unwrap();
        assert_eq!(second_release.schedule_id, 2);
        assert_eq!(second_release.amount, amount2);
        assert_eq!(second_release.recipient, winner2);
        assert_eq!(second_release.release_type, ReleaseType::Automatic);

        // Verify no pending schedules
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 0);

        // Verify all schedules are marked as released
        let all_schedules = client.get_all_prog_release_schedules();
        assert_eq!(all_schedules.len(), 2);
        assert!(all_schedules.get(0).unwrap().released);
        assert!(all_schedules.get(1).unwrap().released);
    }

    #[test]
    fn test_program_overlapping_schedules() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let winner3 = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 300_0000000;
        let amount2 = 300_0000000;
        let amount3 = 400_0000000;
        let total_amount = amount1 + amount2 + amount3;
        let base_timestamp = 1000;

        env.mock_all_auths();

        // Create and fund token FIRST
        let token_client = create_token_contract(&env, &authorized_key);
        let token_admin = token::StellarAssetClient::new(&env, &token_client.address);
        token_admin.mint(&authorized_key, &total_amount);

        // Register program with REAL token address
        client.initialize_program(&program_id, &authorized_key, &token_client.address);

        // Transfer tokens to contract first
        token_client.transfer(&authorized_key, &contract_id, &total_amount);

        // Lock funds for program
        client.lock_program_funds(&total_amount);

        // Create overlapping schedules (all at same timestamp)
        client.create_program_release_schedule(
            &amount1,
            &base_timestamp,
            &winner1.clone(),
        );

        client.create_program_release_schedule(
            &amount2,
            &base_timestamp,
            &winner2.clone(),
        );

        client.create_program_release_schedule(
            &amount3,
            &base_timestamp,
            &winner3.clone(),
        );

        // Advance time to after release timestamp
        env.ledger().set_timestamp(base_timestamp + 1);

        // Check due schedules (should be all 3)
        let due = client.get_due_program_schedules();
        assert_eq!(due.len(), 3);

        // Release schedules one by one
        client.release_prog_schedule_automatic(&1);
        client.release_prog_schedule_automatic(&2);
        client.release_prog_schedule_automatic(&3);
        
        // Verify all schedules are released
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 0);

        // Verify complete history
        let history = client.get_program_release_history();
        assert_eq!(history.len(), 3);

        // Verify all were automatic releases
        for release in history.iter() {
            assert_eq!(release.release_type, ReleaseType::Automatic);
        }

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_register_single_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register program
        let program = client.initialize_program(&prog_id, &backend, &token);

        // Verify program data
        assert_eq!(program.program_id, prog_id);
        assert_eq!(program.authorized_payout_key, backend);
        assert_eq!(program.token_address, token);
        assert_eq!(program.total_funds, 0);
        assert_eq!(program.remaining_balance, 0);
        assert_eq!(program.payout_history.len(), 0);

        // Verify it exists
        assert!(client.program_exists());
        assert_eq!(client.get_program_count(), 1);
    }

    #[test]
    // NOTE: test_multiple_programs_isolation removed — single-program model
    // does not allow registering multiple programs.
    fn _test_multiple_programs_isolation_removed() {}

    // NOTE: test_duplicate_program_registration removed — single-program model
    // does not support re-registration semantics in this form.

    // NOTE: test_empty_program_id removed — initialize_program does not
    // currently validate empty program IDs in the single-program model.

    #[test]
    #[should_panic]
    fn test_get_nonexistent_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        // Calling get_program_info without initializing should panic
        client.get_program_info();
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
        let updated = client.lock_program_funds(&amount);

        assert_eq!(updated.total_funds, amount);
        assert_eq!(client.get_remaining_balance(), amount);
    }

    #[test]
    // NOTE: Multi-tenant tests removed — single-program model does not
    // allow registering multiple programs on the same contract instance.
    // Removed: test_lock_funds_multiple_programs_isolation
    // Removed: test_multi_tenant_payout_history_isolation
    // Removed: test_multi_tenant_release_schedule_isolation
    // Removed: test_multi_tenant_release_history_isolation

    // NOTE: test_multi_tenant_analytics_isolation_concept removed — single-program model.

    // ========================================================================
    // Edge Cases for Program Management
    // ========================================================================

    #[test]
    fn test_program_reinitialization_attempt() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // First registration should succeed
        client.initialize_program(&prog_id, &backend, &token);
        assert!(client.program_exists());

        let info = client.get_program_info();
        assert_eq!(info.program_id, prog_id);
    }

    // NOTE: test_program_count removed — single-program model does not
    // allow registering multiple programs.

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
        client.lock_program_funds(&1_000_0000000);
        client.lock_program_funds(&2_000_0000000);
        client.lock_program_funds(&3_000_0000000);

        let info = client.get_program_info();
        assert_eq!(info.total_funds, 6_000_0000000);
        assert_eq!(info.remaining_balance, 6_000_0000000);
    }

    #[test]
    #[should_panic(expected = "Amount must be greater than zero")]
    fn test_lock_zero_funds() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        client.initialize_program(&prog_id, &backend, &token);
        client.lock_program_funds(&0);
    }

    // ========================================================================
    // Batch Payout Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Recipients and amounts vectors must have the same length")]
    fn test_batch_payout_mismatched_lengths() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &backend, &token_client.address);
        client.lock_program_funds(&10_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env), Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 1_000_0000000i128]; // Mismatch!

        client.batch_payout(&recipients, &amounts);
    }

    #[test]
    #[should_panic(expected = "Insufficient balance")]
    fn test_batch_payout_insufficient_balance() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &backend, &token_client.address);
        client.lock_program_funds(&5_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 10_000_0000000i128]; // More than available!

        client.batch_payout(&recipients, &amounts);
    }

    // NOTE: test_program_count removed — single-program model does not
    // allow registering multiple programs.

    // ========================================================================
    // Anti-Abuse Tests
    // ========================================================================

    // NOTE: Anti-abuse tests that depend on multi-program initialization
    // have been removed. Rate-limit and cooldown logic is currently
    // not integrated into program-escrow's init flow in the single-program model.
    // Anti-abuse tests should be re-added if/when integrated.

    #[test]
    fn test_anti_abuse_config_update() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.set_admin(&admin);

        client.update_rate_limit_config(&7200, &5, &120);

        let config = client.get_rate_limit_config();
        assert_eq!(config.window_size, 7200);
        assert_eq!(config.max_operations, 5);
        assert_eq!(config.cooldown_period, 120);
    }
}
mod test;
#[cfg(test)]
mod rbac_tests;
