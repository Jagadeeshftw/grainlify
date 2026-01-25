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
    contract, contractimpl, contracttype, symbol_short, token, vec, Address, Env, String, Symbol,
    Vec,
};

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

// ============================================================================
// Event Types
// ============================================================================

/// Event emitted when a program is initialized/registerd

const PROGRAM_REGISTERED: Symbol = symbol_short!("ProgReg");

/// Event emitted when funds are locked in the program.
/// Topic: `FundsLocked`
const FUNDS_LOCKED: Symbol = symbol_short!("FundsLock");

/// Event emitted when a batch payout is executed.
/// Topic: `BatchPayout`
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");

/// Event emitted when a single payout is executed.
/// Topic: `Payout`
const PAYOUT: Symbol = symbol_short!("Payout");

// ============================================================================
// Storage Keys
// ============================================================================

/// Storage key for the program registry (list of all program IDs)
const PROGRAM_REGISTRY: Symbol = symbol_short!("ProgReg");

// ============================================================================
// Data Structures
// ============================================================================

// ============================================================================
// Data Structures
// ============================================================================

/// Record of an individual payout transaction.
///
/// # Fields
/// * `recipient` - Address that received the payout
/// * `amount` - Amount transferred (in token's smallest denomination)
/// * `timestamp` - Unix timestamp when payout was executed
///
/// # Usage
/// These records are stored in the payout history to provide a complete
/// audit trail of all prize distributions.
///
/// # Example
/// ```rust
/// let record = PayoutRecord {
///     recipient: winner_address,
///     amount: 1000_0000000, // 1000 USDC
///     timestamp: env.ledger().timestamp(),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

/// Complete program state and configuration.
///
/// # Fields
/// * `program_id` - Unique identifier for the program/hackathon
/// * `total_funds` - Total amount of funds locked (cumulative)
/// * `remaining_balance` - Current available balance for payouts
/// * `authorized_payout_key` - Address authorized to trigger payouts
/// * `payout_history` - Complete record of all payouts
/// * `token_address` - Token contract used for transfers
///
/// # Storage
/// Stored in instance storage with key `PROGRAM_DATA`.
///
/// # Invariants
/// - `remaining_balance <= total_funds` (always)
/// - `remaining_balance = total_funds - sum(payout_history.amounts)`
/// - `payout_history` is append-only
/// - `program_id` and `authorized_payout_key` are immutable after init
///
/// # Example
/// ```rust
/// let program_data = ProgramData {
///     program_id: String::from_str(&env, "Hackathon2024"),
///     total_funds: 10_000_0000000,
///     remaining_balance: 7_000_0000000,
///     authorized_payout_key: backend_address,
///     payout_history: vec![&env],
///     token_address: usdc_token_address,
/// };
/// ```

/// Complete program state and configuration.
///
/// # Storage Key
/// Stored with key: `("Program", program_id)`
///
/// # Invariants
/// - `remaining_balance <= total_funds` (always)
/// - `remaining_balance = total_funds - sum(payout_history.amounts)`
/// - `payout_history` is append-only
/// - `program_id` and `authorized_payout_key` are immutable after registration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address,
}

/// Storage key type for individual programs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String), // program_id -> ProgramData
}

// ============================================================================
// Contract Implementation
// ============================================================================

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    // ========================================================================
    // Program Registration & Initialization
    // ========================================================================

    /// Initializes a new program escrow for managing prize distributions.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `program_id` - Unique identifier for this program/hackathon
    /// * `authorized_payout_key` - Address authorized to trigger payouts (backend)
    /// * `token_address` - Address of the token contract for transfers (e.g., USDC)
    ///
    /// # Returns
    /// * `ProgramData` - The initialized program configuration
    ///
    /// # Panics
    /// * If program is already initialized
    ///
    /// # State Changes
    /// - Creates ProgramData with zero balances
    /// - Sets authorized payout key (immutable after this)
    /// - Initializes empty payout history
    /// - Emits ProgramInitialized event
    ///
    /// # Security Considerations
    /// - Can only be called once (prevents re-configuration)
    /// - No authorization required (first-caller initialization)
    /// - Authorized payout key should be a secure backend service
    /// - Token address must be a valid Stellar Asset Contract
    /// - Program ID should be unique and descriptive
    ///
    /// # Events
    /// Emits: `ProgramInit(program_id, authorized_payout_key, token_address, 0)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{Address, String, Env};
    ///
    /// let program_id = String::from_str(&env, "ETHGlobal2024");
    /// let backend = Address::from_string("GBACKEND...");
    /// let usdc = Address::from_string("CUSDC...");
    ///
    /// let program = escrow_client.init_program(
    ///     &program_id,
    ///     &backend,
    ///     &usdc
    /// );
    ///
    /// println!("Program created: {}", program.program_id);
    /// ```
    ///
    /// # Production Setup
    /// ```bash
    /// # Deploy contract
    /// stellar contract deploy \
    ///   --wasm target/wasm32-unknown-unknown/release/escrow.wasm \
    ///   --source ORGANIZER_KEY
    ///
    /// # Initialize program
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ORGANIZER_KEY \
    ///   -- init_program \
    ///   --program_id "Hackathon2024" \
    ///   --authorized_payout_key GBACKEND... \
    ///   --token_address CUSDC...
    /// ```
    ///
    /// # Gas Cost
    /// Low - Initial storage writes

    pub fn initialize_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
    ) -> ProgramData {
        let start = env.ledger().timestamp();
        let caller = authorized_payout_key.clone();

        // Validate program_id
        if program_id.len() == 0 {
            monitoring::track_operation(&env, symbol_short!("init_prg"), caller, false);
            panic!("Program ID cannot be empty");
        }

        // Check if program already exists
        let program_key = DataKey::Program(program_id.clone());
        if env.storage().instance().has(&program_key) {
            monitoring::track_operation(&env, symbol_short!("init_prg"), caller, false);
            panic!("Program already exists");
        }

        // Create program data
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
            token_address: token_address.clone(),
        };

        // Store program data
        env.storage().instance().set(&program_key, &program_data);

        // Update registry
        let mut registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);
        registry.push_back(program_id.clone());
        env.storage().instance().set(&PROGRAM_REGISTRY, &registry);

        // Emit registration event
        env.events().publish(
            (PROGRAM_REGISTERED,),
            (program_id, authorized_payout_key, token_address, 0i128),
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init_prg"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init_prg"), duration);

        program_data
    }

    /// Lists all registered program IDs in the contract.
    ///
    /// # Returns
    /// * `Vec<String>` - List of all program IDs
    ///
    /// # Example
    /// ```rust
    /// let programs = escrow_client.list_programs();
    /// for program_id in programs.iter() {
    ///     println!("Program: {}", program_id);
    /// }
    /// ```
    pub fn list_programs(env: Env) -> Vec<String> {
        env.storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env])
    }

    /// Checks if a program exists.
    ///
    /// # Arguments
    /// * `program_id` - The program ID to check
    ///
    /// # Returns
    /// * `bool` - True if program exists, false otherwise
    pub fn program_exists(env: Env, program_id: String) -> bool {
        let program_key = DataKey::Program(program_id);
        env.storage().instance().has(&program_key)
    }

    // ========================================================================
    // Fund Management
    // ========================================================================

    /// Locks funds into the program escrow for prize distribution.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `amount` - Amount of tokens to lock (in token's smallest denomination)
    ///
    /// # Returns
    /// * `ProgramData` - Updated program data with new balance
    ///
    /// # Panics
    /// * If amount is zero or negative
    /// * If program is not initialized
    ///
    /// # State Changes
    /// - Increases `total_funds` by amount
    /// - Increases `remaining_balance` by amount
    /// - Emits FundsLocked event
    ///
    /// # Prerequisites
    /// Before calling this function:
    /// 1. Caller must have sufficient token balance
    /// 2. Caller must approve contract for token transfer
    /// 3. Tokens must actually be transferred to contract
    ///
    /// # Security Considerations
    /// - Amount must be positive
    /// - This function doesn't perform the actual token transfer
    /// - Caller is responsible for transferring tokens to contract
    /// - Consider verifying contract balance matches recorded amount
    /// - Multiple lock operations are additive (cumulative)
    ///
    /// # Events
    /// Emits: `FundsLocked(program_id, amount, new_remaining_balance)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::token;
    ///
    /// // 1. Transfer tokens to contract
    /// let amount = 10_000_0000000; // 10,000 USDC
    /// token_client.transfer(
    ///     &organizer,
    ///     &contract_address,
    ///     &amount
    /// );
    ///
    /// // 2. Record the locked funds
    /// let updated = escrow_client.lock_program_funds(&amount);
    /// println!("Locked: {} USDC", amount / 10_000_000);
    /// println!("Remaining: {}", updated.remaining_balance);
    /// ```
    ///
    /// # Production Usage
    /// ```bash
    /// # 1. Transfer USDC to contract
    /// stellar contract invoke \
    ///   --id USDC_TOKEN_ID \
    ///   --source ORGANIZER_KEY \
    ///   -- transfer \
    ///   --from ORGANIZER_ADDRESS \
    ///   --to CONTRACT_ADDRESS \
    ///   --amount 10000000000
    ///
    /// # 2. Record locked funds
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ORGANIZER_KEY \
    ///   -- lock_program_funds \
    ///   --amount 10000000000
    /// ```
    ///
    /// # Gas Cost
    /// Low - Storage update + event emission
    ///
    /// # Common Pitfalls
    /// - Forgetting to transfer tokens before calling
    /// -  Locking amount that exceeds actual contract balance
    /// -  Not verifying contract received the tokens

    pub fn lock_program_funds(env: Env, program_id: String, amount: i128) -> ProgramData {
        let start = env.ledger().timestamp();
        let caller = env.current_contract_address();

        // Validate amount
        if amount <= 0 {
            monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
            panic!("Amount must be greater than zero");
        }

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| {
                monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
                panic!("Program not found")
            });

        // Update balances
        program_data.total_funds += amount;
        program_data.remaining_balance += amount;

        // Store updated data
        env.storage().instance().set(&program_key, &program_data);

        // Emit event
        env.events().publish(
            (FUNDS_LOCKED,),
            (program_id, amount, program_data.remaining_balance),
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("lock"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("lock"), duration);

        program_data
    }

    // ========================================================================
    // Payout Functions
    // ========================================================================

    /// Executes batch payouts to multiple recipients simultaneously.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `recipients` - Vector of recipient addresses
    /// * `amounts` - Vector of amounts (must match recipients length)
    ///
    /// # Returns
    /// * `ProgramData` - Updated program data after payouts
    ///
    /// # Panics
    /// * If caller is not the authorized payout key
    /// * If program is not initialized
    /// * If recipients and amounts vectors have different lengths
    /// * If vectors are empty
    /// * If any amount is zero or negative
    /// * If total payout exceeds remaining balance
    /// * If arithmetic overflow occurs
    ///
    /// # Authorization
    /// - **CRITICAL**: Only authorized payout key can call
    /// - Caller must be exact match to `authorized_payout_key`
    ///
    /// # State Changes
    /// - Transfers tokens from contract to each recipient
    /// - Adds PayoutRecord for each transfer to history
    /// - Decreases `remaining_balance` by total payout amount
    /// - Emits BatchPayout event
    ///
    /// # Atomicity
    /// This operation is atomic - either all transfers succeed or all fail.
    /// If any transfer fails, the entire batch is reverted.
    ///
    /// # Security Considerations
    /// - Verify recipient addresses off-chain before calling
    /// - Ensure amounts match winner rankings/criteria
    /// - Total payout is calculated with overflow protection
    /// - Balance check prevents overdraft
    /// - All transfers are logged for audit trail
    /// - Consider implementing payout limits for additional safety
    ///
    /// # Events
    /// Emits: `BatchPayout(program_id, recipient_count, total_amount, new_balance)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{vec, Address};
    ///
    /// // Define winners and prizes
    /// let winners = vec![
    ///     &env,
    ///     Address::from_string("GWINNER1..."), // 1st place
    ///     Address::from_string("GWINNER2..."), // 2nd place
    ///     Address::from_string("GWINNER3..."), // 3rd place
    /// ];
    ///
    /// let prizes = vec![
    ///     &env,
    ///     5_000_0000000,  // $5,000 USDC
    ///     3_000_0000000,  // $3,000 USDC
    ///     2_000_0000000,  // $2,000 USDC
    /// ];
    ///
    /// // Execute batch payout (only authorized backend can call)
    /// let result = escrow_client.batch_payout(&winners, &prizes);
    /// println!("Paid {} winners", winners.len());
    /// println!("Remaining: {}", result.remaining_balance);
    /// ```
    ///
    /// # Production Usage
    /// ```bash
    /// # Batch payout to 3 winners
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source BACKEND_KEY \
    ///   -- batch_payout \
    ///   --recipients '["GWINNER1...", "GWINNER2...", "GWINNER3..."]' \
    ///   --amounts '[5000000000, 3000000000, 2000000000]'
    /// ```
    ///
    /// # Gas Cost
    /// High - Multiple token transfers + storage updates
    /// Cost scales linearly with number of recipients
    ///
    /// # Best Practices
    /// 1. Verify all winner addresses before execution
    /// 2. Double-check prize amounts match criteria
    /// 3. Test on testnet with same number of recipients
    /// 4. Monitor events for successful completion
    /// 5. Keep batch size reasonable (recommend < 50 recipients)
    ///
    /// # Limitations
    /// - Maximum batch size limited by gas/resource limits
    /// - For very large batches, consider multiple calls
    /// - All amounts must be positive  
    pub fn batch_payout(
        env: Env,
        program_id: String,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> ProgramData {
        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Verify authorization - CRITICAL
        program_data.authorized_payout_key.require_auth();

        // Validate inputs
        if recipients.len() != amounts.len() {
            panic!("Recipients and amounts vectors must have the same length");
        }

        if recipients.is_empty() {
            panic!("Cannot process empty batch");
        }

        // Calculate total with overflow protection
        let mut total_payout: i128 = 0;
        for amount in amounts.iter() {
            if amount <= 0 {
                panic!("All amounts must be greater than zero");
            }
            total_payout = total_payout
                .checked_add(amount)
                .unwrap_or_else(|| panic!("Payout amount overflow"));
        }

        // Validate balance
        if total_payout > program_data.remaining_balance {
            panic!(
                "Insufficient balance: requested {}, available {}",
                total_payout, program_data.remaining_balance
            );
        }

        // Execute transfers
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        for (i, recipient) in recipients.iter().enumerate() {
            let amount = amounts.get(i.try_into().unwrap()).unwrap();

            // Transfer tokens
            token_client.transfer(&contract_address, &recipient, &amount);

            // Record payout
            let payout_record = PayoutRecord {
                recipient: recipient.clone(),
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
        env.storage().instance().set(&program_key, &updated_data);

        // Emit event
        env.events().publish(
            (BATCH_PAYOUT,),
            (
                program_id,
                recipients.len() as u32,
                total_payout,
                updated_data.remaining_balance,
            ),
        );

        updated_data
    }

    /// Executes a single payout to one recipient.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `recipient` - Address of the prize recipient
    /// * `amount` - Amount to transfer (in token's smallest denomination)
    ///
    /// # Returns
    /// * `ProgramData` - Updated program data after payout
    ///
    /// # Panics
    /// * If caller is not the authorized payout key
    /// * If program is not initialized
    /// * If amount is zero or negative
    /// * If amount exceeds remaining balance
    ///
    /// # Authorization
    /// - Only authorized payout key can call this function
    ///
    /// # State Changes
    /// - Transfers tokens from contract to recipient
    /// - Adds PayoutRecord to history
    /// - Decreases `remaining_balance` by amount
    /// - Emits Payout event
    ///
    /// # Security Considerations
    /// - Verify recipient address before calling
    /// - Amount must be positive
    /// - Balance check prevents overdraft
    /// - Transfer is logged in payout history
    ///
    /// # Events
    /// Emits: `Payout(program_id, recipient, amount, new_balance)`
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::Address;
    ///
    /// let winner = Address::from_string("GWINNER...");
    /// let prize = 1_000_0000000; // $1,000 USDC
    ///
    /// // Execute single payout
    /// let result = escrow_client.single_payout(&winner, &prize);
    /// println!("Paid {} to winner", prize);
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Single token transfer + storage update
    ///
    /// # Use Cases
    /// - Individual prize awards
    /// - Bonus payments
    /// - Late additions to prize pool distribution
    pub fn single_payout(
        env: Env,
        program_id: String,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.authorized_payout_key.require_auth();
        // Verify authorization
        // let caller = env.invoker();
        // if caller != program_data.authorized_payout_key {
        //     panic!("Unauthorized: only authorized payout key can trigger payouts");
        // }

        // Validate amount
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // Validate balance
        if amount > program_data.remaining_balance {
            panic!(
                "Insufficient balance: requested {}, available {}",
                amount, program_data.remaining_balance
            );
        }

        // Transfer tokens
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
        env.storage().instance().set(&program_key, &updated_data);

        // Emit event
        env.events().publish(
            (PAYOUT,),
            (
                program_id,
                recipient,
                amount,
                updated_data.remaining_balance,
            ),
        );

        updated_data
    }

    // ========================================================================
    // View Functions (Read-only)
    // ========================================================================

    /// Retrieves complete program information.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `ProgramData` - Complete program state including:
    ///   - Program ID
    ///   - Total funds locked
    ///   - Remaining balance
    ///   - Authorized payout key
    ///   - Complete payout history
    ///   - Token contract address
    ///
    /// # Panics
    /// * If program is not initialized
    ///
    /// # Use Cases
    /// - Verifying program configuration
    /// - Checking balances before payouts
    /// - Auditing payout history
    /// - Displaying program status in UI
    ///
    /// # Example
    /// ```rust
    /// let info = escrow_client.get_program_info();
    /// println!("Program: {}", info.program_id);
    /// println!("Total Locked: {}", info.total_funds);
    /// println!("Remaining: {}", info.remaining_balance);
    /// println!("Payouts Made: {}", info.payout_history.len());
    /// ```
    ///
    /// # Gas Cost
    /// Very Low - Single storage read
    pub fn get_program_info(env: Env, program_id: String) -> ProgramData {
        let program_key = DataKey::Program(program_id);
        env.storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"))
    }

    /// Retrieves the remaining balance for a specific program.
    ///
    /// # Arguments
    /// * `program_id` - The program ID to query
    ///
    /// # Returns
    /// * `i128` - Remaining balance
    ///
    /// # Panics
    /// * If program doesn't exist
    pub fn get_remaining_balance(env: Env, program_id: String) -> i128 {
        let program_key = DataKey::Program(program_id);
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.remaining_balance
    }

    /// Gets the total number of programs registered.
    ///
    /// # Returns
    /// * `u32` - Count of registered programs
    pub fn get_program_count(env: Env) -> u32 {
        let registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);

        registry.len()
    }

    // ========================================================================
    // Monitoring & Analytics Functions
    // ========================================================================

    /// Health check - returns contract health status
    pub fn health_check(env: Env) -> monitoring::HealthStatus {
        monitoring::health_check(&env)
    }

    /// Get analytics - returns usage analytics
    pub fn get_analytics(env: Env) -> monitoring::Analytics {
        monitoring::get_analytics(&env)
    }

    /// Get state snapshot - returns current state
    pub fn get_state_snapshot(env: Env) -> monitoring::StateSnapshot {
        monitoring::get_state_snapshot(&env)
    }

    /// Get performance stats for a function
    pub fn get_performance_stats(env: Env, function_name: Symbol) -> monitoring::PerformanceStats {
        monitoring::get_performance_stats(&env, function_name)
    }
}

/// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env, String,
    };

    // Test helper to create a mock token contract
    fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let token_address = env.register_stellar_asset_contract(admin.clone());
        token::Client::new(env, &token_address)
    }

    // ========================================================================
    // Program Registration Tests
    // ========================================================================

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
        assert!(client.program_exists(&prog_id));
        assert_eq!(client.get_program_count(), 1);
    }

    #[test]
    fn test_multiple_programs_isolation() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend1 = Address::generate(&env);
        let backend2 = Address::generate(&env);
        let backend3 = Address::generate(&env);
        let token = Address::generate(&env);

        // Register three programs
        let prog1 = String::from_str(&env, "ETHGlobal2024");
        let prog2 = String::from_str(&env, "Stellar2024");
        let prog3 = String::from_str(&env, "BuildathonQ1");

        client.initialize_program(&prog1, &backend1, &token);
        client.initialize_program(&prog2, &backend2, &token);
        client.initialize_program(&prog3, &backend3, &token);

        // Verify all exist
        assert!(client.program_exists(&prog1));
        assert!(client.program_exists(&prog2));
        assert!(client.program_exists(&prog3));
        assert_eq!(client.get_program_count(), 3);

        // Verify complete isolation
        let info1 = client.get_program_info(&prog1);
        let info2 = client.get_program_info(&prog2);
        let info3 = client.get_program_info(&prog3);

        assert_eq!(info1.program_id, prog1);
        assert_eq!(info2.program_id, prog2);
        assert_eq!(info3.program_id, prog3);

        assert_eq!(info1.authorized_payout_key, backend1);
        assert_eq!(info2.authorized_payout_key, backend2);
        assert_eq!(info3.authorized_payout_key, backend3);

        // Verify list programs
        let programs = client.list_programs();
        assert_eq!(programs.len(), 3);
    }

    #[test]
    #[should_panic(expected = "Program already exists")]
    fn test_duplicate_program_registration() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register once - should succeed
        client.initialize_program(&prog_id, &backend, &token);

        // Register again - should panic
        client.initialize_program(&prog_id, &backend, &token);
    }

    #[test]
    #[should_panic(expected = "Program ID cannot be empty")]
    fn test_empty_program_id() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let empty_id = String::from_str(&env, "");

        client.initialize_program(&empty_id, &backend, &token);
    }

    #[test]
    #[should_panic(expected = "Program not found")]
    fn test_get_nonexistent_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let prog_id = String::from_str(&env, "DoesNotExist");
        client.get_program_info(&prog_id);
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
        let updated = client.lock_program_funds(&prog_id, &amount);

        assert_eq!(updated.total_funds, amount);
        assert_eq!(updated.remaining_balance, amount);
    }

    #[test]
    fn test_lock_funds_multiple_programs_isolation() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend1 = Address::generate(&env);
        let backend2 = Address::generate(&env);

        let prog1 = String::from_str(&env, "Program1");
        let prog2 = String::from_str(&env, "Program2");

        // Register programs
        client.initialize_program(&prog1, &backend1, &token_client.address);
        client.initialize_program(&prog2, &backend2, &token_client.address);

        // Lock different amounts in each program
        let amount1 = 5_000_0000000i128;
        let amount2 = 10_000_0000000i128;

        client.lock_program_funds(&prog1, &amount1);
        client.lock_program_funds(&prog2, &amount2);

        // Verify isolation - funds don't mix
        let info1 = client.get_program_info(&prog1);
        let info2 = client.get_program_info(&prog2);

        assert_eq!(info1.total_funds, amount1);
        assert_eq!(info1.remaining_balance, amount1);
        assert_eq!(info2.total_funds, amount2);
        assert_eq!(info2.remaining_balance, amount2);
    }

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
        client.lock_program_funds(&prog_id, &1_000_0000000);
        client.lock_program_funds(&prog_id, &2_000_0000000);
        client.lock_program_funds(&prog_id, &3_000_0000000);

        let info = client.get_program_info(&prog_id);
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
        client.lock_program_funds(&prog_id, &0);
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
        client.lock_program_funds(&prog_id, &10_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env), Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 1_000_0000000i128]; // Mismatch!

        client.batch_payout(&prog_id, &recipients, &amounts);
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
        client.lock_program_funds(&prog_id, &5_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 10_000_0000000i128]; // More than available!

        client.batch_payout(&prog_id, &recipients, &amounts);
    }

    #[test]
    fn test_program_count() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        assert_eq!(client.get_program_count(), 0);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);

        client.initialize_program(&String::from_str(&env, "P1"), &backend, &token);
        assert_eq!(client.get_program_count(), 1);

        client.initialize_program(&String::from_str(&env, "P2"), &backend, &token);
        assert_eq!(client.get_program_count(), 2);

        client.initialize_program(&String::from_str(&env, "P3"), &backend, &token);
        assert_eq!(client.get_program_count(), 3);
    }
}
